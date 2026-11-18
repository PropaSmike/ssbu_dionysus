use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use smash::phx::Hash40;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock, RwLock};

use dionysus_api::{
    CloneItemFamilyMemberV2, CloneItemFamilyRegistrationV2, CloneItemRegistrationV1, ItemCategory,
    ItemSpawnSource, API_VERSION_V1, API_VERSION_V2, ERROR_BACKEND_UNAVAILABLE, ERROR_BASE_KIND,
    ERROR_CUSTOM_KIND, ERROR_DUPLICATE, ERROR_ITEM_AGENT_UNAVAILABLE, ERROR_ITEM_CATEGORY,
    ERROR_ITEM_FAMILY_CAPACITY, ERROR_ITEM_FAMILY_EMPTY, ERROR_ITEM_FAMILY_LAYOUT,
    ERROR_ITEM_RESOURCE_UNAVAILABLE, ERROR_NAME, ERROR_NULL, ERROR_REGISTRATION_CLOSED,
    ERROR_STRUCT_SIZE, ERROR_UNSUPPORTED, ERROR_VERSION, RESULT_OK,
};

pub(crate) const FIRST_SPARSE_ITEM_KIND: i32 = 0x36A;
const LAST_ORDINARY_ITEM_KIND: i32 = 0x1AF;
const NATIVE_ITEM_COUNT: usize = 432;
const KIND_AUTO: i32 = -1;
pub(crate) const FIRST_COMPACT_RESOURCE_KIND: i32 = NATIVE_ITEM_COUNT as i32;
pub(crate) const MAX_COMPACT_RESOURCE_COUNT: usize = 0x0FFF;
const UNSUPPORTED_AGENT_BASE_KIND: i32 = 398;
const ITEM_DESCRIPTOR_TABLE: usize = 0x5070FC8;
const ITEM_DESCRIPTOR_STRIDE: usize = 0x20;

const OFF_ITEM_REQUEST_FULL: usize = 0x15B5A40;
const OFF_ITEM_REQUEST_SIMPLE: usize = 0x15B5D00;
const OFF_ITEM_LOWER_CREATOR: usize = 0x15DB0B0;
const OFF_ITEM_HAVE_ITEM: usize = 0x2098BF0;
const OFF_ITEM_BORN_ITEM: usize = 0x2098DF0;
const OFF_ITEM_ATTACH_ITEM: usize = 0x2098EE0;
const OFF_ITEM_GET_HAVE_ITEM_KIND: usize = 0x2098CD0;
const OFF_ITEM_GET_HAVE_ITEM_ID: usize = 0x2098D30;
const OFF_ITEM_GET_PICKABLE_ITEM_KIND: usize = 0x2098D60;
const OFF_ITEM_GET_PICKABLE_ITEM_OBJECT_ID: usize = 0x2098D40;
const OFF_ITEM_DEACTIVATE: usize = 0x15D4570;
const OFF_ITEM_OBJECT_ADOPT: usize = 0x15DBB28;
const OFF_BATTLE_OBJECT_UPDATE: usize = 0x3A84E0;
pub(crate) const BATTLE_OBJECT_MODULE_TABLE: usize = 0x20;

const ITEM_NRO_DISPATCHER: usize = 0x480;
const ITEM_NRO_DISPATCHER_WORDS: [u32; 5] =
    [0xFC190FE8, 0xA9016FFC, 0xA90267FA, 0xA9035FF8, 0xA90457F6];

const MAIN_PREFLIGHT: &[(usize, &[u32])] = &[
    (
        OFF_ITEM_REQUEST_FULL,
        &[0xD103C3FF, 0xA90B5FF8, 0xA90C57F6, 0xA90D4FF4],
    ),
    (
        OFF_ITEM_REQUEST_SIMPLE,
        &[0xD10283FF, 0xFD002BE8, 0xA9065FF8, 0xA90757F6],
    ),
    (
        OFF_ITEM_LOWER_CREATOR,
        &[0xD103C3FF, 0xA9096FFC, 0xA90A67FA, 0xA90B5FF8, 0xA90C57F6],
    ),
    (
        OFF_ITEM_DEACTIVATE,
        &[0xD101C3FF, 0xF9000BFB, 0xA90267FA, 0xA9035FF8, 0xA90457F6],
    ),
    (
        OFF_ITEM_OBJECT_ADOPT,
        &[0xF94002C8, 0xF9400908, 0xD63F0100, 0xF94002C8],
    ),
    (
        OFF_BATTLE_OBJECT_UPDATE,
        &[0xA9BD57F6, 0xA9014FF4, 0xA9027BFD, 0x910083FD],
    ),
];

pub const STATUS_COMPILED: u32 = 1 << 0;
pub const STATUS_MAIN_PREFLIGHT_OK: u32 = 1 << 1;
pub const STATUS_IDENTITY_HOOKS_READY: u32 = 1 << 2;
pub const STATUS_ITEM_NRO_READY: u32 = 1 << 3;
pub const STATUS_STATUS_ROUTER_READY: u32 = 1 << 4;
pub const STATUS_REGISTRATION_CLOSED: u32 = 1 << 5;
pub const STATUS_RESOURCE_ROUTER_READY: u32 = 1 << 6;
pub const STATUS_PARAM_ROUTER_READY: u32 = 1 << 7;
pub const STATUS_CATEGORY_ROUTER_READY: u32 = 1 << 8;
pub const STATUS_FAMILY_ROUTER_READY: u32 = 1 << 9;
pub const STATUS_MULTI_BASE_STATUS_ROUTER_READY: u32 = 1 << 10;
pub const STATUS_TRAINING_UI_READY: u32 = 1 << 15;
pub const STATUS_READY: u32 = 1 << 31;

#[derive(Debug)]
struct ItemCloneDefinition {
    public_kind: i32,
    base_kind: i32,
    compact_kind: i32,
    resource_name: CString,
    agent_name: CString,
    custom_agent_hash: u64,
    base_agent_hash: u64,
    category: ItemCategory,
    family_owner_kind: i32,
    family_member_index: u32,
    family_member_count: u32,
}

#[repr(C, align(16))]
struct ItemCreateDescriptor([u8; 0x50]);

#[derive(Clone, Copy)]
struct ItemDefinitionView {
    public_kind: i32,
    base_kind: i32,
    custom_agent_hash: u64,
    base_agent_hash: u64,
    category: ItemCategory,
    family_owner_kind: i32,
    family_member_index: u32,
}

fn definitions() -> &'static RwLock<Vec<ItemCloneDefinition>> {
    static DEFINITIONS: OnceLock<RwLock<Vec<ItemCloneDefinition>>> = OnceLock::new();
    DEFINITIONS.get_or_init(|| RwLock::new(Vec::new()))
}

fn definition(kind: i32) -> Option<ItemDefinitionView> {
    definitions().read().ok()?.iter().find_map(|definition| {
        (definition.public_kind == kind).then_some(ItemDefinitionView {
            public_kind: definition.public_kind,
            base_kind: definition.base_kind,
            custom_agent_hash: definition.custom_agent_hash,
            base_agent_hash: definition.base_agent_hash,
            category: definition.category,
            family_owner_kind: definition.family_owner_kind,
            family_member_index: definition.family_member_index,
        })
    })
}

pub(crate) fn item_ui_base(public_kind: i32) -> Option<(i32, ItemCategory, u64)> {
    let view = definition(public_kind)?;
    let base_name = unsafe { base_resource_name(view.base_kind)? }
        .to_str()
        .ok()?;
    Some((
        view.base_kind,
        view.category,
        hash40(&format!("ui_item_{base_name}")),
    ))
}

fn hash40(name: &str) -> u64 {
    crate::hash40::hash40(name)
}

pub(crate) unsafe fn base_resource_name(kind: i32) -> Option<&'static CStr> {
    if !(0..=LAST_ORDINARY_ITEM_KIND).contains(&kind) {
        return None;
    }
    let row = crate::game::addr(ITEM_DESCRIPTOR_TABLE) + kind as usize * ITEM_DESCRIPTOR_STRIDE;
    if (row as *const i32).read_unaligned() != kind {
        return None;
    }
    let name = ((row + 0x18) as *const *const c_char).read_unaligned();
    if name.is_null() {
        return None;
    }
    Some(CStr::from_ptr(name))
}

#[derive(Clone, Copy, Debug)]
struct ItemDescriptorMeta {
    class: u32,
    flags: u32,
    variants: u32,
    family: u32,
}

unsafe fn descriptor_meta(kind: i32) -> Option<ItemDescriptorMeta> {
    if !(0..NATIVE_ITEM_COUNT as i32).contains(&kind) {
        return None;
    }
    let row = crate::game::addr(ITEM_DESCRIPTOR_TABLE) + kind as usize * ITEM_DESCRIPTOR_STRIDE;
    if core::ptr::read_unaligned(row as *const i32) != kind {
        return None;
    }
    Some(ItemDescriptorMeta {
        class: core::ptr::read_unaligned((row + 0x04) as *const u32),
        flags: core::ptr::read_unaligned((row + 0x08) as *const u32),
        variants: core::ptr::read_unaligned((row + 0x0c) as *const u32),
        family: core::ptr::read_unaligned((row + 0x10) as *const u32),
    })
}

fn category_from_descriptor(meta: ItemDescriptorMeta) -> ItemCategory {
    if meta.flags & 0x80 != 0 {
        ItemCategory::Boss
    } else if meta.flags & ((1 << 3) | (1 << 5)) == ((1 << 3) | (1 << 5)) {
        ItemCategory::Assist
    } else if meta.flags & ((1 << 4) | (1 << 6)) == ((1 << 4) | (1 << 6)) {
        ItemCategory::Pokemon
    } else {
        ItemCategory::Item
    }
}


const BASE_OWNER_SLOTS: usize = 64;
static BASE_OWNER_ITEM: [AtomicI32; BASE_OWNER_SLOTS] =
    [const { AtomicI32::new(-1) }; BASE_OWNER_SLOTS];
static BASE_OWNER_FIGHTER: [AtomicI32; BASE_OWNER_SLOTS] =
    [const { AtomicI32::new(-1) }; BASE_OWNER_SLOTS];

pub(crate) fn remember_base_item_owner(base_item_kind: i32, fighter_kind: i32) {
    for index in 0..BASE_OWNER_SLOTS {
        let held = BASE_OWNER_ITEM[index].load(Ordering::Relaxed);
        if held == base_item_kind {
            return;
        }
        if held == -1
            && BASE_OWNER_ITEM[index]
                .compare_exchange(-1, base_item_kind, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        {
            BASE_OWNER_FIGHTER[index].store(fighter_kind, Ordering::Release);
            return;
        }
    }
}

fn declared_base_item_owner(base_item_kind: i32) -> Option<i32> {
    (0..BASE_OWNER_SLOTS).find_map(|index| {
        (BASE_OWNER_ITEM[index].load(Ordering::Acquire) == base_item_kind)
            .then(|| BASE_OWNER_FIGHTER[index].load(Ordering::Acquire))
            .filter(|kind| *kind >= 0)
    })
}

pub(crate) fn base_item_owner(base_item_kind: i32) -> Option<i32> {
    if let Some(owner) = declared_base_item_owner(base_item_kind) {
        return Some(owner);
    }
    let name = unsafe { base_resource_name(base_item_kind) }?;
    let owner = crate::custom_articles::fighter_kind_from_name_prefix(name.to_str().ok()?)?;
    remember_base_item_owner(base_item_kind, owner);
    Some(owner)
}

fn owner_class_matches_category(meta: ItemDescriptorMeta, category: ItemCategory) -> bool {
    match category {
        ItemCategory::Assist => matches!(meta.class, 14 | 15),
        ItemCategory::Pokemon => matches!(meta.class, 16 | 17),
        ItemCategory::Boss => meta.class == 18,
        ItemCategory::Item => meta.class != 13,
        ItemCategory::Unknown => false,
    }
}

pub(crate) fn item_category_for_base(base_kind: i32) -> ItemCategory {
    unsafe { descriptor_meta(base_kind) }
        .map(category_from_descriptor)
        .unwrap_or(ItemCategory::Unknown)
}

pub(crate) fn item_category_for_public(public_kind: i32) -> ItemCategory {
    definition(public_kind)
        .map(|definition| definition.category)
        .unwrap_or(ItemCategory::Unknown)
}

pub(crate) unsafe fn native_family_len(base_owner_kind: i32) -> Option<usize> {
    let owner = descriptor_meta(base_owner_kind)?;
    if owner.class == 13 {
        return None;
    }
    let category = category_from_descriptor(owner);
    let mut count = 1usize;
    while base_owner_kind as usize + count < NATIVE_ITEM_COUNT {
        let child = descriptor_meta(base_owner_kind + count as i32)?;
        if child.class != 13 || category_from_descriptor(child) != category {
            break;
        }
        count += 1;
    }
    Some(count)
}

fn valid_registration_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

static REGISTRATION_CLOSED: AtomicBool = AtomicBool::new(false);
static BACKEND_STATUS: AtomicU32 = AtomicU32::new(STATUS_COMPILED);
static LOG_SEQUENCE: AtomicU32 = AtomicU32::new(0);

fn log(message: String) {
    crate::dbg_log_public(&message);
}

fn limited_log(message: String) {
    if LOG_SEQUENCE.fetch_add(1, Ordering::Relaxed) < 1000 {
        log(message);
    }
}

#[derive(Clone, PartialEq, Eq)]
enum StatusSelector {
    Kind(i32),
    Name(String),
}

#[derive(Clone)]
struct ItemStatusScript {
    public_kind: i32,
    line: i32,
    status: StatusSelector,
    function: usize,
}

impl ItemStatusScript {
    fn status_kind(&self) -> Option<i32> {
        match &self.status {
            StatusSelector::Kind(kind) => Some(*kind),
            StatusSelector::Name(name) => item_status_kind_by_name(name),
        }
    }

    fn describe(&self) -> String {
        match &self.status {
            StatusSelector::Kind(kind) => format!("{kind:#x}"),
            StatusSelector::Name(name) => format!("\"{name}\""),
        }
    }
}

fn status_scripts() -> &'static RwLock<Vec<ItemStatusScript>> {
    static SCRIPTS: OnceLock<RwLock<Vec<ItemStatusScript>>> = OnceLock::new();
    SCRIPTS.get_or_init(|| RwLock::new(Vec::new()))
}

