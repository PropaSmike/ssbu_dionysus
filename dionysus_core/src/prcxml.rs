use crate::fighter_common_copies;
use crate::fighter_param_row::{field_in, Field, FieldType, Table};
use crate::fighter_toml::ParamValue;
use crate::hash::hash40;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Hash(u64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Value(Value),
    List(Vec<(usize, Node)>),
    Struct(Vec<(u64, Node)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

pub fn key(text: &str) -> u64 {
    let text = text.trim();
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16).unwrap_or_else(|_| hash40(text)),
        None => hash40(text),
    }
}

pub fn is_dummy(node: &Node) -> bool {
    matches!(node, Node::Value(Value::Hash(hash)) if *hash == hash40("dummy"))
}

struct Tag {
    name: String,
    attributes: Vec<(String, String)>,
    empty: bool,
}

impl Tag {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(known, _)| known == name)
            .map(|(_, value)| value.as_str())
    }
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    fn fail<T>(&self, message: impl Into<String>) -> Result<T, Error> {
        Err(Error {
            line: self.text[..self.at].matches('\n').count() + 1,
            message: message.into(),
        })
    }

    fn skip_space(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    fn skip_past(&mut self, end: &str, what: &str) -> Result<(), Error> {
        match self.rest().find(end) {
            Some(index) => {
                self.at += index + end.len();
                Ok(())
            }
            None => self.fail(format!("{what} never ends")),
        }
    }

    fn skip_misc(&mut self) -> Result<(), Error> {
        loop {
            self.skip_space();
            let rest = self.rest();
            if rest.starts_with("<?") {
                self.skip_past("?>", "a <? declaration")?;
            } else if rest.starts_with("<!--") {
                self.skip_past("-->", "a comment")?;
            } else if rest.starts_with("<!") {
                self.skip_past(">", "a <! declaration")?;
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> String {
        let rest = self.rest();
        let length = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':'))
            .unwrap_or(rest.len());
        self.at += length;
        rest[..length].to_string()
    }

    fn open(&mut self) -> Result<Tag, Error> {
        if !self.rest().starts_with('<') || self.rest().starts_with("</") {
            return self.fail("expected an element");
        }
        self.at += 1;
        let name = self.name();
        if name.is_empty() {
            return self.fail("an element has no name");
        }
        let mut attributes = Vec::new();
        loop {
            self.skip_space();
            let rest = self.rest();
            if rest.starts_with("/>") {
                self.at += 2;
                return Ok(Tag { name, attributes, empty: true });
            }
            if rest.starts_with('>') {
                self.at += 1;
                return Ok(Tag { name, attributes, empty: false });
            }
            let attribute = self.name();
            if attribute.is_empty() {
                return self.fail(format!("<{name}> is not closed"));
            }
            self.skip_space();
            if !self.rest().starts_with('=') {
                return self.fail(format!("{attribute} in <{name}> has no value"));
            }
            self.at += 1;
            self.skip_space();
            let Some(quote) = self.rest().chars().next().filter(|c| *c == '"' || *c == '\'') else {
                return self.fail(format!("{attribute} in <{name}> is not quoted"));
            };
            self.at += 1;
            let Some(length) = self.rest().find(quote) else {
                return self.fail(format!("{attribute} in <{name}> is not closed"));
            };
            attributes.push((attribute, unescape(&self.rest()[..length])));
            self.at += length + 1;
        }
    }

    fn close(&mut self, name: &str) -> Result<(), Error> {
        self.skip_misc()?;
        let expected = format!("</{name}");
        if !self.rest().starts_with(&expected) {
            return self.fail(format!("<{name}> is not closed"));
        }
        self.at += expected.len();
        self.skip_space();
        if !self.rest().starts_with('>') {
            return self.fail(format!("</{name}> is not closed"));
        }
        self.at += 1;
        Ok(())
    }

    fn closing_next(&mut self) -> Result<bool, Error> {
        self.skip_misc()?;
        if self.rest().is_empty() {
            return self.fail("the file ends inside an element");
        }
        Ok(self.rest().starts_with("</"))
    }

    fn text_until_close(&mut self, name: &str) -> Result<String, Error> {
        let Some(length) = self.rest().find('<') else {
            return self.fail(format!("<{name}> is not closed"));
        };
        let text = unescape(self.rest()[..length].trim());
        self.at += length;
        self.close(name)?;
        Ok(text)
    }

    fn element(&mut self) -> Result<(Tag, Node), Error> {
        let tag = self.open()?;
        let node = match tag.name.as_str() {
            "struct" => {
                let mut members = Vec::new();
                if !tag.empty {
                    while !self.closing_next()? {
                        let (child, node) = self.element()?;
                        let Some(hash) = child.attribute("hash") else {
                            return self.fail(format!("a <{}> inside a struct has no hash", child.name));
                        };
                        members.push((key(hash), node));
                    }
                    self.close("struct")?;
                }
                Node::Struct(members)
            }
            "list" => {
                let mut entries = Vec::new();
                if !tag.empty {
                    while !self.closing_next()? {
                        let (child, node) = self.element()?;
                        let index = match child.attribute("index") {
                            Some(index) => match index.trim().parse::<usize>() {
                                Ok(index) => index,
                                Err(_) => return self.fail(format!("index {index:?} is not a number")),
                            },
                            None => entries.len(),
                        };
                        entries.push((index, node));
                    }
                    self.close("list")?;
                }
                Node::List(entries)
            }
            scalar => {
                let text = if tag.empty { String::new() } else { self.text_until_close(scalar)? };
                Node::Value(match self.scalar(scalar, &text) {
                    Some(value) => value,
                    None => return self.fail(format!("<{scalar}> cannot hold {text:?}")),
                })
            }
        };
        Ok((tag, node))
    }

    fn scalar(&self, tag: &str, text: &str) -> Option<Value> {
        let integer = |text: &str| -> Option<i64> {
            let (negative, digits) = match text.strip_prefix('-') {
                Some(digits) => (true, digits),
                None => (false, text),
            };
            let value = match digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")) {
                Some(hex) => i64::from_str_radix(hex, 16).ok()?,
                None => digits.parse::<i64>().ok()?,
            };
            Some(if negative { -value } else { value })
        };
        match tag {
            "bool" => match text.to_ascii_lowercase().as_str() {
                "true" | "1" => Some(Value::Bool(true)),
                "false" | "0" => Some(Value::Bool(false)),
                _ => None,
            },
            "sbyte" | "byte" | "short" | "ushort" | "int" | "uint" => integer(text).map(Value::Int),
            "float" => text.parse::<f64>().ok().filter(|value| value.is_finite()).map(Value::Float),
            "hash40" if !text.is_empty() => Some(Value::Hash(key(text))),
            "string" => Some(Value::Text(text.to_string())),
            _ => None,
        }
    }
}

