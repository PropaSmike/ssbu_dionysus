use dionysus_core::item_common_row::Table;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};

use crate::item_slots::{self, text_word, InlineHook};

const NATIVE_KIND_TERM: u64 = 0x1B0;

const ASSIST_KIND_FIRST: i32 = 0xA6;
const ASSIST_KIND_LAST: i32 = 0x118;
const POKEMON_KIND_FIRST: i32 = 0x119;
const POKEMON_KIND_LAST: i32 = 0x15F;
const BOSS_KIND_FIRST: i32 = 0x160;
const BOSS_KIND_LAST: i32 = 0x1A8;
const KOOPAG_KIND: i32 = 398;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum ItemContentCategory {
    Item,
    Assist,
    Pokemon,
    Boss,
    KoopagExternal,
}

impl ItemContentCategory {
    pub(crate) const fn root(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Assist => "assist",
            Self::Pokemon => "pokemon",
            Self::Boss | Self::KoopagExternal => "boss",
        }
    }

    pub(crate) const fn main_root_offset(self) -> usize {
        match self {
            Self::Item => 0x42C2080,
            Self::Assist => 0x437ADC0,
            Self::Pokemon => 0x444236F,
            Self::Boss | Self::KoopagExternal => 0x4348902,
        }
    }
}

pub(crate) const fn item_content_category_for_base(base_kind: i32) -> ItemContentCategory {
    if base_kind == KOOPAG_KIND {
        ItemContentCategory::KoopagExternal
    } else if base_kind >= ASSIST_KIND_FIRST && base_kind <= ASSIST_KIND_LAST {
        ItemContentCategory::Assist
    } else if base_kind >= POKEMON_KIND_FIRST && base_kind <= POKEMON_KIND_LAST {
        ItemContentCategory::Pokemon
    } else if base_kind >= BOSS_KIND_FIRST && base_kind <= BOSS_KIND_LAST {
        ItemContentCategory::Boss
    } else {
        ItemContentCategory::Item
    }
}

const _: () = {
    use ItemContentCategory::*;
    assert!(item_content_category_for_base(0xA5) as u8 == Item as u8);
    assert!(item_content_category_for_base(0xA6) as u8 == Assist as u8);
    assert!(item_content_category_for_base(0x118) as u8 == Assist as u8);
    assert!(item_content_category_for_base(0x119) as u8 == Pokemon as u8);
    assert!(item_content_category_for_base(0x15F) as u8 == Pokemon as u8);
    assert!(item_content_category_for_base(0x160) as u8 == Boss as u8);
    assert!(item_content_category_for_base(0x18E) as u8 == KoopagExternal as u8);
    assert!(item_content_category_for_base(0x1A8) as u8 == Boss as u8);
    assert!(item_content_category_for_base(0x1A9) as u8 == Item as u8);
};

pub(crate) fn item_content_category(public_kind: i32) -> ItemContentCategory {
    item_content_category_for_base(crate::item_clones::clone_engine_item_base_kind(public_kind))
}

#[cfg(test)]
mod category_tests {
    use super::{item_content_category_for_base, ItemContentCategory::*};

    #[test]
    fn native_category_boundaries_are_exact() {
        assert_eq!(item_content_category_for_base(0xA5), Item);
        assert_eq!(item_content_category_for_base(0xA6), Assist);
        assert_eq!(item_content_category_for_base(0x118), Assist);
        assert_eq!(item_content_category_for_base(0x119), Pokemon);
        assert_eq!(item_content_category_for_base(0x15F), Pokemon);
        assert_eq!(item_content_category_for_base(0x160), Boss);
        assert_eq!(item_content_category_for_base(0x18E), KoopagExternal);
        assert_eq!(item_content_category_for_base(0x1A8), Boss);
        assert_eq!(item_content_category_for_base(0x1A9), Item);
    }
}

const PARAM_ENTRY_ARRAY: usize = 0x1E08;
const DUET_ENTRY_ARRAY: usize = 0x2B88;

const WORD_CELL_SITE: usize = 0x15F8830;
const WORD_CELL_EXPECTED: u32 = 0xB9400321;

const DUET_WORD_CELL_SITE: usize = 0x15F8984;
const DUET_WORD_CELL_EXPECTED: u32 = 0xB94002E1;

const PARAM_CELL_SITE: usize = 0x15F88C4;
const PARAM_CELL_EXPECTED: u32 = 0xA9027C1F;

const DUET_CELL_SITE: usize = 0x15F8A14;
const DUET_CELL_EXPECTED: u32 = 0xA9027C1F;

const PATH_PROBE_SITE: usize = 0x15F87C4;
const PATH_PROBE_EXPECTED: u32 = 0xAA1F03E8;
const PATH_STRING_REGISTER: usize = 9;

const CATEGORY_ROOT_SITE: usize = 0x15F8798;
const CATEGORY_ROOT_EXPECTED: u32 = 0x910083E0;
const CATEGORY_ROOT_REGISTER: usize = 23;

const HASH_PROBE_SITE: usize = 0x15F8814;
const HASH_PROBE_EXPECTED: u32 = 0xD101A3A0;
const HASH_REGISTER: usize = 1;

const RESOLVER_OUTPUT_OFFSET: usize = 0x68;
const FRAME_REGISTER: usize = 29;

const LOOKUP_PROBE_SITE: usize = 0x15F8850;
const LOOKUP_PROBE_EXPECTED: u32 = 0x6B1B031F;
const LOOKUP_RESULT_REGISTER: usize = 24;

const PROBE_KINDS: &[usize] = &[0x3F, 0x40];

fn probe_kind(kind: usize) -> bool {
    PROBE_KINDS.contains(&kind) || position_of(kind).is_some()
}

const GETTER_KIND_REGISTER: usize = 1;

const GETTER_SITES: &[(usize, u32, usize)] = &[
    (0x1602ACC, 0xF9400008, 0),
    (0x1602C28, 0xF9400008, 0),
    (0x1602E4C, 0xF9400108, 8),
    (0x1602FF0, 0xF9400008, 0),
    (0x160326C, 0xF9400008, 0),
    (0x16035AC, 0xF9400108, 8),
    (0x1603880, 0xF9400008, 0),
    (0x1603B8C, 0xF9400008, 0),
    (0x1603F5C, 0xF9400108, 8),
    (0x16042B8, 0xF9400008, 0),
];

const LOOP_BOUND_SITE: usize = 0x15F8A50;
const LOOP_BOUND_EXPECTED: u32 = 0xF106C39F;

const KIND_REGISTER: usize = 28;
const CELL_REGISTER: usize = 8;
const WORD_REGISTER: usize = 25;
const DUET_WORD_REGISTER: usize = 23;

#[repr(C, align(16))]
struct CloneParamCells {
    param_entry: usize,
    duet_entry: usize,
    word: u32,
    duet_word: u32,
    accessor_cell: usize,
    accessor_cell_base: usize,
}

const RESOURCE_NONE: u32 = 0x00FF_FFFF;

impl CloneParamCells {
    const fn new() -> Self {
        Self {
            param_entry: 0,
            duet_entry: 0,
            word: RESOURCE_NONE,
            duet_word: RESOURCE_NONE,
            accessor_cell: 0,
            accessor_cell_base: 0,
        }
    }
}

pub(crate) const MAX_CLONE_KINDS: usize = 256;

static mut CELLS: [CloneParamCells; MAX_CLONE_KINDS] =
    [const { CloneParamCells::new() }; MAX_CLONE_KINDS];

static KINDS: [AtomicUsize; MAX_CLONE_KINDS] =
    [const { AtomicUsize::new(usize::MAX) }; MAX_CLONE_KINDS];
static KIND_COUNT: AtomicUsize = AtomicUsize::new(0);
static REGISTRATION_LOCK: Mutex<()> = Mutex::new(());

static LAST_HASH: [core::sync::atomic::AtomicU64; MAX_CLONE_KINDS] =
    [const { core::sync::atomic::AtomicU64::new(0) }; MAX_CLONE_KINDS];
static SUBST_REPORTED: [AtomicBool; MAX_CLONE_KINDS] =
    [const { AtomicBool::new(false) }; MAX_CLONE_KINDS];
static CACHED_INDEX: [AtomicUsize; MAX_CLONE_KINDS] =
    [const { AtomicUsize::new(usize::MAX) }; MAX_CLONE_KINDS];
static ARRAY_BASE: AtomicUsize = AtomicUsize::new(0);

static LIVE_ACCESSOR: AtomicUsize = AtomicUsize::new(0);

static PREFLIGHT_OK: AtomicBool = AtomicBool::new(false);
static HOOKS_INSTALLED: AtomicBool = AtomicBool::new(false);
static LOOP_RAN: AtomicBool = AtomicBool::new(false);

pub(crate) fn can_register_family(public_kinds: &[i32]) -> bool {
    let Ok(_guard) = REGISTRATION_LOCK.lock() else {
        return false;
    };
    let count = KIND_COUNT.load(Ordering::Acquire);
    if public_kinds.is_empty() || count + public_kinds.len() > MAX_CLONE_KINDS {
        return false;
    }
    public_kinds.iter().enumerate().all(|(index, kind)| {
        *kind >= NATIVE_KIND_TERM as i32
            && position_of(*kind as usize).is_none()
            && !public_kinds[..index].contains(kind)
    })
}

