use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use dionysus_core::stage_items::{row, switched_off, ItemRule};
use std::collections::HashMap;
use std::sync::Mutex;

const VANILLA_STAGE_IDS: u32 = 364;
const FIRST_LOOKUP: usize = 0x17ac478;
const SECOND_LOOKUP: usize = 0x17ac604;
const LOOKUP_OPCODE: u32 = 0x3100053f;
const FIRST_HIT: usize = 0x17ac4e0;
const SECOND_HIT: usize = 0x17ac66c;
const HIT_OPCODE: u32 = 0xf94e8108;
const LOOKUP_REGISTER: usize = 9;
const ACCESSOR_REGISTER: usize = 8;
const ROW_REGISTER: usize = 10;
const MAPPING_BEGIN: usize = 0x1d00;
const MAPPING_END: usize = 0x1d08;
const ROW_BEGIN: usize = 8;
const ROW_END: usize = 0x10;
const ANY_ROW_STAGE: u32 = 0;
const NO_STAGE: u32 = u32::MAX;

#[repr(C)]
struct Entry {
    hash: u64,
    begin: usize,
    end: usize,
    capacity: usize,
}

struct Built {
    _flags: Box<[u8]>,
    entry: Box<Entry>,
}

static RULES: Mutex<Option<HashMap<u32, ItemRule>>> = Mutex::new(None);
static BUILT: Mutex<Option<HashMap<u32, Built>>> = Mutex::new(None);
static PENDING: AtomicU32 = AtomicU32::new(NO_STAGE);
static DONOR_LOGGED: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_rule(stage_id: u32, rule: ItemRule) {
    if let Ok(mut rules) = RULES.lock() {
        rules.get_or_insert_with(HashMap::new).insert(stage_id, rule);
    }
}

#[derive(Clone, Default)]
struct PlaceEdit {
    start_all: Option<bool>,
    items: Vec<(i32, bool)>,
}

static PLACE_EDITS: Mutex<Option<HashMap<u64, PlaceEdit>>> = Mutex::new(None);

fn edit_place(place: u64, change: impl FnOnce(&mut PlaceEdit)) {
    if let Ok(mut edits) = PLACE_EDITS.lock() {
        change(edits.get_or_insert_with(HashMap::new).entry(place).or_default());
    }
    if let Ok(mut built) = BUILT.lock() {
        *built = None;
    }
}

