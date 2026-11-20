use dionysus_core::command_input::{self as input, Command, Frame, Recognizer, Settings, StickRange, Tuning, SECTORS};
use dionysus_core::fighter_common_copies::{ACCESSOR_PARAM_OBJECT, COPIES};
use dionysus_core::fighter_param_row::Op;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use smash::app::lua_bind::{ControlModule, PostureModule, SlowModule, StopModule, WorkModule};
use smash::app::{sv_math, BattleObjectModuleAccessor};
use smash::lib::lua_const::*;
use std::sync::{Mutex, RwLock};

const OFF_EXEC_COMMAND: usize = 0x6bac10;
const FLAG_CAN_SPECIAL_COMMAND: i32 = 0x2000_0085;
const CONTROL_BOMA: usize = 0x8;
const CONTROL_COMMON: usize = 0x140;
const CONTROL_LIFE: usize = 0xab4;
const CONTROL_SUPER_LIFE: usize = 0xab5;
const CONTROL_STICK_RANGE: usize = 0xad8;
const COMMON_PRECEDE: usize = 0x8;
const COMMON_LIFE: usize = 0x334;
const COMMON_BUTTON_LIFE: usize = 0x335;
const COMMON_THRESHOLD: usize = 0x338;
const COMMON_SUPER_LIFE: usize = 0x33c;
const COMMON_STICK_RANGE: usize = 0x340;
const BOMA_OBJECT_ID: usize = 0x8;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const LOG_BUDGET: u32 = 48;

#[repr(C)]
struct StickVector {
    begin: *const [f32; 2],
    end: *const [f32; 2],
    capacity: *const [f32; 2],
    pairs: StickRange,
}

struct Registered {
    kind: i32,
    vanilla: bool,
    stick: usize,
    cpu: bool,
    commands: Vec<(Command, Vec<u8>)>,
}

struct State {
    object: u32,
    kind: i32,
    recognizer: Recognizer,
}

static REGISTERED: RwLock<Vec<Registered>> = RwLock::new(Vec::new());
static ANY: AtomicBool = AtomicBool::new(false);
static STATES: Mutex<Vec<State>> = Mutex::new(Vec::new());
static LOG: AtomicU32 = AtomicU32::new(0);

fn log(message: &str) {
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        crate::dbg_log_public(message);
    }
}

fn leak_stick_vector(range: StickRange) -> usize {
    let vector = Box::leak(Box::new(StickVector {
        begin: core::ptr::null(),
        end: core::ptr::null(),
        capacity: core::ptr::null(),
        pairs: range,
    }));
    let begin = vector.pairs.as_ptr();
    vector.begin = begin;
    vector.end = unsafe { begin.add(SECTORS) };
    vector.capacity = vector.end;
    vector as *const StickVector as usize
}

pub(crate) fn register(kind: i32, settings: &Settings, commands: &[Command]) -> Result<Option<String>, String> {
    if settings.is_empty() && commands.is_empty() {
        return Ok(None);
    }
    let mut compiled = Vec::new();
    for command in commands {
        command.check()?;
        compiled.push((command.clone(), command.steps()?));
    }
    let mut queued = 0;
    for (name, value) in settings.common_overrides() {
        let op = if name == "special_command_neutral_threshold" { Op::Set(value) } else { Op::SetInt(value as i32) };
        let recorded = crate::fighter_common_copies::record(
            kind,
            crate::param_overrides::ANY_SLOT,
            dionysus_core::hash40(name),
            op,
        );
        if recorded.recorded {
            queued += 1;
        } else {
            return Err(format!("{name}: {}", recorded.message));
        }
    }
    let stick = settings.stick_range.map(leak_stick_vector).unwrap_or(0);
    let mut registered = REGISTERED.write().map_err(|_| "busy".to_string())?;
    registered.retain(|entry| entry.kind != kind);
    let cpu = input::wants_roll(&compiled);
    registered.push(Registered { kind, vanilla: settings.vanilla, stick, cpu, commands: compiled });
    ANY.store(true, Ordering::Release);
    let names: Vec<String> = commands
        .iter()
        .map(|command| match command.cpu {
            chance if chance > 0.0 => format!("{} (CPUs {:.0}%)", command.name, chance * 100.0),
            _ => command.name.clone(),
        })
        .collect();
    Ok(Some(format!(
        "vanilla commands {}, {queued} tuning value(s), {} stick range, own commands [{}]",
        if settings.vanilla { "on" } else { "off" },
        if stick == 0 { "common" } else { "own" },
        names.join(", ")
    )))
}

