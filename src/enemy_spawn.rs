use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const OFF_MANAGER: usize = 0x52b_8390;
const OFF_OBJECT_POOLS: usize = 0x52b_7548;
const OFF_STAGE_GATE: usize = 0x134_b0bc;
const STAGE_GATE_OPCODE: u32 = 0x7100_7d1f;
const OFF_STAGE_KINDS: usize = 0x14f_6660;
const STAGE_KINDS_OPCODE: u32 = 0xd001_ee68;
const OFF_REQUEST_KIND: usize = 0x57d_6e0;
const REQUEST_KIND_OPCODE: u32 = 0xd102_43ff;
const OFF_SPAWN: usize = 0x56c_360;
const SPAWN_OPCODE: u32 = 0xd104_43ff;
const OFF_REMOVE: usize = 0x56_c130;
const REMOVE_OPCODE: u32 = 0xf81c_0ff7;
const REMOVE_NOW: u32 = 0;
const MANAGER_KINDS: usize = 0x40;
const MANAGER_RECORDS: usize = 0x20;
const RECORD_STRIDE: usize = 0x70;
const RECORD_REQUEST: usize = 0x44;
const RECORD_ACCESSOR: usize = 0x50;
const RECORD_OBJECT: usize = 0x58;
const RECORD_STATE: usize = 0x5c;
const RECORD_LIVE: u32 = 1;
const ACCESSOR_CONTROL: usize = 0x158;
const CONTROL_START: usize = 0x130;
const START_NOW: u32 = 0;
const GATE_REGISTER: usize = 8;
const ENEMY_STAGES: u32 = 0x4004_8041;
const ENEMY_STAGE_SPAN: u64 = 0x1f;
const FIRST_ENEMY_STAGE: u64 = 0;
const ENEMY_POOL: usize = 0x10;
const POOL_OBJECTS: usize = 8;
const POOL_COUNT: usize = 0x18;
const OBJECT_STRIDE: usize = 0x60c0;
const OBJECT_ID: usize = 8;
const OBJECT_HOLDER: usize = 0x158;
const HOLDER_INSTANCE: usize = 0x10;
const INSTANCE_STATUS: usize = 0x10;
const NO_STATUS: i32 = -1;
const KIND_COUNT: u32 = crate::enemy_clones::KIND_LIMIT;
const SCALE: f32 = 1.0;
const SPAWN_MODE: u32 = 2;
const REFUSED: i32 = -1;

pub(crate) const NO_OBJECT: u32 = 0x5000_0000;
pub(crate) const UNAVAILABLE: u32 = u32::MAX;

#[repr(C, align(16))]
struct Descriptor {
    kind: u32,
    variation: u32,
    scale: f32,
    reserved: u32,
    position: [f32; 4],
    lr: f32,
    mode: u32,
    tail: [u64; 3],
}

type RequestKindFn = unsafe extern "C" fn(usize, u32);
type SpawnFn = unsafe extern "C" fn(usize, *const Descriptor) -> i32;
type StartFn = unsafe extern "C" fn(usize, u32);
type RemoveFn = unsafe extern "C" fn(usize, u32, i32) -> i32;

static ARMED: AtomicBool = AtomicBool::new(false);
static KINDS: AtomicU32 = AtomicU32::new(0);
static FORCED: AtomicBool = AtomicBool::new(false);

unsafe fn manager() -> usize {
    core::ptr::read_volatile(crate::game::addr(OFF_MANAGER) as *const usize)
}

fn enemy_stage(offset: u64) -> bool {
    offset < ENEMY_STAGE_SPAN && ENEMY_STAGES >> offset & 1 != 0
}

pub(crate) fn gate_installed() -> bool {
    ARMED.load(Ordering::Acquire)
}

#[skyline::hook(offset = crate::game::rel(OFF_STAGE_GATE), inline)]
unsafe fn stage_gate(ctx: &mut skyline::hooks::InlineCtx) {
    #[cfg(feature = "css_slot")]
    if let Some(donor) = crate::stage_donor_tests::enemy_gate_offset(ctx.registers[GATE_REGISTER].w()) {
        ctx.registers[GATE_REGISTER].set_x(donor as u64);
    }
    if KINDS.load(Ordering::Acquire) == 0 {
        return;
    }
    let offset = ctx.registers[GATE_REGISTER].x() as u32 as u64;
    if enemy_stage(offset) {
        return;
    }
    FORCED.store(true, Ordering::Release);
    ctx.registers[GATE_REGISTER].set_x(FIRST_ENEMY_STAGE);
    skyline::println!("[enemy] the enemy manager is made for this battle");
}

