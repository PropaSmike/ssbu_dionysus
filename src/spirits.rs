use std::collections::BTreeSet;
use std::sync::{Mutex, OnceLock};

use dionysus_core::prc;
use dionysus_core::spirit_prcxml;
use dionysus_core::spirits::{
    self, Assigned, BattleOutcome, Board, BoardOutcome, Declaration, Outcome, Registry, BATTLE_DB_PATH,
    BOARD_DB_PATH, DB_PATH, LAYOUT_DB_PATH,
};

const REGISTRY_DIRECTORY: &str = "sd:/ultimate/dionysus";
const REGISTRY_PATH: &str = "sd:/ultimate/dionysus/spirits.txt";
const BOARD_REGISTRY_PATH: &str = "sd:/ultimate/dionysus/spirit_boards.txt";
const BOARD_ROOM: usize = 64 << 10;
const BOARD_SPIRIT_ROOM: usize = 1 << 10;
const MANIFEST: &str = "spirit.toml";
const DB_ROOM: usize = 1 << 20;
const BATTLE_ROOM: usize = 2 << 20;
const LAYOUT_ROOM: usize = 1 << 20;
const SPIRIT_ROOM: usize = 2 << 10;
const BATTLE_SPIRIT_ROOM: usize = 12 << 10;

#[derive(Clone, Copy)]
enum File {
    Spirits,
    Battles,
    Layouts,
    Boards,
}

impl File {
    fn name(self) -> &'static str {
        match self {
            File::Spirits => "spirit",
            File::Battles => "spirit battle",
            File::Layouts => "spirit layout",
            File::Boards => "spirit board",
        }
    }
}

struct State {
    declarations: Vec<(String, Declaration)>,
    registry: Registry,
    boards: Vec<(String, Board)>,
    board_registry: Registry,
    added_boards: Option<Vec<(String, String, Option<String>)>>,
    known_spirits: Option<BTreeSet<u64>>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            declarations: Vec::new(),
            registry: Registry::default(),
            boards: Vec::new(),
            board_registry: Registry::default(),
            added_boards: None,
            known_spirits: None,
        })
    })
}

fn save_registry(path: &str, text: String) {
    let _ = std::fs::create_dir_all(REGISTRY_DIRECTORY);
    if let Err(error) = std::fs::write(path, text) {
        skyline::println!("[spirits] could not write {path} ({error}); the save numbers below hold for this boot only");
    }
}

fn board_pack_of<'a>(boards: &'a [(String, Board)], id: &str) -> &'a str {
    boards
        .iter()
        .find(|(_, b)| b.id == id)
        .map(|(pack, _)| pack.as_str())
        .unwrap_or("?")
}

fn report_boards(outcomes: &[BoardOutcome], boards: &[(String, Board)]) {
    for outcome in outcomes {
        match outcome {
            BoardOutcome::Added {
                id,
                save_no,
                assigned,
                spirits,
                unknown_spirits,
                unknown_background,
                left_out,
            } => {
                let how = match assigned {
                    Assigned::Kept(_) => "kept".to_string(),
                    Assigned::New(_) => "new".to_string(),
                    Assigned::Moved { from, .. } => format!("moved from {from}, which the game now uses"),
                };
                let mut notes = String::new();
                if !unknown_spirits.is_empty() {
                    notes.push_str(&format!(
                        "; left out, not spirits of the game or a pack: {}",
                        unknown_spirits.join(", ")
                    ));
                }
                if *unknown_background {
                    notes.push_str("; its background is not a board's, so it keeps its base's");
                }
                if !left_out.is_empty() {
                    notes.push_str(&format!("; left out: {}", left_out.join(", ")));
                }
                skyline::println!(
                    "[spirits] board {id} ({}) added as board number {save_no} with {spirits} spirit(s) ({how}){notes}",
                    board_pack_of(boards, id)
                );
            }
            BoardOutcome::UnknownBase { id, base } => skyline::println!(
                "[spirits] board {id} ({}) skipped: base {base} is not one of the game's boards",
                board_pack_of(boards, id)
            ),
            BoardOutcome::NoSpirits { id } => skyline::println!(
                "[spirits] board {id} ({}) skipped: none of its spirits is a spirit of the game or a pack",
                board_pack_of(boards, id)
            ),
            BoardOutcome::TakenId { id } => skyline::println!(
                "[spirits] board {id} ({}) skipped: a board already has that id",
                board_pack_of(boards, id)
            ),
            BoardOutcome::Full { id } => skyline::println!(
                "[spirits] board {id} ({}) skipped: every board number from {} to {} is taken",
                board_pack_of(boards, id),
                spirits::BOARD_SAVE_FIRST,
                spirits::BOARD_SAVE_LAST
            ),
        }
    }
}

