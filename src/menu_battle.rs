use dionysus_core::menu;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

const OFF_FIGHTER_INFO: usize = 0x5307798;
const FIGHTER_INFO_STRIDE: usize = 0x1c8;
const OFF_PLAYER_PAD: usize = 0x53085b0;
const PLAYER_PAD_STRIDE: usize = 0x240;
const INFO_TYPE: usize = 0x8;
const INFO_CHARA: usize = 0x10;
const INFO_COLOR: usize = 0x20;
const INFO_START_DAMAGE: usize = 0x22;
const PAD_INFO: usize = 0x88;
const OFF_STAGE: usize = 0x53097b8;
const OFF_RULE_KIND: usize = 0x5309975;
const OFF_RULE_TIME: usize = 0x5309978;
const OFF_RULE_STOCK: usize = 0x530997a;
const OFF_RULE_ITEMS: usize = 0x53099a0;
const OFF_RULE_CAN_PAUSE: usize = 0x5309989;
const OFF_FIGHTER_MANAGER: usize = 0x52b84f8;
const OFF_BOSS_BATTLE: usize = 0x52c4268;
const BOSS_KIND: usize = 4;
const NO_BOSS: i32 = i32::MIN;
const OFF_BOSS_MANAGER: usize = 0x52b7ef8;
const BOSS_MANAGER_DATA: usize = 8;
const BOSS_ENTRIES: usize = 0x110;
const BOSS_ENTRY_ITEM: usize = 8;
const BOSS_STATE: usize = 0x100;
const BOSS_STATE_FINISH: i32 = 6;
const ITEM_OBJECT_ID: usize = 8;
const NO_OBJECT: u32 = 0x5000_0000;
const OFF_SETTINGS_RESET: usize = 0x178b7b0;
const SETTINGS_RESET_OPCODE: u32 = 0xd102c3ff;
const OFF_BATTLE_CTOR: usize = 0x2317e10;
const BATTLE_CTOR_OPCODE: u32 = 0xd10103ff;
const OFF_BATTLE_DTOR: usize = 0x2318060;
const BATTLE_DTOR_OPCODE: u32 = 0xf81c0ff7;
const CONTROLLER_STATE: usize = 0x50 + 0x30;
const CHILD_ENDS: u32 = 2;
const OFF_FLOW_BUILD: usize = 0x13444c0;
const FLOW_BUILD_OPCODE: u32 = 0xa9bd57f6;
const OFF_FLOW_HOLDER: usize = 0x52c2760;
const OFF_APPEARANCE_START: usize = 0x14f6c60;
const APPEARANCE_START_OPCODE: u32 = 0xd10203ff;
const FLOW_NO_ENTRANCE: usize = 0x73;
const OFF_INFO_START: usize = 0x1392a40;
const INFO_START_OPCODE: u32 = 0xf81e0ff3;
const OFF_QUEUED_NARRATION: usize = 0x156c0f8;
const QUEUED_NARRATION_OPCODE: u32 = 0xf9400a81;
const QUEUE_NODE: usize = 20;
const QUEUE_NODE_SOUND: usize = 0x10;
const OFF_GO_FROM_UPDATE: usize = 0x158b8e0;
const GO_FROM_UPDATE_OPCODE: u32 = 0x9439ee68;
const OFF_GO_FROM_START: usize = 0x156db38;
const GO_FROM_START_OPCODE: u32 = 0x943a65d2;
const SOUND_ARGUMENT: usize = 1;
const NARRATION_READY_GO: [u64; 5] = [0x123590b7e3, 0xf2ab48f10, 0x125bc0eab9, 0x108ee9b2df, 0x10e54fbe48];
const SILENT_SOUND: u64 = 0;
const INFO_REQUEST: usize = 0xf8;
const INFO_NEXT: usize = 0xfc;
const INFO_READY_GO: [u32; 4] = [1, 2, 3, 10];
const INFO_STATE_INDEX: usize = 0xdc;
const INFO_STATES: usize = 0x90;
const INFO_STATE_COUNT: i32 = 0x13;
const INFO_DONE: u32 = 3;