pub(crate) fn register_family(public_kinds: &[i32]) -> bool {
    let Ok(_guard) = REGISTRATION_LOCK.lock() else {
        return false;
    };
    let count = KIND_COUNT.load(Ordering::Acquire);
    if public_kinds.is_empty() || count + public_kinds.len() > MAX_CLONE_KINDS {
        return false;
    }
    if !public_kinds.iter().enumerate().all(|(index, kind)| {
        *kind >= NATIVE_KIND_TERM as i32
            && position_of(*kind as usize).is_none()
            && !public_kinds[..index].contains(kind)
    }) {
        return false;
    }
    for (index, kind) in public_kinds.iter().enumerate() {
        KINDS[count + index].store(*kind as usize, Ordering::Relaxed);
    }
    KIND_COUNT.store(count + public_kinds.len(), Ordering::Release);
    true
}

pub(crate) fn register(public_kind: i32) -> bool {
    register_family(&[public_kind])
}

fn position_of(kind: usize) -> Option<usize> {
    let count = KIND_COUNT.load(Ordering::Acquire);
    (0..count).find(|index| KINDS[*index].load(Ordering::Acquire) == kind)
}

fn cells_for(kind: usize) -> Option<*mut CloneParamCells> {
    let index = position_of(kind)?;
    Some(unsafe { core::ptr::addr_of_mut!(CELLS[index]) })
}

unsafe extern "C" fn loop_bound(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    if !LOOP_RAN.swap(true, Ordering::AcqRel) {
        crate::dbg_log_public("[itemparam] boot loop reached; extending over clone kinds");
    }
    let count = KIND_COUNT.load(Ordering::Acquire);
    if count == 0 {
        return;
    }
    let kind = ctx.registers[KIND_REGISTER].x() as usize;

    let next = if kind == NATIVE_KIND_TERM as usize {
        Some(KINDS[0].load(Ordering::Acquire))
    } else {
        position_of(kind.wrapping_sub(1)).map(|index| {
            if index + 1 < count {
                KINDS[index + 1].load(Ordering::Acquire)
            } else {
                NATIVE_KIND_TERM as usize
            }
        })
    };
    let Some(next) = next else {
        return;
    };

    if next == NATIVE_KIND_TERM as usize {
        item_slots::clear_forced_clone();
        report("loop-exit");
    } else {
        item_slots::force_clone(next as i32);
    }
    ctx.registers[KIND_REGISTER].set_x(next as u64);
}

unsafe extern "C" fn param_cell(ctx: &mut skyline::hooks::InlineCtx) {
    rebase_entry(ctx, PARAM_ENTRY_ARRAY, |cells| {
        core::ptr::addr_of_mut!((*cells).param_entry) as usize
    });
}

unsafe extern "C" fn duet_cell(ctx: &mut skyline::hooks::InlineCtx) {
    rebase_entry(ctx, DUET_ENTRY_ARRAY, |cells| {
        core::ptr::addr_of_mut!((*cells).duet_entry) as usize
    });
}

unsafe fn rebase_entry(
    ctx: &mut skyline::hooks::InlineCtx,
    array_offset: usize,
    cell: unsafe fn(*mut CloneParamCells) -> usize,
) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let kind = ctx.registers[KIND_REGISTER].x() as usize;
    if array_offset == PARAM_ENTRY_ARRAY {
        let base = (ctx.registers[CELL_REGISTER].x() as usize).wrapping_sub(kind * 8);
        ARRAY_BASE.store(base, Ordering::Release);
    }
    let Some(cells) = cells_for(kind) else {
        return;
    };
    ctx.registers[CELL_REGISTER].set_x(cell(cells).wrapping_sub(array_offset) as u64);
}

unsafe extern "C" fn word_cell(ctx: &mut skyline::hooks::InlineCtx) {
    rebase_word(ctx, WORD_REGISTER, |cells| {
        core::ptr::addr_of_mut!((*cells).word) as usize
    });
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let kind = ctx.registers[KIND_REGISTER].x() as usize;
    if let Some(index) = position_of(kind) {
        substitute_index(ctx, kind, index);
    }
}

unsafe extern "C" fn duet_word_cell(ctx: &mut skyline::hooks::InlineCtx) {
    rebase_word(ctx, DUET_WORD_REGISTER, |cells| {
        core::ptr::addr_of_mut!((*cells).duet_word) as usize
    });
}

unsafe fn rebase_word(
    ctx: &mut skyline::hooks::InlineCtx,
    register: usize,
    cell: unsafe fn(*mut CloneParamCells) -> usize,
) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let kind = ctx.registers[KIND_REGISTER].x() as usize;
    let Some(cells) = cells_for(kind) else {
        return;
    };
    ctx.registers[register].set_x(cell(cells) as u64);
}

pub(crate) fn report(stage: &str) {
    let count = KIND_COUNT.load(Ordering::Acquire);
    for index in 0..count {
        let kind = KINDS[index].load(Ordering::Acquire);
        let cells = unsafe { core::ptr::addr_of!(CELLS[index]) };
        let (param, duet, word, duet_word) = unsafe {
            (
                core::ptr::read_volatile(core::ptr::addr_of!((*cells).param_entry)),
                core::ptr::read_volatile(core::ptr::addr_of!((*cells).duet_entry)),
                core::ptr::read_volatile(core::ptr::addr_of!((*cells).word)),
                core::ptr::read_volatile(core::ptr::addr_of!((*cells).duet_word)),
            )
        };
        let (blob, tag, owner) = unsafe {
            if param == 0 {
                (0usize, 0u8, 0i32)
            } else {
                let blob = core::ptr::read_volatile((param + 8) as *const usize);
                (
                    blob,
                    if blob == 0 {
                        0
                    } else {
                        core::ptr::read_volatile(blob as *const u8)
                    },
                    core::ptr::read_volatile((param + 0x10) as *const i32),
                )
            }
        };
        let base_kind = crate::item_clones::clone_engine_item_base_kind(kind as i32);
        let array_base = ARRAY_BASE.load(Ordering::Acquire);
        if array_base != 0 && base_kind != kind as i32 {
            unsafe {
                let native = core::ptr::read_volatile(
                    (array_base + base_kind as usize * 8 + PARAM_ENTRY_ARRAY) as *const usize,
                );
                if native != 0 {
                    let at =
                        |offset: usize| core::ptr::read_volatile((native + offset) as *const usize);
                    let root = at(0x00);
                    let blob = at(0x08);
                    let tag = if blob == 0 {
                        0
                    } else {
                        core::ptr::read_volatile(blob as *const u8)
                    };
                    crate::dbg_log_public(&format!(
                        "[itemparam] {stage} TEMPLATE base {base_kind:#x} entry={native:#x}                          root={root:#x}{} blob={blob:#x} tag={tag:#x} +0x10={:#x} +0x18={:#x}                          +0x20={:#x} +0x28={:#x}",
                        if root == native { " (self)" } else { "" },
                        core::ptr::read_volatile((native + 0x10) as *const u32),
                        at(0x18),
                        at(0x20),
                        at(0x28),
                    ));
                }
            }
        }
        crate::dbg_log_public(&format!(
            "[itemparam] {stage} clone {kind:#x}: param_entry={param:#x} blob={blob:#x} tag={tag:#x} \
             owner={owner:#x} duet_entry={duet:#x} word={word:#x} duet_word={duet_word:#x}              loaded={} | {}",
            tag == 0x0C,
            unsafe { readiness(word) },
        ));
    }
}

unsafe extern "C" fn path_probe(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let kind = ctx.registers[KIND_REGISTER].x() as usize;
    if !probe_kind(kind) {
        return;
    }
    let start = (ctx.registers[PATH_STRING_REGISTER].x() as usize).wrapping_sub(1);
    if start == 0 {
        return;
    }
    let mut text = String::new();
    for step in 0..96usize {
        let byte = core::ptr::read_volatile((start + step) as *const u8);
        if byte == 0 {
            break;
        }
        text.push(byte as char);
    }
    crate::dbg_log_public(&format!("[itemparam] kind {kind:#x} built {text:?}"));
}

unsafe extern "C" fn category_root(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let public_kind = ctx.registers[KIND_REGISTER].x() as usize;
    if position_of(public_kind).is_none() {
        return;
    }
    let category = item_content_category(public_kind as i32);
    ctx.registers[CATEGORY_ROOT_REGISTER]
        .set_x(crate::game::addr(category.main_root_offset()) as u64);
}

unsafe extern "C" fn hash_probe(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let kind = ctx.registers[KIND_REGISTER].x() as usize;
    if !probe_kind(kind) {
        return;
    }
    let hash = ctx.registers[HASH_REGISTER].x();
    if let Some(index) = position_of(kind) {
        LAST_HASH[index].store(hash, Ordering::Release);
    }
    crate::dbg_log_public(&format!(
        "[itemparam] kind {kind:#x} hash40 {hash:#x} (len {}, crc {:#010x})",
        hash >> 32,
        hash as u32
    ));
}

unsafe extern "C" fn lookup_probe(ctx: &mut skyline::hooks::InlineCtx) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    let kind = ctx.registers[KIND_REGISTER].x() as usize;
    if !probe_kind(kind) {
        return;
    }
    let index = ctx.registers[LOOKUP_RESULT_REGISTER].x() as u32;
    crate::dbg_log_public(&format!(
        "[itemparam] kind {kind:#x} lookup -> {index:#x} ({})",
        if index == RESOURCE_NONE {
            "not found"
        } else {
            "found"
        }
    ));
}

