use std::collections::HashMap;

pub const MAGIC: &[u8; 8] = b"paracobn";
pub const MAX_DEPTH: usize = 64;

const TAG_BOOL: u8 = 1;
const TAG_I8: u8 = 2;
const TAG_U8: u8 = 3;
const TAG_I16: u8 = 4;
const TAG_U16: u8 = 5;
const TAG_I32: u8 = 6;
const TAG_U32: u8 = 7;
const TAG_FLOAT: u8 = 8;
const TAG_HASH: u8 = 9;
const TAG_STR: u8 = 10;
const TAG_LIST: u8 = 11;
const TAG_STRUCT: u8 = 12;

#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    Float(f32),
    Hash(u64),
    Str(String),
    List(Vec<Param>),
    Struct(Vec<(u64, Param)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrcError {
    Magic,
    Truncated(usize),
    BadTag { tag: u8, at: usize },
    BadHashIndex(u32),
    BadString(u32),
    RootNotStruct,
    TooDeep,
}

impl Param {
    pub fn get(&self, key: u64) -> Option<&Param> {
        match self {
            Param::Struct(fields) => fields.iter().find(|(k, _)| *k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: u64) -> Option<&mut Param> {
        match self {
            Param::Struct(fields) => fields.iter_mut().find(|(k, _)| *k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn set(&mut self, key: u64, value: Param) -> bool {
        match self {
            Param::Struct(fields) => {
                match fields.iter_mut().find(|(k, _)| *k == key) {
                    Some(slot) => slot.1 = value,
                    None => fields.push((key, value)),
                }
                true
            }
            _ => false,
        }
    }

    pub fn list(&self) -> Option<&Vec<Param>> {
        match self {
            Param::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn list_mut(&mut self) -> Option<&mut Vec<Param>> {
        match self {
            Param::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Param::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn hash(&self) -> Option<u64> {
        match self {
            Param::Hash(value) => Some(*value),
            _ => None,
        }
    }

    pub fn integer(&self) -> Option<i64> {
        Some(match self {
            Param::Bool(value) => *value as i64,
            Param::I8(value) => *value as i64,
            Param::U8(value) => *value as i64,
            Param::I16(value) => *value as i64,
            Param::U16(value) => *value as i64,
            Param::I32(value) => *value as i64,
            Param::U32(value) => *value as i64,
            _ => return None,
        })
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    hashes: Vec<u64>,
    refs: usize,
    params: usize,
}

impl<'a> Reader<'a> {
    fn need(&self, at: usize, length: usize) -> Result<&'a [u8], PrcError> {
        self.bytes
            .get(at..at.checked_add(length).ok_or(PrcError::Truncated(at))?)
            .ok_or(PrcError::Truncated(at))
    }

    fn u32_at(&self, at: usize) -> Result<u32, PrcError> {
        let raw = self.need(at, 4)?;
        Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    fn hash_at(&self, index: u32) -> Result<u64, PrcError> {
        self.hashes
            .get(index as usize)
            .copied()
            .ok_or(PrcError::BadHashIndex(index))
    }

    fn string_at(&self, offset: u32) -> Result<String, PrcError> {
        let start = self
            .refs
            .checked_add(offset as usize)
            .filter(|start| *start < self.params)
            .ok_or(PrcError::BadString(offset))?;
        let end = self.bytes[start..self.params]
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(PrcError::BadString(offset))?;
        Ok(String::from_utf8_lossy(&self.bytes[start..start + end]).into_owned())
    }

    fn param(&self, at: usize, depth: usize) -> Result<Param, PrcError> {
        if depth > MAX_DEPTH {
            return Err(PrcError::TooDeep);
        }
        let tag = *self.need(at, 1)?.first().unwrap_or(&0);
        let body = at + 1;
        Ok(match tag {
            TAG_BOOL => Param::Bool(self.need(body, 1)?[0] != 0),
            TAG_I8 => Param::I8(self.need(body, 1)?[0] as i8),
            TAG_U8 => Param::U8(self.need(body, 1)?[0]),
            TAG_I16 => {
                let raw = self.need(body, 2)?;
                Param::I16(i16::from_le_bytes([raw[0], raw[1]]))
            }
            TAG_U16 => {
                let raw = self.need(body, 2)?;
                Param::U16(u16::from_le_bytes([raw[0], raw[1]]))
            }
            TAG_I32 => Param::I32(self.u32_at(body)? as i32),
            TAG_U32 => Param::U32(self.u32_at(body)?),
            TAG_FLOAT => Param::Float(f32::from_bits(self.u32_at(body)?)),
            TAG_HASH => Param::Hash(self.hash_at(self.u32_at(body)?)?),
            TAG_STR => Param::Str(self.string_at(self.u32_at(body)?)?),
            TAG_LIST => {
                let count = self.u32_at(body)? as usize;
                self.need(body + 4, count.checked_mul(4).ok_or(PrcError::Truncated(body))?)?;
                let mut items = Vec::with_capacity(count);
                for index in 0..count {
                    let offset = self.u32_at(body + 4 + index * 4)? as usize;
                    items.push(self.param(at + offset, depth + 1)?);
                }
                Param::List(items)
            }
            TAG_STRUCT => {
                let count = self.u32_at(body)? as usize;
                let table = self.refs + self.u32_at(body + 4)? as usize;
                self.need(table, count.checked_mul(8).ok_or(PrcError::Truncated(table))?)?;
                let mut entries = Vec::with_capacity(count);
                for index in 0..count {
                    let key = self.hash_at(self.u32_at(table + index * 8)?)?;
                    let offset = self.u32_at(table + index * 8 + 4)? as usize;
                    entries.push((offset, key));
                }
                entries.sort();
                let mut fields = Vec::with_capacity(count);
                for (offset, key) in entries {
                    fields.push((key, self.param(at + offset, depth + 1)?));
                }
                Param::Struct(fields)
            }
            other => return Err(PrcError::BadTag { tag: other, at }),
        })
    }
}

pub fn parse(bytes: &[u8]) -> Result<Param, PrcError> {
    if bytes.len() < 0x10 || &bytes[..8] != MAGIC {
        return Err(PrcError::Magic);
    }
    let hash_size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let ref_size = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    let refs = 0x10usize
        .checked_add(hash_size)
        .ok_or(PrcError::Truncated(0x10))?;
    let params = refs.checked_add(ref_size).ok_or(PrcError::Truncated(refs))?;
    if params > bytes.len() || hash_size % 8 != 0 {
        return Err(PrcError::Truncated(params));
    }
    let hashes = bytes[0x10..refs]
        .chunks_exact(8)
        .map(|chunk| {
            u64::from_le_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ])
        })
        .collect();
    let reader = Reader {
        bytes,
        hashes,
        refs,
        params,
    };
    let root = reader.param(params, 0)?;
    match root {
        Param::Struct(_) => Ok(root),
        _ => Err(PrcError::RootNotStruct),
    }
}

struct Writer {
    hashes: Vec<u64>,
    hash_index: HashMap<u64, u32>,
    refs: Vec<u8>,
    tables: HashMap<Vec<u8>, u32>,
    string_index: HashMap<String, u32>,
    params: Vec<u8>,
}

fn size(param: &Param) -> usize {
    match param {
        Param::Bool(_) | Param::I8(_) | Param::U8(_) => 2,
        Param::I16(_) | Param::U16(_) => 3,
        Param::I32(_) | Param::U32(_) | Param::Float(_) | Param::Hash(_) | Param::Str(_) => 5,
        Param::List(items) => 5 + items.len() * 4 + items.iter().map(size).sum::<usize>(),
        Param::Struct(fields) => 9 + fields.iter().map(|(_, value)| size(value)).sum::<usize>(),
    }
}

impl Writer {
    fn intern(&mut self, hash: u64) -> u32 {
        if let Some(index) = self.hash_index.get(&hash) {
            return *index;
        }
        let index = self.hashes.len() as u32;
        self.hashes.push(hash);
        self.hash_index.insert(hash, index);
        index
    }

    fn string(&mut self, text: &str) -> u32 {
        if let Some(offset) = self.string_index.get(text) {
            return *offset;
        }
        let offset = self.refs.len() as u32;
        self.refs.extend_from_slice(text.as_bytes());
        self.refs.push(0);
        self.string_index.insert(text.to_string(), offset);
        offset
    }

    fn table(&mut self, table: Vec<u8>) -> u32 {
        if let Some(offset) = self.tables.get(&table) {
            return *offset;
        }
        let offset = self.refs.len() as u32;
        self.refs.extend_from_slice(&table);
        self.tables.insert(table, offset);
        offset
    }

    fn put_u32(&mut self, value: u32) {
        self.params.extend_from_slice(&value.to_le_bytes());
    }

    fn patch_u32(&mut self, at: usize, value: u32) {
        self.params[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn param(&mut self, param: &Param) {
        let at = self.params.len();
        match param {
            Param::Bool(value) => self.params.extend_from_slice(&[TAG_BOOL, *value as u8]),
            Param::I8(value) => self.params.extend_from_slice(&[TAG_I8, *value as u8]),
            Param::U8(value) => self.params.extend_from_slice(&[TAG_U8, *value]),
            Param::I16(value) => {
                self.params.push(TAG_I16);
                self.params.extend_from_slice(&value.to_le_bytes());
            }
            Param::U16(value) => {
                self.params.push(TAG_U16);
                self.params.extend_from_slice(&value.to_le_bytes());
            }
            Param::I32(value) => {
                self.params.push(TAG_I32);
                self.put_u32(*value as u32);
            }
            Param::U32(value) => {
                self.params.push(TAG_U32);
                self.put_u32(*value);
            }
            Param::Float(value) => {
                self.params.push(TAG_FLOAT);
                self.put_u32(value.to_bits());
            }
            Param::Hash(value) => {
                let index = self.intern(*value);
                self.params.push(TAG_HASH);
                self.put_u32(index);
            }
            Param::Str(text) => {
                let offset = self.string(text);
                self.params.push(TAG_STR);
                self.put_u32(offset);
            }
            Param::List(items) => {
                self.params.push(TAG_LIST);
                self.put_u32(items.len() as u32);
                let table = self.params.len();
                self.params.resize(table + items.len() * 4, 0);
                for (index, item) in items.iter().enumerate() {
                    let offset = (self.params.len() - at) as u32;
                    self.patch_u32(table + index * 4, offset);
                    self.param(item);
                }
            }
            Param::Struct(fields) => {
                let mut placed = Vec::with_capacity(fields.len());
                let mut offset = 9usize;
                for (key, value) in fields {
                    placed.push((*key, offset as u32));
                    offset += size(value);
                }
                placed.sort_by_key(|(key, _)| *key);
                let mut table = Vec::with_capacity(placed.len() * 8);
                for (key, at_offset) in placed {
                    let hash = self.intern(key);
                    table.extend_from_slice(&hash.to_le_bytes());
                    table.extend_from_slice(&at_offset.to_le_bytes());
                }
                let slot = self.table(table);
                self.params.push(TAG_STRUCT);
                self.put_u32(fields.len() as u32);
                self.put_u32(slot);
                for (_, value) in fields {
                    self.param(value);
                }
                debug_assert_eq!(self.params.len() - at, offset);
            }
        }
    }
}

pub fn write(root: &Param) -> Vec<u8> {
    let mut writer = Writer {
        hashes: vec![0],
        hash_index: HashMap::from([(0, 0)]),
        refs: Vec::new(),
        tables: HashMap::new(),
        string_index: HashMap::new(),
        params: Vec::with_capacity(size(root)),
    };
    writer.param(root);
    let hash_size = writer.hashes.len() * 8;
    let ref_size = writer.refs.len();
    let mut out = Vec::with_capacity(0x10 + hash_size + ref_size + writer.params.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(hash_size as u32).to_le_bytes());
    out.extend_from_slice(&(ref_size as u32).to_le_bytes());
    for hash in &writer.hashes {
        out.extend_from_slice(&hash.to_le_bytes());
    }
    out.extend_from_slice(&writer.refs);
    out.extend_from_slice(&writer.params);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash40;

    fn sample() -> Param {
        Param::Struct(vec![(
            hash40("db_root"),
            Param::List(vec![
                Param::Struct(vec![
                    (hash40("ui_mii_hat_id"), Param::Hash(hash40("ui_mii_hat_none"))),
                    (hash40("save_no"), Param::I16(-1)),
                    (hash40("mii_hat_id"), Param::Str("none".into())),
                    (hash40("valid_resource"), Param::I8(1)),
                    (hash40("swing_enabled"), Param::Bool(false)),
                    (hash40("scale"), Param::Float(1.5)),
                    (hash40("shop_price"), Param::U32(450)),
                    (hash40("normal_suit"), Param::U8(1)),
                    (hash40("disp_order"), Param::I32(7)),
                    (hash40("color"), Param::U16(9)),
                    (hash40("shop_item_tag"), Param::Hash(0)),
                    (
                        hash40("transforms"),
                        Param::List(vec![Param::Struct(vec![
                            (hash40("hair_type"), Param::I32(-1)),
                            (hash40("t_x"), Param::Float(0.0)),
                        ])]),
                    ),
                ]),
                Param::Struct(vec![
                    (hash40("ui_mii_hat_id"), Param::Hash(hash40("ui_mii_hat_silkhat"))),
                    (hash40("save_no"), Param::I16(8)),
                    (hash40("mii_hat_id"), Param::Str("silkhat".into())),
                ]),
            ]),
        )])
    }

    #[test]
    fn writes_what_it_reads() {
        let root = sample();
        let bytes = write(&root);
        assert_eq!(&bytes[..8], MAGIC);
        assert_eq!(parse(&bytes).unwrap(), root);
    }

    #[test]
    fn struct_entries_are_sorted_by_hash() {
        let bytes = write(&sample());
        let hash_size = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let hashes: Vec<u64> = bytes[0x10..0x10 + hash_size]
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let refs = 0x10 + hash_size;
        let root_count = 1;
        let first = u32::from_le_bytes(bytes[refs..refs + 4].try_into().unwrap());
        assert_eq!(hashes[first as usize], hash40("db_root"));
        let row_table = refs + root_count * 8;
        let keys: Vec<u64> = (0..12)
            .map(|i| {
                let index = u32::from_le_bytes(
                    bytes[row_table + i * 8..row_table + i * 8 + 4].try_into().unwrap(),
                );
                hashes[index as usize]
            })
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn the_zero_hash_is_entry_zero() {
        let bytes = write(&sample());
        assert_eq!(u64::from_le_bytes(bytes[0x10..0x18].try_into().unwrap()), 0);
    }

    #[test]
    fn strings_are_shared() {
        let root = Param::Struct(vec![
            (hash40("a"), Param::Str("same".into())),
            (hash40("b"), Param::Str("same".into())),
        ]);
        let bytes = write(&root);
        let text = bytes.windows(5).filter(|w| w == b"same\0").count();
        assert_eq!(text, 1);
        assert_eq!(parse(&bytes).unwrap(), root);
    }

    #[test]
    fn rows_with_the_same_keys_share_one_key_table() {
        let row = |speed: f32| {
            let mut fields = vec![(hash40("speed"), Param::Float(speed)), (hash40("life"), Param::U8(3))];
            fields.sort_by_key(|(key, _)| *key);
            Param::Struct(fields)
        };
        let root = Param::Struct(vec![(hash40("rows"), Param::List((0..50).map(|i| row(i as f32)).collect()))]);
        let bytes = write(&root);
        let ref_size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        assert_eq!(ref_size, 8 + 2 * 8);
        assert_eq!(parse(&bytes).unwrap(), root);
    }

    #[test]
    fn refuses_what_is_not_a_param_file() {
        assert_eq!(parse(b"not a param file"), Err(PrcError::Magic));
        let mut bytes = write(&sample());
        bytes.truncate(bytes.len() - 3);
        assert!(parse(&bytes).is_err());
    }

    #[test]
    fn edits_survive_a_round_trip() {
        let mut root = sample();
        let rows = root.get_mut(hash40("db_root")).unwrap().list_mut().unwrap();
        let mut copy = rows[1].clone();
        copy.set(hash40("save_no"), Param::I16(133));
        copy.set(hash40("mii_hat_id"), Param::Str("tophat2".into()));
        rows.push(copy);
        let read = parse(&write(&root)).unwrap();
        let rows = read.get(hash40("db_root")).unwrap().list().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[2].get(hash40("save_no")), Some(&Param::I16(133)));
        assert_eq!(rows[2].get(hash40("mii_hat_id")).and_then(Param::text), Some("tophat2"));
    }

    #[test]
    fn reads_and_rewrites_a_real_database_when_one_is_given() {
        let Ok(path) = std::env::var("CLONE_ENGINE_PRC_SAMPLE") else {
            return;
        };
        for file in path.split(';').filter(|p| !p.is_empty()) {
            let bytes = std::fs::read(file).unwrap();
            let root = parse(&bytes).unwrap();
            let again = write(&root);
            eprintln!("{file}: {} bytes, rewritten {}", bytes.len(), again.len());
            assert_eq!(parse(&again).unwrap(), root, "{file}");
        }
    }
}
