use dionysus_core::menu;
use core::ffi::c_char;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::ffi::CString;
use std::sync::Mutex;

const OFF_SET_FUNCS: usize = 0x38f8f60;
const SET_FUNCS_OPCODE: u32 = 0xd102c3ff;
const OFF_PUSH_STRING: usize = 0x38f4570;
const PUSH_STRING_OPCODE: u32 = 0xa9bc5ff8;
const MODE_CALLS: &str = "engine.mode";
const BUTTONS_CALL: &str = "engine.buttons";
const OFF_MAIN_MENU_FUNCS: usize = 0x50ab630;
const OFF_SCRIPT_INVOKE: usize = 0x33747a0;
const STATE_TOP: usize = 0x10;
const STATE_CALL: usize = 0x20;
const CALL_FUNCTION: usize = 0;
const VALUE_SIZE: usize = 0x10;
const VALUE_TAG: usize = 8;
const STRING_SHORT_LENGTH: usize = 0xb;
const STRING_LONG_LENGTH: usize = 0x10;
const STRING_DATA: usize = 0x18;
const ENGINE_PREFIX: &str = "engine.";
const LOG_CALL: &str = "engine.log";
const MOVIE_PLAY_CALL: &str = "engine.movie_play";
const MOVIE_PATH_LIMIT: usize = 0x3e;
const LOG_BUDGET: u32 = 256;
const LOG_LINE_BUDGET: u32 = 512;
const LOG_TEXT_LIMIT: usize = 240;
const NAME_ARGUMENT_LIMIT: usize = 128;

pub(crate) type CallFn = unsafe extern "C" fn(*const i64, usize) -> i64;
type PushStringFn = unsafe extern "C" fn(usize, *const c_char) -> *const c_char;

#[repr(C)]
struct LuaReg {
    name: *const c_char,
    function: Option<unsafe extern "C" fn(usize) -> i32>,
}

unsafe impl Sync for LuaReg {}

static ENGINE_FUNCS: [LuaReg; 3] = [
    LuaReg {
        name: b"engine_call\0".as_ptr() as *const c_char,
        function: Some(engine_call),
    },
    LuaReg {
        name: b"engine_text\0".as_ptr() as *const c_char,
        function: Some(engine_text),
    },
    LuaReg {
        name: core::ptr::null(),
        function: None,
    },
];

static PUSH_STRING: AtomicBool = AtomicBool::new(false);

static CALLS: Mutex<Vec<(String, CallFn)>> = Mutex::new(Vec::new());
static REGISTERED_LOGS: AtomicU32 = AtomicU32::new(0);
static SCENE_LOGS: AtomicU32 = AtomicU32::new(0);
static CALL_LOGS: AtomicU32 = AtomicU32::new(0);
static LAST_ANSWERS: Mutex<Vec<(String, Option<i64>)>> = Mutex::new(Vec::new());
static LOG_LINES: AtomicU32 = AtomicU32::new(0);

pub(crate) fn register(name: &str, handler: CallFn) -> Result<(), i32> {
    if !menu::valid_call_name(name) || name.starts_with(ENGINE_PREFIX) {
        return Err(-1);
    }
    let Ok(mut calls) = CALLS.lock() else {
        return Err(-4);
    };
    if let Some(entry) = calls.iter_mut().find(|(known, _)| known == name) {
        entry.1 = handler;
        return Ok(());
    }
    if calls.len() >= menu::MAX_CALLS {
        return Err(-4);
    }
    calls.push((name.to_string(), handler));
    Ok(())
}

fn answer_changed(name: &str, answer: Option<i64>) -> bool {
    let Ok(mut answers) = LAST_ANSWERS.lock() else {
        return true;
    };
    match answers.iter_mut().find(|(known, _)| known == name) {
        Some(entry) if entry.1 == answer => false,
        Some(entry) => {
            entry.1 = answer;
            true
        }
        None => {
            if answers.len() < menu::MAX_CALLS * 2 {
                answers.push((name.to_string(), answer));
            }
            true
        }
    }
}