const ARC_SERVICE_GLOBAL: usize = 0x5331F20;

pub(crate) unsafe fn scan_file_path_index(hash: u64) -> Option<u32> {
    crate::arc_files::file_path_index(hash)
}

unsafe fn substitute_index(ctx: &mut skyline::hooks::InlineCtx, kind: usize, index: usize) {
    let output = (ctx.registers[FRAME_REGISTER].x() as usize).wrapping_sub(RESOLVER_OUTPUT_OFFSET)
        as *mut u32;
    if core::ptr::read_volatile(output) != RESOURCE_NONE {
        return;
    }
    let hash = LAST_HASH[index].load(Ordering::Acquire);
    if hash == 0 {
        return;
    }
    let cached = CACHED_INDEX[index].load(Ordering::Acquire);
    let scanned = if cached == usize::MAX {
        let found = scan_file_path_index(hash);
        CACHED_INDEX[index].store(
            found.map(|value| value as usize).unwrap_or(usize::MAX - 1),
            Ordering::Release,
        );
        found
    } else if cached == usize::MAX - 1 {
        None
    } else {
        Some(cached as u32)
    };
    let Some(found) = scanned else {
        if !SUBST_REPORTED[index].swap(true, Ordering::AcqRel) {
            crate::dbg_log_public(&format!(
                "[itemparam] clone {kind:#x} hash {hash:#x} is not in the file table either;                  the pack does not declare it"
            ));
        }
        return;
    };
    core::ptr::write_volatile(output, found);
    if !SUBST_REPORTED[index].swap(true, Ordering::AcqRel) {
        crate::dbg_log_public(&format!(
            "[itemparam] clone {kind:#x} hash {hash:#x} -> FilePath {found:#x} (scanned;              the resolver cannot see appended paths)"
        ));
    }
}

unsafe fn readiness(index: u32) -> String {
    if index == RESOURCE_NONE {
        return "no resource index".into();
    }
    let service =
        core::ptr::read_volatile((crate::game::addr(ARC_SERVICE_GLOBAL)) as *const usize);
    if service == 0 {
        return "resource service is null".into();
    }
    let count = core::ptr::read_volatile((service + 0x18) as *const u32);
    if count <= index {
        return format!("index {index:#x} is past the resource table (count {count:#x})");
    }
    let table = core::ptr::read_volatile((service + 0x08) as *const usize);
    if table == 0 {
        return "resource table is null".into();
    }
    let slot = table + index as usize * 8;
    let ready = core::ptr::read_volatile((slot + 4) as *const u8);
    if ready == 0 {
        return format!("index {index:#x} is in range but not ready");
    }
    let second = core::ptr::read_volatile(slot as *const u32);
    if second == RESOURCE_NONE {
        return format!("ready, but its data index is unset");
    }
    let second_count = core::ptr::read_volatile((service + 0x1c) as *const u32);
    if second_count <= second {
        return format!("data index {second:#x} is past its table (count {second_count:#x})");
    }
    format!("ready, data index {second:#x}")
}

unsafe fn resource_data(index: u32) -> Option<usize> {
    if index == RESOURCE_NONE {
        return None;
    }
    let service =
        core::ptr::read_volatile((crate::game::addr(ARC_SERVICE_GLOBAL)) as *const usize);
    if service == 0 || core::ptr::read_volatile((service + 0x18) as *const u32) <= index {
        return None;
    }
    let table = core::ptr::read_volatile((service + 0x08) as *const usize);
    if table == 0 || core::ptr::read_volatile((table + index as usize * 8 + 4) as *const u8) == 0 {
        return None;
    }
    let data_index = core::ptr::read_volatile((table + index as usize * 8) as *const u32);
    if data_index == RESOURCE_NONE
        || core::ptr::read_volatile((service + 0x1c) as *const u32) <= data_index
    {
        return None;
    }
    let records = core::ptr::read_volatile((service + 0x10) as *const usize);
    if records == 0 {
        return None;
    }
    let record = records + data_index as usize * 0x18;
    let state = core::ptr::read_volatile((record + 0xd) as *const u8);
    let data = core::ptr::read_volatile(record as *const usize);
    LAST_STATE.store(((state as usize) << 1) | 1, Ordering::Release);
    LAST_DATA.store(data, Ordering::Release);
    if state != 3 {
        return None;
    }
    (data != 0).then_some(data)
}

const OFF_RESOURCE_REQUEST: usize = 0x3540450;

static REQUESTED: [AtomicBool; MAX_CLONE_KINDS] =
    [const { AtomicBool::new(false) }; MAX_CLONE_KINDS];

unsafe fn request_resource(index: usize, word: u32) {
    if word == RESOURCE_NONE {
        return;
    }
    let Some(cell) = REQUESTED.get(index) else {
        return;
    };
    if cell.load(Ordering::Acquire) {
        return;
    }
    let service =
        core::ptr::read_volatile((crate::game::addr(ARC_SERVICE_GLOBAL)) as *const usize);
    if service == 0 {
        return;
    }
    if core::ptr::read_volatile((service + 0x18) as *const u32) <= word {
        if !cell.swap(true, Ordering::AcqRel) {
            crate::dbg_log_public(&format!(
                "[itemparam] FilePath {word:#x} is past the resource service bound; not requested"
            ));
        }
        return;
    }
    if cell.swap(true, Ordering::AcqRel) {
        return;
    }
    let request: unsafe extern "C" fn(usize, u32) =
        core::mem::transmute(crate::game::addr(OFF_RESOURCE_REQUEST));
    request(service, word);
    crate::dbg_log_public(&format!(
        "[itemparam] requested FilePath {word:#x} through the game's own loader"
    ));
}

pub(crate) unsafe fn request_file_path(word: u32) {
    if word == RESOURCE_NONE {
        return;
    }
    let service =
        core::ptr::read_volatile((crate::game::addr(ARC_SERVICE_GLOBAL)) as *const usize);
    if service == 0 {
        return;
    }
    if core::ptr::read_volatile((service + 0x18) as *const u32) <= word {
        return;
    }
    let request: unsafe extern "C" fn(usize, u32) =
        core::mem::transmute(crate::game::addr(OFF_RESOURCE_REQUEST));
    request(service, word);
}

pub(crate) unsafe fn resource_is_resident(index: u32) -> bool {
    resource_data(index).is_some()
}

static LAST_STATE: AtomicUsize = AtomicUsize::new(0);
static LAST_DATA: AtomicUsize = AtomicUsize::new(0);
static STALL_REPORTED: [AtomicBool; MAX_CLONE_KINDS] =
    [const { AtomicBool::new(false) }; MAX_CLONE_KINDS];

mod arcropolis {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static GET_SIZE: AtomicUsize = AtomicUsize::new(usize::MAX);
    static LOAD_FILE: AtomicUsize = AtomicUsize::new(usize::MAX);

    unsafe fn resolve(cache: &AtomicUsize, name: &[u8]) -> Option<usize> {
        debug_assert!(
            name.last() == Some(&0),
            "symbol name must be NUL-terminated"
        );
        let cached = cache.load(Ordering::Acquire);
        if cached != usize::MAX && cached != 0 {
            return Some(cached);
        }
        let found = crate::css_registration::lookup_symbol(name).unwrap_or(0);
        if found != 0 {
            cache.store(found, Ordering::Release);
        }
        (found != 0).then_some(found)
    }

    pub(super) unsafe fn decompressed_size(hash: u64) -> Result<usize, &'static str> {
        let Some(entry) = resolve(&GET_SIZE, b"arcrop_get_decompressed_size\0") else {
            return Err("arcrop_get_decompressed_size is not exported (yet)");
        };
        let f: extern "C" fn(u64, *mut usize) = core::mem::transmute(entry);
        let mut size = 0usize;
        f(hash, &mut size as *mut usize);
        if size == 0 {
            return Err(
                "arcrop_get_decompressed_size answered 0 - ARCropolis does not own this hash",
            );
        }
        if size >= 0x0400_0000 {
            return Err("arcrop_get_decompressed_size answered implausibly large");
        }
        Ok(size)
    }

    pub(super) unsafe fn load_file(hash: u64, buffer: &mut [u8]) -> Option<usize> {
        let f: extern "C" fn(u64, *mut u8, usize, *mut usize) =
            core::mem::transmute(resolve(&LOAD_FILE, b"arcrop_load_file\0")?);
        let mut written = 0usize;
        f(
            hash,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut written as *mut usize,
        );
        (written != 0 && written <= buffer.len()).then_some(written)
    }
}

pub(crate) unsafe fn read_own_file(hash: u64) -> Result<&'static [u8], String> {
    let size = arcropolis::decompressed_size(hash).map_err(str::to_owned)?;
    let mut buffer = vec![0u8; size];
    let Some(written) = arcropolis::load_file(hash, &mut buffer) else {
        return Err("arcrop_load_file wrote nothing".to_owned());
    };
    buffer.truncate(written);
    Ok(Vec::leak(buffer))
}

static OWNED: [AtomicUsize; MAX_CLONE_KINDS] = [const { AtomicUsize::new(0) }; MAX_CLONE_KINDS];

