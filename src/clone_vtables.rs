use core::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

use dionysus_core::slots::CLONE_SLOTS;
use dionysus_core::vtable_copy;

use crate::custom_articles::{self, FIRST_CUSTOM_WEAPON_KIND, MAX_CUSTOM_ARTICLES};
use crate::item_clones::FIRST_SPARSE_ITEM_KIND;
use crate::item_params::MAX_CLONE_KINDS as MAX_CLONE_ITEMS;
use crate::{clone_base, dbg_log_public, offsets, FIGHTER_CLASS_TABLE, FIRST_CUSTOM_KIND};

pub(crate) const FIGHTER_SLOTS: usize = 147;
pub(crate) const WEAPON_SLOTS: usize = 104;
pub(crate) const ITEM_SLOTS: usize = 181;
pub(crate) const ITEM_OBJECT_VTABLE: usize = 0x5070488;
pub(crate) const HEADER: usize = 3;
const ITEM_HEADER: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Space {
    Fighter,
    Weapon,
    Item,
}

impl Space {
    pub(crate) fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Fighter),
            1 => Some(Self::Weapon),
            2 => Some(Self::Item),
            _ => None,
        }
    }

    pub(crate) fn slots(self) -> usize {
        match self {
            Self::Fighter => FIGHTER_SLOTS,
            Self::Weapon => WEAPON_SLOTS,
            Self::Item => ITEM_SLOTS,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Fighter => "fighter",
            Self::Weapon => "weapon",
            Self::Item => "item",
        }
    }
}

#[allow(clippy::declare_interior_mutable_const)]
const NO_CLASS: AtomicUsize = AtomicUsize::new(0);

static FIGHTER_CLASSES: [AtomicUsize; CLONE_SLOTS] = [NO_CLASS; CLONE_SLOTS];
static WEAPON_CLASSES: [AtomicUsize; MAX_CUSTOM_ARTICLES] = [NO_CLASS; MAX_CUSTOM_ARTICLES];
static ITEM_TABLES: [AtomicUsize; MAX_CLONE_ITEMS] = [NO_CLASS; MAX_CLONE_ITEMS];
static ITEM_ADOPTIONS: AtomicU32 = AtomicU32::new(0);
static BUILD: Mutex<()> = Mutex::new(());
static BUILD_FAILURES: AtomicU32 = AtomicU32::new(0);
static DRIFT_REPORTS: AtomicU32 = AtomicU32::new(0);

struct Record {
    kind: i32,
    space: Space,
    index: usize,
    function: usize,
    original_out: usize,
}

static RECORDS: Mutex<Vec<Record>> = Mutex::new(Vec::new());
static CONSTRUCTION_REPORTS: AtomicU32 = AtomicU32::new(0);
static VANILLA_REPORTS: AtomicU32 = AtomicU32::new(0);
static RECACHE_REPORTS: AtomicU32 = AtomicU32::new(0);

const FIGHTER_CLASS_FIELD: usize = 0xf700;
pub(crate) const WEAPON_CLASS_SUBOBJECT: usize = 0x150;
const WEAPON_CLASS_FIELD: usize = WEAPON_CLASS_SUBOBJECT + 0x38c8;
const WEAPON_CLASS_FIELD_2: usize = 0x3bb0;

const MINTED_RING: usize = 64;
#[allow(clippy::declare_interior_mutable_const)]
const NO_OBJECT: AtomicUsize = AtomicUsize::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const NO_KIND: AtomicI32 = AtomicI32::new(-1);
static MINTED_OBJECTS: [AtomicUsize; MINTED_RING] = [NO_OBJECT; MINTED_RING];
static MINTED_KINDS: [AtomicI32; MINTED_RING] = [NO_KIND; MINTED_RING];
static MINTED_NEXT: AtomicUsize = AtomicUsize::new(0);

fn minted_slot_of(object: usize) -> Option<usize> {
    if object == 0 {
        return None;
    }
    MINTED_OBJECTS.iter().position(|slot| slot.load(Ordering::Acquire) == object)
}

