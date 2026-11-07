#![allow(dead_code)]

use crate::stage_ledger::{CloneStage, Form};
use crate::stage_music::MusicRequest;
use dionysus_core::stage_gimmicks::{
    gimmick_header, Barrel, BarrelDraft, Door, DoorDraft, GimmickError, GimmickKind, Ladder,
    LadderDraft, Spring, SpringDraft, MAX_GIMMICKS,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PackDeclaration {
    pub place: String,
    pub display_name: String,
    pub id_name: Option<String>,
    pub forms: Vec<Form>,
    pub ships_battle_tree: bool,
    pub series: String,
    pub disp_order: i32,
    pub donor: Option<String>,
    pub resource_place: Option<String>,
    pub bgm: Option<String>,
    pub bgm_setting_no: Option<i64>,
    pub bgm_selector: Option<bool>,
    pub dynamic_collisions: Vec<String>,
    pub paused_motions: Vec<String>,
    pub ladders: Vec<Ladder>,
    pub barrels: Vec<Barrel>,
    pub springs: Vec<Spring>,
    pub doors: Vec<Door>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    Malformed { line: usize },
    BadValue { line: usize, key: String },
    MissingPlace,
    UnknownForm { line: usize, form: String },
    BadGimmick { line: usize, problem: GimmickError },
}

enum Value {
    Text(String),
    Number(i64),
    Real(f64),
    Flag(bool),
    List(Vec<String>),
    Numbers(Vec<f64>),
}

enum Section {
    Ladder(usize, LadderDraft),
    Barrel(usize, BarrelDraft),
    Spring(usize, SpringDraft),
    Door(usize, DoorDraft),
}

const BARREL_FLAGS: [&str; 3] = ["full_rotate", "always_rotate", "auto_fire"];

fn close(out: &mut PackDeclaration, section: Option<Section>) -> Result<(), ParseError> {
    let bad = |line: usize| move |problem| ParseError::BadGimmick { line, problem };
    match section {
        Some(Section::Ladder(line, draft)) => out.ladders.push(draft.finish().map_err(bad(line))?),
        Some(Section::Barrel(line, draft)) => out.barrels.push(draft.finish().map_err(bad(line))?),
        Some(Section::Spring(line, draft)) => out.springs.push(draft.finish().map_err(bad(line))?),
        Some(Section::Door(line, draft)) => out.doors.push(draft.finish().map_err(bad(line))?),
        None => {}
    }
    Ok(())
}

fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

fn parse_value(raw: &str) -> Option<Value> {
    let raw = raw.trim();
    if let Some(inner) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let numbers: Option<Vec<f64>> = inner
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| item.parse::<f64>().ok())
            .collect();
        if let Some(numbers) = numbers.filter(|numbers| !numbers.is_empty()) {
            return Some(Value::Numbers(numbers));
        }
        let mut items = Vec::new();
        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let text = item.strip_prefix('"')?.strip_suffix('"')?;
            items.push(text.to_string());
        }
        return Some(Value::List(items));
    }
    if let Some(text) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return Some(Value::Text(text.to_string()));
    }
    match raw {
        "true" => Some(Value::Flag(true)),
        "false" => Some(Value::Flag(false)),
        _ => raw
            .parse::<i64>()
            .map(Value::Number)
            .or_else(|_| raw.parse::<f64>().map(Value::Real))
            .ok(),
    }
}