unsafe fn load_own_param(index: usize, hash: u64) -> Option<usize> {
    let existing = OWNED[index].load(Ordering::Acquire);
    if existing != 0 {
        return Some(existing);
    }
    let why = |reason: &str| {
        static SAID: [AtomicBool; MAX_CLONE_KINDS] =
            [const { AtomicBool::new(false) }; MAX_CLONE_KINDS];
        if let Some(cell) = SAID.get(index) {
            if !cell.swap(true, Ordering::AcqRel) {
                crate::dbg_log_public(&format!(
                    "[itemparam] ARCropolis load for hash {hash:#x} unavailable: {reason}"
                ));
            }
        }
    };
    let size = match arcropolis::decompressed_size(hash) {
        Ok(size) => size,
        Err(reason) => {
            why(reason);
            return None;
        }
    };
    let mut buffer = vec![0u8; size];
    let Some(written) = arcropolis::load_file(hash, &mut buffer) else {
        why("arcrop_load_file wrote nothing");
        return None;
    };
    if written < 0x10 {
        why("file is too short to be a prc");
        return None;
    }
    let data = Box::leak(buffer.into_boxed_slice()).as_ptr() as usize;
    OWNED[index].store(data, Ordering::Release);
    Some(data)
}

unsafe fn fill_entry(entry: usize, data: usize, index: u32) -> Result<usize, &'static str> {
    let hash_size = core::ptr::read_volatile((data + 8) as *const u32) as usize;
    let ref_size = core::ptr::read_volatile((data + 0xC) as *const u32) as usize;
    if hash_size > 0x0100_0000 || ref_size > 0x0100_0000 {
        return Err("implausible prc header");
    }
    let root = data + 0x10 + hash_size + ref_size;
    if core::ptr::read_volatile(root as *const u8) != 0x0C {
        return Err("root node is not a struct");
    }
    core::ptr::write_volatile((entry + 0x18) as *mut usize, data);
    core::ptr::write_volatile((entry + 0x20) as *mut usize, data + 0x10);
    core::ptr::write_volatile((entry + 0x28) as *mut usize, data + 0x10 + hash_size);
    core::ptr::write_volatile((entry + 0x10) as *mut u32, index);
    core::ptr::write_volatile((entry + 0x08) as *mut usize, root);
    Ok(root)
}

unsafe fn dump_params(label: &str, entry: usize, limit: usize) {
    if entry == 0 {
        return;
    }
    let root = core::ptr::read_volatile((entry + 0x08) as *const usize);
    let hashes = core::ptr::read_volatile((entry + 0x20) as *const usize);
    let refs = core::ptr::read_volatile((entry + 0x28) as *const usize);
    if root == 0 || hashes == 0 || refs == 0 {
        return;
    }
    if core::ptr::read_volatile(root as *const u8) != 0x0C {
        return;
    }
    let read_u32 = |at: usize| -> u32 {
        let mut value = 0u32;
        core::ptr::copy_nonoverlapping(at as *const u8, &mut value as *mut u32 as *mut u8, 4);
        value
    };
    let count = read_u32(root + 1) as usize;
    let offset = read_u32(root + 5) as i32 as isize;
    if count == 0 || count > 0x1000 {
        return;
    }
    let table = (refs as isize).wrapping_add(offset) as usize;
    let mut text = String::new();
    for index in 0..count.min(limit) {
        let child = table + index * 8;
        let hash_index = read_u32(child) as usize;
        let value_offset = read_u32(child + 4) as usize;
        let hash = core::ptr::read_volatile((hashes + hash_index * 8) as *const u64);
        let node = root + value_offset;
        let kind = core::ptr::read_volatile(node as *const u8);
        let value = match kind {
            8 => format!("{}", f32::from_bits(read_u32(node + 1))),
            6 | 7 => format!("{}", read_u32(node + 1)),
            1 => format!("{}", core::ptr::read_volatile((node + 1) as *const u8) != 0),
            other => format!("<type {other}>"),
        };
        text.push_str(&format!(" {hash:#x}={value}"));
    }
    crate::dbg_log_public(&format!(
        "[itemparam] {label} params ({count} total):{text}"
    ));
}

unsafe fn publish_stand_ins(cells: *mut CloneParamCells, kind: usize) {
    let entry = core::ptr::addr_of_mut!((*cells).param_entry) as usize;
    let rebase = |for_kind: usize| {
        entry
            .wrapping_sub(for_kind * 8)
            .wrapping_sub(PARAM_ENTRY_ARRAY)
    };
    let base_kind = crate::item_clones::clone_engine_item_base_kind(kind as i32);
    if base_kind >= 0 {
        core::ptr::write_volatile(
            core::ptr::addr_of_mut!((*cells).accessor_cell_base),
            rebase(base_kind as usize),
        );
    }
    core::ptr::write_volatile(
        core::ptr::addr_of_mut!((*cells).accessor_cell),
        rebase(kind),
    );
}

static FILLED: [AtomicBool; MAX_CLONE_KINDS] = [const { AtomicBool::new(false) }; MAX_CLONE_KINDS];
static FILLED_ENTRY: [AtomicUsize; MAX_CLONE_KINDS] =
    [const { AtomicUsize::new(0) }; MAX_CLONE_KINDS];
fn settle_fill(index: usize, entry: usize) {
    FILLED_ENTRY[index].store(entry, Ordering::Release);
    FILLED[index].store(true, Ordering::Release);
}

