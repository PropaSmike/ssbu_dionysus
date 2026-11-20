use dionysus_core::prc::Param;
use dionysus_core::prc_patch::{self, Applied};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

const MOD_ROOT: &str = crate::mod_folders::MOD_ROOT;
const PARAM_BIND: usize = 0x372_0680;
const PARAM_BIND_WORD: u32 = 0xF81D_0FF5;
const SHOWN: usize = 8;
const NOT_FOUND: u32 = 0xFF_FFFF;
const HANDLE_PARAMS: usize = 0x8;
const HANDLE_INDEX: usize = 0x10;
const HANDLE_DATA: usize = 0x18;
const HANDLE_HASHES: usize = 0x20;
const HANDLE_REFS: usize = 0x28;
const HEADER: usize = 0x10;
const ASSET_TOPS: [&str; 6] = ["fighter", "item", "assist", "pokemon", "boss", "enemy"];
const STAGE_TREES: [&str; 2] = ["normal", "battle"];
const STAGE_FOLDERS: [&str; 2] = ["param", "render"];

type DecompressedSizeFn = unsafe extern "C" fn(u64, &mut usize) -> bool;

struct Patch {
    pack: String,
    name: String,
}

struct Watched {
    target: String,
    hash: u64,
    roots: Vec<Param>,
    patches: Vec<Patch>,
}

static WATCHED: OnceLock<Vec<Watched>> = OnceLock::new();
static ARMED: AtomicBool = AtomicBool::new(false);
static INDEXES: Mutex<Vec<Option<u32>>> = Mutex::new(Vec::new());
static BUILT: Mutex<Option<HashMap<usize, Option<&'static [u8]>>>> = Mutex::new(None);

pub(crate) fn load_all() {
    let watched = WATCHED.get_or_init(discover);
    if watched.is_empty() {
        return;
    }
    for file in watched.iter().take(SHOWN) {
        let patches: Vec<String> = file
            .patches
            .iter()
            .map(|patch| format!("{} ({})", patch.name, patch.pack))
            .collect();
        skyline::println!("[parampatch] {} + {}", file.target, patches.join(" + "));
    }
    if watched.len() > SHOWN {
        skyline::println!("[parampatch] and {} more", watched.len() - SHOWN);
    }
    let word = unsafe { crate::game::text_word(PARAM_BIND) };
    if word != PARAM_BIND_WORD {
        skyline::println!(
            "[parampatch] param bind at {PARAM_BIND:#x} not recognised ({word:#010x}); {} file(s) stay unpatched",
            watched.len()
        );
        return;
    }
    if let Ok(mut indexes) = INDEXES.lock() {
        *indexes = vec![None; watched.len()];
    }
    skyline::install_hook!(param_bind_hook);
    ARMED.store(true, Ordering::Release);
    skyline::println!(
        "[parampatch] {} param file(s) are patched when the game reads them",
        watched.len()
    );
}

fn enabled_folders() -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = std::fs::read_dir(MOD_ROOT)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|folder| folder.is_dir() && crate::mod_folders::enabled(folder))
        .collect();
    folders.sort();
    folders
}

fn entries(directory: &Path, folders_only: bool) -> Vec<(String, String)> {
    std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| !folders_only || entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            (name.to_ascii_lowercase(), name)
        })
        .collect()
}

fn param_folders(pack: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    let stage = pack.join("stage");
    for (place, actual) in entries(&stage, true) {
        for tree in STAGE_TREES {
            for folder in STAGE_FOLDERS {
                found.push((format!("stage/{place}/{tree}/{folder}"), stage.join(&actual).join(tree).join(folder)));
            }
        }
    }
    for top in ASSET_TOPS {
        let root = pack.join(top);
        for (name, actual) in entries(&root, true) {
            found.push((format!("{top}/{name}/param"), root.join(&actual).join("param")));
            if top == "fighter" {
                found.push((format!("{top}/{name}/ai/param"), root.join(&actual).join("ai").join("param")));
            }
        }
    }
    found
}

fn discover() -> Vec<Watched> {
    let mut shipped: HashSet<String> = HashSet::new();
    let mut wanted: BTreeMap<String, (Vec<Param>, Vec<Patch>)> = BTreeMap::new();
    for pack in enabled_folders() {
        let pack_name = crate::param_files::file_name(&pack);
        for (directory, path) in param_folders(&pack) {
            let listing = entries(&path, false);
            for (lower, _) in &listing {
                if prc_patch::is_param_file(lower) && prc_patch::base_of(lower).is_none() {
                    shipped.insert(format!("{directory}/{lower}"));
                }
            }
            for (lower, actual) in &listing {
                let relative = format!("{directory}/{lower}");
                let Some(target) = prc_patch::base_of(&relative) else {
                    continue;
                };
                if prc_patch::read_by_dionysus(&relative) {
                    continue;
                }
                let Ok(bytes) = std::fs::read(path.join(actual)) else {
                    continue;
                };
                match prc_patch::read_patch(&bytes) {
                    Ok(root) => {
                        let file = wanted.entry(target).or_default();
                        file.0.push(root);
                        file.1.push(Patch {
                            pack: pack_name.clone(),
                            name: actual.clone(),
                        });
                    }
                    Err(why) => skyline::println!("[parampatch] {pack_name}: {relative}: {why}; the file is not used"),
                }
            }
        }
    }
    wanted
        .into_iter()
        .filter(|(target, _)| !shipped.contains(target))
        .map(|(target, (roots, patches))| Watched {
            hash: prc_patch::path_hash(&target),
            target,
            roots,
            patches,
        })
        .collect()
}