#[skyline::hook(offset = crate::game::rel(OFF_STAGE_KINDS), inline)]
unsafe fn stage_kinds(_ctx: &mut skyline::hooks::InlineCtx) {
    if !FORCED.swap(false, Ordering::AcqRel) {
        return;
    }
    let kinds = KINDS.load(Ordering::Acquire);
    let manager = manager();
    if manager == 0 || kinds == 0 {
        return;
    }
    let request: RequestKindFn = core::mem::transmute(crate::game::addr(OFF_REQUEST_KIND));
    for kind in (0..KIND_COUNT).filter(|kind| kinds >> kind & 1 != 0 && crate::enemy_clones::known(*kind)) {
        request(manager + MANAGER_KINDS, kind);
    }
    skyline::println!("[enemy] enemy kinds {kinds:#x} requested for this battle");
}

pub(crate) fn armed(kind: u32) -> bool {
    kind < KIND_COUNT && KINDS.load(Ordering::Acquire) >> kind & 1 != 0
}

pub(crate) fn arm(kinds: u32) {
    KINDS.store(kinds, Ordering::Release);
    skyline::println!("[enemy] enemy kinds {kinds:#x} come to every battle");
}

pub(crate) fn spawn(kind: u32, x: f32, y: f32, z: f32, lr: f32) -> u32 {
    if !ARMED.load(Ordering::Acquire) || !crate::enemy_clones::known(kind) {
        return UNAVAILABLE;
    }
    crate::enemy_params::ensure(kind);
    let descriptor = Descriptor {
        kind,
        variation: 0,
        scale: SCALE,
        reserved: 0,
        position: [x, y, z, 0.0],
        lr: if lr < 0.0 { -SCALE } else { SCALE },
        mode: SPAWN_MODE,
        tail: [0; 3],
    };
    unsafe {
        let manager = manager();
        if manager == 0 {
            skyline::println!("[enemy] kind {kind}: this battle has no enemy manager");
            return UNAVAILABLE;
        }
        let spawn: SpawnFn = core::mem::transmute(crate::game::addr(OFF_SPAWN));
        let request = spawn(manager, &descriptor);
        if request == REFUSED {
            skyline::println!("[enemy] kind {kind} at ({x:.0}, {y:.0}): refused");
            return UNAVAILABLE;
        }
        skyline::println!("[enemy] kind {kind} at ({x:.0}, {y:.0}): request {request}");
        request as u32
    }
}

pub(crate) fn start(request: u32) -> u32 {
    unsafe {
        let manager = manager();
        if !ARMED.load(Ordering::Acquire) || manager == 0 {
            return NO_OBJECT;
        }
        let first = core::ptr::read_volatile((manager + MANAGER_RECORDS) as *const usize);
        let end = core::ptr::read_volatile((manager + MANAGER_RECORDS + 8) as *const usize);
        let mut record = first;
        while record != 0 && record < end {
            let found = core::ptr::read_volatile((record + RECORD_REQUEST) as *const u32) == request;
            if found && core::ptr::read_volatile((record + RECORD_STATE) as *const u32) == RECORD_LIVE {
                let accessor = core::ptr::read_volatile((record + RECORD_ACCESSOR) as *const usize);
                let control = if accessor == 0 { 0 } else { core::ptr::read_volatile((accessor + ACCESSOR_CONTROL) as *const usize) };
                if control == 0 {
                    return NO_OBJECT;
                }
                let vtable = core::ptr::read_volatile(control as *const usize);
                let start: StartFn = core::mem::transmute(core::ptr::read_volatile((vtable + CONTROL_START) as *const usize));
                start(control, START_NOW);
                let object = core::ptr::read_volatile((record + RECORD_OBJECT) as *const u32);
                skyline::println!("[enemy] request {request}: object {object:#x} comes out");
                return object;
            }
            record += RECORD_STRIDE;
        }
        NO_OBJECT
    }
}

pub(crate) fn remove(object: u32) -> bool {
    unsafe {
        let manager = manager();
        if !ARMED.load(Ordering::Acquire) || manager == 0 {
            return false;
        }
        let first = core::ptr::read_volatile((manager + MANAGER_RECORDS) as *const usize);
        let end = core::ptr::read_volatile((manager + MANAGER_RECORDS + 8) as *const usize);
        let mut record = first;
        while record != 0 && record < end {
            if core::ptr::read_volatile((record + RECORD_OBJECT) as *const u32) == object
                && core::ptr::read_volatile((record + RECORD_STATE) as *const u32) == RECORD_LIVE
            {
                let index = ((record - first) / RECORD_STRIDE) as i32;
                let remove: RemoveFn = core::mem::transmute(crate::game::addr(OFF_REMOVE));
                remove(manager, REMOVE_NOW, index);
                skyline::println!("[enemy] object {object:#x} (record {index}) removed");
                return true;
            }
            record += RECORD_STRIDE;
        }
        false
    }
}