fn reopen_fill(index: usize) {
    FILLED[index].store(false, Ordering::Release);
    REQUESTED[index].store(false, Ordering::Release);
    STALL_REPORTED[index].store(false, Ordering::Release);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FillState {
    Unchanged,
    Fresh,
    Moved,
}

fn fill_state(filled: bool, previous: usize, entry: usize) -> FillState {
    if previous == entry {
        return FillState::Unchanged;
    }
    if filled {
        FillState::Moved
    } else {
        FillState::Fresh
    }
}

fn revalidate_fill(index: usize, kind: usize, entry: usize) {
    let previous = FILLED_ENTRY[index].load(Ordering::Acquire);
    let filled = FILLED[index].load(Ordering::Acquire);
    match fill_state(filled, previous, entry) {
        FillState::Unchanged => {}
        FillState::Fresh => {
            FILLED_ENTRY[index].store(entry, Ordering::Release);
            reopen_fill(index);
        }
        FillState::Moved => {
            FILLED_ENTRY[index].store(entry, Ordering::Release);
            reopen_fill(index);
            crate::dbg_log_public(&format!(
                "[itemparam] clone {kind:#x} entry moved {previous:#x} -> {entry:#x}; refilling"
            ));
        }
    }
}

#[cfg(test)]
mod refill_tests {
    use super::{fill_state, reopen_fill, settle_fill, FillState, FILLED, FILLED_ENTRY, REQUESTED,
        STALL_REPORTED};
    use core::sync::atomic::Ordering;

    #[test]
    fn the_same_entry_holds_its_fill() {
        assert_eq!(fill_state(true, 0x1000, 0x1000), FillState::Unchanged);
        assert_eq!(fill_state(false, 0x1000, 0x1000), FillState::Unchanged);
    }

    #[test]
    fn a_reallocated_entry_is_refilled_even_though_the_kind_was_filled_once() {
        assert_eq!(
            fill_state(true, 0x11DF8435C0, 0x11DF833690),
            FillState::Moved
        );
    }

    #[test]
    fn an_entry_cleared_to_zero_is_reported_as_moved_and_never_written_to() {
        assert_eq!(fill_state(true, 0x11DF8435C0, 0), FillState::Moved);
    }

    #[test]
    fn the_first_entry_of_a_boot_is_adopted_quietly() {
        assert_eq!(fill_state(false, 0, 0x1000), FillState::Fresh);
    }

    #[test]
    fn settling_records_the_entry_it_filled() {
        let index = 200;
        settle_fill(index, 0x11DF8435C0);
        assert!(FILLED[index].load(Ordering::Acquire));
        assert_eq!(FILLED_ENTRY[index].load(Ordering::Acquire), 0x11DF8435C0);
    }

    #[test]
    fn reopening_clears_the_request_and_the_stall_report() {
        let index = 201;
        REQUESTED[index].store(true, Ordering::Release);
        STALL_REPORTED[index].store(true, Ordering::Release);
        FILLED[index].store(true, Ordering::Release);
        reopen_fill(index);
        assert!(!FILLED[index].load(Ordering::Acquire));
        assert!(!REQUESTED[index].load(Ordering::Acquire));
        assert!(!STALL_REPORTED[index].load(Ordering::Acquire));
    }
}

pub(crate) fn try_fill() {
    let count = KIND_COUNT.load(Ordering::Acquire);
    for index in 0..count {
        let kind = KINDS[index].load(Ordering::Acquire);
        unsafe {
            let cells = core::ptr::addr_of_mut!(CELLS[index]);
            let entry = core::ptr::read_volatile(core::ptr::addr_of!((*cells).param_entry));
            revalidate_fill(index, kind, entry);
            if FILLED[index].load(Ordering::Acquire) {
                continue;
            }
            let word = core::ptr::read_volatile(core::ptr::addr_of!((*cells).word));
            LAST_STATE.store(0, Ordering::Release);
            let resident = if entry == 0 {
                None
            } else {
                resource_data(word)
            };
            if resident.is_none() && entry != 0 {
                request_resource(index, word);
            }
            let own = if resident.is_none() && entry != 0 {
                load_own_param(index, LAST_HASH[index].load(Ordering::Acquire))
            } else {
                None
            };
            if let Some(data) = own {
                if let Ok(root) = fill_entry(entry, data, word) {
                    publish_stand_ins(cells, kind);
                    settle_fill(index, entry);
                    crate::dbg_log_public(&format!(
                        "[itemparam] clone {kind:#x} FILLED via ARCropolis entry={entry:#x}                          data={data:#x} root={root:#x}"
                    ));
                    dump_params(&format!("clone {kind:#x}"), entry, 6);
                    continue;
                }
            }
            let Some(data) = resident else {
                if !STALL_REPORTED[index].swap(true, Ordering::AcqRel) {
                    let seen = LAST_STATE.load(Ordering::Acquire);
                    crate::dbg_log_public(&format!(
                        "[itemparam] clone {kind:#x} waiting: entry={entry:#x} FilePath={word:#x}                          {}",
                        if seen & 1 == 0 {
                            "did not reach the record at all".to_string()
                        } else {
                            format!(
                                "record state={:#x} (needs 3) data={:#x}",
                                seen >> 1,
                                LAST_DATA.load(Ordering::Acquire)
                            )
                        }
                    ));
                }
                continue;
            };
            match fill_entry(entry, data, word) {
                Ok(root) => {
                    publish_stand_ins(cells, kind);
                    settle_fill(index, entry);
                    crate::dbg_log_public(&format!(
                        "[itemparam] clone {kind:#x} FILLED entry={entry:#x} data={data:#x}                          root={root:#x} from FilePath {word:#x}"
                    ));
                    dump_params(&format!("clone {kind:#x}"), entry, 6);
                    let base_kind = crate::item_clones::clone_engine_item_base_kind(kind as i32);
                    let array_base = ARRAY_BASE.load(Ordering::Acquire);
                    if array_base != 0 && base_kind != kind as i32 {
                        let native = core::ptr::read_volatile(
                            (array_base + base_kind as usize * 8 + PARAM_ENTRY_ARRAY)
                                as *const usize,
                        );
                        dump_params(&format!("base  {base_kind:#x}"), native, 6);
                    }
                }
                Err(why) => {
                    settle_fill(index, entry);
                    crate::dbg_log_public(&format!(
                        "[itemparam] clone {kind:#x} NOT filled ({why}) entry={entry:#x}                          data={data:#x}"
                    ));
                }
            }
        }
    }
}

const MAX_RUNTIME_SCOPES: usize = 8;
static SCOPE_THREAD: [AtomicUsize; MAX_RUNTIME_SCOPES] =
    [const { AtomicUsize::new(0) }; MAX_RUNTIME_SCOPES];
static SCOPE_PUBLIC: [AtomicI32; MAX_RUNTIME_SCOPES] =
    [const { AtomicI32::new(-1) }; MAX_RUNTIME_SCOPES];
static SCOPE_BASE: [AtomicI32; MAX_RUNTIME_SCOPES] =
    [const { AtomicI32::new(-1) }; MAX_RUNTIME_SCOPES];

unsafe fn current_thread() -> usize {
    skyline::nn::os::GetCurrentThread() as usize
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommonTable {
    Float,
    Word,
    Hash,
}

impl CommonTable {
    fn core(self) -> Table {
        match self {
            CommonTable::Float => Table::Float,
            CommonTable::Word => Table::Word,
            CommonTable::Hash => Table::Hash,
        }
    }

    fn tag(self) -> u32 {
        match self {
            CommonTable::Float => 0,
            CommonTable::Word => 1,
            CommonTable::Hash => 2,
        }
    }

    fn from_tag(tag: u32) -> CommonTable {
        match tag {
            1 => CommonTable::Word,
            2 => CommonTable::Hash,
            _ => CommonTable::Float,
        }
    }
}

const SAVE_TABLE_SHIFT: u32 = 28;
const SAVE_OFFSET_MASK: u32 = (1 << SAVE_TABLE_SHIFT) - 1;

struct CommonOverride {
    public_kind: i32,
    table: CommonTable,
    offset: u32,
    bits: u64,
    mul: bool,
}

fn common_overrides() -> &'static RwLock<Vec<CommonOverride>> {
    static OVERRIDES: OnceLock<RwLock<Vec<CommonOverride>>> = OnceLock::new();
    OVERRIDES.get_or_init(|| RwLock::new(Vec::new()))
}

const MAX_COMMON_SAVES: usize = 192;
static SAVE_OFFSET: [[AtomicU32; MAX_COMMON_SAVES]; MAX_RUNTIME_SCOPES] =
    [const { [const { AtomicU32::new(u32::MAX) }; MAX_COMMON_SAVES] }; MAX_RUNTIME_SCOPES];
static SAVE_VALUE: [[AtomicU64; MAX_COMMON_SAVES]; MAX_RUNTIME_SCOPES] =
    [const { [const { AtomicU64::new(0) }; MAX_COMMON_SAVES] }; MAX_RUNTIME_SCOPES];
static SAVE_COUNT: [AtomicUsize; MAX_RUNTIME_SCOPES] =
    [const { AtomicUsize::new(0) }; MAX_RUNTIME_SCOPES];

struct OwnerOverride {
    public_kind: i32,
    owner_kind: i32,
    offset: u32,
    bits: u32,
    mul: bool,
    reported: AtomicBool,
}

fn owner_overrides() -> &'static RwLock<Vec<OwnerOverride>> {
    static OVERRIDES: OnceLock<RwLock<Vec<OwnerOverride>>> = OnceLock::new();
    OVERRIDES.get_or_init(|| RwLock::new(Vec::new()))
}

const MAX_OWNER_SAVES: usize = 64;
pub(crate) const OWNER_PAYLOAD_BYTES: u32 = 0x4000;
static OWNER_SAVE_ADDRESS: [[AtomicUsize; MAX_OWNER_SAVES]; MAX_RUNTIME_SCOPES] =
    [const { [const { AtomicUsize::new(0) }; MAX_OWNER_SAVES] }; MAX_RUNTIME_SCOPES];
static OWNER_SAVE_BITS: [[AtomicU32; MAX_OWNER_SAVES]; MAX_RUNTIME_SCOPES] =
    [const { [const { AtomicU32::new(0) }; MAX_OWNER_SAVES] }; MAX_RUNTIME_SCOPES];
static OWNER_SAVE_COUNT: [AtomicUsize; MAX_RUNTIME_SCOPES] =
    [const { AtomicUsize::new(0) }; MAX_RUNTIME_SCOPES];

pub(crate) fn owner_offset_valid(offset: u32) -> bool {
    offset % 4 == 0 && offset < OWNER_PAYLOAD_BYTES
}

pub(crate) fn register_owner_override(
    public_kind: i32,
    owner_kind: i32,
    offset: u32,
    bits: u32,
) -> bool {
    store_owner_override(public_kind, owner_kind, offset, bits, false)
}

pub(crate) fn register_owner_mul(public_kind: i32, owner_kind: i32, offset: u32, factor: f32) -> bool {
    let scalable = dionysus_core::owner_param_words::word_at(owner_kind, offset)
        .map_or(false, |word| word.kind != dionysus_core::owner_param_words::WordKind::Bool);
    scalable && store_owner_override(public_kind, owner_kind, offset, factor.to_bits(), true)
}

fn owner_word_is_float(owner_kind: i32, offset: u32) -> bool {
    dionysus_core::owner_param_words::word_at(owner_kind, offset)
        .map_or(false, |word| word.kind == dionysus_core::owner_param_words::WordKind::F32)
}

fn store_owner_override(public_kind: i32, owner_kind: i32, offset: u32, bits: u32, mul: bool) -> bool {
    if !owner_offset_valid(offset) {
        return false;
    }
    let Ok(mut overrides) = owner_overrides().write() else {
        return false;
    };
    match overrides.iter_mut().find(|e| {
        e.public_kind == public_kind && e.owner_kind == owner_kind && e.offset == offset
    }) {
        Some(existing) => {
            existing.bits = bits;
            existing.mul = mul;
        }
        None => overrides.push(OwnerOverride {
            public_kind,
            owner_kind,
            offset,
            bits,
            mul,
            reported: AtomicBool::new(false),
        }),
    }
    true
}

