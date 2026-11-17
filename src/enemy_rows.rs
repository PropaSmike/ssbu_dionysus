use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use dionysus_core::prcxml::{self, Value, Why};
use dionysus_core::row_fields::{self, enemy, RowValue};
use std::collections::HashMap;
use std::sync::Mutex;

const MANAGER: usize = 0x52b8418;
const MANAGER_PARAMS: usize = 0xb8;
const PARAMS_ROWS: usize = 0xa8;
const SITES: usize = 17;
const BUFFER_WORDS: usize = enemy::ROW_SIZE * 2 / 8;

struct Own {
    values: Vec<(usize, RowValue)>,
    buffer: Box<[u64; BUFFER_WORDS]>,
    slot: usize,
    rows: usize,
}

const NO_FRAME: AtomicU64 = AtomicU64::new(0);
const NO_KIND: AtomicU32 = AtomicU32::new(0);
static PENDING_FRAME: [AtomicU64; SITES] = [NO_FRAME; SITES];
static PENDING_KIND: [AtomicU32; SITES] = [NO_KIND; SITES];
static OWN_MASK: AtomicU32 = AtomicU32::new(0);
static OWN: Mutex<Option<HashMap<u32, Own>>> = Mutex::new(None);
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

fn has_own(kind: u32) -> bool {
    kind < 32 && OWN_MASK.load(Ordering::Acquire) & (1 << kind) != 0
}

fn store(kind: u32, offset: usize, value: RowValue) {
    if let Ok(mut own) = OWN.lock() {
        let own = own.get_or_insert_with(HashMap::new).entry(kind).or_insert_with(|| Own {
            values: Vec::new(),
            buffer: Box::new([0; BUFFER_WORDS]),
            slot: 0,
            rows: 0,
        });
        own.values.retain(|(known, _)| *known != offset);
        own.values.push((offset, value));
        own.rows = 0;
    }
    OWN_MASK.fetch_or(1 << kind, Ordering::AcqRel);
}

pub(crate) fn set(kind: u32, field: u64, value: &Value) -> Result<(), Why> {
    let (offset, converted) = row_fields::set(&enemy::FIELDS, field, value)?;
    store(kind, offset, converted);
    Ok(())
}

pub(crate) fn load_file(kind: u32, name: &str) {
    let file = format!("{}.prcxml", prcxml::ENEMY_FILE);
    let Some(path) = crate::param_files::find_in_mods(&["enemy", name, &file]) else {
        return;
    };
    let label = crate::param_files::file_name(path.ancestors().nth(3).unwrap_or(&path));
    let Some(root) = crate::param_files::read(&path, "enemyclone", &label) else {
        return;
    };
    let found = row_fields::changes(&enemy::FIELDS, &prcxml::enemy_row(&root));
    let set = found.values.len();
    for (offset, value) in found.values {
        store(kind, offset, value);
    }
    skyline::println!(
        "[enemyclone] {name}: {file} {}",
        crate::param_files::summary(set, 0, &found.skipped, Some(found.rows))
    );
}

pub(crate) fn note(site: usize, frame: u64, kind: u32, clone: bool) {
    if site >= SITES {
        return;
    }
    if clone && has_own(kind) {
        PENDING_KIND[site].store(kind, Ordering::Relaxed);
        PENDING_FRAME[site].store(frame, Ordering::Release);
    } else {
        PENDING_FRAME[site].store(0, Ordering::Release);
    }
}

