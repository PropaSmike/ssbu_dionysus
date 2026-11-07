use core::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::ffi::{CStr, CString};
use std::sync::Mutex;

const OFF_SET_LABEL: usize = 0x377_9b70;
const SET_LABEL_OPCODE: u32 = 0xf81c_0ffc;
const ROW_LABEL_RETURN: usize = 0x13c_4890;
const CONFIRM_LABEL_RETURN: usize = 0x1ee_f838;
const OFF_PAUSE_DECIDE: usize = 0x13b_ee34;
const PAUSE_DECIDE_OPCODE: u32 = 0xf940_f668;
const OFF_PAUSE_STATE: usize = 0x13c_5cac;
const PAUSE_STATE_OPCODE: u32 = 0x9340_7d08;
const OFF_ROW_ICON: usize = 0x1b8_dfe0;
const OFF_ICON_TEXTURE: usize = 0x323_f1b0;
const OFF_SET_PICTURE: usize = 0x377_85d0;
const REPLACED_LABEL: &[u8] = b"mel_pause_con_connect";
const PAUSE_WIDGET: usize = 0x1e8;
const PAUSE_RESULT: usize = 0x18;
const WIDGET_ROWS: usize = 0x80;
const ROWS_LIST: usize = 0x20;
const LIST_CURSOR: usize = 0x25c;
const ROW_SIZE: usize = 0x48;
const ROW_KIND: usize = 0x78;
const ROW_PICTURE: usize = 0x88;
const ROW_LABEL: usize = 0x90;
const ROW_COUNT: usize = 5;
const KIND_CONNECT: u32 = 3;
const KIND_QUIT: u32 = 4;
const RESULT_QUIT: u32 = 1;
const STATE_MENU: u32 = 2;
const STATE_CLOSING: u32 = 0xa;
const LABEL_LIMIT: usize = 0x40;

pub(crate) const CHOICE_NONE: i32 = 0;
pub(crate) const CHOICE_QUIT: i32 = 1;
pub(crate) const CHOICE_EXIT: i32 = 2;
pub(crate) const UNAVAILABLE: i32 = -1;
pub(crate) const BAD_LABEL: i32 = -2;

struct Exit {
    row: CString,
    confirm: Option<CString>,
}

static ARMED: AtomicBool = AtomicBool::new(false);
static EXIT: Mutex<Option<Exit>> = Mutex::new(None);
static EXIT_SLOT: AtomicUsize = AtomicUsize::new(0);
static CHOICE: AtomicI32 = AtomicI32::new(CHOICE_NONE);
static PENDING: AtomicI32 = AtomicI32::new(CHOICE_NONE);

type RowIconFn = unsafe extern "C" fn(u32) -> u64;
type IconTextureFn = unsafe extern "C" fn(u64, u64) -> u64;
type SetPictureFn = unsafe extern "C" fn(u64, u64) -> u64;

unsafe fn game<T>(offset: usize) -> T {
    core::mem::transmute_copy(&crate::game::addr(offset))
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn write<T>(address: usize, value: T) {
    core::ptr::write_volatile(address as *mut T, value);
}

fn label_of(text: &str) -> Result<CString, i32> {
    if text.is_empty() || text.len() >= LABEL_LIMIT {
        return Err(BAD_LABEL);
    }
    CString::new(text).map_err(|_| BAD_LABEL)
}

pub(crate) fn set_exit(labels: Option<(&str, Option<&str>)>) -> i32 {
    if !ARMED.load(Ordering::Acquire) {
        return UNAVAILABLE;
    }
    let exit = match labels {
        Some((row, confirm)) => {
            let row = match label_of(row) {
                Ok(label) => label,
                Err(code) => return code,
            };
            let confirm = match confirm.map(label_of).transpose() {
                Ok(label) => label,
                Err(code) => return code,
            };
            Some(Exit { row, confirm })
        }
        None => None,
    };
    let Ok(mut current) = EXIT.lock() else {
        return UNAVAILABLE;
    };
    match &exit {
        Some(own) => skyline::println!(
            "[battlepause] pause rows: {} replaces Connect Controller",
            own.row.to_string_lossy()
        ),
        None if current.is_some() => skyline::println!("[battlepause] pause rows: vanilla"),
        None => {}
    }
    *current = exit;
    0
}

pub(crate) fn choice() -> i32 {
    CHOICE.load(Ordering::Acquire)
}

unsafe fn label_bytes(label: *const u8) -> &'static [u8] {
    CStr::from_ptr(label as *const core::ffi::c_char).to_bytes()
}