static BATTLE: AtomicUsize = AtomicUsize::new(0);
static BATTLE_SERIAL: AtomicUsize = AtomicUsize::new(0);
static BATTLE_FRAMES: AtomicUsize = AtomicUsize::new(0);
static SKIP_ENTRANCE: AtomicBool = AtomicBool::new(false);
static ENTRANCE_ARMED: AtomicBool = AtomicBool::new(false);
static SKIPPING_FLOW: AtomicUsize = AtomicUsize::new(0);
static SKIP_READY_GO: AtomicBool = AtomicBool::new(false);
static BOSS: AtomicI32 = AtomicI32::new(NO_BOSS);
static BOSS_CLONE: AtomicI32 = AtomicI32::new(NO_BOSS);
static READY_GO_ARMED: AtomicBool = AtomicBool::new(false);
static QUIET_FLOW: AtomicUsize = AtomicUsize::new(0);
static LAST_RULES: Mutex<Option<(u32, u32, u16, u32)>> = Mutex::new(None);
static CAN_PAUSE: AtomicBool = AtomicBool::new(true);
static LAST_STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    Slot,
    Type,
    Hash,
    NoBattle,
    Busy,
    Mii,
}

impl Refusal {
    pub(crate) fn code(self) -> i32 {
        match self {
            Refusal::Slot => -1,
            Refusal::Type => -2,
            Refusal::Hash => -3,
            Refusal::NoBattle => -4,
            Refusal::Busy => -5,
            Refusal::Mii => -6,
        }
    }
}

const MII_FIGHTERS: [&str; 3] = ["miifighter", "miiswordsman", "miigunner"];

fn is_mii_fighter(hash: u64) -> bool {
    if MII_FIGHTERS
        .iter()
        .any(|name| crate::hash40::hash40(&format!("ui_chara_{name}")) == hash)
    {
        return true;
    }
    crate::clone_definitions()
        .read()
        .map(|definitions| {
            definitions.iter().any(|definition| {
                crate::hash40::hash40(definition.ui_chara) == hash
                    && MII_FIGHTERS.contains(&definition.base_resource_name)
            })
        })
        .unwrap_or(false)
}

unsafe fn write<T>(address: usize, value: T) {
    core::ptr::write_volatile(address as *mut T, value);
}

pub(crate) fn set_player(slot: u32, kind: u32, chara: u64, pad: i32) -> Result<(), Refusal> {
    let slot = slot as usize;
    if slot >= menu::BATTLE_PLAYERS {
        return Err(Refusal::Slot);
    }
    if kind >= menu::BATTLE_PLAYER_TYPES {
        return Err(Refusal::Type);
    }
    let hash = chara & menu::HASH_MASK;
    if hash == 0 && kind != 3 {
        return Err(Refusal::Hash);
    }
    if kind != 3 && is_mii_fighter(hash) {
        skyline::println!(
            "[menubattle] player {} refused: {hash:#012x} is a Mii Fighter, whose Mii only the select screen sets up; the slot is left empty",
            slot + 1
        );
        let _ = set_player(slot as u32, 3, 0, -1);
        return Err(Refusal::Mii);
    }
    let value = menu::tagged(menu::CHARA_TAG, hash);
    unsafe {
        let info = crate::game::addr(OFF_FIGHTER_INFO) + slot * FIGHTER_INFO_STRIDE;
        let pad_record = crate::game::addr(OFF_PLAYER_PAD) + slot * PLAYER_PAD_STRIDE;
        write(info + INFO_TYPE, kind);
        write(info + INFO_CHARA, value);
        write(info + INFO_CHARA + 8, value);
        write(pad_record + PAD_INFO + INFO_TYPE, kind);
        write(pad_record + PAD_INFO + INFO_CHARA, value);
        write(pad_record + PAD_INFO + INFO_CHARA + 8, value);
        write(info + INFO_COLOR, 0u8);
        write(pad_record + PAD_INFO + INFO_COLOR, 0u8);
        write(info + INFO_START_DAMAGE, 0u16);
        write(pad_record + PAD_INFO + INFO_START_DAMAGE, 0u16);
        write(pad_record, if kind == 0 { pad } else { -1 });
        write(pad_record + 4, 0i32);
    }
    skyline::println!(
        "[menubattle] player {} type {kind} fighter {hash:#012x} pad {pad}",
        slot + 1
    );
    Ok(())
}

