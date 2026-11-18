use core::sync::atomic::{AtomicUsize, Ordering};
use dionysus_core::prcxml::{self, Value, Why};
use dionysus_core::row_fields::{self, cpu, RowField, RowValue};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

const PARSE: usize = 0x389170;
const PARSE_HEAD: u32 = 0xd10243ff;
const ALLOCATE: usize = 0x392dce0;
const FREE: usize = 0x392e590;
const ALIGNMENT: u64 = 0x10;
const VECTOR_BEGIN: usize = 8;
const VECTOR_END: usize = 0x10;
const VECTOR_CAPACITY: usize = 0x18;
const KEY_MASK: u64 = 0xff_ffff_ffff;
const VANILLA_KIND_HASHES: usize = 0x453b1c0;
const VANILLA_KINDS: i32 = 94;

const AI_WORLD: usize = 0x52b5fd8;
const WORLD_INFOS: usize = 0x90;
const INFO_STRIDE: usize = 0x280;
const INFO_KIND: usize = 0x28;
const PLAYER_ENTRIES: usize = 8;
const AI_KIND_COPY: usize = 0x988;
const KIND_LIMIT: usize = 256;

static KIND_CELLS: [i32; KIND_LIMIT] = {
    let mut cells = [0i32; KIND_LIMIT];
    let mut index = 0;
    while index < KIND_LIMIT {
        cells[index] = index as i32;
        index += 1;
    }
    cells
};

#[derive(Clone, Copy)]
enum Change {
    Set(RowValue),
    Mul(f32),
}

const IDENTITY_OFFSET: usize = 0;

