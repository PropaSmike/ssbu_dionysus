use std::collections::HashMap;
#[cfg(feature = "css_slot")]
use std::collections::HashSet;
#[cfg(feature = "css_slot")]
use std::sync::Mutex;
use std::sync::OnceLock;

use dionysus_core::ui_pictures::TreePictures;

const MOD_ROOT: &str = "sd:/ultimate/mods";
const TREES: [(&str, bool); 2] = [("replace", false), ("replace_patch", true)];

#[derive(Default)]
struct Found {
    chara: HashMap<String, [TreePictures; 2]>,
    stage: HashMap<String, (bool, bool)>,
}

fn found() -> &'static Found {
    static FOUND: OnceLock<Found> = OnceLock::new();
    FOUND.get_or_init(scan)
}

fn note(map: &mut HashMap<String, (bool, bool)>, name: &str, patch: bool) {
    let seen = map.entry(name.to_string()).or_default();
    if patch {
        seen.1 = true;
    } else {
        seen.0 = true;
    }
}

fn scan() -> Found {
    let mut found = Found::default();
    let Ok(mods) = std::fs::read_dir(MOD_ROOT) else {
        return found;
    };
    for pack in mods.flatten() {
        if !crate::mod_folders::enabled(&pack.path()) {
            continue;
        }
        for (tree, patch) in TREES {
            for kind in ["chara", "stage"] {
                let Ok(folders) = std::fs::read_dir(pack.path().join("ui").join(tree).join(kind)) else {
                    continue;
                };
                for folder in folders.flatten() {
                    let Ok(files) = std::fs::read_dir(folder.path()) else {
                        continue;
                    };
                    for file in files.flatten() {
                        let file_name = file.file_name().to_string_lossy().to_ascii_lowercase();
                        if kind == "chara" {
                            if let Some((slot, name, costume)) = dionysus_core::ui_pictures::chara_parts(&file_name) {
                                found.chara.entry(name.to_string()).or_default()[usize::from(patch)].add(slot, costume);
                            }
                        } else if let Some(name) = dionysus_core::ui_pictures::stage_name(&file_name) {
                            note(&mut found.stage, name, patch);
                        }
                    }
                }
            }
        }
    }
    found
}

#[cfg(feature = "stage_slot")]
fn trees(map: &HashMap<String, (bool, bool)>, name: &str) -> (bool, bool) {
    map.get(&name.to_ascii_lowercase()).copied().unwrap_or_default()
}

#[cfg(feature = "css_slot")]
fn chara_trees(name: &str) -> [TreePictures; 2] {
    found().chara.get(&name.to_ascii_lowercase()).cloned().unwrap_or_default()
}

#[cfg(feature = "css_slot")]
fn base_reads_patch(clone_from: Option<u64>) -> bool {
    clone_from.is_some_and(|base| {
        dionysus_core::ui_pictures::PATCH_ROWS
            .iter()
            .any(|row| crate::hash40::hash40(row) == base)
    })
}

#[cfg(feature = "css_slot")]
pub(crate) fn chara_patch_flag(name: &str, set: Option<bool>, clone_from: Option<u64>) -> Option<bool> {
    let [replace, patch] = chara_trees(name);
    dionysus_core::ui_pictures::row_patch_flag(set, base_reads_patch(clone_from), replace.files > 0, patch.files > 0)
}

#[cfg(feature = "css_slot")]
pub(crate) fn report_chara(name: &str, reads_patch: Option<bool>, clone_from: Option<u64>) {
    static REPORTED: Mutex<Option<HashSet<String>>> = Mutex::new(None);
    {
        let Ok(mut reported) = REPORTED.lock() else {
            return;
        };
        if !reported.get_or_insert_with(HashSet::new).insert(name.to_ascii_lowercase()) {
            return;
        }
    }
    let base = base_reads_patch(clone_from);
    let [replace, patch] = chara_trees(name);
    crate::dbg_log_public(&dionysus_core::ui_pictures::picture_report(
        name,
        &replace,
        &patch,
        reads_patch.unwrap_or(base),
        base,
    ));
}

#[cfg(feature = "stage_slot")]
pub(crate) fn stage_reads_patch_tree(name: &str) -> bool {
    let (in_replace, in_patch) = trees(&found().stage, name);
    dionysus_core::ui_pictures::row_patch_flag(Some(false), false, in_replace, in_patch)
        .unwrap_or(false)
}