pub fn parse(text: &str) -> Result<Node, Error> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = Reader { text, at: 0 };
    reader.skip_misc()?;
    let (tag, node) = reader.element()?;
    if tag.name != "struct" {
        return reader.fail("the file must start with <struct>");
    }
    reader.skip_misc()?;
    if !reader.rest().is_empty() {
        return reader.fail("text after the closing </struct>");
    }
    Ok(node)
}

pub fn member<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    let hash = hash40(name);
    match node {
        Node::Struct(members) => members.iter().find(|(known, _)| *known == hash).map(|(_, node)| node),
        _ => None,
    }
}

pub fn rows<'a>(root: &'a Node, list: &str) -> Vec<&'a [(u64, Node)]> {
    match member(root, list) {
        Some(Node::List(entries)) => entries
            .iter()
            .filter_map(|(_, node)| match node {
                Node::Struct(fields) => Some(fields.as_slice()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn top_level(root: &Node) -> &[(u64, Node)] {
    match root {
        Node::Struct(members) => members,
        _ => &[],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Unknown,
    Fixed,
    Container,
    Label,
    Hash,
    Text,
    Type,
    Range,
}

impl Why {
    pub const fn text(self) -> &'static str {
        match self {
            Why::Unknown => "not a field this file can set",
            Why::Fixed => "fixed by the game",
            Why::Container => "a list or struct",
            Why::Label => "a label",
            Why::Hash => "a hash40",
            Why::Text => "text",
            Why::Type => "the wrong type for this field",
            Why::Range => "outside the values the game takes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub field: String,
    pub why: Why,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FighterParams {
    pub params: Vec<(String, ParamValue)>,
    pub skipped: Vec<Skipped>,
    pub rows: usize,
}

pub const FIGHTER_FILES: [&str; 8] = [
    "fighter_param",
    "fighter_param_motion",
    "common",
    "item",
    "etc",
    "power_up",
    "effect",
    "sound",
];

const IDENTITY: [&str; 4] = ["fighter_kind", "name", "name_hash", "kind"];

fn shown(hash: u64, field: Option<&Field>) -> String {
    match field {
        Some(field) if !field.name.starts_with("unk_") => field.name.to_string(),
        _ => format!("{hash:#x}"),
    }
}

fn fighter_value(field: &Field, value: &Value) -> Result<ParamValue, Why> {
    match (field.kind, value) {
        (FieldType::Hash40, _) => Err(Why::Hash),
        (_, Value::Text(_)) => Err(Why::Text),
        (FieldType::F32, Value::Float(value)) => Ok(ParamValue::Float(*value)),
        (FieldType::F32, Value::Int(value)) => Ok(ParamValue::Float(*value as f64)),
        (FieldType::F32, Value::Bool(value)) => Ok(ParamValue::Float(f64::from(u8::from(*value)))),
        (_, Value::Hash(_)) => Err(Why::Label),
        (_, Value::Bool(value)) => Ok(ParamValue::Int(i32::from(*value))),
        (_, Value::Int(value)) => Ok(ParamValue::Int((*value).clamp(i32::MIN as i64, i32::MAX as i64) as i32)),
        (_, Value::Float(value)) => Ok(ParamValue::Int(value.round() as i32)),
    }
}

pub fn fighter_params(file: &str, root: &Node) -> Option<FighterParams> {
    let identity: Vec<u64> = IDENTITY.iter().map(|name| hash40(name)).collect();
    let (fields, table, prefix): (Vec<&[(u64, Node)]>, Option<Table>, &str) = match file {
        "fighter_param" => (rows(root, "fighter_param_table"), Some(Table::FighterParam), ""),
        "fighter_param_motion" => (rows(root, "fighter_param_motion_table"), Some(Table::Motion), "param_motion."),
        "common" | "item" | "etc" | "power_up" | "effect" | "sound" => (vec![top_level(root)], None, "common."),
        _ => return None,
    };
    let mut out = FighterParams {
        rows: fields.len(),
        ..Default::default()
    };
    let Some(row) = fields.first() else {
        return Some(out);
    };
    for (hash, node) in row.iter() {
        if identity.contains(hash) {
            continue;
        }
        let field = match table {
            Some(table) => field_in(table, *hash),
            None => fighter_common_copies::field(*hash),
        };
        let name = shown(*hash, field);
        let Node::Value(value) = node else {
            out.skipped.push(Skipped { field: name, why: Why::Container });
            continue;
        };
        let Some(field) = field else {
            out.skipped.push(Skipped { field: name, why: Why::Unknown });
            continue;
        };
        if table.is_some() && field.offset == 0 {
            out.skipped.push(Skipped { field: name, why: Why::Fixed });
            continue;
        }
        let key = if field.name.starts_with("unk_") {
            format!("{prefix}{:#x}", field.hash)
        } else {
            format!("{prefix}{}", field.name)
        };
        match fighter_value(field, value) {
            Ok(value) => out.params.push((key, value)),
            Err(why) => out.skipped.push(Skipped { field: name, why }),
        }
    }
    Some(out)
}

pub const VL_FILE: &str = "vl";

fn vl_value(value: &Value) -> Result<ParamValue, Why> {
    match value {
        Value::Float(value) => Ok(ParamValue::Float(*value)),
        Value::Int(value) => Ok(ParamValue::Int((*value).clamp(i32::MIN as i64, i32::MAX as i64) as i32)),
        Value::Bool(value) => Ok(ParamValue::Int(i32::from(*value))),
        Value::Hash(_) => Err(Why::Hash),
        Value::Text(_) => Err(Why::Text),
    }
}

pub fn vl_params(root: &Node) -> FighterParams {
    let mut out = FighterParams {
        rows: 1,
        ..Default::default()
    };
    for (group, node) in top_level(root) {
        let first_row = match node {
            Node::List(entries) => match entries.iter().map(|(_, entry)| entry).find(|entry| !is_dummy(entry)) {
                Some(row @ Node::Struct(_)) => Some(row),
                Some(_) => Some(node),
                None => None,
            },
            _ => Some(node),
        };
        match first_row {
            None => {}
            Some(Node::Value(value)) => match vl_value(value) {
                Ok(value) => out.params.push((format!("{group:#x}"), value)),
                Err(why) => out.skipped.push(Skipped { field: format!("{group:#x}"), why }),
            },
            Some(Node::Struct(fields)) => {
                for (field, node) in fields {
                    let name = format!("{group:#x}.{field:#x}");
                    match node {
                        Node::Value(value) => match vl_value(value) {
                            Ok(value) => out.params.push((name, value)),
                            Err(why) => out.skipped.push(Skipped { field: name, why }),
                        },
                        _ => out.skipped.push(Skipped { field: name, why: Why::Container }),
                    }
                }
            }
            Some(_) => out.skipped.push(Skipped { field: format!("{group:#x}"), why: Why::Container }),
        }
    }
    out
}

pub fn param_key(name: &str) -> (u64, u64) {
    match name.split_once('.') {
        Some((group, field)) => (key(group), key(field)),
        None => (key(name), 0),
    }
}

pub const ITEM_FILE: &str = "common_param";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemRow {
    pub fields: Vec<(u64, Value)>,
    pub skipped: Vec<Skipped>,
    pub rows: usize,
}

pub fn item_row(root: &Node) -> ItemRow {
    row_values(root, "params", &["name"])
}

pub const ENEMY_FILE: &str = "common";
pub const CPU_FILE: &str = "fighter_param";

pub fn enemy_row(root: &Node) -> ItemRow {
    row_values(root, "enemy_param_table", &["kind"])
}

pub fn cpu_row(root: &Node) -> ItemRow {
    row_values(root, "base", &["name_hash"])
}

pub fn row_values(root: &Node, list: &str, identity: &[&str]) -> ItemRow {
    let found = rows(root, list);
    let mut out = ItemRow {
        rows: found.len(),
        ..Default::default()
    };
    let Some(row) = found.first() else {
        return out;
    };
    let identity: Vec<u64> = identity.iter().map(|name| hash40(name)).collect();
    for (hash, node) in row.iter() {
        if identity.contains(hash) {
            continue;
        }
        match node {
            Node::Value(Value::Text(_)) => out.skipped.push(Skipped { field: format!("{hash:#x}"), why: Why::Text }),
            Node::Value(value) => out.fields.push((*hash, value.clone())),
            _ => out.skipped.push(Skipped { field: format!("{hash:#x}"), why: Why::Container }),
        }
    }
    out
}

pub const STAGE_FILE: &str = "item_switch_param";

pub fn item_switch_table(root: &Node) -> Option<Vec<(u16, u8)>> {
    let found = rows(root, "itsw_normal");
    let row = found.first()?;
    let table = row.iter().find(|(hash, _)| *hash == hash40("table")).map(|(_, node)| node)?;
    let Node::List(entries) = table else {
        return None;
    };
    let mut out: Vec<(u16, u8)> = Vec::new();
    for (index, node) in entries {
        let flag = match node {
            Node::Value(Value::Int(value)) => u8::from(*value != 0),
            Node::Value(Value::Bool(value)) => u8::from(*value),
            _ => continue,
        };
        let Ok(index) = u16::try_from(*index) else {
            continue;
        };
        out.retain(|(known, _)| *known != index);
        out.push((index, flag));
    }
    Some(out)
}

pub const STAGE_SOUND_FILE: &str = "stage_sound_settings";

pub const REVERB_PRESETS: [&str; 11] = [
    "StRev_None",
    "StRev_Outdoor",
    "StRev_S",
    "StRev_M",
    "StRev_L",
    "SmplMap_1",
    "SmplMap_6",
    "SmplMap_8",
    "Figure",
    "FX_REV_CROSS",
    "FX_Delay_C2_Stage_plankton",
];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StageSound {
    pub reverb: Option<u32>,
    pub scale: Option<f32>,
    pub offset: Option<f32>,
}

impl StageSound {
    pub fn is_empty(&self) -> bool {
        self.reverb.is_none() && self.scale.is_none() && self.offset.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StageSoundRow {
    pub sound: StageSound,
    pub skipped: Vec<Skipped>,
    pub rows: usize,
}

fn number(value: &Value) -> Option<f32> {
    match value {
        Value::Float(value) => Some(*value as f32),
        Value::Int(value) => Some(*value as f32),
        _ => None,
    }
}

fn named<'a>(names: &[&'a str], hash: u64) -> Option<&'a str> {
    names.iter().copied().find(|name| hash40(name) == hash)
}

pub fn stage_sound(root: &Node) -> StageSoundRow {
    let found = rows(root, "info_");
    let mut out = StageSoundRow {
        rows: found.len(),
        ..Default::default()
    };
    let Some(row) = found.first() else {
        return out;
    };
    let identity = hash40("place_id_");
    for (hash, node) in row.iter() {
        if *hash == identity {
            continue;
        }
        let name = named(&["reverb_type_", "scale_", "offset_"], *hash);
        let shown = name.map(str::to_string).unwrap_or_else(|| format!("{hash:#x}"));
        let Node::Value(value) = node else {
            out.skipped.push(Skipped { field: shown, why: Why::Container });
            continue;
        };
        let why = match (name, value) {
            (None, _) => Why::Unknown,
            (Some("reverb_type_"), Value::Int(kind)) => match u32::try_from(*kind) {
                Ok(kind) if (kind as usize) < REVERB_PRESETS.len() => {
                    out.sound.reverb = Some(kind);
                    continue;
                }
                _ => Why::Range,
            },
            (Some("reverb_type_"), _) => Why::Type,
            (Some(field), value) => match number(value) {
                Some(number) if field == "scale_" => {
                    out.sound.scale = Some(number);
                    continue;
                }
                Some(number) => {
                    out.sound.offset = Some(number);
                    continue;
                }
                None => Why::Type,
            },
        };
        out.skipped.push(Skipped { field: shown, why });
    }
    out
}

pub const STAGE_AREA_FILE: &str = "stage_param";

const AREA_FIELDS: [&str; 4] = ["x0", "y0", "x1", "y1"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StageAreaRow {
    pub area: Option<Area>,
    pub missing: Vec<&'static str>,
    pub skipped: Vec<Skipped>,
    pub rows: usize,
}

pub fn stage_area(root: &Node) -> StageAreaRow {
    let found = rows(root, "area");
    let mut out = StageAreaRow {
        rows: found.len(),
        ..Default::default()
    };
    let Some(row) = found.first() else {
        return out;
    };
    let identity = hash40("stage_name");
    let mut values: [Option<f32>; 4] = [None; 4];
    for (hash, node) in row.iter() {
        if *hash == identity {
            continue;
        }
        let at = AREA_FIELDS.iter().position(|name| hash40(name) == *hash);
        let shown = at.map(|at| AREA_FIELDS[at].to_string()).unwrap_or_else(|| format!("{hash:#x}"));
        let why = match (at, node) {
            (_, Node::List(_) | Node::Struct(_)) => Why::Container,
            (None, _) => Why::Unknown,
            (Some(at), Node::Value(value)) => match number(value) {
                Some(number) => {
                    values[at] = Some(number);
                    continue;
                }
                None => Why::Type,
            },
        };
        out.skipped.push(Skipped { field: shown, why });
    }
    out.missing = AREA_FIELDS
        .iter()
        .zip(values.iter())
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| *name)
        .collect();
    if let [Some(x0), Some(y0), Some(x1), Some(y1)] = values {
        out.area = Some(Area { x0, y0, x1, y1 });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIGHTER_PARAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<struct>
  <list hash="fighter_param_table">
    <hash40 index="0">dummy</hash40>
    <struct index="3">
      <float hash="walk_speed_max">1.25</float>
      <float hash="weight">110</float>
      <int hash="jump_squat_frame">4</int>
      <hash40 hash="fighter_kind">fighter_kind_samus</hash40>
      <hash40 hash="size">fighter_size_l</hash40>
      <hash40 hash="ice_node">hip</hash40>
      <float hash="no_such_field">1</float>
    </struct>
  </list>
</struct>
"#;

    #[test]
    fn a_row_written_for_any_index_becomes_the_clone_values() {
        let root = parse(FIGHTER_PARAM).unwrap();
        let out = fighter_params("fighter_param", &root).unwrap();
        assert_eq!(out.rows, 1);
        assert_eq!(
            out.params,
            vec![
                ("walk_speed_max".to_string(), ParamValue::Float(1.25)),
                ("weight".to_string(), ParamValue::Float(110.0)),
                ("jump_squat_frame".to_string(), ParamValue::Int(4)),
            ]
        );
        let unknown = format!("{:#x}", hash40("no_such_field"));
        let why: Vec<(&str, Why)> = out.skipped.iter().map(|skip| (skip.field.as_str(), skip.why)).collect();
        assert_eq!(why, vec![("size", Why::Label), ("ice_node", Why::Hash), (unknown.as_str(), Why::Unknown)]);
    }

    #[test]
    fn common_files_are_top_level_scalars_and_motion_rows_take_their_prefix() {
        let common = parse(
            "<struct><float hash=\"shield_max\">60</float><list hash=\"pattern_power_mul\"><float index=\"0\">1</float></list></struct>",
        )
        .unwrap();
        let out = fighter_params("common", &common).unwrap();
        assert_eq!(out.params, vec![("common.shield_max".to_string(), ParamValue::Float(60.0))]);
        assert_eq!(out.skipped[0].why, Why::Container);
        let motion = parse(
            "<struct><list hash=\"fighter_param_motion_table\"><struct index=\"40\"><bool hash=\"flip\">false</bool></struct></list></struct>",
        )
        .unwrap();
        let out = fighter_params("fighter_param_motion", &motion).unwrap();
        assert_eq!(out.params, vec![("param_motion.flip".to_string(), ParamValue::Int(0))]);
        assert!(fighter_params("vl", &motion).is_none());
    }

    #[test]
    fn hex_names_comments_entities_and_self_closing_elements_parse() {
        let text = "\u{feff}<!-- edited -->\n<struct>\n <struct hash=\"0x0123456789\" />\n <string hash=\"label\">a &amp; b</string>\n <list hash=\"empty\"/>\n</struct>";
        let root = parse(text).unwrap();
        assert_eq!(
            root,
            Node::Struct(vec![
                (0x0123456789, Node::Struct(vec![])),
                (hash40("label"), Node::Value(Value::Text("a & b".to_string()))),
                (hash40("empty"), Node::List(vec![])),
            ])
        );
        assert_eq!(key("0x0123456789"), 0x0123456789);
        assert_eq!(key("weight"), hash40("weight"));
    }

    #[test]
    fn broken_files_say_which_line() {
        let error = parse("<struct>\n<float hash=\"weight\">heavy</float>\n</struct>").unwrap_err();
        assert_eq!(error.line, 2);
        assert!(parse("<struct>\n<float hash=\"weight\">1</float>\n").is_err());
        assert!(parse("<list hash=\"a\"></list>").is_err());
        assert!(parse("<struct><float>1</float></struct>").is_err());
    }

    #[test]
    fn the_item_row_keeps_every_scalar_but_its_name() {
        let root = parse(
            "<struct><list hash=\"params\"><struct index=\"12\"><hash40 hash=\"name\">killsword</hash40><float hash=\"throw_speed_mul\">0.75</float><hash40 hash=\"hit_kind\">item_hit_kind_fly</hash40><bool hash=\"is_quick_throw\">true</bool></struct></list></struct>",
        )
        .unwrap();
        let out = item_row(&root);
        assert_eq!(out.rows, 1);
        assert_eq!(
            out.fields,
            vec![
                (hash40("throw_speed_mul"), Value::Float(0.75)),
                (hash40("hit_kind"), Value::Hash(hash40("item_hit_kind_fly"))),
                (hash40("is_quick_throw"), Value::Bool(true)),
            ]
        );
    }

    #[test]
    fn the_item_switch_row_skips_dummies_and_keeps_the_last_word() {
        let root = parse(
            "<struct><list hash=\"itsw_normal\"><struct index=\"40\"><hash40 hash=\"name\">battlefield</hash40><list hash=\"table\"><hash40 index=\"0\">dummy</hash40><byte index=\"3\">0</byte><byte index=\"7\">1</byte><byte index=\"3\">2</byte></list></struct></list></struct>",
        )
        .unwrap();
        assert_eq!(item_switch_table(&root), Some(vec![(7, 1), (3, 1)]));
        assert_eq!(item_switch_table(&parse("<struct/>").unwrap()), None);
    }

    #[test]
    fn the_sound_row_takes_a_reverb_preset_and_two_numbers() {
        let root = parse(
            "<struct><list hash=\"info_\"><struct index=\"0\"><float hash=\"scale_\">1.5</float><int hash=\"offset_\">-20</int><hash40 hash=\"place_id_\">battlefield</hash40><byte hash=\"reverb_type_\">10</byte></struct></list></struct>",
        )
        .unwrap();
        let out = stage_sound(&root);
        assert_eq!(out.rows, 1);
        assert_eq!(
            out.sound,
            StageSound {
                reverb: Some(10),
                scale: Some(1.5),
                offset: Some(-20.0)
            }
        );
        assert!(out.skipped.is_empty());
        let wrong = parse(
            "<struct><list hash=\"info_\"><struct index=\"0\"><byte hash=\"reverb_type_\">11</byte><hash40 hash=\"scale_\">big</hash40><float hash=\"volume\">1</float></struct></list></struct>",
        )
        .unwrap();
        let out = stage_sound(&wrong);
        assert!(out.sound.is_empty());
        let why: Vec<Why> = out.skipped.iter().map(|skip| skip.why).collect();
        assert_eq!(why, vec![Why::Range, Why::Type, Why::Unknown]);
        assert_eq!(stage_sound(&parse("<struct/>").unwrap()).rows, 0);
    }

    #[test]
    fn an_area_needs_all_four_edges() {
        let root = parse(
            "<struct><list hash=\"area\"><struct index=\"46\"><hash40 hash=\"stage_name\">my_stage</hash40><float hash=\"x0\">-80</float><float hash=\"x1\">80</float><float hash=\"y0\">-5</float><int hash=\"y1\">40</int></struct></list></struct>",
        )
        .unwrap();
        let out = stage_area(&root);
        assert_eq!(
            out.area,
            Some(Area {
                x0: -80.0,
                y0: -5.0,
                x1: 80.0,
                y1: 40.0
            })
        );
        let partial = parse(
            "<struct><list hash=\"area\"><struct index=\"0\"><float hash=\"x0\">-80</float><float hash=\"y1\">40</float></struct></list></struct>",
        )
        .unwrap();
        let out = stage_area(&partial);
        assert_eq!(out.area, None);
        assert_eq!(out.missing, vec!["y0", "x1"]);
    }

    #[test]
    fn a_vl_file_gives_every_scalar_by_group_and_field() {
        let root = parse(
            "<struct><struct hash=\"param_special_n\"><float hash=\"charge_speed\">1.5</float><bool hash=\"is_new\">true</bool><hash40 hash=\"effect\">sys_fire</hash40></struct><int hash=\"top_level\">3</int><list hash=\"hit_data\"><float index=\"0\">1</float></list></struct>",
        )
        .unwrap();
        let out = vl_params(&root);
        let group = hash40("param_special_n");
        assert_eq!(
            out.params,
            vec![
                (format!("{group:#x}.{:#x}", hash40("charge_speed")), ParamValue::Float(1.5)),
                (format!("{group:#x}.{:#x}", hash40("is_new")), ParamValue::Int(1)),
                (format!("{:#x}", hash40("top_level")), ParamValue::Int(3)),
            ]
        );
        let why: Vec<Why> = out.skipped.iter().map(|skip| skip.why).collect();
        assert_eq!(why, vec![Why::Hash, Why::Container]);
        assert_eq!(param_key(&out.params[0].0), (group, hash40("charge_speed")));
        assert_eq!(param_key("param_special_n.charge_speed"), (group, hash40("charge_speed")));
        assert_eq!(param_key("weight"), (hash40("weight"), 0));
        let article = parse(
            "<struct><list hash=\"param_fireball\"><struct index=\"0\"><float hash=\"speed\">0.4</float></struct><struct index=\"1\"><float hash=\"speed\">9</float></struct></list></struct>",
        )
        .unwrap();
        let out = vl_params(&article);
        assert_eq!(
            out.params,
            vec![(format!("{:#x}.{:#x}", hash40("param_fireball"), hash40("speed")), ParamValue::Float(0.4))]
        );
    }

    #[test]
    fn enemy_and_cpu_rows_skip_their_identity() {
        let enemy = parse(
            "<struct><list hash=\"enemy_param_table\"><struct index=\"4\"><hash40 hash=\"kind\">enemy_kind_nabbit</hash40><int hash=\"hp\">40</int></struct></list></struct>",
        )
        .unwrap();
        assert_eq!(enemy_row(&enemy).fields, vec![(hash40("hp"), Value::Int(40))]);
        let cpu = parse(
            "<struct><list hash=\"base\"><struct index=\"0\"><hash40 hash=\"name_hash\">fighter_kind_mario</hash40><float hash=\"width\">5</float></struct></list></struct>",
        )
        .unwrap();
        assert_eq!(cpu_row(&cpu).fields, vec![(hash40("width"), Value::Float(5.0))]);
    }
}