unsafe fn own_index(kind: u32, base_index: i64) -> Option<i64> {
    let manager = read::<usize>(crate::game::addr(MANAGER));
    if manager == 0 {
        return None;
    }
    let params = read::<usize>(manager + MANAGER_PARAMS);
    if params == 0 {
        return None;
    }
    let rows = read::<usize>(params + PARAMS_ROWS);
    if rows == 0 || base_index < 0 {
        return None;
    }
    let mut own = OWN.lock().ok()?;
    let own = own.as_mut()?.get_mut(&kind)?;
    let buffer = own.buffer.as_ptr() as usize;
    let gap = (rows as i64 - buffer as i64).rem_euclid(enemy::ROW_SIZE as i64) as usize;
    let slot = buffer + gap;
    if own.rows != rows || own.slot != slot {
        let source = rows + base_index as usize * enemy::ROW_SIZE;
        let row = core::slice::from_raw_parts_mut(slot as *mut u8, enemy::ROW_SIZE);
        row.copy_from_slice(core::slice::from_raw_parts(source as *const u8, enemy::ROW_SIZE));
        for (offset, value) in &own.values {
            value.write(row, *offset);
        }
        own.rows = rows;
        own.slot = slot;
        if LOGGED.fetch_or(1 << kind, Ordering::Relaxed) & (1 << kind) == 0 {
            skyline::println!(
                "[enemyclone] enemy kind {kind} reads its own row of enemy/common/common.prc ({} values over its base's)",
                own.values.len()
            );
        }
    }
    Some((slot as i64 - rows as i64) / enemy::ROW_SIZE as i64)
}

macro_rules! row_index_sites {
    ($($name:ident = $site:literal, $offset:literal, $opcode:literal, $register:literal;)*) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let frame = PENDING_FRAME[$site].load(Ordering::Acquire);
                if frame == 0 || frame != ctx.sp.x() {
                    return;
                }
                PENDING_FRAME[$site].store(0, Ordering::Release);
                let kind = PENDING_KIND[$site].load(Ordering::Relaxed);
                let base_index = ctx.registers[$register].x() as i64;
                if let Some(index) = own_index(kind, base_index) {
                    ctx.registers[$register].set_x(index as u64);
                }
            }
        )*

        const ROW_INDEX_SITES: &[(usize, u32)] = &[$(($offset, $opcode)),*];

        fn install_row_index_sites() {
            skyline::install_hooks!($($name),*);
        }
    };
}

row_index_sites! {
    index_object_a = 0, 0x566130, 0xf9405529, 8;
    index_object_b = 1, 0x566544, 0xf9405d28, 23;
    index_object_c = 2, 0x5671a0, 0xf9405529, 8;
    index_object_d = 3, 0x567334, 0xf9405529, 8;
    index_object_e = 4, 0x567f70, 0xf9405529, 8;
    index_object_f = 5, 0x568294, 0xf9405529, 8;
    index_object_g = 6, 0x5696d8, 0xf9405529, 8;
    index_object_h = 7, 0x5699b0, 0xf9405508, 9;
    index_request_a = 8, 0x56e62c, 0xf9405508, 9;
    index_request_b = 9, 0x56e6b8, 0xf9405508, 9;
    index_descriptor = 10, 0x571e8c, 0xf9405529, 10;
    index_foreign_a = 11, 0x90e8d0, 0xf9405529, 8;
    index_foreign_b = 12, 0xb9f97c, 0xf9405529, 8;
    index_foreign_c = 13, 0xe627b0, 0xf9405d28, 20;
    index_foreign_d = 14, 0x34b1a04, 0xf9405529, 8;
    index_foreign_e = 15, 0x34b285c, 0xf9002be9, 9;
    index_foreign_f = 16, 0x3516880, 0xf9405d28, 24;
}

pub(crate) fn sites() -> &'static [(usize, u32)] {
    ROW_INDEX_SITES
}

pub(crate) fn install() {
    install_row_index_sites();
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_common_set(kind: u32, field: u64, tag: u32, bits: u64) -> i32 {
    if crate::enemy_clones::base_of(kind).is_none() {
        return -1;
    }
    let Some(value) = row_fields::value_from_tag(tag, bits) else {
        return -3;
    };
    match set(kind, field, &value) {
        Ok(()) => 0,
        Err(Why::Unknown) => -2,
        Err(_) => -3,
    }
}
