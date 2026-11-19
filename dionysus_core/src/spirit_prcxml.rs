use std::collections::HashMap;

use crate::hash40;
use crate::menu::layout_root;
use crate::prcxml::{self, Node, Value as XmlValue};
use crate::spirits::{self, Board, Declaration, Field, Manifest, SpiritError, Table, Value, MAX_FIGHTERS};

pub const SPIRITS_FILE: &str = "spirit/ui_spirit_db.prcxml";
pub const BATTLES_FILE: &str = "spirit/ui_spirits_battle_db.prcxml";
pub const LAYOUTS_FILE: &str = "spirit/ui_spirit_layout_db.prcxml";
pub const BOARDS_FILE: &str = "spirit/spirits_board_special_param.prcxml";
pub const FILES: [&str; 4] = [SPIRITS_FILE, BATTLES_FILE, LAYOUTS_FILE, BOARDS_FILE];

#[derive(Debug, Clone, Copy, Default)]
pub struct Files<'a> {
    pub spirits: Option<&'a str>,
    pub battles: Option<&'a str>,
    pub layouts: Option<&'a str>,
    pub boards: Option<&'a str>,
}

impl<'a> Files<'a> {
    pub fn any(&self) -> bool {
        self.spirits.is_some() || self.battles.is_some() || self.layouts.is_some() || self.boards.is_some()
    }
}

fn quoted(text: &str, quote: char) -> Option<&str> {
    let end = text.find(quote)?;
    Some(&text[..end])
}

pub fn names(text: &str) -> HashMap<u64, String> {
    let mut out = HashMap::new();
    let mut keep = |name: &str| {
        let name = name.trim();
        if !name.is_empty() && !name.starts_with("0x") && !name.starts_with("0X") {
            out.insert(hash40(name), name.to_string());
        }
    };
    for (index, _) in text.match_indices("hash=") {
        let rest = &text[index + 5..];
        let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            continue;
        };
        if let Some(name) = quoted(&rest[1..], quote) {
            keep(name);
        }
    }
    for (index, _) in text.match_indices("<hash40") {
        let rest = &text[index..];
        let Some(open) = rest.find('>') else {
            continue;
        };
        if rest[..open].ends_with('/') {
            continue;
        }
        if let Some(value) = quoted(&rest[open + 1..], '<') {
            keep(value);
        }
    }
    out
}

fn label(names: &HashMap<u64, String>, hash: u64) -> String {
    names.get(&hash).cloned().unwrap_or_else(|| format!("0x{hash:010x}"))
}

fn text_of(node: &Node, names: &HashMap<u64, String>) -> Option<String> {
    match node {
        Node::Value(XmlValue::Hash(hash)) => Some(label(names, *hash)),
        Node::Value(XmlValue::Text(text)) => Some(text.trim().to_string()),
        _ => None,
    }
}

fn value_of(node: &Node, names: &HashMap<u64, String>) -> Option<Value> {
    match node {
        Node::Value(XmlValue::Bool(flag)) => Some(Value::Flag(*flag)),
        Node::Value(XmlValue::Int(number)) => Some(Value::Number(*number)),
        Node::Value(XmlValue::Float(number)) => Some(Value::Float(*number)),
        Node::Value(XmlValue::Hash(hash)) => Some(Value::Text(label(names, *hash))),
        Node::Value(XmlValue::Text(text)) => Some(Value::Text(text.clone())),
        _ => None,
    }
}

fn problem(file: &'static str, row: usize, message: String) -> SpiritError {
    SpiritError::Prcxml { file, row, message }
}

fn tree(file: &'static str, text: &str, names: &mut HashMap<u64, String>) -> Result<Node, SpiritError> {
    names.extend(self::names(text));
    prcxml::parse(text).map_err(|error| SpiritError::Prcxml {
        file,
        row: 0,
        message: format!("line {}: {}", error.line, error.message),
    })
}

fn field(table: Table, fighter: usize, name: String, value: Value) -> Field {
    Field {
        table,
        fighter,
        name,
        value,
    }
}