pub(crate) fn owner_targets(owner_kind: i32) -> Vec<(i32, u32, u32, bool)> {
    owner_overrides()
        .read()
        .map(|held| {
            held.iter()
                .filter(|e| e.owner_kind == owner_kind)
                .map(|e| (e.public_kind, e.offset, e.bits, e.mul))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn owner_value(owner_kind: i32, offset: u32, raw: u32, bits: u32, mul: bool) -> u32 {
    if mul {
        dionysus_core::row_fields::scaled_word(raw, owner_word_is_float(owner_kind, offset), f32::from_bits(bits))
    } else {
        bits
    }
}

unsafe fn apply_owner_overrides(scope: usize, public_kind: i32) {
    OWNER_SAVE_COUNT[scope].store(0, Ordering::Relaxed);
    let Ok(overrides) = owner_overrides().read() else {
        return;
    };
    if overrides.is_empty() {
        return;
    }
    let mut saved = 0usize;
    for entry in overrides.iter().filter(|e| e.public_kind == public_kind) {
        if saved >= MAX_OWNER_SAVES {
            break;
        }
        if !owner_offset_valid(entry.offset) {
            continue;
        }
        let Some(payload) = crate::fighter_params::payload_of(entry.owner_kind) else {
            continue;
        };
        let field = (payload + entry.offset as usize) as *mut u32;
        let before = core::ptr::read_volatile(field);
        let bits = owner_value(entry.owner_kind, entry.offset, before, entry.bits, entry.mul);
        OWNER_SAVE_ADDRESS[scope][saved].store(field as usize, Ordering::Relaxed);
        OWNER_SAVE_BITS[scope][saved].store(before, Ordering::Relaxed);
        core::ptr::write_volatile(field, bits);
        saved += 1;
        if !entry.reported.swap(true, Ordering::Relaxed) {
            crate::dbg_log_public(&format!(
                "[ownerparam] applied public={public_kind:#x} owner={} +{:#x}: {} -> {}",
                entry.owner_kind,
                entry.offset,
                dionysus_core::owner_param_words::describe(
                    entry.owner_kind,
                    entry.offset,
                    before
                ),
                dionysus_core::owner_param_words::word_at(entry.owner_kind, entry.offset)
                    .map(|word| word.shown(bits))
                    .unwrap_or_else(|| format!("{bits:#x}"))
            ));
        }
    }
    OWNER_SAVE_COUNT[scope].store(saved, Ordering::Release);
}

unsafe fn restore_owner_overrides(scope: usize) {
    let saved = OWNER_SAVE_COUNT[scope].swap(0, Ordering::AcqRel);
    for slot in 0..saved.min(MAX_OWNER_SAVES) {
        let address = OWNER_SAVE_ADDRESS[scope][slot].swap(0, Ordering::Relaxed);
        if address == 0 {
            continue;
        }
        let bits = OWNER_SAVE_BITS[scope][slot].load(Ordering::Relaxed);
        core::ptr::write_volatile(address as *mut u32, bits);
    }
}

pub(crate) fn owner_override_count(public_kind: i32) -> usize {
    owner_overrides()
        .read()
        .map(|held| held.iter().filter(|e| e.public_kind == public_kind).count())
        .unwrap_or(0)
}

pub(crate) unsafe fn common_cell(kind: i32, table: CommonTable) -> Option<*mut u8> {
    if kind < 0 || kind as u64 >= NATIVE_KIND_TERM {
        return None;
    }
    let handle = core::ptr::read_volatile(
        (crate::game::addr(crate::item_common_tables::COMMON_HANDLE_GLOBAL)) as *const usize,
    );
    if handle < 0x10_0000_0000 {
        return None;
    }
    let structure = core::ptr::read_volatile(handle as *const usize);
    if structure < 0x10_0000_0000 {
        return None;
    }
    Some((structure + table.core().cell(kind as usize, 0)) as *mut u8)
}

pub(crate) unsafe fn common_row(kind: i32) -> Option<*mut f32> {
    common_cell(kind, CommonTable::Float).map(|cell| cell as *mut f32)
}

unsafe fn read_cell(cell: *mut u8, table: CommonTable) -> u64 {
    match table.core().width() {
        8 => core::ptr::read_volatile(cell as *const u64),
        _ => u64::from(core::ptr::read_volatile(cell as *const u32)),
    }
}

unsafe fn write_cell(cell: *mut u8, table: CommonTable, bits: u64) {
    match table.core().width() {
        8 => core::ptr::write_volatile(cell as *mut u64, bits),
        _ => core::ptr::write_volatile(cell as *mut u32, bits as u32),
    }
}

pub(crate) fn common_field_offset(hash: u64) -> Option<u32> {
    let table = &crate::item_common_tables::ITEM_COMMON_FLOATS;
    table
        .binary_search_by(|(known, _)| known.cmp(&hash))
        .ok()
        .map(|index| table[index].1)
}

pub(crate) fn register_common_override(public_kind: i32, offset: u32, value: f32) -> bool {
    register_common_bits(public_kind, CommonTable::Float, offset, u64::from(value.to_bits()))
}

pub(crate) fn register_common_bits(
    public_kind: i32,
    table: CommonTable,
    offset: u32,
    bits: u64,
) -> bool {
    store_common_override(public_kind, table, offset, bits, false)
}

pub(crate) fn register_common_mul(public_kind: i32, table: CommonTable, offset: u32, factor: f32) -> bool {
    table != CommonTable::Hash && store_common_override(public_kind, table, offset, u64::from(factor.to_bits()), true)
}

fn store_common_override(public_kind: i32, table: CommonTable, offset: u32, bits: u64, mul: bool) -> bool {
    if offset > SAVE_OFFSET_MASK {
        return false;
    }
    let Ok(mut overrides) = common_overrides().write() else {
        return false;
    };
    match overrides
        .iter_mut()
        .find(|e| e.public_kind == public_kind && e.table == table && e.offset == offset)
    {
        Some(existing) => {
            existing.bits = bits;
            existing.mul = mul;
        }
        None => overrides.push(CommonOverride {
            public_kind,
            table,
            offset,
            bits,
            mul,
        }),
    }
    true
}

pub(crate) fn common_override_count(public_kind: i32) -> usize {
    common_overrides()
        .read()
        .map(|held| held.iter().filter(|e| e.public_kind == public_kind).count())
        .unwrap_or(0)
}

unsafe fn apply_common_overrides(scope: usize, public_kind: i32, base_kind: i32) {
    SAVE_COUNT[scope].store(0, Ordering::Relaxed);
    let Ok(overrides) = common_overrides().read() else {
        return;
    };
    if overrides.is_empty() {
        return;
    }
    let mut saved = 0usize;
    for entry in overrides.iter().filter(|e| e.public_kind == public_kind) {
        if saved >= MAX_COMMON_SAVES {
            break;
        }
        let Some(row) = common_cell(base_kind, entry.table) else {
            continue;
        };
        let field = row.add(entry.offset as usize);
        let was = read_cell(field, entry.table);
        SAVE_OFFSET[scope][saved].store(
            entry.offset | (entry.table.tag() << SAVE_TABLE_SHIFT),
            Ordering::Relaxed,
        );
        SAVE_VALUE[scope][saved].store(was, Ordering::Relaxed);
        let bits = if entry.mul {
            u64::from(dionysus_core::row_fields::scaled_word(
                was as u32,
                entry.table == CommonTable::Float,
                f32::from_bits(entry.bits as u32),
            ))
        } else {
            entry.bits
        };
        write_cell(field, entry.table, bits);
        static REPORTED: [AtomicBool; 6] = [const { AtomicBool::new(false) }; 6];
        let report = entry.table.tag() as usize + if entry.mul { 3 } else { 0 };
        if !REPORTED[report].swap(true, Ordering::AcqRel) {
            crate::dbg_log_public(&format!(
                "[itemcommon] OVERRIDE public={public_kind:#x} base={base_kind:#x} {} +{:#x}: {} -> {}",
                match entry.table {
                    CommonTable::Float => "float",
                    CommonTable::Word => "word",
                    CommonTable::Hash => "hash",
                },
                entry.offset,
                show_bits(entry.table, was),
                show_bits(entry.table, bits)
            ));
        }
        saved += 1;
    }
    SAVE_COUNT[scope].store(saved, Ordering::Release);
}

fn show_bits(table: CommonTable, bits: u64) -> String {
    match table {
        CommonTable::Float => format!("{}", f32::from_bits(bits as u32)),
        CommonTable::Word => format!("{}", bits as u32 as i32),
        CommonTable::Hash => format!("{bits:#x}"),
    }
}

unsafe fn restore_common_overrides(scope: usize, base_kind: i32) {
    let saved = SAVE_COUNT[scope].swap(0, Ordering::AcqRel);
    if saved == 0 {
        return;
    }
    for slot in 0..saved.min(MAX_COMMON_SAVES) {
        let packed = SAVE_OFFSET[scope][slot].load(Ordering::Relaxed);
        if packed == u32::MAX {
            continue;
        }
        let table = CommonTable::from_tag(packed >> SAVE_TABLE_SHIFT);
        let Some(row) = common_cell(base_kind, table) else {
            continue;
        };
        let bits = SAVE_VALUE[scope][slot].load(Ordering::Relaxed);
        write_cell(row.add((packed & SAVE_OFFSET_MASK) as usize), table, bits);
    }
}

pub(crate) fn enter_runtime_clone(public_kind: i32, base_kind: i32) -> Option<usize> {
    let thread = unsafe { current_thread() };
    if thread == 0 || public_kind < 0 || base_kind < 0 {
        return None;
    }
    for index in 0..MAX_RUNTIME_SCOPES {
        if SCOPE_THREAD[index].load(Ordering::Acquire) == thread {
            return None;
        }
    }
    for index in 0..MAX_RUNTIME_SCOPES {
        if SCOPE_THREAD[index]
            .compare_exchange(0, thread, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        SCOPE_PUBLIC[index].store(public_kind, Ordering::Relaxed);
        SCOPE_BASE[index].store(base_kind, Ordering::Relaxed);
        unsafe { apply_common_overrides(index, public_kind, base_kind) };
        unsafe { apply_owner_overrides(index, public_kind) };
        return Some(index);
    }
    None
}

pub(crate) fn leave_runtime_clone(index: usize) {
    if index >= MAX_RUNTIME_SCOPES {
        return;
    }
    unsafe { restore_owner_overrides(index) };
    unsafe { restore_common_overrides(index, SCOPE_BASE[index].load(Ordering::Relaxed)) };
    SCOPE_PUBLIC[index].store(-1, Ordering::Relaxed);
    SCOPE_BASE[index].store(-1, Ordering::Relaxed);
    SCOPE_THREAD[index].store(0, Ordering::Release);
}

pub(crate) fn runtime_clone_on_thread() -> Option<i32> {
    let thread = unsafe { current_thread() };
    if thread == 0 {
        return None;
    }
    (0..MAX_RUNTIME_SCOPES).find_map(|index| {
        (SCOPE_THREAD[index].load(Ordering::Acquire) == thread)
            .then(|| SCOPE_PUBLIC[index].load(Ordering::Relaxed))
            .filter(|public| *public >= 0)
    })
}

pub(crate) fn runtime_clone_for_base(base_kind: i32) -> Option<i32> {
    let thread = unsafe { current_thread() };
    if thread == 0 {
        return None;
    }
    (0..MAX_RUNTIME_SCOPES).find_map(|index| {
        (SCOPE_THREAD[index].load(Ordering::Acquire) == thread
            && SCOPE_BASE[index].load(Ordering::Relaxed) == base_kind)
            .then(|| SCOPE_PUBLIC[index].load(Ordering::Relaxed))
            .filter(|public| *public >= 0)
    })
}

const CALLER_FRAMES: usize = 12;

fn describe_address(address: usize) -> String {
    let text = crate::game::raw_text_base();
    if address > text && address - text < 0x0800_0000 {
        return format!("main+{:#x}", crate::game::offset_of(address));
    }
    let nro = crate::item_clones::item_nro_base();
    if nro != 0 && address > nro && address - nro < 0x0100_0000 {
        return format!("lua2cpp_item+{:#x}", address - nro);
    }
    format!("{address:#x} (unmapped)")
}

unsafe fn caller_chain(frame: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut frame = frame;
    for _ in 0..CALLER_FRAMES {
        if frame == 0 || frame % 16 != 0 {
            break;
        }
        let next = core::ptr::read_volatile(frame as *const usize);
        let link = core::ptr::read_volatile((frame + 8) as *const usize);
        if link != 0 {
            parts.push(describe_address(link));
        }
        if next <= frame {
            break;
        }
        frame = next;
    }
    parts.join(" <- ")
}

static CALLER_REPORTED: [AtomicBool; 4] = [const { AtomicBool::new(false) }; 4];
static GETTER_SEEN: [AtomicBool; MAX_CLONE_KINDS] =
    [const { AtomicBool::new(false) }; MAX_CLONE_KINDS];

unsafe fn getter_redirect(ctx: &mut skyline::hooks::InlineCtx, site: usize) {
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return;
    }
    try_fill();
    LIVE_ACCESSOR.store(ctx.registers[0].x() as usize, Ordering::Release);
    let mut kind = ctx.registers[GETTER_KIND_REGISTER].x() as u32 as i32;
    if (kind as u64) < NATIVE_KIND_TERM {
        if PROBE_KINDS.contains(&(kind as usize)) {
            let slot = PROBE_KINDS
                .iter()
                .position(|p| *p == kind as usize)
                .unwrap();
            if !CALLER_REPORTED[slot].swap(true, Ordering::AcqRel) {
                let lr = ctx.registers[30].x() as usize;
                let frame = ctx.registers[FRAME_REGISTER].x() as usize;
                crate::dbg_log_public(&format!(
                    "[itemparam] getter {:#x} kind {kind:#x} from {} | chain: {}",
                    GETTER_SITES[site].0,
                    describe_address(lr),
                    caller_chain(frame),
                ));
            }
        }
        match runtime_clone_for_base(kind) {
            Some(public) => kind = public,
            None => return,
        }
    }
    let Some(index) = position_of(kind as usize) else {
        return;
    };
    if !FILLED[index].load(Ordering::Acquire) {
        return;
    }
    let cells = core::ptr::addr_of_mut!(CELLS[index]);
    if core::ptr::read_volatile(core::ptr::addr_of!((*cells).accessor_cell)) == 0 {
        return;
    }
    let in_register = ctx.registers[GETTER_KIND_REGISTER].x() as u32 as i32;
    let stand_in = if in_register == kind {
        core::ptr::addr_of!((*cells).accessor_cell) as usize
    } else if in_register == crate::item_clones::clone_engine_item_base_kind(kind) {
        let base = core::ptr::addr_of!((*cells).accessor_cell_base);
        if core::ptr::read_volatile(base) == 0 {
            return;
        }
        base as usize
    } else {
        return;
    };
    let register = GETTER_SITES[site].2;
    ctx.registers[register].set_x(stand_in as u64);
    if !GETTER_SEEN[index].swap(true, Ordering::AcqRel) {
        crate::dbg_log_public(&format!(
            "[itemparam] getter {:#x} resolved to clone {kind:#x}; reading its own entry",
            GETTER_SITES[site].0
        ));
    }
}

pub(crate) fn getter_reads_safely(kind: i32) -> bool {
    if (0..NATIVE_KIND_TERM as i32).contains(&kind) {
        return true;
    }
    if !HOOKS_INSTALLED.load(Ordering::Acquire) {
        return false;
    }
    try_fill();
    let Some(index) = position_of(kind as usize) else {
        return false;
    };
    let cells = unsafe { core::ptr::addr_of_mut!(CELLS[index]) };
    FILLED[index].load(Ordering::Acquire)
        && unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*cells).accessor_cell)) } != 0
}