pub(crate) fn board_count() -> i64 {
    state()
        .lock()
        .map(|state| match &state.added_boards {
            Some(added) => added.len() as i64,
            None => state.boards.len() as i64,
        })
        .unwrap_or(0)
}

pub(crate) fn board_text(name: &str, index: i64) -> Option<String> {
    let index = usize::try_from(index).ok()?;
    let state = state().lock().ok()?;
    let (id, title, art) = match &state.added_boards {
        Some(added) => added.get(index).cloned()?,
        None => state
            .boards
            .get(index)
            .map(|(_, board)| (board.id.clone(), board.display_name().to_string(), board.art_root()))?,
    };
    match name {
        "engine.spirit_board_id" => Some(id),
        "engine.spirit_board_name" => Some(title),
        "engine.spirit_board_art" => art,
        _ => None,
    }
}

fn pack_of<'a>(declarations: &'a [(String, Declaration)], id: &str) -> &'a str {
    declarations
        .iter()
        .find(|(_, d)| d.id == id)
        .map(|(pack, _)| pack.as_str())
        .unwrap_or("?")
}

fn left_out_text(left_out: &[String]) -> String {
    if left_out.is_empty() {
        String::new()
    } else {
        format!("; left out: {}", left_out.join(", "))
    }
}

fn report_spirits(outcomes: &[Outcome], declarations: &[(String, Declaration)]) {
    for outcome in outcomes {
        match outcome {
            Outcome::Added { id, save_no, assigned, left_out } => {
                let how = match assigned {
                    Assigned::Kept(_) => "kept".to_string(),
                    Assigned::New(_) => "new".to_string(),
                    Assigned::Moved { from, .. } => format!("moved from {from}, which the game now uses"),
                };
                skyline::println!(
                    "[spirits] {id} ({}) added as save number {save_no}, spirit no. {} ({how}){}",
                    pack_of(declarations, id),
                    save_no + 1,
                    left_out_text(left_out)
                );
            }
            Outcome::UnknownBase { id, base } => skyline::println!(
                "[spirits] {id} ({}) skipped: base {base} is not a spirit of the game's table",
                pack_of(declarations, id)
            ),
            Outcome::TakenId { id } => skyline::println!(
                "[spirits] {id} ({}) skipped: the game already has a spirit with that id",
                pack_of(declarations, id)
            ),
            Outcome::Full { id } => skyline::println!(
                "[spirits] {id} ({}) skipped: every spirit save number from {} to {} is taken",
                pack_of(declarations, id),
                spirits::SAVE_FIRST,
                spirits::SAVE_LAST
            ),
        }
    }
}

fn report_rows(file: File, outcomes: &[BattleOutcome], declarations: &[(String, Declaration)]) {
    for outcome in outcomes {
        match outcome {
            BattleOutcome::Added { id, left_out, .. } if !left_out.is_empty() => skyline::println!(
                "[spirits] {id} ({}): its {} row{}",
                pack_of(declarations, id),
                file.name(),
                left_out_text(left_out)
            ),
            BattleOutcome::Added { .. } => {}
            BattleOutcome::UnknownBase { id, base } => skyline::println!(
                "[spirits] {id} ({}) has no {} row: base {base} has none",
                pack_of(declarations, id),
                file.name()
            ),
            BattleOutcome::TakenId { id } => skyline::println!(
                "[spirits] {id} ({}): the game already has a {} row with that id, kept as it is",
                pack_of(declarations, id),
                file.name()
            ),
        }
    }
}