fn owner<'a>(
    spirits: &'a mut [Declaration],
    row: &[(u64, Node)],
    key: &str,
    names: &HashMap<u64, String>,
    file: &'static str,
    number: usize,
) -> Result<&'a mut Declaration, SpiritError> {
    let Some(id) = row.iter().find(|(hash, _)| *hash == hash40(key)).and_then(|(_, node)| text_of(node, names)) else {
        return Err(problem(file, number, format!("the row has no {key}")));
    };
    let hash = spirits::name_key(&id);
    spirits.iter_mut().find(|declaration| declaration.id == id || hash40(&declaration.id) == hash).ok_or_else(|| {
        problem(file, number, format!("the row is for {id}, which {SPIRITS_FILE} does not add"))
    })
}

pub fn parse(files: &Files) -> Result<Manifest, SpiritError> {
    let mut names = HashMap::new();
    let mut manifest = Manifest::default();
    if let Some(text) = files.spirits {
        let root = tree(SPIRITS_FILE, text, &mut names)?;
        for (index, row) in prcxml::rows(&root, "db_root").into_iter().enumerate() {
            let number = index + 1;
            let mut declaration = Declaration {
                id: String::new(),
                base: String::new(),
                fighters: None,
                fields: Vec::new(),
                line: number,
            };
            let mut name_id = String::new();
            for (hash, node) in row {
                let name = label(&names, *hash);
                match name.as_str() {
                    "ui_spirit_id" => declaration.id = text_of(node, &names).unwrap_or_default(),
                    "name_id" => name_id = text_of(node, &names).unwrap_or_default(),
                    "base" => declaration.base = text_of(node, &names).unwrap_or_default(),
                    known if Table::Spirit.fixed().contains(&known) => {}
                    _ => {
                        if let Some(value) = value_of(node, &names) {
                            declaration.fields.push(field(Table::Spirit, 0, name, value));
                        }
                    }
                }
            }
            let unnamed = declaration.id.starts_with("0x") || !spirits::valid_id(&declaration.id);
            if unnamed && spirits::valid_id(&name_id) && !name_id.starts_with("0x") {
                declaration.id = name_id;
            }
            spirits::finish(declaration, &mut manifest.spirits)
                .map_err(|error| problem(SPIRITS_FILE, number, format!("{error:?}")))?;
        }
    }
    if let Some(text) = files.battles {
        let root = tree(BATTLES_FILE, text, &mut names)?;
        for (index, row) in prcxml::rows(&root, "battle_data_tbl").into_iter().enumerate() {
            let declaration = owner(&mut manifest.spirits, row, "battle_id", &names, BATTLES_FILE, index + 1)?;
            for (hash, node) in row {
                let name = label(&names, *hash);
                if !Table::Battle.fixed().contains(&name.as_str()) {
                    if let Some(value) = value_of(node, &names) {
                        declaration.fields.push(field(Table::Battle, 0, name, value));
                    }
                }
            }
        }
        let mut counts: HashMap<String, usize> = HashMap::new();
        for (index, row) in prcxml::rows(&root, "fighter_data_tbl").into_iter().enumerate() {
            let declaration = owner(&mut manifest.spirits, row, "battle_id", &names, BATTLES_FILE, index + 1)?;
            let count = counts.entry(declaration.id.clone()).or_insert(0);
            if *count >= MAX_FIGHTERS {
                return Err(problem(BATTLES_FILE, index + 1, format!("{} has more than {MAX_FIGHTERS} fighters", declaration.id)));
            }
            for (hash, node) in row {
                let name = label(&names, *hash);
                if !Table::Fighter.fixed().contains(&name.as_str()) && name != "spirit_name" {
                    if let Some(value) = value_of(node, &names) {
                        declaration.fields.push(field(Table::Fighter, *count, name, value));
                    }
                }
            }
            *count += 1;
            declaration.fighters = Some(*count);
        }
    }
    if let Some(text) = files.layouts {
        let root = tree(LAYOUTS_FILE, text, &mut names)?;
        for (index, row) in prcxml::rows(&root, "db_root").into_iter().enumerate() {
            let declaration =
                owner(&mut manifest.spirits, row, "ui_spirit_layout_id", &names, LAYOUTS_FILE, index + 1)?;
            for (hash, node) in row {
                let name = label(&names, *hash);
                if !Table::Layout.fixed().contains(&name.as_str()) {
                    if let Some(value) = value_of(node, &names) {
                        declaration.fields.push(field(Table::Layout, 0, name, value));
                    }
                }
            }
        }
    }
    if let Some(text) = files.boards {
        let root = tree(BOARDS_FILE, text, &mut names)?;
        for (index, row) in prcxml::rows(&root, "special_board_data_list").into_iter().enumerate() {
            let number = index + 1;
            let mut board = Board {
                id: String::new(),
                base: String::new(),
                name: String::new(),
                background: None,
                spirits: Vec::new(),
                art: None,
                fields: Vec::new(),
                line: number,
            };
            for (hash, node) in row {
                let name = label(&names, *hash);
                match name.as_str() {
                    "label" => {
                        let text = text_of(node, &names).unwrap_or_default();
                        board.id = text.strip_prefix("ui_chara_").unwrap_or(&text).to_string();
                    }
                    "base" => board.base = text_of(node, &names).unwrap_or_default(),
                    "name" => board.name = text_of(node, &names).unwrap_or_default(),
                    "background" => board.background = text_of(node, &names),
                    "art" => {
                        let art = text_of(node, &names).unwrap_or_default().to_ascii_lowercase();
                        if layout_root(&art).is_none() {
                            return Err(problem(BOARDS_FILE, number, format!("art {art:?} is not a layout path ending in /layout.arc")));
                        }
                        board.art = Some(art);
                    }
                    "spirit_list" => {
                        if let Node::List(entries) = node {
                            for (_, entry) in entries {
                                if let Some(spirit) = prcxml::member(entry, "spirit_id").and_then(|node| text_of(node, &names)) {
                                    board.spirits.push(spirit);
                                }
                            }
                        }
                    }
                    "save_no" => {}
                    _ => {
                        if let Some(value) = value_of(node, &names) {
                            board.fields.push(field(Table::Spirit, 0, name, value));
                        }
                    }
                }
            }
            spirits::finish_board(board, &mut manifest.boards)
                .map_err(|error| problem(BOARDS_FILE, number, format!("{error:?}")))?;
        }
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPIRITS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<struct>
  <list hash="db_root">
    <struct index="0">
      <hash40 hash="ui_spirit_id">my_crab</hash40>
      <hash40 hash="base">sidestepper</hash40>
      <string hash="name_id">my_crab</string>
      <ushort hash="save_no">1600</ushort>
      <hash40 hash="rank">legend</hash40>
      <short hash="base_attack">1234</short>
      <bool hash="is_board_appear">true</bool>
      <float hash="0x16db57210b">2.5</float>
    </struct>
  </list>
</struct>
"#;

    const BATTLES: &str = r#"<struct>
  <list hash="battle_data_tbl">
    <struct index="0">
      <hash40 hash="battle_id">my_crab</hash40>
      <hash40 hash="ui_stage_id">ui_stage_battle_field</hash40>
    </struct>
  </list>
  <list hash="fighter_data_tbl">
    <struct index="0"><hash40 hash="battle_id">my_crab</hash40><hash40 hash="fighter_kind">ui_chara_luigi</hash40></struct>
    <struct index="1"><hash40 hash="battle_id">my_crab</hash40><ushort hash="hp">150</ushort><hash40 hash="spirit_name">x</hash40></struct>
  </list>
</struct>
"#;

    const BOARDS: &str = r#"<struct>
  <list hash="special_board_data_list">
    <struct index="0">
      <hash40 hash="label">ui_chara_crab_board</hash40>
      <hash40 hash="base">trail</hash40>
      <string hash="name">Crab Board</string>
      <uint hash="value">5000</uint>
      <uint hash="layout_no">12</uint>
      <string hash="art">ui/layout/menu/my_boards/spb_crab_board/layout.arc</string>
      <list hash="spirit_list">
        <struct index="0"><hash40 hash="spirit_id">my_crab</hash40></struct>
        <struct index="1"><hash40 hash="spirit_id">geno</hash40></struct>
      </list>
    </struct>
  </list>
</struct>
"#;

    #[test]
    fn prcxml_rows_become_the_same_declarations_as_spirit_toml() {
        let manifest = parse(&Files {
            spirits: Some(SPIRITS),
            battles: Some(BATTLES),
            layouts: Some(
                "<struct><list hash=\"db_root\"><struct index=\"0\"><hash40 hash=\"ui_spirit_layout_id\">my_crab</hash40><float hash=\"ui_art_scale\">1.25</float></struct></list></struct>",
            ),
            boards: Some(BOARDS),
        })
        .unwrap();
        let crab = &manifest.spirits[0];
        assert_eq!((crab.id.as_str(), crab.base.as_str(), crab.fighters), ("my_crab", "sidestepper", Some(2)));
        let keys: Vec<(Table, usize, &str)> = crab.fields.iter().map(|f| (f.table, f.fighter, f.name.as_str())).collect();
        assert_eq!(
            keys,
            vec![
                (Table::Spirit, 0, "rank"),
                (Table::Spirit, 0, "base_attack"),
                (Table::Spirit, 0, "is_board_appear"),
                (Table::Spirit, 0, "0x16db57210b"),
                (Table::Battle, 0, "ui_stage_id"),
                (Table::Fighter, 0, "fighter_kind"),
                (Table::Fighter, 1, "hp"),
                (Table::Layout, 0, "ui_art_scale"),
            ]
        );
        assert_eq!(crab.fields[0].value, Value::Text("legend".into()));
        assert_eq!(crab.fields[1].value, Value::Number(1234));
        let board = &manifest.boards[0];
        assert_eq!((board.id.as_str(), board.base.as_str(), board.name.as_str()), ("crab_board", "trail", "Crab Board"));
        assert_eq!(board.spirits, vec!["my_crab", "geno"]);
        assert_eq!(board.art_root().as_deref(), Some("spb_crab_board"));
        assert_eq!(board.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), vec!["value", "layout_no"]);
    }

    #[test]
    fn rows_for_spirits_the_file_does_not_add_and_bad_files_are_refused() {
        assert!(matches!(
            parse(&Files { battles: Some(BATTLES), ..Files::default() }),
            Err(SpiritError::Prcxml { file: BATTLES_FILE, .. })
        ));
        assert!(matches!(
            parse(&Files { spirits: Some(&SPIRITS.replace("<hash40 hash=\"base\">sidestepper</hash40>", "")), ..Files::default() }),
            Err(SpiritError::Prcxml { file: SPIRITS_FILE, row: 1, .. })
        ));
        assert!(matches!(
            parse(&Files { spirits: Some("<struct><list hash=\"db_root\">"), ..Files::default() }),
            Err(SpiritError::Prcxml { file: SPIRITS_FILE, row: 0, .. })
        ));
        let unnamed = SPIRITS.replace("<hash40 hash=\"ui_spirit_id\">my_crab</hash40>", "<hash40 hash=\"ui_spirit_id\">0x1234</hash40>");
        assert_eq!(parse(&Files { spirits: Some(&unnamed), ..Files::default() }).unwrap().spirits[0].id, "my_crab");
        let hex = format!("0x{:010x}", hash40("my_crab"));
        let battles = BATTLES.replace(">my_crab<", &format!(">{hex}<"));
        let read = parse(&Files { spirits: Some(&unnamed), battles: Some(&battles), ..Files::default() }).unwrap();
        assert_eq!(read.spirits[0].fighters, Some(2));
        assert!(!Files::default().any());
    }
}
