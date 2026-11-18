use super::*;

use dionysus_core::param_swap::{BaseState, CloneState};
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

const PARAM_SINGLETON: usize = 0x52bb3b0;
const PARAM_RECORD_BASE: usize = 0x60;
const PARAM_RECORD_STRIDE: usize = 0x38;
const PARAM_RECORD_REFCOUNT: usize = 0x0;
const PARAM_RECORD_PAYLOAD: usize = 0x10;
const PARAM_RECORD_OWNER: usize = 0x18;
const PARAM_LOADER: usize = 0x70c580;
const PARAM_LOADER_B: usize = 0x721ca0;
const PARAM_RECORD_PAYLOAD_B: usize = 0x28;
const PARAM_RECORD_OWNER_B: usize = 0x30;
const PARAM_MUTEX: usize = 0x1988;
const PARAM_MUTEX_LOCK: usize = 0x39c1490;
const PARAM_MUTEX_UNLOCK: usize = 0x39c14a0;
const PARAM_FILE_LOADER: usize = 0x5331f20;
const PARAM_FILE_REQUEST: usize = 0x3540450;
const PARAM_FILE_RELEASE: usize = 0x3540560;
const PARAM_PATH_TYPE_A: i32 = 12;
const PARAM_PATH_TYPE_B: i32 = 13;

pub(crate) use dionysus_core::param_swap::{GuardMode, Instance, LockOutcome, SwapRegistry};
pub(crate) use dionysus_core::slots::{CLONE_SLOTS, PARAM_NATIVE_KINDS};

type ParamLoader = unsafe extern "C" fn(usize, i32, *const i32);

static REGISTRY: SwapRegistry = SwapRegistry::new();

static SWAP_BUSY_LOG: AtomicU32 = AtomicU32::new(0);
const SWAP_SPIN_LIMIT: u32 = 100_000;

pub(crate) static PARAM_INSTANCE_LOG: AtomicU32 = AtomicU32::new(0);

const EMPTY_INSTANCE: Instance = Instance::EMPTY;

pub(crate) struct SwapGuard {
    record: usize,
    saved: Instance,
    kind: i32,
    base_kind: i32,
    mode: GuardMode,
    holds_refcount: bool,
}

impl SwapGuard {
    const fn idle() -> Self {
        Self {
            record: 0,
            saved: EMPTY_INSTANCE,
            kind: -1,
            base_kind: -1,
            mode: GuardMode::Idle,
            holds_refcount: false,
        }
    }
}

static SKIP_LOG: AtomicU32 = AtomicU32::new(0);

fn skip_log(message: &str) {
    let n = SKIP_LOG.fetch_add(1, Ordering::Relaxed);
    if n < 32 {
        dbg_log_public(&format!("[cloneparam] #{n} {message}"));
    }
}

fn base_state(base_kind: i32) -> Option<&'static BaseState> {
    REGISTRY.base(base_kind)
}

fn clone_state(clone_kind: i32) -> Option<&'static CloneState> {
    REGISTRY.clone_slot(clone_kind)
}

pub(crate) fn clone_slot_peak() -> (i32, u32) {
    REGISTRY.peak()
}

unsafe fn singleton() -> Option<usize> {
    let value = core::ptr::read_volatile((crate::game::addr(PARAM_SINGLETON)) as *const usize);
    (value != 0 && value & 7 == 0).then_some(value)
}

pub(crate) unsafe fn param_singleton() -> Option<usize> {
    singleton()
}

unsafe fn record_of(base_kind: i32) -> Option<usize> {
    if !(0..PARAM_NATIVE_KINDS).contains(&base_kind) {
        return None;
    }
    let singleton = singleton()?;
    Some(singleton + PARAM_RECORD_BASE + base_kind as usize * PARAM_RECORD_STRIDE)
}

unsafe fn read_pair(record: usize) -> (usize, usize) {
    (
        core::ptr::read_volatile((record + PARAM_RECORD_PAYLOAD) as *const usize),
        core::ptr::read_volatile((record + PARAM_RECORD_OWNER) as *const usize),
    )
}

fn lock_swaps() -> bool {
    let thread = unsafe { current_thread_key() };
    let outcome = REGISTRY.try_lock(thread, SWAP_SPIN_LIMIT);
    if outcome == LockOutcome::Contended && SWAP_BUSY_LOG.fetch_add(1, Ordering::Relaxed) < 4 {
        dbg_log!(
            "[cloneparam] swap lock held elsewhere for the whole spin budget; proceeding unlocked rather than blocking the load"
        );
    }
    outcome.owned()
}

fn unlock_swaps(owned: bool) {
    REGISTRY.unlock(owned);
}

pub(crate) fn swap_lock() -> bool {
    lock_swaps()
}

pub(crate) fn swap_unlock(owned: bool) {
    unlock_swaps(owned);
}

fn clones_of(base_kind: i32) -> Vec<i32> {
    clone_definitions()
        .read()
        .map(|held| {
            held.iter()
                .filter(|definition| definition.base_kind == base_kind)
                .map(|definition| definition.kind)
                .collect()
        })
        .unwrap_or_default()
}

fn known_payload(_state: &BaseState, base_kind: i32, payload: usize) -> bool {
    REGISTRY.known_payload(base_kind, payload, &clones_of(base_kind))
}

fn forget_all(_state: &BaseState, base_kind: i32) {
    REGISTRY.forget_all(base_kind, &clones_of(base_kind));
}

unsafe fn revalidate(base_kind: i32, record: usize) -> Option<&'static BaseState> {
    let state = base_state(base_kind)?;
    let (payload, _) = read_pair(record);
    if payload == 0 {
        forget_all(state, base_kind);
        return None;
    }
    if !known_payload(state, base_kind, payload) {
        forget_all(state, base_kind);
        state.origin.store(payload, Ordering::Relaxed);
    }
    Some(state)
}