static OWN: Mutex<Option<HashMap<i32, Vec<(&'static RowField, Change)>>>> = Mutex::new(None);
static APPENDED: Mutex<Option<HashSet<i32>>> = Mutex::new(None);
static AIS: Mutex<Option<HashMap<usize, i32>>> = Mutex::new(None);
static LOGGED: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static PARAM_OBJECT: AtomicUsize = AtomicUsize::new(0);
static APPENDED_BEGIN: AtomicUsize = AtomicUsize::new(0);

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn write<T>(address: usize, value: T) {
    core::ptr::write_volatile(address as *mut T, value)
}

fn store(kind: i32, field: &'static RowField, change: Change) {
    if let Ok(mut own) = OWN.lock() {
        let changes = own.get_or_insert_with(HashMap::new).entry(kind).or_default();
        changes.retain(|(known, _)| known.offset != field.offset);
        changes.push((field, change));
    }
}

fn apply(row: &mut [u8], base: &[u8], field: &RowField, change: Change) {
    let value = match change {
        Change::Set(value) => Some(value),
        Change::Mul(factor) => row_fields::multiplied(field, base, factor),
    };
    if let Some(value) = value {
        value.write(row, field.offset);
    }
}

pub(crate) fn set(kind: i32, hash: u64, value: &Value) -> Result<(), Why> {
    let field = row_fields::find(&cpu::FIELDS, hash).ok_or(Why::Unknown)?;
    if field.offset == IDENTITY_OFFSET {
        return Err(Why::Label);
    }
    let converted = row_fields::convert(field, value)?;
    store(kind, field, Change::Set(converted));
    unsafe { write_live(kind, field, Change::Set(converted)) };
    Ok(())
}

pub(crate) fn mul(kind: i32, hash: u64, factor: f32) -> Result<(), Why> {
    let field = row_fields::find(&cpu::FIELDS, hash).ok_or(Why::Unknown)?;
    if row_fields::multiplied(field, &[0; cpu::ROW_SIZE], factor).is_none() {
        return Err(Why::Type);
    }
    store(kind, field, Change::Mul(factor));
    unsafe { write_live(kind, field, Change::Mul(factor)) };
    Ok(())
}

pub(crate) fn note_ai(ai: usize, clone: Option<i32>) {
    if let Ok(mut ais) = AIS.lock() {
        let ais = ais.get_or_insert_with(HashMap::new);
        match clone {
            Some(kind) => {
                ais.insert(ai, kind);
            }
            None => {
                ais.remove(&ai);
            }
        }
    }
}

pub(crate) fn load_file(directory: &str, resource: &str, kind: i32) {
    let name = format!("{}.prcxml", prcxml::CPU_FILE);
    let path = crate::param_files::path(directory, &["fighter", resource, "ai", "param", &name]);
    let Some(root) = crate::param_files::read(&path, "fighterpack", directory) else {
        return;
    };
    let found = row_fields::changes(&cpu::FIELDS, &prcxml::cpu_row(&root));
    let already: Vec<usize> = OWN
        .lock()
        .ok()
        .and_then(|own| own.as_ref()?.get(&kind).map(|changes| changes.iter().map(|(field, _)| field.offset).collect()))
        .unwrap_or_default();
    let mut set = 0;
    for (offset, value) in found.values {
        let Some(field) = cpu::FIELDS.iter().find(|field| field.offset == offset) else {
            continue;
        };
        if already.contains(&offset) {
            continue;
        }
        store(kind, field, Change::Set(value));
        set += 1;
    }
    skyline::println!(
        "[fighterpack] {directory}: ai/param/{name} {}",
        crate::param_files::summary(set, 0, &found.skipped, Some(found.rows))
    );
}

unsafe fn base_key(kind: i32) -> Option<u64> {
    let base = crate::clone_base_of(kind)?;
    if !(0..VANILLA_KINDS).contains(&base) {
        return None;
    }
    Some(read::<u64>(crate::game::addr(VANILLA_KIND_HASHES) + base as usize * 8))
}

unsafe fn rows_of(object: usize) -> Option<(usize, usize)> {
    let begin = read::<usize>(object + VECTOR_BEGIN);
    let end = read::<usize>(object + VECTOR_END);
    (begin != 0 && end >= begin).then_some((begin, (end - begin) / cpu::ROW_SIZE))
}

unsafe fn find_row(begin: usize, from: usize, count: usize, key: u64) -> Option<usize> {
    (from..count)
        .map(|index| begin + index * cpu::ROW_SIZE)
        .find(|row| read::<u64>(*row) & KEY_MASK == key & KEY_MASK)
}

unsafe fn write_live(kind: i32, field: &RowField, change: Change) {
    let object = PARAM_OBJECT.load(Ordering::Acquire);
    let (Some(key), Some(base_key)) = (crate::clone_kind_hash(kind), base_key(kind)) else {
        return;
    };
    if object == 0 || !appended(kind) {
        return;
    }
    let Some((begin, count)) = rows_of(object) else {
        return;
    };
    let (Some(row), Some(base)) = (find_row(begin, 0, count, key), find_row(begin, 0, count, base_key)) else {
        return;
    };
    let base = core::slice::from_raw_parts(base as *const u8, cpu::ROW_SIZE).to_vec();
    let cells = core::slice::from_raw_parts_mut(row as *mut u8, cpu::ROW_SIZE);
    apply(cells, &base, field, change);
}

fn appended(kind: i32) -> bool {
    APPENDED
        .lock()
        .ok()
        .and_then(|appended| appended.as_ref().map(|kinds| kinds.contains(&kind)))
        .unwrap_or(false)
}

unsafe fn append(object: usize) {
    PARAM_OBJECT.store(object, Ordering::Release);
    APPENDED_BEGIN.store(0, Ordering::Release);
    let own: Vec<(i32, Vec<(&'static RowField, Change)>)> = OWN
        .lock()
        .ok()
        .and_then(|own| own.as_ref().map(|own| own.iter().map(|(kind, changes)| (*kind, changes.clone())).collect()))
        .unwrap_or_default();
    if own.is_empty() {
        return;
    }
    let Some((begin, count)) = rows_of(object) else {
        return;
    };
    let mut rows: Vec<(i32, Vec<u8>)> = Vec::new();
    for (kind, changes) in own {
        let (Some(key), Some(base)) = (crate::clone_kind_hash(kind), base_key(kind)) else {
            continue;
        };
        let Some(source) = find_row(begin, 0, count, base) else {
            continue;
        };
        let base = core::slice::from_raw_parts(source as *const u8, cpu::ROW_SIZE).to_vec();
        let mut row = base.clone();
        row[0..8].copy_from_slice(&key.to_le_bytes());
        for (field, change) in &changes {
            apply(&mut row, &base, field, *change);
        }
        rows.push((kind, row));
    }
    if rows.is_empty() {
        return;
    }
    let needed = (count + rows.len()) * cpu::ROW_SIZE;
    let mut begin = begin;
    if read::<usize>(object + VECTOR_CAPACITY).saturating_sub(begin) < needed {
        let allocate: extern "C" fn(u64, u64) -> *mut u8 = core::mem::transmute(crate::game::addr(ALLOCATE));
        let free: extern "C" fn(*mut u8) = core::mem::transmute(crate::game::addr(FREE));
        let fresh = allocate(ALIGNMENT, needed as u64);
        if fresh.is_null() {
            return;
        }
        core::ptr::copy_nonoverlapping(begin as *const u8, fresh, count * cpu::ROW_SIZE);
        free(begin as *mut u8);
        begin = fresh as usize;
        write(object + VECTOR_BEGIN, begin);
        write(object + VECTOR_CAPACITY, begin + needed);
    }
    let mut kinds = HashSet::new();
    for (index, (kind, row)) in rows.iter().enumerate() {
        let at = begin + (count + index) * cpu::ROW_SIZE;
        core::ptr::copy_nonoverlapping(row.as_ptr(), at as *mut u8, cpu::ROW_SIZE);
        kinds.insert(*kind);
    }
    write(object + VECTOR_END, begin + needed);
    if let Ok(mut appended) = APPENDED.lock() {
        *appended = Some(kinds);
    }
    APPENDED_BEGIN.store(begin, Ordering::Release);
}

#[skyline::hook(offset = crate::game::rel(PARSE))]
unsafe fn cpu_param_parse(object: usize, node: usize) -> usize {
    let result = call_original!(object, node);
    append(object);
    result
}

fn live() -> bool {
    let begin = APPENDED_BEGIN.load(Ordering::Acquire);
    let object = PARAM_OBJECT.load(Ordering::Acquire);
    begin != 0 && object != 0 && unsafe { read::<usize>(object + VECTOR_BEGIN) } == begin
}

unsafe fn entry_of(info: usize) -> Option<u8> {
    let cell = read::<usize>(crate::game::addr(AI_WORLD));
    if cell == 0 {
        return None;
    }
    let world = read::<usize>(cell);
    if world == 0 {
        return None;
    }
    let infos = read::<usize>(world + WORLD_INFOS);
    let offset = info.checked_sub(infos)?;
    (offset % INFO_STRIDE == 0 && offset / INFO_STRIDE < PLAYER_ENTRIES).then_some((offset / INFO_STRIDE) as u8)
}

fn steer(kind: Option<i32>) -> Option<i32> {
    let kind = kind?;
    if !(live() && appended(kind) && (kind as usize) < KIND_LIMIT) {
        return None;
    }
    if let Ok(mut logged) = LOGGED.lock() {
        if !logged.contains(&kind) {
            logged.push(kind);
            skyline::println!("[cpurows] kind {kind}'s computer player reads its own row of fighter/common/ai/param/fighter_param.prc");
        }
    }
    Some(kind)
}

unsafe fn clone_of_info(info: usize) -> Option<i32> {
    if !live() {
        return None;
    }
    steer(entry_of(info).and_then(crate::css_registration::entry_custom_kind))
}

macro_rules! info_kind_sites {
    ($($name:ident = $offset:literal, $opcode:literal, $info:literal, $back:literal, $kind:literal;)*) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let info = (ctx.registers[$info].x() as usize).wrapping_sub($back);
                if let Some(kind) = clone_of_info(info) {
                    ctx.registers[$kind].set_x(kind as u64);
                }
            }
        )*

        const INFO_KIND_SITES: &[(usize, u32)] = &[$(($offset, $opcode)),*];

        fn install_info_kind_sites() {
            skyline::install_hooks!($($name),*);
        }
    };
}

