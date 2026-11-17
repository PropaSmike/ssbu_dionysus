use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use smash::app::BattleObjectModuleAccessor;
use std::collections::HashMap;
use std::sync::RwLock;

const PARAM_INT: usize = 0x4e53a0;
const PARAM_FLOAT: usize = 0x4e53e0;
const WORK_VTABLE: usize = 0x4f73738;
const INT_SLOT: usize = 0x250;
const FLOAT_SLOT: usize = 0x270;
const MODULE_ACCESSOR: usize = 8;
const ANY_SLOT: i32 = -1;
const LOG_BUDGET: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Answer {
    Float(f32),
    Int(i32),
    Mul(f32),
}

static ENTRIES: RwLock<Option<HashMap<(i32, u64, u64), Answer>>> = RwLock::new(None);
static BLOCKS: RwLock<Option<HashMap<i32, Vec<u64>>>> = RwLock::new(None);
static ARMED: AtomicBool = AtomicBool::new(false);
static LOGGED: AtomicU32 = AtomicU32::new(0);
static PREVIOUS_INT: AtomicUsize = AtomicUsize::new(0);
static PREVIOUS_FLOAT: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn note(kind: i32, slot: i32, category: u64, field: u64, answer: Answer) {
    if slot != ANY_SLOT || category == 0 {
        return;
    }
    if let Ok(mut entries) = ENTRIES.write() {
        entries.get_or_insert_with(HashMap::new).insert((kind, category, field), answer);
        ARMED.store(true, Ordering::Release);
    }
}

fn block(name: &[u8]) -> u64 {
    let name = name.strip_suffix(&[0]).unwrap_or(name);
    dionysus_core::hash40(&format!("param_{}", String::from_utf8_lossy(name)))
}

fn blocks_of(weapon: i32) -> Vec<u64> {
    if let Some(known) = BLOCKS.read().ok().and_then(|blocks| blocks.as_ref()?.get(&weapon).cloned()) {
        return known;
    }
    let mut found = Vec::new();
    if let Some(own) = crate::custom_articles::own_param_name(weapon) {
        found.push(block(own));
        if let Some(source) = crate::custom_articles::source_weapon_name(weapon) {
            found.push(block(source.to_bytes()));
        }
    } else if let Some(name) = crate::custom_articles::vanilla_weapon_name(weapon) {
        found.push(block(name.to_bytes()));
    }
    if let Ok(mut blocks) = BLOCKS.write() {
        blocks.get_or_insert_with(HashMap::new).insert(weapon, found.clone());
    }
    found
}

unsafe fn answer(module: u64, field: u64) -> Option<Answer> {
    if !ARMED.load(Ordering::Acquire) || module == 0 {
        return None;
    }
    let boma = core::ptr::read_volatile((module as usize + MODULE_ACCESSOR) as *const usize);
    if boma == 0 {
        return None;
    }
    let accessor = &mut *(boma as *mut BattleObjectModuleAccessor);
    if smash::app::utility::get_category(accessor) != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_WEAPON {
        return None;
    }
    let owner = crate::article_owner_kind_by_entry(boma as u64)?;
    let weapon = crate::article_kind_facade::minted_kind_of_boma(boma)
        .unwrap_or_else(|| smash::app::utility::get_kind(accessor));
    let entries = ENTRIES.read().ok()?;
    let entries = entries.as_ref()?;
    let found = blocks_of(weapon)
        .into_iter()
        .find_map(|block| entries.get(&(owner, block, field)).copied())?;
    if LOGGED.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        skyline::println!("[customparam] kind {owner}'s weapon {weapon} reads {field:#x} as {found:?}");
    }
    Some(found)
}

extern "C" fn param_float(module: u64, category: u64, field: u64) -> f32 {
    let previous: extern "C" fn(u64, u64, u64) -> f32 =
        unsafe { core::mem::transmute(PREVIOUS_FLOAT.load(Ordering::Acquire)) };
    let value = previous(module, category, field);
    match unsafe { answer(module, field) } {
        Some(Answer::Float(set)) => set,
        Some(Answer::Int(set)) => set as f32,
        Some(Answer::Mul(factor)) => value * factor,
        None => value,
    }
}

extern "C" fn param_int(module: u64, category: u64, field: u64) -> i32 {
    let previous: extern "C" fn(u64, u64, u64) -> i32 =
        unsafe { core::mem::transmute(PREVIOUS_INT.load(Ordering::Acquire)) };
    let value = previous(module, category, field);
    match unsafe { answer(module, field) } {
        Some(Answer::Int(set)) => set,
        Some(Answer::Float(set)) => set.round() as i32,
        Some(Answer::Mul(factor)) => (value as f32 * factor).round() as i32,
        None => value,
    }
}

unsafe fn slot_holds(slot: usize, expected: usize) -> bool {
    let address = crate::game::addr(WORK_VTABLE) + slot;
    core::ptr::read_volatile(address as *const usize) == crate::game::addr(expected)
}

unsafe fn swap(slot: usize, replacement: usize, previous: &AtomicUsize) -> bool {
    let address = crate::game::addr(WORK_VTABLE) + slot;
    previous.store(core::ptr::read_volatile(address as *const usize), Ordering::Release);
    crate::text_patch::write_bytes(address, &replacement.to_le_bytes())
}

pub(crate) fn install() {
    let swapped = unsafe {
        slot_holds(FLOAT_SLOT, PARAM_FLOAT)
            && slot_holds(INT_SLOT, PARAM_INT)
            && swap(FLOAT_SLOT, param_float as usize, &PREVIOUS_FLOAT)
            && swap(INT_SLOT, param_int as usize, &PREVIOUS_INT)
    };
    if swapped {
        skyline::println!("[customparam] clones' articles read the params their fighter sets for them");
    } else {
        skyline::println!("[customparam] the weapon WorkModule table holds other entries; articles keep their owner's params");
    }
}
