use crate::prcxml::{ItemRow, Skipped, Value, Why};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RowKind {
    F32,
    I32,
    Bool,
    Hash40,
    Label,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RowField {
    pub hash: u64,
    pub offset: usize,
    pub kind: RowKind,
    pub name: &'static str,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RowValue {
    F32(f32),
    I32(i32),
    Bool(bool),
    Hash(u64),
}

pub const TAG_F32: u32 = 0;
pub const TAG_I32: u32 = 1;
pub const TAG_BOOL: u32 = 2;
pub const TAG_HASH: u32 = 3;
pub const TAG_MUL: u32 = 4;
pub const TAG_LABEL: u32 = 5;

pub fn scaled_word(bits: u32, float: bool, factor: f32) -> u32 {
    if float {
        return (f32::from_bits(bits) * factor).to_bits();
    }
    (bits as i32 as f32 * factor).round() as i32 as u32
}

pub fn multiplied(field: &RowField, base: &[u8], factor: f32) -> Option<RowValue> {
    let cells = base.get(field.offset..field.offset + 4)?;
    let bytes = [cells[0], cells[1], cells[2], cells[3]];
    match field.kind {
        RowKind::F32 => Some(RowValue::F32(f32::from_le_bytes(bytes) * factor)),
        RowKind::I32 => Some(RowValue::I32((i32::from_le_bytes(bytes) as f32 * factor).round() as i32)),
        _ => None,
    }
}

pub fn value_from_tag(tag: u32, bits: u64) -> Option<Value> {
    match tag {
        TAG_F32 => Some(Value::Float(f32::from_bits(bits as u32) as f64)),
        TAG_I32 => Some(Value::Int(bits as u32 as i32 as i64)),
        TAG_BOOL => Some(Value::Bool(bits != 0)),
        TAG_HASH => Some(Value::Hash(bits)),
        _ => None,
    }
}

impl RowValue {
    pub fn write(self, row: &mut [u8], offset: usize) -> bool {
        let bytes: Vec<u8> = match self {
            RowValue::F32(value) => value.to_le_bytes().to_vec(),
            RowValue::I32(value) => value.to_le_bytes().to_vec(),
            RowValue::Bool(value) => vec![u8::from(value)],
            RowValue::Hash(value) => value.to_le_bytes().to_vec(),
        };
        match row.get_mut(offset..offset + bytes.len()) {
            Some(cells) => {
                cells.copy_from_slice(&bytes);
                true
            }
            None => false,
        }
    }
}

pub mod enemy {
    include!("enemy_common_row_table.rs");
}

pub mod cpu {
    include!("cpu_param_row_table.rs");
}

pub fn find(fields: &'static [RowField], hash: u64) -> Option<&'static RowField> {
    fields
        .binary_search_by_key(&hash, |field| field.hash)
        .ok()
        .map(|index| &fields[index])
}

pub fn shown(fields: &'static [RowField], hash: u64) -> String {
    match find(fields, hash) {
        Some(field) if !field.name.is_empty() => field.name.to_string(),
        _ => format!("{hash:#x}"),
    }
}

pub fn convert(field: &RowField, value: &Value) -> Result<RowValue, Why> {
    match (field.kind, value) {
        (RowKind::Label, _) => Err(Why::Label),
        (_, Value::Text(_)) => Err(Why::Text),
        (RowKind::F32, Value::Float(value)) => Ok(RowValue::F32(*value as f32)),
        (RowKind::F32, Value::Int(value)) => Ok(RowValue::F32(*value as f32)),
        (RowKind::I32, Value::Int(value)) => Ok(RowValue::I32((*value).clamp(i32::MIN as i64, i32::MAX as i64) as i32)),
        (RowKind::I32, Value::Float(value)) => Ok(RowValue::I32(value.round() as i32)),
        (RowKind::I32, Value::Bool(value)) => Ok(RowValue::I32(i32::from(*value))),
        (RowKind::Bool, Value::Bool(value)) => Ok(RowValue::Bool(*value)),
        (RowKind::Bool, Value::Int(value)) => Ok(RowValue::Bool(*value != 0)),
        (RowKind::Hash40, Value::Hash(value)) => Ok(RowValue::Hash(*value)),
        _ => Err(Why::Type),
    }
}

pub fn set(fields: &'static [RowField], hash: u64, value: &Value) -> Result<(usize, RowValue), Why> {
    let field = find(fields, hash).ok_or(Why::Unknown)?;
    convert(field, value).map(|converted| (field.offset, converted))
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RowChanges {
    pub values: Vec<(usize, RowValue)>,
    pub skipped: Vec<Skipped>,
    pub rows: usize,
}

pub fn changes(fields: &'static [RowField], row: &ItemRow) -> RowChanges {
    let mut out = RowChanges {
        skipped: row.skipped.clone(),
        rows: row.rows,
        ..Default::default()
    };
    for (hash, value) in &row.fields {
        match set(fields, *hash, value) {
            Ok((offset, converted)) => {
                out.values.retain(|(known, _)| *known != offset);
                out.values.push((offset, converted));
            }
            Err(why) => out.skipped.push(Skipped {
                field: shown(fields, *hash),
                why,
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash40;

    #[test]
    fn a_word_scales_as_a_float_or_a_rounded_integer() {
        assert_eq!(f32::from_bits(scaled_word(2.5f32.to_bits(), true, 1.5)), 3.75);
        assert_eq!(scaled_word(600, false, 1.5), 900);
        assert_eq!(scaled_word(7, false, 0.5) as i32, 4);
        assert_eq!(scaled_word(-3i32 as u32, false, 2.0) as i32, -6);
    }

    fn width(kind: RowKind) -> usize {
        match kind {
            RowKind::Hash40 => 8,
            RowKind::Bool => 1,
            _ => 4,
        }
    }

    fn check(fields: &[RowField], size: usize) {
        assert!(fields.windows(2).all(|pair| pair[0].hash < pair[1].hash));
        let mut cells = vec![false; size];
        for field in fields {
            for cell in &mut cells[field.offset..field.offset + width(field.kind)] {
                assert!(!*cell, "{} overlaps", field.name);
                *cell = true;
            }
            if !field.name.is_empty() {
                assert_eq!(hash40(field.name), field.hash);
            }
        }
    }

    #[test]
    fn both_tables_are_sorted_and_never_overlap() {
        check(&enemy::FIELDS, enemy::ROW_SIZE);
        check(&cpu::FIELDS, cpu::ROW_SIZE);
        assert_eq!(enemy::FIELDS.len(), 138);
        assert_eq!(cpu::FIELDS.len(), 67);
        assert_eq!(find(&cpu::FIELDS, hash40("width")).map(|field| field.offset), Some(0x6c));
        assert_eq!(find(&enemy::FIELDS, hash40("egg_scale")).map(|field| field.offset), Some(0x80));
    }

    #[test]
    fn values_fit_the_field_or_are_left_out() {
        let width = find(&cpu::FIELDS, hash40("width")).unwrap();
        assert_eq!(convert(width, &Value::Int(12)), Ok(RowValue::F32(12.0)));
        assert_eq!(convert(width, &Value::Hash(1)), Err(Why::Type));
        let final_type = find(&cpu::FIELDS, hash40("final_type")).unwrap();
        assert_eq!(convert(final_type, &Value::Hash(1)), Err(Why::Label));
        let hp = find(&enemy::FIELDS, hash40("hp")).unwrap();
        assert_eq!(convert(hp, &Value::Float(40.6)), Ok(RowValue::I32(41)));
        let mut row = vec![0u8; enemy::ROW_SIZE];
        assert!(RowValue::I32(41).write(&mut row, hp.offset));
        assert_eq!(&row[hp.offset..hp.offset + 4], &41i32.to_le_bytes());
        assert!(!RowValue::Hash(1).write(&mut row, enemy::ROW_SIZE - 4));
        assert_eq!(set(&cpu::FIELDS, hash40("no_such_field"), &Value::Int(1)), Err(Why::Unknown));
        assert_eq!(value_from_tag(TAG_F32, 2.5f32.to_bits() as u64), Some(Value::Float(2.5)));
    }

    #[test]
    fn a_factor_scales_the_base_value_of_number_fields_only() {
        let mut base = vec![0u8; cpu::ROW_SIZE];
        let width = find(&cpu::FIELDS, hash40("width")).unwrap();
        RowValue::F32(3.0).write(&mut base, width.offset);
        assert_eq!(multiplied(width, &base, 1.5), Some(RowValue::F32(4.5)));
        let frames = find(&cpu::FIELDS, hash40("down_attack_frame")).unwrap();
        RowValue::I32(15).write(&mut base, frames.offset);
        assert_eq!(multiplied(frames, &base, 1.5), Some(RowValue::I32(23)));
        let flag = find(&cpu::FIELDS, hash40("final_delayed")).unwrap();
        assert_eq!(multiplied(flag, &base, 2.0), None);
    }
}