info_kind_sites! {
    cpu_row_a = 0x32aa68, 0xaa1703e8, 8, 0x28, 10;
    cpu_row_b = 0x32ab3c, 0xaa1703e8, 8, 0x28, 10;
    cpu_row_c = 0x32ac4c, 0xaa1703e8, 8, 0x28, 10;
    cpu_row_d = 0x32ad54, 0xaa1703e8, 8, 0x28, 10;
    cpu_row_e = 0x32ae24, 0xaa1703e8, 8, 0x28, 10;
    cpu_row_f = 0x32afa4, 0xaa1703e8, 8, 0x28, 10;
    cpu_row_g = 0x63fe90, 0xf9407589, 0, 0, 22;
    cpu_row_h = 0x32f5d4, 0xf9407509, 12, 0, 13;
}

const POINTER_SITE: usize = 0x32b51c;
const POINTER_SITE_OPCODE: u32 = 0xb9800129;
const POINTER_REGISTER: usize = 9;
const AI_SITE: usize = 0x33f450;
const AI_SITE_OPCODE: u32 = 0xf9405168;
const AI_POINTER_REGISTER: usize = 0;
const AI_KIND_REGISTER: usize = 9;

#[skyline::hook(offset = crate::game::rel(POINTER_SITE), inline)]
unsafe fn cpu_row_pointer(ctx: &mut skyline::hooks::InlineCtx) {
    let info = (ctx.registers[POINTER_REGISTER].x() as usize).wrapping_sub(INFO_KIND);
    if let Some(kind) = clone_of_info(info) {
        ctx.registers[POINTER_REGISTER].set_x(&KIND_CELLS[kind as usize] as *const i32 as u64);
    }
}