unsafe fn read_refcount(record: usize) -> u32 {
    core::ptr::read_volatile((record + PARAM_RECORD_REFCOUNT) as *const u32)
}

unsafe fn write_refcount(record: usize, value: u32) {
    core::ptr::write_volatile((record + PARAM_RECORD_REFCOUNT) as *mut u32, value);
}

unsafe fn hold_unconstructed_record(record: usize) -> bool {
    let Some(singleton) = singleton() else {
        return false;
    };
    lock_param_singleton(singleton);
    let unconstructed = read_refcount(record) == 0;
    if unconstructed {
        write_refcount(record, 1);
    }
    unlock_param_singleton(singleton);
    unconstructed
}

unsafe fn release_record_hold(record: usize) {
    let Some(singleton) = singleton() else {
        return;
    };
    lock_param_singleton(singleton);
    write_refcount(record, read_refcount(record).saturating_sub(1));
    unlock_param_singleton(singleton);
}

unsafe fn read_instance(record: usize) -> Instance {
    Instance {
        payload: core::ptr::read_volatile((record + PARAM_RECORD_PAYLOAD) as *const usize),
        owner: core::ptr::read_volatile((record + PARAM_RECORD_OWNER) as *const usize),
        payload_b: core::ptr::read_volatile((record + PARAM_RECORD_PAYLOAD_B) as *const usize),
        owner_b: core::ptr::read_volatile((record + PARAM_RECORD_OWNER_B) as *const usize),
    }
}

unsafe fn write_instance(record: usize, instance: Instance) {
    core::ptr::write_volatile((record + PARAM_RECORD_PAYLOAD) as *mut usize, instance.payload);
    core::ptr::write_volatile((record + PARAM_RECORD_OWNER) as *mut usize, instance.owner);
    core::ptr::write_volatile(
        (record + PARAM_RECORD_PAYLOAD_B) as *mut usize,
        instance.payload_b,
    );
    core::ptr::write_volatile((record + PARAM_RECORD_OWNER_B) as *mut usize, instance.owner_b);
}

unsafe fn lock_param_singleton(singleton: usize) {
    type Mutex = unsafe extern "C" fn(usize);
    let lock: Mutex = core::mem::transmute(crate::game::addr(PARAM_MUTEX_LOCK));
    lock(singleton + PARAM_MUTEX);
}

unsafe fn unlock_param_singleton(singleton: usize) {
    type Mutex = unsafe extern "C" fn(usize);
    let unlock: Mutex = core::mem::transmute(crate::game::addr(PARAM_MUTEX_UNLOCK));
    unlock(singleton + PARAM_MUTEX);
}

pub(crate) unsafe fn request_resource_file(index: i32) {
    if index < 0 || index == RESOURCE_INDEX_NOT_FOUND {
        return;
    }
    let loader = core::ptr::read_volatile((crate::game::addr(PARAM_FILE_LOADER)) as *const usize);
    if loader == 0 {
        return;
    }
    type Request = unsafe extern "C" fn(usize, i32);
    let request: Request = core::mem::transmute(crate::game::addr(PARAM_FILE_REQUEST));
    request(loader, index);
}

pub(crate) unsafe fn release_resource_file(index: i32) {
    if index < 0 || index == RESOURCE_INDEX_NOT_FOUND {
        return;
    }
    let loader = core::ptr::read_volatile((crate::game::addr(PARAM_FILE_LOADER)) as *const usize);
    if loader == 0 {
        return;
    }
    type Release = unsafe extern "C" fn(usize, i32);
    let release: Release = core::mem::transmute(crate::game::addr(PARAM_FILE_RELEASE));
    release(loader, index);
}

const SEARCH_PATH_TO_FILE_PATH: usize = 0x353e4e0;

unsafe fn file_path_index(search: i32) -> Option<i32> {
    type Convert = unsafe extern "C" fn(u32) -> u32;
    let convert: Convert = core::mem::transmute(crate::game::addr(SEARCH_PATH_TO_FILE_PATH));
    let value = convert(search as u32) as i32;
    (value >= 0 && value != RESOURCE_INDEX_NOT_FOUND).then_some(value)
}

unsafe fn directory_indices(record: usize, base_kind: i32) -> Option<(i32, i32)> {
    let a = crate::load_pipeline::resolve_resource_directory(record, base_kind, PARAM_PATH_TYPE_A)?;
    let b = crate::load_pipeline::resolve_resource_directory(record, base_kind, PARAM_PATH_TYPE_B)?;
    let file_a = file_path_index(a);
    let file_b = file_path_index(b);
    skip_log(&format!(
        "param directories for kind {base_kind}: search ({a},{b}) -> file ({},{})",
        file_a.unwrap_or(-1),
        file_b.unwrap_or(-1)
    ));
    Some((file_a?, file_b?))
}