fn handler(name: &str) -> Option<CallFn> {
    CALLS
        .lock()
        .ok()
        .and_then(|calls| calls.iter().find(|(known, _)| known == name).map(|(_, handler)| *handler))
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn string_at(value: usize) -> Option<String> {
    text_at(value, menu::MAX_CALL_NAME)
}

unsafe fn text_at(value: usize, limit: usize) -> Option<String> {
    let tag: u32 = read(value + VALUE_TAG);
    if !menu::is_lua_string(tag & 0x3f) {
        return None;
    }
    let object: usize = read(value);
    if object == 0 {
        return None;
    }
    let length = if tag & 0x3f == menu::LUA_TSHRSTR {
        usize::from(read::<u8>(object + STRING_SHORT_LENGTH))
    } else {
        read::<usize>(object + STRING_LONG_LENGTH)
    };
    if length == 0 || length > limit {
        return None;
    }
    let bytes = core::slice::from_raw_parts((object + STRING_DATA) as *const u8, length);
    core::str::from_utf8(bytes).ok().map(str::to_string)
}

fn builtin(name: &str, args: &[i64]) -> Option<i64> {
    let arg = |index: usize| args.get(index).copied().unwrap_or(0);
    let outcome = |result: Result<(), crate::menu_battle::Refusal>| {
        i64::from(result.map_or_else(|refusal| refusal.code(), |()| 0))
    };
    match name {
        "engine.active_sequence" => Some(i64::from(crate::menu_sequences::active())),
        "engine.spirit_board_count" => Some(crate::spirits::board_count()),
        "engine.battle_player" => Some(outcome(crate::menu_battle::set_player(
            arg(0) as u32,
            menu::named(arg(1), menu::PLAYER_TYPES) as u32,
            arg(2) as u64,
            arg(3) as i32,
        ))),
        "engine.battle_stage" => Some(outcome(crate::menu_battle::set_stage(arg(0) as u64))),
        "engine.battle_rules" => Some(outcome(crate::menu_battle::set_rules(
            menu::named(arg(0), menu::RULE_KINDS) as u32,
            arg(1).clamp(0, i64::from(u32::MAX)) as u32,
            arg(2).clamp(0, i64::from(u32::MAX)) as u32,
            arg(3).clamp(0, i64::from(u32::MAX)) as u32,
        ))),
        "engine.battle_damage" => Some(outcome(crate::menu_battle::set_damage(
            arg(0) as u32,
            arg(1).clamp(0, i64::from(u32::MAX)) as u32,
        ))),
        "engine.battle_color" => Some(outcome(crate::menu_battle::set_color(
            arg(0) as u32,
            arg(1).clamp(0, i64::from(u32::MAX)) as u32,
        ))),
        "engine.battle_entrance" => Some(outcome(crate::menu_battle::set_entrance(arg(0) != 0))),
        "engine.battle_boss" => Some(outcome(crate::menu_battle::set_boss_named(arg(0)))),
        "engine.sequence" => Some(i64::from(crate::menu_sequences::code_of(arg(0) as u64).unwrap_or(0))),
        "engine.battle_ready_go" => Some(outcome(crate::menu_battle::set_ready_go(arg(0) != 0))),
        "engine.battle_pause" => Some(outcome(crate::menu_battle::set_pause(arg(0) != 0))),
        "engine.movie_state" => Some(i64::from(crate::menu_movie::state())),
        "engine.movie_stop" => Some(i64::from(crate::menu_movie::stop())),
        "engine.movie_keep" => Some(i64::from(crate::menu_movie::keep())),
        "engine.movie_status" => Some(i64::from(crate::menu_movie::status())),
        "engine.movie_hold" => Some(i64::from(crate::menu_movie::hold())),
        "engine.movie_pause" => Some(i64::from(crate::menu_movie::pause())),
        "engine.movie_resume" => Some(i64::from(crate::menu_movie::resume())),
        "engine.loading_icon_only" => Some(i64::from(crate::menu_loading::icon_only())),
        "engine.viewer_begin" => Some(crate::menu_viewer::begin(arg(0))),
        "engine.viewer_state" => Some(crate::menu_viewer::state(arg(0))),
        "engine.viewer_end" => Some(crate::menu_viewer::end()),
        "engine.gallery_count" => Some(crate::menu_viewer::item_count()),
        "engine.gallery_costume" => Some(crate::menu_viewer::item_costume(arg(0))),
        _ if name.starts_with(MODE_CALLS) || name == BUTTONS_CALL => crate::menu_modes::call(name, args),
        _ => None,
    }
}

unsafe fn describe(value: usize) -> String {
    let tag: u32 = read(value + VALUE_TAG);
    if menu::is_lua_string(tag & 0x3f) {
        return text_at(value, LOG_TEXT_LIMIT).unwrap_or_else(|| "<string>".to_string());
    }
    match tag {
        menu::LUA_TBOOLEAN => (read::<u32>(value) != 0).to_string(),
        menu::LUA_TNIL => "nil".to_string(),
        _ => menu::lua_integer(tag, read(value))
            .map(|number| number.to_string())
            .unwrap_or_else(|| format!("<type {tag:#x}>")),
    }
}

unsafe fn log_values(base: usize, count: usize) {
    if LOG_LINES.fetch_add(1, Ordering::Relaxed) >= LOG_LINE_BUDGET {
        return;
    }
    let words: Vec<String> = (1..count).map(|index| describe(base + index * VALUE_SIZE)).collect();
    skyline::println!("[menulog] {}", words.join(" "));
}

unsafe extern "C" fn engine_call(state: usize) -> i32 {
    let top: usize = read(state + STATE_TOP);
    let call: usize = read(state + STATE_CALL);
    let function: usize = read(call + CALL_FUNCTION);
    let base = function + VALUE_SIZE;
    let count = top.saturating_sub(base) / VALUE_SIZE;
    if count > 0 && string_at(base).as_deref() == Some(LOG_CALL) {
        log_values(base, count);
        core::ptr::write_volatile((top + VALUE_TAG) as *mut u32, menu::LUA_TNIL);
        core::ptr::write_volatile((state + STATE_TOP) as *mut usize, top + VALUE_SIZE);
        return 1;
    }
    let result = if count == 0 {
        None
    } else if string_at(base).as_deref() == Some(MOVIE_PLAY_CALL) {
        let path = if count > 1 { text_at(base + VALUE_SIZE, MOVIE_PATH_LIMIT) } else { None };
        let track = if count > 2 {
            let value = base + 2 * VALUE_SIZE;
            menu::lua_integer(read(value + VALUE_TAG), read(value)).unwrap_or(0)
        } else {
            0
        };
        let answer = match path {
            Some(path) => crate::menu_movie::play(&path, track.clamp(0, i64::from(u32::MAX)) as u32)
                .map_or_else(|refusal| refusal.code(), |()| 0),
            None => crate::menu_movie::Refusal::Path.code(),
        };
        skyline::println!("[menucall] {MOVIE_PLAY_CALL} -> {answer}");
        Some(i64::from(answer))
    } else {
        string_at(base).and_then(|name| {
            let mut args = [0i64; menu::MAX_CALL_ARGS];
            let used = (count - 1).min(menu::MAX_CALL_ARGS);
            for (index, slot) in args.iter_mut().enumerate().take(used) {
                let value = base + (index + 1) * VALUE_SIZE;
                *slot = match text_at(value, NAME_ARGUMENT_LIMIT) {
                    Some(text) => menu::call_hash(&text),
                    None => menu::lua_integer(read(value + VALUE_TAG), read(value)).unwrap_or(0),
                };
            }
            let answer = builtin(&name, &args[..used]).or_else(|| {
                handler(&name).map(|handler| handler(args.as_ptr(), used))
            });
            if answer_changed(&name, answer) && CALL_LOGS.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
                skyline::println!(
                    "[menucall] {name}{:?} -> {}",
                    &args[..used],
                    answer.map_or("nil (no handler)".to_string(), |value| value.to_string())
                );
            }
            answer
        })
    };
    let slot = top;
    match result {
        Some(value) => {
            core::ptr::write_volatile(slot as *mut i64, value);
            core::ptr::write_volatile((slot + VALUE_TAG) as *mut u32, menu::LUA_TNUMINT);
        }
        None => core::ptr::write_volatile((slot + VALUE_TAG) as *mut u32, menu::LUA_TNIL),
    }
    core::ptr::write_volatile((state + STATE_TOP) as *mut usize, top + VALUE_SIZE);
    1
}