pub(crate) fn set_damage(slot: u32, percent: u32) -> Result<(), Refusal> {
    let slot = slot as usize;
    if slot >= menu::BATTLE_PLAYERS {
        return Err(Refusal::Slot);
    }
    let percent = percent.min(menu::MAX_START_DAMAGE) as u16;
    unsafe {
        let info = crate::game::addr(OFF_FIGHTER_INFO) + slot * FIGHTER_INFO_STRIDE;
        let pad_record = crate::game::addr(OFF_PLAYER_PAD) + slot * PLAYER_PAD_STRIDE;
        write(info + INFO_START_DAMAGE, percent);
        write(pad_record + PAD_INFO + INFO_START_DAMAGE, percent);
    }
    skyline::println!("[menubattle] player {} starts at {percent}%", slot + 1);
    Ok(())
}

pub(crate) fn set_color(slot: u32, color: u32) -> Result<(), Refusal> {
    let slot = slot as usize;
    if slot >= menu::BATTLE_PLAYERS {
        return Err(Refusal::Slot);
    }
    if color > menu::MAX_BATTLE_COLOR {
        return Err(Refusal::Type);
    }
    unsafe {
        let info = crate::game::addr(OFF_FIGHTER_INFO) + slot * FIGHTER_INFO_STRIDE;
        let pad_record = crate::game::addr(OFF_PLAYER_PAD) + slot * PLAYER_PAD_STRIDE;
        write(info + INFO_COLOR, color as u8);
        write(pad_record + PAD_INFO + INFO_COLOR, color as u8);
    }
    skyline::println!("[menubattle] player {} wears color {color}", slot + 1);
    Ok(())
}

unsafe fn write_rules(kind: u32, stock: u32, seconds: u16, items: u32) {
    write(crate::game::addr(OFF_RULE_KIND), kind as u8);
    write(crate::game::addr(OFF_RULE_TIME), seconds);
    write(crate::game::addr(OFF_RULE_STOCK), stock as u8);
    write(crate::game::addr(OFF_RULE_ITEMS), items as u8);
    write(crate::game::addr(OFF_RULE_CAN_PAUSE), u8::from(CAN_PAUSE.load(Ordering::Acquire)));
}

pub(crate) fn set_pause(allowed: bool) -> Result<(), Refusal> {
    CAN_PAUSE.store(allowed, Ordering::Release);
    unsafe { write(crate::game::addr(OFF_RULE_CAN_PAUSE), u8::from(allowed)) };
    skyline::println!("[menubattle] pausing {}", if allowed { "on" } else { "off" });
    Ok(())
}

pub(crate) fn set_rules(kind: u32, stock: u32, seconds: u32, items: u32) -> Result<(), Refusal> {
    if kind >= menu::BATTLE_RULE_KINDS || stock > menu::MAX_RULE_STOCK || items > menu::MAX_RULE_ITEMS {
        return Err(Refusal::Type);
    }
    let seconds = seconds.min(menu::MAX_RULE_SECONDS) as u16;
    unsafe { write_rules(kind, stock, seconds, items) };
    if let Ok(mut last) = LAST_RULES.lock() {
        *last = Some((kind, stock, seconds, items));
    }
    skyline::println!("[menubattle] rules: kind {kind}, {stock} stock, {seconds} s, items {items}");
    Ok(())
}