unsafe fn build_instance(base_kind: i32, indices: (i32, i32)) -> Option<Instance> {
    let (index_a, index_b) = indices;
    if index_a < 0 || index_a == RESOURCE_INDEX_NOT_FOUND {
        return None;
    }
    if index_b < 0 || index_b == RESOURCE_INDEX_NOT_FOUND {
        return None;
    }
    let record = record_of(base_kind)?;
    let singleton = singleton()?;

    let held_a = substitute_param_file(base_kind, index_a);
    let held_b = substitute_param_file(base_kind, index_b);
    request_resource_file(held_a);
    request_resource_file(held_b);

    lock_param_singleton(singleton);
    let saved = read_instance(record);

    let loader_a: ParamLoader = core::mem::transmute(crate::game::addr(PARAM_LOADER));
    let loader_b: ParamLoader = core::mem::transmute(crate::game::addr(PARAM_LOADER_B));
    write_instance(
        record,
        Instance {
            payload: saved.payload,
            owner: 0,
            payload_b: saved.payload_b,
            owner_b: 0,
        },
    );
    loader_a(singleton, base_kind, &held_a as *const i32);
    loader_b(singleton, base_kind, &held_b as *const i32);
    let built = read_instance(record);
    write_instance(record, saved);
    unlock_param_singleton(singleton);

    if built.payload == 0 || built.payload == saved.payload || built.payload & 7 != 0 {
        skip_log(&format!(
            "kind {base_kind} directories ({index_a},{index_b}) produced no distinct payload A"
        ));
        return None;
    }
    if built.payload_b == 0 || built.payload_b == saved.payload_b {
        skip_log(&format!(
            "kind {base_kind} directories ({index_a},{index_b}) produced no distinct payload B; a half-built record walks fine and dies on use, so discarding it"
        ));
        return None;
    }
    crate::dbg_log_public(&format!(
        "[cloneparam] built instance for kind {base_kind} directories ({index_a},{index_b}): A {:#x}/{:#x} B {:#x}/{:#x}, resident A {:#x} B {:#x}",
        built.payload, built.owner, built.payload_b, built.owner_b, saved.payload, saved.payload_b
    ));
    Some(built)
}

unsafe fn ensure_vanilla(
    state: &BaseState,
    base_kind: i32,
    entry_id: i32,
) -> Option<Instance> {
    if state.vanilla_own_context.load(Ordering::Relaxed) {
        let payload = state.vanilla_payload.load(Ordering::Relaxed);
        if payload != 0 {
            return Some(Instance {
                payload,
                owner: state.vanilla_owner.load(Ordering::Relaxed),
                payload_b: state.vanilla_payload_b.load(Ordering::Relaxed),
                owner_b: state.vanilla_owner_b.load(Ordering::Relaxed),
            });
        }
    }
    let record = crate::load_pipeline::resource_record_for_entry(entry_id)?;
    let built = build_instance(base_kind, directory_indices(record, base_kind)?)?;
    if KIND_TABLE_REQUIRED.contains(&base_kind) && kind_table_of(built.payload) == 0 {
        skip_log(&format!(
            "base {base_kind} instance built from its own entry {entry_id} has no kind table, so the resident is kept"
        ));
        return None;
    }
    state.vanilla_payload.store(built.payload, Ordering::Relaxed);
    state.vanilla_owner.store(built.owner, Ordering::Relaxed);
    state.vanilla_payload_b.store(built.payload_b, Ordering::Relaxed);
    state.vanilla_owner_b.store(built.owner_b, Ordering::Relaxed);
    state.vanilla_own_context.store(true, Ordering::Relaxed);
    skip_log(&format!(
        "base {base_kind} built its own instance A {:#x} B {:#x} from entry {entry_id} with kind table {:#x}",
        built.payload,
        built.payload_b,
        kind_table_of(built.payload)
    ));
    Some(built)
}

unsafe fn load_owned_params(kind: i32, entry_id: i32) -> bool {
    let record = match record_of(kind) {
        Some(record) => record,
        None => return false,
    };
    if read_pair(record).0 != 0 {
        return true;
    }
    let singleton = match singleton() {
        Some(singleton) => singleton,
        None => return false,
    };
    let donor = match crate::load_pipeline::resource_record_for_entry(entry_id)
        .filter(|record| *record != 0)
        .or_else(|| any_live_resource_record())
    {
        Some(donor) => donor,
        None => return false,
    };
    let (index_a, index_b) = match directory_indices(donor, kind) {
        Some(indices) => indices,
        None => return false,
    };
    if index_a < 0 || index_a == RESOURCE_INDEX_NOT_FOUND {
        return false;
    }
    if index_b < 0 || index_b == RESOURCE_INDEX_NOT_FOUND {
        return false;
    }
    let loader_a: ParamLoader = core::mem::transmute(crate::game::addr(PARAM_LOADER));
    let loader_b: ParamLoader = core::mem::transmute(crate::game::addr(PARAM_LOADER_B));
    let held_a = substitute_param_file(kind, index_a);
    let held_b = substitute_param_file(kind, index_b);
    request_resource_file(held_a);
    request_resource_file(held_b);
    lock_param_singleton(singleton);
    loader_a(singleton, kind, &held_a as *const i32);
    loader_b(singleton, kind, &held_b as *const i32);
    let loaded = read_instance(record);
    unlock_param_singleton(singleton);
    if loaded.payload == 0 || loaded.payload_b == 0 {
        return false;
    }
    let table = kind_table_of(loaded.payload);
    let refcount = core::ptr::read_volatile(record as *const u32);
    skip_log(&format!(
        "owned load kind {kind} directories ({index_a},{index_b}) payload {:#x} table {table:#x} refcount {refcount}",
        loaded.payload
    ));
    if KIND_TABLE_REQUIRED.contains(&kind) && table == 0 {
        write_instance(
            record,
            Instance {
                payload: 0,
                owner: 0,
                payload_b: 0,
                owner_b: 0,
            },
        );
        skip_log(&format!(
            "owned load for kind {kind} produced no kind table, so it is withdrawn and the game keeps ownership of this kind"
        ));
        return false;
    }
    true
}

unsafe fn ensure_clone(clone_kind: i32, base_kind: i32, entry_id: i32) -> Option<Instance> {
    let state = clone_state(clone_kind)?;
    let payload = state.payload.load(Ordering::Relaxed);
    if payload != 0 {
        return Some(Instance {
            payload,
            owner: state.owner.load(Ordering::Relaxed),
            payload_b: state.payload_b.load(Ordering::Relaxed),
            owner_b: state.owner_b.load(Ordering::Relaxed),
        });
    }
    let record = crate::load_pipeline::resource_record_for_entry(entry_id)?;
    let built = build_instance(base_kind, directory_indices(record, clone_kind)?)?;
    crate::article_params::serve_own_blocks(clone_kind, built.payload);
    state.payload.store(built.payload, Ordering::Relaxed);
    state.owner.store(built.owner, Ordering::Relaxed);
    state.payload_b.store(built.payload_b, Ordering::Relaxed);
    state.owner_b.store(built.owner_b, Ordering::Relaxed);
    Some(built)
}

