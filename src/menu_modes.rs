use dionysus_core::menu::Mode;
use core::ffi::c_char;
use core::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const CONFIG_PATH: &str = "sd:/ultimate/dionysus/config.toml";
const BUTTON_KEY: &str = "modes_button";
const DEFAULT_BUTTON: &str = "x";
const HUB_BUTTONS: [(&str, u64); 7] = [
    ("x", 1 << 2),
    ("lstick", 1 << 4),
    ("rstick", 1 << 5),
    ("zl", 1 << 8),
    ("zr", 1 << 9),
    ("plus", 1 << 10),
    ("minus", 1 << 11),
];
const NO_BUTTON: &str = "none";
const NPAD_PLAYERS: u32 = 8;
const NPAD_HANDHELD: u32 = 0x20;
const STYLE_FULL_KEY: u32 = 0x1;
const STYLE_HANDHELD: u32 = 0x2;
const STYLE_JOY_DUAL: u32 = 0x4;
const STYLE_GC: u32 = 0x20;
const NONE: i64 = -1;

struct Entry {
    mode: Mode,
    code: u32,
}

static MODES: OnceLock<Vec<Entry>> = OnceLock::new();
static HUB_BUTTON: OnceLock<u64> = OnceLock::new();
static LAST_HELD: AtomicU64 = AtomicU64::new(0);
static RESUME: AtomicI64 = AtomicI64::new(NONE);
static VALUES: Mutex<Vec<i64>> = Mutex::new(Vec::new());

fn modes() -> &'static [Entry] {
    MODES.get().map(Vec::as_slice).unwrap_or(&[])
}

fn entry(index: i64) -> Option<&'static Entry> {
    usize::try_from(index).ok().and_then(|index| modes().get(index))
}

fn configured_button() -> u64 {
    crate::upgrade::carry_old_folder();
    let text = std::fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let value = text
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| key.trim() == BUTTON_KEY)
        .map(|(_, value)| value.trim().trim_matches('"').trim().to_ascii_lowercase())
        .unwrap_or_else(|| DEFAULT_BUTTON.to_string());
    if value == NO_BUTTON {
        skyline::println!("[menumode] {BUTTON_KEY} = \"none\": no button opens the modes page");
        return 0;
    }
    match HUB_BUTTONS.iter().find(|(name, _)| *name == value) {
        Some((name, mask)) => {
            skyline::println!("[menumode] {name} opens the modes page");
            *mask
        }
        None => {
            skyline::println!(
                "[menumode] {CONFIG_PATH}: {BUTTON_KEY} = \"{value}\" is not one of x, zl, zr, plus, minus, lstick, rstick, none; using {DEFAULT_BUTTON}"
            );
            HUB_BUTTONS[0].1
        }
    }
}

fn hub_button() -> u64 {
    *HUB_BUTTON.get_or_init(configured_button)
}

pub(crate) fn settle(list: Vec<(Mode, u32)>) {
    for (index, (mode, code)) in list.iter().enumerate() {
        let does = match (mode.script.as_deref(), *code) {
            (Some(script), 0) => format!("runs {script}"),
            (Some(script), code) => format!("runs {script}, starts {code:#010x}"),
            (None, code) => format!("starts {code:#010x}"),
        };
        skyline::println!("[menumode] {index}: {} \"{}\" {does}", mode.id, mode.name);
    }
    if let Ok(mut values) = VALUES.lock() {
        values.resize(list.len(), 0);
    }
    let _ = MODES.set(list.into_iter().map(|(mode, code)| Entry { mode, code }).collect());
    let _ = hub_button();
}

pub(crate) fn chosen(index: usize) {
    if index < modes().len() {
        RESUME.store(index as i64, Ordering::Release);
    }
}

pub(crate) fn buttons() -> u64 {
    use skyline::nn::hid;
    let mut held = 0u64;
    unsafe {
        let mut state: hid::NpadHandheldState = core::mem::zeroed();
        let mut gc: hid::NpadGcState = core::mem::zeroed();
        for id in (0..NPAD_PLAYERS).chain(core::iter::once(NPAD_HANDHELD)) {
            let style = hid::GetNpadStyleSet(&id).flags;
            if style & STYLE_FULL_KEY != 0 {
                hid::GetNpadFullKeyState(&mut state, &id);
                held |= state.Buttons;
            }
            if style & STYLE_HANDHELD != 0 {
                hid::GetNpadHandheldState(&mut state, &id);
                held |= state.Buttons;
            }
            if style & STYLE_JOY_DUAL != 0 {
                hid::GetNpadJoyDualState(&mut state, &id);
                held |= state.Buttons;
            }
            if style & STYLE_GC != 0 {
                hid::GetNpadGcState(&mut gc, &id);
                held |= gc.Buttons;
            }
        }
    }
    held
}

fn pressed() -> i64 {
    let mask = hub_button();
    let held = buttons();
    let before = LAST_HELD.swap(held, Ordering::AcqRel);
    i64::from(mask != 0 && held & mask != 0 && before & mask == 0)
}

fn value(index: i64) -> i64 {
    let Ok(index) = usize::try_from(index) else {
        return 0;
    };
    VALUES.lock().ok().and_then(|values| values.get(index).copied()).unwrap_or(0)
}

fn set_value(index: i64, value: i64) -> i64 {
    let Ok(index) = usize::try_from(index) else {
        return NONE;
    };
    match VALUES.lock() {
        Ok(mut values) if index < values.len() => {
            values[index] = value;
            0
        }
        _ => NONE,
    }
}

pub(crate) fn call(name: &str, args: &[i64]) -> Option<i64> {
    let arg = |index: usize| args.get(index).copied().unwrap_or(NONE);
    match name {
        "engine.mode_count" => Some(modes().len() as i64),
        "engine.mode_sequence" => Some(entry(arg(0)).map_or(0, |entry| i64::from(entry.code))),
        "engine.mode_pressed" => Some(pressed()),
        "engine.mode_take_resume" => Some(RESUME.swap(NONE, Ordering::AcqRel)),
        "engine.mode_return_to" => Some(match entry(arg(0)) {
            Some(_) => {
                RESUME.store(arg(0), Ordering::Release);
                0
            }
            None => NONE,
        }),
        "engine.mode_value" => Some(value(arg(0))),
        "engine.mode_set_value" => Some(set_value(arg(0), args.get(1).copied().unwrap_or(0))),
        "engine.buttons" => Some(buttons() as i64),
        _ => None,
    }
}

pub(crate) fn text(name: &str, index: i64) -> Option<String> {
    let mode = &entry(index)?.mode;
    match name {
        "engine.mode_id" => Some(mode.id.clone()),
        "engine.mode_name" => Some(mode.name.clone()),
        "engine.mode_description" => Some(mode.description.clone()),
        "engine.mode_script" => mode.script.clone(),
        "engine.mode_button" => mode.button_root(),
        _ => None,
    }
}

pub(crate) fn id_of(index: usize) -> Option<&'static str> {
    modes().get(index).map(|entry| entry.mode.id.as_str())
}

pub(crate) fn sequence_of(id: &str) -> u32 {
    modes().iter().find(|entry| entry.mode.id == id).map_or_else(
        || crate::menu_sequences::code_of(dionysus_core::menu::call_hash(id) as u64).unwrap_or(0),
        |entry| entry.code,
    )
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_mode_sequence(id: *const c_char) -> u32 {
    if id.is_null() {
        return 0;
    }
    match core::ffi::CStr::from_ptr(id).to_str() {
        Ok(id) => sequence_of(id),
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_mode_count() -> u32 {
    modes().len() as u32
}