macro_rules! getter_hook {
    ($name:ident, $site:expr) => {
        unsafe extern "C" fn $name(ctx: &mut skyline::hooks::InlineCtx) {
            getter_redirect(ctx, $site);
        }
    };
}
getter_hook!(getter_0, 0);
getter_hook!(getter_1, 1);
getter_hook!(getter_2, 2);
getter_hook!(getter_3, 3);
getter_hook!(getter_4, 4);
getter_hook!(getter_5, 5);
getter_hook!(getter_6, 6);
getter_hook!(getter_7, 7);
getter_hook!(getter_8, 8);
getter_hook!(getter_9, 9);
const GETTER_HOOKS: &[InlineHook] = &[
    getter_0, getter_1, getter_2, getter_3, getter_4, getter_5, getter_6, getter_7, getter_8,
    getter_9,
];
const _: () = assert!(GETTER_HOOKS.len() == GETTER_SITES.len());

unsafe fn preflight() -> Result<(), (usize, u32, u32)> {
    for (offset, expected) in [
        (CATEGORY_ROOT_SITE, CATEGORY_ROOT_EXPECTED),
        (PATH_PROBE_SITE, PATH_PROBE_EXPECTED),
        (LOOKUP_PROBE_SITE, LOOKUP_PROBE_EXPECTED),
        (HASH_PROBE_SITE, HASH_PROBE_EXPECTED),
        (WORD_CELL_SITE, WORD_CELL_EXPECTED),
        (DUET_WORD_CELL_SITE, DUET_WORD_CELL_EXPECTED),
        (PARAM_CELL_SITE, PARAM_CELL_EXPECTED),
        (DUET_CELL_SITE, DUET_CELL_EXPECTED),
        (LOOP_BOUND_SITE, LOOP_BOUND_EXPECTED),
    ] {
        let actual = text_word(offset);
        if actual != expected {
            return Err((offset, expected, actual));
        }
    }
    for (offset, expected, _) in GETTER_SITES {
        let actual = text_word(*offset);
        if actual != *expected {
            return Err((*offset, *expected, actual));
        }
    }
    Ok(())
}

pub(crate) fn install() {
    match unsafe { preflight() } {
        Ok(()) => unsafe {
            PREFLIGHT_OK.store(true, Ordering::Release);
            let mut installed = 0usize;
            let mut failed: Vec<usize> = Vec::new();
            let mut arm = |offset: usize, original: u32, hook: InlineHook| {
                skyline::hooks::A64InlineHook(
                    crate::game::addr(offset) as *const libc::c_void,
                    hook as *const () as *const libc::c_void,
                );
                if text_word(offset) != original {
                    installed += 1;
                } else {
                    failed.push(offset);
                }
            };
            arm(WORD_CELL_SITE, WORD_CELL_EXPECTED, word_cell);
            arm(DUET_WORD_CELL_SITE, DUET_WORD_CELL_EXPECTED, duet_word_cell);
            arm(PARAM_CELL_SITE, PARAM_CELL_EXPECTED, param_cell);
            arm(DUET_CELL_SITE, DUET_CELL_EXPECTED, duet_cell);
            arm(LOOP_BOUND_SITE, LOOP_BOUND_EXPECTED, loop_bound);
            arm(CATEGORY_ROOT_SITE, CATEGORY_ROOT_EXPECTED, category_root);
            arm(PATH_PROBE_SITE, PATH_PROBE_EXPECTED, path_probe);
            arm(LOOKUP_PROBE_SITE, LOOKUP_PROBE_EXPECTED, lookup_probe);
            arm(HASH_PROBE_SITE, HASH_PROBE_EXPECTED, hash_probe);
            for (site, hook) in GETTER_SITES.iter().zip(GETTER_HOOKS) {
                arm(site.0, site.1, *hook);
            }

            if failed.is_empty() {
                HOOKS_INSTALLED.store(true, Ordering::Release);
                crate::dbg_log_public(&format!(
                    "[itemparam] installed {installed} hooks, {} clone kinds registered",
                    KIND_COUNT.load(Ordering::Acquire)
                ));
            } else {
                crate::dbg_log_public(&format!(
                    "[itemparam] DISARMED: {} of 19 hooks failed to relocate ({:#x?})",
                    failed.len(),
                    failed
                ));
            }
        },
        Err((offset, expected, actual)) => {
            crate::dbg_log_public(&format!(
                "[itemparam] preflight failed at {offset:#x}: expected {expected:#010x}, \
                 found {actual:#010x} - parameter loading stays vanilla"
            ));
        }
    }
}

