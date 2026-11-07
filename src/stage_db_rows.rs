#![allow(dead_code)]

use std::sync::Mutex;

#[cfg(test)]
#[path = "stage_ledger.rs"]
mod stage_ledger;
#[cfg(not(test))]
use crate::stage_ledger::hash40;
#[cfg(test)]
use stage_ledger::hash40;

pub fn db_root_key() -> u64 {
    hash40("db_root")
}
pub fn disp_order_key() -> u64 {
    hash40("disp_order")
}
pub fn ui_stage_id_key() -> u64 {
    hash40("ui_stage_id")
}

pub const TAG_BOOL: u8 = 1;
pub const TAG_I8: u8 = 2;
pub const TAG_U8: u8 = 3;
pub const TAG_I16: u8 = 4;
pub const TAG_U16: u8 = 5;
pub const TAG_I32: u8 = 6;
pub const TAG_U32: u8 = 7;
pub const TAG_HASH40: u8 = 9;
pub const TAG_STR: u8 = 10;
pub const TAG_LIST: u8 = 11;
pub const TAG_STRUCT: u8 = 12;

pub fn payload_width(tag: u8) -> Option<usize> {
    Some(match tag {
        TAG_BOOL | TAG_I8 | TAG_U8 => 1,
        TAG_I16 | TAG_U16 => 2,
        TAG_I32 | TAG_U32 | TAG_HASH40 | TAG_STR => 4,
        8 => 4,
        _ => return None,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum RowError {
    DatabaseNotLoaded,
    RootNotStruct(u8),
    NoDbRoot,
    DbRootNotList(u8),
    RowNotFound(u64),
    FieldMissing(u64),
    UnwritableTag(u8),
    WidthWouldChange { from: u8, to: u8 },
    ValueOutOfRange(u32),
    VerificationFailed { wrote: u32, read: i64 },
}

#[derive(Debug, Clone, Copy)]
pub struct ParamTree {
    pub root: usize,
    pub hashes: usize,
    pub refs: usize,
}

impl ParamTree {
    unsafe fn u8_at(&self, at: usize) -> u8 {
        core::ptr::read_unaligned(at as *const u8)
    }
    unsafe fn u32_at(&self, at: usize) -> u32 {
        core::ptr::read_unaligned(at as *const u32)
    }

    pub unsafe fn tag(&self, node: usize) -> u8 {
        self.u8_at(node)
    }

    pub unsafe fn struct_entries(&self, node: usize) -> Result<Vec<(u64, usize)>, RowError> {
        let tag = self.tag(node);
        if tag != TAG_STRUCT {
            return Err(RowError::RootNotStruct(tag));
        }
        let count = self.u32_at(node + 1) as usize;
        let ref_offset = self.u32_at(node + 5) as usize;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let entry = self.refs + ref_offset + i * 8;
            let hash_index = self.u32_at(entry) as usize;
            let param_offset = self.u32_at(entry + 4) as usize;
            let key = core::ptr::read_unaligned((self.hashes + hash_index * 8) as *const u64);
            out.push((key, node + param_offset));
        }
        Ok(out)
    }

    pub unsafe fn list_elements(&self, node: usize) -> Result<Vec<usize>, RowError> {
        let tag = self.tag(node);
        if tag != TAG_LIST {
            return Err(RowError::DbRootNotList(tag));
        }
        let count = self.u32_at(node + 1) as usize;
        Ok((0..count)
            .map(|i| node + self.u32_at(node + 5 + i * 4) as usize)
            .collect())
    }

    pub unsafe fn field(&self, node: usize, key: u64) -> Option<usize> {
        self.struct_entries(node)
            .ok()?
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, child)| child)
    }

    pub unsafe fn scalar(&self, node: usize) -> Option<i64> {
        let tag = self.tag(node);
        let at = node + 1;
        Some(match tag {
            TAG_BOOL | TAG_U8 => self.u8_at(at) as i64,
            TAG_I8 => self.u8_at(at) as i8 as i64,
            TAG_I16 => core::ptr::read_unaligned(at as *const i16) as i64,
            TAG_U16 => core::ptr::read_unaligned(at as *const u16) as i64,
            TAG_I32 => core::ptr::read_unaligned(at as *const i32) as i64,
            TAG_U32 => self.u32_at(at) as i64,
            _ => return None,
        })
    }

    pub unsafe fn hash_value(&self, node: usize) -> Option<u64> {
        if self.tag(node) != TAG_HASH40 {
            return None;
        }
        let index = self.u32_at(node + 1) as usize;
        Some(core::ptr::read_unaligned(
            (self.hashes + index * 8) as *const u64,
        ))
    }

    pub unsafe fn rows(&self) -> Result<Vec<usize>, RowError> {
        let tag = self.tag(self.root);
        if tag != TAG_STRUCT {
            return Err(RowError::RootNotStruct(tag));
        }
        let list = self
            .field(self.root, db_root_key())
            .ok_or(RowError::NoDbRoot)?;
        self.list_elements(list)
    }

    pub unsafe fn find_row(&self, ui_stage_id: u64) -> Result<usize, RowError> {
        for row in self.rows()? {
            if let Some(field) = self.field(row, ui_stage_id_key()) {
                if self.hash_value(field) == Some(ui_stage_id) {
                    return Ok(row);
                }
            }
        }
        Err(RowError::RowNotFound(ui_stage_id))
    }

    pub unsafe fn write_scalar(
        &self,
        node: usize,
        new_tag: u8,
        value: u32,
    ) -> Result<(), RowError> {
        let old_tag = self.tag(node);
        let old_width = payload_width(old_tag).ok_or(RowError::UnwritableTag(old_tag))?;
        let new_width = payload_width(new_tag).ok_or(RowError::UnwritableTag(new_tag))?;
        if old_width != new_width {
            return Err(RowError::WidthWouldChange {
                from: old_tag,
                to: new_tag,
            });
        }
        match new_width {
            1 => {
                if value > u8::MAX as u32 {
                    return Err(RowError::ValueOutOfRange(value));
                }
                core::ptr::write_volatile((node + 1) as *mut u8, value as u8);
            }
            2 => {
                if value > u16::MAX as u32 {
                    return Err(RowError::ValueOutOfRange(value));
                }
                core::ptr::write_unaligned((node + 1) as *mut u16, value as u16);
            }
            _ => core::ptr::write_unaligned((node + 1) as *mut u32, value),
        }
        core::ptr::write_volatile(node as *mut u8, new_tag);
        Ok(())
    }
}

