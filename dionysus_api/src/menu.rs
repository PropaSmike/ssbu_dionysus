use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::AtomicUsize;

use crate::{resolve, Error};

pub use dionysus_core::menu::vanilla_code;

pub type MenuCall = unsafe extern "C" fn(*const i64, usize) -> i64;

type RegisterSequenceFn = unsafe extern "C" fn(u32, u32) -> i32;
type ActiveSequenceFn = unsafe extern "C" fn() -> u32;
type ModeSequenceFn = unsafe extern "C" fn(*const c_char) -> u32;
type RegisterCallFn = unsafe extern "C" fn(*const c_char, Option<MenuCall>) -> i32;
type BattlePlayerFn = unsafe extern "C" fn(u32, u32, u64, i32) -> i32;
type BattleStageFn = unsafe extern "C" fn(u64) -> i32;
type BattleDamageFn = unsafe extern "C" fn(u32, u32) -> i32;
type BattleColorFn = unsafe extern "C" fn(u32, u32) -> i32;
type BattleRulesFn = unsafe extern "C" fn(u32, u32, u32, u32) -> i32;
type BattleStockFn = unsafe extern "C" fn(u32) -> i32;
type BattleEntranceFn = unsafe extern "C" fn(u32) -> i32;
type BattleBossFn = unsafe extern "C" fn(i32) -> i32;
type EndBattleFn = unsafe extern "C" fn() -> i32;
type BattleSerialFn = unsafe extern "C" fn() -> u32;
type NextBattleFn = unsafe extern "C" fn() -> i32;
type MoviePlayFn = unsafe extern "C" fn(*const c_char, u32) -> i32;
type MovieStateFn = unsafe extern "C" fn() -> i32;
type PauseExitFn = unsafe extern "C" fn(*const c_char, *const c_char) -> i32;
type PauseChoiceFn = unsafe extern "C" fn() -> i32;
type KeepMusicFn = unsafe extern "C" fn(u32) -> i32;
type PlayBgmFn = unsafe extern "C" fn(*const c_char) -> i32;
type StopBgmFn = unsafe extern "C" fn() -> i32;
type ViewerSlotFn = unsafe extern "C" fn(u32) -> i32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PauseExit<'a> {
    pub row: &'a str,
    pub confirm: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PauseChoice {
    None,
    Quit,
    Exit,
}

static PAUSE_EXIT_FN: AtomicUsize = AtomicUsize::new(0);
static PAUSE_CHOICE_FN: AtomicUsize = AtomicUsize::new(0);
static KEEP_MUSIC_FN: AtomicUsize = AtomicUsize::new(0);
static PLAY_BGM_FN: AtomicUsize = AtomicUsize::new(0);
static STOP_BGM_FN: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PlayerType {
    Human = 0,
    Cpu = 1,
    Amiibo = 2,
    None = 3,
}

