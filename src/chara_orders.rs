use dionysus_core::csk_database::{
    chara_disp_orders, prc_disp_orders, prc_duplicate_rows, translate_disp_order, vanilla_disp_orders,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

const MODS_DATABASE: &str = "mods:/database";
const MODS_CHARA_DB: &str = "mods:/ui/param/database/ui_chara_db.prc";
const CHARA_DB: &str = "ui/param/database/ui_chara_db.prc";
const POLL_MS: u64 = 50;
const POLL_LIMIT: u32 = 20 * 60 * 20;

static JSON: RwLock<Option<HashMap<u64, i8>>> = RwLock::new(None);
static TABLE: RwLock<Option<HashMap<u64, i8>>> = RwLock::new(None);
static WATCHING: AtomicBool = AtomicBool::new(false);

fn json_files(directory: &Path) -> Option<Vec<PathBuf>> {
    let entries = std::fs::read_dir(directory).ok()?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .collect();
    files.sort();
    Some(files)
}

fn read_orders(files: &[PathBuf]) -> HashMap<u64, i8> {
    let mut orders = HashMap::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(file) else {
            crate::dbg_log_public(&format!("[cskorder] {} could not be read", file.display()));
            continue;
        };
        match chara_disp_orders(&text) {
            Ok(found) => {
                for refused in &found.refused {
                    crate::dbg_log_public(&format!("[cskorder] {}: {refused}", file.display()));
                }
                for (ui_chara, order) in found.orders {
                    orders.insert(ui_chara, order);
                }
            }
            Err(error) => crate::dbg_log_public(&format!(
                "[cskorder] {}: not JSON at byte {} ({}); no order taken from it",
                file.display(),
                error.offset,
                error.message
            )),
        }
    }
    crate::dbg_log_public(&format!(
        "[cskorder] {MODS_DATABASE}: {} file(s), {} character disp_order(s)",
        files.len(),
        orders.len()
    ));
    orders
}

fn json_orders() -> HashMap<u64, i8> {
    if let Some(orders) = JSON.read().ok().and_then(|held| held.clone()) {
        return orders;
    }
    let orders = match json_files(Path::new(MODS_DATABASE)) {
        Some(files) => read_orders(&files),
        None => HashMap::new(),
    };
    if let Ok(mut held) = JSON.write() {
        held.get_or_insert(orders.clone());
    }
    orders
}

fn table_orders(bytes: &[u8], source: &str) -> Option<HashMap<u64, i8>> {
    for (ui_chara, name, count) in prc_duplicate_rows(bytes).unwrap_or_default() {
        crate::dbg_log_public(&format!(
            "[cskorder] {source} lists {name:?} ({ui_chara:#x}) {count} times; its first row counts, as in the game. Two mods both add this fighter: keep one row"
        ));
    }
    match prc_disp_orders(bytes) {
        Ok(found) => Some(found.into_iter().collect()),
        Err(error) => {
            crate::dbg_log_public(&format!(
                "[cskorder] {source} is not a readable ui_chara_db ({error:?}); no order taken from it"
            ));
            None
        }
    }
}

fn held(table: &RwLock<Option<HashMap<u64, i8>>>) -> Option<HashMap<u64, i8>> {
    table.read().ok().and_then(|held| held.clone())
}

fn store(table: &RwLock<Option<HashMap<u64, i8>>>, orders: HashMap<u64, i8>) {
    if let Ok(mut held) = table.write() {
        *held = Some(orders);
    }
}

pub(crate) fn player_disp_order(ui_chara: u64) -> Option<i8> {
    if !crate::mount_events::mods_fs_mounted() {
        return None;
    }
    if let Some(order) = json_orders().get(&ui_chara) {
        return Some(*order);
    }
    held(&TABLE)?.get(&ui_chara).copied()
}

pub(crate) fn translated(order: i8) -> i8 {
    let Some(mut player) = held(&TABLE) else {
        return order;
    };
    if crate::mount_events::mods_fs_mounted() {
        player.extend(json_orders());
    }
    translate_disp_order(order, &vanilla_disp_orders(), &player)
}

fn read_loaded_table() -> bool {
    let hash = crate::hash40::hash40(CHARA_DB);
    let Some(index) = (unsafe { crate::arc_files::file_path_index(hash) }) else {
        return false;
    };
    let Some(bytes) = (unsafe { crate::arc_files::loaded_copy(hash, index) }) else {
        return false;
    };
    if let Some(orders) = table_orders(&bytes, "the loaded ui_chara_db") {
        crate::dbg_log_public(&format!(
            "[cskorder] the game loaded {CHARA_DB} with {} rows, every mod's file and patch applied; a clone's row there is its place, and the rest follow its numbering",
            orders.len()
        ));
        store(&TABLE, orders);
        crate::csk_row_bridge::apply_player_orders();
    }
    true
}

pub(crate) fn watch() {
    if !crate::mount_events::mods_fs_mounted() || WATCHING.swap(true, Ordering::AcqRel) {
        return;
    }
    if let Some(orders) = std::fs::read(MODS_CHARA_DB).ok().and_then(|bytes| table_orders(&bytes, MODS_CHARA_DB)) {
        crate::dbg_log_public(&format!("[cskorder] {MODS_CHARA_DB}: {} rows, used until the game loads its table", orders.len()));
        store(&TABLE, orders);
    }
    std::thread::spawn(|| {
        for _ in 0..POLL_LIMIT {
            if read_loaded_table() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(POLL_MS));
        }
        crate::dbg_log_public(&format!("[cskorder] the game never loaded {CHARA_DB}; orders come from the files read at mount"));
    });
}
