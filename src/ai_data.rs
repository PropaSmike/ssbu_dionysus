use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Mutex;

const MANAGER: usize = 0x52b_60e8;
const SLOT_BASE: usize = 0x728;
const SLOT_STRIDE: usize = 0x50;
const LIST_HOLDER: usize = 0x0;
const DATA_HOLDER: usize = 0x8;
const LIST_COUNT: usize = 0x10;
const DATA_COUNT: usize = 0x18;
const LIST_HANDLE: usize = 0x20;
const HANDLE_INDEX: usize = 0x10;
const LIST_VTABLE: usize = 0x4f6_1148;
const DATA_VTABLE: usize = 0x4f6_1108;
const ALLOCATE: usize = 0x392_dce0;
const BIND: usize = 0x372_0910;
const SET_HANDLE: usize = 0x372_0680;
const RELEASE_SITE: usize = 0x2c5_dd0;
const RELEASE_SITE_WORD: u32 = 0xB94C_0668;
const ATTACK_MAP_KIND: usize = 0x2f2_840;
const ATTACK_MAP_KIND_WORD: u32 = 0x5100_F029;
const AI_SETUP_KIND: usize = 0xc04;
const AI_DATA_KIND: usize = 0x988;
const AI_HELD: usize = 0x8;
const AI_REGISTER: usize = 19;
const KIND_REGISTER: usize = 1;
const HOLDER_ALIGN: u64 = 0x10;
const HOLDER_SIZE: u64 = 0x20;
const NO_FILE: u32 = 0xFF_FFFF;
const KINDS: usize = 0x5e;
const SPARE: core::ops::RangeInclusive<i32> = 1..=92;
const ENTRIES: i32 = 8;
const SHOWN: u32 = 8;
const FILES: [&str; 2] = ["attack_list_param.prc", "attack_data_param.prc"];
const SPARE_FIRST: [&str; 3] = ["miienemyf", "miienemyg", "miienemys"];
const TOGETHER: [&[&str]; 4] = [
    &["koopa", "koopag"],
    &["popo", "nana"],
    &["eflame", "elight"],
    &["pzenigame", "pfushigisou", "plizardon"],
];

type Allocate = unsafe extern "C" fn(u64, u64) -> *mut u8;
type Bind = unsafe extern "C" fn(usize, u32) -> bool;
type SetHandle = unsafe extern "C" fn(usize, *const u32) -> bool;
type Destroy = unsafe extern "C" fn(usize);

struct Shared {
    clone: i32,
    slot: i32,
    users: u32,
    list: usize,
    data: usize,
}

static ARMED: AtomicBool = AtomicBool::new(false);
static LOGIC: [AtomicI32; KINDS] = [const { AtomicI32::new(-1) }; KINDS];
static USERS: Mutex<Vec<(usize, i32)>> = Mutex::new(Vec::new());
static SHARED: Mutex<Vec<Shared>> = Mutex::new(Vec::new());
static LOGGED: AtomicU32 = AtomicU32::new(0);

fn log(text: String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < SHOWN {
        skyline::println!("{text}");
    }
}

fn kind_named(name: &str) -> Option<i32> {
    crate::custom_articles::fighter_kind_from_str(name).filter(|kind| SPARE.contains(kind))
}

fn name_of(kind: i32) -> String {
    crate::custom_articles::fighter_name(kind)
        .and_then(|name| name.to_str().ok().map(str::to_string))
        .unwrap_or_else(|| format!("kind {kind}"))
}

fn logic_kind(kind: i32) -> Option<i32> {
    if !ARMED.load(Ordering::Acquire) || !(0..KINDS as i32).contains(&kind) {
        return None;
    }
    let logic = LOGIC[kind as usize].load(Ordering::Acquire);
    (logic >= 0).then_some(logic)
}