fn register_status_script(
    public_kind: i32,
    line: i32,
    status: StatusSelector,
    function: usize,
) -> i32 {
    let Ok(mut scripts) = status_scripts().write() else {
        return ERROR_BACKEND_UNAVAILABLE;
    };
    let entry = ItemStatusScript {
        public_kind,
        line,
        status,
        function,
    };
    let described = entry.describe();
    match scripts.iter_mut().find(|script| {
        script.public_kind == public_kind && script.status == entry.status && script.line == line
    }) {
        Some(existing) => *existing = entry,
        None => scripts.push(entry),
    }
    log(format!(
        "[itemclone] item_status registered public={public_kind:#x} status={described} \
         line={line} fn={function:#x}"
    ));
    RESULT_OK
}

fn scripts_for(kind: i32) -> Vec<(usize, ItemStatusScript)> {
    status_scripts()
        .read()
        .map(|scripts| {
            scripts
                .iter()
                .enumerate()
                .filter(|(_, script)| script.public_kind == kind)
                .map(|(index, script)| (index, script.clone()))
                .collect()
        })
        .unwrap_or_default()
}

const MAX_CHAINED_STATUS_SCRIPTS: usize = 64;

#[allow(clippy::declare_interior_mutable_const)]
const ZERO_SLOT: AtomicUsize = AtomicUsize::new(0);

static CHAIN_ORIGINAL: [AtomicUsize; MAX_CHAINED_STATUS_SCRIPTS] =
    [ZERO_SLOT; MAX_CHAINED_STATUS_SCRIPTS];
static CHAIN_TARGET: [AtomicUsize; MAX_CHAINED_STATUS_SCRIPTS] =
    [ZERO_SLOT; MAX_CHAINED_STATUS_SCRIPTS];

type StatusFn = unsafe extern "C" fn(*mut u8) -> smash::lib::L2CValue;

unsafe fn run_chained_status(index: usize, agent: *mut u8) -> smash::lib::L2CValue {
    let original = CHAIN_ORIGINAL[index].load(Ordering::Acquire);
    if original != 0 {
        let call: StatusFn = core::mem::transmute(original);
        call(agent);
    }
    let target = CHAIN_TARGET[index].load(Ordering::Acquire);
    if target == 0 {
        return smash::lib::L2CValue::new_int(0);
    }
    let call: StatusFn = core::mem::transmute(target);
    call(agent)
}

struct ChainShim<const INDEX: usize>;

impl<const INDEX: usize> ChainShim<INDEX> {
    unsafe extern "C" fn call(agent: *mut u8) -> smash::lib::L2CValue {
        run_chained_status(INDEX, agent)
    }
}

macro_rules! chain_shims {
    ($($index:literal),* $(,)?) => {
        static CHAIN_SHIMS: [StatusFn; MAX_CHAINED_STATUS_SCRIPTS] =
            [$(ChainShim::<$index>::call),*];
    };
}

chain_shims!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
);

const L2C_TYPE_VOID: u32 = 0;
const L2C_TYPE_POINTER: u32 = 4;

unsafe fn status_function_pointer(value: &smash::lib::L2CValue) -> Result<usize, u32> {
    let raw = value as *const smash::lib::L2CValue as *const u8;
    let val_type = core::ptr::read_volatile(raw as *const u32);
    match val_type {
        L2C_TYPE_POINTER => Ok(core::ptr::read_volatile(raw.add(8) as *const usize)),
        L2C_TYPE_VOID => Ok(0),
        other => Err(other),
    }
}

struct PendingSlot {
    owner_thread: AtomicUsize,
    stage: AtomicU32,
    public_kind: AtomicI32,
    base_kind: AtomicI32,
    parent_public_kind: AtomicI32,
    spawn_source: AtomicU32,
    boma: AtomicUsize,
    agent: AtomicUsize,
    module: AtomicUsize,
}

impl PendingSlot {
    const fn new() -> Self {
        Self {
            owner_thread: AtomicUsize::new(0),
            stage: AtomicU32::new(0),
            public_kind: AtomicI32::new(-1),
            base_kind: AtomicI32::new(-1),
            parent_public_kind: AtomicI32::new(-1),
            spawn_source: AtomicU32::new(ItemSpawnSource::Unknown as u32),
            boma: AtomicUsize::new(0),
            agent: AtomicUsize::new(0),
            module: AtomicUsize::new(0),
        }
    }
}

static PENDING: [PendingSlot; 16] = [
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
    PendingSlot::new(),
];

struct SpawnTicket {
    state: AtomicU32,
    sequence: AtomicU32,
    public_kind: AtomicI32,
    base_kind: AtomicI32,
    spawn_source: AtomicU32,
}

impl SpawnTicket {
    const fn new() -> Self {
        Self {
            state: AtomicU32::new(0),
            sequence: AtomicU32::new(0),
            public_kind: AtomicI32::new(-1),
            base_kind: AtomicI32::new(-1),
            spawn_source: AtomicU32::new(ItemSpawnSource::Unknown as u32),
        }
    }
}

static SPAWN_TICKET_SEQUENCE: AtomicU32 = AtomicU32::new(1);
static SPAWN_TICKETS: [SpawnTicket; 16] = [const { SpawnTicket::new() }; 16];

fn queue_spawn_ticket(public_kind: i32, spawn_source: ItemSpawnSource) -> bool {
    let Some(view) = definition(public_kind) else {
        return false;
    };
    for ticket in &SPAWN_TICKETS {
        if ticket
            .state
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        ticket.public_kind.store(public_kind, Ordering::Relaxed);
        ticket.base_kind.store(view.base_kind, Ordering::Relaxed);
        ticket
            .spawn_source
            .store(spawn_source as u32, Ordering::Relaxed);
        ticket.sequence.store(
            SPAWN_TICKET_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            Ordering::Relaxed,
        );
        ticket.state.store(1, Ordering::Release);
        return true;
    }
    false
}

pub(crate) fn queue_pocket_ticket(public_kind: i32) -> bool {
    queue_spawn_ticket(public_kind, ItemSpawnSource::Direct)
}

pub(crate) fn queue_lot_ticket(public_kind: i32) -> bool {
    queue_spawn_ticket(public_kind, ItemSpawnSource::Lot)
}

pub(crate) fn boss_clone_base(public_kind: i32) -> Option<i32> {
    definition(public_kind)
        .filter(|view| matches!(view.category, ItemCategory::Boss))
        .map(|view| view.base_kind)
}

pub(crate) fn queue_boss_ticket(public_kind: i32) -> bool {
    queue_spawn_ticket(public_kind, ItemSpawnSource::Direct)
}

pub(crate) fn pocket_ticket_pending(public_kind: i32) -> bool {
    SPAWN_TICKETS.iter().any(|ticket| {
        ticket.state.load(Ordering::Acquire) == 1
            && ticket.public_kind.load(Ordering::Relaxed) == public_kind
    })
}

pub(crate) fn drop_pocket_ticket(public_kind: i32) {
    for ticket in &SPAWN_TICKETS {
        if ticket.public_kind.load(Ordering::Relaxed) != public_kind {
            continue;
        }
        if ticket
            .state
            .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        ticket.public_kind.store(-1, Ordering::Relaxed);
        ticket.base_kind.store(-1, Ordering::Relaxed);
        ticket
            .spawn_source
            .store(ItemSpawnSource::Unknown as u32, Ordering::Relaxed);
        ticket.state.store(0, Ordering::Release);
    }
}

pub(crate) fn clear_spawn_tickets() {
    for ticket in &SPAWN_TICKETS {
        if ticket
            .state
            .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        ticket.public_kind.store(-1, Ordering::Relaxed);
        ticket.base_kind.store(-1, Ordering::Relaxed);
        ticket
            .spawn_source
            .store(ItemSpawnSource::Unknown as u32, Ordering::Relaxed);
        ticket.state.store(0, Ordering::Release);
    }
}

pub(crate) fn queue_training_spawn_tickets(
    public_kind: i32,
    spawn_source: ItemSpawnSource,
) -> bool {
    clear_spawn_tickets();
    queue_spawn_ticket(public_kind, spawn_source)
}

fn take_spawn_ticket(base_kind: i32) -> Option<(ItemDefinitionView, ItemSpawnSource)> {
    let mut best: Option<(usize, u32)> = None;
    for (index, ticket) in SPAWN_TICKETS.iter().enumerate() {
        if ticket.state.load(Ordering::Acquire) != 1
            || ticket.base_kind.load(Ordering::Relaxed) != base_kind
        {
            continue;
        }
        let sequence = ticket.sequence.load(Ordering::Relaxed);
        if best.map_or(true, |(_, current)| {
            sequence.wrapping_sub(current) as i32 <= 0
        }) {
            best = Some((index, sequence));
        }
    }
    let (index, _) = best?;
    let ticket = &SPAWN_TICKETS[index];
    ticket
        .state
        .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Relaxed)
        .ok()?;
    let public_kind = ticket.public_kind.load(Ordering::Relaxed);
    let source = match ticket.spawn_source.load(Ordering::Relaxed) {
        1 => ItemSpawnSource::Direct,
        2 => ItemSpawnSource::Assist,
        3 => ItemSpawnSource::PokeBall,
        4 => ItemSpawnSource::MasterBall,
        5 => ItemSpawnSource::Boss,
        6 => ItemSpawnSource::FamilyChild,
        7 => ItemSpawnSource::Lot,
        _ => ItemSpawnSource::Unknown,
    };
    let view = definition(public_kind);
    ticket.public_kind.store(-1, Ordering::Relaxed);
    ticket.base_kind.store(-1, Ordering::Relaxed);
    ticket
        .spawn_source
        .store(ItemSpawnSource::Unknown as u32, Ordering::Relaxed);
    ticket.state.store(0, Ordering::Release);
    view.map(|definition| (definition, source))
}

struct LiveSlot {
    state: AtomicU32,
    object: AtomicUsize,
    boma: AtomicUsize,
    agent: AtomicUsize,
    module: AtomicUsize,
    object_id: AtomicU32,
    public_kind: AtomicI32,
    base_kind: AtomicI32,
    parent_public_kind: AtomicI32,
    spawn_source: AtomicU32,
}

impl LiveSlot {
    const fn new() -> Self {
        Self {
            state: AtomicU32::new(0),
            object: AtomicUsize::new(0),
            boma: AtomicUsize::new(0),
            agent: AtomicUsize::new(0),
            module: AtomicUsize::new(0),
            object_id: AtomicU32::new(u32::MAX),
            public_kind: AtomicI32::new(-1),
            base_kind: AtomicI32::new(-1),
            parent_public_kind: AtomicI32::new(-1),
            spawn_source: AtomicU32::new(ItemSpawnSource::Unknown as u32),
        }
    }
}

static LIVE: [LiveSlot; 64] = [const { LiveSlot::new() }; 64];

static LIVE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) unsafe fn live_identity_of_object(object: usize) -> Option<(i32, i32)> {
    if object == 0 {
        return None;
    }
    let module_table =
        core::ptr::read_volatile((object + BATTLE_OBJECT_MODULE_TABLE) as *const usize);
    LIVE.iter().find_map(|slot| {
        if slot.state.load(Ordering::Acquire) != 2 {
            return None;
        }
        let boma = slot.boma.load(Ordering::Acquire);
        let matched = slot.object.load(Ordering::Acquire) == object
            || boma == object
            || (module_table != 0 && boma == module_table);
        matched.then(|| {
            (
                slot.public_kind.load(Ordering::Relaxed),
                slot.base_kind.load(Ordering::Relaxed),
            )
        })
    })
}

unsafe fn live_identity_of_boma(boma: usize) -> Option<(i32, i32)> {
    if boma == 0 || LIVE_COUNT.load(Ordering::Acquire) == 0 {
        return None;
    }
    let object_id = core::ptr::read_volatile((boma + BOMA_OBJECT_ID) as *const u32);
    LIVE.iter().find_map(|slot| {
        (slot.state.load(Ordering::Acquire) == 2
            && (slot.boma.load(Ordering::Acquire) == boma
                || slot.object_id.load(Ordering::Acquire) == object_id))
            .then(|| {
                (
                    slot.public_kind.load(Ordering::Relaxed),
                    slot.base_kind.load(Ordering::Relaxed),
                )
            })
    })
}

const ITEM_LUA_VTABLE: usize = 0x5077008;
const LUA_MODULE_ACCESSOR: usize = 8;
const BOMA_OBJECT_ID: usize = 8;
const LUA_EXEC: usize = 0;
const LUA_END: usize = 1;
const LUA_START: usize = 2;
const LUA_SLOTS: [(usize, usize); 3] = [(0xe0, 0x161b780), (0xe8, 0x161bc70), (0xf0, 0x161bd60)];

static LUA_PREVIOUS: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];