static REGISTER_SEQUENCE_FN: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_SEQUENCE_FN: AtomicUsize = AtomicUsize::new(0);
static MODE_SEQUENCE_FN: AtomicUsize = AtomicUsize::new(0);
static REGISTER_CALL_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_PLAYER_FN: AtomicUsize = AtomicUsize::new(0);
static FIGHTER_CHANGE_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_FRAME_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_STAGE_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_DAMAGE_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_COLOR_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_RULES_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_STOCK_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_ENTRANCE_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_BOSS_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_BOSS_OBJECT_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_BOSS_FINISHED_FN: AtomicUsize = AtomicUsize::new(0);
const BOSS_NO_OBJECT: u32 = 0x5000_0000;
static BATTLE_READY_GO_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_PAUSE_FN: AtomicUsize = AtomicUsize::new(0);
static BATTLE_SERIAL_FN: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum RuleKind {
    Time = 0,
    Stock = 1,
    Stamina = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rules {
    pub kind: RuleKind,
    pub stock: u32,
    pub seconds: u32,
    pub items: u32,
}
static END_BATTLE_FN: AtomicUsize = AtomicUsize::new(0);
static NEXT_BATTLE_FN: AtomicUsize = AtomicUsize::new(0);
static MOVIE_PLAY_FN: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STATE_FN: AtomicUsize = AtomicUsize::new(0);
static MOVIE_STOP_FN: AtomicUsize = AtomicUsize::new(0);
static VIEWER_BEGIN_FN: AtomicUsize = AtomicUsize::new(0);
static VIEWER_STATE_FN: AtomicUsize = AtomicUsize::new(0);
static VIEWER_END_FN: AtomicUsize = AtomicUsize::new(0);
static GALLERY_COUNT_FN: AtomicUsize = AtomicUsize::new(0);
static GALLERY_COSTUME_FN: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewerState {
    Loading,
    Built,
    Shown,
}

pub fn register_sequence(code: u32, runs_as: u32) -> Result<(), Error> {
    let address = resolve(&REGISTER_SEQUENCE_FN, b"clone_engine_register_sequence\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: RegisterSequenceFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(code, runs_as) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn active_sequence() -> Option<u32> {
    let address = resolve(&ACTIVE_SEQUENCE_FN, b"clone_engine_active_sequence\0")?;
    let function: ActiveSequenceFn = unsafe { std::mem::transmute(address) };
    match unsafe { function() } {
        0 => None,
        code => Some(code),
    }
}

pub fn mode_sequence(id: &str) -> Option<u32> {
    let id = CString::new(id).ok()?;
    let address = resolve(&MODE_SEQUENCE_FN, b"clone_engine_mode_sequence\0")?;
    let function: ModeSequenceFn = unsafe { std::mem::transmute(address) };
    match unsafe { function(id.as_ptr()) } {
        0 => None,
        code => Some(code),
    }
}

pub fn register_call(name: &str, handler: MenuCall) -> Result<(), Error> {
    let name = CString::new(name).map_err(|_| Error::InvalidName)?;
    let address = resolve(&REGISTER_CALL_FN, b"clone_engine_register_menu_call\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: RegisterCallFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(name.as_ptr(), Some(handler)) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_player(slot: u32, kind: PlayerType, ui_chara: &str, pad: i32) -> Result<(), Error> {
    let address = resolve(&BATTLE_PLAYER_FN, b"clone_engine_battle_player\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattlePlayerFn = unsafe { std::mem::transmute(address) };
    let hash = dionysus_core::hash40(&ui_chara.to_ascii_lowercase());
    let result = unsafe { function(slot, kind as u32, hash, pad) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn change_fighter(slot: u32, ui_chara: &str, color: u32) -> Result<(), Error> {
    let address = resolve(&FIGHTER_CHANGE_FN, b"clone_engine_fighter_change_v1\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: unsafe extern "C" fn(u32, u64, u32) -> i32 = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(slot, dionysus_core::hash40(&ui_chara.to_ascii_lowercase()), color) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_stage(ui_stage: &str) -> Result<(), Error> {
    let address = resolve(&BATTLE_STAGE_FN, b"clone_engine_battle_stage\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleStageFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(dionysus_core::hash40(&ui_stage.to_ascii_lowercase())) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_damage(slot: u32, percent: u32) -> Result<(), Error> {
    let address = resolve(&BATTLE_DAMAGE_FN, b"clone_engine_battle_damage\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleDamageFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(slot, percent) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_color(slot: u32, color: u32) -> Result<(), Error> {
    let address = resolve(&BATTLE_COLOR_FN, b"clone_engine_battle_color\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleColorFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(slot, color) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_rules(rules: Rules) -> Result<(), Error> {
    let address = resolve(&BATTLE_RULES_FN, b"clone_engine_battle_rules\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleRulesFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(rules.kind as u32, rules.stock, rules.seconds, rules.items) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_stock(slot: u32) -> Result<u32, Error> {
    let address = resolve(&BATTLE_STOCK_FN, b"clone_engine_battle_stock\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleStockFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(slot) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result as u32)
}

pub fn battle_entrance(play: bool) -> Result<(), Error> {
    let address = resolve(&BATTLE_ENTRANCE_FN, b"clone_engine_battle_entrance\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleEntranceFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(u32::from(play)) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_boss(kind: Option<i32>) -> Result<(), Error> {
    let address = resolve(&BATTLE_BOSS_FN, b"clone_engine_battle_boss\0").ok_or(Error::EngineUnavailable)?;
    let function: BattleBossFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(kind.unwrap_or(-1)) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_boss_object() -> Option<u32> {
    let address = resolve(&BATTLE_BOSS_OBJECT_FN, b"clone_engine_battle_boss_object\0")?;
    let function: BattleSerialFn = unsafe { std::mem::transmute(address) };
    let object = unsafe { function() };
    (object != BOSS_NO_OBJECT).then_some(object)
}

pub fn battle_boss_finished() -> bool {
    let Some(address) = resolve(&BATTLE_BOSS_FINISHED_FN, b"clone_engine_battle_boss_finished\0") else {
        return false;
    };
    let function: BattleSerialFn = unsafe { std::mem::transmute(address) };
    unsafe { function() != 0 }
}

pub fn battle_pause(allowed: bool) -> Result<(), Error> {
    let address = resolve(&BATTLE_PAUSE_FN, b"clone_engine_battle_pause\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleEntranceFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(u32::from(allowed)) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_ready_go(play: bool) -> Result<(), Error> {
    let address = resolve(&BATTLE_READY_GO_FN, b"clone_engine_battle_ready_go\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleEntranceFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(u32::from(play)) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn battle_serial() -> Result<u32, Error> {
    let address = resolve(&BATTLE_SERIAL_FN, b"clone_engine_battle_serial\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: BattleSerialFn = unsafe { std::mem::transmute(address) };
    Ok(unsafe { function() })
}

pub fn battle_frame() -> Result<u64, Error> {
    let address = resolve(&BATTLE_FRAME_FN, b"clone_engine_battle_frame_v1\0").ok_or(Error::EngineUnavailable)?;
    let function: unsafe extern "C" fn() -> u64 = unsafe { std::mem::transmute(address) };
    Ok(unsafe { function() })
}

pub fn end_battle() -> Result<(), Error> {
    let address = resolve(&END_BATTLE_FN, b"clone_engine_end_battle\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: EndBattleFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function() };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn next_battle() -> Result<(), Error> {
    let address = resolve(&NEXT_BATTLE_FN, b"clone_engine_next_battle\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: NextBattleFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function() };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn play_movie(path: &str, track: u32) -> Result<(), Error> {
    let address = resolve(&MOVIE_PLAY_FN, b"clone_engine_movie_play\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: MoviePlayFn = unsafe { std::mem::transmute(address) };
    let path = CString::new(path).map_err(|_| Error::Engine(-2))?;
    let result = unsafe { function(path.as_ptr(), track) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn movie_playing() -> Result<bool, Error> {
    let address = resolve(&MOVIE_STATE_FN, b"clone_engine_movie_state\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: MovieStateFn = unsafe { std::mem::transmute(address) };
    Ok(unsafe { function() } != 0)
}

pub fn pause_exit(exit: Option<PauseExit>) -> Result<(), Error> {
    let address = resolve(&PAUSE_EXIT_FN, b"clone_engine_pause_exit\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: PauseExitFn = unsafe { std::mem::transmute(address) };
    let label = |text: Option<&str>| text.map(CString::new).transpose().map_err(|_| Error::Engine(-2));
    let row = label(exit.map(|exit| exit.row))?;
    let confirm = label(exit.and_then(|exit| exit.confirm))?;
    let pointer = |text: &Option<CString>| text.as_ref().map_or(std::ptr::null(), |text| text.as_ptr());
    let result = unsafe { function(pointer(&row), pointer(&confirm)) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn pause_choice() -> Result<PauseChoice, Error> {
    let address = resolve(&PAUSE_CHOICE_FN, b"clone_engine_pause_choice\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: PauseChoiceFn = unsafe { std::mem::transmute(address) };
    Ok(match unsafe { function() } {
        1 => PauseChoice::Quit,
        2 => PauseChoice::Exit,
        _ => PauseChoice::None,
    })
}

pub fn keep_music_into_battle(keep: bool) -> Result<(), Error> {
    let address = resolve(&KEEP_MUSIC_FN, b"clone_engine_keep_music\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: KeepMusicFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(u32::from(keep)) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn play_bgm(name: &str) -> Result<(), Error> {
    let address = resolve(&PLAY_BGM_FN, b"clone_engine_play_bgm\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: PlayBgmFn = unsafe { std::mem::transmute(address) };
    let name = CString::new(name).map_err(|_| Error::Engine(-1))?;
    let result = unsafe { function(name.as_ptr()) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn stop_bgm() -> Result<(), Error> {
    let address = resolve(&STOP_BGM_FN, b"clone_engine_stop_bgm\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: StopBgmFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function() };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn viewer_begin(slot: u32) -> Result<(), Error> {
    let address = resolve(&VIEWER_BEGIN_FN, b"clone_engine_viewer_begin\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: ViewerSlotFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(slot) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn viewer_state(slot: u32) -> Result<ViewerState, Error> {
    let address = resolve(&VIEWER_STATE_FN, b"clone_engine_viewer_state\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: ViewerSlotFn = unsafe { std::mem::transmute(address) };
    match unsafe { function(slot) } {
        0 => Ok(ViewerState::Loading),
        1 => Ok(ViewerState::Built),
        2 => Ok(ViewerState::Shown),
        other => Err(Error::Engine(other)),
    }
}

pub fn viewer_end() -> Result<(), Error> {
    let address = resolve(&VIEWER_END_FN, b"clone_engine_viewer_end\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: EndBattleFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function() };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn gallery_item_count() -> Result<usize, Error> {
    let address = resolve(&GALLERY_COUNT_FN, b"clone_engine_gallery_count\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: EndBattleFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function() };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result as usize)
}

pub fn gallery_item_costume(index: usize) -> Result<u8, Error> {
    let address = resolve(&GALLERY_COSTUME_FN, b"clone_engine_gallery_costume\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: ViewerSlotFn = unsafe { std::mem::transmute(address) };
    let index = u32::try_from(index).map_err(|_| Error::Engine(-1))?;
    let result = unsafe { function(index) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(result as u8)
}

pub fn stop_movie() -> Result<bool, Error> {
    let address = resolve(&MOVIE_STOP_FN, b"clone_engine_movie_stop\0")
        .ok_or(Error::EngineUnavailable)?;
    let function: MovieStateFn = unsafe { std::mem::transmute(address) };
    Ok(unsafe { function() } != 0)
}
