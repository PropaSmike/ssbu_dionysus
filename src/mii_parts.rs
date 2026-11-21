use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use dionysus_core::mii_parts::{
    self, Added, Assigned, Declaration, MiiFighter, Outcome, Part, PartSource, Registry, HASH_MASK,
    HAT_DB_PATH, OUTFIT_DB_PATH,
};
use dionysus_core::prc;

const REGISTRY_DIRECTORY: &str = "sd:/ultimate/dionysus";
const REGISTRY_PATH: &str = "sd:/ultimate/dionysus/mii_parts.txt";
const MANIFEST: &str = "mii.toml";
const SERVE_CAPACITY: usize = 4 << 20;
const OFF_HAT_OWNED: usize = 0x179_7ab0;
const OFF_OUTFIT_OWNED: usize = 0x179_7ba0;
const OWNED_OPCODE: u32 = 0xf81d_0ff5;

struct State {
    declarations: Vec<(String, Declaration)>,
    registry: Registry,
    owned: [Vec<u64>; 2],
    base_ui: Vec<(Part, String, u64)>,
}

static REFUSED: AtomicBool = AtomicBool::new(false);

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            declarations: Vec::new(),
            registry: Registry::default(),
            owned: [Vec::new(), Vec::new()],
            base_ui: Vec::new(),
        })
    })
}

fn slot(part: Part) -> usize {
    match part {
        Part::Hat => 0,
        Part::Outfit => 1,
    }
}

pub(crate) fn refused() -> bool {
    REFUSED.load(Ordering::Acquire)
}

fn owned(part: Part, hash: u64) -> bool {
    let hash = hash & HASH_MASK;
    state()
        .lock()
        .map(|state| state.owned[slot(part)].contains(&hash))
        .unwrap_or(false)
}

fn save_registry(registry: &Registry) {
    let _ = std::fs::create_dir_all(REGISTRY_DIRECTORY);
    if let Err(error) = std::fs::write(REGISTRY_PATH, registry.serialise()) {
        skyline::println!(
            "[miiparts] could not write {REGISTRY_PATH} ({error}); the save numbers below hold for this boot only"
        );
    }
}

fn pack_of<'a>(declarations: &'a [(String, Declaration)], part: Part, id: &str) -> &'a str {
    declarations
        .iter()
        .find(|(_, d)| d.part == part && d.id == id)
        .map(|(pack, _)| pack.as_str())
        .unwrap_or("?")
}

fn report(part: Part, outcomes: &[Outcome], declarations: &[(String, Declaration)]) {
    let kind = part.name();
    for outcome in outcomes {
        match outcome {
            Outcome::Added { id, save_no, assigned } => {
                let pack = pack_of(declarations, part, id);
                let how = match assigned {
                    Assigned::Kept(_) => "kept".to_string(),
                    Assigned::New(_) => "new".to_string(),
                    Assigned::Moved { from, .. } => format!("moved from {from}, which the game now uses"),
                };
                skyline::println!("[miiparts] {kind} {id} ({pack}) added as save number {save_no} ({how})");
            }
            Outcome::UnknownBase { id, base } => skyline::println!(
                "[miiparts] {kind} {id} ({}) skipped: base {base} is not a row of the game's {kind} table",
                pack_of(declarations, part, id)
            ),
            Outcome::TakenId { id } => skyline::println!(
                "[miiparts] {kind} {id} ({}) skipped: the game already has a {kind} with that id",
                pack_of(declarations, part, id)
            ),
            Outcome::Full { id } => skyline::println!(
                "[miiparts] {kind} {id} ({}) skipped: every {kind} save number up to {} is taken",
                pack_of(declarations, part, id),
                part.last_save_no()
            ),
        }
    }
}