unsafe fn status_scope(module: usize) -> Option<usize> {
    if module == 0 || LIVE_COUNT.load(Ordering::Acquire) == 0 {
        return None;
    }
    let boma = core::ptr::read_volatile((module + LUA_MODULE_ACCESSOR) as *const usize);
    let (public, base) = live_identity_of_boma(boma)?;
    crate::item_params::enter_runtime_clone(public, base)
}

unsafe fn close_status_scope(scope: Option<usize>) {
    if let Some(index) = scope {
        crate::item_params::leave_runtime_clone(index);
    }
}

unsafe extern "C" fn lua_status_exec(module: usize, rate: f32) -> u64 {
    let scope = status_scope(module);
    let previous: unsafe extern "C" fn(usize, f32) -> u64 =
        core::mem::transmute(LUA_PREVIOUS[LUA_EXEC].load(Ordering::Acquire));
    let result = previous(module, rate);
    close_status_scope(scope);
    result
}

unsafe extern "C" fn lua_status_end(module: usize) -> u64 {
    let scope = status_scope(module);
    let previous: unsafe extern "C" fn(usize) -> u64 =
        core::mem::transmute(LUA_PREVIOUS[LUA_END].load(Ordering::Acquire));
    let result = previous(module);
    close_status_scope(scope);
    result
}

unsafe extern "C" fn lua_status_start(module: usize) -> u64 {
    let scope = status_scope(module);
    let previous: unsafe extern "C" fn(usize) -> u64 =
        core::mem::transmute(LUA_PREVIOUS[LUA_START].load(Ordering::Acquire));
    let result = previous(module);
    close_status_scope(scope);
    result
}

unsafe fn install_status_scope() -> bool {
    let table = crate::game::addr(ITEM_LUA_VTABLE);
    let intact = LUA_SLOTS.iter().all(|(slot, original)| {
        core::ptr::read_volatile((table + slot) as *const usize) == crate::game::addr(*original)
    });
    if !intact {
        return false;
    }
    let wrappers = [
        lua_status_exec as *const () as usize,
        lua_status_end as *const () as usize,
        lua_status_start as *const () as usize,
    ];
    for (index, (_, original)) in LUA_SLOTS.iter().enumerate() {
        LUA_PREVIOUS[index].store(crate::game::addr(*original), Ordering::Release);
    }
    LUA_SLOTS.iter().zip(wrappers).all(|((slot, _), wrapper)| {
        crate::text_patch::write_bytes(table + slot, &wrapper.to_le_bytes())
    })
}

#[skyline::hook(offset = crate::game::rel(OFF_BATTLE_OBJECT_UPDATE))]
unsafe fn battle_object_update(object: *mut u8) {
    crate::block_grid::sample();
    let row = crate::fighter_param_rows::enter_for_object(object as usize);
    if LIVE_COUNT.load(Ordering::Acquire) == 0 {
        call_original!(object);
        crate::fighter_param_rows::leave(row);
        return;
    }
    crate::item_params::try_fill();
    #[cfg(feature = "item_selftest")]
    crate::item_params::report_common_params();
    #[cfg(feature = "item_selftest")]
    report_status_constants();
    let identity = live_identity_of_object(object as usize);
    if let Some((_, base)) = identity {
        crate::block_grid::note_clone_object(base);
    }
    let scope = identity.and_then(|(public, base)| {
        let scope = crate::item_params::enter_runtime_clone(public, base);
        if scope.is_some() {
            static ANNOUNCED: AtomicBool = AtomicBool::new(false);
            if !ANNOUNCED.swap(true, Ordering::AcqRel) {
                limited_log(format!(
                    "[itemclone] per-frame bracket live: object={object:p} is clone {public:#x} (base {base:#x})"
                ));
            }
        }
        scope
    });
    call_original!(object);
    crate::fighter_param_rows::leave(row);
    if let Some(index) = scope {
        crate::item_params::leave_runtime_clone(index);
    }
    if let Some((_, base)) = identity {
        crate::block_grid::note_live_block(object as usize, base);
    }
    #[cfg(feature = "diag_item_work")]
    match live_identity_of_object(object as usize) {
        Some((public, base)) => crate::item_work_probe::observe(object as usize, public, base),
        None => crate::item_work_probe::observe_vanilla(object as usize),
    }
}

unsafe fn current_thread() -> usize {
    skyline::nn::os::GetCurrentThread() as usize
}

unsafe fn arm_request_with_context(
    definition: ItemDefinitionView,
    parent_public_kind: i32,
    spawn_source: ItemSpawnSource,
) -> bool {
    let thread = current_thread();
    if thread == 0 {
        return false;
    }
    for slot in &PENDING {
        if slot
            .owner_thread
            .compare_exchange(0, thread, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        slot.public_kind
            .store(definition.public_kind, Ordering::Relaxed);
        slot.base_kind
            .store(definition.base_kind, Ordering::Relaxed);
        slot.parent_public_kind
            .store(parent_public_kind, Ordering::Relaxed);
        slot.spawn_source
            .store(spawn_source as u32, Ordering::Relaxed);
        slot.boma.store(0, Ordering::Relaxed);
        slot.agent.store(0, Ordering::Relaxed);
        slot.module.store(0, Ordering::Relaxed);
        slot.stage.store(1, Ordering::Release);
        return true;
    }
    false
}

unsafe fn arm_request(definition: ItemDefinitionView) -> bool {
    arm_request_with_context(definition, -1, ItemSpawnSource::Direct)
}

unsafe fn take_request(base_kind: i32) -> Option<usize> {
    let thread = current_thread();
    PENDING.iter().enumerate().rev().find_map(|(index, slot)| {
        if slot.owner_thread.load(Ordering::Acquire) != thread
            || slot.base_kind.load(Ordering::Acquire) != base_kind
        {
            return None;
        }
        slot.stage
            .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Relaxed)
            .ok()
            .map(|_| index)
    })
}

#[cfg(feature = "item_selftest")]
fn has_armed_request(base_kind: i32) -> bool {
    let thread = unsafe { current_thread() };
    PENDING.iter().any(|slot| {
        slot.owner_thread.load(Ordering::Acquire) == thread
            && slot.base_kind.load(Ordering::Acquire) == base_kind
            && slot.stage.load(Ordering::Acquire) == 1
    })
}

fn active_pending() -> Option<usize> {
    let thread = unsafe { current_thread() };
    PENDING.iter().enumerate().rev().find_map(|(index, slot)| {
        (slot.owner_thread.load(Ordering::Acquire) == thread
            && slot.stage.load(Ordering::Acquire) == 2)
            .then_some(index)
    })
}

pub(crate) fn active_public_kind() -> Option<i32> {
    let slot = active_pending()?;
    let kind = PENDING[slot].public_kind.load(Ordering::Acquire);
    definition(kind).map(|_| kind)
}

fn bind_active_agent(boma: usize, agent: usize) {
    let Some(index) = active_pending() else {
        return;
    };
    PENDING[index].boma.store(boma, Ordering::Release);
    PENDING[index].agent.store(agent, Ordering::Release);
}

fn release_pending(index: usize) {
    let slot = &PENDING[index];
    slot.stage.store(0, Ordering::Release);
    slot.public_kind.store(-1, Ordering::Relaxed);
    slot.base_kind.store(-1, Ordering::Relaxed);
    slot.parent_public_kind.store(-1, Ordering::Relaxed);
    slot.spawn_source
        .store(ItemSpawnSource::Unknown as u32, Ordering::Relaxed);
    slot.boma.store(0, Ordering::Relaxed);
    slot.agent.store(0, Ordering::Relaxed);
    slot.module.store(0, Ordering::Relaxed);
    slot.owner_thread.store(0, Ordering::Release);
}