fn note_minted_weapon(object: usize, kind: i32) {
    if object == 0 {
        return;
    }
    let index = match minted_slot_of(object) {
        Some(index) => index,
        None => MINTED_NEXT.fetch_add(1, Ordering::Relaxed) % MINTED_RING,
    };
    MINTED_OBJECTS[index].store(object, Ordering::Release);
    MINTED_KINDS[index].store(kind, Ordering::Release);
}

fn minted_kind_of(object: usize) -> Option<i32> {
    let kind = MINTED_KINDS[minted_slot_of(object)?].load(Ordering::Acquire);
    (kind >= 0).then_some(kind)
}

fn fighter_slot(kind: i32) -> Option<&'static AtomicUsize> {
    let index = usize::try_from(kind.checked_sub(FIRST_CUSTOM_KIND)?).ok()?;
    FIGHTER_CLASSES.get(index)
}

fn weapon_slot(kind: i32) -> Option<&'static AtomicUsize> {
    WEAPON_CLASSES.get(custom_articles::row_of(kind)?)
}

fn item_slot(kind: i32) -> Option<&'static AtomicUsize> {
    let index = usize::try_from(kind.checked_sub(FIRST_SPARSE_ITEM_KIND)?).ok()?;
    ITEM_TABLES.get(index)
}

fn item_image_vtable() -> usize {
    crate::game::addr(ITEM_OBJECT_VTABLE)
}

pub(crate) unsafe fn item_table_for(kind: i32) -> Option<usize> {
    let slot = item_slot(kind)?;
    let known = slot.load(Ordering::Acquire);
    if known != 0 {
        sync(kind, Space::Item, known, item_image_vtable());
        return Some(known);
    }
    let _guard = BUILD.lock().ok()?;
    let known = slot.load(Ordering::Acquire);
    if known != 0 {
        return Some(known);
    }
    let source = item_image_vtable();
    if *(source as *const usize) == 0 {
        return None;
    }
    let blob = Box::leak(vec![0usize; ITEM_HEADER + 2 * ITEM_SLOTS].into_boxed_slice());
    let table = blob.as_mut_ptr().add(ITEM_HEADER) as usize;
    take(table, ITEM_SLOTS, source);
    let applied = apply_records(kind, Space::Item, table);
    dbg_log_public(&format!(
        "[vtable] item kind {kind:#x} owns a private copy of the Item object's {ITEM_SLOTS}-slot vtable at {table:#x} (image +{ITEM_OBJECT_VTABLE:#x}){}",
        applied_note(applied)
    ));
    slot.store(table, Ordering::Release);
    Some(table)
}

unsafe fn take(table: usize, slots: usize, source: usize) {
    let mut entries = [0usize; ITEM_SLOTS];
    let mut seen = [0usize; ITEM_SLOTS];
    let mut vanilla = [0usize; ITEM_SLOTS];
    read_words(source, &mut vanilla[..slots]);
    vtable_copy::take(&mut entries[..slots], &mut seen[..slots], &vanilla[..slots]);
    write_words(table, &entries[..slots]);
    write_words(table + slots * 8, &seen[..slots]);
}

unsafe fn read_words(from: usize, into: &mut [usize]) {
    for (index, word) in into.iter_mut().enumerate() {
        *word = core::ptr::read_volatile((from as *const usize).add(index));
    }
}

unsafe fn write_words(to: usize, from: &[usize]) {
    for (index, word) in from.iter().enumerate() {
        core::ptr::write_volatile((to as *mut usize).add(index), *word);
    }
}

fn applied_note(applied: usize) -> String {
    match applied {
        0 => String::new(),
        n => format!("; {n} override(s) applied"),
    }
}

unsafe fn write_original(original_out: usize, value: usize) {
    if original_out != 0 {
        (*(original_out as *const AtomicUsize)).store(value, Ordering::Release);
    }
}

unsafe fn apply_records(kind: i32, space: Space, table: usize) -> usize {
    let Ok(records) = RECORDS.lock() else {
        return 0;
    };
    let mut applied = 0;
    for record in records.iter().filter(|record| record.kind == kind && record.space == space) {
        let entry = (table as *mut usize).add(record.index);
        let previous = core::ptr::read_volatile(entry);
        core::ptr::write_volatile(entry, record.function);
        write_original(record.original_out, previous);
        applied += 1;
    }
    applied
}

