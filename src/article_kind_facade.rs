use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::RwLock;

use crate::custom_articles;

const OFF_WEAPON_CONSTRUCT: usize = 0x33b5640;
const OFF_WEAPON_INIT: usize = 0x339e810;
const OFF_WEAPON_TEARDOWN: usize = 0x33b78d0;
const OFF_BATTLE_OBJECT_FROM_ID: usize = 0x3ac560;
const WEAPON_VTABLE: usize = 0x51853b8;
const SLOT_START: usize = 6;
const SLOT_STOP: usize = 9;
const START_ENTRY: usize = 0x33a07d0;
const STOP_ENTRY: usize = 0x33a16e0;
const WEAPON_SUBOBJECT: usize = 0x150;
const OBJECT_KIND: usize = 0xc;
const OBJECT_MODULE_ACCESSOR: usize = 0x20;
const ACCESSOR_OBJECT_ID: usize = 0x8;
const LOG_BUDGET: u32 = 12;

#[derive(Clone, Copy)]
struct Shown {
    object: usize,
    minted: i32,
    source: i32,
}

static SHOWN: RwLock<Vec<Shown>> = RwLock::new(Vec::new());
static SHOWN_COUNT: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_START: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_STOP: AtomicUsize = AtomicUsize::new(0);
static SHOW_LOG: AtomicU32 = AtomicU32::new(0);
static RESTORE_LOG: AtomicU32 = AtomicU32::new(0);

type LifecycleFn = unsafe extern "C" fn(u64, u64, u64, u64, u64, u64, u64, u64) -> u64;

fn enabled() -> bool {
    crate::settings::get().article_kind_facade
}

unsafe fn read_kind(object: usize) -> i32 {
    core::ptr::read_volatile((object + OBJECT_KIND) as *const i32)
}

unsafe fn write_kind(object: usize, kind: i32) {
    core::ptr::write_volatile((object + OBJECT_KIND) as *mut i32, kind);
}

fn find(object: usize) -> Option<Shown> {
    if SHOWN_COUNT.load(Ordering::Acquire) == 0 {
        return None;
    }
    SHOWN.read().ok()?.iter().find(|entry| entry.object == object).copied()
}

fn forget(object: usize) -> Option<Shown> {
    if SHOWN_COUNT.load(Ordering::Acquire) == 0 {
        return None;
    }
    let mut shown = SHOWN.write().ok()?;
    let index = shown.iter().position(|entry| entry.object == object)?;
    let entry = shown.swap_remove(index);
    SHOWN_COUNT.store(shown.len(), Ordering::Release);
    Some(entry)
}

fn remember(entry: Shown) {
    let Ok(mut shown) = SHOWN.write() else {
        return;
    };
    match shown.iter_mut().find(|known| known.object == entry.object) {
        Some(known) => *known = entry,
        None => shown.push(entry),
    }
    SHOWN_COUNT.store(shown.len(), Ordering::Release);
}

pub(crate) fn minted_kind_of(object: usize) -> Option<i32> {
    if object == 0 {
        return None;
    }
    let entry = find(object)?;
    let field = unsafe { read_kind(object) };
    (field == entry.source || field == entry.minted).then_some(entry.minted)
}

unsafe fn object_of_id(id: u32) -> usize {
    let lookup: extern "C" fn(u32) -> usize = core::mem::transmute(crate::game::addr(OFF_BATTLE_OBJECT_FROM_ID));
    lookup(id)
}

pub(crate) unsafe fn minted_kind_of_boma(boma: usize) -> Option<i32> {
    if boma == 0 || SHOWN_COUNT.load(Ordering::Acquire) == 0 {
        return None;
    }
    let object = object_of_id(core::ptr::read_volatile((boma + ACCESSOR_OBJECT_ID) as *const u32));
    if object == 0 || core::ptr::read_volatile((object + OBJECT_MODULE_ACCESSOR) as *const usize) != boma {
        return None;
    }
    minted_kind_of(object)
}

pub(crate) unsafe fn minted_kind_of_object_id(id: u32) -> Option<i32> {
    if SHOWN_COUNT.load(Ordering::Acquire) == 0 {
        return None;
    }
    minted_kind_of(object_of_id(id))
}

pub(crate) struct SetUpWindow {
    object: usize,
    source: i32,
}

impl Drop for SetUpWindow {
    fn drop(&mut self) {
        unsafe { write_kind(self.object, self.source) };
    }
}

pub(crate) unsafe fn set_up_window(object: usize) -> Option<SetUpWindow> {
    if object == 0 {
        return None;
    }
    let entry = find(object)?;
    if read_kind(object) != entry.source {
        return None;
    }
    write_kind(object, entry.minted);
    Some(SetUpWindow { object, source: entry.source })
}