pub fn parse(text: &str) -> Result<PackDeclaration, ParseError> {
    let mut out = PackDeclaration {
        forms: vec![Form::Normal],
        ..Default::default()
    };
    let mut saw_forms = false;
    let mut section: Option<Section> = None;

    for (index, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let number = index + 1;
        if let Some(kind) = gimmick_header(line) {
            close(&mut out, section.take())?;
            section = Some(match kind {
                GimmickKind::Ladder => Section::Ladder(number, LadderDraft::default()),
                GimmickKind::Barrel => Section::Barrel(number, BarrelDraft::default()),
                GimmickKind::Spring => Section::Spring(number, SpringDraft::default()),
                GimmickKind::Door => Section::Door(number, DoorDraft::default()),
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ParseError::Malformed { line: number });
        };
        let key = key.trim();
        let Some(value) = parse_value(value) else {
            return Err(ParseError::BadValue {
                line: number,
                key: key.to_string(),
            });
        };
        let bad = || ParseError::BadValue {
            line: number,
            key: key.to_string(),
        };
        match (section.as_mut(), value) {
            (None, value) => {
                top_level(&mut out, &mut saw_forms, number, key, value)?;
                continue;
            }
            (Some(Section::Ladder(_, draft)), Value::Number(amount)) => {
                draft.set(key, amount as f32);
            }
            (Some(Section::Ladder(_, draft)), Value::Real(amount)) => {
                draft.set(key, amount as f32);
            }
            (Some(Section::Ladder(..)), _) if matches!(key, "x" | "y" | "height" | "width") => {
                return Err(bad())
            }
            (Some(Section::Barrel(_, draft)), Value::Number(amount)) => {
                if !draft.set_number(key, amount as f32) && BARREL_FLAGS.contains(&key) {
                    return Err(bad());
                }
            }
            (Some(Section::Barrel(_, draft)), Value::Real(amount)) => {
                if !draft.set_number(key, amount as f32) && BARREL_FLAGS.contains(&key) {
                    return Err(bad());
                }
            }
            (Some(Section::Barrel(_, draft)), Value::Flag(flag)) => {
                if !draft.set_flag(key, flag) && BarrelDraft::default().set_number(key, 0.0) {
                    return Err(bad());
                }
            }
            (Some(Section::Barrel(_, draft)), Value::Numbers(numbers)) => {
                let numbers: Vec<f32> = numbers.iter().map(|value| *value as f32).collect();
                if !draft.set_numbers(key, &numbers) && key == "path" {
                    return Err(bad());
                }
            }
            (Some(Section::Barrel(_, draft)), Value::Text(text)) => {
                if !draft.set_text(key, &text)
                    && (BARREL_FLAGS.contains(&key) || BarrelDraft::default().set_number(key, 0.0))
                {
                    return Err(bad());
                }
            }
            (Some(Section::Barrel(..)), _)
                if key == "model"
                    || key == "path"
                    || BARREL_FLAGS.contains(&key)
                    || BarrelDraft::default().set_number(key, 0.0) =>
            {
                return Err(bad())
            }
            (Some(Section::Spring(_, draft)), Value::Number(amount)) => {
                draft.set_number(key, amount as f32);
            }
            (Some(Section::Spring(_, draft)), Value::Real(amount)) => {
                draft.set_number(key, amount as f32);
            }
            (Some(Section::Spring(_, draft)), Value::Text(text)) => {
                if !draft.set_text(key, &text) && SpringDraft::default().set_number(key, 0.0) {
                    return Err(bad());
                }
            }
            (Some(Section::Spring(..)), _)
                if key == "model" || key == "joint" || SpringDraft::default().set_number(key, 0.0) =>
            {
                return Err(bad())
            }
            (Some(Section::Door(_, draft)), Value::Number(amount)) => {
                if !draft.set_number(key, amount as f32) && key == "air" {
                    return Err(bad());
                }
            }
            (Some(Section::Door(_, draft)), Value::Real(amount)) => {
                if key == "player" || (!draft.set_number(key, amount as f32) && key == "air") {
                    return Err(bad());
                }
            }
            (Some(Section::Door(_, draft)), Value::Flag(flag)) => {
                if !draft.set_flag(key, flag) && DoorDraft::default().set_number(key, 0.0) {
                    return Err(bad());
                }
            }
            (Some(Section::Door(..)), _)
                if key == "air" || DoorDraft::default().set_number(key, 0.0) =>
            {
                return Err(bad())
            }
            _ => {}
        }
    }

    close(&mut out, section.take())?;
    if [out.ladders.len(), out.barrels.len(), out.springs.len(), out.doors.len()]
        .iter()
        .any(|count| *count > MAX_GIMMICKS)
    {
        return Err(ParseError::BadGimmick {
            line: 0,
            problem: GimmickError::OutOfRange("count"),
        });
    }
    if out.place.is_empty() {
        return Err(ParseError::MissingPlace);
    }
    if saw_forms && out.forms.is_empty() {
        out.forms.push(Form::Normal);
    }
    if out.display_name.is_empty() {
        out.display_name = out.place.clone();
    }
    Ok(out)
}

fn top_level(
    out: &mut PackDeclaration,
    saw_forms: &mut bool,
    number: usize,
    key: &str,
    value: Value,
) -> Result<(), ParseError> {
    let bad = || ParseError::BadValue {
        line: number,
        key: key.to_string(),
    };
    match (key, value) {
        ("place", Value::Text(text)) => out.place = text.to_ascii_lowercase(),
        ("display_name", Value::Text(text)) => out.display_name = text,
        ("id_name", Value::Text(text)) => out.id_name = Some(text),
        ("series", Value::Text(text)) => out.series = text.to_ascii_lowercase(),
        ("donor", Value::Text(text)) => out.donor = Some(text.to_ascii_lowercase()),
        ("resource_place", Value::Text(text)) => {
            out.resource_place = Some(text.to_ascii_lowercase())
        }
        ("bgm", Value::Text(text)) => out.bgm = Some(text.to_ascii_lowercase()),
        ("bgm_setting_no", Value::Number(value)) => out.bgm_setting_no = Some(value),
        ("bgm_selector", Value::Flag(flag)) => out.bgm_selector = Some(flag),
        ("ships_battle_tree", Value::Flag(flag)) => out.ships_battle_tree = flag,
        ("disp_order", Value::Number(value)) => out.disp_order = value as i32,
        ("dynamic_collisions", Value::List(items)) => {
            out.dynamic_collisions = items.iter().map(|item| item.to_ascii_lowercase()).collect()
        }
        ("paused_motions", Value::List(items)) => {
            out.paused_motions = items.iter().map(|item| item.to_ascii_lowercase()).collect()
        }
        ("forms", Value::List(items)) => {
            *saw_forms = true;
            out.forms.clear();
            for item in items {
                out.forms.push(match item.to_ascii_lowercase().as_str() {
                    "normal" => Form::Normal,
                    "omega" | "end" => Form::Omega,
                    "battlefield" | "battle" => Form::Battlefield,
                    _ => {
                        return Err(ParseError::UnknownForm {
                            line: number,
                            form: item,
                        })
                    }
                });
            }
        }
        (
            "place" | "display_name" | "id_name" | "series" | "donor" | "resource_place"
            | "ships_battle_tree" | "disp_order" | "forms" | "bgm" | "bgm_setting_no"
            | "bgm_selector" | "dynamic_collisions" | "paused_motions",
            _,
        ) => return Err(bad()),
        _ => {}
    }
    Ok(())
}

pub fn parse_all(text: &str) -> Result<Vec<PackDeclaration>, ParseError> {
    let blocks = dionysus_core::manifest::blocks(text, "stage")
        .map_err(|line| ParseError::Malformed { line })?;
    blocks
        .iter()
        .map(|block| {
            parse(&block.text).map_err(|error| match error {
                ParseError::Malformed { line } => ParseError::Malformed {
                    line: line + block.first_line - 1,
                },
                ParseError::BadValue { line, key } => ParseError::BadValue {
                    line: line + block.first_line - 1,
                    key,
                },
                ParseError::UnknownForm { line, form } => ParseError::UnknownForm {
                    line: line + block.first_line - 1,
                    form,
                },
                ParseError::BadGimmick { line, problem } if line > 0 => ParseError::BadGimmick {
                    line: line + block.first_line - 1,
                    problem,
                },
                other => other,
            })
        })
        .collect()
}

impl PackDeclaration {
    pub fn to_clone_stage(&self) -> CloneStage {
        let mut stage = CloneStage::new(&self.place);
        stage.ships_battle_tree = self.ships_battle_tree;
        stage.forms = self.forms.clone();
        stage.ui_name_id = self.id_name.clone();
        stage.resource_place = self.resource_place.clone();
        stage
    }

    pub fn music_request(&self) -> MusicRequest {
        MusicRequest {
            set: self.bgm.clone(),
            setting_no: self.bgm_setting_no,
            selector: self.bgm_selector,
        }
    }

    pub fn music_donor(&self) -> Option<&str> {
        self.donor
            .as_deref()
            .or(self.resource_place.as_deref())
            .or(Some(self.place.as_str()))
    }
}

const MOD_ROOT: &str = "sd:/ultimate/mods";

#[cfg(all(not(test), feature = "stage_mint"))]
fn load_orders(directory: &std::path::Path) {
    use dionysus_core::stage_order;
    let Ok(text) = std::fs::read_to_string(directory.join(stage_order::FILE_NAME)) else {
        return;
    };
    let name = directory
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let (orders, problems) = stage_order::parse(&text);
    for problem in problems {
        skyline::println!("[stagepack] {name}/{} {problem}", stage_order::FILE_NAME);
    }
    for (stage, order) in &orders {
        crate::stage_db_rows::request_disp_order(crate::stage_ledger::hash40(stage), *order);
    }
    skyline::println!("[stagepack] {name}: {} stage position(s) queued", orders.len());
}

pub fn load_all() {
    let Ok(entries) = std::fs::read_dir(MOD_ROOT) else {
        skyline::println!("[stagepack] no {MOD_ROOT}; no data-only stage packs to load");
        return;
    };
    let mut declarations: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        load_orders(&directory);
        let manifest = directory.join("stage.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        declarations.push((
            directory
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            text,
        ));
    }
    declarations.sort_by(|a, b| a.0.cmp(&b.0));
    if declarations.is_empty() {
        return;
    }
    skyline::println!(
        "[stagepack] {} stage.toml pack(s) under {MOD_ROOT}",
        declarations.len()
    );
    for (directory, text) in declarations {
        match parse_all(&text) {
            Ok(parsed) => {
                if parsed.len() > 1 {
                    skyline::println!(
                        "[stagepack] {directory}/stage.toml: {} stages",
                        parsed.len()
                    );
                }
                for declaration in &parsed {
                    mint(&directory, declaration);
                }
            }
            Err(error) => skyline::println!(
                "[stagepack] {directory}/stage.toml is not readable: {error:?}; skipped"
            ),
        }
    }
}