fn retire_live(object: usize) {
    if object == 0 {
        return;
    }
    for slot in &LIVE {
        if slot.state.load(Ordering::Acquire) != 2 || slot.object.load(Ordering::Acquire) != object
        {
            continue;
        }
        if slot
            .state
            .compare_exchange(2, 1, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        let stale = slot.public_kind.load(Ordering::Relaxed);
        clear_live_slot(slot);
        LIVE_COUNT.fetch_sub(1, Ordering::AcqRel);
        limited_log(format!(
            "[itemclone] retired duplicate live entry object={object:#x} public={stale:#x}"
        ));
    }
}

fn clear_live_slot(slot: &LiveSlot) {
    slot.object.store(0, Ordering::Relaxed);
    slot.boma.store(0, Ordering::Relaxed);
    slot.agent.store(0, Ordering::Relaxed);
    slot.module.store(0, Ordering::Relaxed);
    slot.object_id.store(u32::MAX, Ordering::Relaxed);
    slot.public_kind.store(-1, Ordering::Relaxed);
    slot.base_kind.store(-1, Ordering::Relaxed);
    slot.parent_public_kind.store(-1, Ordering::Relaxed);
    slot.spawn_source
        .store(ItemSpawnSource::Unknown as u32, Ordering::Relaxed);
    slot.state.store(0, Ordering::Release);
}

unsafe fn publish_live(pending_index: usize, object: *mut u8) {
    if object.is_null() {
        return;
    }
    let pending = &PENDING[pending_index];
    let public_kind = pending.public_kind.load(Ordering::Acquire);
    let base_kind = pending.base_kind.load(Ordering::Acquire);
    let object_id = (object.add(0x08) as *const u32).read_unaligned();
    retire_live(object as usize);
    for slot in &LIVE {
        if slot
            .state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        slot.object.store(object as usize, Ordering::Relaxed);
        slot.boma
            .store(pending.boma.load(Ordering::Acquire), Ordering::Relaxed);
        slot.agent
            .store(pending.agent.load(Ordering::Acquire), Ordering::Relaxed);
        slot.module.store(
            pending.module.load(Ordering::Acquire),
            Ordering::Relaxed,
        );
        slot.object_id.store(object_id, Ordering::Relaxed);
        slot.public_kind.store(public_kind, Ordering::Relaxed);
        slot.base_kind.store(base_kind, Ordering::Relaxed);
        slot.parent_public_kind.store(
            pending.parent_public_kind.load(Ordering::Acquire),
            Ordering::Relaxed,
        );
        slot.spawn_source.store(
            pending.spawn_source.load(Ordering::Acquire),
            Ordering::Relaxed,
        );
        slot.state.store(2, Ordering::Release);
        LIVE_COUNT.fetch_add(1, Ordering::AcqRel);
        limited_log(format!(
            "[itemclone] live object={object:p} id={object_id:#x} public={public_kind:#x} base={base_kind:#x}"
        ));
        return;
    }
    log(format!(
        "[itemclone] ERROR live sidecar full; object={object:p} public={public_kind:#x} remains base-safe but has no custom identity"
    ));
}

pub(crate) unsafe fn bind_live_module(module: usize, base_kind: i32) {
    if module == 0 {
        return;
    }
    let object = core::ptr::read_volatile(module as *const usize);
    for slot in &LIVE {
        if slot.state.load(Ordering::Acquire) == 2
            && slot.object.load(Ordering::Acquire) == object
            && slot.base_kind.load(Ordering::Acquire) == base_kind
        {
            slot.module.store(module, Ordering::Release);
            return;
        }
    }
    let _ = active_pending();
}

fn live_kind_by(value: usize, field: fn(&LiveSlot) -> &AtomicUsize) -> Option<i32> {
    if value == 0 {
        return None;
    }
    LIVE.iter().find_map(|slot| {
        (slot.state.load(Ordering::Acquire) == 2 && field(slot).load(Ordering::Acquire) == value)
            .then(|| slot.public_kind.load(Ordering::Acquire))
    })
}

fn live_kind_by_object(object: usize) -> Option<i32> {
    live_kind_by(object, |slot| &slot.object)
}

fn live_kind_by_boma(boma: usize) -> Option<i32> {
    live_kind_by(boma, |slot| &slot.boma)
}

fn live_kind_by_agent(agent: usize) -> Option<i32> {
    live_kind_by(agent, |slot| &slot.agent)
}

pub(crate) fn live_kind_by_object_id(object_id: u32) -> Option<i32> {
    if object_id == u32::MAX {
        return None;
    }
    LIVE.iter().find_map(|slot| {
        (slot.state.load(Ordering::Acquire) == 2
            && slot.object_id.load(Ordering::Acquire) == object_id)
            .then(|| slot.public_kind.load(Ordering::Acquire))
    })
}

struct RetiredItem {
    public_kind: AtomicI32,
    base_kind: AtomicI32,
    generation: AtomicU32,
    claimed: AtomicBool,
}

impl RetiredItem {
    const fn new() -> Self {
        Self {
            public_kind: AtomicI32::new(-1),
            base_kind: AtomicI32::new(-1),
            generation: AtomicU32::new(0),
            claimed: AtomicBool::new(true),
        }
    }
}

static RETIRED: [RetiredItem; 16] = [const { RetiredItem::new() }; 16];
static RETIRED_CURSOR: AtomicUsize = AtomicUsize::new(0);
static RETIRED_GENERATION: AtomicU32 = AtomicU32::new(0);

const RETIRED_CLAIM_WINDOW: u32 = 4;

static SWEEP_LOG: AtomicU32 = AtomicU32::new(0);

fn open_retirement_window() {
    RETIRED_GENERATION.fetch_add(1, Ordering::AcqRel);
}

fn remember_retired(public_kind: i32, base_kind: i32) {
    if public_kind < 0 {
        return;
    }
    let index = RETIRED_CURSOR.fetch_add(1, Ordering::Relaxed) % RETIRED.len();
    let entry = &RETIRED[index];
    entry.claimed.store(true, Ordering::Release);
    entry.public_kind.store(public_kind, Ordering::Relaxed);
    entry.base_kind.store(base_kind, Ordering::Relaxed);
    entry.generation.store(
        RETIRED_GENERATION.load(Ordering::Acquire),
        Ordering::Relaxed,
    );
    entry.claimed.store(false, Ordering::Release);
}

pub(crate) fn live_clones_with_base(base_kind: i32, out: &mut [(u32, i32)]) -> usize {
    let mut count = 0;
    for slot in &LIVE {
        if count == out.len() {
            break;
        }
        if slot.state.load(Ordering::Acquire) != 2
            || slot.base_kind.load(Ordering::Relaxed) != base_kind
        {
            continue;
        }
        let object_id = slot.object_id.load(Ordering::Relaxed);
        let public_kind = slot.public_kind.load(Ordering::Relaxed);
        if object_id == u32::MAX || public_kind < 0 {
            continue;
        }
        out[count] = (object_id, public_kind);
        count += 1;
    }
    count
}

pub(crate) fn claim_retired_clone(base_kind: i32) -> Option<i32> {
    let now = RETIRED_GENERATION.load(Ordering::Acquire);
    let mut best: Option<(usize, u32)> = None;
    for (index, entry) in RETIRED.iter().enumerate() {
        if entry.claimed.load(Ordering::Acquire)
            || entry.base_kind.load(Ordering::Relaxed) != base_kind
            || entry.public_kind.load(Ordering::Relaxed) < 0
        {
            continue;
        }
        let generation = entry.generation.load(Ordering::Relaxed);
        if now.wrapping_sub(generation) > RETIRED_CLAIM_WINDOW {
            continue;
        }
        if best.map_or(true, |(_, newest)| {
            generation.wrapping_sub(newest) as i32 >= 0
        }) {
            best = Some((index, generation));
        }
    }
    let (index, _) = best?;
    RETIRED[index]
        .claimed
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .ok()?;
    Some(RETIRED[index].public_kind.load(Ordering::Relaxed))
}

#[cfg(test)]
mod retired_window_tests {
    use super::{claim_retired_clone, open_retirement_window, remember_retired};
    use std::sync::Mutex;

    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn a_clone_that_vanished_in_this_window_is_claimable_once() {
        let _serial = SERIAL.lock().unwrap();
        open_retirement_window();
        remember_retired(0x36b, 0x7001);
        assert_eq!(claim_retired_clone(0x7001), Some(0x36b));
        assert_eq!(claim_retired_clone(0x7001), None);
    }

    #[test]
    fn a_base_kind_no_clone_vanished_under_resolves_to_nothing() {
        let _serial = SERIAL.lock().unwrap();
        open_retirement_window();
        remember_retired(0x36b, 0x7002);
        assert_eq!(claim_retired_clone(0x7012), None);
    }

    #[test]
    fn a_clone_that_vanished_long_ago_is_out_of_the_window() {
        let _serial = SERIAL.lock().unwrap();
        open_retirement_window();
        remember_retired(0x36a, 0x7003);
        for _ in 0..=super::RETIRED_CLAIM_WINDOW {
            open_retirement_window();
        }
        assert_eq!(claim_retired_clone(0x7003), None);
    }

    #[test]
    fn the_newest_of_two_vanishings_wins() {
        let _serial = SERIAL.lock().unwrap();
        open_retirement_window();
        remember_retired(0x36a, 0x7004);
        open_retirement_window();
        remember_retired(0x36c, 0x7004);
        assert_eq!(claim_retired_clone(0x7004), Some(0x36c));
    }
}

pub(crate) fn sweep_live(is_active: impl Fn(u32) -> bool) {
    open_retirement_window();
    for slot in &LIVE {
        if slot.state.load(Ordering::Acquire) != 2 {
            continue;
        }
        let object_id = slot.object_id.load(Ordering::Acquire);
        if object_id == u32::MAX || is_active(object_id) {
            continue;
        }
        if slot
            .state
            .compare_exchange(2, 1, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        let public_kind = slot.public_kind.load(Ordering::Relaxed);
        let base_kind = slot.base_kind.load(Ordering::Relaxed);
        remember_retired(public_kind, base_kind);
        clear_live_slot(slot);
        LIVE_COUNT.fetch_sub(1, Ordering::AcqRel);
        let n = SWEEP_LOG.fetch_add(1, Ordering::Relaxed);
        if n < 24 {
            limited_log(format!(
                "[itemclone] sweep #{n} retired clone {public_kind:#x} base {base_kind:#x} object id {object_id:#x} left the match without a deactivate"
            ));
        }
    }
}

pub(crate) fn clone_base_kind(public_kind: i32) -> Option<i32> {
    definition(public_kind).map(|view| view.base_kind)
}

pub(crate) fn clone_category(public_kind: i32) -> Option<ItemCategory> {
    definition(public_kind).map(|view| view.category)
}

fn remove_live(object: usize, object_id: u32) -> Option<i32> {
    let mut released = None;
    for slot in &LIVE {
        if slot.state.load(Ordering::Acquire) != 2
            || slot.object.load(Ordering::Acquire) != object
            || slot.object_id.load(Ordering::Acquire) != object_id
        {
            continue;
        }
        if slot
            .state
            .compare_exchange(2, 1, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            continue;
        }
        let kind = slot.public_kind.load(Ordering::Relaxed);
        remember_retired(kind, slot.base_kind.load(Ordering::Relaxed));
        clear_live_slot(slot);
        LIVE_COUNT.fetch_sub(1, Ordering::AcqRel);
        released = released.or(Some(kind));
    }
    released
}

unsafe fn rewrite_requested_kind(ctx: &mut skyline::hooks::InlineCtx) {
    let requested = ctx.registers[1].x() as u32 as i32;
    if let Some(definition) = definition(requested) {
        REGISTRATION_CLOSED.store(true, Ordering::Release);
        BACKEND_STATUS.fetch_or(STATUS_REGISTRATION_CLOSED, Ordering::AcqRel);
        let armed = arm_request(definition);
        ctx.registers[1].set_x(definition.base_kind as u32 as u64);
        limited_log(format!(
            "[itemclone] request public={requested:#x} -> base={:#x} armed={armed}",
            definition.base_kind
        ));
        return;
    }
    let Some((definition, source)) = take_spawn_ticket(requested) else {
        return;
    };
    REGISTRATION_CLOSED.store(true, Ordering::Release);
    BACKEND_STATUS.fetch_or(STATUS_REGISTRATION_CLOSED, Ordering::AcqRel);
    let armed = arm_request_with_context(definition, -1, source);
    limited_log(format!(
        "[itemclone] request ticket base={requested:#x} -> public={:#x} source={source:?} armed={armed}",
        definition.public_kind
    ));
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_REQUEST_FULL), inline)]
unsafe fn item_request_full_bridge(ctx: &mut skyline::hooks::InlineCtx) {
    rewrite_requested_kind(ctx);
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_REQUEST_SIMPLE), inline)]
unsafe fn item_request_simple_bridge(ctx: &mut skyline::hooks::InlineCtx) {
    rewrite_requested_kind(ctx);
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_HAVE_ITEM), inline)]
unsafe fn item_have_item_bridge(ctx: &mut skyline::hooks::InlineCtx) {
    rewrite_requested_kind(ctx);
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_BORN_ITEM), inline)]
unsafe fn item_born_item_bridge(ctx: &mut skyline::hooks::InlineCtx) {
    rewrite_requested_kind(ctx);
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_ATTACH_ITEM), inline)]
unsafe fn item_attach_item_bridge(ctx: &mut skyline::hooks::InlineCtx) {
    rewrite_requested_kind(ctx);
}

#[skyline::from_offset(crate::game::rel(OFF_ITEM_GET_HAVE_ITEM_ID))]
fn native_get_have_item_id(boma: *mut u8, index: i32) -> u32;

#[skyline::from_offset(crate::game::rel(OFF_ITEM_GET_PICKABLE_ITEM_OBJECT_ID))]
fn native_get_pickable_item_object_id(boma: *mut u8) -> u32;

unsafe fn publish_kind(reported: i32, object_id: u32) -> i32 {
    match live_kind_by_object_id(object_id) {
        Some(public) if definition(public).is_some_and(|view| view.base_kind == reported) => public,
        _ => reported,
    }
}

#[skyline::from_offset(crate::game::rel(OFF_ITEM_GET_HAVE_ITEM_KIND))]
fn native_get_have_item_kind(boma: *mut u8, index: i32) -> i32;

#[skyline::from_offset(crate::game::rel(OFF_ITEM_GET_PICKABLE_ITEM_KIND))]
fn native_get_pickable_item_kind(boma: *mut u8) -> i32;

#[no_mangle]
pub unsafe extern "C" fn clone_engine_item_kind_held_v1(boma: *mut u8, index: i32) -> i32 {
    if boma.is_null() {
        return -1;
    }
    let reported = native_get_have_item_kind(boma, index);
    if LIVE_COUNT.load(Ordering::Acquire) == 0 {
        return reported;
    }
    publish_kind(reported, native_get_have_item_id(boma, index))
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_item_kind_pickable_v1(boma: *mut u8) -> i32 {
    if boma.is_null() {
        return -1;
    }
    let reported = native_get_pickable_item_kind(boma);
    if LIVE_COUNT.load(Ordering::Acquire) == 0 {
        return reported;
    }
    publish_kind(reported, native_get_pickable_item_object_id(boma))
}

const ITEM_MODULE_OFFSET: usize = 0xC8;
const ITEM_MODULE_ENTRIES: usize = 0x18;

unsafe fn item_module_is_ready(boma: *mut u8) -> bool {
    if boma.is_null() {
        return false;
    }
    let module = core::ptr::read_volatile(boma.add(ITEM_MODULE_OFFSET) as *const usize);
    if module == 0 || core::ptr::read_volatile(module as *const usize) == 0 {
        return false;
    }
    let entries = core::ptr::read_volatile((module + ITEM_MODULE_ENTRIES) as *const usize);
    if entries == 0 {
        return false;
    }
    core::ptr::read_volatile((entries + 8) as *const usize) != 0
}

pub(crate) unsafe fn pocket_contact_object_ids(boma: *mut u8) -> (u32, u32) {
    if LIVE_COUNT.load(Ordering::Acquire) == 0 || !item_module_is_ready(boma) {
        return (u32::MAX, u32::MAX);
    }
    (
        native_get_have_item_id(boma, 0),
        native_get_pickable_item_object_id(boma),
    )
}

pub(crate) fn clone_kind_of_object(object_id: u32, base_kind: i32) -> Option<i32> {
    if object_id == u32::MAX || object_id == crate::article_probes::INVALID_BATTLE_OBJECT_ID {
        return None;
    }
    let public_kind = live_kind_by_object_id(object_id)?;
    (definition(public_kind)?.base_kind == base_kind).then_some(public_kind)
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_LOWER_CREATOR))]
unsafe fn item_lower_creator_bridge(
    manager: *mut u8,
    descriptor: *const u8,
    creator_flag: i32,
    arg3: i32,
    arg4: i32,
) -> *mut u8 {
    if descriptor.is_null() {
        return call_original!(manager, descriptor, creator_flag, arg3, arg4);
    }

    let raw_kind = (descriptor.add(0x20) as *const i32).read_unaligned();
    let claimed = take_spawn_ticket(raw_kind).or_else(|| {
        let public_kind = crate::pocket_release_claim(raw_kind)?;
        Some((definition(public_kind)?, ItemSpawnSource::Direct))
    });
    if let Some((definition, source)) = claimed {
        REGISTRATION_CLOSED.store(true, Ordering::Release);
        BACKEND_STATUS.fetch_or(STATUS_REGISTRATION_CLOSED, Ordering::AcqRel);
        if matches!(definition.category, ItemCategory::Assist | ItemCategory::Pokemon | ItemCategory::Boss) {
            crate::item_slots::prepare(definition.public_kind);
        }
        let armed = arm_request_with_context(definition, -1, source);
        limited_log(format!(
            "[itemclone] spawn ticket public={:#x} base={raw_kind:#x} source={source:?} armed={armed}",
            definition.public_kind
        ));
    }
    #[cfg(feature = "item_selftest")]
    let raw_kind = selftest::redirect_spawn(raw_kind);
    let mut descriptor_copy = ItemCreateDescriptor([0; 0x50]);
    let (effective_descriptor, pending_index) = if let Some(index) = take_request(raw_kind) {
        (descriptor, Some(index))
    } else if let Some(definition) = definition(raw_kind) {
        if matches!(definition.category, ItemCategory::Assist | ItemCategory::Pokemon | ItemCategory::Boss) {
            crate::item_slots::prepare(raw_kind);
        }
        core::ptr::copy_nonoverlapping(descriptor, descriptor_copy.0.as_mut_ptr(), 0x50);
        (descriptor_copy.0.as_mut_ptr().add(0x20) as *mut i32)
            .write_unaligned(definition.base_kind);
        if !arm_request(definition) {
            return core::ptr::null_mut();
        }
        let Some(index) = take_request(definition.base_kind) else {
            return core::ptr::null_mut();
        };
        (descriptor_copy.0.as_ptr(), Some(index))
    } else {
        (descriptor, None)
    };

    if let Some(index) = pending_index {
        let public_kind = PENDING[index].public_kind.load(Ordering::Acquire);
        #[cfg(feature = "item_clone_backend")]
        crate::item_scripts::ensure_agent(public_kind);
        limited_log(format!(
            "[itemclone] construct-enter public={public_kind:#x} descriptor={effective_descriptor:p}"
        ));
    }
    let construct_scope = pending_index.and_then(|index| {
        let public = PENDING[index].public_kind.load(Ordering::Acquire);
        let base = PENDING[index].base_kind.load(Ordering::Acquire);
        #[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
        if let Some(owner) = base_item_owner(base) {
            crate::fighter_params::ensure_kind_params_loaded(owner);
        }
        let scope = crate::item_params::enter_runtime_clone(public, base);
        let live = crate::item_params::common_row(base)
            .map(|row| core::ptr::read_volatile(row.byte_add(0x18)))
            .unwrap_or(f32::NAN);
        limited_log(format!(
            "[itemcommon] construct scope={} base={base:#x} row+0x18={live} (want 3000 if the override is in force)",
            if scope.is_some() { "OPEN" } else { "REFUSED" }
        ));
        scope
    });
    let item = call_original!(manager, effective_descriptor, creator_flag, arg3, arg4);
    if let Some(scope) = construct_scope {
        crate::item_params::leave_runtime_clone(scope);
    }
    if let Some(index) = pending_index {
        limited_log(format!(
            "[itemclone] construct-exit public={:#x} item={item:p}",
            PENDING[index].public_kind.load(Ordering::Acquire)
        ));
        publish_live(index, item);
        release_pending(index);
    }
    item
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_OBJECT_ADOPT), inline)]
unsafe fn item_object_adopt(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(public_kind) = crate::item_params::runtime_clone_on_thread() else {
        return;
    };
    crate::clone_vtables::adopt_item_object(public_kind, ctx.registers[22].x() as usize);
}