unsafe fn capture_clone(state: &BaseState, clone_kind: i32, record: usize) {
    let Some(slot) = clone_state(clone_kind) else {
        return;
    };
    let current = read_instance(record);
    if current.payload == 0 || current.payload == state.vanilla_payload.load(Ordering::Relaxed) {
        skip_log(&format!(
            "capture-clone clone={clone_kind} declined resident={:#x}",
            current.payload
        ));
        return;
    }
    crate::article_params::serve_own_blocks(clone_kind, current.payload);
    slot.payload.store(current.payload, Ordering::Relaxed);
    slot.owner.store(current.owner, Ordering::Relaxed);
    slot.payload_b.store(current.payload_b, Ordering::Relaxed);
    slot.owner_b.store(current.owner_b, Ordering::Relaxed);
    skip_log(&format!(
        "capture-clone clone={clone_kind} instance A {:#x} B {:#x}",
        current.payload, current.payload_b
    ));
}

pub(crate) const PARAM_KIND_TABLE: usize = 0x110;

pub(crate) const KIND_TABLE_REQUIRED: [i32; 2] = [0x15, 0x16];

unsafe fn kind_table_of(payload: usize) -> usize {
    if payload == 0 || payload & 7 != 0 {
        return 0;
    }
    core::ptr::read_volatile((payload + PARAM_KIND_TABLE) as *const usize)
}

pub(crate) unsafe fn borrow_vanilla_resident(base_kind: i32) -> Option<(usize, Instance, usize, usize)> {
    let record = record_of(base_kind)?;
    let state = base_state(base_kind)?;
    let resident = read_instance(record);
    let resident_table = kind_table_of(resident.payload);
    let payload = state.vanilla_payload.load(Ordering::Relaxed);
    let vanilla_table = kind_table_of(payload);
    if resident_table != 0 || payload == 0 || payload == resident.payload || vanilla_table == 0 {
        return None;
    }
    write_instance(
        record,
        Instance {
            payload,
            owner: state.vanilla_owner.load(Ordering::Relaxed),
            payload_b: state.vanilla_payload_b.load(Ordering::Relaxed),
            owner_b: state.vanilla_owner_b.load(Ordering::Relaxed),
        },
    );
    Some((record, resident, resident_table, vanilla_table))
}

pub(crate) unsafe fn release_vanilla_resident(held: (usize, Instance, usize, usize)) {
    write_instance(held.0, held.1);
}

pub(crate) unsafe fn resident_kind_table(base_kind: i32) -> (usize, usize) {
    match record_of(base_kind) {
        Some(record) => {
            let payload = read_pair(record).0;
            (payload, kind_table_of(payload))
        }
        None => (0, 0),
    }
}

unsafe fn capture_vanilla(state: &BaseState, base_kind: i32, record: usize) {
    let current = read_instance(record);
    if current.payload == 0 {
        skip_log(&format!("capture-vanilla base={base_kind} resident is null"));
        return;
    }
    state.vanilla_payload.store(current.payload, Ordering::Relaxed);
    state.vanilla_owner.store(current.owner, Ordering::Relaxed);
    state.vanilla_payload_b.store(current.payload_b, Ordering::Relaxed);
    state.vanilla_owner_b.store(current.owner_b, Ordering::Relaxed);
    state.origin.store(current.payload, Ordering::Relaxed);
    skip_log(&format!(
        "capture-vanilla base={base_kind} instance A {:#x} B {:#x}",
        current.payload, current.payload_b
    ));
}

pub(crate) unsafe fn enter_clone_window(
    clone_kind: i32,
    base_kind: i32,
    entry_id: i32,
) -> SwapGuard {
    let Some(record) = record_of(base_kind) else {
        skip_log(&format!(
            "enter clone={clone_kind} base={base_kind} has no record (singleton absent)"
        ));
        return SwapGuard::idle();
    };
    let Some(state) = base_state(base_kind) else {
        skip_log(&format!("enter clone={clone_kind} base={base_kind} out of range"));
        return SwapGuard::idle();
    };

    let swap_owned = lock_swaps();
    let mut resident = read_pair(record).0;
    if resident == 0 {
        forget_all(state, base_kind);
        state.saw_clone.store(true, Ordering::Relaxed);
        if load_owned_params(base_kind, entry_id) {
            capture_vanilla(state, base_kind, record);
            resident = read_pair(record).0;
        }
    }
    if resident == 0 {
        unlock_swaps(swap_owned);
        let as_vanilla = KIND_TABLE_REQUIRED.contains(&base_kind);
        skip_log(&format!(
            "enter clone={clone_kind} base={base_kind} params not loaded yet; capturing after construction as {}",
            if as_vanilla { "the base's own" } else { "the clone's" }
        ));
        return SwapGuard {
            record,
            saved: EMPTY_INSTANCE,
            kind: clone_kind,
            base_kind,
            mode: if as_vanilla {
                GuardMode::CaptureVanilla
            } else {
                GuardMode::CaptureClone
            },
            holds_refcount: false,
        };
    }

    revalidate(base_kind, record);
    state.saw_clone.store(true, Ordering::Relaxed);

    let current = window_resident(base_kind, record);
    let instance = ensure_clone(clone_kind, base_kind, entry_id);
    let guard = match instance {
        Some(instance)
            if instance.payload != current.payload || instance.payload != read_pair(record).0 =>
        {
            write_instance(record, instance);
            SwapGuard {
                record,
                saved: current,
                kind: clone_kind,
                base_kind,
                mode: GuardMode::Restore,
                holds_refcount: hold_unconstructed_record(record),
            }
        }
        _ => SwapGuard::idle(),
    };
    unlock_swaps(swap_owned);

    let n = PARAM_INSTANCE_LOG.fetch_add(1, Ordering::Relaxed);
    if n < 32 {
        let (peak, refused) = clone_slot_peak();
        dbg_log!(
            "[cloneparam] #{n} enter clone={clone_kind} base={base_kind} resident={:#x} instance={:#x} swapped={} record_held={} slots peak {peak}/{CLONE_SLOTS} refused {refused}",
            current.payload,
            instance.map(|instance| instance.payload).unwrap_or(0),
            matches!(guard.mode, GuardMode::Restore),
            guard.holds_refcount
        );
    }
    guard
}