unsafe extern "C" fn engine_text(state: usize) -> i32 {
    let top: usize = read(state + STATE_TOP);
    let call: usize = read(state + STATE_CALL);
    let function: usize = read(call + CALL_FUNCTION);
    let base = function + VALUE_SIZE;
    let count = top.saturating_sub(base) / VALUE_SIZE;
    let text = if count == 0 || !PUSH_STRING.load(Ordering::Acquire) {
        None
    } else {
        string_at(base).and_then(|name| {
            let index = if count > 1 {
                let value = base + VALUE_SIZE;
                menu::lua_integer(read(value + VALUE_TAG), read(value)).unwrap_or(-1)
            } else {
                -1
            };
            crate::menu_modes::text(&name, index)
                .or_else(|| crate::menu_viewer::text(&name, index))
                .or_else(|| crate::spirits::board_text(&name, index))
        })
    };
    match text.and_then(|text| CString::new(text).ok()) {
        Some(text) => {
            let push: PushStringFn = core::mem::transmute(crate::game::addr(OFF_PUSH_STRING));
            push(state, text.as_ptr());
        }
        None => {
            core::ptr::write_volatile((top + VALUE_TAG) as *mut u32, menu::LUA_TNIL);
            core::ptr::write_volatile((state + STATE_TOP) as *mut usize, top + VALUE_SIZE);
        }
    }
    1
}