unsafe fn door_icon(slot: usize, row: usize) {
    if row >= ROW_COUNT {
        return;
    }
    let rows = slot - row * ROW_SIZE;
    let icon = game::<RowIconFn>(OFF_ROW_ICON)(KIND_QUIT);
    let texture = game::<IconTextureFn>(OFF_ICON_TEXTURE)(read::<u64>(rows), icon);
    let picture: u64 = read(slot + ROW_PICTURE);
    if picture != 0 {
        game::<SetPictureFn>(OFF_SET_PICTURE)(picture, texture);
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_SET_LABEL))]
unsafe fn set_label(text: usize, label: *const u8) -> u64 {
    let lr: usize;
    let row: usize;
    core::arch::asm!("mov {}, x30", "mov {}, x25", out(reg) lr, out(reg) row, options(nomem, nostack, preserves_flags));
    if label.is_null() {
        return call_original!(text, label);
    }
    match crate::game::offset_of(lr) {
        ROW_LABEL_RETURN => {
            CHOICE.store(CHOICE_NONE, Ordering::Release);
            PENDING.store(CHOICE_NONE, Ordering::Release);
            if label_bytes(label) != REPLACED_LABEL {
                return call_original!(text, label);
            }
            EXIT_SLOT.store(0, Ordering::Release);
            let Ok(exit) = EXIT.lock() else {
                return call_original!(text, label);
            };
            let Some(own) = exit.as_ref().filter(|_| crate::menu_battle::active()) else {
                return call_original!(text, label);
            };
            let slot = text - ROW_LABEL;
            EXIT_SLOT.store(slot, Ordering::Release);
            let result = call_original!(text, own.row.as_ptr() as *const u8);
            door_icon(slot, row);
            result
        }
        CONFIRM_LABEL_RETURN if PENDING.load(Ordering::Acquire) == CHOICE_EXIT => {
            let Ok(exit) = EXIT.lock() else {
                return call_original!(text, label);
            };
            match exit.as_ref().and_then(|own| own.confirm.as_ref()) {
                Some(confirm) => call_original!(text, confirm.as_ptr() as *const u8),
                None => call_original!(text, label),
            }
        }
        _ => call_original!(text, label),
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_PAUSE_DECIDE), inline)]
unsafe fn pause_decide(ctx: &mut skyline::hooks::InlineCtx) {
    PENDING.store(CHOICE_NONE, Ordering::Release);
    let exit_slot = EXIT_SLOT.load(Ordering::Acquire);
    if exit_slot == 0 {
        return;
    }
    let pause = ctx.registers[19].x() as usize;
    let widget: usize = read(pause + PAUSE_WIDGET);
    if widget == 0 {
        return;
    }
    let rows: usize = read(widget + WIDGET_ROWS);
    let list: usize = read(rows + ROWS_LIST);
    if list == 0 {
        return;
    }
    let cursor: i32 = read(list + LIST_CURSOR);
    let chosen = usize::try_from(cursor)
        .ok()
        .filter(|row| *row < ROW_COUNT)
        .map(|row| rows + row * ROW_SIZE);
    if chosen == Some(exit_slot) {
        write::<u32>(exit_slot + ROW_KIND, KIND_QUIT);
        PENDING.store(CHOICE_EXIT, Ordering::Release);
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_PAUSE_STATE), inline)]
unsafe fn pause_state(ctx: &mut skyline::hooks::InlineCtx) {
    let exit_slot = EXIT_SLOT.load(Ordering::Acquire);
    if exit_slot != 0 {
        write::<u32>(exit_slot + ROW_KIND, KIND_CONNECT);
    }
    let state = ctx.registers[8].w();
    if state == STATE_MENU {
        PENDING.store(CHOICE_NONE, Ordering::Release);
        return;
    }
    let pause = ctx.registers[19].x() as usize;
    if state != STATE_CLOSING || read::<u32>(pause + PAUSE_RESULT) != RESULT_QUIT {
        return;
    }
    let choice = match PENDING.swap(CHOICE_NONE, Ordering::AcqRel) {
        CHOICE_EXIT => CHOICE_EXIT,
        _ => CHOICE_QUIT,
    };
    CHOICE.store(choice, Ordering::Release);
    let own_battle = choice == CHOICE_EXIT || crate::menu_sequences::active() != 0;
    let ended = own_battle && crate::menu_battle::end_battle().is_ok();
    skyline::println!(
        "[battlepause] pause ended the battle: {}{}",
        if choice == CHOICE_EXIT { "exit row" } else { "quit" },
        if ended { "; back to the menu" } else { "" }
    );
}

pub(crate) fn install() {
    let label = unsafe { crate::game::text_word(OFF_SET_LABEL) };
    let decide = unsafe { crate::game::text_word(OFF_PAUSE_DECIDE) };
    let state = unsafe { crate::game::text_word(OFF_PAUSE_STATE) };
    if label != SET_LABEL_OPCODE || decide != PAUSE_DECIDE_OPCODE || state != PAUSE_STATE_OPCODE {
        skyline::println!(
            "[battlepause] REFUSED pause hooks: found {label:#010x} / {decide:#010x} / {state:#010x}; pause rows stay vanilla"
        );
        return;
    }
    skyline::install_hooks!(set_label, pause_decide, pause_state);
    ARMED.store(true, Ordering::Release);
}

unsafe fn optional(label: *const core::ffi::c_char) -> Result<Option<&'static str>, i32> {
    if label.is_null() {
        return Ok(None);
    }
    CStr::from_ptr(label).to_str().map(Some).map_err(|_| BAD_LABEL)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_pause_exit(row: *const core::ffi::c_char, confirm: *const core::ffi::c_char) -> i32 {
    let (row, confirm) = match (optional(row), optional(confirm)) {
        (Ok(row), Ok(confirm)) => (row, confirm),
        _ => return BAD_LABEL,
    };
    set_exit(row.map(|row| (row, confirm)))
}

#[no_mangle]
pub extern "C" fn clone_engine_pause_choice() -> i32 {
    choice()
}
