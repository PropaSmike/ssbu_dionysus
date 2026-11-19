use std::collections::BTreeSet;

use crate::hash40;
use crate::prc::Param;

pub const DB_PATH: &str = "ui/param/database/ui_spirit_db.prc";
pub const BATTLE_DB_PATH: &str = "ui/param/database/ui_spirits_battle_db.prc";
pub const LAYOUT_DB_PATH: &str = "ui/param/database/ui_spirit_layout_db.prc";
pub const BOARD_DB_PATH: &str = "ui/param_patch/spirits_board_special/spirits_board_special_param.prc";
pub const BOARD_SAVE_FIRST: u16 = 11;
pub const BOARD_SAVE_LAST: u16 = 31;
pub const MAX_BOARDS: usize = (BOARD_SAVE_LAST - BOARD_SAVE_FIRST + 1) as usize;
pub const MAX_BOARD_SPIRITS: usize = 64;
pub const MAX_NAME: usize = 64;
pub const SAVE_FIRST: u16 = 1513;
pub const SAVE_LAST: u16 = 1999;
pub const FIXED_BASE: u16 = 10000;
pub const MAX_ID: usize = 48;
pub const MAX_SPIRITS: usize = (SAVE_LAST - SAVE_FIRST + 1) as usize;
pub const MAX_FIGHTERS: usize = 16;
pub const HASH_MASK: u64 = 0xff_ffff_ffff;

const ROW_FIXED: [&str; 5] = ["ui_spirit_id", "name_id", "save_no", "directory_id", "fixed_no"];
const BATTLE_FIXED: [&str; 1] = ["battle_id"];
const BOARD_FIXED: [&str; 3] = ["label", "save_no", "spirit_list"];
const FIGHTER_FIXED: [&str; 1] = ["battle_id"];
const LAYOUT_FIXED: [&str; 1] = ["ui_spirit_layout_id"];

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Text(String),
    Flag(bool),
    Number(i64),
    Float(f64),
    List(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Spirit,
    Battle,
    Fighter,
    Layout,
}