unsafe fn is_script_player_list(list: usize) -> bool {
    list != 0 && read::<usize>(list + core::mem::size_of::<usize>()) == crate::game::addr(OFF_SCRIPT_INVOKE)
}

#[skyline::hook(offset = crate::game::rel(OFF_SET_FUNCS))]
unsafe fn set_funcs(state: usize, list: usize, upvalues: i32) {
    call_original!(state, list, upvalues);
    if upvalues != 0 {
        return;
    }
    if list == crate::game::addr(OFF_MAIN_MENU_FUNCS) {
        call_original!(state, ENGINE_FUNCS.as_ptr() as usize, 0);
        if REGISTERED_LOGS.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
            skyline::println!("[menucall] prg_func.engine_call and engine_text registered for the main menu");
        }
    } else if is_script_player_list(list) {
        call_original!(state, ENGINE_FUNCS.as_ptr() as usize, 0);
        if SCENE_LOGS.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
            skyline::println!("[menucall] UiScriptPlayer.engine_call and engine_text registered for a menu script");
        }
    }
}

pub(crate) fn install() {
    let observed = unsafe { crate::game::text_word(OFF_SET_FUNCS) };
    if observed != SET_FUNCS_OPCODE {
        skyline::println!(
            "[menucall] REFUSED luaL_setfuncs {OFF_SET_FUNCS:#x}: expected {SET_FUNCS_OPCODE:#010x}, found {observed:#010x}; prg_func.engine_call stays off"
        );
        return;
    }
    let push = unsafe { crate::game::text_word(OFF_PUSH_STRING) };
    if push == PUSH_STRING_OPCODE {
        PUSH_STRING.store(true, Ordering::Release);
    } else {
        skyline::println!(
            "[menucall] REFUSED lua_pushstring {OFF_PUSH_STRING:#x}: expected {PUSH_STRING_OPCODE:#010x}, found {push:#010x}; prg_func.engine_text answers nil"
        );
    }
    skyline::install_hook!(set_funcs);
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_register_menu_call(name: *const c_char, handler: Option<CallFn>) -> i32 {
    let Some(handler) = handler else {
        return -2;
    };
    if name.is_null() {
        return -1;
    }
    let Ok(name) = core::ffi::CStr::from_ptr(name).to_str() else {
        return -1;
    };
    match register(name, handler) {
        Ok(()) => 0,
        Err(code) => code,
    }
}