unsafe fn read(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

unsafe fn write(address: usize, value: usize) {
    core::ptr::write_volatile(address as *mut usize, value)
}

unsafe fn manager() -> usize {
    let holder = read(crate::game::addr(MANAGER));
    if holder == 0 {
        return 0;
    }
    read(holder)
}

fn slot_at(manager: usize, slot: i32) -> usize {
    manager + SLOT_BASE + slot as usize * SLOT_STRIDE
}

unsafe fn slot_empty(manager: usize, slot: i32) -> bool {
    let at = slot_at(manager, slot);
    let index = core::ptr::read_volatile((at + LIST_HANDLE + HANDLE_INDEX) as *const u32);
    read(at + LIST_HOLDER) == 0
        && read(at + DATA_HOLDER) == 0
        && read(at + LIST_COUNT) == 0
        && read(at + DATA_COUNT) == 0
        && (index == NO_FILE || index == 0)
}

#[cfg(feature = "clone_runtime")]
unsafe fn match_kinds() -> Vec<i32> {
    let mut kinds = Vec::new();
    for entry in 0..ENTRIES {
        let (count, first, second, third) = crate::load_pipeline::entry_kind_array(entry);
        if count > 8 {
            continue;
        }
        kinds.extend([first, second, third].into_iter().filter(|kind| *kind >= 0));
    }
    kinds
}

#[cfg(not(feature = "clone_runtime"))]
unsafe fn match_kinds() -> Vec<i32> {
    Vec::new()
}

unsafe fn taken_kinds() -> Vec<i32> {
    let mut taken = Vec::new();
    for kind in match_kinds() {
        taken.push(kind);
        taken.extend(crate::clone_base_of(kind));
        taken.extend(crate::cpu_ai::chosen(kind));
    }
    let names: Vec<String> = taken.iter().map(|kind| name_of(*kind)).collect();
    for group in TOGETHER {
        if group.iter().any(|name| names.iter().any(|known| known == name)) {
            taken.extend(group.iter().filter_map(|name| crate::custom_articles::fighter_kind_from_str(name)));
        }
    }
    taken
}

unsafe fn choose(manager: usize, held: &[Shared]) -> Option<i32> {
    let taken = taken_kinds();
    let first: Vec<i32> = SPARE_FIRST.iter().filter_map(|name| kind_named(name)).collect();
    let mut order = first.clone();
    order.extend(SPARE.rev().filter(|kind| !first.contains(kind)));
    order.into_iter().find(|kind| {
        !taken.contains(kind) && !held.iter().any(|entry| entry.slot == *kind) && slot_empty(manager, *kind)
    })
}

unsafe fn holder(vtable: usize) -> usize {
    let allocate: Allocate = core::mem::transmute(crate::game::addr(ALLOCATE));
    let made = allocate(HOLDER_ALIGN, HOLDER_SIZE) as usize;
    if made != 0 {
        write(made, crate::game::addr(vtable));
        write(made + 0x8, 0);
        write(made + 0x10, 0);
        write(made + 0x18, 0);
    }
    made
}

unsafe fn destroy(object: usize) {
    if object == 0 {
        return;
    }
    let function: Destroy = core::mem::transmute(read(read(object) + 0x8));
    function(object);
}

unsafe fn fill(manager: usize, slot: i32, list: u32, data: u32) -> Option<(usize, usize)> {
    let at = slot_at(manager, slot);
    let bind: Bind = core::mem::transmute(crate::game::addr(BIND));
    let set_handle: SetHandle = core::mem::transmute(crate::game::addr(SET_HANDLE));
    let list_holder = holder(LIST_VTABLE);
    let data_holder = holder(DATA_VTABLE);
    if list_holder == 0 || data_holder == 0 {
        destroy(list_holder);
        destroy(data_holder);
        return None;
    }
    write(at + LIST_HOLDER, list_holder);
    bind(list_holder, list);
    set_handle(at + LIST_HANDLE, &list);
    write(at + LIST_COUNT, 1);
    write(at + DATA_HOLDER, data_holder);
    bind(data_holder, data);
    write(at + DATA_COUNT, 1);
    Some((list_holder, data_holder))
}

unsafe fn empty(manager: usize, slot: i32) {
    let at = slot_at(manager, slot);
    let set_handle: SetHandle = core::mem::transmute(crate::game::addr(SET_HANDLE));
    let list = read(at + LIST_HOLDER);
    write(at + LIST_HOLDER, 0);
    destroy(list);
    let none = NO_FILE;
    set_handle(at + LIST_HANDLE, &none);
    let data = read(at + DATA_HOLDER);
    write(at + DATA_HOLDER, 0);
    destroy(data);
}

unsafe fn own_files(clone: i32) -> Option<(u32, u32, String)> {
    let resource = crate::clone_definition(clone)?.resource_name;
    let folder = format!("fighter/{resource}/ai/param");
    let list = crate::fighter_modules::file_path_index(&format!("{folder}/{}", FILES[0]))?;
    let data = crate::fighter_modules::file_path_index(&format!("{folder}/{}", FILES[1]))?;
    Some((list, data, folder))
}

pub(crate) unsafe fn assign(ai: usize, clone: i32) {
    if !ARMED.load(Ordering::Acquire) || ai == 0 {
        return;
    }
    if let Some(slot) = USERS.lock().ok().and_then(|users| users.iter().find(|(known, _)| *known == ai).map(|(_, slot)| *slot)) {
        core::ptr::write_volatile((ai + AI_DATA_KIND) as *mut i32, slot);
        return;
    }
    let Some((list, data, folder)) = own_files(clone) else {
        return;
    };
    if !(crate::fighter_modules::data_ready(list) && crate::fighter_modules::data_ready(data)) {
        log(format!(
            "[aidata] kind {clone}: {folder} holds its own AI files but they did not load with the fighter (list them in its costume groups); its CPU keeps its base's"
        ));
        return;
    }
    let state = read(ai);
    if state == 0 || core::ptr::read_volatile((state + AI_HELD) as *const u8) == 0 {
        log(format!("[aidata] kind {clone}: the game did not take AI data for this CPU; its own AI files stay unused"));
        return;
    }
    let manager = manager();
    if manager == 0 {
        return;
    }
    let logic = core::ptr::read_volatile((ai + AI_SETUP_KIND) as *const i32);
    let Ok(mut shared) = SHARED.lock() else {
        return;
    };
    let slot = if let Some(entry) = shared.iter_mut().find(|entry| entry.clone == clone) {
        let at = slot_at(manager, entry.slot);
        write(at + LIST_COUNT, read(at + LIST_COUNT) + 1);
        write(at + DATA_COUNT, read(at + DATA_COUNT) + 1);
        entry.users += 1;
        entry.slot
    } else {
        let Some(slot) = choose(manager, &shared) else {
            log(format!("[aidata] kind {clone}: no free AI slot in this match; its CPU keeps its base's AI data"));
            return;
        };
        let Some((list_holder, data_holder)) = fill(manager, slot, list, data) else {
            return;
        };
        shared.push(Shared { clone, slot, users: 1, list: list_holder, data: data_holder });
        slot
    };
    drop(shared);
    LOGIC[slot as usize].store(logic, Ordering::Release);
    if let Ok(mut users) = USERS.lock() {
        users.push((ai, slot));
    }
    core::ptr::write_volatile((ai + AI_DATA_KIND) as *mut i32, slot);
    log(format!(
        "[aidata] kind {clone}'s CPU plays from its own attack list and data ({folder}, held in {}'s slot, kind {slot}); its AI code is {}'s",
        name_of(slot),
        name_of(logic)
    ));
}

unsafe fn release(ai: usize) {
    let Some(slot) = USERS.lock().ok().and_then(|mut users| {
        let found = users.iter().position(|(known, _)| *known == ai)?;
        Some(users.remove(found).1)
    }) else {
        return;
    };
    let manager = manager();
    let Ok(mut shared) = SHARED.lock() else {
        return;
    };
    let Some(position) = shared.iter().position(|entry| entry.slot == slot) else {
        return;
    };
    if manager == 0 {
        shared.remove(position);
        LOGIC[slot as usize].store(-1, Ordering::Release);
        return;
    }
    let at = slot_at(manager, slot);
    let list_count = read(at + LIST_COUNT).saturating_sub(1);
    let data_count = read(at + DATA_COUNT).saturating_sub(1);
    write(at + LIST_COUNT, list_count);
    write(at + DATA_COUNT, data_count);
    shared[position].users = shared[position].users.saturating_sub(1);
    if shared[position].users > 0 {
        return;
    }
    let entry = shared.remove(position);
    LOGIC[slot as usize].store(-1, Ordering::Release);
    let ours = read(at + LIST_HOLDER) == entry.list && read(at + DATA_HOLDER) == entry.data;
    if list_count == 0 && data_count == 0 && ours {
        empty(manager, slot);
        log(format!("[aidata] kind {}'s own AI data released from {}'s slot", entry.clone, name_of(slot)));
    }
}

#[skyline::hook(offset = crate::game::rel(RELEASE_SITE), inline)]
unsafe fn ai_release_site(ctx: &mut skyline::hooks::InlineCtx) {
    release(ctx.registers[AI_REGISTER].x() as usize);
}

#[skyline::hook(offset = crate::game::rel(ATTACK_MAP_KIND), inline)]
unsafe fn ai_attack_map_kind(ctx: &mut skyline::hooks::InlineCtx) {
    if let Some(logic) = logic_kind(ctx.registers[KIND_REGISTER].w() as i32) {
        ctx.registers[KIND_REGISTER].set_x(logic as u64);
    }
}

pub(crate) fn install() {
    let release_word = unsafe { crate::game::text_word(RELEASE_SITE) };
    let map_word = unsafe { crate::game::text_word(ATTACK_MAP_KIND) };
    if release_word != RELEASE_SITE_WORD || map_word != ATTACK_MAP_KIND_WORD {
        skyline::println!(
            "[aidata] AI code not recognised ({release_word:#010x}, {map_word:#010x}); clones' own AI files stay unused"
        );
        return;
    }
    skyline::install_hooks!(ai_release_site, ai_attack_map_kind);
    ARMED.store(true, Ordering::Release);
}