fn rebuild(part: Part, bytes: &[u8]) -> Option<Vec<u8>> {
    let mut root = match prc::parse(bytes) {
        Ok(root) => root,
        Err(error) => {
            skyline::println!("[miiparts] the game's {} table is not readable ({error:?}); served as it is", part.name());
            return None;
        }
    };
    let mut guard = state().lock().ok()?;
    let state = &mut *guard;
    let mut bases: Vec<(Part, String, u64)> = Vec::new();
    for (_, d) in state.declarations.iter().filter(|(_, d)| d.part == part) {
        if let Some(hash) = mii_parts::base_ui_hash(&root, part, &d.base, d.fighter) {
            bases.push((part, d.id.clone(), hash));
        }
    }
    for entry in state.registry.entries.iter().filter(|e| e.part == part) {
        if !bases.iter().any(|(_, id, _)| *id == entry.id) {
            if let Some(hash) = mii_parts::base_ui_hash(&root, part, &entry.base, None) {
                bases.push((part, entry.id.clone(), hash));
            }
        }
    }
    state.base_ui.retain(|(p, _, _)| *p != part);
    state.base_ui.extend(bases);
    let before = state.registry.clone();
    let declarations: Vec<&Declaration> = state.declarations.iter().map(|(_, d)| d).collect();
    let (outcomes, added): (Vec<Outcome>, Vec<Added>) =
        mii_parts::append(&mut root, part, &declarations, &mut state.registry);
    report(part, &outcomes, &state.declarations);
    state.owned[slot(part)] = added.iter().filter(|a| a.owned).map(|a| a.ui_hash).collect();
    let ghosts = mii_parts::add_ghosts(&mut root, part, &state.registry);
    for entry in &ghosts {
        skyline::println!(
            "[miiparts] {} {} (save number {}) is not installed; a Mii wearing it shows {} instead",
            part.name(),
            entry.id,
            entry.save_no,
            entry.base
        );
    }
    if state.registry != before {
        save_registry(&state.registry);
    }
    if added.is_empty() && ghosts.is_empty() {
        return None;
    }
    Some(prc::write(&root))
}

fn serve(part: Part, hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    if buffer.is_null() || length == 0 {
        return false;
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(buffer, length) };
    let Some(size) = arcropolis_api::load_original_file(hash, &mut *slice) else {
        skyline::println!("[miiparts] the game's {} table could not be loaded; nothing added", part.name());
        return false;
    };
    let original = slice[..size].to_vec();
    match rebuild(part, &original) {
        Some(bytes) if bytes.len() <= length => {
            slice[..bytes.len()].copy_from_slice(&bytes);
            *out_size = bytes.len();
            skyline::println!(
                "[miiparts] served the {} table: {size} bytes from the game and mods, {} with the packs' rows and placeholders",
                part.name(),
                bytes.len()
            );
        }
        Some(bytes) => {
            skyline::println!(
                "[miiparts] the {} table grew to {} bytes, past the {length} reserved; served without the packs' rows",
                part.name(),
                bytes.len()
            );
            *out_size = size;
        }
        None => *out_size = size,
    }
    true
}

fn part_sources() -> Vec<(PartSource, bool)> {
    let Ok(state) = state().lock() else {
        return Vec::new();
    };
    let base_hash = |part: Part, id: &str, base: &str| {
        state
            .base_ui
            .iter()
            .find(|(p, known, _)| *p == part && known == id)
            .map(|(_, _, hash)| *hash)
            .unwrap_or_else(|| mii_parts::part_ui_hash(part, base))
    };
    let mut out: Vec<(PartSource, bool)> = state
        .declarations
        .iter()
        .map(|(_, d)| {
            let source = PartSource {
                part: d.part,
                id: d.id.clone(),
                base: d.base.clone(),
                base_hash: base_hash(d.part, &d.id, &d.base),
            };
            (source, d.part == Part::Outfit && d.fighter == Some(MiiFighter::Swordfighter))
        })
        .collect();
    for entry in &state.registry.entries {
        if !out.iter().any(|(source, _)| source.part == entry.part && source.id == entry.id) {
            let source = PartSource {
                part: entry.part,
                id: entry.id.clone(),
                base: entry.base.clone(),
                base_hash: base_hash(entry.part, &entry.id, &entry.base),
            };
            out.push((source, false));
        }
    }
    out
}

fn serve_part_rows(path: &str, hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    if buffer.is_null() || length == 0 {
        return false;
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(buffer, length) };
    let Some(size) = arcropolis_api::load_original_file(hash, &mut *slice) else {
        skyline::println!("[miiparts] {path} could not be loaded; no hat or outfit rows added to it");
        return false;
    };
    *out_size = size;
    let mut root = match prc::parse(&slice[..size]) {
        Ok(root) => root,
        Err(error) => {
            skyline::println!("[miiparts] {path} is not readable ({error:?}); served as it is");
            return true;
        }
    };
    let sources = part_sources();
    let parts: Vec<PartSource> = sources.iter().map(|(source, _)| source.clone()).collect();
    let rows = mii_parts::add_part_rows(&mut root, &parts);
    let swordfighter = path.contains("miiswordsman");
    for (row, (_, sword)) in rows.iter().zip(&sources) {
        if row.copied > 0 {
            skyline::println!(
                "[miiparts] {} {} takes {}'s {} row(s) in {path}",
                row.part.name(),
                row.id,
                row.base,
                row.copied
            );
        } else if swordfighter && *sword && !row.own {
            skyline::println!(
                "[miiparts] outfit {}: base {} has no sword trail row in {path}, so it has no trail",
                row.id,
                row.base
            );
        }
    }
    if rows.iter().all(|row| row.copied == 0) {
        return true;
    }
    let bytes = prc::write(&root);
    if bytes.len() > length {
        skyline::println!(
            "[miiparts] {path} grew to {} bytes, past the {length} reserved; served without the hats' and outfits' rows",
            bytes.len()
        );
        return true;
    }
    slice[..bytes.len()].copy_from_slice(&bytes);
    *out_size = bytes.len();
    true
}

