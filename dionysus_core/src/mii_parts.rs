use std::collections::BTreeSet;

use crate::hash40;
use crate::prc::Param;

pub const HAT_DB_PATH: &str = "ui/param/database/ui_mii_hat_db.prc";
pub const OUTFIT_DB_PATH: &str = "ui/param/database/ui_mii_body_db.prc";
pub const HAT_SAVE_LAST: i16 = 0x19f;
pub const OUTFIT_SAVE_LAST: i16 = 0x3ff;
pub const MAX_ID: usize = 48;
pub const MAX_PARTS: usize = 256;
pub const MAX_HAIR_TYPES: usize = 256;
pub const HASH_MASK: u64 = 0xff_ffff_ffff;

const START_OWNED: u64 = 0x18_ef46_7708;
const HAIR_MODEL_TYPES: u64 = 0x17_adde_f638;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Hat,
    Outfit,
}

impl Part {
    pub fn name(self) -> &'static str {
        match self {
            Part::Hat => "hat",
            Part::Outfit => "outfit",
        }
    }

    pub fn last_save_no(self) -> i16 {
        match self {
            Part::Hat => HAT_SAVE_LAST,
            Part::Outfit => OUTFIT_SAVE_LAST,
        }
    }

    pub fn row_id_key(self) -> u64 {
        match self {
            Part::Hat => hash40("mii_hat_id"),
            Part::Outfit => hash40("mii_body_id"),
        }
    }

    pub fn ui_key(self) -> u64 {
        match self {
            Part::Hat => hash40("ui_mii_hat_id"),
            Part::Outfit => hash40("ui_mii_body_id"),
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "hat" => Some(Part::Hat),
            "outfit" => Some(Part::Outfit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    Normal,
    Hat,
    FaceOnly,
    Term,
}

impl ModelType {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "normal" => Some(ModelType::Normal),
            "hat" => Some(ModelType::Hat),
            "face_only" => Some(ModelType::FaceOnly),
            "term" | "no_head" => Some(ModelType::Term),
            _ => None,
        }
    }

    pub fn hash(self) -> u64 {
        hash40(match self {
            ModelType::Normal => "mii_model_type_normal",
            ModelType::Hat => "mii_model_type_hat",
            ModelType::FaceOnly => "mii_model_type_face_only",
            ModelType::Term => "mii_model_type_term",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiiFighter {
    Brawler,
    Swordfighter,
    Gunner,
}

impl MiiFighter {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "miifighter" | "brawler" => Some(MiiFighter::Brawler),
            "miiswordsman" | "swordfighter" => Some(MiiFighter::Swordfighter),
            "miigunner" | "gunner" => Some(MiiFighter::Gunner),
            _ => None,
        }
    }

    pub fn fighter_name(self) -> &'static str {
        match self {
            MiiFighter::Brawler => "miifighter",
            MiiFighter::Swordfighter => "miiswordsman",
            MiiFighter::Gunner => "miigunner",
        }
    }

    pub fn row_type(self) -> u64 {
        hash40(match self {
            MiiFighter::Brawler => "mii_fighter",
            MiiFighter::Swordfighter => "mii_swordsman",
            MiiFighter::Gunner => "mii_gunner",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    Both,
}

impl Gender {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "male" | "m" => Some(Gender::Male),
            "female" | "f" => Some(Gender::Female),
            "both" => Some(Gender::Both),
            _ => None,
        }
    }

    pub fn hash(self) -> u64 {
        hash40(match self {
            Gender::Male => "gender_male",
            Gender::Female => "gender_female",
            Gender::Both => "gender_both",
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub part: Part,
    pub id: String,
    pub base: String,
    pub fighter: Option<MiiFighter>,
    pub gender: Option<Gender>,
    pub name: Option<String>,
    pub model_type: Option<ModelType>,
    pub hair_model_types: Vec<(i32, ModelType)>,
    pub owned: bool,
    pub disp_order: Option<i32>,
    pub swing: Option<bool>,
    pub is_patch: Option<bool>,
    pub line: usize,
}

pub fn ui_id(part: Part, id: &str) -> String {
    match part {
        Part::Hat => format!("ui_mii_hat_{id}"),
        Part::Outfit => format!("ui_mii_body_{}", id.strip_prefix("b_").unwrap_or(id)),
    }
}

impl Declaration {
    pub fn ui_id(&self) -> String {
        ui_id(self.part, &self.id)
    }

    pub fn ui_hash(&self) -> u64 {
        hash40(&self.ui_id())
    }

    pub fn icon_name(&self) -> String {
        match self.part {
            Part::Hat => self.id.clone(),
            Part::Outfit => self.name.clone().unwrap_or_else(|| outfit_name(&self.id)),
        }
    }

    pub fn folder(&self) -> String {
        match self.part {
            Part::Hat => format!("miihat/{}", self.id),
            Part::Outfit => format!(
                "miibody/{}/{}",
                self.fighter.map(MiiFighter::fighter_name).unwrap_or("miifighter"),
                self.id
            ),
        }
    }
}

pub fn outfit_name(id: &str) -> String {
    for suffix in ["_m", "_f"] {
        if let Some(stem) = id.strip_suffix(suffix) {
            if !stem.is_empty() {
                return stem.to_string();
            }
        }
    }
    id.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiiError {
    UnknownTable { line: usize, table: String },
    Malformed { line: usize },
    UnknownKey { line: usize, key: String },
    BadValue { line: usize, key: String },
    Missing { line: usize, key: &'static str },
    BadId { line: usize, id: String },
    Duplicate { line: usize, id: String },
    TooMany,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Text(String),
    Flag(bool),
    Number(i64),
    List(Vec<String>),
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (index, byte) in line.bytes().enumerate() {
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
        let mut items = Vec::new();
        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            items.push(item.strip_prefix('"')?.strip_suffix('"')?.to_string());
        }
        return Some(Value::List(items));
    }
    if let Some(text) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return Some(Value::Text(text.to_string()));
    }
    match raw {
        "true" => return Some(Value::Flag(true)),
        "false" => return Some(Value::Flag(false)),
        _ => {}
    }
    raw.replace('_', "").parse::<i64>().ok().map(Value::Number)
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn hair_entry(text: &str) -> Option<(i32, ModelType)> {
    let (hair, model) = text.split_once('=')?;
    let hair = hair.trim().parse::<i32>().ok()?;
    if !(0..MAX_HAIR_TYPES as i32).contains(&hair) {
        return None;
    }
    Some((hair, ModelType::parse(model.trim())?))
}

fn finish(block: Declaration, out: &mut Vec<Declaration>) -> Result<(), MiiError> {
    let line = block.line;
    if block.id.is_empty() {
        return Err(MiiError::Missing { line, key: "id" });
    }
    if block.base.is_empty() {
        return Err(MiiError::Missing { line, key: "base" });
    }
    if !valid_id(&block.id) {
        return Err(MiiError::BadId { line, id: block.id });
    }
    if block.part == Part::Outfit {
        if block.fighter.is_none() {
            return Err(MiiError::Missing { line, key: "fighter" });
        }
        if !block.id.starts_with("b_") {
            return Err(MiiError::BadId { line, id: block.id });
        }
        if let Some(name) = &block.name {
            if !valid_id(name) {
                return Err(MiiError::BadId { line, id: name.clone() });
            }
        }
    }
    if out.iter().any(|other| other.part == block.part && other.id == block.id) {
        return Err(MiiError::Duplicate { line, id: block.id });
    }
    if out.len() >= MAX_PARTS {
        return Err(MiiError::TooMany);
    }
    out.push(block);
    Ok(())
}

pub fn parse(text: &str) -> Result<Vec<Declaration>, MiiError> {
    let mut out = Vec::new();
    let mut pending: Option<Declaration> = None;
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
            if let Some(done) = pending.take() {
                finish(done, &mut out)?;
            }
            let part = match name.trim() {
                "mii_hat" => Part::Hat,
                "mii_outfit" => Part::Outfit,
                other => {
                    return Err(MiiError::UnknownTable {
                        line: number,
                        table: other.to_string(),
                    })
                }
            };
            pending = Some(Declaration {
                part,
                id: String::new(),
                base: String::new(),
                fighter: None,
                gender: None,
                name: None,
                model_type: None,
                hair_model_types: Vec::new(),
                owned: true,
                disp_order: None,
                swing: None,
                is_patch: None,
                line: number,
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(MiiError::Malformed { line: number });
        };
        let Some(block) = pending.as_mut() else {
            return Err(MiiError::Malformed { line: number });
        };
        let key = key.trim();
        let bad = || MiiError::BadValue {
            line: number,
            key: key.to_string(),
        };
        let value = parse_value(value).ok_or_else(bad)?;
        let hat = block.part == Part::Hat;
        match (key, value) {
            ("id", Value::Text(text)) => block.id = text.trim().to_string(),
            ("base", Value::Text(text)) => block.base = text.trim().to_string(),
            ("owned", Value::Flag(flag)) => block.owned = flag,
            ("swing", Value::Flag(flag)) => block.swing = Some(flag),
            ("is_patch", Value::Flag(flag)) => block.is_patch = Some(flag),
            ("disp_order", Value::Number(n)) if (0..=i32::MAX as i64).contains(&n) => {
                block.disp_order = Some(n as i32)
            }
            ("model_type", Value::Text(text)) if hat => {
                block.model_type = Some(ModelType::parse(text.trim()).ok_or_else(bad)?)
            }
            ("hair_model_types", Value::List(items)) if hat => {
                if items.len() > MAX_HAIR_TYPES {
                    return Err(bad());
                }
                let mut entries: Vec<(i32, ModelType)> = Vec::new();
                for item in &items {
                    let entry = hair_entry(item).ok_or_else(bad)?;
                    entries.retain(|(hair, _)| *hair != entry.0);
                    entries.push(entry);
                }
                block.hair_model_types = entries;
            }
            ("fighter", Value::Text(text)) if !hat => {
                block.fighter = Some(MiiFighter::parse(text.trim()).ok_or_else(bad)?)
            }
            ("gender", Value::Text(text)) if !hat => {
                block.gender = Some(Gender::parse(text.trim()).ok_or_else(bad)?)
            }
            ("name", Value::Text(text)) if !hat => block.name = Some(text.trim().to_string()),
            ("id" | "base" | "owned" | "swing" | "is_patch" | "disp_order", _) => return Err(bad()),
            ("model_type" | "hair_model_types", _) if hat => return Err(bad()),
            ("fighter" | "gender" | "name", _) if !hat => return Err(bad()),
            _ => {
                return Err(MiiError::UnknownKey {
                    line: number,
                    key: key.to_string(),
                })
            }
        }
    }
    if let Some(done) = pending.take() {
        finish(done, &mut out)?;
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub part: Part,
    pub save_no: i16,
    pub id: String,
    pub base: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assigned {
    Kept(i16),
    New(i16),
    Moved { from: i16, to: i16 },
}

impl Assigned {
    pub fn save_no(self) -> i16 {
        match self {
            Assigned::Kept(n) | Assigned::New(n) => n,
            Assigned::Moved { to, .. } => to,
        }
    }
}

impl Registry {
    pub fn parse(text: &str) -> Self {
        let mut entries: Vec<Entry> = Vec::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [part, save_no, id, base] = fields.as_slice() else {
                continue;
            };
            let (Some(part), Ok(save_no)) = (Part::parse(part), save_no.parse::<i16>()) else {
                continue;
            };
            if save_no < 0 || save_no > part.last_save_no() || !valid_id(id) || !valid_id(base) {
                continue;
            }
            if entries
                .iter()
                .any(|e| e.part == part && (e.save_no == save_no || e.id == *id))
            {
                continue;
            }
            entries.push(Entry {
                part,
                save_no,
                id: id.to_string(),
                base: base.to_string(),
            });
        }
        Registry { entries }
    }

    pub fn serialise(&self) -> String {
        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| (a.part, a.save_no).cmp(&(b.part, b.save_no)));
        let mut out = String::from(
            "# Dionysus Mii hat and outfit save numbers.\n\
             # <hat|outfit> <save_no> <id> <base>, one per line. A saved Mii keeps\n\
             # its hat and outfit by this number, so a line is never reused.\n",
        );
        for entry in entries {
            out.push_str(&format!(
                "{} {} {} {}\n",
                entry.part.name(),
                entry.save_no,
                entry.id,
                entry.base
            ));
        }
        out
    }

    pub fn find(&self, part: Part, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.part == part && e.id == id)
    }

    pub fn by_save_no(&self, part: Part, save_no: i16) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.part == part && e.save_no == save_no)
    }

    pub fn assign(
        &mut self,
        part: Part,
        id: &str,
        base: &str,
        taken: &BTreeSet<i16>,
        first: i16,
    ) -> Option<Assigned> {
        let first = first.max(0);
        let reserved: BTreeSet<i16> = self
            .entries
            .iter()
            .filter(|e| e.part == part && e.id != id)
            .map(|e| e.save_no)
            .collect();
        let previous = self.find(part, id).map(|e| e.save_no);
        if let Some(save_no) = previous {
            if !taken.contains(&save_no) {
                if let Some(entry) = self.entries.iter_mut().find(|e| e.part == part && e.id == id) {
                    entry.base = base.to_string();
                }
                return Some(Assigned::Kept(save_no));
            }
        }
        let free = (first..=part.last_save_no())
            .find(|n| !taken.contains(n) && !reserved.contains(n))?;
        self.entries.retain(|e| !(e.part == part && e.id == id));
        self.entries.push(Entry {
            part,
            save_no: free,
            id: id.to_string(),
            base: base.to_string(),
        });
        Some(match previous {
            Some(from) => Assigned::Moved { from, to: free },
            None => Assigned::New(free),
        })
    }
}

pub fn rows(root: &Param) -> Option<&Vec<Param>> {
    root.get(hash40("db_root"))?.list()
}

pub fn rows_mut(root: &mut Param) -> Option<&mut Vec<Param>> {
    root.get_mut(hash40("db_root"))?.list_mut()
}

pub fn row_id(row: &Param, part: Part) -> Option<&str> {
    row.get(part.row_id_key())?.text()
}

pub fn row_save_no(row: &Param) -> Option<i16> {
    row.get(hash40("save_no"))?
        .integer()
        .and_then(|n| i16::try_from(n).ok())
}

pub fn find_row<'a>(root: &'a Param, part: Part, id: &str) -> Option<&'a Param> {
    rows(root)?.iter().find(|row| row_id(row, part) == Some(id))
}

pub fn taken_save_numbers(root: &Param) -> BTreeSet<i16> {
    rows(root)
        .map(|rows| rows.iter().filter_map(row_save_no).filter(|n| *n >= 0).collect())
        .unwrap_or_default()
}

pub fn first_free_save_no(root: &Param) -> i16 {
    taken_save_numbers(root)
        .iter()
        .next_back()
        .map(|n| n.saturating_add(1))
        .unwrap_or(0)
}

pub fn next_disp_order(root: &Param) -> i32 {
    rows(root)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get(hash40("disp_order"))?.integer())
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
        .saturating_add(1) as i32
}