static SEPARATE_VANILLA_PARAMS: AtomicBool = AtomicBool::new(true);
static SHARED_PARAM_LOG: AtomicU32 = AtomicU32::new(0);

pub(crate) unsafe fn enter_vanilla_window(base_kind: i32, entry_id: i32) -> SwapGuard {
    let Some(record) = record_of(base_kind) else {
        return SwapGuard::idle();
    };
    let Some(state) = base_state(base_kind) else {
        return SwapGuard::idle();
    };

    let swap_owned = lock_swaps();
    let resident = read_pair(record).0;
    if resident == 0 {
        unlock_swaps(swap_owned);
        return SwapGuard {
            record,
            saved: EMPTY_INSTANCE,
            kind: -1,
            base_kind,
            mode: GuardMode::CaptureVanilla,
            holds_refcount: false,
        };
    }

    revalidate(base_kind, record);
    let separate = state.saw_clone.load(Ordering::Relaxed)
        && !state.separated.load(Ordering::Relaxed);
    let installed = if separate && SEPARATE_VANILLA_PARAMS.load(Ordering::Relaxed) {
        let vanilla = ensure_vanilla(state, base_kind, entry_id);
        if let Some(instance) = vanilla {
            if read_instance(record).payload != instance.payload {
                write_instance(record, instance);
            }
            state.separated.store(true, Ordering::Relaxed);
        }
        vanilla
    } else {
        if separate {
            let n = SHARED_PARAM_LOG.fetch_add(1, Ordering::Relaxed);
            if n < 8 {
                crate::dbg_log_public(&format!(
                    "[cloneparam] base {base_kind} shares its vl.prc struct with a clone; running SHARED (per-instance separation is off). Its params may read as the clone's"
                ));
            }
        }
        None
    };
    let holds_refcount = installed.is_some() && hold_unconstructed_record(record);
    unlock_swaps(swap_owned);

    if let Some(instance) = installed {
        dbg_log_public(&format!(
            "[cloneparam] base {base_kind} shares its vl.prc struct with a clone; installed the base fighter's own instance A {:#x} B {:#x} as resident (record_held={holds_refcount})",
            instance.payload, instance.payload_b
        ));
    } else if separate {
        dbg_log_public(&format!(
            "[cloneparam] WARNING base {base_kind} needs its own vl.prc instance but none could be built; it will run on the clone's params this match"
        ));
    }
    if !holds_refcount {
        return SwapGuard::idle();
    }
    SwapGuard {
        record,
        saved: EMPTY_INSTANCE,
        kind: -1,
        base_kind,
        mode: GuardMode::Idle,
        holds_refcount,
    }
}

