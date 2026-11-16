use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::hash40::hash40;

const STAGE_MODELS: &str = "stage/settingstage/normal/model";
const SETS: [&str; 2] = ["stc_skydome_set", "stc_ring_set"];
const FILES: [&str; 7] = [
    "model.numshb",
    "model.numatb",
    "model.numdlb",
    "model.numshexb",
    "model.nusrcmdlb",
    "model.nusktb",
    "settingstage_white_con.nutexb",
];
const FOLDER: &str = "gallery_backdrop";
const ORIGINAL_LENGTHS: [(&str, usize); 14] = [
    ("stc_skydome_set/model.numshb", 59760),
    ("stc_skydome_set/model.numatb", 504),
    ("stc_skydome_set/model.numdlb", 208),
    ("stc_skydome_set/model.numshexb", 208),
    ("stc_skydome_set/model.nusrcmdlb", 208),
    ("stc_skydome_set/model.nusktb", 104),
    ("stc_skydome_set/settingstage_white_con.nutexb", 5296),
    ("stc_ring_set/model.numshb", 81552),
    ("stc_ring_set/model.numatb", 1008),
    ("stc_ring_set/model.numdlb", 307),
    ("stc_ring_set/model.numshexb", 288),
    ("stc_ring_set/model.nusrcmdlb", 307),
    ("stc_ring_set/model.nusktb", 104),
    ("stc_ring_set/settingstage_white_con.nutexb", 5296),
];
const PAGE: usize = 4096;

struct Patched {
    path: &'static str,
    length: usize,
    digest: u64,
    patches: &'static [(usize, &'static [u8])],
}

const ZERO_COUNT: &[u8] = &[0, 0, 0, 0];

const PATCHED: [Patched; 3] = [
    Patched {
        path: "stage/settingstage/normal/model/stc_skydome_set/model.numshb",
        length: 59760,
        digest: 0x3d41ad7fcf203683,
        patches: &[(276, ZERO_COUNT)],
    },
    Patched {
        path: "stage/settingstage/normal/model/stc_ring_set/model.numshb",
        length: 81552,
        digest: 0x26d8a81cd148a309,
        patches: &[(276, ZERO_COUNT), (484, ZERO_COUNT)],
    },
    Patched {
        path: "stage/settingstage/normal/render/render_param.prc",
        length: 4368,
        digest: 0x55602ec89bad3082,
        patches: &[
            (2792, &[0x29, 0x5c, 0x0f, 0x3d]),
            (2797, &[0x8f, 0xc2, 0x75, 0x3c]),
            (2802, &[0xae, 0x47, 0xe1, 0x3d]),
            (2970, &[0]),
        ],
    },
];

static REPLACEMENTS: OnceLock<HashMap<u64, (String, PathBuf)>> = OnceLock::new();

fn digest(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0xcbf29ce484222325u64, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3))
}

fn discover() -> HashMap<u64, (String, PathBuf)> {
    let mut found = HashMap::new();
    let Ok(mods) = std::fs::read_dir(crate::fighter_packs::MOD_ROOT) else {
        return found;
    };
    let mut folders: Vec<PathBuf> = mods.flatten().map(|entry| entry.path()).collect();
    folders.sort();
    for folder in folders {
        if !crate::mod_folders::enabled(&folder) {
            continue;
        }
        for set in SETS {
            for file in FILES {
                let source = folder.join(FOLDER).join(set).join(file);
                if source.is_file() {
                    let target = format!("{STAGE_MODELS}/{set}/{file}");
                    found.entry(hash40(&target)).or_insert((target, source));
                }
            }
        }
    }
    found
}

fn replacement(hash: u64) -> Option<&'static (String, PathBuf)> {
    REPLACEMENTS.get()?.get(&hash)
}

extern "C" fn serve(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    if buffer.is_null() || length == 0 {
        return false;
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(buffer, length) };
    if crate::menu_viewer::gallery_active() {
        if let Some((target, source)) = replacement(hash) {
            match std::fs::read(source) {
                Ok(bytes) if bytes.len() <= length => {
                    slice[..bytes.len()].copy_from_slice(&bytes);
                    *out_size = bytes.len();
                    skyline::println!("[menubackdrop] {target}: the pack's file, {} bytes", bytes.len());
                    return true;
                }
                Ok(bytes) => skyline::println!("[menubackdrop] {target}: {} bytes do not fit in {length}", bytes.len()),
                Err(error) => skyline::println!("[menubackdrop] {target}: {error}"),
            }
        }
    }
    let Some(size) = arcropolis_api::load_original_file(hash, &mut *slice) else {
        return false;
    };
    *out_size = size;
    if !crate::menu_viewer::gallery_active() {
        return true;
    }
    let Some(file) = PATCHED.iter().find(|file| hash40(file.path) == hash) else {
        return true;
    };
    if size != file.length || digest(&slice[..size]) != file.digest {
        skyline::println!("[menubackdrop] {} is not the game's file; left as it is", file.path);
        return true;
    }
    for (offset, bytes) in file.patches {
        slice[*offset..*offset + bytes.len()].copy_from_slice(bytes);
    }
    skyline::println!("[menubackdrop] {}: patched for the model viewer", file.path);
    true
}

fn capacity(path: &str, replacement: Option<&PathBuf>) -> usize {
    let original = ORIGINAL_LENGTHS
        .iter()
        .find(|(name, _)| path.ends_with(name))
        .map(|(_, length)| *length)
        .or_else(|| PATCHED.iter().find(|file| file.path == path).map(|file| file.length))
        .unwrap_or(1 << 18);
    let replaced = replacement
        .and_then(|source| std::fs::metadata(source).ok())
        .map_or(0, |meta| meta.len() as usize);
    original.max(replaced).div_ceil(PAGE) * PAGE
}

pub(crate) fn install() {
    let replacements = REPLACEMENTS.get_or_init(discover);
    let mut paths: Vec<(String, usize)> = PATCHED
        .iter()
        .map(|file| (file.path.to_string(), capacity(file.path, None)))
        .collect();
    for (target, source) in replacements.values() {
        paths.push((target.clone(), capacity(target, Some(source))));
    }
    paths.sort();
    paths.dedup_by(|later, earlier| {
        let same = later.0 == earlier.0;
        if same {
            earlier.1 = earlier.1.max(later.1);
        }
        same
    });
    for (path, reserved) in &paths {
        arcropolis_api::register_callback(path.as_str(), *reserved, serve);
    }
    let total: usize = paths.iter().map(|(_, reserved)| reserved).sum();
    skyline::println!(
        "[menubackdrop] {} file(s) watched for the model viewer, {} of them replaced by packs, {total} bytes reserved",
        paths.len(),
        replacements.len()
    );
}
