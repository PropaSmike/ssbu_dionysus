use dionysus_core::grab_rule::{EVENT_SENDER_ID, ON_LINK_EVENT_SLOT};
use dionysus_core::tether::{Capsule, Tether, TetherBox, FIRST_BOX};
use dionysus_core::{clone_row, CLONE_SLOTS};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const STATUS_MODULE_BOMA: usize = 8;
const BOMA_GROUND_MODULE: usize = 0x58;
const GROUND_HANG_DATA: usize = 0x140;
const VECTOR_END: usize = 8;
const BOX_BYTES: usize = 16;
const MAX_BASE_BOXES: usize = 64;
const LOWEST_POINTER: usize = 0x1_0000;
const LOG_BUDGET: u32 = 16;
const OBJECT_MODULE_ACCESSOR: usize = 0x20;
const CAPSULE_LOOKUP: usize = 0x62_18C4;
const CAPSULE_LOOKUP_WORD: u32 = 0xA940_2428;
const CAPSULE_BYTES: usize = 0x28;
const CAPSULE_SIZE_AT: usize = 0x18;
const CAPSULE_NODE_AT: usize = 0x20;
const GET_BATTLE_OBJECT_FROM_ID: usize = 0x3A_C560;
const OBJECT_KIND: usize = 0xC;

type LinkEventFn = unsafe extern "C" fn(usize, usize, usize) -> u64;

#[allow(clippy::declare_interior_mutable_const)]
const NO_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
#[allow(clippy::declare_interior_mutable_const)]
const NOT_INSTALLED: AtomicBool = AtomicBool::new(false);

static ACTIVE: AtomicBool = AtomicBool::new(false);
static LOG_LINES: AtomicU32 = AtomicU32::new(0);
static CAPSULE_LOG: AtomicU32 = AtomicU32::new(0);
static NO_CAPSULES: [usize; 3] = [0; 3];
static ORIGINALS: [AtomicUsize; CLONE_SLOTS] = [NO_ORIGINAL; CLONE_SLOTS];
static INSTALLED: [AtomicBool; CLONE_SLOTS] = [NOT_INSTALLED; CLONE_SLOTS];
static EVENT_LOG: AtomicU32 = AtomicU32::new(0);

struct Entry {
    kind: i32,
    boxes: Vec<TetherBox>,
    built: Option<([u32; 4], usize)>,
    capsules: &'static [Capsule],
    capsule_list: Option<usize>,
    donor: Option<i32>,
}

fn entries() -> &'static Mutex<Vec<Entry>> {
    static ENTRIES: OnceLock<Mutex<Vec<Entry>>> = OnceLock::new();
    ENTRIES.get_or_init(|| Mutex::new(Vec::new()))
}

fn fighters() -> &'static Mutex<Vec<(usize, i32)>> {
    static FIGHTERS: OnceLock<Mutex<Vec<(usize, i32)>>> = OnceLock::new();
    FIGHTERS.get_or_init(|| Mutex::new(Vec::new()))
}

fn readable(address: usize) -> bool {
    address >= LOWEST_POINTER && address % 8 == 0
}

fn log(message: String) {
    if LOG_LINES.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&message);
    }
}

pub(crate) fn register(kind: i32, tether: &Tether) -> Result<usize, String> {
    tether.check()?;
    let boxes = tether.boxes();
    let count = boxes.len();
    let mut held = entries().lock().map_err(|_| "tether registry poisoned".to_string())?;
    held.retain(|entry| entry.kind != kind);
    let donor = tether.donor().map(|donor| donor.kind);
    held.push(Entry { kind, boxes, built: None, capsules: tether.capsules(), capsule_list: None, donor });
    drop(held);
    ACTIVE.store(true, Ordering::Release);
    if donor.is_some() && !unsafe { install_events(kind) } {
        crate::dbg_log_public(&format!(
            "[tether] clone kind {kind}: no private fighter table for the link event entry; its tether article cannot reach its donor's events"
        ));
    }
    Ok(count)
}

pub(crate) fn on_status_agent(boma: u64, clone_kind: Option<i32>) {
    if !ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let tethered = clone_kind.filter(|kind| {
        entries()
            .lock()
            .map(|held| held.iter().any(|entry| entry.kind == *kind))
            .unwrap_or(false)
    });
    if let Ok(mut held) = fighters().lock() {
        held.retain(|(known, _)| *known != boma as usize);
        if let Some(kind) = tethered {
            held.push((boma as usize, kind));
        }
    }
}