fn rebuild(file: File, bytes: &[u8]) -> Option<Vec<u8>> {
    let mut root = match prc::parse(bytes) {
        Ok(root) => root,
        Err(error) => {
            skyline::println!("[spirits] the game's {} table is not readable ({error:?}); served as it is", file.name());
            return None;
        }
    };
    let mut guard = state().lock().ok()?;
    let state = &mut *guard;
    if let File::Boards = file {
        let boards: Vec<&Board> = state.boards.iter().map(|(_, b)| b).collect();
        let before = state.board_registry.clone();
        let outcomes =
            spirits::append_boards(&mut root, &boards, &mut state.board_registry, state.known_spirits.as_ref());
        report_boards(&outcomes, &state.boards);
        if state.board_registry != before {
            save_registry(BOARD_REGISTRY_PATH, state.board_registry.serialise_boards());
        }
        let added: Vec<(String, String, Option<String>)> = outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                BoardOutcome::Added { id, .. } => state
                    .boards
                    .iter()
                    .find(|(_, board)| &board.id == id)
                    .map(|(_, board)| (id.clone(), board.display_name().to_string(), board.art_root())),
                _ => None,
            })
            .collect();
        let any = !added.is_empty();
        state.added_boards = Some(added);
        return any.then(|| prc::write(&root));
    }
    let declarations: Vec<&Declaration> = state.declarations.iter().map(|(_, d)| d).collect();
    if declarations.is_empty() {
        return None;
    }
    match file {
        File::Spirits => {
            let before = state.registry.clone();
            let outcomes = spirits::append_spirits(&mut root, &declarations, &mut state.registry);
            report_spirits(&outcomes, &state.declarations);
            if state.registry != before {
                save_registry(REGISTRY_PATH, state.registry.serialise());
            }
            state.known_spirits = Some(spirits::spirit_ids(&root));
            if !outcomes.iter().any(|o| matches!(o, Outcome::Added { .. })) {
                return None;
            }
        }
        File::Battles => {
            let outcomes = spirits::append_battles(&mut root, &declarations);
            report_rows(file, &outcomes, &state.declarations);
            if !outcomes.iter().any(|o| matches!(o, BattleOutcome::Added { .. })) {
                return None;
            }
        }
        File::Layouts => {
            let outcomes = spirits::append_layouts(&mut root, &declarations);
            report_rows(file, &outcomes, &state.declarations);
            if !outcomes.iter().any(|o| matches!(o, BattleOutcome::Added { .. })) {
                return None;
            }
        }
        File::Boards => return None,
    }
    Some(prc::write(&root))
}

fn serve(file: File, hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    if buffer.is_null() || length == 0 {
        return false;
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(buffer, length) };
    let Some(size) = arcropolis_api::load_original_file(hash, &mut *slice) else {
        skyline::println!("[spirits] the game's {} table could not be loaded; nothing added", file.name());
        return false;
    };
    let original = slice[..size].to_vec();
    match rebuild(file, &original) {
        Some(bytes) if bytes.len() <= length => {
            slice[..bytes.len()].copy_from_slice(&bytes);
            *out_size = bytes.len();
            skyline::println!(
                "[spirits] served the {} table: {size} bytes from the game and mods, {} with the packs' rows",
                file.name(),
                bytes.len()
            );
        }
        Some(bytes) => {
            skyline::println!(
                "[spirits] the {} table grew to {} bytes, past the {length} reserved; served without the packs' rows",
                file.name(),
                bytes.len()
            );
            *out_size = size;
        }
        None => *out_size = size,
    }
    true
}

extern "C" fn serve_spirits(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve(File::Spirits, hash, buffer, length, out_size)
}

extern "C" fn serve_battles(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve(File::Battles, hash, buffer, length, out_size)
}

extern "C" fn serve_layouts(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve(File::Layouts, hash, buffer, length, out_size)
}

extern "C" fn serve_boards(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    serve(File::Boards, hash, buffer, length, out_size)
}