unsafe fn show_source(object: usize) {
    let minted = read_kind(object);
    let Some(source) = custom_articles::custom_weapon_source_kind(minted) else {
        return;
    };
    if !custom_articles::shows_original_kind(minted) {
        return;
    }
    remember(Shown { object, minted, source });
    write_kind(object, source);
    if SHOW_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(&format!(
            "[articlekind] weapon kind {minted} object {object:#x} shows its source kind {source} to the game"
        ));
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_WEAPON_CONSTRUCT))]
unsafe fn weapon_construct(this: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64) -> u64 {
    if this != 0 {
        forget((this as usize).wrapping_sub(WEAPON_SUBOBJECT));
    }
    call_original!(this, a1, a2, a3, a4, a5, a6, a7)
}

#[skyline::hook(offset = crate::game::rel(OFF_WEAPON_INIT))]
unsafe fn weapon_init(object: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64) -> u64 {
    if object == 0 {
        return call_original!(object, a1, a2, a3, a4, a5, a6, a7);
    }
    let window = set_up_window(object as usize);
    let result = call_original!(object, a1, a2, a3, a4, a5, a6, a7);
    if window.is_none() {
        show_source(object as usize);
    }
    drop(window);
    result
}

#[skyline::hook(offset = crate::game::rel(OFF_WEAPON_TEARDOWN))]
unsafe fn weapon_teardown(this: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    if this != 0 {
        let object = (this as usize).wrapping_sub(WEAPON_SUBOBJECT);
        if let Some(entry) = forget(object) {
            if read_kind(object) == entry.source {
                write_kind(object, entry.minted);
            }
            if RESTORE_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
                crate::dbg_log_public(&format!(
                    "[articlekind] weapon kind {} object {object:#x} torn down as itself",
                    entry.minted
                ));
            }
        }
    }
    call_original!(this, a1, a2, a3)
}

unsafe fn through(original: &AtomicUsize, args: [u64; 8]) -> u64 {
    let function: LifecycleFn = core::mem::transmute(original.load(Ordering::Acquire));
    let _window = set_up_window(args[0] as usize);
    function(args[0], args[1], args[2], args[3], args[4], args[5], args[6], args[7])
}

unsafe extern "C" fn weapon_start(a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64) -> u64 {
    through(&ORIGINAL_START, [a0, a1, a2, a3, a4, a5, a6, a7])
}

unsafe extern "C" fn weapon_stop(a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64) -> u64 {
    through(&ORIGINAL_STOP, [a0, a1, a2, a3, a4, a5, a6, a7])
}

unsafe fn wrap_slot(slot: usize, expected: usize, original: &AtomicUsize, wrapper: LifecycleFn) -> Result<(), String> {
    let address = crate::game::addr(WEAPON_VTABLE + slot * 8);
    let held = core::ptr::read_volatile(address as *const usize);
    if held != crate::game::addr(expected) {
        return Err(format!(
            "weapon vtable slot {slot} holds {:#x}, not the game's {expected:#x}",
            crate::game::offset_of(held)
        ));
    }
    original.store(held, Ordering::Release);
    if !crate::text_patch::write_bytes(address, &(wrapper as usize).to_le_bytes()) {
        return Err(format!("weapon vtable slot {slot} could not be written"));
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_article_kind_v1(module_accessor: u64) -> i32 {
    if module_accessor == 0 {
        return -1;
    }
    if let Some(minted) = minted_kind_of_boma(module_accessor as usize) {
        return minted;
    }
    smash::app::utility::get_kind(&mut *(module_accessor as *mut smash::app::BattleObjectModuleAccessor))
}

pub(crate) fn install() {
    if !enabled() {
        skyline::println!("[articlekind] off: cloned articles show their own weapon kind (settings.toml article_kind_facade = false)");
        return;
    }
    let wrapped = unsafe {
        wrap_slot(SLOT_START, START_ENTRY, &ORIGINAL_START, weapon_start)
            .and_then(|()| wrap_slot(SLOT_STOP, STOP_ENTRY, &ORIGINAL_STOP, weapon_stop))
    };
    if let Err(why) = wrapped {
        if ORIGINAL_START.load(Ordering::Acquire) != 0 && ORIGINAL_STOP.load(Ordering::Acquire) == 0 {
            let restore = ORIGINAL_START.load(Ordering::Acquire);
            unsafe {
                crate::text_patch::write_bytes(crate::game::addr(WEAPON_VTABLE + SLOT_START * 8), &restore.to_le_bytes());
            }
        }
        skyline::println!("[articlekind] REFUSED: {why}; cloned articles keep their own weapon kind");
        return;
    }
    skyline::install_hooks!(weapon_construct, weapon_init, weapon_teardown);
    skyline::println!("[articlekind] cloned articles show their source weapon kind to the game once built");
}