pub(crate) fn reapply(rules: bool, stage: bool) {
    if rules {
        if let Some((kind, stock, seconds, items)) = LAST_RULES.lock().ok().and_then(|last| *last) {
            unsafe { write_rules(kind, stock, seconds, items) };
            skyline::println!("[menubattle] rules again before the battle: kind {kind}, {stock} stock, {seconds} s, items {items}");
        }
    }
    if stage {
        let value = LAST_STAGE.load(Ordering::Acquire);
        if value != 0 {
            unsafe {
                let address = crate::game::addr(OFF_STAGE);
                write(address, value);
                write(address + 8, value);
            }
            skyline::println!("[menubattle] stage again before the battle: {:#012x}", value & menu::HASH_MASK);
        }
    }
}

pub(crate) fn stock_left(slot: u32) -> Result<u32, Refusal> {
    if slot as usize >= menu::BATTLE_PLAYERS {
        return Err(Refusal::Slot);
    }
    unsafe {
        let manager: usize = core::ptr::read_volatile(crate::game::addr(OFF_FIGHTER_MANAGER) as *const usize);
        if manager == 0 {
            return Err(Refusal::NoBattle);
        }
        let info = smash::app::lua_bind::FighterManager::get_fighter_information(
            manager as *mut smash::app::FighterManager,
            smash::app::FighterEntryID(slot as i32),
        );
        if info.is_null() {
            return Err(Refusal::NoBattle);
        }
        Ok(smash::app::lua_bind::FighterInformation::stock_count(info) as u32)
    }
}

pub(crate) fn set_stage(stage: u64) -> Result<(), Refusal> {
    let hash = stage & menu::HASH_MASK;
    if hash == 0 {
        return Err(Refusal::Hash);
    }
    let value = menu::tagged(menu::STAGE_TAG, hash);
    unsafe {
        let address = crate::game::addr(OFF_STAGE);
        write(address, value);
        write(address + 8, value);
    }
    LAST_STAGE.store(value, Ordering::Release);
    skyline::println!("[menubattle] stage {hash:#012x}");
    Ok(())
}

pub(crate) fn set_boss_named(value: i64) -> Result<(), Refusal> {
    if value <= i64::from(i32::MAX) {
        return set_boss(value.max(-1) as i32);
    }
    #[cfg(feature = "item_clone_backend")]
    let clone = crate::item_clones::item_kind_for_hash(value);
    #[cfg(not(feature = "item_clone_backend"))]
    let clone: Option<i32> = None;
    match menu::boss_kind(value).or(clone) {
        Some(kind) => set_boss(kind),
        None => Err(Refusal::Hash),
    }
}

pub(crate) fn set_boss(kind: i32) -> Result<(), Refusal> {
    let kind = if kind < 0 { NO_BOSS } else { kind };
    #[cfg(feature = "item_clone_backend")]
    let base = crate::item_clones::boss_clone_base(kind);
    #[cfg(not(feature = "item_clone_backend"))]
    let base: Option<i32> = None;
    match base {
        Some(base) => {
            BOSS.store(base, Ordering::Release);
            BOSS_CLONE.store(kind, Ordering::Release);
            skyline::println!("[menubattle] boss item kind {kind:#x} (on {base:#x}) for the next battle");
        }
        None => {
            BOSS.store(kind, Ordering::Release);
            BOSS_CLONE.store(NO_BOSS, Ordering::Release);
            skyline::println!("[menubattle] boss item kind {kind:#x} for the next battle");
        }
    }
    Ok(())
}

unsafe fn apply_boss() {
    let kind = BOSS.load(Ordering::Acquire);
    if kind == NO_BOSS {
        return;
    }
    let address = crate::game::addr(OFF_BOSS_BATTLE);
    write(address, 1u8);
    write(address + BOSS_KIND, kind);
    skyline::println!("[menubattle] this battle is a boss battle against item kind {kind:#x}");
}

