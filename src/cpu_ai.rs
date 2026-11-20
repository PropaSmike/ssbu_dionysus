use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, RwLock};

const LAST_FIGHTER_WITH_AI: i32 = 0x5d;
const SHOWN: u32 = 8;
const MODULE_WAIT_MS: u32 = 3000;
const FILE_WAIT_MS: u32 = 3000;
const AI_FILES: [&str; 2] = ["attack_data_param.prc", "attack_list_param.prc"];

static ACTIVE: AtomicBool = AtomicBool::new(false);
static CHOSEN: RwLock<Vec<(i32, i32)>> = RwLock::new(Vec::new());
static USABLE: RwLock<Vec<i32>> = RwLock::new(Vec::new());
static REQUESTED: Mutex<Vec<i32>> = Mutex::new(Vec::new());
static PATHS_LOGGED: AtomicU32 = AtomicU32::new(0);
static MISSING_LOGGED: AtomicU32 = AtomicU32::new(0);

pub(crate) fn register(kind: i32, base: &str, fighter: &str) -> Result<Option<i32>, String> {
    if !dionysus_core::cpu_ai::is_ai_owner(fighter) {
        return Err(format!("{fighter} has no CPU AI"));
    }
    let mut chosen = CHOSEN.write().map_err(|_| "cpu_ai registry poisoned".to_string())?;
    chosen.retain(|(candidate, _)| *candidate != kind);
    if fighter == base {
        return Ok(None);
    }
    let Some(ai_kind) = crate::custom_articles::fighter_kind_from_str(fighter)
        .filter(|ai_kind| (0..=LAST_FIGHTER_WITH_AI).contains(ai_kind))
    else {
        return Err(format!("{fighter} is not a fighter kind this game version knows"));
    };
    chosen.push((kind, ai_kind));
    ACTIVE.store(true, Ordering::Release);
    Ok(Some(ai_kind))
}

pub(crate) fn chosen(kind: i32) -> Option<i32> {
    if !ACTIVE.load(Ordering::Acquire) {
        return None;
    }
    CHOSEN.read().ok()?.iter().find(|(candidate, _)| *candidate == kind).map(|(_, ai)| *ai)
}

fn fighter_name(kind: i32) -> String {
    crate::custom_articles::fighter_name(kind)
        .and_then(|name| name.to_str().ok().map(str::to_string))
        .unwrap_or_else(|| format!("kind {kind}"))
}

fn ai_files(ai: i32) -> Vec<String> {
    let name = fighter_name(ai);
    AI_FILES.iter().map(|file| format!("fighter/{name}/ai/param/{file}")).collect()
}

fn files_loaded(ai: i32) -> bool {
    let Some(indexes) = ai_files(ai)
        .iter()
        .map(|path| crate::fighter_modules::file_path_index(path))
        .collect::<Option<Vec<u32>>>()
    else {
        return false;
    };
    for _ in 0..FILE_WAIT_MS {
        if indexes.iter().all(|index| crate::fighter_modules::data_ready(*index)) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    false
}

fn settle(clone: i32) -> Option<i32> {
    let ai = chosen(clone)?;
    let module = crate::fighter_modules::ensure_loaded(ai, MODULE_WAIT_MS);
    let files = module && files_loaded(ai);
    let loaded = module && files;
    if let Ok(mut usable) = USABLE.write() {
        usable.retain(|known| *known != clone);
        if loaded {
            usable.push(clone);
        }
    }
    if !loaded && MISSING_LOGGED.fetch_add(1, Ordering::Relaxed) < SHOWN {
        let missing = if module { "AI files are" } else { "lua2cpp module is" };
        skyline::println!(
            "[cpuai] kind {clone}: {}'s {missing} not loaded, so its CPU keeps its base's AI",
            fighter_name(ai)
        );
    }
    loaded.then_some(ai)
}

fn usable(kind: i32) -> Option<i32> {
    let ai = chosen(kind)?;
    USABLE.read().ok()?.contains(&kind).then_some(ai)
}

pub(crate) fn ai_kind(kind: i32) -> Option<i32> {
    usable(kind).or_else(|| crate::clone_base_of(kind))
}

pub(crate) fn setup_kind(kind: i32) -> Option<i32> {
    if chosen(kind).is_some() {
        if let Some(ai) = settle(kind) {
            return Some(ai);
        }
    }
    crate::clone_base_of(kind)
}

pub(crate) fn path_kind(kind: i32, entry_clone: Option<i32>) -> Option<i32> {
    if !ACTIVE.load(Ordering::Acquire) {
        return None;
    }
    let clone = if chosen(kind).is_some() {
        kind
    } else {
        entry_clone.filter(|clone| crate::clone_base_of(*clone) == Some(kind))?
    };
    let ai = settle(clone)?;
    if PATHS_LOGGED.fetch_add(1, Ordering::Relaxed) < SHOWN {
        skyline::println!("[cpuai] kind {clone}'s CPU loads the AI files of {}", fighter_name(ai));
    }
    Some(ai)
}

pub(crate) fn request_module(kind: i32) {
    let Some(ai) = chosen(kind) else {
        return;
    };
    if let Ok(mut held) = REQUESTED.lock() {
        if held.contains(&ai) {
            return;
        }
        held.push(ai);
    }
    let module = crate::fighter_modules::request(ai);
    skyline::println!(
        "[cpuai] kind {kind} loads {}'s AI: lua2cpp module {}",
        fighter_name(ai),
        if module { "requested" } else { "REFUSED" }
    );
}
