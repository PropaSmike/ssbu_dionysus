use dionysus_core::menu::{self, GalleryItem};
use std::sync::{Mutex, OnceLock};

const OFF_INTERFACE: usize = 0x532e8b0;
const OFF_WORLD: usize = 0x532e7a0;
const OFF_FIGHTER_MANAGER: usize = 0x52b84f8;
const OFF_SIDE_BAR: usize = 0x532e7f0;
const OFF_SET_SIDE_BAR: usize = 0x32eacc0;
const SET_SIDE_BAR_OPCODE: u32 = 0xf81d0ff5;
const MANAGER: usize = 8;
const SIDE_BAR: usize = 8;
const SLOTS: i64 = 2;
const SLOT_SIZE: usize = 0xb8;
const SLOT_TYPE: usize = 0x150;
const TYPE_SHOWN: u32 = 2;
const STEPS: usize = 0xe8;
const STEP_TOP: usize = 0x134;
const STEP_CAPACITY: i32 = 0x13;
const STEP_RELOAD: u32 = 4;
const WORLD_SLOT_SIZE: usize = 0x50;
const WORLD_SLOT_STATE: usize = 0x48;
const WORLD_SLOT_LOADING: u32 = 1;
const ENTRIES: usize = 0x20;
const ENTRY_INDEX: usize = 0x5918;
const ENTRY_COUNT: usize = 0x4150;
const ENTRY_FIGHTERS: usize = 0x4160;
const ENTRY_ACTIVATION_OBJECT: usize = 0x4178;
const VT_IS_USABLE: usize = 0x518;
const FIGHTER_PHASE: usize = 0x3a;
const PHASE_BUILT: u8 = 3;
const REFUSED: i64 = -1;
const LOADING: i64 = 0;
const READY: i64 = 1;
const SHOWN: i64 = 2;
const HOST_MODULE: &str = "menu/chara_make/make_mii_select_actor";
const HOST_LAYOUTS: u64 = 2;
const DESCRIPTOR_WORDS: usize = 4;

type SetSideBarFn = unsafe extern "C" fn(usize, u32);
type IsUsableFn = unsafe extern "C" fn(usize) -> u32;

static GALLERIES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static ITEMS: Mutex<Vec<GalleryItem>> = Mutex::new(Vec::new());
static LAYOUTS: Mutex<Vec<u64>> = Mutex::new(Vec::new());
static MUSIC: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static HOST_DESCRIPTOR: OnceLock<usize> = OnceLock::new();

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn through(global: usize, offset: usize) -> Option<usize> {
    let holder: usize = read(crate::game::addr(global));
    if holder == 0 {
        return None;
    }
    let value: usize = read(holder + offset);
    (value != 0).then_some(value)
}

unsafe fn manager() -> Option<usize> {
    through(OFF_INTERFACE, MANAGER)
}

unsafe fn world() -> Option<usize> {
    through(OFF_WORLD, 0)
}

fn slot_index(slot: i64) -> Option<usize> {
    (0..SLOTS).contains(&slot).then_some(slot as usize)
}

unsafe fn set_side_bar(visible: bool) -> bool {
    if crate::game::text_word(OFF_SET_SIDE_BAR) != SET_SIDE_BAR_OPCODE {
        return false;
    }
    let Some(bar) = through(OFF_SIDE_BAR, SIDE_BAR) else {
        return false;
    };
    let set: SetSideBarFn = core::mem::transmute(crate::game::addr(OFF_SET_SIDE_BAR));
    set(bar, u32::from(visible));
    true
}

unsafe fn step_pending(manager: usize) -> bool {
    let top: i32 = read(manager + STEP_TOP);
    (0..=top.min(STEP_CAPACITY - 1)).any(|index| read::<u32>(manager + STEPS + index as usize * 4) == STEP_RELOAD)
}

unsafe fn fighter(slot: usize) -> Option<usize> {
    let fighters = through(OFF_FIGHTER_MANAGER, 0)?;
    let entry: usize = read(fighters + ENTRIES + slot * 8);
    if entry == 0 || read::<usize>(entry + ENTRY_ACTIVATION_OBJECT) == 0 {
        return None;
    }
    let index = usize::from(read::<u8>(entry + ENTRY_INDEX));
    let count: usize = read(entry + ENTRY_COUNT);
    if count <= index {
        return None;
    }
    let object: usize = read(entry + ENTRY_FIGHTERS + index * 8);
    (object != 0).then_some(object)
}

unsafe fn is_shown(object: usize) -> bool {
    let vtable: usize = read(object);
    let usable: IsUsableFn = core::mem::transmute(read::<usize>(vtable + VT_IS_USABLE));
    usable(object) & 1 != 0 && read::<u8>(object + FIGHTER_PHASE) > PHASE_BUILT
}

pub(crate) fn begin(slot: i64) -> i64 {
    let Some(slot) = slot_index(slot) else {
        return REFUSED;
    };
    unsafe {
        let (Some(manager), Some(_)) = (manager(), world()) else {
            return REFUSED;
        };
        core::ptr::write_volatile((manager + slot * SLOT_SIZE + SLOT_TYPE) as *mut u32, TYPE_SHOWN);
        let hidden = set_side_bar(false);
        skyline::println!("[menuviewer] slot {slot} open, side bar hidden: {hidden}");
    }
    0
}