impl Table {
    pub fn name(self) -> &'static str {
        match self {
            Table::Spirit => "spirit",
            Table::Battle => "battle",
            Table::Fighter => "fighter",
            Table::Layout => "layout",
        }
    }

    pub(crate) fn fixed(self) -> &'static [&'static str] {
        match self {
            Table::Spirit => &ROW_FIXED,
            Table::Battle => &BATTLE_FIXED,
            Table::Fighter => &FIGHTER_FIXED,
            Table::Layout => &LAYOUT_FIXED,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub table: Table,
    pub fighter: usize,
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub id: String,
    pub base: String,
    pub fighters: Option<usize>,
    pub fields: Vec<Field>,
    pub line: usize,
}

impl Declaration {
    pub fn id_hash(&self) -> u64 {
        hash40(&self.id)
    }

    pub fn base_hash(&self) -> u64 {
        name_key(&self.base)
    }

    pub fn fields_of(&self, table: Table) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(move |field| field.table == table)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpiritError {
    UnknownTable { line: usize, table: String },
    Malformed { line: usize },
    BadValue { line: usize, key: String },
    BadKey { line: usize, key: String },
    Fixed { line: usize, key: String },
    Missing { line: usize, key: &'static str },
    BadId { line: usize, id: String },
    Duplicate { line: usize, id: String },
    TooMany,
    Prcxml { file: &'static str, row: usize, message: String },
}

pub fn name_key(text: &str) -> u64 {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        if let Ok(value) = u64::from_str_radix(hex, 16) {
            return value & HASH_MASK;
        }
    }
    hash40(text)
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_field(name: &str) -> bool {
    if let Some(hex) = name.strip_prefix("0x") {
        return !hex.is_empty() && hex.len() <= 10 && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    valid_id(name)
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
            items.push(item.strip_prefix('"')?.strip_suffix('"')?.trim().to_string());
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
    let digits = raw.replace('_', "");
    if let Ok(number) = digits.parse::<i64>() {
        return Some(Value::Number(number));
    }
    if digits.contains(['.', 'e', 'E']) {
        if let Ok(number) = digits.parse::<f64>() {
            if number.is_finite() {
                return Some(Value::Float(number));
            }
        }
    }
    None
}

fn field_of(key: &str) -> Option<(Table, usize, String)> {
    let parts: Vec<&str> = key.split('.').map(str::trim).collect();
    let (table, fighter, name) = match parts.as_slice() {
        [name] => (Table::Spirit, 0, *name),
        ["battle", name] => (Table::Battle, 0, *name),
        ["layout", name] => (Table::Layout, 0, *name),
        ["fighter", index, name] => {
            let index = index.parse::<usize>().ok()?;
            if index >= MAX_FIGHTERS {
                return None;
            }
            (Table::Fighter, index, *name)
        }
        _ => return None,
    };
    valid_field(name).then(|| (table, fighter, name.to_string()))
}

pub(crate) fn finish(block: Declaration, out: &mut Vec<Declaration>) -> Result<(), SpiritError> {
    let line = block.line;
    if block.id.is_empty() {
        return Err(SpiritError::Missing { line, key: "id" });
    }
    if block.base.is_empty() {
        return Err(SpiritError::Missing { line, key: "base" });
    }
    if !valid_id(&block.id) {
        return Err(SpiritError::BadId { line, id: block.id });
    }
    if out.iter().any(|other| other.id == block.id) {
        return Err(SpiritError::Duplicate { line, id: block.id });
    }
    if out.len() >= MAX_SPIRITS {
        return Err(SpiritError::TooMany);
    }
    out.push(block);
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct Board {
    pub id: String,
    pub base: String,
    pub name: String,
    pub background: Option<String>,
    pub spirits: Vec<String>,
    pub art: Option<String>,
    pub fields: Vec<Field>,
    pub line: usize,
}

impl Board {
    pub fn art_root(&self) -> Option<String> {
        self.art.as_deref().and_then(crate::menu::layout_root)
    }

    pub fn label(&self) -> u64 {
        board_label(&self.id)
    }

    pub fn base_label(&self) -> u64 {
        fighter_label(&self.base)
    }

    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            &self.id
        } else {
            &self.name
        }
    }
}

pub fn board_label(id: &str) -> u64 {
    hash40(&format!("ui_chara_{id}"))
}

pub fn fighter_label(name: &str) -> u64 {
    let name = name.trim();
    if name.starts_with("ui_chara_") || name.starts_with("0x") || name.starts_with("0X") {
        name_key(name)
    } else {
        hash40(&format!("ui_chara_{}", name.to_ascii_lowercase()))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Manifest {
    pub spirits: Vec<Declaration>,
    pub boards: Vec<Board>,
}

enum Block {
    Spirit(Declaration),
    Board(Board),
}

pub(crate) fn finish_board(board: Board, out: &mut Vec<Board>) -> Result<(), SpiritError> {
    let line = board.line;
    if board.id.is_empty() {
        return Err(SpiritError::Missing { line, key: "id" });
    }
    if board.base.is_empty() {
        return Err(SpiritError::Missing { line, key: "base" });
    }
    if board.spirits.is_empty() {
        return Err(SpiritError::Missing { line, key: "spirits" });
    }
    if !valid_id(&board.id) {
        return Err(SpiritError::BadId { line, id: board.id });
    }
    if out.iter().any(|other| other.id == board.id) {
        return Err(SpiritError::Duplicate { line, id: board.id });
    }
    if out.len() >= MAX_BOARDS {
        return Err(SpiritError::TooMany);
    }
    out.push(board);
    Ok(())
}

fn finish_block(block: Block, manifest: &mut Manifest) -> Result<(), SpiritError> {
    match block {
        Block::Spirit(declaration) => finish(declaration, &mut manifest.spirits),
        Block::Board(board) => finish_board(board, &mut manifest.boards),
    }
}

fn board_key(board: &mut Board, key: &str, value: Value, line: usize) -> Result<(), SpiritError> {
    let bad = || SpiritError::BadValue {
        line,
        key: key.to_string(),
    };
    match (key, value) {
        ("id", Value::Text(text)) => board.id = text.trim().to_string(),
        ("base", Value::Text(text)) => board.base = text.trim().to_string(),
        ("name", Value::Text(text)) if text.len() <= MAX_NAME && !text.contains(['\n', '\r']) => board.name = text,
        ("background", Value::Text(text)) => board.background = Some(text.trim().to_string()),
        ("art", Value::Text(text)) if crate::menu::layout_root(&text).is_some() => {
            board.art = Some(text.trim().to_ascii_lowercase())
        }
        ("spirits", Value::List(items))
            if items.len() <= MAX_BOARD_SPIRITS && items.iter().all(|item| valid_field(item)) =>
        {
            board.spirits = items
        }
        ("id" | "base" | "name" | "background" | "art" | "spirits", _) | (_, Value::List(_)) => return Err(bad()),
        (key, _) if !valid_field(key) => {
            return Err(SpiritError::BadKey {
                line,
                key: key.to_string(),
            })
        }
        (key, _) if BOARD_FIXED.contains(&key) => {
            return Err(SpiritError::Fixed {
                line,
                key: key.to_string(),
            })
        }
        (key, value) => {
            board.fields.retain(|field| field.name != key);
            board.fields.push(Field {
                table: Table::Spirit,
                fighter: 0,
                name: key.to_string(),
                value,
            });
        }
    }
    Ok(())
}

pub fn parse(text: &str) -> Result<Vec<Declaration>, SpiritError> {
    parse_manifest(text).map(|manifest| manifest.spirits)
}

pub fn parse_manifest(text: &str) -> Result<Manifest, SpiritError> {
    let mut manifest = Manifest::default();
    let mut pending: Option<Block> = None;
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
            if let Some(done) = pending.take() {
                finish_block(done, &mut manifest)?;
            }
            pending = Some(match name.trim() {
                "spirit" => Block::Spirit(Declaration {
                    id: String::new(),
                    base: String::new(),
                    fighters: None,
                    fields: Vec::new(),
                    line: number,
                }),
                "board" => Block::Board(Board {
                    id: String::new(),
                    base: String::new(),
                    name: String::new(),
                    background: None,
                    spirits: Vec::new(),
                    art: None,
                    fields: Vec::new(),
                    line: number,
                }),
                other => {
                    return Err(SpiritError::UnknownTable {
                        line: number,
                        table: other.to_string(),
                    })
                }
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(SpiritError::Malformed { line: number });
        };
        let key = key.trim();
        let bad = || SpiritError::BadValue {
            line: number,
            key: key.to_string(),
        };
        let value = parse_value(value).ok_or_else(bad)?;
        let block = match pending.as_mut() {
            Some(Block::Spirit(block)) => block,
            Some(Block::Board(board)) => {
                board_key(board, key, value, number)?;
                continue;
            }
            None => return Err(SpiritError::Malformed { line: number }),
        };
        match (key, value) {
            ("id", Value::Text(text)) => block.id = text.trim().to_string(),
            ("base", Value::Text(text)) => block.base = text.trim().to_string(),
            ("fighters", Value::Number(n)) if (1..=MAX_FIGHTERS as i64).contains(&n) => {
                block.fighters = Some(n as usize)
            }
            ("id" | "base" | "fighters", _) => return Err(bad()),
            (key, value) => {
                let Some((table, fighter, name)) = field_of(key) else {
                    return Err(SpiritError::BadKey {
                        line: number,
                        key: key.to_string(),
                    });
                };
                if table.fixed().contains(&name.as_str()) {
                    return Err(SpiritError::Fixed {
                        line: number,
                        key: key.to_string(),
                    });
                }
                block.fields.retain(|f| !(f.table == table && f.fighter == fighter && f.name == name));
                block.fields.push(Field {
                    table,
                    fighter,
                    name,
                    value,
                });
            }
        }
    }
    if let Some(done) = pending.take() {
        finish_block(done, &mut manifest)?;
    }
    Ok(manifest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Unknown,
    WrongType,
    OutOfRange,
}

impl Refusal {
    pub fn text(self) -> &'static str {
        match self {
            Refusal::Unknown => "not a field of the base's row",
            Refusal::WrongType => "the wrong kind of value",
            Refusal::OutOfRange => "out of range",
        }
    }
}

fn whole(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => Some(*n),
        Value::Flag(flag) => Some(i64::from(*flag)),
        _ => None,
    }
}

fn fit<T: TryFrom<i64>>(value: &Value) -> Result<T, Refusal> {
    let number = whole(value).ok_or(Refusal::WrongType)?;
    T::try_from(number).map_err(|_| Refusal::OutOfRange)
}

pub fn converted(existing: &Param, value: &Value) -> Result<Param, Refusal> {
    Ok(match existing {
        Param::Hash(_) => match value {
            Value::Text(text) => Param::Hash(name_key(text)),
            Value::Number(n) if *n >= 0 => Param::Hash(*n as u64 & HASH_MASK),
            _ => return Err(Refusal::WrongType),
        },
        Param::Str(_) => match value {
            Value::Text(text) => Param::Str(text.clone()),
            _ => return Err(Refusal::WrongType),
        },
        Param::Bool(_) => match value {
            Value::Flag(flag) => Param::Bool(*flag),
            Value::Number(0) => Param::Bool(false),
            Value::Number(1) => Param::Bool(true),
            _ => return Err(Refusal::WrongType),
        },
        Param::I8(_) => Param::I8(fit(value)?),
        Param::U8(_) => Param::U8(fit(value)?),
        Param::I16(_) => Param::I16(fit(value)?),
        Param::U16(_) => Param::U16(fit(value)?),
        Param::I32(_) => Param::I32(fit(value)?),
        Param::U32(_) => Param::U32(fit(value)?),
        Param::Float(_) => match value {
            Value::Number(n) => Param::Float(*n as f32),
            Value::Float(f) if f.abs() <= f32::MAX as f64 => Param::Float(*f as f32),
            Value::Float(_) => return Err(Refusal::OutOfRange),
            _ => return Err(Refusal::WrongType),
        },
        Param::List(_) | Param::Struct(_) => return Err(Refusal::WrongType),
    })
}

fn field_key(name: &str) -> u64 {
    name_key(name)
}

pub fn apply(row: &mut Param, field: &Field) -> Result<(), Refusal> {
    let key = field_key(&field.name);
    let existing = row.get(key).ok_or(Refusal::Unknown)?;
    let value = converted(existing, &field.value)?;
    row.set(key, value);
    Ok(())
}

fn set_keep_type(row: &mut Param, key: u64, value: i64) {
    let replacement = match row.get(key) {
        Some(Param::Bool(_)) => Param::Bool(value != 0),
        Some(Param::I8(_)) => Param::I8(value as i8),
        Some(Param::U8(_)) => Param::U8(value as u8),
        Some(Param::I16(_)) => Param::I16(value as i16),
        Some(Param::I32(_)) => Param::I32(value as i32),
        Some(Param::U32(_)) => Param::U32(value as u32),
        _ => Param::U16(value as u16),
    };
    row.set(key, replacement);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub save_no: u16,
    pub id: String,
    pub base: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assigned {
    Kept(u16),
    New(u16),
    Moved { from: u16, to: u16 },
}

impl Assigned {
    pub fn save_no(self) -> u16 {
        match self {
            Assigned::Kept(n) | Assigned::New(n) => n,
            Assigned::Moved { to, .. } => to,
        }
    }
}

impl Registry {
    pub fn parse(text: &str) -> Self {
        Self::parse_range(text, SAVE_FIRST, SAVE_LAST)
    }

    pub fn parse_range(text: &str, first: u16, last: u16) -> Self {
        let mut entries: Vec<Entry> = Vec::new();
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [save_no, id, base] = fields.as_slice() else {
                continue;
            };
            let Ok(save_no) = save_no.parse::<u16>() else {
                continue;
            };
            if !(first..=last).contains(&save_no) || !valid_id(id) || base.is_empty() {
                continue;
            }
            if entries.iter().any(|e| e.save_no == save_no || e.id == *id) {
                continue;
            }
            entries.push(Entry {
                save_no,
                id: id.to_string(),
                base: base.to_string(),
            });
        }
        Registry { entries }
    }

    pub fn serialise(&self) -> String {
        self.serialise_with(
            "# Dionysus spirit save numbers.\n\
             # <save_no> <id> <base>, one per line. The save keeps a spirit's\n\
             # progress by this number, so a line is never reused.\n",
        )
    }

    pub fn serialise_boards(&self) -> String {
        self.serialise_with(
            "# Dionysus spirit board save numbers.\n\
             # <save_no> <id> <base>, one per line. The save keeps a board's\n\
             # prize by this number, so a line is never reused.\n",
        )
    }

    fn serialise_with(&self, header: &str) -> String {
        let mut entries = self.entries.clone();
        entries.sort_by_key(|e| e.save_no);
        let mut out = String::from(header);
        for entry in entries {
            out.push_str(&format!("{} {} {}\n", entry.save_no, entry.id, entry.base));
        }
        out
    }

    pub fn find(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn assign(&mut self, id: &str, base: &str, taken: &BTreeSet<u16>) -> Option<Assigned> {
        self.assign_range(id, base, taken, SAVE_FIRST, SAVE_LAST)
    }

    pub fn assign_range(
        &mut self,
        id: &str,
        base: &str,
        taken: &BTreeSet<u16>,
        first: u16,
        last: u16,
    ) -> Option<Assigned> {
        let reserved: BTreeSet<u16> = self
            .entries
            .iter()
            .filter(|e| e.id != id)
            .map(|e| e.save_no)
            .collect();
        let previous = self.find(id).map(|e| e.save_no);
        if let Some(save_no) = previous {
            if !taken.contains(&save_no) {
                if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
                    entry.base = base.to_string();
                }
                return Some(Assigned::Kept(save_no));
            }
        }
        let free = (first..=last).find(|n| !taken.contains(n) && !reserved.contains(n))?;
        self.entries.retain(|e| e.id != id);
        self.entries.push(Entry {
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

fn list_mut<'a>(root: &'a mut Param, name: &str) -> Option<&'a mut Vec<Param>> {
    root.get_mut(hash40(name))?.list_mut()
}

fn list<'a>(root: &'a Param, name: &str) -> Option<&'a Vec<Param>> {
    root.get(hash40(name))?.list()
}

fn hash_of(row: &Param, field: &str) -> Option<u64> {
    row.get(hash40(field)).and_then(Param::hash).map(|h| h & HASH_MASK)
}

fn number_of(row: &Param, field: &str) -> Option<i64> {
    row.get(hash40(field)).and_then(Param::integer)
}

pub fn taken_save_numbers(root: &Param) -> BTreeSet<u16> {
    let mut taken = BTreeSet::new();
    for row in list(root, "db_root").map(Vec::as_slice).unwrap_or_default() {
        if let Some(save_no) = number_of(row, "save_no").and_then(|n| u16::try_from(n).ok()) {
            taken.insert(save_no);
        }
        if let Some(number) = number_of(row, "directory_id").and_then(|n| u16::try_from(n).ok()) {
            if number > 0 {
                taken.insert(number - 1);
            }
        }
    }
    taken
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Added { id: String, save_no: u16, assigned: Assigned, left_out: Vec<String> },
    UnknownBase { id: String, base: String },
    TakenId { id: String },
    Full { id: String },
}

fn left_out(row: &mut Param, fields: &[&Field], prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    for field in fields {
        if let Err(refusal) = apply(row, field) {
            out.push(format!("{prefix}{} ({})", field.name, refusal.text()));
        }
    }
    out
}

pub fn build_spirit(base: &Param, declaration: &Declaration, save_no: u16) -> (Param, Vec<String>) {
    let mut row = base.clone();
    row.set(hash40("ui_spirit_id"), Param::Hash(declaration.id_hash()));
    row.set(hash40("name_id"), Param::Str(declaration.id.clone()));
    set_keep_type(&mut row, hash40("save_no"), i64::from(save_no));
    set_keep_type(&mut row, hash40("directory_id"), i64::from(save_no) + 1);
    set_keep_type(&mut row, hash40("fixed_no"), i64::from(FIXED_BASE) + i64::from(save_no));
    for (field, value) in [("is_dlc", false), ("is_bcat", false)] {
        if row.get(hash40(field)).is_some() {
            row.set(hash40(field), Param::Bool(value));
        }
    }
    for field in ["replace_battle_id", "check_id"] {
        if row.get(hash40(field)).is_some() {
            row.set(hash40(field), Param::Hash(0));
        }
    }
    let fields: Vec<&Field> = declaration.fields_of(Table::Spirit).collect();
    let refused = left_out(&mut row, &fields, "");
    (row, refused)
}

pub fn append_spirits(root: &mut Param, declarations: &[&Declaration], registry: &mut Registry) -> Vec<Outcome> {
    let mut outcomes = Vec::new();
    let mut taken = taken_save_numbers(root);
    for declaration in declarations {
        let id = declaration.id.clone();
        let rows = list(root, "db_root").map(Vec::as_slice).unwrap_or_default();
        if rows.iter().any(|row| hash_of(row, "ui_spirit_id") == Some(declaration.id_hash())) {
            outcomes.push(Outcome::TakenId { id });
            continue;
        }
        let Some(base) = rows
            .iter()
            .find(|row| hash_of(row, "ui_spirit_id") == Some(declaration.base_hash()))
            .cloned()
        else {
            outcomes.push(Outcome::UnknownBase {
                id,
                base: declaration.base.clone(),
            });
            continue;
        };
        let Some(assigned) = registry.assign(&id, &declaration.base, &taken) else {
            outcomes.push(Outcome::Full { id });
            continue;
        };
        let save_no = assigned.save_no();
        let (row, refused) = build_spirit(&base, declaration, save_no);
        if let Some(rows) = list_mut(root, "db_root") {
            rows.push(row);
        }
        taken.insert(save_no);
        outcomes.push(Outcome::Added {
            id,
            save_no,
            assigned,
            left_out: refused,
        });
    }
    outcomes
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleOutcome {
    Added { id: String, fighters: usize, left_out: Vec<String> },
    UnknownBase { id: String, base: String },
    TakenId { id: String },
}

pub fn append_battles(root: &mut Param, declarations: &[&Declaration]) -> Vec<BattleOutcome> {
    let mut outcomes = Vec::new();
    for declaration in declarations {
        let id = declaration.id.clone();
        let battles = list(root, "battle_data_tbl").map(Vec::as_slice).unwrap_or_default();
        if battles.iter().any(|row| hash_of(row, "battle_id") == Some(declaration.id_hash())) {
            outcomes.push(BattleOutcome::TakenId { id });
            continue;
        }
        let base_battle = battles
            .iter()
            .find(|row| hash_of(row, "battle_id") == Some(declaration.base_hash()))
            .cloned();
        let base_fighters: Vec<Param> = list(root, "fighter_data_tbl")
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter(|row| hash_of(row, "battle_id") == Some(declaration.base_hash()))
            .cloned()
            .collect();
        let (Some(mut battle), false) = (base_battle, base_fighters.is_empty()) else {
            outcomes.push(BattleOutcome::UnknownBase {
                id,
                base: declaration.base.clone(),
            });
            continue;
        };
        battle.set(hash40("battle_id"), Param::Hash(declaration.id_hash()));
        let fields: Vec<&Field> = declaration.fields_of(Table::Battle).collect();
        let mut refused = left_out(&mut battle, &fields, "battle.");
        let count = declaration.fighters.unwrap_or(base_fighters.len());
        let mut fighters = Vec::with_capacity(count);
        for index in 0..count {
            let mut fighter = base_fighters[index.min(base_fighters.len() - 1)].clone();
            fighter.set(hash40("battle_id"), Param::Hash(declaration.id_hash()));
            if fighter.get(hash40("spirit_name")).is_some() {
                fighter.set(hash40("spirit_name"), Param::Hash(declaration.id_hash()));
            }
            let fields: Vec<&Field> = declaration
                .fields_of(Table::Fighter)
                .filter(|field| field.fighter == index)
                .collect();
            refused.extend(left_out(&mut fighter, &fields, &format!("fighter.{index}.")));
            fighters.push(fighter);
        }
        let unused: Vec<String> = declaration
            .fields_of(Table::Fighter)
            .filter(|field| field.fighter >= count)
            .map(|field| format!("fighter.{}.{} (past the {count} fighter(s))", field.fighter, field.name))
            .collect();
        refused.extend(unused);
        if let Some(rows) = list_mut(root, "battle_data_tbl") {
            rows.push(battle);
        }
        if let Some(rows) = list_mut(root, "fighter_data_tbl") {
            rows.extend(fighters);
        }
        outcomes.push(BattleOutcome::Added {
            id,
            fighters: count,
            left_out: refused,
        });
    }
    outcomes
}

pub fn append_layouts(root: &mut Param, declarations: &[&Declaration]) -> Vec<BattleOutcome> {
    let mut outcomes = Vec::new();
    for declaration in declarations {
        let id = declaration.id.clone();
        let rows = list(root, "db_root").map(Vec::as_slice).unwrap_or_default();
        if rows.iter().any(|row| hash_of(row, "ui_spirit_layout_id") == Some(declaration.id_hash())) {
            outcomes.push(BattleOutcome::TakenId { id });
            continue;
        }
        let Some(mut layout) = rows
            .iter()
            .find(|row| hash_of(row, "ui_spirit_layout_id") == Some(declaration.base_hash()))
            .cloned()
        else {
            outcomes.push(BattleOutcome::UnknownBase {
                id,
                base: declaration.base.clone(),
            });
            continue;
        };
        layout.set(hash40("ui_spirit_layout_id"), Param::Hash(declaration.id_hash()));
        let fields: Vec<&Field> = declaration.fields_of(Table::Layout).collect();
        let refused = left_out(&mut layout, &fields, "layout.");
        if let Some(rows) = list_mut(root, "db_root") {
            rows.push(layout);
        }
        outcomes.push(BattleOutcome::Added {
            id,
            fighters: 0,
            left_out: refused,
        });
    }
    outcomes
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardOutcome {
    Added {
        id: String,
        save_no: u16,
        assigned: Assigned,
        spirits: usize,
        unknown_spirits: Vec<String>,
        unknown_background: bool,
        left_out: Vec<String>,
    },
    UnknownBase { id: String, base: String },
    NoSpirits { id: String },
    TakenId { id: String },
    Full { id: String },
}

const BOARD_LIST: &str = "special_board_data_list";

fn board_rows(root: &Param) -> &[Param] {
    list(root, BOARD_LIST).map(Vec::as_slice).unwrap_or_default()
}

fn board_by_label(root: &Param, label: u64) -> Option<Param> {
    board_rows(root)
        .iter()
        .find(|row| hash_of(row, "label") == Some(label & HASH_MASK))
        .cloned()
}

pub fn taken_board_numbers(root: &Param) -> BTreeSet<u16> {
    board_rows(root)
        .iter()
        .filter_map(|row| number_of(row, "save_no").and_then(|n| u16::try_from(n).ok()))
        .collect()
}

fn spirit_list(template: &Param, spirits: &[u64]) -> Param {
    let entry = template
        .list()
        .and_then(|entries| entries.first())
        .cloned()
        .unwrap_or_else(|| Param::Struct(vec![(hash40("spirit_id"), Param::Hash(0))]));
    Param::List(
        spirits
            .iter()
            .map(|spirit| {
                let mut row = entry.clone();
                row.set(hash40("spirit_id"), Param::Hash(*spirit));
                row
            })
            .collect(),
    )
}

pub fn append_boards(
    root: &mut Param,
    boards: &[&Board],
    registry: &mut Registry,
    known_spirits: Option<&BTreeSet<u64>>,
) -> Vec<BoardOutcome> {
    let mut outcomes = Vec::new();
    let mut taken = taken_board_numbers(root);
    for board in boards {
        let id = board.id.clone();
        if board_by_label(root, board.label()).is_some() {
            outcomes.push(BoardOutcome::TakenId { id });
            continue;
        }
        let Some(mut row) = board_by_label(root, board.base_label()) else {
            outcomes.push(BoardOutcome::UnknownBase {
                id,
                base: board.base.clone(),
            });
            continue;
        };
        let mut unknown_spirits = Vec::new();
        let mut spirits = Vec::new();
        for name in &board.spirits {
            let spirit = name_key(name);
            if known_spirits.is_some_and(|known| !known.contains(&spirit)) {
                unknown_spirits.push(name.clone());
                continue;
            }
            if !spirits.contains(&spirit) {
                spirits.push(spirit);
            }
        }
        if spirits.is_empty() {
            outcomes.push(BoardOutcome::NoSpirits { id });
            continue;
        }
        let Some(assigned) = registry.assign_range(&id, &board.base, &taken, BOARD_SAVE_FIRST, BOARD_SAVE_LAST)
        else {
            outcomes.push(BoardOutcome::Full { id });
            continue;
        };
        let save_no = assigned.save_no();
        let mut unknown_background = false;
        if let Some(background) = &board.background {
            match board_by_label(root, fighter_label(background)).and_then(|other| other.get(hash40("layout_no")).cloned())
            {
                Some(layout) => {
                    row.set(hash40("layout_no"), layout);
                }
                None => unknown_background = true,
            }
        }
        row.set(hash40("label"), Param::Hash(board.label()));
        set_keep_type(&mut row, hash40("save_no"), i64::from(save_no));
        if row.get(hash40("shop_item_tag")).is_some() {
            row.set(hash40("shop_item_tag"), Param::Hash(0));
        }
        let mut left_out = Vec::new();
        for field in &board.fields {
            if let Err(refusal) = apply(&mut row, field) {
                left_out.push(format!("{} ({})", field.name, refusal.text()));
            }
        }
        let template = row.get(hash40("spirit_list")).cloned().unwrap_or(Param::List(Vec::new()));
        row.set(hash40("spirit_list"), spirit_list(&template, &spirits));
        if let Some(rows) = list_mut(root, BOARD_LIST) {
            rows.push(row);
        }
        taken.insert(save_no);
        outcomes.push(BoardOutcome::Added {
            id,
            save_no,
            assigned,
            spirits: spirits.len(),
            unknown_spirits,
            unknown_background,
            left_out,
        });
    }
    outcomes
}

pub fn spirit_ids(root: &Param) -> BTreeSet<u64> {
    list(root, "db_root")
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|row| hash_of(row, "ui_spirit_id"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spirit_row(id: &str, save_no: u16) -> Param {
        Param::Struct(vec![
            (hash40("ui_spirit_id"), Param::Hash(hash40(id))),
            (hash40("name_id"), Param::Str(id.to_string())),
            (hash40("save_no"), Param::U16(save_no)),
            (hash40("directory_id"), Param::U16(save_no + 1)),
            (hash40("fixed_no"), Param::U16(7845)),
            (hash40("rank"), Param::Hash(hash40("normal"))),
            (hash40("is_dlc"), Param::Bool(true)),
            (hash40("check_id"), Param::Hash(hash40("creeper"))),
            (hash40("base_attack"), Param::I16(42)),
            (hash40("exp_up_rate"), Param::Float(1.03)),
            (hash40("is_board_appear"), Param::Bool(true)),
        ])
    }

    fn database() -> Param {
        Param::Struct(vec![(
            hash40("db_root"),
            Param::List(vec![spirit_row("mario", 0), spirit_row("sidestepper", 8)]),
        )])
    }

    fn battles() -> Param {
        let battle = |id: &str| {
            Param::Struct(vec![
                (hash40("battle_id"), Param::Hash(hash40(id))),
                (hash40("ui_stage_id"), Param::Hash(hash40("ui_stage_mariobros"))),
                (hash40("basic_stock"), Param::U8(1)),
            ])
        };
        let fighter = |id: &str, kind: &str| {
            Param::Struct(vec![
                (hash40("battle_id"), Param::Hash(hash40(id))),
                (hash40("spirit_name"), Param::Hash(hash40(id))),
                (hash40("fighter_kind"), Param::Hash(hash40(kind))),
                (hash40("scale"), Param::Float(0.9)),
            ])
        };
        Param::Struct(vec![
            (hash40("battle_data_tbl"), Param::List(vec![battle("mario"), battle("sidestepper")])),
            (
                hash40("fighter_data_tbl"),
                Param::List(vec![
                    fighter("mario", "ui_chara_mario"),
                    fighter("sidestepper", "ui_chara_gamewatch"),
                    fighter("sidestepper", "ui_chara_gamewatch"),
                ]),
            ),
        ])
    }

    const TOML: &str = r#"
[[spirit]]
id = "wawa"
base = "sidestepper"   # a comment
rank = "hope"
base_attack = 120
exp_up_rate = 1.5
battle.ui_stage_id = "ui_stage_battle_field"
fighter.0.fighter_kind = "ui_chara_mario"
fighter.2.scale = 2
fighters = 3
"#;

    #[test]
    fn a_spirit_toml_reads_its_fields_by_table() {
        let parsed = parse(TOML).unwrap();
        assert_eq!(parsed.len(), 1);
        let wawa = &parsed[0];
        assert_eq!((wawa.id.as_str(), wawa.base.as_str(), wawa.fighters), ("wawa", "sidestepper", Some(3)));
        assert_eq!(wawa.fields_of(Table::Spirit).count(), 3);
        assert_eq!(wawa.fields_of(Table::Battle).count(), 1);
        let fighters: Vec<usize> = wawa.fields_of(Table::Fighter).map(|f| f.fighter).collect();
        assert_eq!(fighters, vec![0, 2]);
        assert!(matches!(wawa.fields_of(Table::Spirit).nth(2).unwrap().value, Value::Float(_)));
    }

    #[test]
    fn fields_the_engine_sets_are_refused() {
        for key in ["save_no", "name_id", "battle.battle_id", "layout.ui_spirit_layout_id", "fighter.0.battle_id"] {
            let text = format!("[[spirit]]\nid = \"wawa\"\nbase = \"mario\"\n{key} = 3\n");
            assert!(matches!(parse(&text), Err(SpiritError::Fixed { .. })), "{key}");
        }
        assert!(matches!(
            parse("[[spirit]]\nid = \"Wawa\"\nbase = \"mario\"\n"),
            Err(SpiritError::BadId { .. })
        ));
        assert!(matches!(
            parse("[[spirit]]\nid = \"wawa\"\nbase = \"mario\"\nfighter.99.scale = 1\n"),
            Err(SpiritError::BadKey { .. })
        ));
        assert!(matches!(parse("[[spirit]]\nbase = \"mario\"\n"), Err(SpiritError::Missing { key: "id", .. })));
        assert!(matches!(
            parse("[[spirit]]\nid = \"a\"\nbase = \"mario\"\n[[spirit]]\nid = \"a\"\nbase = \"mario\"\n"),
            Err(SpiritError::Duplicate { .. })
        ));
    }

    #[test]
    fn a_spirit_copies_its_base_and_takes_a_number_past_the_event_spirits() {
        let declarations = parse(TOML).unwrap();
        let refs: Vec<&Declaration> = declarations.iter().collect();
        let mut root = database();
        let mut registry = Registry::default();
        let outcomes = append_spirits(&mut root, &refs, &mut registry);
        assert_eq!(
            outcomes,
            vec![Outcome::Added {
                id: "wawa".into(),
                save_no: SAVE_FIRST,
                assigned: Assigned::New(SAVE_FIRST),
                left_out: Vec::new()
            }]
        );
        let row = &list(&root, "db_root").unwrap()[2];
        assert_eq!(hash_of(row, "ui_spirit_id"), Some(hash40("wawa")));
        assert_eq!(row.get(hash40("name_id")), Some(&Param::Str("wawa".into())));
        assert_eq!(row.get(hash40("save_no")), Some(&Param::U16(SAVE_FIRST)));
        assert_eq!(row.get(hash40("directory_id")), Some(&Param::U16(SAVE_FIRST + 1)));
        assert_eq!(row.get(hash40("fixed_no")), Some(&Param::U16(FIXED_BASE + SAVE_FIRST)));
        assert_eq!(row.get(hash40("rank")), Some(&Param::Hash(hash40("hope"))));
        assert_eq!(row.get(hash40("base_attack")), Some(&Param::I16(120)));
        assert_eq!(row.get(hash40("exp_up_rate")), Some(&Param::Float(1.5)));
        assert_eq!(row.get(hash40("is_dlc")), Some(&Param::Bool(false)));
        assert_eq!(row.get(hash40("check_id")), Some(&Param::Hash(0)));
        assert_eq!(row.get(hash40("is_board_appear")), Some(&Param::Bool(true)));
        let again = append_spirits(&mut database(), &refs, &mut registry);
        assert!(matches!(again[0], Outcome::Added { assigned: Assigned::Kept(SAVE_FIRST), .. }));
    }

    #[test]
    fn a_number_the_game_starts_using_moves_the_spirit() {
        let declarations = parse(TOML).unwrap();
        let refs: Vec<&Declaration> = declarations.iter().collect();
        let mut registry = Registry::parse("1513 wawa sidestepper\n1514 gone mario\n");
        let mut root = database();
        if let Some(rows) = list_mut(&mut root, "db_root") {
            rows.push(spirit_row("nintendo_new", SAVE_FIRST));
        }
        let outcomes = append_spirits(&mut root, &refs, &mut registry);
        assert!(matches!(
            outcomes[0],
            Outcome::Added { assigned: Assigned::Moved { from: 1513, to: 1515 }, .. }
        ));
        assert!(registry.serialise().contains("1514 gone mario\n1515 wawa sidestepper\n"));
    }

    #[test]
    fn unknown_bases_taken_ids_and_bad_fields_are_reported() {
        let mut declarations = parse(TOML).unwrap();
        declarations.extend(parse("[[spirit]]\nid = \"mario\"\nbase = \"sidestepper\"\n").unwrap());
        declarations.extend(parse("[[spirit]]\nid = \"lost\"\nbase = \"nobody\"\n").unwrap());
        declarations.extend(
            parse("[[spirit]]\nid = \"odd\"\nbase = \"mario\"\nno_such = 1\nbase_attack = 99999\nrank = 4\n").unwrap(),
        );
        let refs: Vec<&Declaration> = declarations.iter().collect();
        let mut root = database();
        let outcomes = append_spirits(&mut root, &refs, &mut Registry::default());
        assert!(matches!(outcomes[1], Outcome::TakenId { .. }));
        assert!(matches!(outcomes[2], Outcome::UnknownBase { .. }));
        let Outcome::Added { left_out, save_no, .. } = &outcomes[3] else {
            panic!("{:?}", outcomes[3]);
        };
        assert_eq!(*save_no, SAVE_FIRST + 1);
        assert_eq!(
            left_out,
            &vec![
                "no_such (not a field of the base's row)".to_string(),
                "base_attack (out of range)".to_string()
            ]
        );
    }

    #[test]
    fn the_battle_copies_the_bases_fighters_and_takes_overrides() {
        let declarations = parse(TOML).unwrap();
        let refs: Vec<&Declaration> = declarations.iter().collect();
        let mut root = battles();
        let outcomes = append_battles(&mut root, &refs);
        assert_eq!(
            outcomes,
            vec![BattleOutcome::Added {
                id: "wawa".into(),
                fighters: 3,
                left_out: Vec::new()
            }]
        );
        let battle = &list(&root, "battle_data_tbl").unwrap()[2];
        assert_eq!(hash_of(battle, "battle_id"), Some(hash40("wawa")));
        assert_eq!(hash_of(battle, "ui_stage_id"), Some(hash40("ui_stage_battle_field")));
        let fighters: Vec<&Param> = list(&root, "fighter_data_tbl")
            .unwrap()
            .iter()
            .filter(|row| hash_of(row, "battle_id") == Some(hash40("wawa")))
            .collect();
        assert_eq!(fighters.len(), 3);
        assert_eq!(hash_of(fighters[0], "fighter_kind"), Some(hash40("ui_chara_mario")));
        assert_eq!(hash_of(fighters[1], "fighter_kind"), Some(hash40("ui_chara_gamewatch")));
        assert_eq!(hash_of(fighters[1], "spirit_name"), Some(hash40("wawa")));
        assert_eq!(fighters[2].get(hash40("scale")), Some(&Param::Float(2.0)));
        let again = append_battles(&mut root, &refs);
        assert!(matches!(again[0], BattleOutcome::TakenId { .. }));
    }

    #[test]
    fn the_layout_copies_the_bases() {
        let declarations = parse("[[spirit]]\nid = \"wawa\"\nbase = \"mario\"\nlayout.ui_art_scale = 1.25\n").unwrap();
        let refs: Vec<&Declaration> = declarations.iter().collect();
        let mut root = Param::Struct(vec![(
            hash40("db_root"),
            Param::List(vec![Param::Struct(vec![
                (hash40("ui_spirit_layout_id"), Param::Hash(hash40("mario"))),
                (hash40("ui_art_scale"), Param::Float(1.0)),
            ])]),
        )]);
        let outcomes = append_layouts(&mut root, &refs);
        assert!(matches!(outcomes[0], BattleOutcome::Added { .. }));
        let row = &list(&root, "db_root").unwrap()[1];
        assert_eq!(hash_of(row, "ui_spirit_layout_id"), Some(hash40("wawa")));
        assert_eq!(row.get(hash40("ui_art_scale")), Some(&Param::Float(1.25)));
    }

    #[test]
    fn the_registry_keeps_numbers_in_range_and_never_twice() {
        let registry = Registry::parse("# header\n1513 wawa sidestepper\n1513 other mario\n12 low mario\n2000 high mario\n1600 Bad mario\n");
        assert_eq!(registry.entries.len(), 1);
        assert_eq!(Registry::parse(&registry.serialise()), registry);
    }

    #[test]
    fn every_free_number_is_used_before_the_table_is_full() {
        let mut registry = Registry::default();
        let taken = BTreeSet::new();
        for index in 0..MAX_SPIRITS {
            assert!(registry.assign(&format!("s{index}"), "mario", &taken).is_some());
        }
        assert_eq!(registry.assign("one_more", "mario", &taken), None);
        assert_eq!(registry.entries.last().unwrap().save_no, SAVE_LAST);
    }

    fn board_row(label: &str, save_no: u32, layout_no: u32, spirits: &[&str]) -> Param {
        Param::Struct(vec![
            (hash40("label"), Param::Hash(hash40(label))),
            (hash40("value"), Param::U32(10000)),
            (hash40("bgm_id"), Param::Hash(hash40("ui_bgm_x"))),
            (hash40("save_no"), Param::U32(save_no)),
            (hash40("prize_id"), Param::Hash(hash40("ui_prize_gold_direct"))),
            (hash40("layout_no"), Param::U32(layout_no)),
            (
                hash40("spirit_list"),
                Param::List(
                    spirits
                        .iter()
                        .map(|s| Param::Struct(vec![(hash40("spirit_id"), Param::Hash(hash40(s)))]))
                        .collect(),
                ),
            ),
            (hash40("shop_item_tag"), Param::Hash(hash40("sc-12"))),
        ])
    }

    fn boards() -> Param {
        Param::Struct(vec![(
            hash40("special_board_data_list"),
            Param::List(vec![
                board_row("ui_chara_jack", 0, 0, &["jack_morgana"]),
                board_row("ui_chara_trail", 10, 10, &["trail_kairi", "trail_riku"]),
            ]),
        )])
    }

    const BOARD_TOML: &str = r#"
[[spirit]]
id = "wawa"
base = "sidestepper"

[[board]]
id = "wawa_board"
base = "trail"
name = "Wawa's Board"
background = "JACK"
spirits = ["wawa", "geno", "nobody"]
bgm_id = "ui_bgm_mine"
value = 5000
prize_id = "ui_prize_gold_direct"
shop_item_tag = "sb-99"
layout_no = 12
not_a_field = 3
art = "ui/layout/menu/my_boards/spb_wawa_board/layout.arc"
"#;

    #[test]
    fn a_manifest_holds_spirits_and_boards() {
        let manifest = parse_manifest(BOARD_TOML).unwrap();
        assert_eq!(manifest.spirits.len(), 1);
        let board = &manifest.boards[0];
        assert_eq!(board.display_name(), "Wawa's Board");
        assert_eq!(board.spirits, vec!["wawa", "geno", "nobody"]);
        assert_eq!(board.label(), hash40("ui_chara_wawa_board"));
        assert_eq!(board.base_label(), hash40("ui_chara_trail"));
        assert_eq!(fighter_label("JACK"), hash40("ui_chara_jack"));
        assert_eq!(parse(BOARD_TOML).unwrap().len(), 1);
        assert!(matches!(
            parse_manifest("[[board]]\nid = \"b\"\nbase = \"trail\"\n"),
            Err(SpiritError::Missing { key: "spirits", .. })
        ));
        assert!(matches!(
            parse_manifest("[[board]]\nid = \"b\"\nbase = \"trail\"\nspirits = [\"a\"]\nla.yout = 3\n"),
            Err(SpiritError::BadKey { .. })
        ));
        for fixed in ["label", "save_no", "spirit_list"] {
            assert!(matches!(
                parse_manifest(&format!("[[board]]\nid = \"b\"\nbase = \"trail\"\nspirits = [\"a\"]\n{fixed} = 3\n")),
                Err(SpiritError::Fixed { .. })
            ));
        }
        assert!(matches!(
            parse_manifest("[[board]]\nid = \"b\"\nbase = \"trail\"\nspirits = [\"a\"]\nart = \"ui/x.arc\"\n"),
            Err(SpiritError::BadValue { .. })
        ));
        assert_eq!(board.art_root().as_deref(), Some("spb_wawa_board"));
        assert_eq!(board.fields.len(), 6);
    }

    #[test]
    fn a_board_copies_its_base_with_its_own_spirits_number_and_background() {
        let manifest = parse_manifest(BOARD_TOML).unwrap();
        let refs: Vec<&Board> = manifest.boards.iter().collect();
        let mut root = boards();
        let mut registry = Registry::default();
        let known: BTreeSet<u64> = ["wawa", "geno"].iter().map(|s| hash40(s)).collect();
        let outcomes = append_boards(&mut root, &refs, &mut registry, Some(&known));
        assert_eq!(
            outcomes,
            vec![BoardOutcome::Added {
                id: "wawa_board".into(),
                save_no: BOARD_SAVE_FIRST,
                assigned: Assigned::New(BOARD_SAVE_FIRST),
                spirits: 2,
                unknown_spirits: vec!["nobody".into()],
                unknown_background: false,
                left_out: vec!["not_a_field (not a field of the base's row)".into()],
            }]
        );
        let row = &list(&root, "special_board_data_list").unwrap()[2];
        assert_eq!(hash_of(row, "label"), Some(hash40("ui_chara_wawa_board")));
        assert_eq!(row.get(hash40("save_no")), Some(&Param::U32(u32::from(BOARD_SAVE_FIRST))));
        assert_eq!(row.get(hash40("layout_no")), Some(&Param::U32(12)));
        assert_eq!(row.get(hash40("value")), Some(&Param::U32(5000)));
        assert_eq!(hash_of(row, "bgm_id"), Some(hash40("ui_bgm_mine")));
        assert_eq!(hash_of(row, "prize_id"), Some(hash40("ui_prize_gold_direct")));
        assert_eq!(hash_of(row, "shop_item_tag"), Some(hash40("sb-99")));
        let ids: Vec<u64> = row
            .get(hash40("spirit_list"))
            .and_then(Param::list)
            .unwrap()
            .iter()
            .filter_map(|entry| hash_of(entry, "spirit_id"))
            .collect();
        assert_eq!(ids, vec![hash40("wawa"), hash40("geno")]);
        assert!(registry.serialise_boards().contains("11 wawa_board trail\n"));
        let again = append_boards(&mut boards(), &refs, &mut registry, None);
        assert!(matches!(again[0], BoardOutcome::Added { assigned: Assigned::Kept(11), spirits: 3, .. }));
    }

    #[test]
    fn boards_stop_at_the_saves_last_board_number() {
        let mut root = boards();
        let mut registry = Registry::default();
        let mut declared = Vec::new();
        for index in 0..=MAX_BOARDS {
            declared.push(Board {
                id: format!("b{index}"),
                base: "jack".into(),
                name: String::new(),
                background: Some("nobody".into()),
                spirits: vec!["geno".into()],
                art: None,
                fields: Vec::new(),
                line: 1,
            });
        }
        let refs: Vec<&Board> = declared.iter().collect();
        let outcomes = append_boards(&mut root, &refs, &mut registry, None);
        assert!(matches!(outcomes[0], BoardOutcome::Added { unknown_background: true, .. }));
        assert!(matches!(outcomes[MAX_BOARDS - 1], BoardOutcome::Added { save_no: BOARD_SAVE_LAST, .. }));
        assert!(matches!(outcomes[MAX_BOARDS], BoardOutcome::Full { .. }));
        let unknown = append_boards(
            &mut boards(),
            &[&Board {
                base: "kirby".into(),
                ..declared[0].clone()
            }],
            &mut Registry::default(),
            None,
        );
        assert!(matches!(unknown[0], BoardOutcome::UnknownBase { .. }));
    }
}