fn rule_for(stage_id: u32) -> Option<ItemRule> {
    let base = RULES.lock().ok().and_then(|rules| rules.as_ref()?.get(&stage_id).cloned());
    let edit = crate::stage_rows::place_of(stage_id)
        .and_then(|(place, _)| PLACE_EDITS.lock().ok()?.as_ref()?.get(&place).cloned());
    let Some(edit) = edit else {
        return base;
    };
    let mut rule = base.unwrap_or_else(|| ItemRule {
        start_all: crate::stage_dispatch::donor_for(stage_id).is_none(),
        ..Default::default()
    });
    if let Some(all) = edit.start_all {
        rule.start_all = all;
    }
    for (kind, on) in edit.items {
        rule.off.retain(|known| *known != kind);
        rule.on.retain(|known| *known != kind);
        if on {
            rule.on.push(kind);
        } else {
            rule.off.push(kind);
        }
    }
    Some(rule)
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

fn steer(ctx: &mut skyline::hooks::InlineCtx) {
    PENDING.store(NO_STAGE, Ordering::Relaxed);
    let stage_id = ctx.registers[LOOKUP_REGISTER].w();
    if stage_id < VANILLA_STAGE_IDS {
        return;
    }
    let donor = crate::stage_dispatch::donor_for(stage_id);
    if let Some(rule) = rule_for(stage_id) {
        let read_from = match donor {
            Some(donor) if !rule.start_all => donor,
            _ => ANY_ROW_STAGE,
        };
        ctx.registers[LOOKUP_REGISTER].set_x(read_from as u64);
        PENDING.store(stage_id, Ordering::Relaxed);
        return;
    }
    if !crate::stage_donor_tests::enabled() {
        return;
    }
    let Some(donor) = donor else {
        return;
    };
    ctx.registers[LOOKUP_REGISTER].set_x(donor as u64);
    if !DONOR_LOGGED.swap(true, Ordering::Relaxed) {
        crate::dbg_log_public(&format!(
            "[stageitems] stage id {stage_id} leaves out the items, Assist Trophies and Poke Ball Pokemon its donor {donor} does"
        ));
    }
}

unsafe fn build(stage_id: u32, accessor: usize, matched: usize) -> Option<usize> {
    let mut built = BUILT.lock().ok()?;
    let built = built.get_or_insert_with(HashMap::new);
    if let Some(done) = built.get(&stage_id) {
        return Some(&*done.entry as *const Entry as usize);
    }
    let rule = rule_for(stage_id)?;
    let base_begin = read::<usize>(matched + ROW_BEGIN);
    let base_end = read::<usize>(matched + ROW_END);
    let mapping_begin = read::<usize>(accessor + MAPPING_BEGIN);
    let mapping_end = read::<usize>(accessor + MAPPING_END);
    if base_begin == 0 || mapping_begin == 0 || base_end < base_begin || mapping_end < mapping_begin {
        return None;
    }
    let base = core::slice::from_raw_parts(base_begin as *const u8, base_end - base_begin);
    let mapping =
        core::slice::from_raw_parts(mapping_begin as *const i32, (mapping_end - mapping_begin) / 4);
    let flags: Box<[u8]> = row(&rule, base, mapping).into_boxed_slice();
    let begin = flags.as_ptr() as usize;
    let end = begin + flags.len();
    let entry = Box::new(Entry {
        hash: read::<u64>(matched),
        begin,
        end,
        capacity: end,
    });
    crate::dbg_log_public(&format!(
        "[stageitems] stage id {stage_id} switches off {} of {} items, Assist Trophies and Poke Ball Pokemon (starting from {}, {} off, {} on)",
        switched_off(&flags),
        flags.len(),
        if rule.start_all { "all of them" } else { "its donor's list" },
        rule.off.len(),
        rule.on.len(),
    ));
    let address = &*entry as *const Entry as usize;
    built.insert(stage_id, Built { _flags: flags, entry });
    Some(address)
}

fn hand_over(ctx: &mut skyline::hooks::InlineCtx) {
    let stage_id = PENDING.swap(NO_STAGE, Ordering::Relaxed);
    if stage_id == NO_STAGE {
        return;
    }
    let accessor = ctx.registers[ACCESSOR_REGISTER].x() as usize;
    let matched = ctx.registers[ROW_REGISTER].x() as usize;
    if accessor == 0 || matched == 0 {
        return;
    }
    if let Some(entry) = unsafe { build(stage_id, accessor, matched) } {
        ctx.registers[ROW_REGISTER].set_x(entry as u64);
    }
}

#[skyline::hook(offset = crate::game::rel(FIRST_LOOKUP), inline)]
unsafe fn first_lookup(ctx: &mut skyline::hooks::InlineCtx) {
    steer(ctx);
}

#[skyline::hook(offset = crate::game::rel(SECOND_LOOKUP), inline)]
unsafe fn second_lookup(ctx: &mut skyline::hooks::InlineCtx) {
    steer(ctx);
}

#[skyline::hook(offset = crate::game::rel(FIRST_HIT), inline)]
unsafe fn first_hit(ctx: &mut skyline::hooks::InlineCtx) {
    hand_over(ctx);
}

#[skyline::hook(offset = crate::game::rel(SECOND_HIT), inline)]
unsafe fn second_hit(ctx: &mut skyline::hooks::InlineCtx) {
    hand_over(ctx);
}

pub(crate) fn install() {
    let sites = [
        (FIRST_LOOKUP, LOOKUP_OPCODE),
        (SECOND_LOOKUP, LOOKUP_OPCODE),
        (FIRST_HIT, HIT_OPCODE),
        (SECOND_HIT, HIT_OPCODE),
    ];
    if let Some((offset, _)) = sites
        .iter()
        .find(|(offset, opcode)| unsafe { crate::game::text_word(*offset) } != *opcode)
    {
        skyline::println!(
            "[stageitems] {offset:#x} holds another instruction; cloned stages keep every item"
        );
        return;
    }
    skyline::install_hooks!(first_lookup, second_lookup, first_hit, second_hit);
    skyline::println!("[stageitems] cloned stages leave out their donor's items, or the ones their stage.toml names");
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_stage_item_set(place: u64, item: *const core::ffi::c_char, on: i32) -> i32 {
    if item.is_null() {
        return -2;
    }
    let Ok(name) = core::ffi::CStr::from_ptr(item).to_str() else {
        return -2;
    };
    let Some(kind) = dionysus_core::item_kinds::item_kind(name) else {
        return -2;
    };
    edit_place(place, |edit| {
        edit.items.retain(|(known, _)| *known != kind);
        edit.items.push((kind, on != 0));
    });
    0
}

#[no_mangle]
pub extern "C" fn clone_engine_stage_items_start(place: u64, all: i32) -> i32 {
    edit_place(place, |edit| edit.start_all = Some(all != 0));
    0
}