pub(crate) fn object(index: u32) -> u32 {
    unsafe {
        let pools = core::ptr::read_volatile(crate::game::addr(OFF_OBJECT_POOLS) as *const usize);
        if pools == 0 {
            return NO_OBJECT;
        }
        let pool = core::ptr::read_volatile((pools + ENEMY_POOL) as *const usize);
        if pool == 0 || index >= core::ptr::read_volatile((pool + POOL_COUNT) as *const u32) {
            return NO_OBJECT;
        }
        let objects = core::ptr::read_volatile((pool + POOL_OBJECTS) as *const usize);
        core::ptr::read_volatile((objects + index as usize * OBJECT_STRIDE + OBJECT_ID) as *const u32)
    }
}

pub(crate) fn object_address(object: u32) -> usize {
    unsafe {
        let pools = core::ptr::read_volatile(crate::game::addr(OFF_OBJECT_POOLS) as *const usize);
        let pool = if pools == 0 { 0 } else { core::ptr::read_volatile((pools + ENEMY_POOL) as *const usize) };
        if pool == 0 {
            return 0;
        }
        let objects = core::ptr::read_volatile((pool + POOL_OBJECTS) as *const usize);
        let count = core::ptr::read_volatile((pool + POOL_COUNT) as *const u32) as usize;
        (0..count)
            .map(|index| objects + index * OBJECT_STRIDE)
            .find(|base| core::ptr::read_volatile((base + OBJECT_ID) as *const u32) == object)
            .unwrap_or(0)
    }
}

pub(crate) fn class_status(object: u32) -> i32 {
    unsafe {
        let pools = core::ptr::read_volatile(crate::game::addr(OFF_OBJECT_POOLS) as *const usize);
        let pool = if pools == 0 { 0 } else { core::ptr::read_volatile((pools + ENEMY_POOL) as *const usize) };
        if pool == 0 {
            return NO_STATUS;
        }
        let objects = core::ptr::read_volatile((pool + POOL_OBJECTS) as *const usize);
        let count = core::ptr::read_volatile((pool + POOL_COUNT) as *const u32) as usize;
        for index in 0..count {
            let base = objects + index * OBJECT_STRIDE;
            if core::ptr::read_volatile((base + OBJECT_ID) as *const u32) != object {
                continue;
            }
            let holder = core::ptr::read_volatile((base + OBJECT_HOLDER) as *const usize);
            let instance = if holder == 0 { 0 } else { core::ptr::read_volatile((holder + HOLDER_INSTANCE) as *const usize) };
            if instance == 0 {
                return NO_STATUS;
            }
            return core::ptr::read_volatile((instance + INSTANCE_STATUS) as *const i32);
        }
        NO_STATUS
    }
}

pub(crate) fn install() {
    for (offset, expected) in [
        (OFF_STAGE_GATE, STAGE_GATE_OPCODE),
        (OFF_STAGE_KINDS, STAGE_KINDS_OPCODE),
        (OFF_REQUEST_KIND, REQUEST_KIND_OPCODE),
        (OFF_SPAWN, SPAWN_OPCODE),
        (OFF_REMOVE, REMOVE_OPCODE),
    ] {
        let observed = unsafe { crate::game::text_word(offset) };
        if observed != expected {
            skyline::println!("[enemy] REFUSED {offset:#x}: expected {expected:#010x}, found {observed:#010x}");
            return;
        }
    }
    skyline::install_hooks!(stage_gate, stage_kinds);
    ARMED.store(true, Ordering::Release);
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_arm(kinds: u32) {
    arm(kinds)
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_spawn(kind: u32, x: f32, y: f32, z: f32, lr: f32) -> u32 {
    spawn(kind, x, y, z, lr)
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_start(request: u32) -> u32 {
    start(request)
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_object(index: u32) -> u32 {
    object(index)
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_class_status(object: u32) -> i32 {
    class_status(object)
}

#[no_mangle]
pub extern "C" fn clone_engine_enemy_remove(object: u32) -> u32 {
    u32::from(remove(object))
}