fn set_keep_type(row: &mut Param, key: u64, value: i64) {
    let replacement = match row.get(key) {
        Some(Param::Bool(_)) => Param::Bool(value != 0),
        Some(Param::I8(_)) => Param::I8(value as i8),
        Some(Param::U8(_)) => Param::U8(value as u8),
        Some(Param::I16(_)) => Param::I16(value as i16),
        Some(Param::U16(_)) => Param::U16(value as u16),
        Some(Param::U32(_)) => Param::U32(value as u32),
        _ => Param::I32(value as i32),
    };
    row.set(key, replacement);
}

pub fn build_row(base: &Param, declaration: &Declaration, save_no: i16, disp_order: i32) -> Param {
    let mut row = base.clone();
    let id = declaration.id.as_str();
    row.set(declaration.part.row_id_key(), Param::Str(id.to_string()));
    row.set(declaration.part.ui_key(), Param::Hash(declaration.ui_hash()));
    row.set(hash40("text_id"), Param::Str(id.to_string()));
    row.set(hash40("save_no"), Param::I16(save_no));
    row.set(hash40("motif_id"), Param::Hash(hash40(id)));
    row.set(hash40("dlc_type"), Param::Hash(hash40("dlc_none")));
    row.set(hash40("prize_lottery"), Param::Bool(false));
    row.set(hash40("shop_item_tag"), Param::Hash(0));
    set_keep_type(&mut row, hash40("disp_order"), disp_order as i64);
    set_keep_type(&mut row, hash40("valid_resource"), 1);
    row.set(START_OWNED, Param::Bool(declaration.owned));
    if let Some(flag) = declaration.is_patch {
        row.set(hash40("is_patch"), Param::Bool(flag));
    }
    if let Some(flag) = declaration.swing {
        row.set(hash40("swing_enabled"), Param::Bool(flag));
    }
    match declaration.part {
        Part::Hat => {
            row.set(hash40("name_id"), Param::Str(id.to_string()));
            if let Some(model) = declaration.model_type {
                row.set(hash40("mii_model_type"), Param::Hash(model.hash()));
            }
            if !declaration.hair_model_types.is_empty() {
                let entries = declaration
                    .hair_model_types
                    .iter()
                    .map(|(hair, model)| {
                        Param::Struct(vec![
                            (hash40("hair_type"), Param::I32(*hair)),
                            (hash40("mii_model_type"), Param::Hash(model.hash())),
                        ])
                    })
                    .collect();
                row.set(HAIR_MODEL_TYPES, Param::List(entries));
            }
        }
        Part::Outfit => {
            let name = declaration.icon_name();
            row.set(hash40("name_id"), Param::Str(name.clone()));
            row.set(hash40("replace_id"), Param::Str(name));
            if let Some(fighter) = declaration.fighter {
                row.set(hash40("type"), Param::Hash(fighter.row_type()));
            }
            if let Some(gender) = declaration.gender {
                row.set(hash40("gender"), Param::Hash(gender.hash()));
            }
            set_keep_type(&mut row, hash40("normal_suit"), 0);
        }
    }
    row
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Added { id: String, save_no: i16, assigned: Assigned },
    UnknownBase { id: String, base: String },
    TakenId { id: String },
    Full { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Added {
    pub ui_hash: u64,
    pub save_no: i16,
    pub owned: bool,
}

pub fn append(
    root: &mut Param,
    part: Part,
    declarations: &[&Declaration],
    registry: &mut Registry,
) -> (Vec<Outcome>, Vec<Added>) {
    let mut outcomes = Vec::new();
    let mut added = Vec::new();
    let first = first_free_save_no(root);
    let mut taken = taken_save_numbers(root);
    let mut disp_order = next_disp_order(root);
    for declaration in declarations.iter().filter(|d| d.part == part) {
        let id = declaration.id.clone();
        if find_row(root, part, &id).is_some() {
            outcomes.push(Outcome::TakenId { id });
            continue;
        }
        let Some(base) = find_row(root, part, &declaration.base).cloned() else {
            outcomes.push(Outcome::UnknownBase {
                id,
                base: declaration.base.clone(),
            });
            continue;
        };
        let Some(assigned) = registry.assign(part, &id, &declaration.base, &taken, first) else {
            outcomes.push(Outcome::Full { id });
            continue;
        };
        let save_no = assigned.save_no();
        let order = declaration.disp_order.unwrap_or(disp_order);
        if declaration.disp_order.is_none() {
            disp_order = disp_order.saturating_add(1);
        }
        let row = build_row(&base, declaration, save_no, order);
        if let Some(list) = rows_mut(root) {
            list.push(row);
        }
        taken.insert(save_no);
        added.push(Added {
            ui_hash: declaration.ui_hash() & HASH_MASK,
            save_no,
            owned: declaration.owned,
        });
        outcomes.push(Outcome::Added { id, save_no, assigned });
    }
    (outcomes, added)
}

pub fn ghost_row(base: &Param, entry: &Entry) -> Param {
    let mut row = base.clone();
    row.set(entry.part.ui_key(), Param::Hash(hash40(&ui_id(entry.part, &entry.id))));
    row.set(hash40("save_no"), Param::I16(entry.save_no));
    row.set(hash40("motif_id"), Param::Hash(hash40(&entry.id)));
    row.set(hash40("dlc_type"), Param::Hash(hash40("dlc_none")));
    row.set(hash40("prize_lottery"), Param::Bool(false));
    row.set(hash40("shop_item_tag"), Param::Hash(0));
    row.set(START_OWNED, Param::Bool(false));
    set_keep_type(&mut row, hash40("disp_order"), -1);
    if entry.part == Part::Outfit {
        set_keep_type(&mut row, hash40("normal_suit"), 0);
    }
    row
}

pub const MII_PARAM_PATHS: [&str; 3] = [
    "fighter/miifighter/param/vl.prc",
    "fighter/miiswordsman/param/vl.prc",
    "fighter/miigunner/param/vl.prc",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartRows {
    pub part: Part,
    pub id: String,
    pub base: String,
    pub copied: usize,
    pub own: bool,
}

pub fn part_ui_hash(part: Part, id: &str) -> u64 {
    hash40(&ui_id(part, id).to_ascii_lowercase())
}

fn field_naming(row: &Param, hash: u64) -> Option<u64> {
    match row {
        Param::Struct(fields) => fields
            .iter()
            .find(|(_, value)| matches!(value, Param::Hash(found) if *found == hash))
            .map(|(key, _)| *key),
        _ => None,
    }
}

fn copy_rows(param: &mut Param, from: u64, to: u64, rows: &mut PartRows) {
    match param {
        Param::Struct(fields) => {
            for (_, value) in fields.iter_mut() {
                copy_rows(value, from, to, rows);
            }
        }
        Param::List(items) => {
            for item in items.iter_mut() {
                copy_rows(item, from, to, rows);
            }
            if items.iter().any(|row| field_naming(row, to).is_some()) {
                rows.own = true;
                return;
            }
            let copies: Vec<Param> = items
                .iter()
                .filter_map(|row| {
                    let key = field_naming(row, from)?;
                    let mut copy = row.clone();
                    copy.set(key, Param::Hash(to));
                    Some(copy)
                })
                .collect();
            rows.copied += copies.len();
            items.extend(copies);
        }
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartSource {
    pub part: Part,
    pub id: String,
    pub base: String,
    pub base_hash: u64,
}

pub fn base_ui_hash(root: &Param, part: Part, base: &str, fighter: Option<MiiFighter>) -> Option<u64> {
    let named: Vec<&Param> = rows(root)?.iter().filter(|row| row_id(row, part) == Some(base)).collect();
    let typed = fighter.and_then(|fighter| {
        named
            .iter()
            .find(|row| row.get(hash40("type")).and_then(Param::hash) == Some(fighter.row_type()))
            .copied()
    });
    typed.or_else(|| named.first().copied())?.get(part.ui_key()).and_then(Param::hash)
}

pub fn add_part_rows(root: &mut Param, parts: &[PartSource]) -> Vec<PartRows> {
    parts
        .iter()
        .map(|source| {
            let mut rows = PartRows {
                part: source.part,
                id: source.id.clone(),
                base: source.base.clone(),
                copied: 0,
                own: false,
            };
            let to = part_ui_hash(source.part, &source.id);
            if source.base_hash != to {
                copy_rows(root, source.base_hash, to, &mut rows);
            }
            rows
        })
        .collect()
}

pub fn add_ghosts(root: &mut Param, part: Part, registry: &Registry) -> Vec<Entry> {
    let mut out = Vec::new();
    for entry in registry.entries.iter().filter(|e| e.part == part) {
        let ui_hash = hash40(&ui_id(part, &entry.id));
        let present = rows(root).map(|rows| {
            rows.iter().any(|row| {
                row_save_no(row) == Some(entry.save_no)
                    || row.get(part.ui_key()).and_then(Param::hash) == Some(ui_hash)
            })
        });
        if present != Some(false) {
            continue;
        }
        let Some(base) = find_row(root, part, &entry.base).cloned() else {
            continue;
        };
        let ghost = ghost_row(&base, entry);
        if let Some(list) = rows_mut(root) {
            list.push(ghost);
            out.push(entry.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prc;

    fn keyed_row(key: &str, ui: &str, tex: &str) -> Param {
        Param::Struct(vec![
            (hash40("tex_id"), Param::Hash(hash40(tex))),
            (hash40(key), Param::Hash(hash40(ui))),
        ])
    }

    fn swordfighter_params() -> Param {
        Param::Struct(vec![
            (hash40("walk_speed_max"), Param::Float(1.0)),
            (hash40("param_mii_effect_id"), Param::List(vec![
                keyed_row("mii_body", "ui_mii_body_normalswordman_m", "tex_miiswordsman_sword01"),
                keyed_row("mii_body", "ui_mii_body_p3hero_m", "tex_miiswordsman_sword40"),
                keyed_row("mii_body", "ui_mii_body_ironsuit_m", "tex_miiswordsman_sword09"),
            ])),
            (0x1e_1553_8020, Param::List(vec![keyed_row("mii_body", "ui_mii_body_rex_m", "")])),
            (hash40("deep"), Param::Struct(vec![(hash40("hat_rows"), Param::List(vec![
                keyed_row("mii_hat", "ui_mii_hat_h_sans", "tex_hat"),
            ]))])),
        ])
    }

    fn texture_of(list: &Param, key: &str, ui: &str) -> Vec<u64> {
        list.list().unwrap().iter()
            .filter(|row| row.get(hash40(key)).and_then(Param::hash) == Some(hash40(ui)))
            .filter_map(|row| row.get(hash40("tex_id")).and_then(Param::hash))
            .collect()
    }

    fn trail_of(root: &Param, ui: &str) -> Vec<u64> {
        texture_of(root.get(hash40("param_mii_effect_id")).unwrap(), "mii_body", ui)
    }

    fn source(part: Part, id: &str, base: &str) -> PartSource {
        PartSource { part, id: id.into(), base: base.into(), base_hash: part_ui_hash(part, base) }
    }

    fn outfit(id: &str, base: &str) -> PartSource {
        source(Part::Outfit, id, base)
    }

    #[test]
    fn the_base_id_comes_from_the_row_of_the_new_outfits_fighter() {
        let maid = |kind: &str, ui: &str| Param::Struct(vec![
            (hash40("type"), Param::Hash(hash40(kind))),
            (hash40("mii_body_id"), Param::Str("b_maid_f".into())),
            (hash40("ui_mii_body_id"), Param::Hash(hash40(ui))),
        ]);
        let root = Param::Struct(vec![(hash40("db_root"), Param::List(vec![
            maid("mii_fighter", "ui_mii_body_maidfighter_f"),
            maid("mii_swordsman", "ui_mii_body_maidswordsman_f"),
        ]))]);
        assert_eq!(base_ui_hash(&root, Part::Outfit, "b_maid_f", Some(MiiFighter::Swordfighter)),
                   Some(hash40("ui_mii_body_maidswordsman_f")));
        assert_eq!(base_ui_hash(&root, Part::Outfit, "b_maid_f", None), Some(hash40("ui_mii_body_maidfighter_f")));
        assert_eq!(base_ui_hash(&root, Part::Outfit, "b_none_f", None), None);
    }

    #[test]
    fn a_new_part_takes_every_row_its_base_has() {
        let mut root = swordfighter_params();
        let rows = add_part_rows(&mut root, &[outfit("b_kotone_m", "b_p3hero_m"), outfit("b_tin_m", "b_ironSuit_m"),
                                              outfit("b_blade_m", "b_rex_m"),
                                              source(Part::Hat, "h_mask", "h_sans")]);
        assert_eq!(trail_of(&root, "ui_mii_body_kotone_m"), vec![hash40("tex_miiswordsman_sword40")]);
        assert_eq!(trail_of(&root, "ui_mii_body_tin_m"), vec![hash40("tex_miiswordsman_sword09")]);
        assert_eq!(rows.iter().map(|r| r.copied).collect::<Vec<_>>(), vec![1, 1, 1, 1]);
        let joint = root.get(0x1e_1553_8020).and_then(Param::list).unwrap();
        assert_eq!(joint.len(), 2);
        assert_eq!(joint[1].get(hash40("mii_body")).and_then(Param::hash), Some(hash40("ui_mii_body_blade_m")));
        let hats = root.get(hash40("deep")).and_then(|deep| deep.get(hash40("hat_rows"))).unwrap();
        assert_eq!(texture_of(hats, "mii_hat", "ui_mii_hat_h_mask"), vec![hash40("tex_hat")]);
        let reparsed = prc::parse(&prc::write(&root)).unwrap();
        assert_eq!(trail_of(&reparsed, "ui_mii_body_kotone_m"), vec![hash40("tex_miiswordsman_sword40")]);
    }

    #[test]
    fn a_part_with_its_own_row_or_a_base_without_one_gets_nothing() {
        let mut root = swordfighter_params();
        root.get_mut(hash40("param_mii_effect_id")).and_then(Param::list_mut).unwrap()
            .push(keyed_row("mii_body", "ui_mii_body_kotone_m", "tex_miiswordsman_sword71"));
        let before = root.clone();
        let rows = add_part_rows(&mut root, &[outfit("b_kotone_m", "b_p3hero_m"), outfit("b_brawl_m", "b_chinese_m")]);
        assert_eq!(trail_of(&root, "ui_mii_body_kotone_m"), vec![hash40("tex_miiswordsman_sword71")]);
        assert_eq!((rows[0].copied, rows[0].own), (0, true));
        assert_eq!((rows[1].copied, rows[1].own), (0, false));
        assert_eq!(root, before);
    }

    fn hat_row(id: &str, save_no: i16, disp: i32) -> Param {
        Param::Struct(vec![
            (hash40("text_id"), Param::Str(format!("{id}_m"))),
            (hash40("name_id"), Param::Str(id.into())),
            (hash40("save_no"), Param::I16(save_no)),
            (hash40("is_patch"), Param::Bool(false)),
            (hash40("motif_id"), Param::Hash(hash40(id))),
            (hash40("dlc_type"), Param::Hash(hash40("dlc_buying"))),
            (hash40("shop_price"), Param::U32(450)),
            (hash40("mii_hat_id"), Param::Str(id.into())),
            (hash40("disp_order"), Param::I32(disp)),
            (hash40("ui_mii_hat_id"), Param::Hash(hash40(&format!("ui_mii_hat_{id}")))),
            (hash40("prize_lottery"), Param::Bool(true)),
            (hash40("shop_item_tag"), Param::Hash(hash40("fd-02-31"))),
            (hash40("swing_enabled"), Param::Bool(false)),
            (hash40("mii_model_type"), Param::Hash(ModelType::Hat.hash())),
            (hash40("valid_resource"), Param::I8(1)),
            (
                HAIR_MODEL_TYPES,
                Param::List(vec![Param::Struct(vec![
                    (hash40("hair_type"), Param::I32(-1)),
                    (hash40("mii_model_type"), Param::Hash(ModelType::Normal.hash())),
                ])]),
            ),
            (START_OWNED, Param::Bool(false)),
        ])
    }

    fn body_row(id: &str, save_no: i16) -> Param {
        Param::Struct(vec![
            (hash40("type"), Param::Hash(hash40("mii_fighter"))),
            (hash40("gender"), Param::Hash(hash40("gender_male"))),
            (hash40("text_id"), Param::Str(id.into())),
            (hash40("name_id"), Param::Str(outfit_name(id))),
            (hash40("save_no"), Param::I16(save_no)),
            (hash40("replace_id"), Param::Str(outfit_name(id))),
            (hash40("disp_order"), Param::I32(save_no as i32 + 1)),
            (hash40("mii_body_id"), Param::Str(id.into())),
            (hash40("normal_suit"), Param::U8(1)),
            (hash40("ui_mii_body_id"), Param::Hash(hash40("ui_mii_body_x"))),
            (hash40("valid_resource"), Param::I8(1)),
            (START_OWNED, Param::Bool(true)),
        ])
    }

    fn db(rows: Vec<Param>) -> Param {
        Param::Struct(vec![(hash40("db_root"), Param::List(rows))])
    }

    fn hat_db() -> Param {
        db(vec![
            hat_row("none", -1, 0),
            hat_row("silkhat", 8, 9),
            hat_row("crown", 132, 40),
        ])
    }

    const MANIFEST: &str = r#"
# a pack
[[mii_hat]]
id = "miku_hair"
base = "silkhat"
model_type = "term"
hair_model_types = ["12=face_only", "40 = hat"]

[[mii_outfit]]
id = "b_miku_f"
base = "b_normalfighter_m"
fighter = "miiswordsman"
gender = "female"
owned = false
"#;

    #[test]
    fn reads_hats_and_outfits() {
        let parts = parse(MANIFEST).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].part, Part::Hat);
        assert_eq!(parts[0].model_type, Some(ModelType::Term));
        assert_eq!(
            parts[0].hair_model_types,
            vec![(12, ModelType::FaceOnly), (40, ModelType::Hat)]
        );
        assert!(parts[0].owned);
        assert_eq!(parts[1].fighter, Some(MiiFighter::Swordfighter));
        assert_eq!(parts[1].gender, Some(Gender::Female));
        assert!(!parts[1].owned);
        assert_eq!(parts[1].icon_name(), "b_miku");
        assert_eq!(parts[1].ui_id(), "ui_mii_body_miku_f");
        assert_eq!(parts[1].folder(), "miibody/miiswordsman/b_miku_f");
        assert_eq!(parts[0].folder(), "miihat/miku_hair");
    }

    #[test]
    fn refuses_what_it_cannot_place() {
        assert!(matches!(
            parse("[[mii_hat]]\nbase = \"silkhat\"\n"),
            Err(MiiError::Missing { key: "id", .. })
        ));
        assert!(matches!(
            parse("[[mii_outfit]]\nid = \"b_x_m\"\nbase = \"b_chinese_m\"\n"),
            Err(MiiError::Missing { key: "fighter", .. })
        ));
        assert!(matches!(
            parse("[[mii_outfit]]\nid = \"x_m\"\nbase = \"b_chinese_m\"\nfighter = \"miigunner\"\n"),
            Err(MiiError::BadId { .. })
        ));
        assert!(matches!(
            parse("[[mii_hat]]\nid = \"Bad Id\"\nbase = \"silkhat\"\n"),
            Err(MiiError::BadId { .. })
        ));
        assert!(matches!(
            parse("[[mii_hat]]\nid = \"a\"\nbase = \"b\"\nmodel_type = \"big\"\n"),
            Err(MiiError::BadValue { .. })
        ));
        assert!(matches!(
            parse("[[mii_hat]]\nid = \"a\"\nbase = \"b\"\nfighter = \"miigunner\"\n"),
            Err(MiiError::UnknownKey { .. })
        ));
        assert!(matches!(
            parse("[[mii_hat]]\nid = \"a\"\nbase = \"b\"\n[[mii_hat]]\nid = \"a\"\nbase = \"b\"\n"),
            Err(MiiError::Duplicate { .. })
        ));
        assert!(matches!(parse("id = \"a\"\n"), Err(MiiError::Malformed { .. })));
        assert!(matches!(
            parse("[[mii_hair]]\n"),
            Err(MiiError::UnknownTable { .. })
        ));
    }

    #[test]
    fn numbers_start_past_the_game_and_stay_put() {
        let root = hat_db();
        let mut registry = Registry::default();
        let taken = taken_save_numbers(&root);
        let first = first_free_save_no(&root);
        assert_eq!(first, 133);
        assert_eq!(registry.assign(Part::Hat, "a", "silkhat", &taken, first), Some(Assigned::New(133)));
        assert_eq!(registry.assign(Part::Hat, "b", "silkhat", &taken, first), Some(Assigned::New(134)));
        assert_eq!(registry.assign(Part::Hat, "a", "crown", &taken, first), Some(Assigned::Kept(133)));
        assert_eq!(registry.find(Part::Hat, "a").unwrap().base, "crown");
        let text = registry.serialise();
        let mut again = Registry::parse(&text);
        assert_eq!(again, registry);
        assert_eq!(again.assign(Part::Hat, "c", "silkhat", &taken, first), Some(Assigned::New(135)));
        assert_eq!(again.assign(Part::Outfit, "b_c_m", "x", &taken, 154), Some(Assigned::New(154)));
    }

    #[test]
    fn an_absent_pack_keeps_its_number() {
        let root = hat_db();
        let mut registry = Registry::parse("hat 133 gone silkhat\n");
        let taken = taken_save_numbers(&root);
        assert_eq!(registry.assign(Part::Hat, "new", "silkhat", &taken, 133), Some(Assigned::New(134)));
    }

    #[test]
    fn a_number_the_game_took_later_moves() {
        let mut root = hat_db();
        rows_mut(&mut root).unwrap().push(hat_row("dlc", 133, 50));
        let mut registry = Registry::parse("hat 133 mine silkhat\n");
        let taken = taken_save_numbers(&root);
        let first = first_free_save_no(&root);
        assert_eq!(
            registry.assign(Part::Hat, "mine", "silkhat", &taken, first),
            Some(Assigned::Moved { from: 133, to: 134 })
        );
    }

    #[test]
    fn the_range_ends_where_the_save_bits_end() {
        let root = hat_db();
        let taken = taken_save_numbers(&root);
        let mut registry = Registry::default();
        for n in 0..(HAT_SAVE_LAST - 133 + 1) {
            assert!(registry.assign(Part::Hat, &format!("h{n}"), "silkhat", &taken, 133).is_some());
        }
        assert_eq!(registry.assign(Part::Hat, "over", "silkhat", &taken, 133), None);
        assert_eq!(registry.entries.iter().map(|e| e.save_no).max(), Some(HAT_SAVE_LAST));
    }

    #[test]
    fn the_registry_skips_lines_it_cannot_trust() {
        let registry = Registry::parse(
            "hat 133 a silkhat\nhat 133 b silkhat\nhat 999 c silkhat\nhat -4 d x\nscarf 140 e x\nhat 140 a crown\noutfit 900 b_q_m b_x_m\n",
        );
        assert_eq!(registry.entries.len(), 2);
    }

    #[test]
    fn a_hat_row_copies_its_base_and_takes_its_own_identity() {
        let mut root = hat_db();
        let parts = parse(MANIFEST).unwrap();
        let mut registry = Registry::default();
        let declarations: Vec<&Declaration> = parts.iter().collect();
        let (outcomes, added) = append(&mut root, Part::Hat, &declarations, &mut registry);
        assert_eq!(
            outcomes,
            vec![Outcome::Added {
                id: "miku_hair".into(),
                save_no: 133,
                assigned: Assigned::New(133)
            }]
        );
        assert_eq!(added[0].save_no, 133);
        assert!(added[0].owned);
        let row = find_row(&root, Part::Hat, "miku_hair").unwrap();
        assert_eq!(row.get(hash40("ui_mii_hat_id")), Some(&Param::Hash(hash40("ui_mii_hat_miku_hair"))));
        assert_eq!(row.get(hash40("text_id")).and_then(Param::text), Some("miku_hair"));
        assert_eq!(row.get(hash40("name_id")).and_then(Param::text), Some("miku_hair"));
        assert_eq!(row.get(hash40("save_no")), Some(&Param::I16(133)));
        assert_eq!(row.get(hash40("dlc_type")), Some(&Param::Hash(hash40("dlc_none"))));
        assert_eq!(row.get(hash40("motif_id")), Some(&Param::Hash(hash40("miku_hair"))));
        assert_eq!(row.get(hash40("shop_item_tag")), Some(&Param::Hash(0)));
        assert_eq!(row.get(hash40("prize_lottery")), Some(&Param::Bool(false)));
        assert_eq!(row.get(hash40("disp_order")), Some(&Param::I32(41)));
        assert_eq!(row.get(hash40("mii_model_type")), Some(&Param::Hash(ModelType::Term.hash())));
        assert_eq!(row.get(START_OWNED), Some(&Param::Bool(true)));
        assert_eq!(row.get(hash40("shop_price")), Some(&Param::U32(450)));
        let list = row.get(HAIR_MODEL_TYPES).unwrap().list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[1].get(hash40("hair_type")), Some(&Param::I32(40)));
        let read = prc::parse(&prc::write(&root)).unwrap();
        assert_eq!(read, root);
        let base = find_row(&read, Part::Hat, "silkhat").unwrap();
        assert_eq!(base.get(hash40("save_no")), Some(&Param::I16(8)));
    }

    #[test]
    fn an_outfit_row_takes_its_type_gender_and_names() {
        let mut root = db(vec![body_row("none", -1), body_row("b_normalfighter_m", 0), body_row("b_chinese_m", 153)]);
        let parts = parse(MANIFEST).unwrap();
        let declarations: Vec<&Declaration> = parts.iter().collect();
        let mut registry = Registry::default();
        let (outcomes, added) = append(&mut root, Part::Outfit, &declarations, &mut registry);
        assert!(matches!(outcomes[0], Outcome::Added { save_no: 154, .. }));
        assert!(!added[0].owned);
        let row = find_row(&root, Part::Outfit, "b_miku_f").unwrap();
        assert_eq!(row.get(hash40("type")), Some(&Param::Hash(hash40("mii_swordsman"))));
        assert_eq!(row.get(hash40("gender")), Some(&Param::Hash(hash40("gender_female"))));
        assert_eq!(row.get(hash40("replace_id")).and_then(Param::text), Some("b_miku"));
        assert_eq!(row.get(hash40("name_id")).and_then(Param::text), Some("b_miku"));
        assert_eq!(row.get(hash40("text_id")).and_then(Param::text), Some("b_miku_f"));
        assert_eq!(row.get(hash40("normal_suit")), Some(&Param::U8(0)));
        assert_eq!(row.get(hash40("ui_mii_body_id")), Some(&Param::Hash(hash40("ui_mii_body_miku_f"))));
    }

    #[test]
    fn a_missing_base_or_a_taken_id_adds_nothing() {
        let mut root = hat_db();
        let parts = parse(
            "[[mii_hat]]\nid = \"crown\"\nbase = \"silkhat\"\n[[mii_hat]]\nid = \"x\"\nbase = \"nope\"\n",
        )
        .unwrap();
        let declarations: Vec<&Declaration> = parts.iter().collect();
        let mut registry = Registry::default();
        let (outcomes, added) = append(&mut root, Part::Hat, &declarations, &mut registry);
        assert_eq!(
            outcomes,
            vec![
                Outcome::TakenId { id: "crown".into() },
                Outcome::UnknownBase { id: "x".into(), base: "nope".into() }
            ]
        );
        assert!(added.is_empty());
        assert!(registry.entries.is_empty());
        assert_eq!(rows(&root).unwrap().len(), 3);
    }

    #[test]
    fn a_removed_pack_leaves_a_hidden_copy_of_its_base() {
        let mut root = hat_db();
        let registry = Registry::parse("hat 133 gone silkhat\nhat 134 lost nothing\n");
        let ghosts = add_ghosts(&mut root, Part::Hat, &registry);
        assert_eq!(ghosts.len(), 1);
        assert_eq!(ghosts[0].id, "gone");
        let rows = rows(&root).unwrap();
        assert_eq!(rows.len(), 4);
        let ghost = &rows[3];
        assert_eq!(ghost.get(hash40("ui_mii_hat_id")), Some(&Param::Hash(hash40("ui_mii_hat_gone"))));
        assert_eq!(ghost.get(hash40("save_no")), Some(&Param::I16(133)));
        assert_eq!(ghost.get(hash40("mii_hat_id")).and_then(Param::text), Some("silkhat"));
        assert_eq!(ghost.get(hash40("disp_order")), Some(&Param::I32(-1)));
        assert_eq!(ghost.get(START_OWNED), Some(&Param::Bool(false)));
        assert_eq!(ghost.get(hash40("dlc_type")), Some(&Param::Hash(hash40("dlc_none"))));
    }

    #[test]
    fn an_installed_pack_needs_no_ghost() {
        let mut root = hat_db();
        let parts = parse(MANIFEST).unwrap();
        let declarations: Vec<&Declaration> = parts.iter().collect();
        let mut registry = Registry::default();
        append(&mut root, Part::Hat, &declarations, &mut registry);
        let before = rows(&root).unwrap().len();
        assert!(add_ghosts(&mut root, Part::Hat, &registry).is_empty());
        assert_eq!(rows(&root).unwrap().len(), before);
    }

    #[test]
    fn a_ghost_never_takes_a_number_the_game_uses() {
        let mut root = hat_db();
        let registry = Registry::parse("hat 132 clash silkhat\n");
        assert!(add_ghosts(&mut root, Part::Hat, &registry).is_empty());
    }

    #[test]
    fn outfit_ghosts_keep_the_base_folder() {
        let mut root = db(vec![body_row("none", -1), body_row("b_chinese_m", 4)]);
        let registry = Registry::parse("outfit 154 b_gone_m b_chinese_m\n");
        let ghosts = add_ghosts(&mut root, Part::Outfit, &registry);
        assert_eq!(ghosts.len(), 1);
        let ghost = &rows(&root).unwrap()[2];
        assert_eq!(ghost.get(hash40("mii_body_id")).and_then(Param::text), Some("b_chinese_m"));
        assert_eq!(ghost.get(hash40("ui_mii_body_id")), Some(&Param::Hash(hash40("ui_mii_body_gone_m"))));
        assert_eq!(ghost.get(hash40("normal_suit")), Some(&Param::U8(0)));
    }

    #[test]
    fn adds_rows_to_a_real_database_when_one_is_given() {
        let Ok(path) = std::env::var("CLONE_ENGINE_MII_HAT_DB") else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let mut root = prc::parse(&bytes).unwrap();
        let before = rows(&root).unwrap().len();
        assert_eq!(first_free_save_no(&root), 133);
        let parts = parse(MANIFEST).unwrap();
        let declarations: Vec<&Declaration> = parts.iter().collect();
        let mut registry = Registry::default();
        let (outcomes, _) = append(&mut root, Part::Hat, &declarations, &mut registry);
        assert!(matches!(outcomes[0], Outcome::Added { save_no: 133, .. }));
        let written = prc::write(&root);
        if let Ok(out) = std::env::var("CLONE_ENGINE_MII_HAT_DB_OUT") {
            std::fs::write(out, &written).unwrap();
        }
        let again = prc::parse(&written).unwrap();
        assert_eq!(rows(&again).unwrap().len(), before + 1);
        let row = find_row(&again, Part::Hat, "miku_hair").unwrap();
        assert_eq!(row.get(hash40("mii_model_type")), Some(&Param::Hash(ModelType::Term.hash())));
    }
}