unsafe fn sync(kind: i32, space: Space, table: usize, vanilla: usize) {
    if table == 0 || vanilla == 0 {
        return;
    }
    let slots = space.slots();
    let Ok(_guard) = BUILD.lock() else {
        return;
    };
    let mut entries = [0usize; ITEM_SLOTS];
    let mut seen = [0usize; ITEM_SLOTS];
    let mut now = [0usize; ITEM_SLOTS];
    read_words(table, &mut entries[..slots]);
    read_words(table + slots * 8, &mut seen[..slots]);
    read_words(vanilla, &mut now[..slots]);
    let drifts = vtable_copy::reconcile(&mut entries[..slots], &mut seen[..slots], &now[..slots]);
    if drifts.is_empty() {
        return;
    }
    crate::smashline_vtables::invalidate();
    let records = RECORDS.lock().ok();
    for drift in drifts {
        core::ptr::write_volatile((table as *mut usize).add(slots + drift.index), drift.now);
        let mut refreshed = 0;
        if drift.adopted {
            core::ptr::write_volatile((table as *mut usize).add(drift.index), drift.now);
        } else if let Some(records) = records.as_ref() {
            for record in records
                .iter()
                .filter(|record| record.kind == kind && record.space == space && record.index == drift.index)
            {
                if record.original_out == 0 {
                    continue;
                }
                let cell = &*(record.original_out as *const AtomicUsize);
                if cell.load(Ordering::Acquire) == drift.old {
                    cell.store(drift.now, Ordering::Release);
                    refreshed += 1;
                }
            }
        }
        let n = DRIFT_REPORTS.fetch_add(1, Ordering::Relaxed);
        if n < 32 {
            dbg_log_public(&format!(
                "[vtable] {} kind {kind} slot {}: the base's entry changed after the copy was taken ({:#x} -> {:#x}, image +{:#x}); {}",
                space.label(),
                drift.index,
                drift.old,
                drift.now,
                crate::game::offset_of(drift.now),
                if drift.adopted {
                    "adopted".to_string()
                } else {
                    format!("override kept, {refreshed} call_original target(s) refreshed")
                }
            ));
        }
    }
}

pub(crate) unsafe fn adopt_item_object(kind: i32, object: usize) {
    if object == 0 {
        return;
    }
    let Some(table) = item_table_for(kind) else {
        return;
    };
    let held = core::ptr::read_volatile(object as *const usize);
    if held == table {
        return;
    }
    let n = ITEM_ADOPTIONS.fetch_add(1, Ordering::Relaxed);
    if held != item_image_vtable() {
        if n < 8 {
            dbg_log_public(&format!(
                "[vtable] item kind {kind:#x} object {object:#x} holds vtable {held:#x} (image +{:#x}), not the Item object's; left alone",
                crate::game::offset_of(held)
            ));
        }
        return;
    }
    core::ptr::write_volatile(object as *mut usize, table);
    if n < 8 {
        dbg_log_public(&format!(
            "[vtable] item kind {kind:#x} object {object:#x} dispatches through its private vtable {table:#x}"
        ));
    }
}

pub(crate) unsafe fn release_item_object(object: usize) {
    if object == 0 {
        return;
    }
    let held = core::ptr::read_volatile(object as *const usize);
    if held == 0 || held == item_image_vtable() {
        return;
    }
    if ITEM_TABLES.iter().any(|slot| slot.load(Ordering::Acquire) == held) {
        core::ptr::write_volatile(object as *mut usize, item_image_vtable());
    }
}

unsafe fn table_of(class: usize) -> usize {
    if class == 0 {
        0
    } else {
        *(class as *const usize)
    }
}

unsafe fn build(source_class: usize, slots: usize) -> Option<usize> {
    let table = table_of(source_class);
    if table == 0 {
        return None;
    }
    let blob = Box::leak(vec![0usize; HEADER + 2 * slots].into_boxed_slice());
    let words = blob.as_mut_ptr();
    take(words.add(HEADER) as usize, slots, table);
    *words = words.add(HEADER) as usize;
    Some(words as usize)
}