pub(crate) fn boss_object() -> Option<u32> {
    unsafe {
        let manager: usize = core::ptr::read_volatile(crate::game::addr(OFF_BOSS_MANAGER) as *const usize);
        if manager == 0 {
            return None;
        }
        let data: usize = core::ptr::read_volatile((manager + BOSS_MANAGER_DATA) as *const usize);
        if data == 0 {
            return None;
        }
        let first: usize = core::ptr::read_volatile((data + BOSS_ENTRIES) as *const usize);
        let end: usize = core::ptr::read_volatile((data + BOSS_ENTRIES + 8) as *const usize);
        if first == 0 || first >= end {
            return None;
        }
        let entry: usize = core::ptr::read_volatile(first as *const usize);
        if entry == 0 {
            return None;
        }
        let item: usize = core::ptr::read_volatile((entry + BOSS_ENTRY_ITEM) as *const usize);
        if item == 0 {
            return None;
        }
        let object: u32 = core::ptr::read_volatile((item + ITEM_OBJECT_ID) as *const u32);
        (object != NO_OBJECT).then_some(object)
    }
}

pub(crate) fn boss_finished() -> bool {
    unsafe {
        let manager: usize = core::ptr::read_volatile(crate::game::addr(OFF_BOSS_MANAGER) as *const usize);
        if manager == 0 {
            return false;
        }
        let data: usize = core::ptr::read_volatile((manager + BOSS_MANAGER_DATA) as *const usize);
        if data == 0 {
            return false;
        }
        core::ptr::read_volatile((data + BOSS_STATE) as *const i32) == BOSS_STATE_FINISH
    }
}

fn queue_boss_clone() {
    let clone = BOSS_CLONE.load(Ordering::Acquire);
    if clone == NO_BOSS {
        return;
    }
    #[cfg(feature = "item_clone_backend")]
    {
        let queued = crate::item_clones::queue_boss_ticket(clone);
        skyline::println!("[menubattle] the battle's boss is made as item kind {clone:#x}: {queued}");
    }
}

fn drop_boss_clone() {
    let clone = BOSS_CLONE.swap(NO_BOSS, Ordering::AcqRel);
    #[cfg(feature = "item_clone_backend")]
    if clone != NO_BOSS {
        crate::item_clones::drop_pocket_ticket(clone);
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_SETTINGS_RESET))]
unsafe fn settings_reset() {
    call_original!();
    apply_boss();
}

pub(crate) fn set_entrance(play: bool) -> Result<(), Refusal> {
    if !ENTRANCE_ARMED.load(Ordering::Acquire) {
        return Err(Refusal::NoBattle);
    }
    SKIP_ENTRANCE.store(!play, Ordering::Release);
    skyline::println!(
        "[menubattle] next battle {} the entrance and countdown",
        if play { "plays" } else { "skips" }
    );
    Ok(())
}

pub(crate) fn set_ready_go(play: bool) -> Result<(), Refusal> {
    if !READY_GO_ARMED.load(Ordering::Acquire) {
        return Err(Refusal::NoBattle);
    }
    SKIP_READY_GO.store(!play, Ordering::Release);
    skyline::println!(
        "[menubattle] next battle {} \"Ready\" and \"GO!\"",
        if play { "plays" } else { "skips" }
    );
    Ok(())
}

pub(crate) fn active() -> bool {
    BATTLE.load(Ordering::Acquire) != 0
}

pub(crate) fn end_battle() -> Result<(), Refusal> {
    let battle = BATTLE.load(Ordering::Acquire);
    if battle == 0 {
        return Err(Refusal::NoBattle);
    }
    let state = (battle + CONTROLLER_STATE) as *mut u32;
    unsafe {
        if core::ptr::read_volatile(state) != 0 {
            return Err(Refusal::Busy);
        }
        core::ptr::write_volatile(state, CHILD_ENDS);
    }
    skyline::println!(
        "[menubattle] battle ended by request; queue {}",
        crate::menu_sequences::describe_queue()
    );
    Ok(())
}

#[skyline::hook(offset = crate::game::rel(OFF_BATTLE_CTOR))]
unsafe fn battle_ctor(this: usize, a: u64, b: u64, c: u64) -> u64 {
    apply_boss();
    queue_boss_clone();
    let result = call_original!(this, a, b, c);
    BATTLE.store(this, Ordering::Release);
    BATTLE_SERIAL.fetch_add(1, Ordering::AcqRel);
    result
}