pub(crate) fn state(slot: i64) -> i64 {
    let Some(slot) = slot_index(slot) else {
        return REFUSED;
    };
    unsafe {
        let (Some(manager), Some(world)) = (manager(), world()) else {
            return REFUSED;
        };
        if step_pending(manager) {
            return LOADING;
        }
        let Some(object) = fighter(slot) else {
            return LOADING;
        };
        if is_shown(object) {
            return SHOWN;
        }
        let world_state: u32 = read(world + slot * WORLD_SLOT_SIZE + WORLD_SLOT_STATE);
        if world_state == WORLD_SLOT_LOADING {
            return LOADING;
        }
        READY
    }
}

pub(crate) fn end() -> i64 {
    let shown = unsafe { set_side_bar(true) };
    skyline::println!("[menuviewer] closed, side bar shown: {shown}");
    0
}

#[no_mangle]
pub extern "C" fn clone_engine_viewer_begin(slot: u32) -> i32 {
    begin(i64::from(slot)) as i32
}

#[no_mangle]
pub extern "C" fn clone_engine_viewer_state(slot: u32) -> i32 {
    state(i64::from(slot)) as i32
}

#[no_mangle]
pub extern "C" fn clone_engine_viewer_end() -> i32 {
    end() as i32
}

#[no_mangle]
pub extern "C" fn clone_engine_gallery_count() -> i32 {
    item_count() as i32
}

#[no_mangle]
pub extern "C" fn clone_engine_gallery_costume(index: u32) -> i32 {
    item_costume(i64::from(index)) as i32
}

pub(crate) fn declare(galleries: Vec<String>, items: Vec<GalleryItem>, layouts: Vec<u64>, music: Vec<(String, String)>) {
    if let Ok(mut known) = MUSIC.lock() {
        known.extend(music);
    }
    if let Ok(mut known) = GALLERIES.lock() {
        known.extend(galleries);
    }
    if let Ok(mut known) = ITEMS.lock() {
        known.extend(items);
    }
    if let Ok(mut known) = LAYOUTS.lock() {
        known.extend(layouts);
    }
}

fn active_mode() -> Option<usize> {
    let index = menu::mode_of_code(crate::menu_sequences::active())?;
    let id = crate::menu_modes::id_of(index)?;
    GALLERIES.lock().ok()?.iter().any(|known| known == id).then_some(index)
}

pub(crate) fn gallery_active() -> bool {
    active_mode().is_some()
}

fn items() -> Vec<GalleryItem> {
    let Some(gallery) = active_mode().and_then(crate::menu_modes::id_of) else {
        return Vec::new();
    };
    ITEMS
        .lock()
        .map(|items| items.iter().filter(|item| item.gallery == gallery).cloned().collect())
        .unwrap_or_default()
}

fn item(index: i64) -> Option<GalleryItem> {
    usize::try_from(index).ok().and_then(|index| items().into_iter().nth(index))
}

pub(crate) fn item_count() -> i64 {
    if active_mode().is_none() {
        return REFUSED;
    }
    items().len() as i64
}

pub(crate) fn item_costume(index: i64) -> i64 {
    item(index).map_or(REFUSED, |item| i64::from(item.costume))
}

pub(crate) fn text(name: &str, index: i64) -> Option<String> {
    match name {
        "engine.gallery_title" => crate::menu_modes::text("engine.mode_name", active_mode()? as i64),
        "engine.gallery_fighter" => item(index).map(|item| item.fighter),
        "engine.gallery_name" => item(index).map(|item| item.name),
        "engine.gallery_text" => item(index).map(|item| item.text),
        "engine.gallery_music" => {
            let id = crate::menu_modes::id_of(active_mode()?)?;
            MUSIC.lock().ok()?.iter().find(|(gallery, _)| gallery == id).map(|(_, music)| music.clone())
        }
        "engine.gallery_group" => item(index).map(|item| item.group),
        "engine.gallery_sources" => item(index).map(|item| item.sources),
        _ => None,
    }
}

unsafe fn extended_descriptor(descriptor: usize) -> usize {
    let extra = LAYOUTS.lock().map(|layouts| layouts.clone()).unwrap_or_default();
    let count: u64 = read(descriptor + 8);
    if extra.is_empty() || count != HOST_LAYOUTS {
        skyline::println!("[menuviewer] the host screen keeps its layouts: {} pack layout(s), {count} of its own", extra.len());
        return 0;
    }
    let table: usize = read(descriptor);
    let mut layouts: Vec<u64> = (0..count as usize).map(|index| read(table + index * 8)).collect();
    layouts.extend(extra);
    let layouts: &'static [u64] = Box::leak(layouts.into_boxed_slice());
    let mut words = [0u64; DESCRIPTOR_WORDS];
    for (index, word) in words.iter_mut().enumerate() {
        *word = read(descriptor + index * 8);
    }
    words[0] = layouts.as_ptr() as u64;
    words[1] = layouts.len() as u64;
    Box::leak(Box::new(words)).as_ptr() as usize
}

pub(crate) fn scene_descriptor(hash: u64, descriptor: usize) -> usize {
    if hash & menu::HASH_MASK != menu::script_hash(menu::ScriptRoot::Script, HOST_MODULE) || active_mode().is_none() {
        return descriptor;
    }
    match *HOST_DESCRIPTOR.get_or_init(|| unsafe { extended_descriptor(descriptor) }) {
        0 => descriptor,
        extended => extended,
    }
}