fn discover() -> (Vec<(String, Declaration)>, Vec<(String, Board)>) {
    let Ok(entries) = std::fs::read_dir(crate::fighter_packs::MOD_ROOT) else {
        return (Vec::new(), Vec::new());
    };
    let mut manifests: Vec<(String, Option<String>, [Option<String>; 4])> = Vec::new();
    for entry in entries.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let toml = std::fs::read_to_string(directory.join(MANIFEST)).ok();
        let xml = spirit_prcxml::FILES.map(|file| std::fs::read_to_string(directory.join(file)).ok());
        if (toml.is_none() && xml.iter().all(Option::is_none)) || !crate::mod_folders::enabled(&directory) {
            continue;
        }
        manifests.push((
            directory
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            toml,
            xml,
        ));
    }
    manifests.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out: Vec<(String, Declaration)> = Vec::new();
    let mut boards: Vec<(String, Board)> = Vec::new();
    for (pack, toml, xml) in manifests {
        let mut manifest = spirits::Manifest::default();
        if let Some(text) = toml {
            match spirits::parse_manifest(&text) {
                Ok(read) => manifest = read,
                Err(error) => skyline::println!("[spirits] {pack}/{MANIFEST} is not readable: {error:?}; skipped"),
            }
        }
        let files = spirit_prcxml::Files {
            spirits: xml[0].as_deref(),
            battles: xml[1].as_deref(),
            layouts: xml[2].as_deref(),
            boards: xml[3].as_deref(),
        };
        if files.any() {
            match spirit_prcxml::parse(&files) {
                Ok(read) => {
                    skyline::println!(
                        "[spirits] {pack}: {} spirit(s) and {} board(s) from its spirit/ prcxml files",
                        read.spirits.len(),
                        read.boards.len()
                    );
                    manifest.spirits.extend(read.spirits);
                    manifest.boards.extend(read.boards);
                }
                Err(error) => skyline::println!("[spirits] {pack}: its spirit/ prcxml files are not readable: {error:?}; skipped"),
            }
        }
        for board in manifest.boards {
            if let Some((other, _)) = boards.iter().find(|(_, b)| b.id == board.id) {
                skyline::println!("[spirits] {pack}: board {} is already declared by {other}; skipped", board.id);
                continue;
            }
            if boards.len() >= spirits::MAX_BOARDS {
                skyline::println!(
                    "[spirits] {pack}: board {} skipped: {} boards are declared, as many as the save has numbers for",
                    board.id,
                    spirits::MAX_BOARDS
                );
                continue;
            }
            boards.push((pack.clone(), board));
        }
        for declaration in manifest.spirits {
            if let Some((other, _)) = out.iter().find(|(_, d)| d.id == declaration.id) {
                skyline::println!(
                    "[spirits] {pack}: spirit {} is already declared by {other}; skipped",
                    declaration.id
                );
                continue;
            }
            if out.len() >= spirits::MAX_SPIRITS {
                skyline::println!(
                    "[spirits] {pack}: spirit {} skipped: {} spirits are declared, as many as the save has numbers for",
                    declaration.id,
                    spirits::MAX_SPIRITS
                );
                continue;
            }
            out.push((pack.clone(), declaration));
        }
    }
    (out, boards)
}

pub(crate) fn load_all() {
    let (declarations, boards) = discover();
    if declarations.is_empty() && boards.is_empty() {
        return;
    }
    let registry = std::fs::read_to_string(REGISTRY_PATH)
        .map(|text| Registry::parse(&text))
        .unwrap_or_default();
    let board_registry = std::fs::read_to_string(BOARD_REGISTRY_PATH)
        .map(|text| Registry::parse_range(&text, spirits::BOARD_SAVE_FIRST, spirits::BOARD_SAVE_LAST))
        .unwrap_or_default();
    let count = declarations.len();
    let board_count = boards.len();
    let board_spirits: usize = boards.iter().map(|(_, b)| b.spirits.len()).sum();
    for (pack, board) in &boards {
        let Some(art) = &board.art else {
            continue;
        };
        match crate::menu_sequences::add_pack_layout(art) {
            Ok(()) => skyline::println!("[spirits] board {} ({pack}): art {art} joins the main menu", board.id),
            Err(refusal) => skyline::println!("[spirits] board {} ({pack}): art {art} REFUSED ({refusal:?})", board.id),
        }
    }
    let fighters: usize = declarations
        .iter()
        .map(|(_, d)| d.fighters.unwrap_or(4))
        .sum();
    if let Ok(mut state) = state().lock() {
        state.declarations = declarations;
        state.registry = registry;
        state.boards = boards;
        state.board_registry = board_registry;
    }
    if count > 0 {
        arcropolis_api::register_callback(DB_PATH, DB_ROOM + count * SPIRIT_ROOM, serve_spirits);
        arcropolis_api::register_callback(
            BATTLE_DB_PATH,
            BATTLE_ROOM + count * BATTLE_SPIRIT_ROOM + fighters * SPIRIT_ROOM,
            serve_battles,
        );
        arcropolis_api::register_callback(LAYOUT_DB_PATH, LAYOUT_ROOM + count * SPIRIT_ROOM, serve_layouts);
    }
    if board_count > 0 {
        arcropolis_api::register_callback(
            BOARD_DB_PATH,
            BOARD_ROOM + board_count * BOARD_SPIRIT_ROOM + board_spirits * 16,
            serve_boards,
        );
    }
    skyline::println!(
        "[spirits] {count} spirit(s) and {board_count} board(s) from spirit packs; rows are added when the game loads its tables"
    );
}
