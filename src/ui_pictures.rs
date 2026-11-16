use std::collections::HashMap;
use std::sync::OnceLock;

const MOD_ROOT: &str = "sd:/ultimate/mods";
const TREES: [(&str, bool); 2] = [("replace", false), ("replace_patch", true)];

#[derive(Default)]
struct Found {
    chara: HashMap<String, (bool, bool)>,
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
                            if let Some(name) = dionysus_core::ui_pictures::chara_name(&file_name) {
                                note(&mut found.chara, name, patch);
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

fn trees(map: &HashMap<String, (bool, bool)>, name: &str) -> (bool, bool) {
    map.get(&name.to_ascii_lowercase()).copied().unwrap_or_default()
}

#[cfg(feature = "css_slot")]
pub(crate) fn chara_patch_flag(name: &str, set: Option<bool>, clone_from: Option<u64>) -> Option<bool> {
    let base_patch = clone_from.is_some_and(|base| {
        dionysus_core::ui_pictures::PATCH_ROWS
            .iter()
            .any(|row| crate::hash40::hash40(row) == base)
    });
    let (in_replace, in_patch) = trees(&found().chara, name);
    let patch =
        dionysus_core::ui_pictures::row_patch_flag(set, base_patch, in_replace, in_patch)?;
    crate::dbg_log_public(&format!(
        "[cskrow] {name}: select-screen pictures only under ui/{}/chara, so its row reads that tree",
        if patch { "replace_patch" } else { "replace" }
    ));
    Some(patch)
}

#[cfg(feature = "stage_slot")]
pub(crate) fn stage_reads_patch_tree(name: &str) -> bool {
    let (in_replace, in_patch) = trees(&found().stage, name);
    dionysus_core::ui_pictures::row_patch_flag(Some(false), false, in_replace, in_patch)
        .unwrap_or(false)
}