pub(crate) unsafe fn leave_window(guard: SwapGuard) {
    if guard.record == 0 {
        return;
    }
    if guard.holds_refcount {
        release_record_hold(guard.record);
    }
    match guard.mode {
        GuardMode::Idle => {}
        GuardMode::Restore => {
            let swap_owned = lock_swaps();
            write_instance(guard.record, guard.saved);
            unlock_swaps(swap_owned);
        }
        GuardMode::CaptureClone => {
            let swap_owned = lock_swaps();
            if let Some(state) = base_state(guard.base_kind) {
                capture_clone(state, guard.kind, guard.record);
            }
            unlock_swaps(swap_owned);
        }
        GuardMode::CaptureVanilla => {
            let swap_owned = lock_swaps();
            if let Some(state) = base_state(guard.base_kind) {
                capture_vanilla(state, guard.base_kind, guard.record);
            }
            unlock_swaps(swap_owned);
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct FrameGuard {
    record: usize,
    base_kind: i32,
    installed: usize,
    previous: Instance,
}

impl FrameGuard {
    pub(crate) const IDLE: Self = Self {
        record: 0,
        base_kind: -1,
        installed: 0,
        previous: EMPTY_INSTANCE,
    };

    pub(crate) fn installed(&self) -> usize {
        self.installed
    }
}

struct FrameHome {
    depth: AtomicU32,
    payload: AtomicUsize,
    owner: AtomicUsize,
    payload_b: AtomicUsize,
    owner_b: AtomicUsize,
}

impl FrameHome {
    const fn new() -> Self {
        Self {
            depth: AtomicU32::new(0),
            payload: AtomicUsize::new(0),
            owner: AtomicUsize::new(0),
            payload_b: AtomicUsize::new(0),
            owner_b: AtomicUsize::new(0),
        }
    }

    fn store(&self, instance: Instance) {
        self.payload.store(instance.payload, Ordering::Relaxed);
        self.owner.store(instance.owner, Ordering::Relaxed);
        self.payload_b.store(instance.payload_b, Ordering::Relaxed);
        self.owner_b.store(instance.owner_b, Ordering::Relaxed);
    }

    fn load(&self) -> Instance {
        Instance {
            payload: self.payload.load(Ordering::Relaxed),
            owner: self.owner.load(Ordering::Relaxed),
            payload_b: self.payload_b.load(Ordering::Relaxed),
            owner_b: self.owner_b.load(Ordering::Relaxed),
        }
    }
}

static FRAME_HOMES: [FrameHome; PARAM_NATIVE_KINDS as usize] =
    [const { FrameHome::new() }; PARAM_NATIVE_KINDS as usize];
static FRAME_LOG: AtomicU32 = AtomicU32::new(0);
static FRAME_MOVED_LOG: AtomicU32 = AtomicU32::new(0);

fn frame_home(base_kind: i32) -> Option<&'static FrameHome> {
    usize::try_from(base_kind).ok().and_then(|base| FRAME_HOMES.get(base))
}

unsafe fn window_resident(base_kind: i32, record: usize) -> Instance {
    match frame_home(base_kind) {
        Some(home) if home.depth.load(Ordering::Relaxed) != 0 => home.load(),
        _ => read_instance(record),
    }
}

fn cached_clone(clone_kind: i32) -> Option<Instance> {
    let state = clone_state(clone_kind)?;
    let payload = state.payload.load(Ordering::Relaxed);
    (payload != 0).then(|| Instance {
        payload,
        owner: state.owner.load(Ordering::Relaxed),
        payload_b: state.payload_b.load(Ordering::Relaxed),
        owner_b: state.owner_b.load(Ordering::Relaxed),
    })
}

pub(crate) unsafe fn enter_object_frame(clone_kind: i32) -> FrameGuard {
    let Some(base_kind) = clone_base(clone_kind) else {
        return FrameGuard::IDLE;
    };
    let (Some(instance), Some(record), Some(home)) =
        (cached_clone(clone_kind), record_of(base_kind), frame_home(base_kind))
    else {
        return FrameGuard::IDLE;
    };
    let owned = lock_swaps();
    let current = read_instance(record);
    if current.payload == 0 || current.payload == instance.payload {
        unlock_swaps(owned);
        return FrameGuard::IDLE;
    }
    if home.depth.fetch_add(1, Ordering::Relaxed) == 0 {
        home.store(current);
    }
    write_instance(record, instance);
    unlock_swaps(owned);
    if FRAME_LOG.fetch_add(1, Ordering::Relaxed) < 4 {
        dbg_log_public(&format!(
            "[cloneparam] clone {clone_kind} runs on its own vl.prc: A {:#x} B {:#x} stands in for A {:#x} B {:#x} in base {base_kind}'s record",
            instance.payload, instance.payload_b, current.payload, current.payload_b
        ));
    }
    FrameGuard {
        record,
        base_kind,
        installed: instance.payload,
        previous: current,
    }
}

pub(crate) unsafe fn leave_object_frame(guard: FrameGuard) {
    if guard.record == 0 {
        return;
    }
    let owned = lock_swaps();
    if read_pair(guard.record).0 == guard.installed {
        write_instance(guard.record, guard.previous);
    } else if FRAME_MOVED_LOG.fetch_add(1, Ordering::Relaxed) < 4 {
        dbg_log_public(&format!(
            "[cloneparam] base {}'s record changed while a clone ran (a fighter was being built); the builder restores it",
            guard.base_kind
        ));
    }
    if let Some(home) = frame_home(guard.base_kind) {
        let _ = home
            .depth
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |depth| depth.checked_sub(1));
    }
    unlock_swaps(owned);
}

const WORK_MODULE_OFFSET: usize = 0x50;
const WORK_DATA_BLOCK_OFFSET: usize = 0x198;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;

static WORK_ROOT_LOG: AtomicU32 = AtomicU32::new(0);

unsafe fn plausible(value: usize) -> bool {
    value >= LOWEST_PLAUSIBLE_POINTER && value & 7 == 0
}

unsafe fn read_plausible(at: usize) -> Option<usize> {
    if !plausible(at) {
        return None;
    }
    let value = core::ptr::read_volatile(at as *const usize);
    plausible(value).then_some(value)
}

fn describe_root(root: usize) -> &'static str {
    REGISTRY.describe_root(root)
}