unsafe fn build(base: [u32; 4], boxes: &[TetherBox]) -> usize {
    let mut words: Vec<u32> = base.to_vec();
    for shape in boxes {
        words.extend(shape.words().iter().map(|value| value.to_bits()));
    }
    let storage = Box::leak(words.into_boxed_slice());
    let begin = storage.as_ptr() as usize;
    let end = begin + storage.len() * 4;
    Box::leak(Box::new([begin, end, end])).as_ptr() as usize
}

pub(crate) unsafe fn refresh(module: usize) {
    if !ACTIVE.load(Ordering::Acquire) || !readable(module) {
        return;
    }
    let boma = core::ptr::read_volatile((module + STATUS_MODULE_BOMA) as *const usize);
    let Some(kind) = fighters()
        .lock()
        .ok()
        .and_then(|held| held.iter().find(|(known, _)| *known == boma).map(|(_, kind)| *kind))
    else {
        return;
    };
    let ground = core::ptr::read_volatile((boma + BOMA_GROUND_MODULE) as *const usize);
    if !readable(ground) {
        return;
    }
    let cell = ground + GROUND_HANG_DATA;
    let current = core::ptr::read_volatile(cell as *const usize);
    if !readable(current) {
        return;
    }
    let Ok(mut held) = entries().lock() else {
        return;
    };
    let Some(entry) = held.iter_mut().find(|entry| entry.kind == kind && !entry.boxes.is_empty()) else {
        return;
    };
    if entry.built.map(|(_, object)| object) == Some(current) {
        return;
    }
    let begin = core::ptr::read_volatile(current as *const usize);
    let end = core::ptr::read_volatile((current + VECTOR_END) as *const usize);
    if !readable(begin) || end <= begin || (end - begin) % BOX_BYTES != 0 || (end - begin) / BOX_BYTES > MAX_BASE_BOXES {
        return;
    }
    let base = core::ptr::read_volatile(begin as *const [u32; 4]);
    let object = match entry.built {
        Some((known, object)) if known == base => object,
        _ => {
            let object = build(base, &entry.boxes);
            entry.built = Some((base, object));
            object
        }
    };
    core::ptr::write_volatile(cell as *mut usize, object);
    log(format!(
        "[tether] clone kind {kind} hangs {} tether box(es) as cliff hang data {FIRST_BOX}..{}, its base's {} box(es) replaced past box 0",
        entry.boxes.len(),
        FIRST_BOX + entry.boxes.len() - 1,
        (end - begin) / BOX_BYTES
    ));
}

fn build_capsules(capsules: &[Capsule]) -> usize {
    let mut bytes = vec![0u8; capsules.len() * CAPSULE_BYTES];
    for (index, capsule) in capsules.iter().enumerate() {
        let row = &mut bytes[index * CAPSULE_BYTES..(index + 1) * CAPSULE_BYTES];
        for (slot, value) in capsule.p1.iter().chain(capsule.p2.iter()).enumerate() {
            row[slot * 4..slot * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        row[CAPSULE_SIZE_AT..CAPSULE_SIZE_AT + 4].copy_from_slice(&capsule.size.to_le_bytes());
        row[CAPSULE_NODE_AT..CAPSULE_NODE_AT + 8]
            .copy_from_slice(&crate::hash40::hash40(capsule.node).to_le_bytes());
    }
    let storage = Box::leak(bytes.into_boxed_slice());
    let begin = storage.as_ptr() as usize;
    let end = begin + storage.len();
    Box::leak(Box::new([begin, end, end])).as_ptr() as usize
}

unsafe fn capsule_list_for(object: usize) -> (usize, Option<(i32, usize)>) {
    let empty = NO_CAPSULES.as_ptr() as usize;
    if !readable(object) {
        return (empty, None);
    }
    let boma = core::ptr::read_volatile((object + OBJECT_MODULE_ACCESSOR) as *const usize);
    let Some(kind) = fighters()
        .lock()
        .ok()
        .and_then(|held| held.iter().find(|(known, _)| *known == boma).map(|(_, kind)| *kind))
    else {
        return (empty, None);
    };
    let Ok(mut held) = entries().lock() else {
        return (empty, None);
    };
    let Some(entry) = held.iter_mut().find(|entry| entry.kind == kind && !entry.capsules.is_empty()) else {
        return (empty, None);
    };
    let list = *entry.capsule_list.get_or_insert_with(|| build_capsules(entry.capsules));
    (list, Some((kind, entry.capsules.len())))
}

#[skyline::hook(offset = crate::game::rel(CAPSULE_LOOKUP), inline)]
unsafe fn capsule_lookup(ctx: &mut skyline::hooks::InlineCtx) {
    if ctx.registers[1].x() != 0 {
        return;
    }
    let (list, owner) = capsule_list_for(ctx.registers[20].x() as usize);
    ctx.registers[1].set_x(list as u64);
    if CAPSULE_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&match owner {
            Some((kind, count)) => format!(
                "[tether] clone kind {kind} reads its donor's {count} tether body capsule(s); its base has none"
            ),
            None => "[tether] a fighter without tether body capsules reached the tether capsule read; answered with none".to_string(),
        });
    }
}