pub(crate) fn ready() -> bool {
    PREFLIGHT_OK.load(Ordering::Acquire) && HOOKS_INSTALLED.load(Ordering::Acquire)
}

const COMMON_ACCESSOR_GLOBAL: usize = 0x52C31E0;
const COMMON_GATE_ARRAY: usize = 0xEF8;
const COMMON_ROW_ARRAY: usize = 0x73188;
const COMMON_ROW_STRIDE: usize = 0x64;
const COMMON_ROW_FLOATS: usize = COMMON_ROW_STRIDE / 4;

pub(crate) fn report_common_params() {
    static DONE: AtomicBool = AtomicBool::new(false);
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    if DONE.load(Ordering::Acquire) {
        return;
    }
    let call = CALLS.fetch_add(1, Ordering::Relaxed);
    if call < 600 || call % 120 != 0 {
        return;
    }
    let attempt = (call - 600) / 120;
    if attempt > 12 {
        if !DONE.swap(true, Ordering::AcqRel) {
            crate::dbg_log_public("[itemcommon] gave up: +0x28 never populated");
        }
        return;
    }
    unsafe {
        let global = crate::game::addr(COMMON_ACCESSOR_GLOBAL);
        let accessor = core::ptr::read_volatile(global as *const usize);
        let live = LIVE_ACCESSOR.load(Ordering::Acquire);
        crate::dbg_log_public(&format!(
            "[itemcommon] accessor {accessor:#x} (global {global:#x}) PHASE5 getter x0={live:#x} agree={}",
            accessor == live || live == 0
        ));
        if accessor < 0x1000_0000 {
            crate::dbg_log_public("[itemcommon] accessor not readable; nothing probed");
            return;
        }
        const ROW_STRIDE: usize = 0x448;
        const FIELDS: [usize; 3] = [0xac, 0xd0, 0xe0];
        const KS_ROW: [f32; 3] = [12.0, 9.1, 0.45];
        const DS_ROW: [f32; 3] = [11.0, 11.3, 0.0];
        const KILLSWORD: [f32; 5] = [9.55, 0.45, 9.1, -4.5, 1.4];
        const DEATHSCYTHE: [f32; 4] = [11.3, -6.0, 1.3, 15.0];
        let heap = |p: usize| (0x10_0000_0000..0x100_0000_0000).contains(&p) && p % 8 == 0;
        const ACCESSOR_SPAN: usize = 0x82000;
        const TARGET_SPAN: usize = 0x200;
        let marks = |p: usize| -> (u32, u32, bool) {
            let (mut ks, mut ds) = (0u32, 0u32);
            let magic = core::ptr::read_unaligned(p as *const [u8; 8]) == *b"paracobn";
            for step in 0..TARGET_SPAN {
                let value = core::ptr::read_unaligned((p + step) as *const f32);
                if KILLSWORD.iter().any(|w| (value - *w).abs() < 0.0005) {
                    ks += 1;
                }
                if DEATHSCYTHE.iter().any(|w| (value - *w).abs() < 0.0005) {
                    ds += 1;
                }
            }
            (ks, ds, magic)
        };
        let row_matches = |base: usize, kind: usize, want: &[f32; 3]| -> u32 {
            let row = base + kind * ROW_STRIDE;
            let mut n = 0;
            for (i, offset) in FIELDS.iter().enumerate() {
                let value = core::ptr::read_volatile((row + offset) as *const f32);
                if (value - want[i]).abs() < 0.002 {
                    n += 1;
                }
            }
            n
        };
        let real = core::ptr::read_volatile(accessor as *const usize);
        let expected_vtable = crate::game::addr(0x5077D30);
        let vtable = if heap(real) {
            core::ptr::read_volatile((real + 0x20) as *const usize)
        } else {
            0
        };
        crate::dbg_log_public(&format!(
            "[itemcommon] handle {accessor:#x} -> real {real:#x}; +0x20={vtable:#x} expected={expected_vtable:#x} match={}",
            vtable == expected_vtable
        ));
        if !heap(real) {
            crate::dbg_log_public("[itemcommon] real structure not readable");
            return;
        }
        let base = core::ptr::read_volatile((real + 0x28) as *const usize);
        {
            const PACKED_BASE: usize = 0x3914;
            const PACKED_STRIDE: usize = 0x284;
            for kind in [0x3Fusize, 0x40] {
                let row = real + PACKED_BASE + kind * PACKED_STRIDE;
                let mut line = String::new();
                for index in 0..PACKED_STRIDE / 4 {
                    if index % 16 == 0 && !line.is_empty() {
                        crate::dbg_log_public(&format!("[packed {kind:#x}]{line}"));
                        line.clear();
                    }
                    if index % 16 == 0 {
                        line.push_str(&format!(" +{:#05x}:", index * 4));
                    }
                    line.push_str(&format!(
                        " {:.4}",
                        core::ptr::read_volatile((row + index * 4) as *const f32)
                    ));
                }
                if !line.is_empty() {
                    crate::dbg_log_public(&format!("[packed {kind:#x}]{line}"));
                }
            }
        }

        {
            const KS_MARKS: [f32; 4] = [9.55, 0.45, 9.1, -4.5];
            const STRUCT_SIZE: usize = 0x81E08;
            let mut hits = 0usize;
            let mut window_start = 0usize;
            let mut window_marks = 0u32;
            for step in 0..STRUCT_SIZE / 4 {
                let at = real + step * 4;
                let value = core::ptr::read_volatile(at as *const f32);
                if KS_MARKS.iter().any(|w| (value - *w).abs() < 0.002) {
                    if step * 4 >= window_start + 0x448 {
                        window_start = step * 4;
                        window_marks = 0;
                    }
                    window_marks += 1;
                    if window_marks == 2 {
                        hits += 1;
                        if hits <= 10 {
                            crate::dbg_log_public(&format!(
                                "[itemcommon] killsword marks inside the STRUCT at +{:#x} (array we write is at *(+0x28)={base:#x})",
                                window_start
                            ));
                        }
                    }
                }
            }
            crate::dbg_log_public(&format!(
                "[itemcommon] struct sweep: {hits} neighbourhood(s) carry >=2 killsword marks"
            ));
        }

        if heap(base) {
            for kind in [0usize, 0x3F, 0x40] {
                let row = base + kind * ROW_STRIDE;
                let mut dump = String::new();
                for offset in [0x10usize, 0xa4, 0xac, 0xb0, 0xd0, 0xe0] {
                    let value = core::ptr::read_volatile((row + offset) as *const f32);
                    dump.push_str(&format!(" +{offset:#x}={value:.3}"));
                }
                crate::dbg_log_public(&format!("[itemcommon] row[{kind:#x}] {row:#x}{dump}"));
            }
            for kind in [
                0x3Fusize, 0x40, 0xE2, 0xBE, 0x1AA, 0xA8, 0x02, 0x41, 0x5C, 0xA9, 0x08, 0xAD,
            ] {
                let mut line = String::new();
                for word in 0..ROW_STRIDE / 4 {
                    if word % 16 == 0 && !line.is_empty() {
                        crate::dbg_log_public(&format!("[itemrow {kind:#x}]{line}"));
                        line.clear();
                    }
                    if word % 16 == 0 {
                        line.push_str(&format!(" +{:#05x}:", word * 4));
                    }
                    let at = base + kind * ROW_STRIDE + word * 4;
                    line.push_str(&format!(
                        " {:.4}",
                        core::ptr::read_volatile(at as *const f32)
                    ));
                }
                if !line.is_empty() {
                    crate::dbg_log_public(&format!("[itemrow {kind:#x}]{line}"));
                }
            }
            crate::dbg_log_public(&format!(
                "[itemcommon] CONFIRMED base={base:#x} stride={ROW_STRIDE:#x}; killsword row dumped ({} floats)",
                ROW_STRIDE / 4
            ));
            let _ = (
                row_matches(base, 0x3F, &KS_ROW),
                row_matches(base, 0x40, &DS_ROW),
            );
            DONE.store(true, Ordering::Release);
        } else {
            crate::dbg_log_public(&format!(
                "[itemcommon] attempt {attempt}: real+0x28={base:#x}, not populated yet"
            ));
        }

        let _ = (&KILLSWORD, &DEATHSCYTHE, &marks, ACCESSOR_SPAN);
    }
}