pub fn stdat_carries_data(bytes: &[u8]) -> bool {
    if bytes.len() < 16 || &bytes[0..8] != b"paracobn" {
        return false;
    }
    let word = |at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
    };
    let Some(root) = 16usize
        .checked_add(word(8))
        .and_then(|at| at.checked_add(word(12)))
    else {
        return false;
    };
    if root + 5 > bytes.len() || bytes[root] != 12 {
        return false;
    }
    word(root + 1) > 0
}

#[cfg(all(not(test), feature = "stage_mint"))]
fn ships_stdat(directory: &str, asset_place: &str) -> bool {
    fn any_stdat(at: &std::path::Path, depth: usize) -> bool {
        if depth == 0 {
            return false;
        }
        let Ok(entries) = std::fs::read_dir(at) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if any_stdat(&path, depth - 1) {
                    return true;
                }
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("stdat"))
                && std::fs::read(&path).is_ok_and(|bytes| stdat_carries_data(&bytes))
            {
                return true;
            }
        }
        false
    }
    any_stdat(
        &std::path::Path::new(MOD_ROOT)
            .join(directory)
            .join("stage")
            .join(asset_place)
            .join("normal"),
        6,
    )
}

#[cfg(all(not(test), feature = "stage_mint"))]
fn mint(directory: &str, declaration: &PackDeclaration) {
    let stage = declaration.to_clone_stage();
    let place = {
        let Ok(mut registry) = crate::stage_registry::registry().lock() else {
            skyline::println!("[stagepack] {directory}: registry unavailable");
            return;
        };
        if registry.by_name(&stage.place_name).is_some() {
            skyline::println!(
                "[stagepack] {directory}: {} is already minted; leaving it alone",
                stage.place_name
            );
            return;
        }
        match registry.register(&stage) {
            Ok(minted) => minted.place,
            Err(error) => {
                skyline::println!("[stagepack] {directory}: refused: {error:?}");
                return;
            }
        }
    };

    if let Some(donor) = declaration.donor.as_deref() {
        if let Ok(mut registry) = crate::stage_registry::registry().lock() {
            registry.set_behaviour(&stage.place_name, donor);
        }
    }

    if ships_stdat(directory, stage.asset_place()) {
        if let Ok(mut registry) = crate::stage_registry::registry().lock() {
            registry.set_owns_stdat(&stage.place_name, true);
        }
        skyline::println!(
            "[stagepack] {}: ships its own .stdat, so its forms read that and not {}'s",
            stage.place_name,
            declaration.donor.as_deref().unwrap_or("its donor"),
        );
    }

    if let Ok(mut registry) = crate::stage_registry::registry().lock() {
        registry.set_music(&stage.place_name, declaration.music_request());
    }

    if !declaration.dynamic_collisions.is_empty() {
        let sets = declaration
            .dynamic_collisions
            .iter()
            .map(|set| crate::stage_ledger::hash40(set))
            .collect();
        if let Ok(mut registry) = crate::stage_registry::registry().lock() {
            registry.set_dynamic_collisions(&stage.place_name, sets);
        }
        crate::stage_move_floors::arm();
        skyline::println!(
            "[movefloor] {}: binds the collisions of {:?} to their model bones",
            stage.place_name,
            declaration.dynamic_collisions
        );
    }

    if !declaration.paused_motions.is_empty() {
        let sets = declaration
            .paused_motions
            .iter()
            .map(|set| crate::stage_ledger::hash40(set))
            .collect();
        if let Ok(mut registry) = crate::stage_registry::registry().lock() {
            registry.set_paused_motions(&stage.place_name, sets);
        }
        crate::stage_gimmicks::arm(false, false);
        skyline::println!(
            "[motion] {}: starts {:?} paused",
            stage.place_name,
            declaration.paused_motions
        );
    }

    if !declaration.ladders.is_empty()
        || !declaration.barrels.is_empty()
        || !declaration.springs.is_empty()
        || !declaration.doors.is_empty()
    {
        crate::stage_gimmicks::set_gimmicks(
            &stage.place_name,
            declaration.ladders.clone(),
            declaration.barrels.clone(),
            declaration.springs.clone(),
            declaration.doors.clone(),
        );
        skyline::println!(
            "[gimmick] {}: {} ladder(s), {} barrel(s), {} spring(s), {} door(s) declared",
            stage.place_name,
            declaration.ladders.len(),
            declaration.barrels.len(),
            declaration.springs.len(),
            declaration.doors.len()
        );
    }

    skyline::println!(
        "[stagepack] {directory}: minted {} at place {place}, behaviour {}, disp_order {}",
        stage.place_name,
        declaration.donor.as_deref().unwrap_or("(its own)"),
        declaration.disp_order,
    );

    let Some(music) = crate::stage_registration::resolve_music(
        &stage.place_name,
        &declaration.music_request(),
        declaration.music_donor(),
    ) else {
        return;
    };

    if !crate::stage_registry::registry()
        .lock()
        .map(|mut registry| registry.claim_row(&stage.place_name))
        .unwrap_or(false)
    {
        return;
    }

    match crate::stage_registration::plan(
        &stage,
        crate::stage_ledger::hash40(&declaration.series),
        declaration.disp_order,
        &music,
    ) {
        Ok(_registration) => {
            #[cfg(feature = "stage_slot")]
            {
                crate::stage_registration::register(&_registration);
                if let Some(wanted) = _registration.deferred_disp_order {
                    crate::stage_db_rows::request_disp_order(_registration.stage_hash, wanted);
                    skyline::println!(
                        "[stagepack] {}: registered hidden; disp_order {wanted} queued for the \
                         row backend (CSK's SignedByteType stops at 127)",
                        stage.place_name
                    );
                }
            }
        }
        Err(error) => skyline::println!(
            "[stagepack] {}: has an identity but no grid row: {error:?}",
            stage.place_name
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUMPKIN: &str = r#"
# Pumpkin Hill, rehoused from `!Knuckles Moveset` onto a MINTED place.
place        = "pumpkin_hill"
display_name = "Pumpkin Hill"
id_name      = "PumpkinHill"
forms        = ["normal"]
ships_battle_tree = false
series       = "sonic"
disp_order   = 119
donor        = "photostage"
"#;

    #[test]
    fn reads_the_pack_that_already_ships() {
        let pack = parse(PUMPKIN).unwrap();
        assert_eq!(pack.place, "pumpkin_hill");
        assert_eq!(pack.display_name, "Pumpkin Hill");
        assert_eq!(pack.id_name.as_deref(), Some("PumpkinHill"));
        assert_eq!(pack.forms, vec![Form::Normal]);
        assert!(!pack.ships_battle_tree);
        assert_eq!(pack.series, "sonic");
        assert_eq!(pack.disp_order, 119);
        assert_eq!(pack.donor.as_deref(), Some("photostage"));
        assert_eq!(pack.resource_place, None);
    }

    #[test]
    fn a_declaration_becomes_the_same_stage_a_plugin_would_build() {
        let stage = parse(PUMPKIN).unwrap().to_clone_stage();
        assert_eq!(stage.place_name, "pumpkin_hill");
        assert_eq!(stage.ui_name_id.as_deref(), Some("PumpkinHill"));
        assert_eq!(stage.forms, vec![Form::Normal]);
        assert_eq!(stage.resource_place, None);
    }

    #[test]
    fn all_three_forms_parse() {
        let pack =
            parse("place = \"x\"\nforms = [\"normal\", \"omega\", \"battlefield\"]").unwrap();
        assert_eq!(
            pack.forms,
            vec![Form::Normal, Form::Omega, Form::Battlefield]
        );
    }

    #[test]
    fn a_hash_inside_a_string_is_not_a_comment() {
        let pack = parse("place = \"x\"\ndisplay_name = \"Stage #2\"").unwrap();
        assert_eq!(pack.display_name, "Stage #2");
    }

    #[test]
    fn a_missing_display_name_falls_back_to_the_place() {
        assert_eq!(parse("place = \"tetris\"").unwrap().display_name, "tetris");
    }

    #[test]
    fn an_unknown_key_is_ignored_but_a_mistyped_known_one_is_not() {
        assert!(parse("place = \"x\"\nfuture_key = \"whatever\"").is_ok());
        assert_eq!(
            parse("place = \"x\"\ndisp_order = \"119\""),
            Err(ParseError::BadValue {
                line: 2,
                key: "disp_order".to_string()
            })
        );
    }

    #[test]
    fn the_music_keys_reach_the_request() {
        let pack = parse(
            r#"
place          = "demon_mirror"
bgm            = "demon"
bgm_setting_no = 0
bgm_selector   = true
"#,
        )
        .unwrap();
        assert_eq!(pack.bgm.as_deref(), Some("demon"));
        assert_eq!(pack.bgm_setting_no, Some(0));
        assert_eq!(pack.bgm_selector, Some(true));
        let request = pack.music_request();
        assert_eq!(request.set.as_deref(), Some("demon"));
        assert_eq!(request.setting_no, Some(0));
        assert_eq!(request.selector, Some(true));
    }

    #[test]
    fn a_pack_that_names_no_music_borrows_from_its_donor() {
        let pack = parse(
            r#"
place = "demon_mirror"
donor = "dolly_stadium"
"#,
        )
        .unwrap();
        assert!(pack.music_request().is_empty());
        assert_eq!(pack.music_donor(), Some("dolly_stadium"));
        let music = crate::stage_music::resolve(&pack.music_request(), pack.music_donor())
            .unwrap()
            .music;
        assert_eq!(music.set_label, "bgmdolly");
    }

    #[test]
    fn a_mistyped_music_key_is_refused_like_any_other() {
        assert_eq!(
            parse("place=\"x\"\nbgm_setting_no = \"0\""),
            Err(ParseError::BadValue {
                line: 2,
                key: "bgm_setting_no".to_string()
            })
        );
    }

    #[test]
    fn a_pack_without_a_place_is_refused() {
        assert_eq!(
            parse("display_name = \"Nameless\""),
            Err(ParseError::MissingPlace)
        );
    }

    #[test]
    fn a_bad_form_names_itself() {
        assert_eq!(
            parse("place = \"x\"\nforms = [\"normal\", \"omga\"]"),
            Err(ParseError::UnknownForm {
                line: 2,
                form: "omga".to_string()
            })
        );
    }

    #[test]
    fn ladders_are_tables_after_the_stage_keys() {
        let pack = parse(
            r#"
place = "x"
disp_order = -1

[[ladder]]
x = 120
y = -20.5
height = 80

[[stage.ladder]]
x = 0
y = 0
height = 30
width = 14
"#,
        )
        .unwrap();
        assert_eq!(pack.disp_order, -1);
        assert_eq!(pack.ladders.len(), 2);
        assert_eq!(pack.ladders[0].y, -20.5);
        assert_eq!(pack.ladders[0].width, 10.0);
        assert_eq!(pack.ladders[1].width, 14.0);
        assert_eq!(
            parse(
                r#"place = "x"
[[ladder]]
x = 1
y = 2
"#
            ),
            Err(ParseError::BadGimmick {
                line: 2,
                problem: GimmickError::Missing("height")
            })
        );
    }

    #[test]
    fn barrels_take_numbers_and_flags() {
        let pack = parse(
            r#"
place = "x"

[[barrel]]
x = 1165
y = 190
rotation_max = 90
rotation_speed = 1.05
always_rotate = true
"#,
        )
        .unwrap();
        assert_eq!(pack.barrels.len(), 1);
        assert_eq!(pack.barrels[0].rotation_max, 90.0);
        assert!(pack.barrels[0].always_rotate);
        assert!(!pack.barrels[0].auto_fire);
        assert_eq!(
            parse(
                r#"place = "x"
[[barrel]]
x = 1
y = 2
auto_fire = 1
"#
            ),
            Err(ParseError::BadValue {
                line: 5,
                key: "auto_fire".to_string()
            })
        );
    }

    #[test]
    fn springs_take_numbers_and_a_model() {
        let pack = parse(
            r#"
place = "x"

[[spring]]
x = 155
y = -30
rotation = -26.5
speed = 4.2
model = "spring_01_set"
"#,
        )
        .unwrap();
        assert_eq!(pack.springs.len(), 1);
        assert_eq!(pack.springs[0].rotation, -26.5);
        assert_eq!(pack.springs[0].speed, 4.2);
        assert_eq!(pack.springs[0].model.as_deref(), Some("spring_01_set"));
        assert_eq!(
            parse(
                r#"place = "x"
[[spring]]
x = 1
y = 2
speed = "fast"
"#
            ),
            Err(ParseError::BadValue {
                line: 5,
                key: "speed".to_string()
            })
        );
    }

    #[test]
    fn doors_take_their_area_kind_and_player() {
        let pack = parse(
            r#"
place = "x"

[[door]]
x = 625
y = 193
offset_y = 10
air = true
player = 1

[[door]]
x = 1770
y = 75
"#,
        )
        .unwrap();
        assert_eq!(pack.doors.len(), 2);
        assert!(pack.doors[0].air);
        assert_eq!(pack.doors[0].player, Some(1));
        assert_eq!(pack.doors[0].offset_y, 10.0);
        assert!(!pack.doors[1].air);
        assert_eq!(pack.doors[1].player, None);
        for (line, key) in [("air = 1", "air"), ("player = 1.5", "player"), ("x = true", "x")] {
            assert_eq!(
                parse(&format!("place = \"x\"\n[[door]]\nx = 1\ny = 2\n{line}\n")),
                Err(ParseError::BadValue {
                    line: 5,
                    key: key.to_string()
                })
            );
        }
    }

    #[test]
    fn negative_and_deferred_disp_orders_survive_the_round_trip() {
        assert_eq!(
            parse("place=\"x\"\ndisp_order = -1").unwrap().disp_order,
            -1
        );
        assert_eq!(
            parse("place=\"x\"\ndisp_order = 128").unwrap().disp_order,
            128
        );
    }
}

#[cfg(all(test, feature = "stage_mint"))]
pub fn load_all() {}

#[cfg(test)]
mod stdat_tests {
    use super::stdat_carries_data;

    fn paracobn(hash_bytes: usize, ref_bytes: usize, entries: u32) -> Vec<u8> {
        let mut out = b"paracobn".to_vec();
        out.extend_from_slice(&(hash_bytes as u32).to_le_bytes());
        out.extend_from_slice(&(ref_bytes as u32).to_le_bytes());
        out.extend(std::iter::repeat(0).take(hash_bytes + ref_bytes));
        out.push(12);
        out.extend_from_slice(&entries.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out
    }

    #[test]
    fn the_empty_placeholder_shipped_by_tabuu_residence_is_not_data() {
        let empty = paracobn(8, 0, 0);
        assert_eq!(empty.len(), 33);
        assert!(!stdat_carries_data(&empty));
    }

    #[test]
    fn a_populated_stdat_is_data() {
        assert!(stdat_carries_data(&paracobn(8, 0, 1)));
        assert!(stdat_carries_data(&paracobn(0x7e0, 0x7d8, 24)));
    }

    #[test]
    fn anything_that_is_not_a_paracobn_is_refused() {
        assert!(!stdat_carries_data(b""));
        assert!(!stdat_carries_data(b"paracobn"));
        assert!(!stdat_carries_data(&[0u8; 64]));
        let mut truncated = paracobn(8, 0, 3);
        truncated.truncate(20);
        assert!(!stdat_carries_data(&truncated));
    }
}