#[skyline::hook(offset = crate::game::rel(AI_SITE), inline)]
unsafe fn cpu_row_ai(ctx: &mut skyline::hooks::InlineCtx) {
    if !live() {
        return;
    }
    let ai = (ctx.registers[AI_POINTER_REGISTER].x() as usize).wrapping_sub(AI_KIND_COPY);
    let clone = AIS.lock().ok().and_then(|ais| ais.as_ref()?.get(&ai).copied());
    if let Some(kind) = steer(clone) {
        ctx.registers[AI_KIND_REGISTER].set_x(kind as u64);
    }
}

pub(crate) fn install() {
    let sites = INFO_KIND_SITES
        .iter()
        .chain(core::iter::once(&(POINTER_SITE, POINTER_SITE_OPCODE)))
        .chain(core::iter::once(&(AI_SITE, AI_SITE_OPCODE)))
        .chain(core::iter::once(&(PARSE, PARSE_HEAD)));
    for &(offset, expected) in sites {
        let observed = unsafe { crate::game::text_word(offset) };
        if observed != expected {
            skyline::println!(
                "[cpurows] {offset:#x} holds {observed:#010x}, not {expected:#010x}; clones' computer players keep their base's row"
            );
            return;
        }
    }
    install_info_kind_sites();
    skyline::install_hooks!(cpu_param_parse, cpu_row_pointer, cpu_row_ai);
    skyline::println!("[cpurows] clones can give their computer players their own row of fighter/common/ai/param/fighter_param.prc");
}

#[no_mangle]
pub extern "C" fn clone_engine_cpu_param_set(kind: i32, field: u64, tag: u32, bits: u64) -> i32 {
    if crate::clone_base_of(kind).is_none() {
        return -1;
    }
    let result = if tag == row_fields::TAG_MUL {
        mul(kind, field, f32::from_bits(bits as u32))
    } else {
        let Some(value) = row_fields::value_from_tag(tag, bits) else {
            return -3;
        };
        set(kind, field, &value)
    };
    match result {
        Ok(()) => 0,
        Err(Why::Unknown) => -2,
        Err(_) => -3,
    }
}