pub(crate) fn install() {
    let word = unsafe { crate::game::text_word(CAPSULE_LOOKUP) };
    if word != CAPSULE_LOOKUP_WORD {
        crate::dbg_log_public(&format!(
            "[tether] capsule read at {CAPSULE_LOOKUP:#x} not recognised ({word:#010x}); a clone on a base without tether capsules faults when its tether reaches"
        ));
        return;
    }
    skyline::install_hooks!(capsule_lookup);
}

unsafe fn install_events(kind: i32) -> bool {
    let Some(row) = clone_row(kind) else {
        return false;
    };
    if INSTALLED[row].swap(true, Ordering::AcqRel) {
        return true;
    }
    let installed = crate::clone_vtables::set_entry(
        kind,
        ON_LINK_EVENT_SLOT,
        crate::clone_vtables::Space::Fighter,
        link_event as *const () as usize,
        &ORIGINALS[row] as *const AtomicUsize as usize,
    )
    .is_some();
    if !installed {
        INSTALLED[row].store(false, Ordering::Release);
    }
    installed
}

unsafe fn route_to_donor(kind: i32, object: usize, event: usize) -> Option<u64> {
    if !readable(event) {
        return None;
    }
    let donor = entries().lock().ok()?.iter().find(|entry| entry.kind == kind)?.donor?;
    let sender_id = core::ptr::read_unaligned((event + EVENT_SENDER_ID) as *const u32);
    let lookup: extern "C" fn(u32) -> usize = core::mem::transmute(crate::game::addr(GET_BATTLE_OBJECT_FROM_ID));
    let sender = lookup(sender_id);
    if !readable(sender) {
        return None;
    }
    let shown = core::ptr::read_volatile((sender + OBJECT_KIND) as *const i32);
    let minted = crate::clone_vtables::minted_kind_of_object(sender).unwrap_or(shown);
    let source = crate::custom_articles::custom_weapon_source_kind(minted)?;
    let table = crate::custom_articles::custom_weapon_owner_kind(minted);
    let owned = table == Some(kind) || (table.is_some() && table == crate::clone_base(kind));
    if !owned || crate::custom_articles::source_weapon_owner_kind(minted) != Some(donor) {
        if EVENT_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
            crate::dbg_log_public(&format!(
                "[tether] clone kind {kind}: weapon {minted} (minted from {source}, table {table:?}) sent a link event that is not its donor's tether; the base answers"
            ));
        }
        return None;
    }
    let (donor_class, handler) = crate::clone_vtables::vanilla_fighter_entry(donor, ON_LINK_EVENT_SLOT)?;
    let handler: LinkEventFn = core::mem::transmute(handler);
    core::ptr::write_volatile((sender + OBJECT_KIND) as *mut i32, source);
    let answer = handler(donor_class, object, event);
    core::ptr::write_volatile((sender + OBJECT_KIND) as *mut i32, shown);
    if EVENT_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&format!(
            "[tether] clone kind {kind}: weapon {minted} (minted from {source}) sent a link event; fighter kind {donor}'s handler answered {answer}"
        ));
    }
    Some(answer)
}

unsafe extern "C" fn link_event(class: usize, object: usize, event: usize) -> u64 {
    let kind = crate::clone_vtables::fighter_kind_of_class(class).or_else(|| crate::clone_kind_of_object(object as u64));
    if let Some(answer) = kind.and_then(|kind| route_to_donor(kind, object, event)) {
        return answer;
    }
    let original = kind.and_then(clone_row).map_or(0, |row| ORIGINALS[row].load(Ordering::Acquire));
    if original == 0 {
        return 1;
    }
    let original: LinkEventFn = core::mem::transmute(original);
    original(class, object, event)
}