#[skyline::hook(offset = crate::game::rel(OFF_ITEM_DEACTIVATE))]
unsafe fn item_deactivate_bridge(manager: *mut u8, item: *mut u8, recycle: u32) {
    let object_id = if item.is_null() {
        u32::MAX
    } else {
        (item.add(0x08) as *const u32).read_unaligned()
    };
    call_original!(manager, item, recycle);
    crate::clone_vtables::release_item_object(item as usize);
    if let Some(kind) = remove_live(item as usize, object_id) {
        crate::block_grid::note_released_object(item as usize);
        limited_log(format!(
            "[itemclone] release object={item:p} id={object_id:#x} public={kind:#x} recycle={recycle}"
        ));
    }
}

type ItemAgentDispatcher =
    unsafe extern "C" fn(Hash40, *mut u8, *mut u8, *mut libc::c_void) -> *mut u8;

static ITEM_NRO_BASE: AtomicUsize = AtomicUsize::new(0);
static ITEM_AGENT_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn item_nro_base() -> usize {
    ITEM_NRO_BASE.load(Ordering::Acquire)
}

pub(crate) unsafe fn const_table() -> Option<usize> {
    let base = item_nro_base();
    if base == 0 {
        return None;
    }
    let slot = (base + crate::item_status_tables::CONST_VALUE_TABLE_GOT) as *const usize;
    let table = core::ptr::read_volatile(slot);
    if table == 0 {
        return None;
    }
    let sentinel = core::ptr::read_volatile(
        (table + crate::item_status_tables::CONST_VALUE_TABLE_SENTINEL as usize) as *const i32,
    );
    if sentinel == 0 {
        return None;
    }
    Some(table)
}

unsafe fn const_value(offset: u32) -> Option<i32> {
    let table = const_table()?;
    Some(core::ptr::read_volatile(
        (table + offset as usize) as *const i32,
    ))
}

pub(crate) fn item_status_kind_by_name(name: &str) -> Option<i32> {
    let table = &crate::item_status_tables::ITEM_STATUS_KINDS;
    let index = table
        .binary_search_by(|(known, _)| (*known).cmp(name))
        .ok()?;
    unsafe { const_value(table[index].1) }
}

pub(crate) fn item_status_line_value(ordinal: u32) -> Option<i32> {
    let (_, offset) = *crate::item_status_tables::ITEM_STATUS_LINES.get(ordinal as usize)?;
    unsafe { const_value(offset) }
}

#[cfg(feature = "item_selftest")]
unsafe extern "C" fn selftest_status_init(
    _agent: &mut smash::lua2cpp::L2CFighterCommon,
) -> smash::lib::L2CValue {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::AcqRel) {
        log("[itemstatus] CALLBACK RAN: clone status script entered".to_string());
    }
    smash::lib::L2CValue::new_int(0)
}

#[cfg(feature = "item_selftest")]
fn arm_status_selftest() {
    static ARMED: AtomicBool = AtomicBool::new(false);
    if ARMED.load(Ordering::Acquire) {
        return;
    }
    let Some(throw) = item_status_kind_by_name("THROW") else {
        return;
    };
    let Some(definition) = definitions()
        .read()
        .ok()
        .and_then(|all| all.first().map(|d| d.public_kind))
    else {
        return;
    };
    if ARMED.swap(true, Ordering::AcqRel) {
        return;
    }
    let result = clone_engine_item_status_v1(
        definition,
        2,
        throw,
        selftest_status_init as *const () as usize,
    );
    const EXERCISE_COMMON: bool = false;
    let common = if EXERCISE_COMMON {
        let mut worst = RESULT_OK;
        for (index, (hash, _offset)) in crate::item_common_tables::ITEM_COMMON_FLOATS
            .iter()
            .enumerate()
        {
            let result = clone_engine_item_common_set(definition, *hash, 3000.0 + index as f32);
            if result != RESULT_OK {
                worst = result;
            }
        }
        worst
    } else {
        i32::MIN
    };
    log(format!(
        "[itemstatus] selftest armed public={definition:#x} THROW={throw} line=2(init) \
         status_result={result} common_result={common}"
    ));
}

pub(crate) fn report_status_constants() {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::AcqRel) {
        return;
    }
    let base = item_nro_base();
    let table = if base == 0 {
        0
    } else {
        unsafe {
            core::ptr::read_volatile(
                (base + crate::item_status_tables::CONST_VALUE_TABLE_GOT) as *const usize,
            )
        }
    };
    log(format!(
        "[itemstatus] nro={base:#x} const_value_table__={table:#x}"
    ));
    let names = [
        "INITIALIZE",
        "HAVE",
        "FALL",
        "LANDING",
        "THROW",
        "LOST",
        "BORN",
        "WAIT",
    ];
    let mut resolved = String::new();
    for name in names {
        match item_status_kind_by_name(name) {
            Some(kind) => resolved.push_str(&format!(" {name}={kind}")),
            None => resolved.push_str(&format!(" {name}=?")),
        }
    }
    log(format!("[itemstatus] kinds{resolved}"));
    let mut lines = String::new();
    for (ordinal, (name, _)) in crate::item_status_tables::ITEM_STATUS_LINES
        .iter()
        .enumerate()
    {
        match item_status_line_value(ordinal as u32) {
            Some(value) => lines.push_str(&format!(" {ordinal}:{name}={value}")),
            None => lines.push_str(&format!(" {ordinal}:{name}=?")),
        }
    }
    log(format!("[itemstatus] lines{lines}"));
    #[cfg(feature = "item_selftest")]
    arm_status_selftest();
}

unsafe fn ensure_set_status_hook(base_kind: i32) -> bool {
    let Some(shape) = crate::item_status_tables::ITEM_STATUS_AGENTS
        .get(base_kind as usize)
        .copied()
        .flatten()
    else {
        return false;
    };
    let nro = item_nro_base();
    if nro == 0 {
        return false;
    }
    let target = nro + shape.set_status as usize;
    let cell = &SET_STATUS_ORIGINALS[base_kind as usize];
    if cell.load(Ordering::Acquire) != 0 {
        return true;
    }
    let Ok(_guard) = SET_STATUS_HOOK_LOCK.lock() else {
        return false;
    };
    if cell.load(Ordering::Acquire) != 0 {
        return true;
    }

    for (kind, candidate) in crate::item_status_tables::ITEM_STATUS_AGENTS
        .iter()
        .enumerate()
    {
        let Some(candidate) = candidate else {
            continue;
        };
        if candidate.set_status != shape.set_status {
            continue;
        }
        let original = SET_STATUS_ORIGINALS[kind].load(Ordering::Acquire);
        if original != 0 {
            cell.store(original, Ordering::Release);
            return true;
        }
    }

    let found = core::ptr::read_volatile(target as *const u32);
    if found != shape.set_status_word {
        log(format!(
            "[itemclone] set_status preflight FAILED base={base_kind:#x} at {target:#x}: found={found:#010x} expected={:#010x}",
            shape.set_status_word
        ));
        return false;
    }

    let mut original = core::ptr::null_mut();
    skyline::hooks::A64HookFunction(
        target as *const libc::c_void,
        item_set_status_bridge as *const () as *const libc::c_void,
        &mut original,
    );
    if original.is_null() {
        log(format!(
            "[itemclone] set_status hook FAILED base={base_kind:#x} at {target:#x}"
        ));
        return false;
    }

    for (kind, candidate) in crate::item_status_tables::ITEM_STATUS_AGENTS
        .iter()
        .enumerate()
    {
        if candidate
            .map(|candidate| candidate.set_status == shape.set_status)
            .unwrap_or(false)
        {
            SET_STATUS_ORIGINALS[kind].store(original as usize, Ordering::Release);
        }
    }
    log(format!(
        "[itemclone] set_status hooked base={base_kind:#x} at {target:#x} original={original:p}"
    ));
    true
}

fn live_slot_by_object(object: usize) -> Option<&'static LiveSlot> {
    if object == 0 {
        return None;
    }
    LIVE.iter().find(|slot| {
        slot.state.load(Ordering::Acquire) == 2 && slot.object.load(Ordering::Acquire) == object
    })
}

static SET_STATUS_ORIGINALS: [AtomicUsize; NATIVE_ITEM_COUNT] =
    [const { AtomicUsize::new(0) }; NATIVE_ITEM_COUNT];
static SET_STATUS_HOOK_LOCK: Mutex<()> = Mutex::new(());

pub(crate) unsafe fn agent_vtable_slots(vtable: usize) -> Option<usize> {
    let nro = item_nro_base();
    if nro == 0 {
        return None;
    }
    let relative = vtable.checked_sub(nro)?;
    crate::item_status_tables::ITEM_STATUS_AGENTS
        .iter()
        .flatten()
        .find(|candidate| candidate.vtable as usize == relative)
        .map(|candidate| candidate.slots as usize)
}

unsafe fn status_agent_base_kind(agent: *mut u8) -> Option<i32> {
    if agent.is_null() {
        return None;
    }
    let nro = item_nro_base();
    let held = core::ptr::read_volatile(agent as *const usize);
    let vtable = crate::common_vtables::original_vtable(held).unwrap_or(held);
    let relative = vtable.checked_sub(nro)?;
    crate::item_status_tables::ITEM_STATUS_AGENTS
        .iter()
        .position(|candidate| {
            candidate
                .map(|candidate| candidate.vtable as usize == relative)
                .unwrap_or(false)
        })
        .and_then(|kind| i32::try_from(kind).ok())
}

