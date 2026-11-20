use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use dionysus_core::prcxml::{Area, StageSound, REVERB_PRESETS};
use std::collections::HashMap;
use std::sync::Mutex;

const VANILLA_STAGE_IDS: u32 = 364;

const SOUND_PLACE: usize = 0x240c770;
const REVERB: usize = 0x240cd00;
const FUNCTION_HEAD: u32 = 0xd10143ff;

const SOUND_PLACE_STORES: [(usize, u32); 4] = [
    (0x240cc70, 0x90024e48),
    (0x240cc74, 0xbd05c500),
    (0x240cc78, 0xf0017448),
    (0x240cc7c, 0xbd0fa901),
];
const OFFSET_GLOBAL: usize = 0x6dd45c4;
const SCALE_GLOBAL: usize = 0x5297fa8;
const DEFAULT_OFFSET: f32 = 0.0;
const DEFAULT_SCALE: f32 = 1.0;

const REVERB_SITES: [(usize, u32); 9] = [
    (0x240cd18, 0x900178e8),
    (0x240cd1c, 0xf947a108),
    (0x240d178, 0xd0017168),
    (0x240d17c, 0x913d0108),
    (0x240d180, 0xb000ff80),
    (0x240d184, 0x91397000),
    (0x240d18c, 0x52803e82),
    (0x240d190, 0x944ca6d4),
    (0x240d194, 0xb90492d3),
];
const REVERB_SETTER: usize = 0x3736ce0;
const REVERB_BUS: usize = 0x43fee5c;
const REVERB_NAMES: usize = 0x523bf40;
const SOUND_MANAGER: usize = 0x5328f40;
const REVERB_FIELD: usize = 0x490;
const REVERB_FADE: u32 = 500;
const NO_REVERB: u32 = 0;

const AREA_LOOKUP: usize = 0x2ed130;
const AREA_LOOKUP_OPCODE: u32 = 0xf940012b;
const AREA_STAGE: usize = 11;
const AREA_KEY: usize = 10;
const AREA_MANAGER_CELL: usize = 9;
const AREA_LIST: usize = 0x100;

const STAGE_ID_TABLE: usize = 0x45497e8;
const STAGE_ID_ROW: usize = 0x48;
const ROW_FORM: usize = 8;
const ROW_NAME: usize = 0x10;
const BATTLE_FORM: u32 = 2;