unsafe fn class_for(
    slot: &AtomicUsize,
    space: Space,
    kind: i32,
    source: i32,
    source_class: impl Fn() -> usize,
) -> Option<usize> {
    let slots = space.slots();
    let known = slot.load(Ordering::Acquire);
    if known != 0 {
        sync(kind, space, known + HEADER * 8, table_of(source_class()));
        return Some(known);
    }
    let _guard = BUILD.lock().ok()?;
    let known = slot.load(Ordering::Acquire);
    if known != 0 {
        return Some(known);
    }
    let source_class = source_class();
    let Some(built) = build(source_class, slots) else {
        let n = BUILD_FAILURES.fetch_add(1, Ordering::Relaxed);
        if n < 8 {
            dbg_log_public(&format!(
                "[vtable] {} kind {kind}: source {source} has no class or vtable yet (class {source_class:#x}); serving the source's own until it does",
                space.label()
            ));
        }
        return None;
    };
    let applied = apply_records(kind, space, built + HEADER * 8);
    dbg_log_public(&format!(
        "[vtable] {} kind {kind} owns a private copy of {source}'s {slots}-slot vtable at {:#x} (class {built:#x}){}",
        space.label(),
        built + HEADER * 8,
        applied_note(applied)
    ));
    slot.store(built, Ordering::Release);
    Some(built)
}

unsafe fn vanilla_fighter_class(base: i32) -> usize {
    if !(0..94).contains(&base) {
        return 0;
    }
    *((crate::game::addr(FIGHTER_CLASS_TABLE) + base as usize * 8) as *const usize)
}

pub(crate) unsafe fn vanilla_fighter_entry(kind: i32, index: u32) -> Option<(usize, usize)> {
    let class = vanilla_fighter_class(kind);
    let table = table_of(class);
    if table == 0 || index as usize >= Space::Fighter.slots() {
        return None;
    }
    let entry = *((table + index as usize * 8) as *const usize);
    (entry != 0).then_some((class, entry))
}

pub(crate) unsafe fn fighter_class_for(kind: i32, base: i32) -> Option<usize> {
    if !(0..94).contains(&base) {
        return None;
    }
    let slot = fighter_slot(kind)?;
    class_for(slot, Space::Fighter, kind, base, || vanilla_fighter_class(base))
}

pub(crate) unsafe fn fighter_class(kind: i32) -> Option<usize> {
    let base = clone_base(kind)?;
    fighter_class_for(kind, base)
}

pub(crate) unsafe fn vanilla_weapon_class(kind: i32) -> usize {
    let getter: extern "C" fn(u32) -> usize =
        core::mem::transmute(crate::game::addr(offsets::OFF_WEAPON_CLASS_RESOLVER));
    getter(kind as u32)
}

pub(crate) unsafe fn weapon_class_for(kind: i32, source: i32) -> Option<usize> {
    if !(0..FIRST_CUSTOM_WEAPON_KIND).contains(&source) {
        return None;
    }
    let slot = weapon_slot(kind)?;
    class_for(slot, Space::Weapon, kind, source, || vanilla_weapon_class(source))
}

pub(crate) unsafe fn weapon_class(kind: i32) -> Option<usize> {
    let source = custom_articles::custom_weapon_source_kind(kind)?;
    weapon_class_for(kind, source)
}

unsafe fn vanilla_table(kind: i32, space: Space) -> Option<usize> {
    let table = match space {
        Space::Fighter => table_of(vanilla_fighter_class(clone_base(kind)?)),
        Space::Weapon => table_of(vanilla_weapon_class(custom_articles::custom_weapon_source_kind(kind)?)),
        Space::Item => {
            item_slot(kind)?;
            item_image_vtable()
        }
    };
    (table != 0 && *(table as *const usize) != 0).then_some(table)
}

fn known_table(kind: i32, space: Space) -> Option<usize> {
    let held = match space {
        Space::Fighter => fighter_slot(kind)?.load(Ordering::Acquire),
        Space::Weapon => weapon_slot(kind)?.load(Ordering::Acquire),
        Space::Item => return Some(item_slot(kind)?.load(Ordering::Acquire)).filter(|table| *table != 0),
    };
    (held != 0).then_some(held + HEADER * 8)
}

