#![allow(dead_code)]

use dionysus_core::fighter_toml::{ParamDeclaration, ParamOp};
use dionysus_core::prcxml::{self, Node, Skipped};
use std::path::{Path, PathBuf};

const MOD_ROOT: &str = "sd:/ultimate/mods";
const SHOWN: usize = 6;

pub(crate) fn path(directory: &str, parts: &[&str]) -> PathBuf {
    let mut path = Path::new(MOD_ROOT).join(directory);
    for part in parts {
        path.push(part);
    }
    path
}

pub(crate) fn read(path: &Path, tag: &str, directory: &str) -> Option<Node> {
    let text = std::fs::read_to_string(path).ok()?;
    match prcxml::parse(&text) {
        Ok(root) => Some(root),
        Err(error) => {
            skyline::println!(
                "[{tag}] {directory}: {} line {}: {}; the file is not used",
                file_name(path),
                error.line,
                error.message
            );
            None
        }
    }
}

pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub(crate) fn left_out(skipped: &[Skipped]) -> String {
    let mut parts: Vec<String> = skipped
        .iter()
        .take(SHOWN)
        .map(|skip| format!("{} ({})", skip.field, skip.why.text()))
        .collect();
    if skipped.len() > SHOWN {
        parts.push(format!("{} more", skipped.len() - SHOWN));
    }
    parts.join(", ")
}

pub(crate) fn summary(set: usize, kept_back: usize, skipped: &[Skipped], rows: Option<usize>) -> String {
    let mut text = format!("sets {set}");
    if kept_back > 0 {
        text.push_str(&format!(", {kept_back} left to the toml"));
    }
    match rows {
        Some(0) => text.push_str("; it has no row"),
        Some(rows) if rows > 1 => text.push_str(&format!("; only the first of its {rows} rows is read")),
        _ => {}
    }
    if !skipped.is_empty() {
        text.push_str(&format!("; left out: {}", left_out(skipped)));
    }
    text
}

pub(crate) fn fighter_params(directory: &str, resource: &str, declared: &[ParamDeclaration]) -> Vec<ParamDeclaration> {
    let mut out: Vec<ParamDeclaration> = Vec::new();
    for file in prcxml::FIGHTER_FILES {
        let name = format!("{file}.prcxml");
        let file_path = path(directory, &["fighter", resource, "param", &name]);
        let Some(root) = read(&file_path, "fighterpack", directory) else {
            continue;
        };
        let Some(found) = prcxml::fighter_params(file, &root) else {
            continue;
        };
        let mut set = 0;
        let mut kept_back = 0;
        for (param, value) in found.params {
            if declared.iter().any(|known| known.slot.is_none() && known.name == param) {
                kept_back += 1;
                continue;
            }
            out.retain(|known| known.name != param);
            out.push(ParamDeclaration {
                name: param,
                slot: None,
                op: ParamOp::Set,
                value,
            });
            set += 1;
        }
        let rows = matches!(file, "fighter_param" | "fighter_param_motion").then_some(found.rows);
        skyline::println!(
            "[fighterpack] {directory}: {name} {}",
            summary(set, kept_back, &found.skipped, rows)
        );
    }
    out
}

pub(crate) fn fighter_vl(directory: &str, resource: &str, declared: &[ParamDeclaration]) -> Vec<ParamDeclaration> {
    let name = format!("{}.prcxml", prcxml::VL_FILE);
    let file_path = path(directory, &["fighter", resource, "param", &name]);
    let Some(root) = read(&file_path, "fighterpack", directory) else {
        return Vec::new();
    };
    let found = prcxml::vl_params(&root);
    let taken: Vec<(u64, u64)> = declared
        .iter()
        .filter(|known| known.slot.is_none())
        .map(|known| prcxml::param_key(&known.name))
        .collect();
    let mut out: Vec<ParamDeclaration> = Vec::new();
    let mut kept_back = 0;
    for (param, value) in found.params {
        if taken.contains(&prcxml::param_key(&param)) {
            kept_back += 1;
            continue;
        }
        out.push(ParamDeclaration {
            name: param,
            slot: None,
            op: ParamOp::Set,
            value,
        });
    }
    skyline::println!(
        "[fighterpack] {directory}: {name} {}",
        summary(out.len(), kept_back, &found.skipped, None)
    );
    out
}

