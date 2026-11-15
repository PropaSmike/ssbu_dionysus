use crate::hash40;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.iter().rev().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: &'static str,
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

const MAX_DEPTH: usize = 64;

impl<'a> Parser<'a> {
    fn fail<T>(&self, message: &'static str) -> Result<T, JsonError> {
        Err(JsonError { offset: self.at, message })
    }

    fn skip_space(&mut self) {
        while let Some(byte) = self.bytes.get(self.at) {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn literal(&mut self, word: &[u8], value: Json) -> Result<Json, JsonError> {
        if self.bytes[self.at..].starts_with(word) {
            self.at += word.len();
            Ok(value)
        } else {
            self.fail("unknown literal")
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return self.fail("nested too deep");
        }
        self.skip_space();
        match self.bytes.get(self.at) {
            None => self.fail("unexpected end"),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b't') => self.literal(b"true", Json::Bool(true)),
            Some(b'f') => self.literal(b"false", Json::Bool(false)),
            Some(b'n') => self.literal(b"null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.fail("unexpected character"),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.at += 1;
        let mut fields = Vec::new();
        self.skip_space();
        if self.eat(b'}') {
            return Ok(Json::Object(fields));
        }
        loop {
            self.skip_space();
            if self.bytes.get(self.at) != Some(&b'"') {
                return self.fail("expected a key");
            }
            let key = self.string()?;
            self.skip_space();
            if !self.eat(b':') {
                return self.fail("expected ':'");
            }
            let value = self.value(depth + 1)?;
            fields.push((key, value));
            self.skip_space();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b'}') {
                return Ok(Json::Object(fields));
            }
            return self.fail("expected ',' or '}'");
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.at += 1;
        let mut items = Vec::new();
        self.skip_space();
        if self.eat(b']') {
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b']') {
                return Ok(Json::Array(items));
            }
            return self.fail("expected ',' or ']'");
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self.bytes.get(self.at..self.at + 4).ok_or(JsonError {
            offset: self.at,
            message: "short \\u escape",
        })?;
        let text = core::str::from_utf8(digits).map_err(|_| JsonError {
            offset: self.at,
            message: "bad \\u escape",
        })?;
        let value = u32::from_str_radix(text, 16).map_err(|_| JsonError {
            offset: self.at,
            message: "bad \\u escape",
        })?;
        self.at += 4;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.at += 1;
        let mut out = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.at) else {
                return self.fail("unterminated string");
            };
            self.at += 1;
            match byte {
                b'"' => break,
                b'\\' => {
                    let Some(&escape) = self.bytes.get(self.at) else {
                        return self.fail("unterminated escape");
                    };
                    self.at += 1;
                    match escape {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xd800..0xdc00).contains(&code) {
                                if !(self.eat(b'\\') && self.eat(b'u')) {
                                    return self.fail("lone surrogate");
                                }
                                let low = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&low) {
                                    return self.fail("lone surrogate");
                                }
                                code = 0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                            }
                            let Some(character) = char::from_u32(code) else {
                                return self.fail("bad \\u escape");
                            };
                            let mut buffer = [0u8; 4];
                            out.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                        }
                        _ => return self.fail("unknown escape"),
                    }
                }
                0x00..=0x1f => return self.fail("control character in string"),
                _ => out.push(byte),
            }
        }
        String::from_utf8(out).map_err(|_| JsonError {
            offset: self.at,
            message: "string is not UTF-8",
        })
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.at;
        self.eat(b'-');
        if self.bytes.get(self.at) == Some(&b'0') && matches!(self.bytes.get(self.at + 1), Some(b'0'..=b'9')) {
            return self.fail("leading zero");
        }
        let digits = |parser: &mut Parser| {
            let from = parser.at;
            while matches!(parser.bytes.get(parser.at), Some(b'0'..=b'9')) {
                parser.at += 1;
            }
            parser.at > from
        };
        if !digits(self) {
            return self.fail("expected digits");
        }
        if self.eat(b'.') && !digits(self) {
            return self.fail("expected digits after '.'");
        }
        if self.eat(b'e') || self.eat(b'E') {
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if !digits(self) {
                return self.fail("expected exponent digits");
            }
        }
        let text = core::str::from_utf8(&self.bytes[start..self.at]).unwrap_or("");
        text.parse::<f64>()
            .map(Json::Number)
            .map_err(|_| JsonError { offset: start, message: "bad number" })
    }
}