pub(crate) unsafe fn clone_table(kind: i32, space: Space) -> Option<usize> {
    known_table(kind, space).or_else(|| vanilla_table(kind, space))
}

pub(crate) fn is_overridden(kind: i32, space: Space, index: usize) -> bool {
    RECORDS.lock().map_or(false, |records| {
        records
            .iter()
            .any(|record| record.kind == kind && record.space == space && record.index == index)
    })
}

unsafe fn report_construction(
    label: &str,
    verb: &str,
    kind: i32,
    object: usize,
    field: usize,
    held: usize,
    private: Option<usize>,
) {
    if object == 0 {
        return;
    }
    let n = CONSTRUCTION_REPORTS.fetch_add(1, Ordering::Relaxed);
    if n >= 32 {
        return;
    }
    let verdict = match private {
        Some(class) if held == class => "PRIVATE",
        Some(_) if held == 0 => "unset yet",
        Some(_) => "NOT the private class",
        None => "no private class exists",
    };
    dbg_log_public(&format!(
        "[vtable] {label} kind {kind} object {object:#x} {verb} class {held:#x} at +{field:#x}: {verdict}{}",
        match private {
            Some(class) => format!(" (private {class:#x})"),
            None => String::new(),
        }
    ));
}

pub(crate) unsafe fn report_fighter_construction(kind: i32, object: usize) {
    if object == 0 {
        return;
    }
    let held = core::ptr::read_volatile((object + FIGHTER_CLASS_FIELD) as *const usize);
    let private = if crate::smashline_vtables::kind_of_variant(held) == Some(kind) {
        Some(held)
    } else {
        fighter_slot(kind).map(|slot| slot.load(Ordering::Acquire)).filter(|class| *class != 0)
    };
    report_construction("fighter", "holds", kind, object, FIGHTER_CLASS_FIELD, held, private);
}

pub(crate) unsafe fn report_weapon_construction(kind: i32, object: usize, class: usize) {
    note_minted_weapon(object, kind);
    let private = weapon_slot(kind).map(|slot| slot.load(Ordering::Acquire)).filter(|class| *class != 0);
    report_construction("weapon", "caches", kind, object, WEAPON_CLASS_FIELD, class, private);
}

pub(crate) unsafe fn recache_weapon_class(asked: i32, object: usize, answered: usize) -> Option<usize> {
    let kind = minted_kind_of(object)?;
    let private = weapon_slot(kind)?.load(Ordering::Acquire);
    if private == 0 {
        return None;
    }
    let corrected = answered != private;
    let n = RECACHE_REPORTS.fetch_add(1, Ordering::Relaxed);
    if n < 32 {
        dbg_log_public(&format!(
            "[vtable] weapon kind {kind} object {object:#x} re-caches its class at +{WEAPON_CLASS_FIELD_2:#x}, asked as kind {asked}: getter answered {answered:#x}, {}",
            if corrected {
                format!("corrected to the private class {private:#x}")
            } else {
                "PRIVATE kept".to_string()
            }
        ));
    }
    corrected.then_some(private)
}

fn fighter_kind_of_index(index: usize) -> Option<i32> {
    Some(FIRST_CUSTOM_KIND + index as i32)
}

pub(crate) fn fighter_kind_of_class(class: usize) -> Option<i32> {
    private_owner(&FIGHTER_CLASSES, fighter_kind_of_index, class)
        .or_else(|| crate::smashline_vtables::kind_of_variant(class))
}

pub(crate) fn weapon_kind_of_class(class: usize) -> Option<i32> {
    private_owner(&WEAPON_CLASSES, custom_articles::kind_of_row, class)
}

pub(crate) fn minted_kind_of_object(object: usize) -> Option<i32> {
    #[cfg(feature = "css_slot")]
    if let Some(kind) = crate::article_kind_facade::minted_kind_of(object) {
        return Some(kind);
    }
    minted_kind_of(object)
}

