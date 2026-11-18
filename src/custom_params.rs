use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use dionysus_core::fighter_toml::ParamValue;
use dionysus_core::row_fields::{TAG_BOOL, TAG_F32, TAG_I32, TAG_MUL};
use smash::app::BattleObjectModuleAccessor;
use std::collections::HashMap;
use std::sync::RwLock;

const PARAM_INT: usize = 0x4e53a0;
const PARAM_FLOAT: usize = 0x4e53e0;
const WORK_VTABLE: usize = 0x4f73738;
const INT_SLOT: usize = 0x250;
const FLOAT_SLOT: usize = 0x270;
const MODULE_ACCESSOR: usize = 8;
const MODULE_PARAMS: usize = 0x38;
const ACCESSOR_OBJECT_ID: usize = 8;
const OBJECT_KIND: usize = 0xc;
const BATTLE_OBJECT_FROM_ID: usize = 0x3ac560;
const ANY_SLOT: i32 = -1;
const LOG_BUDGET: u32 = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Answer {
    Float(f32),
    Int(i32),
    Mul(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Space {
    Article,
    Item,
    Enemy,
    Stage,
}

impl Space {
    fn from_code(code: u32) -> Option<Space> {
        match code {
            1 => Some(Space::Item),
            2 => Some(Space::Enemy),
            3 => Some(Space::Stage),
            _ => None,
        }
    }

    fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

type Key = (Space, u64, u64, u64);

static ENTRIES: RwLock<Option<HashMap<Key, Answer>>> = RwLock::new(None);
static BLOCKS: RwLock<Option<HashMap<i32, Vec<u64>>>> = RwLock::new(None);
static ARMED: AtomicU32 = AtomicU32::new(0);
static SWAPPED: AtomicBool = AtomicBool::new(false);
static LOGGED: AtomicU32 = AtomicU32::new(0);
static PREVIOUS_INT: AtomicUsize = AtomicUsize::new(0);
static PREVIOUS_FLOAT: AtomicUsize = AtomicUsize::new(0);

fn store(space: Space, owner: u64, group: u64, field: u64, answer: Answer) {
    if let Ok(mut entries) = ENTRIES.write() {
        entries.get_or_insert_with(HashMap::new).insert((space, owner, group, field), answer);
        ARMED.fetch_or(space.bit(), Ordering::AcqRel);
    }
}

fn armed(space: Space) -> bool {
    ARMED.load(Ordering::Acquire) & space.bit() != 0
}

pub(crate) fn note(kind: i32, slot: i32, group: u64, field: u64, answer: Answer) {
    if slot == ANY_SLOT && group != 0 {
        store(Space::Article, kind as u64, group, field, answer);
    }
}

pub(crate) fn answer_of(value: ParamValue) -> Answer {
    match value {
        ParamValue::Float(value) => Answer::Float(value as f32),
        ParamValue::Int(value) => Answer::Int(value),
    }
}

pub(crate) fn set_asset(space: Space, owner: u64, group: u64, field: u64, answer: Answer) {
    store(space, owner, group, field, answer);
}

fn get(space: Space, owner: u64, group: u64, field: u64) -> Option<Answer> {
    ENTRIES.read().ok()?.as_ref()?.get(&(space, owner, group, field)).copied()
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

unsafe fn object_kind(boma: usize) -> Option<i32> {
    let lookup: extern "C" fn(u32) -> usize = core::mem::transmute(crate::game::addr(BATTLE_OBJECT_FROM_ID));
    let object = lookup(core::ptr::read_volatile((boma + ACCESSOR_OBJECT_ID) as *const u32));
    (object != 0).then(|| core::ptr::read_volatile((object + OBJECT_KIND) as *const i32))
}

#[cfg(feature = "item_clone_backend")]
unsafe fn item_kind(boma: usize) -> Option<i32> {
    crate::item_clones::live_kind_by_object_id(core::ptr::read_volatile((boma + ACCESSOR_OBJECT_ID) as *const u32))
}

#[cfg(not(feature = "item_clone_backend"))]
unsafe fn item_kind(_boma: usize) -> Option<i32> {
    None
}

fn log(text: impl FnOnce() -> String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        skyline::println!("{}", text());
    }
}

unsafe fn answer(module: u64, group: u64, field: u64) -> Option<Answer> {
    if ARMED.load(Ordering::Acquire) == 0 || module == 0 {
        return None;
    }
    let boma = core::ptr::read_volatile((module as usize + MODULE_ACCESSOR) as *const usize);
    if boma == 0 {
        return None;
    }
    let accessor = &mut *(boma as *mut BattleObjectModuleAccessor);
    let category = smash::app::utility::get_category(accessor);
    if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_WEAPON && armed(Space::Article) {
        let owner = crate::article_owner_kind_by_entry(boma as u64)?;
        let weapon = crate::article_kind_facade::minted_kind_of_boma(boma)
            .unwrap_or_else(|| smash::app::utility::get_kind(accessor));
        let found = blocks_of(weapon)
            .into_iter()
            .find_map(|block| get(Space::Article, owner as u64, block, field))?;
        log(|| format!("[customparam] kind {owner}'s weapon {weapon} reads {field:#x} as {found:?}"));
        return Some(found);
    }
    if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_ITEM && armed(Space::Item) {
        let kind = item_kind(boma)?;
        let found = get(Space::Item, kind as u64, group, field)?;
        log(|| format!("[customparam] item kind {kind:#x} reads {group:#x}.{field:#x} as {found:?}"));
        return Some(found);
    }
    if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_ENEMY && armed(Space::Enemy) {
        let kind = object_kind(boma)?;
        let found = get(Space::Enemy, kind as u64, group, field)?;
        log(|| format!("[customparam] enemy kind {kind} reads {group:#x}.{field:#x} as {found:?}"));
        return Some(found);
    }
    None
}

unsafe fn has_params(module: u64) -> bool {
    module != 0 && core::ptr::read_volatile((module as usize + MODULE_PARAMS) as *const usize) != 0
}

fn float_of(found: Option<Answer>, read: impl FnOnce() -> f32) -> f32 {
    match found {
        Some(Answer::Float(set)) => set,
        Some(Answer::Int(set)) => set as f32,
        Some(Answer::Mul(factor)) => read() * factor,
        None => read(),
    }
}

fn int_of(found: Option<Answer>, read: impl FnOnce() -> i32) -> i32 {
    match found {
        Some(Answer::Int(set)) => set,
        Some(Answer::Float(set)) => set.round() as i32,
        Some(Answer::Mul(factor)) => (read() as f32 * factor).round() as i32,
        None => read(),
    }
}

extern "C" fn param_float(module: u64, group: u64, field: u64) -> f32 {
    let found = unsafe { answer(module, group, field) };
    float_of(found, || {
        if !unsafe { has_params(module) } {
            return 0.0;
        }
        let previous: extern "C" fn(u64, u64, u64) -> f32 =
            unsafe { core::mem::transmute(PREVIOUS_FLOAT.load(Ordering::Acquire)) };
        previous(module, group, field)
    })
}

extern "C" fn param_int(module: u64, group: u64, field: u64) -> i32 {
    let found = unsafe { answer(module, group, field) };
    int_of(found, || {
        if !unsafe { has_params(module) } {
            return 0;
        }
        let previous: extern "C" fn(u64, u64, u64) -> i32 =
            unsafe { core::mem::transmute(PREVIOUS_INT.load(Ordering::Acquire)) };
        previous(module, group, field)
    })
}

#[cfg(feature = "item_clone_backend")]
mod item_stubs {
    use super::{float_of, get, int_of, log, Answer, Space};
    use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    const PARAM_HOLDER: usize = 0x52c31e0;
    const LDR_X17_LITERAL_8: u32 = 0x5800_0051;
    const BR_X17: u32 = 0xd61f_0220;
    const LDR_X0_HOLDER: u32 = 0xf940_f100;
    const ADRP_X8_MASK: u32 = 0x9f00_001f;
    const ADRP_X8: u32 = 0x9000_0008;
    const SELF_FLOAT: usize = 0;
    const SELF_INT: usize = 1;
    const STRUCT_FLOAT: usize = 2;
    const STRUCT_INT: usize = 3;
    const STUBS: [(usize, usize, usize); 4] = [
        (0x15c4c10, 0x1602a30, 4),
        (0x15c4c30, 0x1602b90, 4),
        (0x15c4c70, 0x1602f50, 5),
        (0x15c4c90, 0x16031d0, 5),
    ];

    static GETTERS: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
    static PARAM_GROUP: AtomicU64 = AtomicU64::new(0);

    fn owner(kind: i32) -> Option<i32> {
        if crate::item_clones::clone_base_kind(kind).is_some() {
            return Some(kind);
        }
        crate::item_params::runtime_clone_for_base(kind)
    }

    fn answer(kind: i32, group: u64, field: u64) -> Option<Answer> {
        if !super::armed(Space::Item) {
            return None;
        }
        let owner = owner(kind)?;
        let found = get(Space::Item, owner as u64, group, field)?;
        log(|| format!("[customparam] item kind {owner:#x} reads {group:#x}.{field:#x} as {found:?}"));
        Some(found)
    }

    unsafe fn holder() -> u64 {
        core::ptr::read_volatile(crate::game::addr(PARAM_HOLDER) as *const u64)
    }

    fn readable(kind: i32) -> bool {
        crate::item_params::getter_reads_safely(kind)
    }

    extern "C" fn self_float(kind: i32, field: u64) -> f32 {
        float_of(answer(kind, PARAM_GROUP.load(Ordering::Relaxed), field), || unsafe {
            if !readable(kind) {
                return 0.0;
            }
            let getter: extern "C" fn(u64, i32, u64) -> f32 =
                core::mem::transmute(GETTERS[SELF_FLOAT].load(Ordering::Acquire));
            getter(holder(), kind, field)
        })
    }

    extern "C" fn self_int(kind: i32, field: u64) -> i32 {
        int_of(answer(kind, PARAM_GROUP.load(Ordering::Relaxed), field), || unsafe {
            if !readable(kind) {
                return 0;
            }
            let getter: extern "C" fn(u64, i32, u64) -> i32 =
                core::mem::transmute(GETTERS[SELF_INT].load(Ordering::Acquire));
            getter(holder(), kind, field)
        })
    }

    extern "C" fn struct_float(kind: i32, group: u64, field: u64) -> f32 {
        float_of(answer(kind, group, field), || unsafe {
            if !readable(kind) {
                return 0.0;
            }
            let getter: extern "C" fn(u64, i32, u64, u64) -> f32 =
                core::mem::transmute(GETTERS[STRUCT_FLOAT].load(Ordering::Acquire));
            getter(holder(), kind, group, field)
        })
    }

    extern "C" fn struct_int(kind: i32, group: u64, field: u64) -> i32 {
        int_of(answer(kind, group, field), || unsafe {
            if !readable(kind) {
                return 0;
            }
            let getter: extern "C" fn(u64, i32, u64, u64) -> i32 =
                core::mem::transmute(GETTERS[STRUCT_INT].load(Ordering::Acquire));
            getter(holder(), kind, group, field)
        })
    }

    unsafe fn branch_target(address: usize) -> Option<usize> {
        let word = core::ptr::read_volatile(address as *const u32);
        if word & 0xfc00_0000 != 0x1400_0000 {
            return None;
        }
        let offset = (((word & 0x03ff_ffff) << 6) as i32 >> 4) as isize;
        Some(address.wrapping_add_signed(offset))
    }

    fn page_of(stub: usize, word: u32) -> usize {
        let immediate = ((((word >> 5) & 0x7ffff) << 2) | ((word >> 29) & 3)) as i32;
        let pages = ((immediate << 11) >> 11) as isize;
        (stub & !0xfff).wrapping_add_signed(pages << 12)
    }

    unsafe fn stub_intact(stub: usize, getter: usize, words: usize) -> bool {
        let first = core::ptr::read_volatile(stub as *const u32);
        let load = core::ptr::read_volatile((stub + (words - 1) * 4) as *const u32);
        first & ADRP_X8_MASK == ADRP_X8
            && page_of(stub, first) + (PARAM_HOLDER & 0xfff) == crate::game::addr(PARAM_HOLDER)
            && load == LDR_X0_HOLDER
            && branch_target(stub + words * 4) == Some(getter)
    }

    pub(super) fn install() -> bool {
        PARAM_GROUP.store(dionysus_core::hash40(dionysus_core::prcxml::ASSET_GROUP), Ordering::Relaxed);
        let intact = STUBS.iter().all(|(stub, getter, words)| unsafe {
            stub_intact(crate::game::addr(*stub), crate::game::addr(*getter), *words)
        });
        if !intact {
            return false;
        }
        let handlers = [
            self_float as *const () as usize,
            self_int as *const () as usize,
            struct_float as *const () as usize,
            struct_int as *const () as usize,
        ];
        for (index, (_, getter, _)) in STUBS.iter().enumerate() {
            GETTERS[index].store(crate::game::addr(*getter), Ordering::Release);
        }
        STUBS.iter().zip(handlers).all(|((stub, _, _), handler)| unsafe {
            let handler = handler as u64;
            crate::text_patch::write_words(
                crate::game::addr(*stub),
                &[LDR_X17_LITERAL_8, BR_X17, handler as u32, (handler >> 32) as u32],
            )
        })
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
            && swap(FLOAT_SLOT, param_float as *const () as usize, &PREVIOUS_FLOAT)
            && swap(INT_SLOT, param_int as *const () as usize, &PREVIOUS_INT)
    };
    SWAPPED.store(swapped, Ordering::Release);
    if swapped {
        skyline::println!("[customparam] clones' articles, items and enemies read the params set for them");
    } else {
        skyline::println!("[customparam] the WorkModule table for articles, items and enemies holds other entries; they keep the game's params");
    }
    #[cfg(feature = "item_clone_backend")]
    if item_stubs::install() {
        skyline::println!("[customparam] item scripts read the params set for clone items");
    } else {
        skyline::println!("[customparam] app::item::self_param_* hold other code; item scripts keep the game's params");
    }
}

fn valid_owner(space: Space, owner: u64) -> bool {
    match space {
        #[cfg(feature = "item_clone_backend")]
        Space::Item => crate::item_clones::clone_base_kind(owner as i32).is_some(),
        #[cfg(feature = "item_clone_backend")]
        Space::Enemy => crate::enemy_clones::base_of(owner as u32).is_some(),
        Space::Stage => owner != 0,
        _ => false,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_asset_param_set(space: u32, owner: u64, group: u64, field: u64, tag: u32, bits: u64) -> i32 {
    let Some(space) = Space::from_code(space) else {
        return -1;
    };
    if !valid_owner(space, owner) {
        return -1;
    }
    let answer = match tag {
        TAG_F32 => Answer::Float(f32::from_bits(bits as u32)),
        TAG_I32 => Answer::Int(bits as u32 as i32),
        TAG_BOOL => Answer::Int(i32::from(bits != 0)),
        TAG_MUL if space != Space::Stage => Answer::Mul(f32::from_bits(bits as u32)),
        _ => return -3,
    };
    set_asset(space, owner, group, field, answer);
    0
}

#[no_mangle]
pub extern "C" fn clone_engine_stage_param_get(group: u64, field: u64, out: *mut u64) -> i32 {
    if out.is_null() {
        return -1;
    }
    let stage_id = unsafe { smash::app::stage::get_stage_id() } as u32;
    let Some((place, _)) = crate::stage_rows::place_of(stage_id) else {
        return -1;
    };
    match get(Space::Stage, place, group, field) {
        Some(Answer::Float(value)) => {
            unsafe { *out = u64::from(value.to_bits()) };
            TAG_F32 as i32
        }
        Some(Answer::Int(value)) => {
            unsafe { *out = u64::from(value as u32) };
            TAG_I32 as i32
        }
        _ => -1,
    }
}