fn plausible(address: usize) -> bool {
    address >= LOWEST_PLAUSIBLE_POINTER && address & 7 == 0
}

unsafe fn read_usize(address: usize) -> usize {
    core::ptr::read_volatile(address as *const usize)
}

unsafe fn read_u8(address: usize) -> u8 {
    core::ptr::read_volatile(address as *const u8)
}

unsafe fn own_common(boma: usize) -> usize {
    let param_object = read_usize(boma + ACCESSOR_PARAM_OBJECT);
    if plausible(param_object) {
        param_object + COPIES[0].base
    } else {
        0
    }
}

unsafe fn prepare(control: usize, boma: *mut BattleObjectModuleAccessor, entry: &Registered) -> usize {
    let common = own_common(boma as usize);
    if common != 0 {
        if read_usize(control + CONTROL_COMMON) != common {
            core::ptr::write_volatile((control + CONTROL_COMMON) as *mut usize, common);
            log(&format!(
                "[commands] kind {}: the control module reads the fighter's own common.prc copy {common:#x}",
                entry.kind
            ));
        }
        core::ptr::write_volatile((control + CONTROL_LIFE) as *mut u8, read_u8(common + COMMON_LIFE));
        core::ptr::write_volatile((control + CONTROL_SUPER_LIFE) as *mut u8, read_u8(common + COMMON_SUPER_LIFE));
    }
    if entry.stick != 0 && read_usize(control + CONTROL_STICK_RANGE) != entry.stick {
        core::ptr::write_volatile((control + CONTROL_STICK_RANGE) as *mut usize, entry.stick);
    }
    if entry.vanilla && !WorkModule::is_flag(boma, FLAG_CAN_SPECIAL_COMMAND) {
        WorkModule::on_flag(boma, FLAG_CAN_SPECIAL_COMMAND);
        ControlModule::reset_special_command(boma, true);
        log(&format!(
            "[commands] kind {}: the game's own command inputs are on (FIGHTER_PAD_CMD_CAT4_FLAG_*)",
            entry.kind
        ));
    }
    common
}

unsafe fn tuning(common: usize) -> Tuning {
    if common == 0 {
        return input::VANILLA;
    }
    Tuning {
        life: read_u8(common + COMMON_LIFE),
        super_life: read_u8(common + COMMON_SUPER_LIFE),
        button_life: read_u8(common + COMMON_BUTTON_LIFE),
        neutral_threshold: core::ptr::read_volatile((common + COMMON_THRESHOLD) as *const f32),
        precede: core::ptr::read_volatile((common + COMMON_PRECEDE) as *const u32).min(u8::MAX as u32) as u8,
    }
}

unsafe fn stick_range(entry: &Registered, common: usize) -> StickRange {
    if entry.stick != 0 {
        return (*(entry.stick as *const StickVector)).pairs;
    }
    if common == 0 {
        return input::VANILLA_STICK_RANGE;
    }
    let begin = read_usize(common + COMMON_STICK_RANGE);
    let end = read_usize(common + COMMON_STICK_RANGE + 8);
    if !plausible(begin & !3) || end.wrapping_sub(begin) != SECTORS * 8 {
        return input::VANILLA_STICK_RANGE;
    }
    core::ptr::read_unaligned(begin as *const StickRange)
}

unsafe fn computer_roll(boma: *mut BattleObjectModuleAccessor, entry: &Registered, frame: &Frame) -> Option<f32> {
    if !(frame.attack || frame.special) || !entry.cpu {
        return None;
    }
    let player = WorkModule::get_int(boma, *FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID);
    if !crate::entry_groups::entry_is_cpu(player) {
        return None;
    }
    Some(sv_math::randf(dionysus_core::hash40("fighter"), 1.0).clamp(0.0, 1.0))
}