#[skyline::hook(offset = crate::game::rel(OFF_BATTLE_DTOR))]
unsafe fn battle_dtor(this: usize) -> u64 {
    let _ = BATTLE.compare_exchange(this, 0, Ordering::AcqRel, Ordering::Acquire);
    BOSS.store(NO_BOSS, Ordering::Release);
    drop_boss_clone();
    call_original!(this)
}

unsafe fn flow_of(holder: usize) -> usize {
    if holder == 0 {
        return 0;
    }
    let inner: usize = core::ptr::read_volatile(holder as *const usize);
    if inner == 0 {
        return 0;
    }
    core::ptr::read_volatile(inner as *const usize)
}

#[skyline::hook(offset = crate::game::rel(OFF_FLOW_BUILD))]
unsafe fn flow_build(states: usize, context: u64) -> u64 {
    let result = call_original!(states, context);
    let flow = flow_of(crate::game::addr(OFF_FLOW_HOLDER));
    if flow != 0 && SKIP_ENTRANCE.swap(false, Ordering::AcqRel) {
        write(flow + FLOW_NO_ENTRANCE, 1u8);
        SKIPPING_FLOW.store(flow, Ordering::Release);
        skyline::println!("[menubattle] entrance and countdown skipped");
    }
    if flow != 0 {
        let quiet = SKIP_READY_GO.swap(false, Ordering::AcqRel);
        QUIET_FLOW.store(if quiet { flow } else { 0 }, Ordering::Release);
        if quiet {
            skyline::println!("[menubattle] \"Ready\" and \"GO!\" skipped");
        }
    }
    result
}

unsafe fn quiet_battle() -> bool {
    let quiet = QUIET_FLOW.load(Ordering::Acquire);
    quiet != 0 && quiet == flow_of(crate::game::addr(OFF_FLOW_HOLDER))
}

unsafe fn read_u32(address: usize) -> u32 {
    core::ptr::read_volatile(address as *const u32)
}

#[skyline::hook(offset = crate::game::rel(OFF_INFO_START))]
unsafe fn info_start(info: usize) -> u64 {
    if info != 0 && quiet_battle() && INFO_READY_GO.contains(&read_u32(info + INFO_REQUEST)) {
        let next = read_u32(info + INFO_NEXT);
        let request = if INFO_READY_GO.contains(&next) { 0 } else { next };
        write(info + INFO_REQUEST, request);
        write(info + INFO_NEXT, 0u32);
        if request == 0 {
            let index = core::ptr::read_volatile((info + INFO_STATE_INDEX) as *const i32);
            if (0..INFO_STATE_COUNT).contains(&index) {
                write(info + INFO_STATES + index as usize * 4, INFO_DONE);
            }
            return 0;
        }
    }
    call_original!(info)
}

unsafe fn mute_sound_argument(ctx: &mut skyline::hooks::InlineCtx) {
    if NARRATION_READY_GO.contains(&ctx.registers[SOUND_ARGUMENT].x()) && quiet_battle() {
        ctx.registers[SOUND_ARGUMENT].set_x(SILENT_SOUND);
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_QUEUED_NARRATION), inline)]
unsafe fn queued_narration(ctx: &mut skyline::hooks::InlineCtx) {
    let node = ctx.registers[QUEUE_NODE].x() as usize;
    if node == 0 || !quiet_battle() {
        return;
    }
    let sound = (node + QUEUE_NODE_SOUND) as *mut u64;
    if NARRATION_READY_GO.contains(&core::ptr::read_volatile(sound)) {
        core::ptr::write_volatile(sound, SILENT_SOUND);
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_GO_FROM_UPDATE), inline)]
unsafe fn go_from_update(ctx: &mut skyline::hooks::InlineCtx) {
    mute_sound_argument(ctx);
}

#[skyline::hook(offset = crate::game::rel(OFF_GO_FROM_START), inline)]
unsafe fn go_from_start(ctx: &mut skyline::hooks::InlineCtx) {
    mute_sound_argument(ctx);
}