#[repr(C)]
struct AreaEntry {
    key: AtomicU64,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

#[repr(C)]
struct AreaManager {
    _head: [u8; AREA_LIST],
    begin: usize,
    end: usize,
}

struct OwnArea {
    entry: Box<AreaEntry>,
    _manager: Box<AreaManager>,
    cell: Box<usize>,
}

static SOUNDS: Mutex<Option<HashMap<u64, StageSound>>> = Mutex::new(None);
static AREAS: Mutex<Option<HashMap<u64, OwnArea>>> = Mutex::new(None);
static EDGES: Mutex<Option<HashMap<u64, [Option<f32>; 4]>>> = Mutex::new(None);
static PLACES: Mutex<Option<HashMap<u32, Option<(u64, bool)>>>> = Mutex::new(None);
static LOGGED: Mutex<Vec<(u32, u8)>> = Mutex::new(Vec::new());
static REVERB_READY: AtomicBool = AtomicBool::new(false);
static PLACE_READY: AtomicBool = AtomicBool::new(false);

pub(crate) fn place_of(stage_id: u32) -> Option<(u64, bool)> {
    if let Some(known) = PLACES.lock().ok().and_then(|places| places.as_ref()?.get(&stage_id).copied()) {
        return known;
    }
    let found = crate::stage_registry::registry().lock().ok().and_then(|registry| {
        registry.stages().iter().find_map(|stage| {
            stage
                .forms
                .iter()
                .find(|(minted, _)| *minted == stage_id)
                .map(|(_, form)| (stage.place_hash, *form == crate::stage_ledger::Form::Normal))
        })
    });
    if found.is_some() {
        if let Ok(mut places) = PLACES.lock() {
            places.get_or_insert_with(HashMap::new).insert(stage_id, found);
        }
    }
    found
}

pub(crate) fn set_sound(place: u64, sound: StageSound) {
    if let Ok(mut sounds) = SOUNDS.lock() {
        let known = sounds.get_or_insert_with(HashMap::new).entry(place).or_default();
        if sound.reverb.is_some() {
            known.reverb = sound.reverb;
        }
        if sound.scale.is_some() {
            known.scale = sound.scale;
        }
        if sound.offset.is_some() {
            known.offset = sound.offset;
        }
    }
}

pub(crate) fn set_area(place: u64, area: Area) {
    set_edges(place, [Some(area.x0), Some(area.y0), Some(area.x1), Some(area.y1)]);
}

fn set_edges(place: u64, edges: [Option<f32>; 4]) -> bool {
    let known = {
        let Ok(mut known) = EDGES.lock() else {
            return false;
        };
        let known = known.get_or_insert_with(HashMap::new).entry(place).or_default();
        for (held, edge) in known.iter_mut().zip(edges) {
            if edge.is_some() {
                *held = edge;
            }
        }
        *known
    };
    let [Some(x0), Some(y0), Some(x1), Some(y1)] = known else {
        return false;
    };
    install_area(place, Area { x0, y0, x1, y1 });
    true
}

fn install_area(place: u64, area: Area) {
    let entry = Box::new(AreaEntry {
        key: AtomicU64::new(0),
        x0: area.x0,
        y0: area.y0,
        x1: area.x1,
        y1: area.y1,
    });
    let begin = &*entry as *const AreaEntry as usize;
    let manager = Box::new(AreaManager {
        _head: [0; AREA_LIST],
        begin,
        end: begin + core::mem::size_of::<AreaEntry>(),
    });
    let cell = Box::new(&*manager as *const AreaManager as usize);
    if let Ok(mut areas) = AREAS.lock() {
        areas.get_or_insert_with(HashMap::new).insert(
            place,
            OwnArea {
                entry,
                _manager: manager,
                cell,
            },
        );
    }
}

fn sound_for(stage_id: u32) -> StageSound {
    let Some((place, _)) = place_of(stage_id) else {
        return StageSound::default();
    };
    SOUNDS
        .lock()
        .ok()
        .and_then(|sounds| sounds.as_ref()?.get(&place).copied())
        .unwrap_or_default()
}

fn donor(stage_id: u32) -> Option<u32> {
    if !crate::stage_donor_tests::enabled() {
        return None;
    }
    crate::stage_dispatch::donor_for(stage_id).filter(|donor| *donor < VANILLA_STAGE_IDS)
}

fn first_time(stage_id: u32, what: u8) -> bool {
    let Ok(mut logged) = LOGGED.lock() else {
        return false;
    };
    if logged.contains(&(stage_id, what)) {
        return false;
    }
    logged.push((stage_id, what));
    true
}

fn source(own: bool, donor: Option<u32>) -> String {
    match (own, donor) {
        (true, _) => "its own".to_string(),
        (false, Some(donor)) => format!("its donor {donor}'s"),
        (false, None) => "the default".to_string(),
    }
}

unsafe fn write<T>(offset: usize, value: T) {
    core::ptr::write_volatile(crate::game::addr(offset) as *mut T, value);
}

unsafe fn set_reverb(kind: u32) {
    let manager_cell = core::ptr::read_volatile(crate::game::addr(SOUND_MANAGER) as *const usize);
    if manager_cell == 0 {
        return;
    }
    let manager = core::ptr::read_volatile(manager_cell as *const usize);
    if manager == 0 {
        return;
    }
    let names = crate::game::addr(REVERB_NAMES) as *const *const u8;
    let name = core::ptr::read_volatile(names.add(kind as usize));
    if name.is_null() {
        return;
    }
    let setter: extern "C" fn(*const u8, *const u8, u32) =
        core::mem::transmute(crate::game::addr(REVERB_SETTER));
    setter(crate::game::addr(REVERB_BUS) as *const u8, name, REVERB_FADE);
    core::ptr::write_volatile((manager + REVERB_FIELD) as *mut u32, kind);
}

#[skyline::hook(offset = crate::game::rel(SOUND_PLACE))]
unsafe fn stage_sound_place(stage_id: u32) {
    if stage_id < VANILLA_STAGE_IDS {
        return call_original!(stage_id);
    }
    let ready = PLACE_READY.load(Ordering::Relaxed);
    let own = sound_for(stage_id);
    let donor = donor(stage_id);
    match donor {
        Some(donor) => call_original!(donor),
        None if ready => {
            write(OFFSET_GLOBAL, DEFAULT_OFFSET);
            write(SCALE_GLOBAL, DEFAULT_SCALE);
        }
        None => call_original!(stage_id),
    }
    if !ready {
        return;
    }
    if let Some(offset) = own.offset {
        write(OFFSET_GLOBAL, offset);
    }
    if let Some(scale) = own.scale {
        write(SCALE_GLOBAL, scale);
    }
    if first_time(stage_id, 0) {
        let offset = core::ptr::read_volatile(crate::game::addr(OFFSET_GLOBAL) as *const f32);
        let scale = core::ptr::read_volatile(crate::game::addr(SCALE_GLOBAL) as *const f32);
        crate::dbg_log_public(&format!(
            "[stagerows] stage id {stage_id} takes {} sound scale {scale} and {} offset {offset}",
            source(own.scale.is_some(), donor),
            source(own.offset.is_some(), donor),
        ));
    }
}

#[skyline::hook(offset = crate::game::rel(REVERB))]
unsafe fn stage_reverb(stage_id: u32) {
    if stage_id < VANILLA_STAGE_IDS {
        return call_original!(stage_id);
    }
    let own = sound_for(stage_id).reverb.filter(|_| REVERB_READY.load(Ordering::Relaxed));
    let donor = donor(stage_id);
    match (own, donor) {
        (Some(kind), _) => set_reverb(kind),
        (None, Some(donor)) => call_original!(donor),
        (None, None) if REVERB_READY.load(Ordering::Relaxed) => set_reverb(NO_REVERB),
        (None, None) => call_original!(stage_id),
    }
    if first_time(stage_id, 1) {
        let shown = match (own, donor) {
            (Some(kind), _) => format!("its own reverb {}", REVERB_PRESETS[kind as usize]),
            (None, Some(donor)) => format!("its donor {donor}'s reverb"),
            (None, None) => format!("no reverb ({})", REVERB_PRESETS[NO_REVERB as usize]),
        };
        crate::dbg_log_public(&format!("[stagerows] stage id {stage_id} takes {shown}"));
    }
}

unsafe fn donor_key(donor: u32) -> u64 {
    let row = crate::game::addr(STAGE_ID_TABLE) + donor as usize * STAGE_ID_ROW;
    if core::ptr::read_volatile((row + ROW_FORM) as *const u32) == BATTLE_FORM {
        return crate::stage_ledger::hash40("battlefield");
    }
    core::ptr::read_volatile((row + ROW_NAME) as *const u64)
}

#[skyline::hook(offset = crate::game::rel(AREA_LOOKUP), inline)]
unsafe fn area_lookup(ctx: &mut skyline::hooks::InlineCtx) {
    let stage_id = ctx.registers[AREA_STAGE].w();
    if stage_id < VANILLA_STAGE_IDS || (stage_id as i32) < 0 {
        return;
    }
    let place = place_of(stage_id).filter(|(_, normal)| *normal).map(|(place, _)| place);
    let own = place.and_then(|place| AREAS.lock().ok().and_then(|areas| {
        let area = areas.as_ref()?.get(&place)?;
        area.entry.key.store(ctx.registers[AREA_KEY].x(), Ordering::Relaxed);
        Some(&*area.cell as *const usize as u64)
    }));
    let donor = match own {
        Some(cell) => {
            ctx.registers[AREA_MANAGER_CELL].set_x(cell);
            None
        }
        None => {
            let Some(donor) = donor(stage_id) else {
                return;
            };
            ctx.registers[AREA_KEY].set_x(donor_key(donor));
            Some(donor)
        }
    };
    if first_time(stage_id, 2) {
        crate::dbg_log_public(&format!(
            "[stagerows] stage id {stage_id} gives CPUs {} area",
            source(own.is_some(), donor)
        ));
    }
}

fn holds(sites: &[(usize, u32)]) -> bool {
    sites
        .iter()
        .all(|(offset, opcode)| unsafe { crate::game::text_word(*offset) } == *opcode)
}

pub(crate) fn install() {
    let heads = holds(&[(SOUND_PLACE, FUNCTION_HEAD), (REVERB, FUNCTION_HEAD)]);
    if heads {
        PLACE_READY.store(holds(&SOUND_PLACE_STORES), Ordering::Relaxed);
        REVERB_READY.store(holds(&REVERB_SITES), Ordering::Relaxed);
        skyline::install_hooks!(stage_sound_place, stage_reverb);
    }
    let area = holds(&[(AREA_LOOKUP, AREA_LOOKUP_OPCODE)]);
    if area {
        skyline::install_hooks!(area_lookup);
    }
    skyline::println!(
        "[stagerows] cloned stages take their donor's reverb and sound settings {}, own reverb {}, CPU areas {}",
        if heads { "on" } else { "off (another instruction)" },
        if REVERB_READY.load(Ordering::Relaxed) { "on" } else { "off" },
        if area { "on" } else { "off (another instruction)" },
    );
}

#[no_mangle]
pub extern "C" fn clone_engine_stage_sound_set(place: u64, reverb: i32, scale: f32, offset: f32) -> i32 {
    if reverb >= REVERB_PRESETS.len() as i32 {
        return -2;
    }
    set_sound(
        place,
        StageSound {
            reverb: u32::try_from(reverb).ok(),
            scale: (!scale.is_nan()).then_some(scale),
            offset: (!offset.is_nan()).then_some(offset),
        },
    );
    0
}

#[no_mangle]
pub extern "C" fn clone_engine_stage_area_set(place: u64, x0: f32, y0: f32, x1: f32, y1: f32) -> i32 {
    let edges = [x0, y0, x1, y1].map(|edge| (!edge.is_nan()).then_some(edge));
    if edges.iter().all(Option::is_none) {
        return -2;
    }
    i32::from(!set_edges(place, edges))
}