unsafe fn recognise(boma: *mut BattleObjectModuleAccessor, entry: &Registered, common: usize) {
    let tuning = tuning(common);
    let range = stick_range(entry, common);
    let mut frame = Frame {
        direction: input::direction(
            ControlModule::get_stick_x(boma),
            ControlModule::get_stick_y(boma),
            tuning.neutral_threshold,
            &range,
        ),
        facing: PostureModule::lr(boma),
        attack: ControlModule::check_button_trigger(boma, *CONTROL_PAD_BUTTON_ATTACK),
        special: ControlModule::check_button_trigger(boma, *CONTROL_PAD_BUTTON_SPECIAL),
        frozen: StopModule::is_hit(boma) || SlowModule::mag(boma, 1) != 1,
        roll: None,
    };
    frame.roll = computer_roll(boma, entry, &frame);
    let object = core::ptr::read_volatile((boma as usize + BOMA_OBJECT_ID) as *const u32);
    let Ok(mut states) = STATES.lock() else {
        return;
    };
    let index = match states.iter().position(|state| state.object == object) {
        Some(index) => index,
        None => {
            states.push(State { object, kind: entry.kind, recognizer: Recognizer::new() });
            states.len() - 1
        }
    };
    let state = &mut states[index];
    if state.kind != entry.kind {
        state.kind = entry.kind;
        state.recognizer.reset();
    }
    let bits = state.recognizer.update(&entry.commands, &tuning, frame);
    if bits == 0 {
        return;
    }
    for (index, (command, _)) in entry.commands.iter().enumerate() {
        if bits & (1 << index) != 0 {
            let lr = state.recognizer.ready(index).unwrap_or(0.0);
            log(&format!(
                "[commands] kind {} object {object:#x}: {} ({}) read, pointing {}{}",
                entry.kind,
                command.name,
                command.input,
                if lr * frame.facing >= 0.0 { "the way it faces" } else { "behind it" },
                if frame.roll.is_some() { ", on a CPU's press" } else { "" }
            ));
        }
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_EXEC_COMMAND))]
unsafe fn exec_command(control: u64, update: u64) -> u64 {
    if !ANY.load(Ordering::Acquire) || control == 0 {
        return call_original!(control, update);
    }
    let boma = read_usize(control as usize + CONTROL_BOMA) as *mut BattleObjectModuleAccessor;
    let kind = if plausible(boma as usize) { crate::entry_groups::clone_of_boma(boma) } else { None };
    let Some(kind) = kind else {
        return call_original!(control, update);
    };
    let Ok(registered) = REGISTERED.read() else {
        return call_original!(control, update);
    };
    let Some(entry) = registered.iter().find(|entry| entry.kind == kind) else {
        drop(registered);
        return call_original!(control, update);
    };
    let common = prepare(control as usize, boma, entry);
    let result = call_original!(control, update);
    if !entry.commands.is_empty() {
        recognise(boma, entry, common);
    }
    result
}

pub(crate) fn forget_all() {
    if let Ok(mut states) = STATES.lock() {
        states.clear();
    }
}

unsafe fn with_command<R>(
    boma: *mut BattleObjectModuleAccessor,
    hash: u64,
    action: impl FnOnce(&mut Recognizer, usize) -> R,
) -> Option<R> {
    if !ANY.load(Ordering::Acquire) || !plausible(boma as usize) {
        return None;
    }
    let kind = crate::entry_groups::clone_of_boma(boma)?;
    let registered = REGISTERED.read().ok()?;
    let entry = registered.iter().find(|entry| entry.kind == kind)?;
    let index = entry.commands.iter().position(|(command, _)| command.hash() == hash)?;
    let object = core::ptr::read_volatile((boma as usize + BOMA_OBJECT_ID) as *const u32);
    let mut states = STATES.lock().ok()?;
    let state = states.iter_mut().find(|state| state.object == object && state.kind == kind)?;
    Some(action(&mut state.recognizer, index))
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_command_ready_v1(
    boma: *mut BattleObjectModuleAccessor,
    hash: u64,
    lr: *mut f32,
) -> bool {
    let Some(Some(direction)) = with_command(boma, hash, |recognizer, index| recognizer.ready(index)) else {
        return false;
    };
    if !lr.is_null() {
        *lr = direction;
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_command_clear_v1(boma: *mut BattleObjectModuleAccessor, hash: u64) -> bool {
    with_command(boma, hash, |recognizer, index| recognizer.clear(index)).is_some()
}

pub(crate) fn install() {
    skyline::install_hooks!(exec_command);
}