pub unsafe fn set_disp_order(
    tree: &ParamTree,
    ui_stage_id: u64,
    value: u16,
) -> Result<(), RowError> {
    if value > u8::MAX as u16 {
        return Err(RowError::ValueOutOfRange(value as u32));
    }
    let row = tree.find_row(ui_stage_id)?;
    let field = tree
        .field(row, disp_order_key())
        .ok_or(RowError::FieldMissing(disp_order_key()))?;
    let tag = if value <= i8::MAX as u16 {
        TAG_I8
    } else {
        TAG_U8
    };
    tree.write_scalar(field, tag, value as u32)?;

    match tree.scalar(field) {
        Some(read) if read == value as i64 => Ok(()),
        other => Err(RowError::VerificationFailed {
            wrote: value as u32,
            read: other.unwrap_or(-1),
        }),
    }
}

static PENDING: Mutex<Vec<(u64, u16)>> = Mutex::new(Vec::new());

#[cfg(not(test))]
pub unsafe fn listed_count() -> Option<usize> {
    let tree = resolve()?;
    let rows = tree.rows().ok()?;
    Some(
        rows.into_iter()
            .filter(|row| {
                tree.field(*row, disp_order_key())
                    .and_then(|field| tree.scalar(field))
                    .map_or(false, |value| value >= 0)
            })
            .count(),
    )
}

#[cfg(not(test))]
pub unsafe fn resolve() -> Option<ParamTree> {
    let global = core::ptr::read_volatile((crate::game::addr(0x532E730)) as *const usize);
    if global == 0 {
        return None;
    }
    let manager = core::ptr::read_volatile((global + 0x8) as *const usize);
    if manager == 0 {
        return None;
    }
    let db = core::ptr::read_volatile((manager + 0x178) as *const usize);
    if db == 0 {
        return None;
    }
    let inner = core::ptr::read_volatile((db + 0x8) as *const usize);
    if inner == 0 {
        return None;
    }
    let root = core::ptr::read_volatile((inner + 0x20) as *const usize);
    let context = core::ptr::read_volatile((inner + 0x18) as *const usize);
    if root == 0 || context == 0 {
        return None;
    }
    let hashes = core::ptr::read_volatile((context + 0x20) as *const usize);
    let refs = core::ptr::read_volatile((context + 0x28) as *const usize);
    if hashes == 0 || refs == 0 {
        return None;
    }
    Some(ParamTree { root, hashes, refs })
}

#[cfg(not(test))]
pub fn request_disp_order(ui_stage_id: u64, value: u16) {
    if let Ok(mut pending) = PENDING.lock() {
        pending.retain(|(id, _)| *id != ui_stage_id);
        pending.push((ui_stage_id, value));
    }
}

