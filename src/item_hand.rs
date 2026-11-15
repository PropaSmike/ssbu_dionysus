use dionysus_core::item_hand::{self as hand, ItemHand, COMMONHAVE, HASH_MASK, HOLD_WORDS};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};

const BOMA_ITEM_MODULE: usize = 0xc8;
const ITEM_MODULE_HOLD: usize = 0x20;
const MODEL_OWNER: usize = 0x8;
const MODEL_KIND: usize = 0x2c8;
const BLOB_HOLD: usize = 0x10;
const BLOB_BATTERING: usize = 0x20;
const LOWEST_POINTER: usize = 0x1_0000;
const MODEL_JOINT_INDEX: usize = 0x6d_fcb0;
const MODEL_JOINT_INDEX_WORD: u32 = 0xa9be_4ff4;
const LOG_BUDGET: u32 = 16;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static SWAP_LOG: AtomicU32 = AtomicU32::new(0);
static BATTERING_LOG: AtomicU32 = AtomicU32::new(0);

struct Built {
    kind: i32,
    source: usize,
    words: usize,
}

fn hands() -> &'static RwLock<Vec<(i32, ItemHand)>> {
    static HANDS: OnceLock<RwLock<Vec<(i32, ItemHand)>>> = OnceLock::new();
    HANDS.get_or_init(|| RwLock::new(Vec::new()))
}

fn built() -> &'static Mutex<Vec<Built>> {
    static BUILT: OnceLock<Mutex<Vec<Built>>> = OnceLock::new();
    BUILT.get_or_init(|| Mutex::new(Vec::new()))
}

fn hand_of(kind: i32) -> Option<ItemHand> {
    hands()
        .read()
        .ok()?
        .iter()
        .find(|(candidate, _)| *candidate == kind)
        .map(|(_, hand)| *hand)
}

pub(crate) fn register(kind: i32, base: &str, choice: ItemHand) -> Result<(), String> {
    if !hand::base_has_hand_bones(base) {
        return Err(format!("{base} has no HaveL or HaveR bone"));
    }
    let mut held = hands().write().map_err(|_| "item hand registry poisoned".to_string())?;
    held.retain(|(candidate, _)| *candidate != kind);
    held.push((kind, choice));
    ACTIVE.store(true, Ordering::Release);
    Ok(())
}

unsafe fn read_word(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

unsafe fn words_for(kind: i32, source: usize, choice: ItemHand) -> Option<usize> {
    let mut held = built().lock().ok()?;
    if held.iter().any(|entry| entry.words == source) {
        return None;
    }
    if let Some(entry) = held.iter().find(|entry| entry.kind == kind && entry.source == source) {
        return (entry.words != 0).then_some(entry.words);
    }
    let base = core::ptr::read_volatile(source as *const [u64; HOLD_WORDS]);
    let words = hand::hold_words(base, choice)
        .map(|words| Box::leak(Box::new(words)) as *const [u64; HOLD_WORDS] as usize)
        .unwrap_or(0);
    held.push(Built { kind, source, words });
    (words != 0).then_some(words)
}

pub(crate) unsafe fn on_status_agent(boma: u64, clone_kind: Option<i32>) {
    if !ACTIVE.load(Ordering::Acquire) || boma == 0 {
        return;
    }
    let Some(kind) = clone_kind else {
        return;
    };
    let Some(choice) = hand_of(kind) else {
        return;
    };
    let module = read_word(boma as usize + BOMA_ITEM_MODULE);
    if module < LOWEST_POINTER {
        return;
    }
    let slot = module + ITEM_MODULE_HOLD;
    let source = read_word(slot);
    if source < LOWEST_POINTER {
        return;
    }
    let Some(words) = words_for(kind, source, choice) else {
        return;
    };
    core::ptr::write_volatile(slot as *mut usize, words);
    if SWAP_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        skyline::println!(
            "[itemhand] kind {kind}: items held in the {} hand (hold bones {source:#x} -> {words:#x})",
            choice.name()
        );
    }
}

unsafe fn base_hand_words(model_kind: i32) -> Option<(u64, u64)> {
    #[skyline::from_offset(crate::game::rel(crate::offsets::OFF_STATIC_FIGHTER_DATA))]
    fn static_fighter_data(kind: i32) -> *const u8;

    let blob = static_fighter_data(model_kind) as usize;
    if blob < LOWEST_POINTER {
        return None;
    }
    let hold = read_word(blob + BLOB_HOLD);
    let battering = read_word(blob + BLOB_BATTERING);
    if hold < LOWEST_POINTER || battering < LOWEST_POINTER {
        return None;
    }
    Some((read_word(hold) as u64, read_word(battering) as u64))
}

unsafe fn battering_bone(model: u64) -> Option<u64> {
    let boma = read_word(model as usize + MODEL_OWNER) as *mut smash::app::BattleObjectModuleAccessor;
    if (boma as usize) < LOWEST_POINTER
        || smash::app::utility::get_category(&mut *boma)
            != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        return None;
    }
    let kind = crate::entry_groups::clone_of_boma(boma)?;
    let choice = hand_of(kind)?;
    let model_kind = core::ptr::read_volatile((model as usize + MODEL_KIND) as *const i32);
    let (main, battering) = base_hand_words(model_kind)?;
    let bone = hand::battering_bone(battering, main, choice)?;
    if BATTERING_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        skyline::println!(
            "[itemhand] kind {kind}: commonhave -> {:#x} ({} hand)",
            bone & HASH_MASK,
            choice.name()
        );
    }
    Some(bone)
}

#[skyline::hook(offset = crate::game::rel(MODEL_JOINT_INDEX))]
unsafe fn model_joint_index(model: u64, bone: u64, flag: u64) -> u64 {
    if ACTIVE.load(Ordering::Relaxed) && bone & HASH_MASK == COMMONHAVE && model != 0 {
        if let Some(own) = battering_bone(model) {
            return call_original!(model, own, flag);
        }
    }
    call_original!(model, bone, flag)
}

pub(crate) fn install() {
    let word = unsafe { crate::game::text_word(MODEL_JOINT_INDEX) };
    if word != MODEL_JOINT_INDEX_WORD {
        crate::dbg_log_public(&format!(
            "[itemhand] model joint lookup at {MODEL_JOINT_INDEX:#x} not recognised ({word:#010x}); item_hand is refused"
        ));
        return;
    }
    skyline::install_hooks!(model_joint_index);
}