pub(crate) unsafe fn observe_work_param_root(boma: usize) {
    if WORK_ROOT_LOG.load(Ordering::Relaxed) >= 16 {
        return;
    }
    let Some(work) = read_plausible(boma + WORK_MODULE_OFFSET) else {
        return;
    };
    let Some(block) = read_plausible(work + WORK_DATA_BLOCK_OFFSET) else {
        return;
    };
    let Some(root) = read_plausible(block) else {
        return;
    };
    let n = WORK_ROOT_LOG.fetch_add(1, Ordering::Relaxed);
    if n < 16 {
        dbg_log!(
            "[cloneparam] #{n} work param root boma={boma:#x} block={block:#x} root={root:#x} classified={}",
            describe_root(root)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_slots_cover_the_backend_kind_range() {
        assert!(FIRST_CUSTOM_KIND + CLONE_SLOTS as i32 - 1 >= 255);
    }

    #[test]
    fn out_of_range_clone_kind_is_counted_not_wrapped() {
        let before = clone_slot_peak().1;
        assert!(clone_state(FIRST_CUSTOM_KIND + CLONE_SLOTS as i32).is_none());
        assert_eq!(clone_slot_peak().1, before + 1);
    }

    #[test]
    fn native_kind_bound_matches_the_record_array() {
        assert_eq!(PARAM_NATIVE_KINDS, 94);
    }
}

static FOREIGN_LOAD_LOG: AtomicU32 = AtomicU32::new(0);
static PARAM_DATA_REAL: [AtomicBool; PARAM_NATIVE_KINDS as usize] =
    [const { AtomicBool::new(false) }; PARAM_NATIVE_KINDS as usize];

static FOREIGN_LOADED: [AtomicBool; PARAM_NATIVE_KINDS as usize] =
    [const { AtomicBool::new(false) }; PARAM_NATIVE_KINDS as usize];

static FOREIGN_PAYLOAD: [AtomicUsize; PARAM_NATIVE_KINDS as usize] =
    [const { AtomicUsize::new(0) }; PARAM_NATIVE_KINDS as usize];

const PAYLOAD_STUB_BYTES: usize = 0x4000;

pub(crate) fn kind_params_present(kind: i32) -> bool {
    unsafe { payload_of(kind).is_some() }
}

pub(crate) unsafe fn payload_of(kind: i32) -> Option<usize> {
    let record = record_of(kind)?;
    let payload = read_pair(record).0;
    (payload != 0).then_some(payload)
}

pub(crate) unsafe fn payload_fields(kind: i32, offsets: &[u32]) -> Option<Vec<(u32, usize)>> {
    let record = record_of(kind)?;
    let payload = read_pair(record).0;
    if payload == 0 {
        return None;
    }
    Some(
        offsets
            .iter()
            .copied()
            .filter(|offset| (*offset as usize) < PAYLOAD_STUB_BYTES)
            .map(|offset| {
                (
                    offset,
                    core::ptr::read_volatile((payload + offset as usize) as *const usize),
                )
            })
            .collect(),
    )
}

pub(crate) unsafe fn fill_payload_pointers(kind: i32, offsets: &[u32]) -> (usize, usize) {
    let Some(record) = record_of(kind) else {
        return (0, 0);
    };
    let payload = read_pair(record).0;
    if payload == 0 {
        return (0, 0);
    }
    let (mut filled, mut present) = (0, 0);
    for offset in offsets.iter().copied() {
        if offset as usize >= PAYLOAD_STUB_BYTES {
            continue;
        }
        let slot = (payload + offset as usize) as *mut usize;
        if core::ptr::read_volatile(slot) != 0 {
            present += 1;
            continue;
        }
        let block = vec![0u8; PAYLOAD_STUB_BYTES].leak();
        core::ptr::write_volatile(slot, block.as_ptr() as usize);
        filled += 1;
    }
    (filled, present)
}

const RESOURCE_FILE_MISSING: u32 = 0xffffff;

pub(crate) unsafe fn file_resident(index: u32) -> Result<u64, &'static str> {
    if index == RESOURCE_FILE_MISSING {
        return Err("the index is the not-found sentinel");
    }
    let loader = core::ptr::read_volatile((crate::game::addr(PARAM_FILE_LOADER)) as *const u64);
    if loader == 0 {
        return Err("the resource loader is null");
    }
    let paths = core::ptr::read_volatile((loader + 8) as *const u64);
    let path_count = core::ptr::read_volatile((loader + 0x18) as *const u32);
    if paths == 0 || index >= path_count {
        return Err("the file index is past the loader's path table");
    }
    let entry = paths + index as u64 * 8;
    if core::ptr::read_volatile((entry + 4) as *const u8) == 0 {
        return Err("the path entry's residency byte is clear");
    }
    let data_index = core::ptr::read_volatile(entry as *const u32);
    if data_index == RESOURCE_FILE_MISSING {
        return Err("the path entry has no data index");
    }
    let datas = core::ptr::read_volatile((loader + 0x10) as *const u64);
    let data_count = core::ptr::read_volatile((loader + 0x1c) as *const u32);
    if datas == 0 || data_index >= data_count {
        return Err("the data index is past the loader's data table");
    }
    let data = datas + data_index as u64 * 0x18;
    if core::ptr::read_volatile((data + 0xc) as *const u8) == 0 {
        return Err("the data entry's residency byte is clear");
    }
    let resident = core::ptr::read_volatile(data as *const u64);
    if resident == 0 {
        return Err("the data entry is flagged resident but its pointer is null");
    }
    Ok(resident)
}

pub(crate) unsafe fn substitute_param_file(kind: i32, index: i32) -> i32 {
    if index < 0 {
        return index;
    }
    let Some(source) = crate::custom_articles::fighter_name(kind).and_then(|n| n.to_str().ok())
    else {
        skip_log(&format!("no lowercase name for fighter kind {kind}; param file kept"));
        return index;
    };
    let candidates = crate::custom_articles::resource_owners_for_source_owner(kind);
    if candidates.is_empty() {
        skip_log(&format!(
            "no registered clone article sources from fighter {kind} ({source}); param file kept"
        ));
        return index;
    }
    let owners: Vec<&str> = candidates
        .iter()
        .filter_map(|owner| {
            core::str::from_utf8(owner.split(|byte| *byte == 0).next().unwrap_or(owner)).ok()
        })
        .collect();
    if owners.is_empty() {
        skip_log("no clone resource owner name is utf8; param file kept");
        return index;
    }

    for file in ["vl.prc", "expression_vl.prc"] {
        let vanilla = format!("fighter/{source}/param/{file}");
        let from = crate::item_params::scan_file_path_index(crate::hash40::hash40(&vanilla));
        skip_log(&format!(
            "{vanilla} scans to {from:?}; the loader was handed {index:#x}"
        ));
        let Some(from) = from else {
            continue;
        };
        if from as i32 != index {
            continue;
        }
        if file == "vl.prc" && file_resident(from).is_ok() {
            skip_log(&format!("{vanilla} is already resident; keeping it"));
            note_param_data_real(kind);
            return index;
        }
        let mut tried = Vec::new();
        for owner in owners.iter() {
            let ours = format!("fighter/{owner}/param/{source}_{file}");
            let Some(replacement) =
                crate::item_params::scan_file_path_index(crate::hash40::hash40(&ours))
            else {
                tried.push(format!("{ours} (not shipped)"));
                continue;
            };
            match file_resident(replacement) {
                Ok(_) => {
                    skip_log(&format!(
                        "{vanilla} (index {index:#x}) has no resident data; loading {ours} (index {replacement:#x}) in its place"
                    ));
                    if file == "vl.prc" {
                        note_param_data_real(kind);
                    }
                    return replacement as i32;
                }
                Err(reason) => tried.push(format!("{ours} ({reason})")),
            }
        }
        skip_log(&format!(
            "{vanilla} is not resident and no registered pack ships a usable copy; {source}'s params will be defaults only. Tried: {}",
            tried.join(", ")
        ));
        return index;
    }
    index
}

pub(crate) unsafe fn kind_params_absent(kind: i32) -> bool {
    if kind < 0 || kind >= PARAM_NATIVE_KINDS {
        return false;
    }
    if kind_params_are_foreign(kind) {
        return true;
    }
    match record_of(kind) {
        Some(record) => read_pair(record).0 == 0,
        None => true,
    }
}

fn note_param_data_real(kind: i32) {
    if (0..PARAM_NATIVE_KINDS).contains(&kind) {
        PARAM_DATA_REAL[kind as usize].store(true, Ordering::Relaxed);
    }
}

pub(crate) fn kind_params_have_real_data(kind: i32) -> bool {
    if kind < 0 || kind >= PARAM_NATIVE_KINDS {
        return false;
    }
    if !kind_params_are_foreign(kind) {
        return kind_params_present(kind);
    }
    PARAM_DATA_REAL[kind as usize].load(Ordering::Relaxed) && kind_params_present(kind)
}

pub(crate) fn kind_params_are_foreign(kind: i32) -> bool {
    if kind < 0 || kind >= PARAM_NATIVE_KINDS {
        return false;
    }
    FOREIGN_LOADED[kind as usize].load(Ordering::Relaxed)
}

pub(crate) fn forget_foreign_loads() {
    for slot in FOREIGN_LOADED.iter() {
        slot.store(false, Ordering::Relaxed);
    }
}

unsafe fn any_live_resource_record() -> Option<usize> {
    (0..8).find_map(|entry| {
        crate::load_pipeline::resource_record_for_entry(entry as i32).filter(|r| *r != 0)
    })
}

pub(crate) unsafe fn ensure_kind_params_loaded(kind: i32) -> bool {
    if kind < 0 || kind >= PARAM_NATIVE_KINDS {
        return false;
    }
    let Some(record) = record_of(kind) else {
        return false;
    };
    if read_pair(record).0 != 0 {
        return true;
    }
    if FOREIGN_PAYLOAD[kind as usize].swap(0, Ordering::Relaxed) != 0 {
        FOREIGN_LOADED[kind as usize].store(false, Ordering::Relaxed);
        PARAM_DATA_REAL[kind as usize].store(false, Ordering::Relaxed);
        foreign_log(kind, "the game released the earlier load with its match; loading it again");
    }
    if FOREIGN_LOADED[kind as usize].swap(true, Ordering::Relaxed) {
        return false;
    }
    let Some(singleton) = singleton() else {
        return false;
    };
    let Some(donor) = any_live_resource_record() else {
        foreign_log(kind, "no live resource record to resolve its param directory with");
        return false;
    };
    let Some((index_a, index_b)) = directory_indices(donor, kind) else {
        foreign_log(kind, "its param directories did not resolve");
        return false;
    };
    let loader_a: ParamLoader = core::mem::transmute(crate::game::addr(PARAM_LOADER));
    let loader_b: ParamLoader = core::mem::transmute(crate::game::addr(PARAM_LOADER_B));
    let held_a = substitute_param_file(kind, index_a);
    let held_b = substitute_param_file(kind, index_b);
    request_resource_file(held_a);
    request_resource_file(held_b);
    lock_param_singleton(singleton);
    loader_a(singleton, kind, &held_a as *const i32);
    loader_b(singleton, kind, &held_b as *const i32);
    let loaded = read_instance(record);
    unlock_param_singleton(singleton);
    if loaded.payload == 0 || loaded.payload_b == 0 {
        foreign_log(kind, "the loaders produced an incomplete record");
        return false;
    }
    FOREIGN_PAYLOAD[kind as usize].store(loaded.payload, Ordering::Relaxed);
    foreign_log(
        kind,
        &format!(
            "loaded from directories ({index_a},{index_b}), A {:#x} B {:#x}",
            loaded.payload, loaded.payload_b
        ),
    );
    report_owner_fields(kind, loaded.payload);
    true
}

unsafe fn report_owner_fields(kind: i32, payload: usize) {
    if payload == 0 {
        return;
    }
    let targets = crate::item_params::owner_targets(kind);
    let known = dionysus_core::owner_param_words::words_of(kind).len();
    if targets.is_empty() {
        crate::dbg_log_public(&format!(
            "[ownerparam] {kind} loaded for a clone item, {known} known word(s), none overridden"
        ));
        return;
    }
    for (public_kind, offset, bits, mul) in targets {
        if !crate::item_params::owner_offset_valid(offset) {
            continue;
        }
        let raw = core::ptr::read_volatile((payload + offset as usize) as *const u32);
        let bits = crate::item_params::owner_value(kind, offset, raw, bits, mul);
        crate::dbg_log_public(&format!(
            "[ownerparam] {kind} loaded for public={public_kind:#x}: +{offset:#x} holds {raw:#x}, {}, override {}",
            dionysus_core::owner_param_words::describe(kind, offset, raw),
            dionysus_core::owner_param_words::word_at(kind, offset)
                .map(|word| word.shown(bits))
                .unwrap_or_else(|| format!("{bits:#x}"))
        ));
    }
}


unsafe fn foreign_log(kind: i32, what: &str) {
    let n = FOREIGN_LOAD_LOG.fetch_add(1, Ordering::Relaxed);
    if n < 16 {
        crate::dbg_log_public(&format!(
            "[cloneparam] absent fighter {kind} is referenced by a clone item: {what}"
        ));
    }
}