#[skyline::hook(offset = crate::game::rel(OFF_APPEARANCE_START))]
unsafe fn appearance_start(flow: usize) -> u64 {
    if flow != 0 && SKIPPING_FLOW.compare_exchange(flow, 0, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        write(flow + FLOW_NO_ENTRANCE, 1u8);
    }
    call_original!(flow)
}

pub(crate) fn install() {
    let ctor = unsafe { crate::game::text_word(OFF_BATTLE_CTOR) };
    let dtor = unsafe { crate::game::text_word(OFF_BATTLE_DTOR) };
    if ctor != BATTLE_CTOR_OPCODE || dtor != BATTLE_DTOR_OPCODE {
        skyline::println!(
            "[menubattle] REFUSED battle scene hooks: found {ctor:#010x} / {dtor:#010x}; end_battle stays off"
        );
        return;
    }
    skyline::install_hooks!(battle_ctor, battle_dtor);
    if unsafe { crate::game::text_word(OFF_SETTINGS_RESET) } == SETTINGS_RESET_OPCODE {
        skyline::install_hooks!(settings_reset);
    }
    let build = unsafe { crate::game::text_word(OFF_FLOW_BUILD) };
    let appearance = unsafe { crate::game::text_word(OFF_APPEARANCE_START) };
    if build != FLOW_BUILD_OPCODE || appearance != APPEARANCE_START_OPCODE {
        skyline::println!(
            "[menubattle] REFUSED entrance hooks: found {build:#010x} / {appearance:#010x}; battle_entrance stays off"
        );
        return;
    }
    skyline::install_hooks!(flow_build, appearance_start);
    ENTRANCE_ARMED.store(true, Ordering::Release);
    let sites = [
        (OFF_INFO_START, INFO_START_OPCODE),
        (OFF_QUEUED_NARRATION, QUEUED_NARRATION_OPCODE),
        (OFF_GO_FROM_UPDATE, GO_FROM_UPDATE_OPCODE),
        (OFF_GO_FROM_START, GO_FROM_START_OPCODE),
    ];
    for (offset, expected) in sites {
        let found = unsafe { crate::game::text_word(offset) };
        if found != expected {
            skyline::println!(
                "[menubattle] REFUSED Ready/GO hooks: {offset:#x} holds {found:#010x}, not {expected:#010x}; battle_ready_go stays off"
            );
            return;
        }
    }
    skyline::install_hooks!(info_start, queued_narration, go_from_update, go_from_start);
    READY_GO_ARMED.store(true, Ordering::Release);
}

#[no_mangle]
pub extern "C" fn clone_engine_end_battle() -> i32 {
    match end_battle() {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_player(slot: u32, kind: u32, chara: u64, pad: i32) -> i32 {
    match set_player(slot, kind, chara, pad) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_damage(slot: u32, percent: u32) -> i32 {
    match set_damage(slot, percent) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_color(slot: u32, color: u32) -> i32 {
    match set_color(slot, color) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_rules(kind: u32, stock: u32, seconds: u32, items: u32) -> i32 {
    match set_rules(kind, stock, seconds, items) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_stock(slot: u32) -> i32 {
    match stock_left(slot) {
        Ok(count) => count.min(i32::MAX as u32) as i32,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_boss(kind: i32) -> i32 {
    match set_boss(kind) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_boss_object() -> u32 {
    boss_object().unwrap_or(NO_OBJECT)
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_boss_finished() -> u32 {
    u32::from(boss_finished())
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_entrance(play: u32) -> i32 {
    match set_entrance(play != 0) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_serial() -> u32 {
    BATTLE_SERIAL.load(Ordering::Acquire) as u32
}

pub(crate) fn count_frame() {
    BATTLE_FRAMES.fetch_add(1, Ordering::AcqRel);
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_frame_v1() -> u64 {
    BATTLE_FRAMES.load(Ordering::Acquire) as u64
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_pause(allowed: u32) -> i32 {
    match set_pause(allowed != 0) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_ready_go(play: u32) -> i32 {
    match set_ready_go(play != 0) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_battle_stage(stage: u64) -> i32 {
    match set_stage(stage) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}