unsafe extern "C" fn item_agent_dispatch_bridge(
    mut agent_name: Hash40,
    object: *mut u8,
    boma: *mut u8,
    lua_state: *mut libc::c_void,
) -> *mut u8 {
    let custom_kind = active_public_kind().or_else(|| live_kind_by_object(object as usize));
    if let Some(kind) = custom_kind {
        if let Some(definition) = definition(kind) {
            if agent_name.hash == definition.custom_agent_hash {
                agent_name.hash = definition.base_agent_hash;
            }
        }
    }

    let original = ITEM_AGENT_ORIGINAL.load(Ordering::Acquire);
    if original == 0 {
        return core::ptr::null_mut();
    }
    let mut lua_state = lua_state;
    if let Some(kind) = custom_kind {
        let states = crate::item_slots::script_lua_states(kind);
        if let Some((clone_state, base_state)) = states {
            if lua_state as usize == base_state && clone_state != base_state {
                lua_state = clone_state as *mut libc::c_void;
                static SWAPPED: AtomicBool = AtomicBool::new(false);
                if !SWAPPED.swap(true, Ordering::AcqRel) {
                    limited_log(format!(
                        "[itemclone] clone {kind:#x} agent built with its OWN lua_State {clone_state:#x} (base was {base_state:#x})"
                    ));
                }
            }
        }
        static EXPLAINED: AtomicBool = AtomicBool::new(false);
        if !EXPLAINED.swap(true, Ordering::AcqRel) {
            limited_log(format!(
                "[itemclone] clone {kind:#x} agent lua_State in={lua_state:p} known={}",
                match states {
                    Some((clone_state, base_state)) =>
                        format!("clone {clone_state:#x} / base {base_state:#x}"),
                    None => "neither yet".to_string(),
                }
            ));
            if let Some((clone_state, base_state)) = states {
                let global =
                    |state: usize| core::ptr::read_volatile((state + 0x18) as *const usize);
                let head = |state: usize| {
                    (0..4)
                        .map(|step| {
                            format!(
                                "{:016x}",
                                core::ptr::read_volatile((state + step * 8) as *const usize)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                let (agent_g, clone_g, base_g) = (
                    global(lua_state as usize),
                    global(clone_state),
                    global(base_state),
                );
                limited_log(format!(
                    "[itemclone]   l_G agent={agent_g:#x} clone={clone_g:#x} base={base_g:#x} -> agent shares {}",
                    if agent_g == clone_g && agent_g == base_g {
                        "ONE global state with both"
                    } else if agent_g == clone_g {
                        "the CLONE's Lua world"
                    } else if agent_g == base_g {
                        "the BASE's Lua world"
                    } else {
                        "NEITHER - a third Lua world"
                    }
                ));
                limited_log(format!(
                    "[itemclone]   agent {lua_state:p}: {}",
                    head(lua_state as usize)
                ));
                limited_log(format!(
                    "[itemclone]   clone {clone_state:#x}: {}",
                    head(clone_state)
                ));
                limited_log(format!(
                    "[itemclone]   base  {base_state:#x}: {}",
                    head(base_state)
                ));
            }
        }
    }

    let original: ItemAgentDispatcher = core::mem::transmute(original);
    let scope = custom_kind.and_then(|kind| {
        definition(kind).and_then(|view| {
            crate::item_params::enter_runtime_clone(view.public_kind, view.base_kind)
        })
    });
    let agent = original(agent_name, object, boma, lua_state);
    if let Some(index) = scope {
        crate::item_params::leave_runtime_clone(index);
    }
    if let Some(kind) = custom_kind {
        bind_active_agent(boma as usize, agent as usize);
        crate::common_vtables::install(
            crate::common_vtables::SPACE_ITEM,
            kind,
            agent as u64,
        );
        #[cfg(feature = "item_selftest")]
        arm_status_selftest();
        let hooked = if scripts_for(kind).is_empty() {
            false
        } else {
            definition(kind)
                .map(|view| ensure_set_status_hook(view.base_kind))
                .unwrap_or(false)
        };
        limited_log(format!(
            "[itemclone] agent public={kind:#x} object={object:p} boma={boma:p} agent={agent:p} base_hash={:#x} set_status_hook={hooked}",
            agent_name.hash,
        ));
    }
    agent
}

unsafe extern "C" fn item_set_status_bridge(agent: *mut u8) {
    let Some(base_kind) = status_agent_base_kind(agent) else {
        static UNKNOWN: AtomicBool = AtomicBool::new(false);
        if !UNKNOWN.swap(true, Ordering::AcqRel) {
            limited_log(format!(
                "[itemclone] set_status bridge could not identify agent={agent:p}; clone layer skipped"
            ));
        }
        return;
    };
    let original = SET_STATUS_ORIGINALS[base_kind as usize].load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    let call: unsafe extern "C" fn(*mut u8) = core::mem::transmute(original);
    call(agent);

    let kind = active_public_kind().or_else(|| live_kind_by_agent(agent as usize));
    let Some(kind) = kind else {
        static MISSED: AtomicBool = AtomicBool::new(false);
        if !MISSED.swap(true, Ordering::AcqRel) {
            limited_log(format!(
                "[itemclone] set_status hook ran for agent={agent:p} with no clone identity \
                 (vanilla item, or the scope closed early)"
            ));
        }
        return;
    };
    let Some(view) = definition(kind) else {
        return;
    };
    if view.base_kind != base_kind {
        static MISMATCHED: AtomicBool = AtomicBool::new(false);
        if !MISMATCHED.swap(true, Ordering::AcqRel) {
            limited_log(format!(
                "[itemclone] set_status hook/base mismatch public={kind:#x} expected_base={:#x} hook_base={base_kind:#x}; clone layer skipped",
                view.base_kind
            ));
        }
        return;
    }
    let scripts = scripts_for(kind);
    if scripts.is_empty() {
        return;
    }
    let base = &mut *(agent as *mut smash::lua2cpp::L2CAgentBase);
    for (index, script) in scripts {
        let Some(line_value) = item_status_line_value(script.line as u32) else {
            continue;
        };
        let Some(status_kind) = script.status_kind() else {
            static UNRESOLVED: AtomicBool = AtomicBool::new(false);
            if !UNRESOLVED.swap(true, Ordering::AcqRel) {
                limited_log(format!(
                    "[itemclone] status {} for public={kind:#x} did not resolve at dispatch",
                    script.describe()
                ));
            }
            continue;
        };
        let status = smash::lib::L2CValue::new_int(status_kind as u32 as u64);
        let line = smash::lib::L2CValue::new_int(line_value as u32 as u64);

        let installed = if index < MAX_CHAINED_STATUS_SCRIPTS {
            let existing = base.sv_get_status_func(&status, &line);
            match status_function_pointer(&existing) {
                Ok(original) => {
                    let shim = CHAIN_SHIMS[index] as usize;
                    if original != shim {
                        CHAIN_ORIGINAL[index].store(original, Ordering::Release);
                    }
                    CHAIN_TARGET[index].store(script.function, Ordering::Release);
                    base.sv_set_status_func(status, line, core::mem::transmute(shim));
                    let kept = CHAIN_ORIGINAL[index].load(Ordering::Acquire);
                    format!("chained over {kept:#x}")
                }
                Err(val_type) => {
                    base.sv_set_status_func(status, line, core::mem::transmute(script.function));
                    format!("REPLACED: existing function has val_type {val_type}")
                }
            }
        } else {
            base.sv_set_status_func(status, line, core::mem::transmute(script.function));
            format!("REPLACED: over {MAX_CHAINED_STATUS_SCRIPTS} scripts registered")
        };

        limited_log(format!(
            "[itemclone] status public={kind:#x} status={}={status_kind:#x} line={}={line_value:#x} ({installed})",
            script.describe(),
            script.line
        ));
    }
}

unsafe fn words_match(address: usize, expected: &[u32]) -> bool {
    expected
        .iter()
        .enumerate()
        .all(|(index, word)| (address as *const u32).add(index).read() == *word)
}

unsafe fn main_preflight() -> Result<(), (usize, u32, u32)> {
    for (offset, expected) in MAIN_PREFLIGHT {
        for (index, wanted) in expected.iter().enumerate() {
            let address = crate::game::addr(*offset) + index * 4;
            let found = crate::game::text_word(*offset + index * 4);
            if found != *wanted {
                return Err((address, found, *wanted));
            }
        }
    }
    Ok(())
}

fn item_nro_hook(info: &skyline::nro::NroInfo) {
    if info.name != "item" {
        return;
    }
    unsafe {
        let module = info.module.ModuleObject;
        if module.is_null() {
            log("[itemclone] lua2cpp_item has no ModuleObject; routing disabled".to_string());
            return;
        }
        let base = (*module).module_base as usize;
        let dispatcher = base + ITEM_NRO_DISPATCHER;
        if !words_match(dispatcher, &ITEM_NRO_DISPATCHER_WORDS) {
            log("[itemclone] lua2cpp_item preflight mismatch; routing disabled".to_string());
            return;
        }
        if ITEM_NRO_BASE.load(Ordering::Acquire) == base {
            return;
        }

        #[allow(unreachable_code)]
        let mut dispatcher_original = core::ptr::null_mut();
        #[allow(unreachable_code)]
        skyline::hooks::A64HookFunction(
            dispatcher as *const libc::c_void,
            item_agent_dispatch_bridge as *const () as *const libc::c_void,
            &mut dispatcher_original,
        );
        if dispatcher_original.is_null() {
            log("[itemclone] item-agent hook failed".to_string());
            return;
        }
        ITEM_AGENT_ORIGINAL.store(dispatcher_original as usize, Ordering::Release);
        ITEM_NRO_BASE.store(base, Ordering::Release);
        log(format!("[itemclone] lua2cpp_item base={base:#x}"));
        BACKEND_STATUS.fetch_or(
            STATUS_ITEM_NRO_READY
                | STATUS_STATUS_ROUTER_READY
                | STATUS_MULTI_BASE_STATUS_ROUTER_READY,
            Ordering::AcqRel,
        );
        limited_log(format!(
            "[itemclone] lua2cpp_item routing ready base={base:#x} dispatcher={dispatcher:#x}; clone status callbacks use base-specific set_status hooks"
        ));
    }
}

fn registration_conflicts(
    existing: &[ItemCloneDefinition],
    staged: &[ItemCloneDefinition],
) -> bool {
    staged.iter().enumerate().any(|(index, candidate)| {
        existing.iter().chain(&staged[..index]).any(|known| {
            known.public_kind == candidate.public_kind
                || known.resource_name.as_c_str() == candidate.resource_name.as_c_str()
                || known.agent_name.as_c_str() == candidate.agent_name.as_c_str()
        })
    })
}

fn commit_item_definitions(mut staged: Vec<ItemCloneDefinition>) -> i32 {
    if staged.is_empty() {
        return ERROR_ITEM_FAMILY_EMPTY;
    }
    let Ok(mut existing) = definitions().write() else {
        return ERROR_UNSUPPORTED;
    };
    if REGISTRATION_CLOSED.load(Ordering::Acquire) {
        return ERROR_REGISTRATION_CLOSED;
    }
    if registration_conflicts(&existing, &staged) {
        return ERROR_DUPLICATE;
    }
    let Some(total) = existing.len().checked_add(staged.len()) else {
        return ERROR_ITEM_FAMILY_CAPACITY;
    };
    if total > crate::item_params::MAX_CLONE_KINDS {
        return ERROR_ITEM_FAMILY_CAPACITY;
    }
    let first_compact = FIRST_COMPACT_RESOURCE_KIND as usize + existing.len();
    let Some(compact_end) = first_compact.checked_add(staged.len()) else {
        return ERROR_ITEM_FAMILY_CAPACITY;
    };
    if compact_end > MAX_COMPACT_RESOURCE_COUNT {
        return ERROR_ITEM_FAMILY_CAPACITY;
    }
    for (index, definition) in staged.iter_mut().enumerate() {
        definition.compact_kind = (first_compact + index) as i32;
    }

    let public_kinds = staged
        .iter()
        .map(|definition| definition.public_kind)
        .collect::<Vec<_>>();
    let resources = staged
        .iter()
        .map(|definition| crate::item_slots::CloneResourceRegistration {
            public_kind: definition.public_kind,
            base_kind: definition.base_kind,
            resource_name: definition.resource_name.as_ptr(),
        })
        .collect::<Vec<_>>();
    if !crate::item_params::can_register_family(&public_kinds) {
        return ERROR_ITEM_FAMILY_CAPACITY;
    }
    if !crate::item_slots::can_register_family(&resources) {
        return ERROR_ITEM_RESOURCE_UNAVAILABLE;
    }

    if !crate::item_params::register_family(&public_kinds) {
        return ERROR_ITEM_FAMILY_CAPACITY;
    }
    if !crate::item_slots::register_family(&resources) {
        log(format!(
            "[itemclone] family resource commit unexpectedly failed after preflight; owner={:#x}",
            staged[0].public_kind
        ));
        return ERROR_ITEM_RESOURCE_UNAVAILABLE;
    }

    for definition in &staged {
        limited_log(format!(
            "[itemclone] registered public={:#x} base={:#x} compact={:#x} category={:?} family={:#x}[{}/{}] resource={} agent={}",
            definition.public_kind,
            definition.base_kind,
            definition.compact_kind,
            definition.category,
            definition.family_owner_kind,
            definition.family_member_index,
            definition.family_member_count,
            definition.resource_name.to_string_lossy(),
            definition.agent_name.to_string_lossy(),
        ));
    }
    for definition in &staged {
        crate::block_grid::note_registered_base_kind(definition.base_kind);
        #[cfg(feature = "css_slot")]
        crate::owner_effects::note_registered_base_kind(definition.base_kind);
    }
    existing.extend(staged);
    RESULT_OK
}

fn item_identity(resource: &str) -> String {
    format!("item_kind_{resource}")
}

fn allocate_item_kind(existing: &[ItemCloneDefinition], chosen: &[i32], resource: &str) -> Option<i32> {
    let identity = item_identity(resource);
    let taken = |kind: i32| chosen.contains(&kind) || existing.iter().any(|known| known.public_kind == kind);
    if let Some(kind) = crate::kind_ledger::reserved_kind(&identity) {
        if kind >= FIRST_SPARSE_ITEM_KIND && !taken(kind) {
            return Some(kind);
        }
    }
    let limit = FIRST_SPARSE_ITEM_KIND + crate::item_params::MAX_CLONE_KINDS as i32;
    let kind = (FIRST_SPARSE_ITEM_KIND..limit).find(|candidate| !taken(*candidate))?;
    crate::kind_ledger::record(&identity, kind);
    Some(kind)
}

pub(crate) fn item_kind_for_hash(hash: i64) -> Option<i32> {
    let definitions = definitions().read().ok()?;
    definitions
        .iter()
        .find(|known| {
            known
                .resource_name
                .to_str()
                .map_or(false, |name| dionysus_core::menu::call_hash(name) == hash)
        })
        .map(|known| known.public_kind)
}

pub(crate) fn item_kind_for_identity(resource: &str) -> Option<i32> {
    let definitions = definitions().read().ok()?;
    definitions
        .iter()
        .find(|known| known.resource_name.to_str().ok() == Some(resource))
        .map(|known| known.public_kind)
}

unsafe fn make_item_definition(
    public_kind: i32,
    base_kind: i32,
    resource_ptr: *const c_char,
    agent_ptr: *const c_char,
    category: ItemCategory,
    family_owner_kind: i32,
    family_member_index: u32,
    family_member_count: u32,
) -> Result<ItemCloneDefinition, i32> {
    if public_kind < FIRST_SPARSE_ITEM_KIND {
        return Err(ERROR_CUSTOM_KIND);
    }
    if !(0..NATIVE_ITEM_COUNT as i32).contains(&base_kind) {
        return Err(ERROR_BASE_KIND);
    }
    if resource_ptr.is_null() || agent_ptr.is_null() {
        return Err(ERROR_NAME);
    }
    if crate::item_status_tables::ITEM_STATUS_AGENTS[base_kind as usize].is_none() {
        return Err(ERROR_ITEM_AGENT_UNAVAILABLE);
    }
    let resource_text = CStr::from_ptr(resource_ptr)
        .to_str()
        .map_err(|_| ERROR_NAME)?;
    let agent_text = CStr::from_ptr(agent_ptr).to_str().map_err(|_| ERROR_NAME)?;
    if !valid_registration_name(resource_text) || !valid_registration_name(agent_text) {
        return Err(ERROR_NAME);
    }
    let base_name = base_resource_name(base_kind)
        .and_then(|name| name.to_str().ok())
        .ok_or(ERROR_BASE_KIND)?;
    Ok(ItemCloneDefinition {
        public_kind,
        base_kind,
        compact_kind: -1,
        resource_name: CString::new(resource_text).map_err(|_| ERROR_NAME)?,
        agent_name: CString::new(agent_text).map_err(|_| ERROR_NAME)?,
        custom_agent_hash: hash40(agent_text),
        base_agent_hash: hash40(base_name),
        category,
        family_owner_kind,
        family_member_index,
        family_member_count,
    })
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_register_item_v1(
    registration: *const CloneItemRegistrationV1,
) -> i32 {
    if registration.is_null() {
        return ERROR_NULL;
    }
    if REGISTRATION_CLOSED.load(Ordering::Acquire) {
        return ERROR_REGISTRATION_CLOSED;
    }
    let registration = &*registration;
    if registration.api_version != API_VERSION_V1 {
        return ERROR_VERSION;
    }
    if registration.struct_size < core::mem::size_of::<CloneItemRegistrationV1>() as u32 {
        return ERROR_STRUCT_SIZE;
    }
    if registration.flags != 0
        || registration.reserved_u32 != 0
        || registration.reserved.iter().any(|value| *value != 0)
    {
        return ERROR_UNSUPPORTED;
    }
    if registration.base_item_kind == UNSUPPORTED_AGENT_BASE_KIND {
        return ERROR_UNSUPPORTED;
    }
    let category = item_category_for_base(registration.base_item_kind);
    if category != ItemCategory::Item {
        return ERROR_ITEM_CATEGORY;
    }
    let mut item_kind = registration.item_kind;
    if item_kind == KIND_AUTO {
        let Ok(existing) = definitions().read() else {
            return ERROR_UNSUPPORTED;
        };
        let Ok(resource) = CStr::from_ptr(registration.resource_name).to_str() else {
            return ERROR_NAME;
        };
        let Some(allocated) = allocate_item_kind(&existing, &[], resource) else {
            return ERROR_ITEM_FAMILY_CAPACITY;
        };
        item_kind = allocated;
    }
    let definition = match make_item_definition(
        item_kind,
        registration.base_item_kind,
        registration.resource_name,
        registration.agent_name,
        category,
        item_kind,
        0,
        1,
    ) {
        Ok(definition) => definition,
        Err(error) => return error,
    };
    commit_item_definitions(vec![definition])
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_register_item_family_v2(
    registration: *const CloneItemFamilyRegistrationV2,
) -> i32 {
    {
        if registration.is_null() {
            return ERROR_NULL;
        }
        if REGISTRATION_CLOSED.load(Ordering::Acquire) {
            return ERROR_REGISTRATION_CLOSED;
        }
        let registration = &*registration;
        if registration.api_version != API_VERSION_V2 {
            return ERROR_VERSION;
        }
        if registration.struct_size < core::mem::size_of::<CloneItemFamilyRegistrationV2>() as u32
            || registration.member_struct_size
                < core::mem::size_of::<CloneItemFamilyMemberV2>() as u32
        {
            return ERROR_STRUCT_SIZE;
        }
        if registration.flags != 0 || registration.reserved.iter().any(|value| *value != 0) {
            return ERROR_UNSUPPORTED;
        }
        if registration.member_count == 0 {
            return ERROR_ITEM_FAMILY_EMPTY;
        }
        if registration.members.is_null() {
            return ERROR_NULL;
        }
        if registration.member_count as usize > crate::item_params::MAX_CLONE_KINDS {
            return ERROR_ITEM_FAMILY_CAPACITY;
        }
        if !(0..NATIVE_ITEM_COUNT as i32).contains(&registration.base_owner_kind) {
            return ERROR_BASE_KIND;
        }
        if registration.base_owner_kind == UNSUPPORTED_AGENT_BASE_KIND {
            return ERROR_ITEM_AGENT_UNAVAILABLE;
        }
        let Some(owner_meta) = descriptor_meta(registration.base_owner_kind) else {
            return ERROR_BASE_KIND;
        };
        let category = category_from_descriptor(owner_meta);
        if !owner_class_matches_category(owner_meta, category) {
            return ERROR_ITEM_CATEGORY;
        }
        let Some(expected_count) = native_family_len(registration.base_owner_kind) else {
            return ERROR_ITEM_FAMILY_LAYOUT;
        };
        if expected_count != registration.member_count as usize {
            return ERROR_ITEM_FAMILY_LAYOUT;
        }
        let stride = registration.member_struct_size as usize;
        let Some(_) = stride.checked_mul(registration.member_count as usize) else {
            return ERROR_STRUCT_SIZE;
        };
        let mut kinds: Vec<i32> = Vec::with_capacity(expected_count);
        {
            let Ok(existing) = definitions().read() else {
                return ERROR_UNSUPPORTED;
            };
            for index in 0..expected_count {
                let member_ptr = (registration.members as *const u8).add(index * stride)
                    as *const CloneItemFamilyMemberV2;
                let member = core::ptr::read_unaligned(member_ptr);
                if member.item_kind != KIND_AUTO {
                    kinds.push(member.item_kind);
                    continue;
                }
                if member.resource_name.is_null() {
                    return ERROR_NAME;
                }
                let Ok(resource) = CStr::from_ptr(member.resource_name).to_str() else {
                    return ERROR_NAME;
                };
                let Some(allocated) = allocate_item_kind(&existing, &kinds, resource) else {
                    return ERROR_ITEM_FAMILY_CAPACITY;
                };
                kinds.push(allocated);
            }
        }
        let owner_public_kind = kinds[0];
        let mut staged = Vec::with_capacity(expected_count);
        for index in 0..expected_count {
            let member_ptr = (registration.members as *const u8).add(index * stride)
                as *const CloneItemFamilyMemberV2;
            let member = core::ptr::read_unaligned(member_ptr);
            if member.flags != 0 || member.reserved.iter().any(|value| *value != 0) {
                return ERROR_UNSUPPORTED;
            }
            let base_kind = registration.base_owner_kind + index as i32;
            let Some(meta) = descriptor_meta(base_kind) else {
                return ERROR_ITEM_FAMILY_LAYOUT;
            };
            if (index == 0 && meta.class == 13)
                || (index != 0 && meta.class != 13)
                || category_from_descriptor(meta) != category
            {
                return ERROR_ITEM_FAMILY_LAYOUT;
            }
            let definition = match make_item_definition(
                kinds[index],
                base_kind,
                member.resource_name,
                member.agent_name,
                category,
                owner_public_kind,
                index as u32,
                expected_count as u32,
            ) {
                Ok(definition) => definition,
                Err(error) => return error,
            };
            staged.push(definition);
        }
        limited_log(format!(
        "[itemclone] family-v2 owner_base={:#x} owner_public={owner_public_kind:#x} category={category:?} members={expected_count} variants={} native_family={}",
        registration.base_owner_kind, owner_meta.variants, owner_meta.family
    ));
        commit_item_definitions(staged)
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_base_kind(item_kind: i32) -> i32 {
    definition(item_kind)
        .map(|definition| definition.base_kind)
        .unwrap_or(item_kind)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_item_kind_for_identity_v1(
    resource_name: *const c_char,
) -> i32 {
    if resource_name.is_null() {
        return -1;
    }
    let Ok(resource) = CStr::from_ptr(resource_name).to_str() else {
        return -1;
    };
    item_kind_for_identity(resource).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn clone_engine_is_item_kind(item_kind: i32) -> bool {
    definition(item_kind).is_some()
}

#[no_mangle]
pub extern "C" fn clone_engine_item_resource_name(item_kind: i32) -> *const c_char {
    definitions()
        .read()
        .ok()
        .and_then(|definitions| {
            definitions
                .iter()
                .find(|definition| definition.public_kind == item_kind)
                .map(|definition| definition.resource_name.as_ptr())
        })
        .unwrap_or(core::ptr::null())
}

#[no_mangle]
pub extern "C" fn clone_engine_item_category(item_kind: i32) -> u32 {
    item_category_for_public(item_kind) as u32
}

#[no_mangle]
pub extern "C" fn clone_engine_item_family_owner(item_kind: i32) -> i32 {
    definition(item_kind)
        .map(|definition| definition.family_owner_kind)
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn clone_engine_item_family_member_index(item_kind: i32) -> i32 {
    definition(item_kind)
        .and_then(|definition| i32::try_from(definition.family_member_index).ok())
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn clone_engine_item_kind_from_object(object: *const libc::c_void) -> i32 {
    live_kind_by_object(object as usize)
        .or_else(|| active_public_kind())
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn clone_engine_item_kind_from_boma(boma: *const libc::c_void) -> i32 {
    live_kind_by_boma(boma as usize)
        .or_else(|| {
            active_pending().and_then(|index| {
                (PENDING[index].boma.load(Ordering::Acquire) == boma as usize)
                    .then(|| PENDING[index].public_kind.load(Ordering::Acquire))
            })
        })
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn clone_engine_item_spawn_source(object: *const libc::c_void) -> u32 {
    live_slot_by_object(object as usize)
        .map(|slot| slot.spawn_source.load(Ordering::Acquire))
        .unwrap_or(ItemSpawnSource::Unknown as u32)
}

#[no_mangle]
pub extern "C" fn clone_engine_item_parent_kind(object: *const libc::c_void) -> i32 {
    live_slot_by_object(object as usize)
        .map(|slot| slot.parent_public_kind.load(Ordering::Acquire))
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn clone_engine_item_status_v1(
    item_kind: i32,
    line: i32,
    status_kind: i32,
    function: usize,
) -> i32 {
    let Some(item) = definition(item_kind) else {
        return ERROR_CUSTOM_KIND;
    };
    if function == 0 || status_kind < 0 {
        return ERROR_UNSUPPORTED;
    }
    if status_kind == 0 {
        log(format!(
            "[itemclone] item_status refused public={item_kind:#x}: status kind 0 is never valid \
             (item statuses are numbered from 1). This is what an unresolved name looks like - \
             register by name instead"
        ));
        return ERROR_UNSUPPORTED;
    }
    let line_count = crate::item_status_tables::ITEM_STATUS_LINES.len() as i32;
    if !(0..line_count).contains(&line) {
        log(format!(
            "[itemclone] item_status refused public={item_kind:#x} line={line}: item agents have \
             {line_count} lines (setting/joint_srt/init/update/coroutine/exit), not the fighter set"
        ));
        return ERROR_UNSUPPORTED;
    }
    register_status_script(item_kind, line, StatusSelector::Kind(status_kind), function)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_item_status_named_v1(
    item_kind: i32,
    line: i32,
    status_name: *const c_char,
    function: usize,
) -> i32 {
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    if function == 0 || status_name.is_null() {
        return ERROR_UNSUPPORTED;
    }
    let Ok(name) = CStr::from_ptr(status_name).to_str() else {
        return ERROR_NAME;
    };
    let line_count = crate::item_status_tables::ITEM_STATUS_LINES.len() as i32;
    if !(0..line_count).contains(&line) {
        log(format!(
            "[itemclone] item_status refused public={item_kind:#x} line={line}: item agents have \
             {line_count} lines (setting/joint_srt/init/update/coroutine/exit), not the fighter set"
        ));
        return ERROR_UNSUPPORTED;
    }
    if crate::item_status_tables::ITEM_STATUS_KINDS
        .binary_search_by(|(known, _)| (*known).cmp(name))
        .is_err()
    {
        log(format!(
            "[itemclone] item_status refused public={item_kind:#x}: status name \"{name}\" is not \
             one of the {} this engine knows",
            crate::item_status_tables::ITEM_STATUS_KINDS.len()
        ));
        return ERROR_NAME;
    }
    register_status_script(
        item_kind,
        line,
        StatusSelector::Name(name.to_string()),
        function,
    )
}

#[no_mangle]
pub extern "C" fn clone_engine_item_status_kind(name: *const c_char) -> i32 {
    if name.is_null() {
        return ERROR_UNSUPPORTED;
    }
    let Ok(name) = (unsafe { CStr::from_ptr(name) }).to_str() else {
        return ERROR_UNSUPPORTED;
    };
    match item_status_kind_by_name(name) {
        Some(kind) => kind,
        None => {
            log(format!(
                "[itemclone] item_status_kind({name}) unresolved; either the name is not in the \
                 713 known constants or lua2cpp_item is not loaded yet"
            ));
            ERROR_UNSUPPORTED
        }
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_common_set(item_kind: i32, field: u64, value: f32) -> i32 {
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    let Some(offset) = crate::item_params::common_field_offset(field) else {
        log(format!(
            "[itemclone] item_common_set refused public={item_kind:#x} field={field:#x}: \
             not one of the 161 measured float fields"
        ));
        return ERROR_UNSUPPORTED;
    };
    match crate::item_params::register_common_override(item_kind, offset, value) {
        true => {
            log(format!(
                "[itemclone] item_common_set public={item_kind:#x} field={field:#x} -> +{offset:#x} = {value}"
            ));
            RESULT_OK
        }
        false => ERROR_BACKEND_UNAVAILABLE,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_owner_param_set_f32(
    item_kind: i32,
    owner_kind: i32,
    offset: u32,
    value: f32,
) -> i32 {
    clone_engine_item_owner_param_set_bits(item_kind, owner_kind, offset, value.to_bits())
}

#[no_mangle]
pub extern "C" fn clone_engine_item_owner_param_set_i32(
    item_kind: i32,
    owner_kind: i32,
    offset: u32,
    value: i32,
) -> i32 {
    clone_engine_item_owner_param_set_bits(item_kind, owner_kind, offset, value as u32)
}

fn clone_engine_item_owner_param_set_bits(
    item_kind: i32,
    owner_kind: i32,
    offset: u32,
    bits: u32,
) -> i32 {
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    if !(0..crate::fighter_params::PARAM_NATIVE_KINDS).contains(&owner_kind) {
        log(format!(
            "[itemclone] item_owner_param_set refused public={item_kind:#x} owner={owner_kind}:              the owner must be one of the {} native fighter kinds",
            crate::fighter_params::PARAM_NATIVE_KINDS
        ));
        return ERROR_UNSUPPORTED;
    }
    if !crate::item_params::owner_offset_valid(offset) {
        log(format!(
            "[itemclone] item_owner_param_set refused public={item_kind:#x} owner={owner_kind}              +{offset:#x}: the offset must be 4-byte aligned and below {:#x}",
            crate::item_params::OWNER_PAYLOAD_BYTES
        ));
        return ERROR_UNSUPPORTED;
    }
    match crate::item_params::register_owner_override(item_kind, owner_kind, offset, bits) {
        true => {
            log(format!(
                "[itemclone] item_owner_param_set public={item_kind:#x} owner={owner_kind} \
                 +{offset:#x} = {bits:#x}: {} ({} override(s) for this item)",
                dionysus_core::owner_param_words::describe(owner_kind, offset, bits),
                crate::item_params::owner_override_count(item_kind)
            ));
            RESULT_OK
        }
        false => ERROR_BACKEND_UNAVAILABLE,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_common_param(item_kind: i32, field: u64, tag: u32, bits: u64) -> i32 {
    use dionysus_core::item_common_row::{self as row, WordKind};
    use dionysus_core::row_fields::{TAG_BOOL, TAG_F32, TAG_HASH, TAG_I32, TAG_LABEL, TAG_MUL};
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    let float = crate::item_params::common_field_offset(field);
    let int_word = row::word(field).filter(|word| word.kind == WordKind::Int);
    match tag {
        TAG_F32 => match (float, int_word) {
            (Some(_), _) => clone_engine_item_common_set(item_kind, field, f32::from_bits(bits as u32)),
            (None, Some(_)) => {
                clone_engine_item_common_set_i32(item_kind, field, f32::from_bits(bits as u32).round() as i32)
            }
            _ => common_word_refusal(field, row::Reject::UnknownField),
        },
        TAG_I32 | TAG_BOOL => match float {
            Some(_) => clone_engine_item_common_set(item_kind, field, bits as u32 as i32 as f32),
            None => clone_engine_item_common_set_i32(item_kind, field, bits as u32 as i32),
        },
        TAG_LABEL => clone_engine_item_common_set_label(item_kind, field, bits),
        TAG_HASH => clone_engine_item_common_set_hash(item_kind, field, bits),
        TAG_MUL => {
            let factor = f32::from_bits(bits as u32);
            let target = match (float, int_word) {
                (Some(offset), _) => Some((crate::item_params::CommonTable::Float, offset)),
                (None, Some(word)) => Some((crate::item_params::CommonTable::Word, word.offset)),
                _ => None,
            };
            let Some((table, offset)) = target else {
                log(format!(
                    "[itemclone] item_common mul refused public={item_kind:#x} field={field:#x}: only number fields scale"
                ));
                return ERROR_UNSUPPORTED;
            };
            match crate::item_params::register_common_mul(item_kind, table, offset, factor) {
                true => {
                    log(format!(
                        "[itemclone] item_common public={item_kind:#x} field={field:#x} -> +{offset:#x} times {factor}"
                    ));
                    RESULT_OK
                }
                false => ERROR_BACKEND_UNAVAILABLE,
            }
        }
        _ => ERROR_UNSUPPORTED,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_owner_param_mul(item_kind: i32, owner_kind: i32, offset: u32, factor: f32) -> i32 {
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    if !(0..crate::fighter_params::PARAM_NATIVE_KINDS).contains(&owner_kind)
        || !crate::item_params::owner_offset_valid(offset)
    {
        return ERROR_UNSUPPORTED;
    }
    match crate::item_params::register_owner_mul(item_kind, owner_kind, offset, factor) {
        true => {
            log(format!(
                "[itemclone] item_owner_param public={item_kind:#x} owner={owner_kind} +{offset:#x} times {factor} ({} override(s) for this item)",
                crate::item_params::owner_override_count(item_kind)
            ));
            RESULT_OK
        }
        false => ERROR_UNSUPPORTED,
    }
}

fn common_word_refusal(field: u64, reject: dionysus_core::item_common_row::Reject) -> i32 {
    use dionysus_core::item_common_row::Reject;
    let why = match reject {
        Reject::UnknownField => "not one of the 43 word fields (bools, ints, kinds) of the common row",
        Reject::NotABool => "a bool takes 0 or 1",
        Reject::NotAWord => "a label only fits a kind field (have_kind, size_kind, hit_kind, ...)",
        Reject::NotAHash => "a hash40 field takes a 40-bit hash",
        Reject::UnknownLabel => "the label is not one the game's item resolver knows",
    };
    log(format!("[itemclone] item_common_set refused field={field:#x}: {why}"));
    ERROR_UNSUPPORTED
}

fn register_common_word(item_kind: i32, field: u64, name: &str, offset: u32, bits: u64, shown: String) -> i32 {
    match crate::item_params::register_common_bits(
        item_kind,
        crate::item_params::CommonTable::Word,
        offset,
        bits,
    ) {
        true => {
            log(format!(
                "[itemclone] item_common_set public={item_kind:#x} {name} ({field:#x}) -> word +{offset:#x} = {shown} ({} override(s) for this item)",
                crate::item_params::common_override_count(item_kind)
            ));
            RESULT_OK
        }
        false => ERROR_BACKEND_UNAVAILABLE,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_common_set_i32(item_kind: i32, field: u64, value: i32) -> i32 {
    use dionysus_core::item_common_row as row;
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    match row::resolve_i32(field, value) {
        Ok((offset, bits)) => {
            let name = row::word(field).map(|word| word.name).unwrap_or("?");
            register_common_word(item_kind, field, name, offset, u64::from(bits), value.to_string())
        }
        Err(reject) => common_word_refusal(field, reject),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_common_set_label(item_kind: i32, field: u64, label: u64) -> i32 {
    use dionysus_core::item_common_row as row;
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    match row::resolve_label(field, label) {
        Ok((offset, bits)) => {
            let name = row::word(field).map(|word| word.name).unwrap_or("?");
            register_common_word(item_kind, field, name, offset, u64::from(bits), format!("{bits} (label {label:#x})"))
        }
        Err(reject) => common_word_refusal(field, reject),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_common_set_hash(item_kind: i32, field: u64, value: u64) -> i32 {
    use dionysus_core::item_common_row as row;
    if definition(item_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    match row::resolve_hash(field, value) {
        Ok((offset, bits)) => {
            let name = row::hash_field(field).map(|f| f.name).unwrap_or("?");
            match crate::item_params::register_common_bits(
                item_kind,
                crate::item_params::CommonTable::Hash,
                offset,
                bits,
            ) {
                true => {
                    log(format!(
                        "[itemclone] item_common_set public={item_kind:#x} {name} ({field:#x}) -> hash +{offset:#x} = {bits:#x} ({} override(s) for this item)",
                        crate::item_params::common_override_count(item_kind)
                    ));
                    RESULT_OK
                }
                false => ERROR_BACKEND_UNAVAILABLE,
            }
        }
        Err(reject) => common_word_refusal(field, reject),
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_item_common_has(field: u64) -> i32 {
    (crate::item_params::common_field_offset(field).is_some()
        || dionysus_core::item_common_row::describe(field).is_some()) as i32
}

#[no_mangle]
pub extern "C" fn clone_engine_item_backend_status() -> u32 {
    BACKEND_STATUS.load(Ordering::Acquire)
}

pub(crate) fn mark_resource_router_ready() {
    BACKEND_STATUS.fetch_or(STATUS_RESOURCE_ROUTER_READY, Ordering::AcqRel);
}

pub(crate) fn mark_param_router_ready() {
    BACKEND_STATUS.fetch_or(STATUS_PARAM_ROUTER_READY, Ordering::AcqRel);
}

pub(crate) fn mark_category_router_ready() {
    BACKEND_STATUS.fetch_or(STATUS_CATEGORY_ROUTER_READY, Ordering::AcqRel);
    BACKEND_STATUS.fetch_or(STATUS_FAMILY_ROUTER_READY, Ordering::AcqRel);
}

pub(crate) fn mark_training_ui_ready() {
    BACKEND_STATUS.fetch_or(STATUS_TRAINING_UI_READY, Ordering::AcqRel);
}

#[cfg(feature = "item_selftest")]
mod selftest {
    use super::*;

    struct SelfTestClone {
        public_kind: i32,
        base_kind: i32,
        base_name: &'static str,
        resource_name: &'static [u8],
        agent_name: &'static [u8],
        spawns: AtomicU32,
        armed: AtomicBool,
    }

    static CLONES: [SelfTestClone; 2] = [
        SelfTestClone {
            public_kind: FIRST_SPARSE_ITEM_KIND,
            base_kind: 63,
            base_name: "killsword",
            resource_name: b"wawa\0",
            agent_name: b"wawa\0",
            spawns: AtomicU32::new(0),
            armed: AtomicBool::new(false),
        },
        SelfTestClone {
            public_kind: FIRST_SPARSE_ITEM_KIND + 1,
            base_kind: 64,
            base_name: "deathscythe",
            resource_name: b"bonk\0",
            agent_name: b"bonk\0",
            spawns: AtomicU32::new(0),
            armed: AtomicBool::new(false),
        },
    ];

    pub(super) fn register() {
        for clone in &CLONES {
            let name = clone.resource_name.as_ptr() as *const c_char;
            let agent = clone.agent_name.as_ptr() as *const c_char;
            let registration = CloneItemRegistrationV1 {
                api_version: API_VERSION_V1,
                struct_size: core::mem::size_of::<CloneItemRegistrationV1>() as u32,
                item_kind: clone.public_kind,
                base_item_kind: clone.base_kind,
                resource_name: name,
                agent_name: agent,
                flags: 0,
                reserved_u32: 0,
                reserved: [0; 4],
            };
            let result = unsafe { super::clone_engine_register_item_v1(&registration) };
            clone.armed.store(result == RESULT_OK, Ordering::Release);
            log(format!(
                "[itemtest] register public={:#x} base={:#x} ({}) resource={} result={result} status={:#010x}",
                clone.public_kind,
                clone.base_kind,
                clone.base_name,
                String::from_utf8_lossy(&clone.resource_name[..clone.resource_name.len() - 1]),
                BACKEND_STATUS.load(Ordering::Acquire)
            ));
        }
    }

    pub(super) fn redirect_spawn(raw_kind: i32) -> i32 {
        #[cfg(feature = "item_ui_backend")]
        return raw_kind;

        #[cfg(not(feature = "item_ui_backend"))]
        {
            let Some(clone) = CLONES
                .iter()
                .find(|clone| clone.base_kind == raw_kind && clone.armed.load(Ordering::Acquire))
            else {
                return raw_kind;
            };
            if has_armed_request(raw_kind) {
                return raw_kind;
            }
            let sequence = clone.spawns.fetch_add(1, Ordering::Relaxed);
            let redirected = sequence % 2 == 1;
            limited_log(format!(
                "[itemtest] {} spawn #{sequence} -> {}",
                clone.base_name,
                if redirected { "clone" } else { "vanilla" }
            ));
            if redirected {
                clone.public_kind
            } else {
                raw_kind
            }
        }
    }
}

pub fn install() {
    unsafe {
        if let Err((address, found, expected)) = main_preflight() {
            log(format!(
                "[itemclone] main preflight failed at {address:#x}: found={found:#010x} expected={expected:#010x}; backend inert"
            ));
            return;
        }
        BACKEND_STATUS.fetch_or(STATUS_MAIN_PREFLIGHT_OK, Ordering::AcqRel);
        skyline::install_hooks!(
            item_request_full_bridge,
            item_request_simple_bridge,
            item_have_item_bridge,
            item_born_item_bridge,
            item_attach_item_bridge,
            item_lower_creator_bridge,
            item_object_adopt,
            item_deactivate_bridge,
            battle_object_update
        );
        BACKEND_STATUS.fetch_or(STATUS_IDENTITY_HOOKS_READY, Ordering::AcqRel);
        if install_status_scope() {
            log("[itemclone] clone items run every status change as themselves".to_string());
        } else {
            log("[itemclone] the item status table holds other entries; a status change made by another object reads the base item's params".to_string());
        }
    }
    match skyline::nro::add_hook(item_nro_hook) {
        Ok(()) => {
            BACKEND_STATUS.fetch_or(STATUS_READY, Ordering::AcqRel);
            log("[itemclone] sparse sidecar backend armed; native objects retain their base ItemKind"
                .to_string());
        }
        Err(_) => log(
            "[itemclone] identity hooks ready, but lua2cpp_item routing is unavailable".to_string(),
        ),
    }
    #[cfg(feature = "item_selftest")]
    selftest::register();
    #[cfg(feature = "diag_item_work")]
    crate::item_work_probe::install();
}