extern "C" fn serve_brawler_params(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve_part_rows(mii_parts::MII_PARAM_PATHS[0], hash, buffer, length, out_size)
}

extern "C" fn serve_swordfighter_params(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve_part_rows(mii_parts::MII_PARAM_PATHS[1], hash, buffer, length, out_size)
}

extern "C" fn serve_gunner_params(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve_part_rows(mii_parts::MII_PARAM_PATHS[2], hash, buffer, length, out_size)
}

extern "C" fn serve_hats(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve(Part::Hat, hash, buffer, length, out_size)
}

extern "C" fn serve_outfits(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve(Part::Outfit, hash, buffer, length, out_size)
}

#[skyline::hook(offset = crate::game::rel(OFF_HAT_OWNED))]
unsafe fn hat_owned(hash: u64) -> bool {
    owned(Part::Hat, hash) || call_original!(hash)
}

#[skyline::hook(offset = crate::game::rel(OFF_OUTFIT_OWNED))]
unsafe fn outfit_owned(hash: u64) -> bool {
    owned(Part::Outfit, hash) || call_original!(hash)
}

fn discover() -> Vec<(String, Declaration)> {
    let Ok(entries) = std::fs::read_dir(crate::fighter_packs::MOD_ROOT) else {
        return Vec::new();
    };
    let mut manifests: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(directory.join(MANIFEST)) else {
            continue;
        };
        if !crate::mod_folders::enabled(&directory) {
            continue;
        }
        manifests.push((
            directory
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            text,
        ));
    }
    manifests.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out: Vec<(String, Declaration)> = Vec::new();
    for (pack, text) in manifests {
        let parsed = match mii_parts::parse(&text) {
            Ok(parsed) => parsed,
            Err(error) => {
                skyline::println!("[miiparts] {pack}/{MANIFEST} is not readable: {error:?}; skipped");
                continue;
            }
        };
        for declaration in parsed {
            if let Some((other, _)) = out
                .iter()
                .find(|(_, d)| d.part == declaration.part && d.id == declaration.id)
            {
                skyline::println!(
                    "[miiparts] {pack}: {} {} is already declared by {other}; skipped",
                    declaration.part.name(),
                    declaration.id
                );
                continue;
            }
            out.push((pack.clone(), declaration));
        }
    }
    out
}

pub(crate) fn load_all() {
    crate::upgrade::carry_old_folder();
    let declarations = discover();
    let registry = std::fs::read_to_string(REGISTRY_PATH)
        .map(|text| Registry::parse(&text))
        .unwrap_or_default();
    if declarations.is_empty() && registry.entries.is_empty() {
        return;
    }
    let hats = declarations.iter().filter(|(_, d)| d.part == Part::Hat).count();
    let outfits = declarations.len() - hats;
    let sites = [
        (OFF_HAT_OWNED, OWNED_OPCODE),
        (OFF_OUTFIT_OWNED, OWNED_OPCODE),
    ];
    for (offset, expected) in sites {
        let found = unsafe { crate::game::text_word(offset) };
        if found != expected {
            skyline::println!(
                "[miiparts] REFUSED: {offset:#x} holds {found:#010x}, not {expected:#010x}; no Mii hat or outfit is added"
            );
            REFUSED.store(true, Ordering::Release);
            return;
        }
    }
    if let Ok(mut state) = state().lock() {
        state.declarations = declarations;
        state.registry = registry;
    }
    arcropolis_api::register_callback(HAT_DB_PATH, SERVE_CAPACITY, serve_hats);
    arcropolis_api::register_callback(OUTFIT_DB_PATH, SERVE_CAPACITY, serve_outfits);
    let params: [extern "C" fn(u64, *mut u8, usize, &mut usize) -> bool; 3] =
        [serve_brawler_params, serve_swordfighter_params, serve_gunner_params];
    for (path, serve) in mii_parts::MII_PARAM_PATHS.into_iter().zip(params) {
        arcropolis_api::register_callback(path, SERVE_CAPACITY, serve);
    }
    skyline::install_hooks!(hat_owned, outfit_owned);
    skyline::println!(
        "[miiparts] {hats} hat(s) and {outfits} outfit(s) from mii.toml packs; rows are added when the game loads its tables"
    );
}