fn private_owner(classes: &[AtomicUsize], kind_of: fn(usize) -> Option<i32>, class: usize) -> Option<i32> {
    if class == 0 {
        return None;
    }
    classes
        .iter()
        .position(|slot| slot.load(Ordering::Acquire) == class)
        .and_then(kind_of)
}

unsafe fn report_vanilla(
    label: &str,
    verb: &str,
    kind: i32,
    object: usize,
    field: usize,
    held: usize,
    own: usize,
    classes: &[AtomicUsize],
    kind_of: fn(usize) -> Option<i32>,
) {
    if object == 0 {
        return;
    }
    let n = VANILLA_REPORTS.fetch_add(1, Ordering::Relaxed);
    if n >= 48 {
        return;
    }
    let verdict = if held == own {
        "its own image class".to_string()
    } else if let Some(clone) = private_owner(classes, kind_of, held) {
        format!("clone {clone}'s PRIVATE class, not its own")
    } else if held == 0 {
        "unset yet".to_string()
    } else {
        "neither its own nor any private class".to_string()
    };
    dbg_log_public(&format!(
        "[vtable] vanilla {label} kind {kind} object {object:#x} {verb} class {held:#x} at +{field:#x}: {verdict} (own {own:#x})"
    ));
}

pub(crate) unsafe fn report_vanilla_fighter_construction(kind: i32, object: usize) {
    if object == 0 || !(0..94).contains(&kind) {
        return;
    }
    let own = *((crate::game::addr(FIGHTER_CLASS_TABLE) + kind as usize * 8) as *const usize);
    let held = core::ptr::read_volatile((object + FIGHTER_CLASS_FIELD) as *const usize);
    report_vanilla("fighter", "holds", kind, object, FIGHTER_CLASS_FIELD, held, own, &FIGHTER_CLASSES, fighter_kind_of_index);
}

pub(crate) unsafe fn report_vanilla_weapon_construction(kind: i32, object: usize, class: usize) {
    if !(0..FIRST_CUSTOM_WEAPON_KIND).contains(&kind) || !custom_articles::is_source_of_custom_weapon(kind) {
        return;
    }
    let own = vanilla_weapon_class(kind);
    report_vanilla("weapon", "caches", kind, object, WEAPON_CLASS_FIELD, class, own, &WEAPON_CLASSES, custom_articles::kind_of_row);
}

pub(crate) unsafe fn set_entry(
    kind: i32,
    index: u32,
    space: Space,
    function: usize,
    original_out: usize,
) -> Option<usize> {
    let index = index as usize;
    if index >= space.slots() || function == 0 {
        return None;
    }
    match space {
        Space::Fighter => clone_base(kind).and(fighter_slot(kind))?,
        Space::Weapon => custom_articles::custom_weapon_source_kind(kind).and(weapon_slot(kind))?,
        Space::Item => item_slot(kind)?,
    };
    let _guard = BUILD.lock().ok()?;
    let built = known_table(kind, space);
    let previous = match built {
        Some(table) => {
            let entry = (table as *mut usize).add(index);
            let previous = core::ptr::read_volatile(entry);
            core::ptr::write_volatile(entry, function);
            previous
        }
        None => {
            let queued = RECORDS.lock().ok().and_then(|records| {
                records
                    .iter()
                    .rev()
                    .find(|record| record.kind == kind && record.space == space && record.index == index)
                    .map(|record| record.function)
            });
            match queued {
                Some(function) => function,
                None => match vanilla_table(kind, space) {
                    Some(table) => core::ptr::read_volatile((table as *const usize).add(index)),
                    None => 0,
                },
            }
        }
    };
    if let Ok(mut records) = RECORDS.lock() {
        records.push(Record {
            kind,
            space,
            index,
            function,
            original_out,
        });
    }
    write_original(original_out, previous);
    crate::smashline_vtables::invalidate();
    dbg_log_public(&format!(
        "[vtable] {} kind {kind} slot {index}: {previous:#x} -> {function:#x}{}",
        space.label(),
        if built.is_some() {
            ""
        } else {
            " (written into the copy at the first construction)"
        }
    ));
    Some(previous)
}