pub fn parse(text: &str) -> Result<Json, JsonError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut parser = Parser { bytes: text.as_bytes(), at: 0 };
    let value = parser.value(0)?;
    parser.skip_space();
    if parser.at != parser.bytes.len() {
        return parser.fail("trailing data");
    }
    Ok(value)
}

pub fn hash_value(value: &Json) -> Option<u64> {
    match value {
        Json::String(text) => match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            Some(hex) => u64::from_str_radix(hex, 16).ok(),
            None => Some(hash40(text)),
        },
        Json::Number(number) if *number >= 0.0 && number.fract() == 0.0 => Some(*number as u64),
        _ => None,
    }
}

fn signed_byte(value: &Json) -> Result<Option<i8>, ()> {
    let number = match value {
        Json::Number(number) => *number,
        Json::Object(_) => match value.get("Overwrite").or_else(|| value.get("Optional")) {
            Some(Json::Number(number)) => *number,
            Some(Json::Null) => return Ok(None),
            _ => return Err(()),
        },
        Json::Null => return Ok(None),
        _ => return Err(()),
    };
    if number.fract() != 0.0 || number < i8::MIN as f64 || number > i8::MAX as f64 {
        return Err(());
    }
    Ok(Some(number as i8))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharaOrders {
    pub orders: Vec<(u64, i8)>,
    pub refused: Vec<String>,
}

pub fn chara_disp_orders(text: &str) -> Result<CharaOrders, JsonError> {
    let root = parse(text)?;
    let mut out = CharaOrders::default();
    let Some(Json::Array(entries)) = root.get("character_database_entries") else {
        return Ok(out);
    };
    for entry in entries {
        let Some(order) = entry.get("disp_order") else {
            continue;
        };
        let Some(ui_chara) = entry.get("ui_chara_id").and_then(hash_value) else {
            out.refused.push("an entry with disp_order has no usable ui_chara_id".to_string());
            continue;
        };
        match signed_byte(order) {
            Ok(Some(order)) => {
                out.orders.retain(|(known, _)| *known != ui_chara);
                out.orders.push((ui_chara, order));
            }
            Ok(None) => {}
            Err(()) => out.refused.push(format!("{ui_chara:#x}: disp_order {order:?} is not a whole number from -128 to 127")),
        }
    }
    Ok(out)
}

pub fn prc_disp_orders(bytes: &[u8]) -> Result<Vec<(u64, i8)>, crate::prc::PrcError> {
    let root = crate::prc::parse(bytes)?;
    let rows = root
        .get(hash40("db_root"))
        .and_then(crate::prc::Param::list)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut orders: Vec<(u64, i8)> = Vec::new();
    for row in rows {
        let Some(ui_chara) = row.get(hash40("ui_chara_id")).and_then(crate::prc::Param::hash) else {
            continue;
        };
        let Some(order) = row
            .get(hash40("disp_order"))
            .and_then(crate::prc::Param::integer)
            .and_then(|order| i8::try_from(order).ok())
        else {
            continue;
        };
        if orders.iter().any(|(known, _)| *known == ui_chara) {
            continue;
        }
        orders.push((ui_chara, order));
    }
    Ok(orders)
}

pub fn prc_duplicate_rows(bytes: &[u8]) -> Result<Vec<(u64, String, usize)>, crate::prc::PrcError> {
    let root = crate::prc::parse(bytes)?;
    let rows = root
        .get(hash40("db_root"))
        .and_then(crate::prc::Param::list)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut seen: Vec<(u64, String, usize)> = Vec::new();
    for row in rows {
        let Some(ui_chara) = row.get(hash40("ui_chara_id")).and_then(crate::prc::Param::hash) else {
            continue;
        };
        match seen.iter_mut().find(|(known, _, _)| *known == ui_chara) {
            Some((_, _, count)) => *count += 1,
            None => {
                let name = row
                    .get(hash40("name_id"))
                    .and_then(crate::prc::Param::text)
                    .unwrap_or_default()
                    .to_string();
                seen.push((ui_chara, name, 1));
            }
        }
    }
    seen.retain(|(_, _, count)| *count > 1);
    Ok(seen)
}

pub const VANILLA_DISP_ORDERS: &[(&str, i8)] = &[
    ("ui_chara_mario", 0),
    ("ui_chara_donkey", 1),
    ("ui_chara_link", 2),
    ("ui_chara_samus", 3),
    ("ui_chara_samusd", 4),
    ("ui_chara_yoshi", 5),
    ("ui_chara_kirby", 6),
    ("ui_chara_fox", 7),
    ("ui_chara_pikachu", 8),
    ("ui_chara_luigi", 9),
    ("ui_chara_ness", 10),
    ("ui_chara_captain", 11),
    ("ui_chara_purin", 12),
    ("ui_chara_peach", 13),
    ("ui_chara_daisy", 14),
    ("ui_chara_koopa", 15),
    ("ui_chara_ice_climber", 16),
    ("ui_chara_sheik", 17),
    ("ui_chara_zelda", 18),
    ("ui_chara_mariod", 19),
    ("ui_chara_pichu", 20),
    ("ui_chara_falco", 21),
    ("ui_chara_marth", 22),
    ("ui_chara_lucina", 23),
    ("ui_chara_younglink", 24),
    ("ui_chara_ganon", 25),
    ("ui_chara_mewtwo", 26),
    ("ui_chara_roy", 27),
    ("ui_chara_chrom", 28),
    ("ui_chara_gamewatch", 29),
    ("ui_chara_metaknight", 30),
    ("ui_chara_pit", 31),
    ("ui_chara_pitb", 32),
    ("ui_chara_szerosuit", 33),
    ("ui_chara_wario", 34),
    ("ui_chara_snake", 35),
    ("ui_chara_ike", 36),
    ("ui_chara_ptrainer", 37),
    ("ui_chara_diddy", 38),
    ("ui_chara_lucas", 39),
    ("ui_chara_sonic", 40),
    ("ui_chara_dedede", 41),
    ("ui_chara_pikmin", 42),
    ("ui_chara_lucario", 43),
    ("ui_chara_robot", 44),
    ("ui_chara_toonlink", 45),
    ("ui_chara_wolf", 46),
    ("ui_chara_murabito", 47),
    ("ui_chara_rockman", 48),
    ("ui_chara_wiifit", 49),
    ("ui_chara_rosetta", 50),
    ("ui_chara_littlemac", 51),
    ("ui_chara_gekkouga", 52),
    ("ui_chara_palutena", 53),
    ("ui_chara_pacman", 54),
    ("ui_chara_reflet", 55),
    ("ui_chara_shulk", 56),
    ("ui_chara_koopajr", 57),
    ("ui_chara_duckhunt", 58),
    ("ui_chara_ryu", 59),
    ("ui_chara_ken", 60),
    ("ui_chara_cloud", 61),
    ("ui_chara_kamui", 62),
    ("ui_chara_bayonetta", 63),
    ("ui_chara_richter", 67),
    ("ui_chara_inkling", 64),
    ("ui_chara_ridley", 65),
    ("ui_chara_krool", 68),
    ("ui_chara_simon", 66),
    ("ui_chara_shizue", 69),
    ("ui_chara_gaogaen", 70),
    ("ui_chara_miiall", 83),
    ("ui_chara_miifighter", 84),
    ("ui_chara_miiswordsman", 85),
    ("ui_chara_miigunner", 86),
    ("ui_chara_packun", 71),
    ("ui_chara_jack", 72),
    ("ui_chara_brave", 73),
    ("ui_chara_buddy", 74),
    ("ui_chara_dolly", 75),
    ("ui_chara_master", 76),
    ("ui_chara_tantan", 77),
    ("ui_chara_pickel", 78),
    ("ui_chara_edge", 79),
    ("ui_chara_flame_first", 80),
    ("ui_chara_light_first", 80),
    ("ui_chara_demon", 81),
    ("ui_chara_trail", 82),
];

pub fn vanilla_disp_orders() -> HashMap<u64, i8> {
    VANILLA_DISP_ORDERS.iter().map(|(name, order)| (hash40(name), *order)).collect()
}

pub fn translate_disp_order(order: i8, vanilla: &HashMap<u64, i8>, player: &HashMap<u64, i8>) -> i8 {
    if order < 0 {
        return order;
    }
    let mut pairs: Vec<(i8, i8)> = vanilla
        .iter()
        .filter(|(_, v)| **v >= 0)
        .filter_map(|(row, v)| player.get(row).filter(|p| **p >= 0).map(|p| (*v, *p)))
        .collect();
    pairs.sort();
    let Some(&(below, _)) = pairs.iter().filter(|(v, _)| *v <= order).last() else {
        return order;
    };
    let at = pairs.iter().filter(|(v, _)| *v == below).map(|(_, p)| *p).max().unwrap_or(order);
    let wanted = at as i16 + (order as i16 - below as i16);
    let next = pairs.iter().find(|(v, _)| *v > below).map(|(v, _)| *v);
    let ceiling = next
        .and_then(|next| pairs.iter().filter(|(v, _)| *v == next).map(|(_, p)| *p).min())
        .map(|p| (p as i16 - 1).max(at as i16))
        .unwrap_or(i8::MAX as i16);
    wanted.min(ceiling).clamp(at as i16, i8::MAX as i16) as i8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prc::{write, Param};

    fn table(rows: &[(&str, i8)]) -> HashMap<u64, i8> {
        rows.iter().map(|(name, order)| (hash40(name), *order)).collect()
    }

    #[test]
    fn the_vanilla_table_is_the_games_select_screen() {
        let vanilla = vanilla_disp_orders();
        assert_eq!(vanilla.len(), 88);
        assert_eq!(vanilla[&hash40("ui_chara_mario")], 0);
        assert_eq!(vanilla[&hash40("ui_chara_sonic")], 40);
        assert_eq!(vanilla[&hash40("ui_chara_miigunner")], 86);
        assert!(!vanilla.contains_key(&hash40("ui_chara_random")));
    }

    #[test]
    fn a_vanilla_numbering_leaves_every_pack_order_alone() {
        let vanilla = table(&[("mario", 0), ("donkey", 1), ("link", 2), ("sonic", 40), ("gunner", 86)]);
        for order in [-1, 0, 1, 2, 3, 39, 40, 41, 86, 120, 127] {
            assert_eq!(translate_disp_order(order, &vanilla, &vanilla), order, "order {order}");
        }
    }

    #[test]
    fn a_pack_order_follows_the_vanilla_neighbour_it_was_picked_against() {
        let vanilla = table(&[("mario", 0), ("donkey", 1), ("link", 2), ("samus", 3), ("sonic", 40), ("snake", 41), ("gunner", 86)]);
        let player = table(&[("mario", 0), ("donkey", 1), ("link", 4), ("samus", 5), ("sonic", 63), ("snake", 70), ("gunner", 120), ("funky", 2)]);
        assert_eq!(translate_disp_order(40, &vanilla, &player), 63);
        assert_eq!(translate_disp_order(3, &vanilla, &player), 5);
        assert_eq!(translate_disp_order(1, &vanilla, &player), 1);
        assert_eq!(translate_disp_order(120, &vanilla, &player), 127);
        assert_eq!(translate_disp_order(90, &vanilla, &player), 124);
        assert_eq!(translate_disp_order(-1, &vanilla, &player), -1);
    }

    #[test]
    fn a_gap_in_vanilla_stays_between_the_same_neighbours() {
        let vanilla = table(&[("a", 10), ("b", 20)]);
        assert_eq!(translate_disp_order(15, &vanilla, &table(&[("a", 30), ("b", 50)])), 35);
        assert_eq!(translate_disp_order(15, &vanilla, &table(&[("a", 30), ("b", 31)])), 30);
        assert_eq!(translate_disp_order(15, &vanilla, &table(&[("a", 30), ("b", 12)])), 30);
    }

    #[test]
    fn hidden_or_missing_vanilla_rows_are_no_anchor() {
        let vanilla = table(&[("a", 10), ("b", 20)]);
        assert_eq!(translate_disp_order(20, &vanilla, &table(&[("a", 30), ("b", -1)])), 40);
        assert_eq!(translate_disp_order(20, &vanilla, &table(&[("a", 30)])), 40);
        assert_eq!(translate_disp_order(5, &vanilla, &table(&[("a", 30)])), 5);
        assert_eq!(translate_disp_order(20, &vanilla, &HashMap::new()), 20);
    }

    fn chara_row(ui_chara: &str, order: Option<i8>) -> Param {
        let mut fields = vec![(hash40("ui_chara_id"), Param::Hash(hash40(ui_chara))), (hash40("name_id"), Param::Str("x".into()))];
        if let Some(order) = order {
            fields.push((hash40("disp_order"), Param::I8(order)));
        }
        Param::Struct(fields)
    }

    #[test]
    fn a_players_chara_db_gives_each_rows_disp_order() {
        let root = Param::Struct(vec![(
            hash40("db_root"),
            Param::List(vec![
                chara_row("ui_chara_mario", Some(0)),
                chara_row("ui_chara_mecha", Some(6)),
                chara_row("ui_chara_latiicebeam", Some(-1)),
                chara_row("ui_chara_nothing", None),
                chara_row("ui_chara_mecha", Some(7)),
            ]),
        )]);
        let orders = prc_disp_orders(&write(&root)).unwrap();
        assert_eq!(
            orders,
            vec![
                (hash40("ui_chara_mario"), 0),
                (hash40("ui_chara_mecha"), 6),
                (hash40("ui_chara_latiicebeam"), -1),
            ]
        );
    }

    #[test]
    fn a_row_listed_twice_keeps_its_first_order_and_is_reported() {
        let root = Param::Struct(vec![(
            hash40("db_root"),
            Param::List(vec![
                chara_row("ui_chara_vegeta", Some(115)),
                chara_row("ui_chara_mario", Some(0)),
                chara_row("ui_chara_vegeta", Some(-1)),
            ]),
        )]);
        let bytes = write(&root);
        assert_eq!(
            prc_disp_orders(&bytes).unwrap(),
            vec![(hash40("ui_chara_vegeta"), 115), (hash40("ui_chara_mario"), 0)]
        );
        assert_eq!(
            prc_duplicate_rows(&bytes).unwrap(),
            vec![(hash40("ui_chara_vegeta"), "x".to_string(), 2)]
        );
    }

    #[test]
    fn a_file_that_is_not_a_chara_db_gives_nothing_or_an_error() {
        let other = Param::Struct(vec![(hash40("db_root"), Param::List(vec![Param::Struct(vec![(hash40("ui_layout_id"), Param::Hash(1))])]))]);
        assert_eq!(prc_disp_orders(&write(&other)).unwrap(), vec![]);
        assert_eq!(prc_disp_orders(&write(&Param::Struct(vec![]))).unwrap(), vec![]);
        assert!(prc_disp_orders(b"not a param file").is_err());
    }

    #[test]
    fn parses_the_csk_schema_with_labels_and_plain_values() {
        let text = r#"{
            "series_database_entries": [{"ui_series_id": "ui_series_brainage", "disp_order": 0}],
            "character_database_entries": [
                {"ui_chara_id": "ui_chara_kawashima", "disp_order": 12},
                {"ui_chara_id": "0x0FEDCBA987", "disp_order": -1},
                {"ui_chara_id": "ui_chara_mecha", "name_id": "mecha"}
            ]
        }"#;
        let orders = chara_disp_orders(text).unwrap();
        assert_eq!(
            orders.orders,
            vec![(hash40("ui_chara_kawashima"), 12), (0xfedcba987, -1)]
        );
        assert!(orders.refused.is_empty());
    }

    #[test]
    fn a_later_entry_for_the_same_row_wins() {
        let text = r#"{"character_database_entries": [
            {"ui_chara_id": "ui_chara_x", "disp_order": 3},
            {"ui_chara_id": "ui_chara_x", "disp_order": 9}
        ]}"#;
        assert_eq!(chara_disp_orders(text).unwrap().orders, vec![(hash40("ui_chara_x"), 9)]);
    }

    #[test]
    fn accepts_tagged_values_and_null() {
        let text = r#"{"character_database_entries": [
            {"ui_chara_id": "ui_chara_a", "disp_order": {"Overwrite": 5}},
            {"ui_chara_id": "ui_chara_b", "disp_order": {"Optional": 6}},
            {"ui_chara_id": "ui_chara_c", "disp_order": null}
        ]}"#;
        assert_eq!(
            chara_disp_orders(text).unwrap().orders,
            vec![(hash40("ui_chara_a"), 5), (hash40("ui_chara_b"), 6)]
        );
    }

    #[test]
    fn out_of_range_and_fractional_orders_are_refused_not_wrapped() {
        let text = r#"{"character_database_entries": [
            {"ui_chara_id": "ui_chara_a", "disp_order": 200},
            {"ui_chara_id": "ui_chara_b", "disp_order": 1.5},
            {"disp_order": 4}
        ]}"#;
        let orders = chara_disp_orders(text).unwrap();
        assert!(orders.orders.is_empty());
        assert_eq!(orders.refused.len(), 3);
    }

    #[test]
    fn files_without_character_entries_give_nothing() {
        assert_eq!(chara_disp_orders("{}").unwrap(), CharaOrders::default());
        assert_eq!(chara_disp_orders("[1, 2]").unwrap(), CharaOrders::default());
    }

    #[test]
    fn strict_like_serde_json() {
        assert!(parse(r#"{"a": 1,}"#).is_err());
        assert!(parse(r#"{"a": 1} x"#).is_err());
        assert!(parse(r#"{'a': 1}"#).is_err());
        assert!(parse("{\"a\": \"line\nbreak\"}").is_err());
        assert!(parse(r#"{"a": 01}"#).is_err());
        assert!(parse(r#"{"a": -0}"#).is_ok());
    }

    #[test]
    fn strings_numbers_and_bom() {
        let value = parse("\u{feff}{\"s\": \"a\\\"b\\\\c\\u00e9\\ud83d\\ude00\", \"n\": -1.25e2, \"t\": [true, false, null]}").unwrap();
        assert_eq!(value.get("s"), Some(&Json::String("a\"b\\c\u{e9}\u{1f600}".to_string())));
        assert_eq!(value.get("n"), Some(&Json::Number(-125.0)));
        assert_eq!(
            value.get("t"),
            Some(&Json::Array(vec![Json::Bool(true), Json::Bool(false), Json::Null]))
        );
    }

    #[test]
    fn depth_is_bounded() {
        let deep = "[".repeat(MAX_DEPTH + 2) + &"]".repeat(MAX_DEPTH + 2);
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn real_pack_files_parse() {
        let springtrap = r#"{
    "series_database_entries": [
        {
            "ui_series_id": "ui_series_five_nights_at_freddys",
            "clone_from_series_id": "ui_series_mario",
            "name_id": "five_nights_at_freddys",
            "disp_order": 0,
            "disp_order_sound": 100,
            "dlc_chara_id": "",
            "is_use_amiibo_bg": false
        }
    ],
    "gametitle_database_entries": [
        {"ui_gametitle_id": "ui_gametitle_x", "test_disp_order": 15000, "record_type": "record_original"}
    ]
}"#;
        assert_eq!(chara_disp_orders(springtrap).unwrap(), CharaOrders::default());
    }
}