pub(crate) fn item_file(directory: &str, resource: &str) -> Option<PathBuf> {
    item_named_file(directory, resource, &format!("{}.prcxml", prcxml::ITEM_FILE))
}

pub(crate) fn item_named_file(directory: &str, resource: &str, name: &str) -> Option<PathBuf> {
    let inside = |root: &Path| {
        ["item", "assist", "pokemon", "boss"]
            .iter()
            .map(|top| root.join(top).join(resource).join("param").join(name))
            .find(|candidate| candidate.is_file())
    };
    inside(&Path::new(MOD_ROOT).join(directory)).or_else(|| enabled_folders().find_map(|folder| inside(&folder)))
}

fn enabled_folders() -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(MOD_ROOT)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|folder| folder.is_dir() && crate::mod_folders::enabled(folder))
}

pub(crate) fn find_in_mods(parts: &[&str]) -> Option<PathBuf> {
    enabled_folders().find_map(|folder| {
        let mut candidate = folder;
        for part in parts {
            candidate.push(part);
        }
        candidate.is_file().then_some(candidate)
    })
}

fn stage_file(directory: &str, places: &[&str], tree: &str, file: &str) -> Option<(String, Node)> {
    let name = format!("{file}.prcxml");
    let file_path = places
        .iter()
        .map(|place| path(directory, &["stage", place, tree, "param", &name]))
        .find(|candidate| candidate.is_file())?;
    let root = read(&file_path, "stagepack", directory)?;
    Some((name, root))
}

pub(crate) fn stage_item_table(directory: &str, places: &[&str], tree: &str) -> Option<Vec<(u16, u8)>> {
    let (name, root) = stage_file(directory, places, tree, prcxml::STAGE_FILE)?;
    let table = prcxml::item_switch_table(&root);
    if table.is_none() {
        skyline::println!("[stagepack] {directory}: {name} has no itsw_normal row with a table; the file is not used");
    }
    table
}

pub(crate) fn stage_sound(directory: &str, places: &[&str], tree: &str) -> Option<prcxml::StageSound> {
    let (name, root) = stage_file(directory, places, tree, prcxml::STAGE_SOUND_FILE)?;
    let found = prcxml::stage_sound(&root);
    let set = [
        found.sound.reverb.is_some(),
        found.sound.scale.is_some(),
        found.sound.offset.is_some(),
    ]
    .iter()
    .filter(|set| **set)
    .count();
    skyline::println!(
        "[stagepack] {directory}: {name} {}",
        summary(set, 0, &found.skipped, Some(found.rows))
    );
    (!found.sound.is_empty()).then_some(found.sound)
}

pub(crate) fn stage_area(directory: &str, places: &[&str], tree: &str) -> Option<prcxml::Area> {
    let (name, root) = stage_file(directory, places, tree, prcxml::STAGE_AREA_FILE)?;
    let found = prcxml::stage_area(&root);
    let mut text = summary(
        if found.area.is_some() { 4 } else { 0 },
        0,
        &found.skipped,
        Some(found.rows),
    );
    if found.area.is_none() && found.rows > 0 {
        text.push_str(&format!(
            "; it needs {} too, so the file is not used",
            found.missing.join(", ")
        ));
    }
    skyline::println!("[stagepack] {directory}: {name} {text}");
    found.area
}

#[cfg(feature = "css_slot")]
pub(crate) fn asset_params(path: &Path, tag: &str, label: &str, space: crate::custom_params::Space, owner: u64) {
    let Some(root) = read(path, tag, label) else {
        return;
    };
    let name = file_name(path);
    let found = prcxml::asset_params(&root);
    if found.shared_row {
        skyline::println!(
            "[{tag}] {label}: {name} holds a params list, the shared item row, which common_param.prcxml sets; the file is not used"
        );
        return;
    }
    let set = found.values.len();
    for (group, field, value) in found.values {
        crate::custom_params::set_asset(space, owner, group, field, crate::custom_params::answer_of(value));
    }
    skyline::println!("[{tag}] {label}: {name} {}", summary(set, 0, &found.skipped, None));
}