#[cfg(not(test))]
pub fn apply_pending() {
    let queued: Vec<(u64, u16)> = match PENDING.lock() {
        Ok(pending) if !pending.is_empty() => pending.clone(),
        _ => return,
    };
    let tree = match unsafe { resolve() } {
        Some(tree) => tree,
        None => return,
    };
    let mut done: Vec<u64> = Vec::new();
    for (ui_stage_id, value) in &queued {
        match unsafe { set_disp_order(&tree, *ui_stage_id, *value) } {
            Ok(()) => done.push(*ui_stage_id),
            Err(RowError::RowNotFound(_)) => continue,
            Err(error) => {
                skyline::println!("[stagerow] {ui_stage_id:#x} -> {value} refused: {error:?}");
                done.push(*ui_stage_id);
            }
        }
    }
    if done.is_empty() {
        return;
    }
    if let Ok(mut pending) = PENDING.lock() {
        pending.retain(|(id, _)| !done.contains(id));
    }
    skyline::println!(
        "[stagerow] settled {} disp_order request(s), {} still waiting for their row",
        done.len(),
        queued.len() - done.len()
    );
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_set_stage_disp_order(ui_stage_id: u64, value: u16) -> i32 {
    if value > u8::MAX as u16 {
        skyline::println!(
            "[stagerow] refused {ui_stage_id:#x}: disp_order {value} exceeds 255, which is all \
             a u8 node can carry"
        );
        return -1;
    }
    request_disp_order(ui_stage_id, value);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Builder {
        hashes: Vec<u64>,
        refs: Vec<u8>,
        params: Vec<u8>,
    }

    impl Builder {
        fn new() -> Self {
            Self {
                hashes: vec![0],
                refs: Vec::new(),
                params: Vec::new(),
            }
        }

        fn intern(&mut self, hash: u64) -> u32 {
            if let Some(i) = self.hashes.iter().position(|h| *h == hash) {
                return i as u32;
            }
            self.hashes.push(hash);
            (self.hashes.len() - 1) as u32
        }

        fn build(rows: &[(u64, u8, u8)]) -> (Self, usize) {
            let mut b = Builder::new();
            let db_root = b.intern(db_root_key());
            let stage_id = b.intern(ui_stage_id_key());
            let disp = b.intern(disp_order_key());

            let root = 0usize;
            b.params.extend_from_slice(&[TAG_STRUCT]);
            b.params.extend_from_slice(&1u32.to_le_bytes());
            let root_ref = b.refs.len() as u32;
            b.params.extend_from_slice(&root_ref.to_le_bytes());
            let list_at = b.params.len();
            b.refs.extend_from_slice(&db_root.to_le_bytes());
            b.refs
                .extend_from_slice(&((list_at - root) as u32).to_le_bytes());

            b.params.push(TAG_LIST);
            b.params
                .extend_from_slice(&(rows.len() as u32).to_le_bytes());
            let offsets_at = b.params.len();
            b.params.resize(offsets_at + rows.len() * 4, 0);

            for (index, (ui_stage_id, tag, value)) in rows.iter().enumerate() {
                let element = b.params.len();
                let rel = (element - list_at) as u32;
                b.params[offsets_at + index * 4..offsets_at + index * 4 + 4]
                    .copy_from_slice(&rel.to_le_bytes());

                b.params.push(TAG_STRUCT);
                b.params.extend_from_slice(&2u32.to_le_bytes());
                let element_ref = b.refs.len() as u32;
                b.params.extend_from_slice(&element_ref.to_le_bytes());

                let id_node = b.params.len();
                b.params.push(TAG_HASH40);
                let interned = b.intern(*ui_stage_id);
                b.params.extend_from_slice(&interned.to_le_bytes());

                let disp_node = b.params.len();
                b.params.push(*tag);
                b.params.push(*value);

                b.refs.extend_from_slice(&stage_id.to_le_bytes());
                b.refs
                    .extend_from_slice(&((id_node - element) as u32).to_le_bytes());
                b.refs.extend_from_slice(&disp.to_le_bytes());
                b.refs
                    .extend_from_slice(&((disp_node - element) as u32).to_le_bytes());
            }
            (b, root)
        }

        fn tree(&self) -> ParamTree {
            ParamTree {
                root: self.params.as_ptr() as usize,
                hashes: self.hashes.as_ptr() as usize,
                refs: self.refs.as_ptr() as usize,
            }
        }
    }

    fn stage(name: &str) -> u64 {
        hash40(name)
    }

    #[test]
    fn the_keys_match_the_hashes_the_image_uses() {
        assert_eq!(disp_order_key(), 0x0000_000A_CB22_637C);
    }

    #[test]
    fn walks_a_tree_and_reads_a_row() {
        let (b, _) = Builder::build(&[
            (stage("ui_stage_battlefield"), TAG_I8, 0),
            (stage("ui_stage_pumpkin_hill"), TAG_I8, 42),
        ]);
        let tree = b.tree();
        unsafe {
            assert_eq!(tree.rows().unwrap().len(), 2);
            let row = tree.find_row(stage("ui_stage_pumpkin_hill")).unwrap();
            let field = tree.field(row, disp_order_key()).unwrap();
            assert_eq!(tree.scalar(field), Some(42));
        }
    }

    #[test]
    fn promotes_one_row_past_127_and_leaves_the_other_alone() {
        let (b, _) = Builder::build(&[
            (stage("ui_stage_battlefield"), TAG_I8, 7),
            (stage("ui_stage_pumpkin_hill"), TAG_I8, 0),
        ]);
        let tree = b.tree();
        unsafe {
            set_disp_order(&tree, stage("ui_stage_pumpkin_hill"), 200).unwrap();

            let promoted = tree
                .field(
                    tree.find_row(stage("ui_stage_pumpkin_hill")).unwrap(),
                    disp_order_key(),
                )
                .unwrap();
            assert_eq!(tree.tag(promoted), TAG_U8, "should have been retyped");
            assert_eq!(tree.scalar(promoted), Some(200));

            let other = tree
                .field(
                    tree.find_row(stage("ui_stage_battlefield")).unwrap(),
                    disp_order_key(),
                )
                .unwrap();
            assert_eq!(tree.tag(other), TAG_I8);
            assert_eq!(tree.scalar(other), Some(7));
        }
    }

    #[test]
    fn values_within_i8_stay_i8() {
        let (b, _) = Builder::build(&[(stage("ui_stage_pumpkin_hill"), TAG_I8, 0)]);
        let tree = b.tree();
        unsafe {
            set_disp_order(&tree, stage("ui_stage_pumpkin_hill"), 127).unwrap();
            let field = tree
                .field(
                    tree.find_row(stage("ui_stage_pumpkin_hill")).unwrap(),
                    disp_order_key(),
                )
                .unwrap();
            assert_eq!(tree.tag(field), TAG_I8);
            assert_eq!(tree.scalar(field), Some(127));
        }
    }

    #[test]
    fn refuses_past_what_one_byte_holds() {
        let (b, _) = Builder::build(&[(stage("ui_stage_pumpkin_hill"), TAG_I8, 0)]);
        let tree = b.tree();
        unsafe {
            assert_eq!(
                set_disp_order(&tree, stage("ui_stage_pumpkin_hill"), 256),
                Err(RowError::ValueOutOfRange(256))
            );
        }
    }

    #[test]
    fn refuses_a_retype_that_would_move_every_later_offset() {
        let (b, _) = Builder::build(&[(stage("ui_stage_pumpkin_hill"), TAG_I8, 0)]);
        let tree = b.tree();
        unsafe {
            let field = tree
                .field(
                    tree.find_row(stage("ui_stage_pumpkin_hill")).unwrap(),
                    disp_order_key(),
                )
                .unwrap();
            assert_eq!(
                tree.write_scalar(field, TAG_I16, 300),
                Err(RowError::WidthWouldChange {
                    from: TAG_I8,
                    to: TAG_I16
                })
            );
            assert_eq!(tree.tag(field), TAG_I8, "must not have written anything");
        }
    }

    #[test]
    fn reports_a_stage_that_is_not_there() {
        let (b, _) = Builder::build(&[(stage("ui_stage_battlefield"), TAG_I8, 0)]);
        let tree = b.tree();
        let missing = stage("ui_stage_nowhere");
        unsafe {
            assert_eq!(
                set_disp_order(&tree, missing, 10),
                Err(RowError::RowNotFound(missing))
            );
        }
    }

    #[test]
    fn a_row_already_stored_as_u8_is_written_without_complaint() {
        let (b, _) = Builder::build(&[(stage("ui_stage_pumpkin_hill"), TAG_U8, 200)]);
        let tree = b.tree();
        unsafe {
            set_disp_order(&tree, stage("ui_stage_pumpkin_hill"), 250).unwrap();
            let field = tree
                .field(
                    tree.find_row(stage("ui_stage_pumpkin_hill")).unwrap(),
                    disp_order_key(),
                )
                .unwrap();
            assert_eq!(tree.scalar(field), Some(250));
        }
    }

    #[test]
    fn scalar_widens_the_way_the_games_dispatch_does() {
        let (b, _) = Builder::build(&[(stage("x"), TAG_I8, 0xFF)]);
        let tree = b.tree();
        unsafe {
            let field = tree
                .field(tree.find_row(stage("x")).unwrap(), disp_order_key())
                .unwrap();
            assert_eq!(tree.scalar(field), Some(-1));
            tree.write_scalar(field, TAG_U8, 0xFF).unwrap();
            assert_eq!(tree.scalar(field), Some(255));
        }
    }
}

#[cfg(test)]
pub fn apply_pending() {}