fn watched_slot(index: u32) -> Option<usize> {
    let watched = WATCHED.get()?;
    let mut indexes = INDEXES.lock().ok()?;
    for (slot, file) in watched.iter().enumerate() {
        let known = match indexes.get(slot).copied().flatten() {
            Some(known) => known,
            None => {
                let Some(found) = crate::fighter_modules::file_path_index(&file.target) else {
                    continue;
                };
                if let Some(entry) = indexes.get_mut(slot) {
                    *entry = Some(found);
                }
                found
            }
        };
        if known == index {
            return Some(slot);
        }
    }
    None
}

fn decompressed_size(hash: u64) -> Option<usize> {
    static FOUND: OnceLock<usize> = OnceLock::new();
    let address = *FOUND.get_or_init(|| {
        let mut address = 0usize;
        let result = unsafe {
            skyline::nn::ro::LookupSymbol(&mut address as *mut usize, b"arcrop_get_decompressed_size\0".as_ptr())
        };
        if result == 0 {
            address
        } else {
            0
        }
    });
    if address == 0 {
        return None;
    }
    let function: DecompressedSizeFn = unsafe { core::mem::transmute(address) };
    let mut size = 0usize;
    (unsafe { function(hash, &mut size) } && size > HEADER).then_some(size)
}

fn outcome_text(patch: &Patch, outcome: &Applied) -> String {
    match outcome {
        Applied::Values(count) => format!("{} sets {count}", patch.name),
        Applied::Mismatch(mismatch) => format!(
            "{} LEFT OUT: it has a {} at {} where the file has a {}",
            patch.name, mismatch.patch, mismatch.at, mismatch.file
        ),
    }
}

unsafe fn patched(slot: usize, data: usize) -> Option<&'static [u8]> {
    let mut built = BUILT.lock().ok()?;
    let built = built.get_or_insert_with(HashMap::new);
    if let Some(known) = built.get(&slot) {
        return *known;
    }
    let file = WATCHED.get()?.get(slot)?;
    let result = match decompressed_size(file.hash) {
        None => {
            skyline::println!("[parampatch] {}: its size is not known; read unpatched", file.target);
            None
        }
        Some(size) => {
            let source = core::slice::from_raw_parts(data as *const u8, size).to_vec();
            match prc_patch::patch_file(&source, &file.roots) {
                Ok((bytes, outcomes)) => {
                    let said: Vec<String> = file
                        .patches
                        .iter()
                        .zip(&outcomes)
                        .map(|(patch, outcome)| outcome_text(patch, outcome))
                        .collect();
                    skyline::println!("[parampatch] patched {} as the game read it ({})", file.target, said.join(", "));
                    Some(&*Box::leak(bytes.into_boxed_slice()))
                }
                Err(error) => {
                    skyline::println!("[parampatch] {}: not a param file ({error:?}); read unpatched", file.target);
                    None
                }
            }
        }
    };
    built.insert(slot, result);
    result
}

unsafe fn substitute(handle: usize) {
    let index = core::ptr::read_volatile((handle + HANDLE_INDEX) as *const u32);
    if index == NOT_FOUND {
        return;
    }
    let Some(slot) = watched_slot(index) else {
        return;
    };
    let data = core::ptr::read_volatile((handle + HANDLE_DATA) as *const usize);
    if data == 0 {
        return;
    }
    let Some(bytes) = patched(slot, data) else {
        return;
    };
    let base = bytes.as_ptr() as usize;
    let hashes = core::ptr::read_volatile((base + 8) as *const i32) as usize;
    let refs = core::ptr::read_volatile((base + 0xc) as *const i32) as usize;
    if HEADER + hashes + refs > bytes.len() {
        return;
    }
    core::ptr::write_volatile((handle + HANDLE_DATA) as *mut usize, base);
    core::ptr::write_volatile((handle + HANDLE_HASHES) as *mut usize, base + HEADER);
    core::ptr::write_volatile((handle + HANDLE_REFS) as *mut usize, base + HEADER + hashes);
    core::ptr::write_volatile((handle + HANDLE_PARAMS) as *mut usize, base + HEADER + hashes + refs);
}

#[skyline::hook(offset = crate::game::rel(PARAM_BIND))]
unsafe fn param_bind_hook(handle: *mut u8, index: *const u32) -> bool {
    let bound = call_original!(handle, index);
    if bound && !handle.is_null() && ARMED.load(Ordering::Acquire) {
        substitute(handle as usize);
    }
    bound
}
