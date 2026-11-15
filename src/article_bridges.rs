use super::*;

macro_rules! custom_article_name_hooks {
    ($install:ident, $lookup:path; $($name:ident($src:expr, $dst:expr, $offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[$src].x() as i32;
                let game = ctx.registers[$dst].x() as *const u8;
                let chosen = $lookup(weapon_kind);
                if let Some(value) = chosen {
                    ctx.registers[$dst].set_x(value.as_ptr() as u64);
                }
                if weapon_kind >= INHERITED_KIND_LOG_FLOOR {
                    static NAME_LOG: core::sync::atomic::AtomicU32 =
                        core::sync::atomic::AtomicU32::new(0);
                    if NAME_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 6 {
                        dbg_log!(
                            "[articlename] {:#x}: weapon {weapon_kind} game name {:?} ours {:?}",
                            $offset as usize,
                            (!game.is_null())
                                .then(|| core::ffi::CStr::from_ptr(game.cast()).to_str().ok())
                                .flatten(),
                            chosen.and_then(|value| core::ffi::CStr::from_bytes_until_nul(value)
                                .ok()
                                .and_then(|name| name.to_str().ok()))
                        );
                    }
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

macro_rules! custom_article_scaled_name_hooks {
    ($install:ident, $lookup:path, $vanilla:path; $($name:ident($src:expr, $dst:expr, $offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = (ctx.registers[$src].x() >> 3) as i32;
                if let Some(value) = $lookup(weapon_kind) {
                    ctx.registers[$dst].set_x(value.as_ptr() as u64);
                    return;
                }
                if let Some(pointer) = $vanilla(weapon_kind) {
                    ctx.registers[$dst].set_x(pointer as u64);
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_scaled_name_hooks! {
    install_custom_article_scaled_weapon_name_hooks, custom_articles::source_weapon_name,
        crate::smashline_names::vanilla_weapon_name;
    custom_weapon_name_game_acmd(8, 3, 0x33ace8c);
    custom_weapon_name_sound_acmd(8, 3, 0x33aed4c);
    custom_weapon_name_effect_acmd(8, 3, 0x33addec);
    custom_weapon_name_status(8, 3, 0x33abf64);
}

custom_article_scaled_name_hooks! {
    install_custom_article_scaled_owner_name_hooks, custom_articles::source_weapon_owner_name,
        crate::smashline_names::vanilla_weapon_owner_name;
    custom_weapon_owner_name_game_acmd(8, 2, 0x33ace7c);
    custom_weapon_owner_name_sound_acmd(8, 2, 0x33aed3c);
    custom_weapon_owner_name_effect_acmd(8, 2, 0x33adddc);
    custom_weapon_owner_name_status(8, 2, 0x33abf54);
}

macro_rules! custom_article_agent_gate_hooks {
    ($install:ident; $($name:ident($offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[22].x() as i32;
                if custom_articles::is_custom_weapon_kind(weapon_kind) {
                    ctx.registers[23].set_x(0);
                }
                if let Some(minted) = crate::pending_weapon_kind()
                    .filter(|minted| !custom_articles::runs_original_code(*minted))
                {
                    ctx.registers[23].set_x(1);
                    static BLANK_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
                    if BLANK_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
                        crate::dbg_log_public(&format!(
                            "[articlecode] {:#x}: weapon kind {minted} has original_code = false; the game builds no agent for it, Smashline builds a blank one",
                            $offset as usize
                        ));
                    }
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_agent_gate_hooks! {
    install_custom_article_agent_gate_hooks;
    custom_weapon_agent_gate_status(0x33abf24);
    custom_weapon_agent_gate_game_acmd(0x33ace2c);
    custom_weapon_agent_gate_sound_acmd(0x33aecec);
    custom_weapon_agent_gate_effect_acmd(0x33add8c);
}

custom_article_name_hooks! {
    install_custom_article_weapon_name_hooks, custom_articles::param_lookup_weapon_name;
    custom_weapon_name_param(21, 27, 0x33b6830);
    custom_weapon_name_map_collision(21, 2, 0x33b69f0);
    custom_weapon_name_visibility(21, 2, 0x33b6d14);
    custom_weapon_name_visibility_data(21, 2, 0x33b6c80);
}

macro_rules! custom_article_owner_kind_hooks {
    ($install:ident, $lookup:path; $($name:ident($src:expr, $dst:expr, $offset:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[$src].x() as i32;
                let game_owner = ctx.registers[$dst].x() as i32;
                let ours = $lookup(weapon_kind);
                if let Some(owner) = ours {
                    ctx.registers[$dst].set_x(owner as u64);
                }
                if weapon_kind >= INHERITED_KIND_LOG_FLOOR {
                    static OWNER_LOG: core::sync::atomic::AtomicU32 =
                        core::sync::atomic::AtomicU32::new(0);
                    if OWNER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
                        dbg_log!(
                            "[articleowner] {:#x}: weapon kind {weapon_kind} game owner                              {game_owner} ours {ours:?}",
                            $offset as usize
                        );
                    }
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

#[cfg(feature = "css_slot")]
fn weapon_owner_kind_for_params(weapon_kind: i32) -> Option<i32> {
    let original = custom_articles::runs_original_code(weapon_kind);
    if let Some(source_owner) = custom_articles::source_weapon_owner_kind(weapon_kind).filter(|_| original) {
        unsafe { crate::fighter_params::ensure_kind_params_loaded(source_owner) };
        if crate::fighter_params::kind_params_have_real_data(source_owner) {
            return Some(source_owner);
        }
    }
    if let Some(owner) = custom_articles::custom_weapon_owner_kind(weapon_kind) {
        return Some(owner);
    }
    let (source_owner, destination) = custom_articles::inherited_child_owner(weapon_kind)?;
    unsafe { crate::fighter_params::ensure_kind_params_loaded(source_owner) };
    if crate::fighter_params::kind_params_have_real_data(source_owner) {
        return None;
    }
    crate::fighter_params::kind_params_are_foreign(source_owner).then_some(destination)
}

custom_article_owner_kind_hooks! {
    install_custom_article_owner_kind_hooks, weapon_owner_kind_for_params;
    custom_weapon_owner_kind_param(21, 26, 0x33b6628);
}

custom_article_owner_kind_hooks! {
    install_custom_article_creator_owner_kind_hooks, custom_articles::source_weapon_owner_kind;
    custom_weapon_owner_kind_game_acmd(22, 8, 0x33acf78);
    custom_weapon_owner_kind_sound_acmd(22, 8, 0x33aee38);
    custom_weapon_owner_kind_effect_acmd(22, 8, 0x33aded8);
    custom_weapon_owner_kind_status(22, 8, 0x33ac040);
}

#[cfg(feature = "css_slot")]
const WEAPON_RECORD_SLOTS: usize = 64;

const ARTICLE_SPEC_DESCRIPTOR_SLOT: u64 = 0x1b8;
const FIGHTER_PARAM_SINGLETON: u64 = 0x52bb3b0;
const INHERITED_KIND_LOG_FLOOR: i32 = 280;

const FIGHTER_PARAM_RECORD_BASE: u64 = 0x60;
const FIGHTER_PARAM_RECORD_STRIDE: u64 = 0x38;
const FIGHTER_PARAM_RECORD_PAYLOAD: u64 = 0x10;
const HORIZON_TLS_THREAD: u64 = 0x1f8;
const HORIZON_THREAD_HANDLE: u64 = 0x1b0;

#[cfg(feature = "css_slot")]
static WEAPON_RECORD_KIND: [core::sync::atomic::AtomicI32; WEAPON_RECORD_SLOTS] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; WEAPON_RECORD_SLOTS];

#[cfg(feature = "css_slot")]
static WEAPON_RECORD_RESULT: [core::sync::atomic::AtomicU64; WEAPON_RECORD_SLOTS] =
    [const { core::sync::atomic::AtomicU64::new(0) }; WEAPON_RECORD_SLOTS];

#[cfg(feature = "css_slot")]
static WEAPON_RECORD_REBIAS_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static WEAPON_RECORD_VANILLA_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static WEAPON_RECORD_UNDECODED_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const WEAPON_RECORD_SWITCH: usize = 0x452c5c4;

#[cfg(feature = "css_slot")]
const WEAPON_RECORD_SWITCH_JOIN: usize = 0x33b5f30;

#[cfg(feature = "css_slot")]
unsafe fn weapon_record_kind(weapon_kind: i32) -> Option<i32> {
    use dionysus_core::weapon_record::{arm_record_kind, switch_index};
    let Some(index) = switch_index(weapon_kind) else {
        return Some(weapon_kind);
    };
    let table = crate::game::addr(WEAPON_RECORD_SWITCH);
    let offset = core::ptr::read_volatile((table as *const i32).add(index));
    let arm = table.wrapping_add_signed(offset as isize);
    let join = crate::game::addr(WEAPON_RECORD_SWITCH_JOIN);
    let words = [
        core::ptr::read_volatile(arm as *const u32),
        core::ptr::read_volatile((arm + 4) as *const u32),
    ];
    arm_record_kind(weapon_kind, arm, join, words)
}

fn weapon_record_slot(weapon_kind: i32) -> Option<usize> {
    use core::sync::atomic::Ordering;
    for index in 0..WEAPON_RECORD_SLOTS {
        let held = WEAPON_RECORD_KIND[index].load(Ordering::Relaxed);
        if held == weapon_kind {
            return Some(index);
        }
        if held == -1
            && WEAPON_RECORD_KIND[index]
                .compare_exchange(-1, weapon_kind, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        {
            return Some(index);
        }
    }
    None
}

#[cfg(feature = "css_slot")]
std::thread_local! {
    static BUILDER_KIND: core::cell::Cell<i32> = const { core::cell::Cell::new(-1) };
}

#[cfg(feature = "css_slot")]
fn note_builder_kind(kind: i32) {
    let _ = BUILDER_KIND.try_with(|slot| slot.set(kind));
}

#[cfg(feature = "css_slot")]
fn current_builder_kind() -> i32 {
    BUILDER_KIND.try_with(|slot| slot.get()).unwrap_or(-1)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33b5f44), inline)]
pub(crate) unsafe fn custom_article_weapon_record_base(ctx: &mut skyline::hooks::InlineCtx) {
    const WEAPON_RECORD_STRIDE: i64 = 0xe8;

    let weapon_kind = ctx.registers[28].x() as i32;
    let row = ctx.registers[24].x() as i32;
    note_builder_kind(weapon_kind);

    {
        static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 48 {
            dbg_log!("[wpnctor] #{n} kind={weapon_kind}");
        }
        if custom_articles::custom_weapon_source_kind(weapon_kind).is_some()
            || custom_articles::source_weapon_owner_kind(weapon_kind).is_some()
        {
            static REGS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
            let r = REGS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if r < 8 {
                let mut hits: Vec<String> = Vec::new();
                for index in 0..31usize {
                    let value = ctx.registers[index].x() as usize;
                    if let Some(entry) = crate::entry_of_fighter_boma_public(value) {
                        hits.push(format!("x{index}=>entry{entry}"));
                    }
                }
                let stack = crate::scan_stack_for_owner(ctx.sp.x() as usize, 0x400);
                dbg_log!(
                    "[wpnscan] #{r} kind={weapon_kind} regs[{}] stack[{stack}]",
                    if hits.is_empty() { "none".to_string() } else { hits.join(",") }
                );
            }
        }
    }

    let Some(source) = custom_articles::custom_weapon_source_kind(weapon_kind) else {
        return;
    };

    if let Some(vanilla) = crate::active_vanilla_construction_kind() {
        let n = WEAPON_RECORD_VANILLA_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 {
            crate::dbg_log_public(&format!(
                "[articlerecord] weapon kind {weapon_kind} belongs to vanilla fighter {vanilla}                  under construction; leaving its own record alone (would have used source {source})"
            ));
        }
        return;
    }

    if let Some(owner) = custom_articles::source_weapon_owner_kind(weapon_kind)
        .filter(|_| custom_articles::runs_original_code(weapon_kind))
    {
        fighter_modules::ensure_loaded(owner, 20000);
    }

    let record = match weapon_record_kind(source) {
        Some(record) => record,
        None => {
            let n = WEAPON_RECORD_UNDECODED_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 4 {
                crate::dbg_log_public(&format!(
                    "[articlerecord] weapon kind {weapon_kind}: source {source}'s record switch arm did not decode; using record {source}"
                ));
            }
            source
        }
    };
    let bias = (record as i64 - row as i64) * WEAPON_RECORD_STRIDE;
    let base = ctx.registers[25].x();
    let slot = weapon_record_slot(weapon_kind);

    if let Some(index) = slot {
        let produced = WEAPON_RECORD_RESULT[index].load(core::sync::atomic::Ordering::Relaxed);
        if produced != 0 && base == produced {
            let n = WEAPON_RECORD_REBIAS_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 8 {
                crate::dbg_log_public(&format!(
                    "[articlerecord] weapon kind {weapon_kind} re-entered with an ALREADY biased                      record {base:#x}; skipping the second bias (would have produced {:#x})",
                    base.wrapping_add(bias as u64)
                ));
            }
            return;
        }
    }

    let source_record = base.wrapping_add((record as i64 * WEAPON_RECORD_STRIDE) as u64) as usize;
    let biased = match crate::weapon_records::private_record(weapon_kind, source_record) {
        Some(private) => (private as u64)
            .wrapping_sub((row as i64 * WEAPON_RECORD_STRIDE) as u64),
        None => base.wrapping_add(bias as u64),
    };
    ctx.registers[25].set_x(biased);
    if let Some(index) = slot {
        WEAPON_RECORD_RESULT[index].store(biased, core::sync::atomic::Ordering::Relaxed);
    }

    static RECORD_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    let n = RECORD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 16 {
        dbg_log!(
            "[articlerecord] #{n} weapon kind {weapon_kind} (row {row}) uses source {source}'s record {record} (base {base:#x} biased by {bias} -> {biased:#x})"
        );
    }
}

#[cfg(all(feature = "css_slot", target_arch = "aarch64"))]
unsafe fn horizon_thread_local() -> u64 {
    let value: u64;
    core::arch::asm!("mrs {}, tpidrro_el0", out(reg) value, options(nomem, nostack));
    value
}

#[cfg(all(feature = "css_slot", not(target_arch = "aarch64")))]
unsafe fn horizon_thread_local() -> u64 {
    0
}

#[cfg(feature = "css_slot")]
static PAYLOAD_FILLED: [core::sync::atomic::AtomicI32; WEAPON_RECORD_SLOTS] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; WEAPON_RECORD_SLOTS];

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x3a6fe8), inline)]
pub(crate) unsafe fn article_init_owner_payload(ctx: &mut skyline::hooks::InlineCtx) {
    let entry = ctx.registers[8].x();
    if entry == 0 {
        return;
    }
    let weapon_kind = core::ptr::read_volatile(entry as *const i32);
    let callback = core::ptr::read_volatile((entry + 8) as *const u64);
    if callback == 0 {
        return;
    }
    let Some(owner) = custom_articles::source_weapon_owner_kind(weapon_kind) else {
        return;
    };
    if !crate::fighter_params::kind_params_absent(owner) {
        return;
    }
    if PAYLOAD_FILLED
        .iter()
        .any(|slot| slot.load(core::sync::atomic::Ordering::Acquire) == weapon_kind)
    {
        return;
    }

    let offsets = custom_articles::callback_payload_offsets(callback as *const u8, owner);
    if offsets.is_empty() {
        return;
    }
    let resident = custom_articles::custom_weapon_owner_kind(weapon_kind)
        .and_then(|destination| crate::fighter_params::payload_fields(destination, &offsets))
        .map(|fields| {
            fields
                .iter()
                .map(|(at, value)| format!("{at:#x}={value:#x}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_else(|| "unavailable".to_owned());
    let (filled, present) = crate::fighter_params::fill_payload_pointers(owner, &offsets);
    if filled == 0 && present == 0 {
        return;
    }
    for slot in PAYLOAD_FILLED.iter() {
        if slot
            .compare_exchange(
                -1,
                weapon_kind,
                core::sync::atomic::Ordering::AcqRel,
                core::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
        {
            break;
        }
    }
    dbg_log!(
        "[articleinit] weapon {weapon_kind}: its source init {:#x} reads absent fighter          {owner}'s payload at {:?}; filled {filled} null field(s), {present} already had data.          The same fields on the RESIDENT destination fighter read: {resident}",
        crate::game::offset_of(callback as usize) as u64,
        offsets.iter().map(|at| format!("{at:#x}")).collect::<Vec<_>>()
    );
}

#[cfg(feature = "css_slot")]
const RESOURCE_LOADER: usize = 0x5331f20;
#[cfg(feature = "css_slot")]
const ARTICLE_LVD_FILE_INDEX: u64 = 0x128;
#[cfg(feature = "css_slot")]
const RESOURCE_INDEX_MISSING: u32 = 0xffffff;
#[cfg(feature = "css_slot")]
const ARTICLE_LVD_COLLISIONS: u64 = 0x168;

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33bf610))]
pub(crate) unsafe fn article_lvd_parse_guard(article: u64) {
    if article == 0 {
        return;
    }
    let mut index = core::ptr::read_volatile((article + ARTICLE_LVD_FILE_INDEX) as *const u32);
    let read = ground_module_owner_kind(article);
    let kind = match read {
        Some(kind) if custom_articles::custom_weapon_source_kind(kind).is_some() => Some(kind),
        _ => crate::pending_weapon_kind().or(read),
    };
    if kind.is_none() {
        static KIND_LOG: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
        if !KIND_LOG.swap(true, core::sync::atomic::Ordering::Relaxed) {
            dbg_log!(
                "[articlelvd] could not read the kind of the article parsing file {index:#x}; using the first clone registered for it"
            );
        }
    }
    if let Some(ours) = lvd_remap(index, kind) {
        static REMAP_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        if REMAP_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
            dbg_log!(
                "[articlelvd] weapon {kind:?}: the setup resolved the source file {index:#x}; parsing the clone's own {ours:#x} instead"
            );
        }
        crate::fighter_params::request_resource_file(ours as i32);
        crate::fighter_params::release_resource_file(index as i32);
        core::ptr::write_volatile((article + ARTICLE_LVD_FILE_INDEX) as *mut u32, ours);
        index = ours;
    }
    static LVD_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    match crate::fighter_params::file_resident(index) {
        Ok(resident) => {
            if LVD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
                let magic = core::ptr::read_volatile((resident + 6) as *const [u8; 4]);
                dbg_log!(
                    "[articlelvd] file index {index:#x} data at {resident:#x}, magic {:?};                      parsing it",
                    core::str::from_utf8(&magic)
                );
            }
            call_original!(article)
        }
        Err(reason) => {
            let collisions =
                core::ptr::read_volatile((article + ARTICLE_LVD_COLLISIONS) as *const u64);
            core::ptr::write_volatile((article + ARTICLE_LVD_COLLISIONS) as *mut u64, 0);
            if LVD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
                dbg_log!(
                    "[articlelvd] skipping the parse for file index {index:#x}: {reason};                      cleared the collision list at +{ARTICLE_LVD_COLLISIONS:#x} (was                      {collisions:#x}) so the setup takes its own empty branch"
                );
            }
        }
    }
}

#[cfg(feature = "css_slot")]
static LVD_REMAP: std::sync::RwLock<dionysus_core::article_lvd::RemapTable> =
    std::sync::RwLock::new(dionysus_core::article_lvd::RemapTable::new());

#[cfg(feature = "css_slot")]
fn note_lvd_remap(source: u32, kind: i32, clone: u32) {
    if let Ok(mut table) = LVD_REMAP.write() {
        table.note(source, kind, clone);
    }
}

#[cfg(feature = "css_slot")]
fn lvd_remap(source: u32, kind: Option<i32>) -> Option<u32> {
    LVD_REMAP
        .read()
        .ok()?
        .lookup(source, kind)
        .filter(|index| *index != RESOURCE_INDEX_MISSING)
}

#[cfg(feature = "css_slot")]
const MODULE_OWNER_ACCESSOR: u64 = 0x20;

#[cfg(feature = "css_slot")]
unsafe fn ground_module_owner_kind(module: u64) -> Option<i32> {
    let accessor = core::ptr::read_volatile((module + MODULE_OWNER_ACCESSOR) as *const u64);
    if accessor < 0x1000 || accessor % 8 != 0 {
        return None;
    }
    if let Some(minted) = crate::article_kind_facade::minted_kind_of_boma(accessor as usize) {
        return Some(minted);
    }
    let accessor = accessor as *mut smash::app::BattleObjectModuleAccessor;
    let kind = smash::app::utility::get_kind(&mut *accessor);
    (kind >= 0).then_some(kind)
}

#[cfg(feature = "css_slot")]
unsafe fn keep_article_lvd_resident(weapon_kind: i32) {
    let clones = match LVD_REMAP.read() {
        Ok(table) => table.clones_of(weapon_kind),
        Err(_) => return,
    };
    for index in clones {
        if crate::fighter_params::file_resident(index).is_err() {
            crate::fighter_params::request_resource_file(index as i32);
        }
    }
}

#[cfg(feature = "css_slot")]
const ARTICLE_COSTUME_SLOTS: usize = dionysus_core::article_lvd::COSTUMES;

#[cfg(feature = "css_slot")]
static LVD_REQUESTED: [core::sync::atomic::AtomicI32; WEAPON_RECORD_SLOTS] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; WEAPON_RECORD_SLOTS];

#[cfg(feature = "css_slot")]
unsafe fn request_article_lvd(weapon_kind: i32) {
    if LVD_REQUESTED
        .iter()
        .any(|slot| slot.load(core::sync::atomic::Ordering::Acquire) == weapon_kind)
    {
        keep_article_lvd_resident(weapon_kind);
        return;
    }

    let Some(owner) = custom_articles::resource_owner_of(weapon_kind) else {
        return;
    };
    let Some(name) = custom_articles::custom_weapon_name(weapon_kind) else {
        return;
    };
    let Some(source) = custom_articles::source_weapon_name(weapon_kind) else {
        return;
    };
    let trim = |bytes: &[u8]| -> Option<String> {
        core::str::from_utf8(bytes.split(|byte| *byte == 0).next()?)
            .ok()
            .map(str::to_owned)
    };
    let (Some(owner), Some(name)) = (trim(owner), trim(name)) else {
        return;
    };
    let Ok(source) = source.to_str() else {
        return;
    };
    let source_owner = custom_articles::source_weapon_owner_name(weapon_kind)
        .and_then(|name| name.to_str().ok())
        .unwrap_or("?");

    let scan = |path: String| crate::item_params::scan_file_path_index(crate::hash40::hash40(&path));
    let shipped: Vec<Option<u32>> = (0..ARTICLE_COSTUME_SLOTS)
        .map(|costume| scan(format!("fighter/{owner}/model/{name}/c{costume:02}/{source}.lvd")))
        .collect();
    let vanilla: Vec<Option<u32>> = (0..ARTICLE_COSTUME_SLOTS)
        .map(|costume| {
            scan(format!(
                "fighter/{source_owner}/model/{source}/c{costume:02}/{source}.lvd"
            ))
        })
        .collect();
    let found = shipped.iter().flatten().count();
    let plan = dionysus_core::article_lvd::plan(&shipped);
    for (costume, chosen) in plan.chosen.iter().enumerate() {
        let (Some((_, index)), Some(from)) = (chosen, vanilla[costume]) else {
            continue;
        };
        note_lvd_remap(from, weapon_kind, *index);
    }
    keep_article_lvd_resident(weapon_kind);
    if !plan.borrowed.is_empty() {
        let pairs: Vec<String> = plan
            .borrowed
            .iter()
            .map(|(costume, from)| format!("c{costume:02}<-c{from:02}"))
            .collect();
        dbg_log!(
            "[articlelvd] weapon {weapon_kind}: {name} ships no {source}.lvd for {} costume(s), borrowing the nearest: {}",
            pairs.len(),
            pairs.join(" ")
        );
    }
    if found == 0 && vanilla.iter().any(Option::is_some) {
        dbg_log!(
            "[articlelvd] weapon {weapon_kind}: its source {source_owner}/{source} has collision but {name} ships no fighter/{owner}/model/{name}/cNN/{source}.lvd, so it will have none"
        );
    }

    for slot in LVD_REQUESTED.iter() {
        if slot
            .compare_exchange(
                -1,
                weapon_kind,
                core::sync::atomic::Ordering::AcqRel,
                core::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
        {
            break;
        }
    }

    if found != 0 {
        let census: Vec<String> = [
            format!("fighter/{owner}/model/{name}/c00/{source}.lvd"),
            format!("fighter/{owner}/model/{name}/c00/model.numdlb"),
            format!("fighter/{owner}/motion/{name}/c00/motion_list.bin"),
            format!("fighter/{owner}/model/body/c00/model.numdlb"),
            format!("fighter/{source_owner}/model/{source}/c00/{source}.lvd"),
            format!("fighter/{source_owner}/param/vl.prc"),
            format!("fighter/{owner}/param/vl.prc"),
        ]
        .iter()
        .map(|path| {
            let short = path
                .rsplit('/')
                .take(2)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("/");
            let short = short.as_str();
            match crate::item_params::scan_file_path_index(crate::hash40::hash40(path)) {
                None => format!("{short}=absent"),
                Some(index) => match crate::fighter_params::file_resident(index) {
                    Ok(data) => format!("{short}@{index:#x}=data {data:#x}"),
                    Err(reason) => format!("{short}@{index:#x}=NOT RESIDENT ({reason})"),
                },
            }
        })
        .collect();
        dbg_log!(
            "[articlelvd] weapon {weapon_kind}: {found} of {ARTICLE_COSTUME_SLOTS} lvd paths              exist; residency census {}",
            census.join(" | ")
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33b6fb8), inline)]
pub(crate) unsafe fn custom_article_owner_params(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[28].x() as i32;
    let Some(source) = custom_articles::custom_weapon_source_kind(weapon_kind) else {
        crate::clone_vtables::report_vanilla_weapon_construction(
            weapon_kind,
            (ctx.registers[20].x() as usize).wrapping_sub(crate::clone_vtables::WEAPON_CLASS_SUBOBJECT),
            ctx.registers[0].x() as usize,
        );
        return;
    };
    crate::clone_vtables::report_weapon_construction(
        weapon_kind,
        (ctx.registers[20].x() as usize).wrapping_sub(crate::clone_vtables::WEAPON_CLASS_SUBOBJECT),
        ctx.registers[0].x() as usize,
    );

    let owner = custom_articles::source_weapon_owner_kind(weapon_kind)
        .filter(|_| custom_articles::runs_original_code(weapon_kind));
    let loaded = match owner {
        Some(owner) => crate::fighter_params::ensure_kind_params_loaded(owner),
        None => false,
    };
    request_article_lvd(weapon_kind);
    crate::article_lvds::keep_resident(weapon_kind);

    static SPEC_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    if SPEC_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) >= 12 {
        return;
    }

    let read_u64 = |address: u64| -> u64 {
        if address == 0 {
            0
        } else {
            core::ptr::read_volatile(address as *const u64)
        }
    };

    let spec = ctx.registers[0].x();
    let vtable = read_u64(spec);
    let setup = read_u64(vtable.wrapping_add(ARTICLE_SPEC_DESCRIPTOR_SLOT));
    let manager = read_u64(crate::game::addr(FIGHTER_PARAM_SINGLETON as usize) as u64);
    let payload = match owner {
        Some(owner) if manager != 0 && owner >= 0 => read_u64(
            manager
                + FIGHTER_PARAM_RECORD_BASE
                + owner as u64 * FIGHTER_PARAM_RECORD_STRIDE
                + FIGHTER_PARAM_RECORD_PAYLOAD,
        ),
        _ => 0,
    };

    let thread_local = horizon_thread_local();
    let thread = read_u64(thread_local.wrapping_add(HORIZON_TLS_THREAD));
    let handle = if thread == 0 {
        0
    } else {
        core::ptr::read_volatile((thread + HORIZON_THREAD_HANDLE) as *const u32)
    };

    dbg_log!(
        "[articlespec] weapon kind {weapon_kind} source {source} owner {owner:?}          params={loaded} spec={:#x} vtable={:#x} setup={:#x} manager={manager:#x}          owner_payload={payload:#x} tls={thread_local:#x} thread={thread:#x} handle={handle:#x}",
        crate::game::offset_of(spec as usize),
        crate::game::offset_of(vtable as usize),
        crate::game::offset_of(setup as usize)
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33bab50), inline)]
pub(crate) unsafe fn article_param_fetch_probe(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = current_builder_kind();
    if kind < INHERITED_KIND_LOG_FLOOR {
        return;
    }
    static FETCH_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    if FETCH_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) >= 16 {
        return;
    }
    let object = ctx.registers[0].x();
    let index = ctx.registers[1].x() as u32;
    let accessor = if object == 0 {
        0
    } else {
        core::ptr::read_volatile((object + 0x30) as *const u64)
    };
    let (vtable, entries) = if accessor == 0 {
        (0, 0)
    } else {
        (
            core::ptr::read_volatile(accessor as *const u64),
            core::ptr::read_volatile((accessor + 8) as *const u64),
        )
    };
    dbg_log!(
        "[articleparam] building kind {kind}: fetch index {index} object={object:#x}          accessor={accessor:#x} vtable={:#x} entries={entries:#x}",
        crate::game::offset_of(vtable as usize)
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17e09a8), inline)]
pub(crate) unsafe fn custom_article_owner_category(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[26].x() as i32;
    if let Some(category) = custom_articles::custom_weapon_owner_category(weapon_kind) {
        ctx.registers[25].set_x(category as u64);
    }
}

macro_rules! custom_article_source_kind_sites {
    ($install:ident, $gate:path; $($name:ident($reg:expr, $offset:expr, $what:expr));* $(;)?) => {
        $(
            #[doc = $what]
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let kind = ctx.registers[$reg].x() as i32;
                if let Some(source) = custom_articles::custom_weapon_source_kind(kind).filter(|_| $gate(kind)) {
                    ctx.registers[$reg].set_x(source as u64);
                    static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
                    if SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 2 {
                        crate::dbg_log_public(&format!(
                            "[articlesource] {:#x}: weapon kind {kind} reads as its source {source}",
                            $offset as usize
                        ));
                    }
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_source_kind_sites! {
    install_custom_article_source_data_sites, custom_articles::always;
    custom_article_kind_table_452a(0, 0x33aa1e0, "per-kind table 0x452abd8");
}

custom_article_source_kind_sites! {
    install_custom_article_source_kind_sites, custom_articles::shows_original_kind;
    custom_article_gbeam_owner_row(8, 0x34ed348, "gbeam param row: samusd's gbeam reads kind 4, any other kind 3");
    custom_article_hookshot_owner_row(8, 0x3522938, "hookshot param row: younglink's reads kind 23, any other kind 46");
    custom_article_whip_owner_row(12, 0x34f87c8, "whip param row: one of simon/richter by weapon kind");
    custom_article_whipwire_owner_row(10, 0x34fc6f0, "whipwire param row: one of simon/richter by weapon kind");
    custom_article_arrow_shoot_link(8, 0xc2bf9c, "link/toonlink arrow shoot callback: double-arrow speed for link's arrow (31)");
    custom_article_arrow_shoot_toonlink(8, 0xc2bffc, "link/toonlink arrow shoot callback: charge speed for toonlink's arrow (195)");
    custom_article_arrow_hit_record(8, 0x342ea44, "arrow specializer slot 22: records the fighter an arrow (31, 195) hit");
    custom_article_arrow_hit_stick(8, 0x342eae4, "arrow specializer slot 29: an arrow (31, 195) sticks into the fighter it hit");
    custom_article_arrow_flight_root(8, 0x342f6cc, "arrow flight rotation: toonlink's arrow (195) turns its root joint");
    custom_article_arrow_stick_pose(8, 0x342fd48, "arrow specializer slot 17: toonlink's arrow (195) stuck pose");
    custom_article_arrow_stick_pose_end(8, 0x34300d8, "arrow specializer slot 17: toonlink's arrow (195) stuck pose, second joint");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(crate::offsets::OFF_WEAPON_CLASS_RECACHE), inline)]
unsafe fn custom_article_class_recache(ctx: &mut skyline::hooks::InlineCtx) {
    let asked = ctx.registers[20].w() as i32;
    let object = ctx.registers[19].x() as usize;
    let answered = ctx.registers[0].x() as usize;
    if let Some(class) = crate::clone_vtables::recache_weapon_class(asked, object, answered) {
        ctx.registers[0].set_x(class as u64);
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(crate::offsets::OFF_WEAPON_CLASS_RESOLVER))]
unsafe fn custom_article_kind_spec(kind: u32) -> u64 {
    let Some(source) = custom_articles::custom_weapon_source_kind(kind as i32) else {
        return call_original!(kind);
    };
    if let Some(class) = crate::clone_vtables::weapon_class_for(kind as i32, source) {
        return class as u64;
    }
    call_original!(source as u32)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17e0890), inline)]
pub(crate) unsafe fn custom_article_path_weapon_name(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[23].x() as i32;
    if let Some(base) = custom_articles::weapon_name_table_bias(weapon_kind) {
        ctx.registers[9].set_x(base);
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17e0990), inline)]
pub(crate) unsafe fn shared_article_path_weapon_name(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[23].x() as i32;
    if let Some(name) = custom_articles::custom_weapon_name(weapon_kind) {
        ctx.registers[22].set_x(name.as_ptr() as u64);
    }
}

#[cfg(all(feature = "css_slot", feature = "diag_article"))]
#[skyline::hook(offset = crate::game::rel(0x48aba4), inline)]
pub(crate) unsafe fn module_190_factory(ctx: &mut skyline::hooks::InlineCtx) {
    static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    if SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed) >= 64 {
        return;
    }
    let object = ctx.registers[0].x();
    let vtable = ctx.registers[8].x();
    let method = if vtable == 0 {
        0
    } else {
        core::ptr::read_volatile((vtable + 0x30) as *const u64)
    };
    let target = if object == 0 {
        0
    } else {
        core::ptr::read_volatile((object + 8) as *const u64)
    };
    dbg_log!(
        "[mod190] factory obj={object:#x} vt={vtable:#x} method={method:#x} rel={:#x} target={target:#x} target_rel={:#x} x9={:#x} x10={:#x}",
        crate::game::offset_of(method as usize),
        crate::game::offset_of(target as usize),
        ctx.registers[9].x(),
        ctx.registers[10].x()
    );
}

#[cfg(feature = "css_slot")]
pub(crate) static SPOOFED_ARTICLE_KIND: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn article_object_kind_field(setup: u64) -> Option<*mut i32> {
    if setup == 0 {
        return None;
    }
    let object = core::ptr::read_volatile(setup as *const u64);
    if object == 0 {
        return None;
    }
    Some((object + 0xc) as *mut i32)
}

#[cfg(feature = "css_slot")]
#[allow(dead_code)]
#[skyline::hook(offset = crate::game::rel(0x48aba4), inline)]
pub(crate) unsafe fn article_agent_kind_spoof_enter(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(field) = article_object_kind_field(ctx.registers[20].x()) else {
        return;
    };
    let kind = core::ptr::read_volatile(field);
    let Some(source) = custom_articles::custom_weapon_source_kind(kind) else {
        return;
    };
    core::ptr::write_volatile(field, source);
    SPOOFED_ARTICLE_KIND.store(kind, core::sync::atomic::Ordering::Relaxed);

    static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    let n = SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 8 {
        dbg_log!("[agentspoof] #{n} object kind {kind} -> {source} for agent creation");
    }
}

#[cfg(feature = "css_slot")]
#[allow(dead_code)]
#[skyline::hook(offset = crate::game::rel(0x48abb0), inline)]
pub(crate) unsafe fn article_agent_kind_spoof_leave(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = SPOOFED_ARTICLE_KIND.swap(-1, core::sync::atomic::Ordering::Relaxed);
    if kind < 0 {
        return;
    }
    if let Some(field) = article_object_kind_field(ctx.registers[20].x()) {
        core::ptr::write_volatile(field, kind);
    }
    dbg_log!(
        "[agentspoof] restored object kind {kind}; agent={:#x}",
        ctx.registers[0].x()
    );
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33abee0))]
pub(crate) unsafe fn article_status_agent_create(
    object: *mut u8,
    boma: *mut u8,
    lua_state: *mut u8,
) -> *mut u8 {
    let kind = if object.is_null() {
        -1
    } else {
        core::ptr::read_volatile(object.add(0xc) as *const i32)
    };
    let minted = custom_articles::custom_weapon_source_kind(kind).is_some();
    if !minted {
        let agent = call_original!(object, boma, lua_state);
        crate::common_vtables::on_weapon_agent(object as u64, agent as u64);
        return agent;
    }

    let source = custom_articles::custom_weapon_source_kind(kind);
    let kind_field = object.add(0xc) as *mut i32;
    if let Some(source) = source {
        core::ptr::write_volatile(kind_field, source);
    }
    dbg_log!(
        "[articleagent] creator entered for weapon kind {kind} (presenting {:?})",
        source
    );

    let agent = {
        let _pending = crate::enter_pending_weapon_kind(kind);
        let _names = source
            .map(|source| crate::smashline_names::hold_clone_weapon_names(source, kind))
            .unwrap_or_else(crate::smashline_names::WeaponNameHold::inactive);
        call_original!(object, boma, lua_state)
    };

    if source.is_some() {
        core::ptr::write_volatile(kind_field, kind);
    }
    crate::common_vtables::on_weapon_agent(object as u64, agent as u64);

    dbg_log!(
        "[articleagent] creator returned {:#x} vtable {:#x}",
        agent as usize,
        if agent.is_null() {
            0
        } else {
            core::ptr::read_volatile(agent as *const usize)
        }
    );
    if agent.is_null() {
        dbg_log!("[articleagent] game declined weapon kind {kind}; passing the null through");
    }
    dbg_log!(
        "[articleagent] creator hook returning {:#x}",
        agent as usize
    );
    agent
}

#[cfg(feature = "css_slot")]
pub(crate) fn seen_motion_modules() -> &'static std::sync::RwLock<std::collections::HashSet<usize>>
{
    static MODULES: OnceLock<std::sync::RwLock<std::collections::HashSet<usize>>> = OnceLock::new();
    MODULES.get_or_init(|| std::sync::RwLock::new(std::collections::HashSet::new()))
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x49dc50))]
pub(crate) unsafe fn motion_rate_guard(module: *mut u8) -> f32 {
    const DEFAULT_RATE: f32 = 1.0;
    const MAX_LOGGED: usize = 64;

    if module.is_null() {
        static NULLS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        if NULLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed) == 0 {
            dbg_log!("[motionrate] null module");
        }
        return DEFAULT_RATE;
    }

    let first_sight = match seen_motion_modules().write() {
        Ok(mut seen) => seen.len() < MAX_LOGGED && seen.insert(module as usize),
        Err(_) => false,
    };

    if first_sight {
        let flag = core::ptr::read_volatile(module.add(0x239));
        let holder = core::ptr::read_volatile(module.add(0x10) as *const *const u8);
        let set = if holder.is_null() {
            core::ptr::null()
        } else {
            core::ptr::read_volatile(holder.add(0x38) as *const *const u8)
        };
        let vtable = if set.is_null() {
            core::ptr::null()
        } else {
            core::ptr::read_volatile(set as *const *const usize)
        };
        let lookup = if vtable.is_null() {
            0
        } else {
            core::ptr::read_volatile(vtable.add(2))
        };

        let resource = core::ptr::read_volatile(module.add(0x148) as *const *const u8);
        let motion_count = if resource.is_null() {
            0
        } else {
            core::ptr::read_volatile(resource.add(0x98) as *const u32)
        };

        let module_vtable = core::ptr::read_volatile(module as *const usize);
        let change_motion = if module_vtable == 0 {
            0
        } else {
            core::ptr::read_volatile((module_vtable + 0xe0) as *const usize)
        };

        let stops_at = if flag != 0 && resource.is_null() {
            "NO MOTION RESOURCE: [module+0x148] null, change_motion bails at 0x4983d0"
        } else if flag != 0 && motion_count == 0 {
            "EMPTY MOTION RESOURCE: count 0, change_motion bails at 0x4983d8"
        } else if flag != 0 {
            "flag set but the resource looks bindable"
        } else if holder.is_null() {
            "[module+0x10] null: faults at 0x49dc70"
        } else if set.is_null() {
            "[holder+0x38] null: faults at 0x49dc74"
        } else if vtable.is_null() {
            "[set] null: faults at 0x49dc78"
        } else if lookup == 0 {
            "vtable+0x10 null: branches to zero at 0x49dc80"
        } else {
            "chain intact up to the lookup"
        };

        dbg_log!(
            "[motionrate] module={:#x} flag={flag} res={:#x} count={motion_count} holder={:#x} mvt={:#x} change_motion={:#x} set={:#x} vt={:#x} vt+0x10={:#x} - {stops_at}",
            module as usize,
            resource as usize,
            holder as usize,
            module_vtable,
            change_motion,
            set as usize,
            vtable as usize,
            lookup
        );
    }

    call_original!(module)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x497350))]
pub(crate) unsafe fn motion_bind_probe(
    module: *mut u8,
    arg1: *mut u8,
    arg2: u64,
    arg3: u32,
    arg4: u32,
    arg5: u32,
    arg6: u32,
) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller = crate::game::offset_of(lr);

    macro_rules! pass_through {
        () => {
            call_original!(module, arg1, arg2, arg3, arg4, arg5, arg6)
        };
    }

    if module.is_null() {
        return pass_through!();
    }

    static BOUND: OnceLock<std::sync::RwLock<std::collections::HashSet<usize>>> = OnceLock::new();
    let seen = BOUND.get_or_init(|| std::sync::RwLock::new(std::collections::HashSet::new()));
    let first_sight = match seen.write() {
        Ok(mut seen) => seen.len() < 64 && seen.insert(module as usize),
        Err(_) => false,
    };
    if !first_sight {
        return pass_through!();
    }

    let res_before = core::ptr::read_volatile(module.add(0x148) as *const usize);
    let holder_before = core::ptr::read_volatile(module.add(0x10) as *const usize);
    let teardown_flag = core::ptr::read_volatile(module.add(0x256));

    let ret = pass_through!();

    let res_after = core::ptr::read_volatile(module.add(0x148) as *const usize);
    let holder_after = core::ptr::read_volatile(module.add(0x10) as *const usize);
    let motion_flag = core::ptr::read_volatile(module.add(0x239));
    let expected = if res_after == 0 { 0 } else { res_after + 0xa0 };

    dbg_log!(
        "[motionbind] module={:#x} lr=@{caller:#x} +0x256={teardown_flag} res {:#x}->{:#x} holder {:#x}->{:#x} (expected {:#x}) flag={motion_flag}",
        module as usize,
        res_before,
        res_after,
        holder_before,
        holder_after,
        expected
    );

    ret
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x6defa0))]
pub(crate) unsafe fn change_motion_probe(
    module: *mut u8,
    motion: u64,
    start_frame: f32,
    rate: f32,
    flag2: u32,
    frame: f32,
    flag3: u32,
    flag4: u32,
) -> u64 {
    const SHOOT: u64 = 0x5_7044_fcbe;

    static SEEN: OnceLock<std::sync::RwLock<std::collections::HashSet<usize>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| std::sync::RwLock::new(std::collections::HashSet::new()));
    let first_sight = match seen.write() {
        Ok(mut seen) => seen.len() < 64 && seen.insert(module as usize),
        Err(_) => false,
    };

    if !module.is_null() && (first_sight || motion == SHOOT) {
        static LINES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        if LINES.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 80 {
            let res = core::ptr::read_volatile(module.add(0x148) as *const usize);
            let holder = core::ptr::read_volatile(module.add(0x10) as *const usize);
            let flag = core::ptr::read_volatile(module.add(0x239));
            dbg_log!(
                "[changemotion] module={:#x} motion={:#x} res={:#x} holder={:#x} flag={flag}",
                module as usize,
                motion,
                res,
                holder
            );
        }
    }

    call_original!(
        module,
        motion,
        start_frame,
        rate,
        flag2,
        frame,
        flag3,
        flag4
    )
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x339fee0))]
pub(crate) unsafe fn weapon_motion_setup_probe(object: *mut u8) -> u64 {
    if object.is_null() {
        return call_original!(object);
    }
    let _window = crate::article_kind_facade::set_up_window(object as usize);

    #[cfg(feature = "diag_article_motion")]
    let kind = core::ptr::read_volatile(object.add(0xc) as *const i32);
    let boma = core::ptr::read_volatile(object.add(0x20) as *const usize);
    let owner_kind = if boma == 0 || crate::active_construction_kind_public().is_some() {
        None
    } else {
        crate::article_owner_kind_by_entry(boma as u64)
    };

    #[cfg(feature = "diag_article_motion")]
    let module = if boma == 0 {
        0
    } else {
        core::ptr::read_volatile((boma + 0x88) as *const usize)
    };
    #[cfg(feature = "diag_article_motion")]
    let holder_before = if module == 0 {
        0
    } else {
        core::ptr::read_volatile((module + 0x10) as *const usize)
    };

    #[cfg(feature = "diag_article_motion")]
    let model = if boma == 0 {
        0
    } else {
        core::ptr::read_volatile((boma + 0x78) as *const usize)
    };
    #[cfg(feature = "diag_article_motion")]
    let model_before = read_head(model);

    let ret = match owner_kind {
        Some(kind) => with_construction_context(kind, || call_original!(object)),
        None => call_original!(object),
    };

    if let Some(kind) = owner_kind {
        static SCOPE_LINES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = SCOPE_LINES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 {
            dbg_log!(
                "[articlescope] #{n} weapon motion setup scoped to clone {kind} by entry                  (object={:#x})",
                object as usize
            );
        }
    }

    #[cfg(feature = "diag_article_motion")]
    let model_after = read_head(model);
    #[cfg(feature = "diag_article_motion")]
    let holder_after = if module == 0 {
        0
    } else {
        core::ptr::read_volatile((module + 0x10) as *const usize)
    };

    #[cfg(feature = "diag_article_motion")]
    {
        static LINES: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = LINES.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        let minted = custom_articles::is_custom_weapon_kind(kind);
        if minted || n < 24 {
            dbg_log!(
            "[weaponmotion] #{n} kind={kind} minted={minted} obj={:#x} module={:#x} holder {:#x}->{:#x}",
            object as usize,
            module,
            holder_before,
            holder_after
        );
            dbg_log!(
            "[weaponmodel] #{n} kind={kind} model={model:#x} before={model_before:x?} after={model_after:x?}"
        );
        }
    }

    ret
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn read_head(object: usize) -> [usize; 24] {
    let mut head = [0usize; 24];
    if object == 0 {
        return head;
    }
    for (index, slot) in head.iter_mut().enumerate() {
        *slot = core::ptr::read_volatile((object + index * 8) as *const usize);
    }
    head
}

#[cfg(all(feature = "css_slot", feature = "diag_article_motion"))]
pub(crate) fn install_article_motion_diagnostics() {
    skyline::install_hooks!(motion_rate_guard, motion_bind_probe, change_motion_probe,);
}

#[cfg(all(feature = "css_slot", not(feature = "diag_article_motion")))]
pub(crate) fn install_article_motion_diagnostics() {}

#[cfg(feature = "css_slot")]
const ARTICLE_MODULE_OWNER: usize = 0x8;

#[cfg(feature = "css_slot")]
static ARTICLE_SETUP_SCOPE_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static ARTICLE_SETUP_VANILLA_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x3d3f30))]
pub(crate) unsafe fn article_module_setup_scope(module: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    if module == 0
        || crate::active_construction_kind_public().is_some()
        || crate::kirby_copy::active_kirby_copy_kind().is_some()
    {
        return call_original!(module, a1, a2, a3);
    }
    let owner = core::ptr::read_volatile((module as usize + ARTICLE_MODULE_OWNER) as *const u64);
    let thread = current_thread_key();
    let Some(kind) = crate::article_owner_kind_by_entry(owner) else {
        let n = ARTICLE_SETUP_VANILLA_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 && owner != 0 {
            let owner_kind =
                smash::app::utility::get_kind(&mut *(owner as *mut smash::app::BattleObjectModuleAccessor));
            dbg_log!("[articlesetup] vanilla #{n} ArticleModule setup (module={module:#x}) of kind {owner_kind}: articles built here are not a clone's");
        }
        return crate::ARTICLE_GENERATOR_OWNER.scope(thread, crate::GENERATOR_NOT_A_CLONE, || {
            call_original!(module, a1, a2, a3)
        });
    };
    let n = ARTICLE_SETUP_SCOPE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 16 {
        dbg_log!("[articlesetup] #{n} ArticleModule setup outside construction (module={module:#x}) names clone {kind} as the owner for hard-coded article paths");
    }
    crate::ARTICLE_SETUP_OWNER.scope(thread, kind, || {
        crate::ARTICLE_GENERATOR_OWNER.scope(thread, kind, || call_original!(module, a1, a2, a3))
    })
}

#[cfg(feature = "css_slot")]
pub(crate) fn install_article_motion_scope_bridge() {
    skyline::install_hooks!(weapon_motion_setup_probe, article_module_setup_scope);
}

#[cfg(feature = "css_slot")]
macro_rules! custom_article_gate_base_hooks {
    ($install:ident; $($name:ident($offset:expr, $base:expr, $kind_reg:expr));* $(;)?) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let weapon_kind = ctx.registers[$kind_reg].x() as i32;
                let Some(source) = custom_articles::custom_weapon_source_kind(weapon_kind) else {
                    return;
                };
                let bias = source as i64 - weapon_kind as i64;
                let base = ctx.registers[$base].x();
                ctx.registers[$base].set_x(base.wrapping_add(bias as u64));

                static GATE_LOG: core::sync::atomic::AtomicU32 =
                    core::sync::atomic::AtomicU32::new(0);
                let n = GATE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                if n < 4 {
                    dbg_log!(
                        "[articlecaps] {:#x}: weapon kind {weapon_kind} reads source {source}'s gate byte",
                        $offset as usize
                    );
                }
            }
        )*

        #[cfg(feature = "css_slot")]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

custom_article_gate_base_hooks! {
    install_custom_article_gate_base_hooks;
    custom_article_gate_motion(0x339ff90, 8, 0);
    custom_article_gate_0x3db770(0x3db770, 9, 8);
    custom_article_gate_0x641b40(0x641b40, 9, 8);
    custom_article_gate_0x64588c(0x64588c, 8, 9);
    custom_article_gate_0x33a486c(0x33a486c, 8, 9);
    custom_article_gate_0x33b6530(0x33b6530, 22, 21);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33afc00))]
pub(crate) unsafe fn custom_article_capability_index(weapon_kind: i32) -> i32 {
    let Some(source) = custom_articles::custom_weapon_source_kind(weapon_kind) else {
        return call_original!(weapon_kind);
    };

    let index = call_original!(source);

    static INDEX_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
    let n = INDEX_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 8 {
        dbg_log!(
            "[articlecaps] #{n} weapon kind {weapon_kind} indexes source {source}'s capability row ({index})"
        );
    }
    index
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_article_status_v1(
    weapon_kind: i32,
    line: i32,
    status_kind: i32,
    function: *const (),
) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        return crate::smashline_articles::install_status(weapon_kind, line, status_kind, function);
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (weapon_kind, line, status_kind, function);
        dionysus_api::ERROR_UNSUPPORTED
    }
}

#[cfg(all(feature = "css_slot", feature = "diag_article"))]
pub(crate) static MODULE_PROBE_SEEN: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

macro_rules! module_190_probes {
    ($install:ident; $($name:ident($offset:expr, $tag:expr, [$($reg:expr),*]));* $(;)?) => {
        $(
            #[cfg(all(feature = "css_slot", feature = "diag_article"))]
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let n = MODULE_PROBE_SEEN.load(core::sync::atomic::Ordering::Relaxed);
                if n >= 36 {
                    return;
                }
                MODULE_PROBE_SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                let mut parts = String::new();
                $(
                    parts.push_str(&format!(" x{}={:#x}", $reg, ctx.registers[$reg].x()));
                )*
                dbg_log!("[mod190] {}{}", $tag, parts);
            }
        )*

        #[cfg(all(feature = "css_slot", feature = "diag_article"))]
        fn $install() {
            skyline::install_hooks!($($name,)*);
        }
    };
}

module_190_probes! {
    install_module_190_probes;
    module_190_entry(0x48aa54, "entry", [0, 1]);
    module_190_graph(0x48aaf0, "graph", [19, 20, 9, 10]);
    module_190_list(0x48ab30, "list", [0, 20]);
    module_190_vtable(0x48ab68, "vtable", [0, 8]);
    module_190_past_call(0x48ab7c, "past-blr", [0, 19]);

    module_190_new_agent(0x48abb0, "newagent", [0, 19]);
    module_190_swap(0x48abb8, "swap", [0, 8, 19]);
    module_190_deleter(0x48abc4, "deleter", [0, 8]);
    module_190_teardown(0x48abd8, "teardown", [0]);
}

#[cfg(feature = "css_slot")]
static SHARED_ARTICLE_FALLBACK_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static SHARED_ARTICLE_KEPT_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const SHARED_ARTICLE_FALLBACK_REPORTS: u32 = 32;

#[cfg(feature = "css_slot")]
unsafe fn shared_article_not_loaded(
    out: *mut u32,
    construction_kind: Option<i32>,
    weapon_kind: i32,
) -> Option<(&'static CloneDefinition, u32, &'static str, u32)> {
    if out.is_null() || custom_articles::is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    let definition = construction_kind.and_then(clone_definition)?;
    if definition.ships_own_param_resources() {
        return None;
    }
    let index = core::ptr::read_volatile(out);
    if index == u32::MAX {
        return None;
    }
    let (loads, stop, file_path) = crate::copy_model_probe::model_residency(index);
    (!loads).then_some((definition, index, stop, file_path))
}

#[cfg(feature = "css_slot")]
static LITERAL_ARTICLE_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const LITERAL_ARTICLE_REPORTS: u32 = 32;

#[cfg(feature = "css_slot")]
const LITERAL_NOT_FOUND: u32 = 0xffffff;

#[cfg(feature = "css_slot")]
unsafe fn literal_resident(path: &str, resource_type: i32) -> bool {
    use dionysus_core::article_literals::{residency_probe, MODEL};
    let index = fighter_modules::search_path_index(&residency_probe(path, resource_type));
    if resource_type == MODEL {
        crate::copy_model_probe::model_residency(index).0
    } else {
        crate::copy_model_probe::search_path_loaded(index)
    }
}

#[cfg(feature = "css_slot")]
unsafe fn literal_article_path(
    out: *mut u32,
    construction_kind: Option<i32>,
    weapon_kind: i32,
    resource_type: i32,
    color: i32,
) -> Option<(&'static CloneDefinition, u32, u32, String, &'static str)> {
    use dionysus_core::article_literals::{base_path, clone_path};
    if out.is_null() || crate::kirby_copy::active_kirby_copy_kind().is_some() {
        return None;
    }
    let definition = construction_kind.and_then(clone_definition)?;
    let path = clone_path(
        weapon_kind,
        resource_type,
        color,
        definition.base_resource_name,
        definition.resource_name,
    )?;
    let index = fighter_modules::search_path_index(&path);
    if index == LITERAL_NOT_FOUND {
        return None;
    }
    let base = base_path(weapon_kind, resource_type, color)?;
    let base_ready = literal_resident(&base, resource_type);
    let owned = definition
        .articles
        .iter()
        .any(|article| article.base_weapon_kind == weapon_kind);
    let reason = match (owned, base_ready, literal_resident(&path, resource_type)) {
        (true, _, true) => "the clone owns this article",
        (true, false, false) => "the clone owns this article; neither path is loaded yet",
        (false, false, _) => "the base's file is not loaded",
        _ => return None,
    };
    let previous = core::ptr::read_volatile(out);
    core::ptr::write_volatile(out, index);
    Some((definition, previous, index, path, reason))
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn copy_article_model_loads(weapon_kind: i32, color: i32) -> bool {
    let path: extern "C" fn(*mut u32, i32, i32, i32, i32, i32) =
        core::mem::transmute(crate::game::addr(0x17e0840));
    let lookup = |color: i32| {
        let mut index: u32 = u32::MAX;
        path(&mut index, weapon_kind, 0, 0, color, 0);
        index
    };
    let missing = |index: u32| index == u32::MAX || index == LITERAL_NOT_FOUND;
    let index = lookup(color);
    if missing(index) {
        return missing(lookup(0));
    }
    crate::copy_model_probe::model_residency(index).0
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17e0840))]
pub(crate) unsafe fn custom_article_path_probe(
    out: *mut u32,
    weapon_kind: i32,
    resource_type: i32,
    variant: i32,
    color: i32,
    flags: i32,
) {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let caller = crate::game::offset_of(lr);

    call_original!(out, weapon_kind, resource_type, variant, color, flags);

    let construction_kind = active_construction_kind();
    let literal_owner =
        construction_kind.or_else(|| crate::ARTICLE_SETUP_OWNER.active(current_thread_key()));
    if let Some((definition, previous, index, path, reason)) =
        literal_article_path(out, literal_owner, weapon_kind, resource_type, color)
    {
        let n = LITERAL_ARTICLE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < LITERAL_ARTICLE_REPORTS {
            dbg_log!(
                "[articleliteral] kind={} weapon={weapon_kind:#x} type={resource_type} color={color}: the game's hard-coded {} path {previous:#x} became {path} {index:#x} ({reason})",
                definition.kind,
                definition.base_resource_name
            );
        }
    } else if let Some((definition, index, stop, file_path)) =
        shared_article_not_loaded(out, construction_kind, weapon_kind)
    {
        let mut retry: u32 = u32::MAX;
        crate::ARTICLE_OWNER_OVERRIDE.scope(current_thread_key(), definition.kind, || {
            call_original!(&mut retry as *mut u32, weapon_kind, resource_type, variant, color, flags);
        });
        let found = retry != u32::MAX && retry != LITERAL_NOT_FOUND;
        if found || resource_type == 0 {
            core::ptr::write_volatile(out, retry);
        } else {
            let n = SHARED_ARTICLE_KEPT_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < SHARED_ARTICLE_FALLBACK_REPORTS {
                dbg_log!(
                    "[articleshare] kind={} weapon={weapon_kind:#x} type={resource_type} color={color}: the clone has no path of its own; keeping base path {index:#x}",
                    definition.kind
                );
            }
        }
        let n = SHARED_ARTICLE_FALLBACK_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if (found || resource_type == 0) && n < SHARED_ARTICLE_FALLBACK_REPORTS {
            dbg_log!(
                "[articleshare] kind={} weapon={weapon_kind:#x} type={resource_type} color={color}: base path {index:#x} (file {file_path:#x}) not loaded ({stop}); clone path {retry:#x}",
                definition.kind
            );
        }
    }
    if (resource_type == dionysus_core::article_literals::MOTION_DIRECTORY
        || resource_type == dionysus_core::article_literals::MOTION_LIST)
        && !out.is_null()
        && !custom_articles::is_custom_weapon_kind(weapon_kind)
    {
        if let Some(definition) = construction_kind.and_then(clone_definition) {
            if !definition.ships_own_param_resources() {
                let mut model: u32 = u32::MAX;
                call_original!(&mut model as *mut u32, weapon_kind, 0, variant, color, 0);
                if model != u32::MAX
                    && model != LITERAL_NOT_FOUND
                    && !crate::copy_model_probe::model_residency(model).0
                {
                    let mut own: u32 = u32::MAX;
                    crate::ARTICLE_OWNER_OVERRIDE.scope(current_thread_key(), definition.kind, || {
                        call_original!(&mut own as *mut u32, weapon_kind, 0, variant, color, 0);
                    });
                    if own == LITERAL_NOT_FOUND {
                        core::ptr::write_volatile(out, LITERAL_NOT_FOUND);
                        let n = SHARED_ARTICLE_KEPT_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                        if n < SHARED_ARTICLE_FALLBACK_REPORTS {
                            dbg_log!(
                                "[articleshare] kind={} weapon={weapon_kind:#x} type={resource_type} color={color}: no model, so no motions",
                                definition.kind
                            );
                        }
                    }
                }
            }
        }
    }
    let tracked = custom_articles::is_custom_weapon_kind(weapon_kind)
        || construction_kind
            .and_then(clone_definition)
            .map(|definition| {
                definition
                    .articles
                    .iter()
                    .any(|article| article.base_weapon_kind == weapon_kind)
            })
            .unwrap_or_else(|| (0x25..=0x2d).contains(&weapon_kind));
    if !tracked {
        return;
    }
    let n = ARTICLE_PATH_RESOLVE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n >= 96 {
        return;
    }
    let index = if out.is_null() {
        u32::MAX
    } else {
        core::ptr::read_volatile(out)
    };
    let namespace = construction_kind
        .and_then(clone_definition)
        .map(|definition| definition.resource_name)
        .unwrap_or("vanilla");
    dbg_log!(
        "[articlepath] #{n} lr=@{caller:#x} namespace={namespace} true_kind={:?} weapon_kind={weapon_kind:#x} type={resource_type} variant={variant} color={color} flags={flags:#x} index={index:#x}",
        construction_kind
    );

    if n == 0 && custom_articles::is_custom_weapon_kind(weapon_kind) {
        for candidate in [
            "fighter/donkey/motion/cannonballcloned/c00",
            "fighter/donkey/motion/cannonballcloned/c00/motion_list.bin",
            "fighter/donkey/model/cannonballcloned/c00",
            "fighter/donkey/model/cannonballcloned/c00/model.numdlb",
            "fighter/koopajr/motion/cannonball/c00",
            "fighter/koopajr/motion/cannonball/c00/motion_list.bin",
        ] {
            let candidate_index = fighter_modules::search_path_index(candidate);
            dbg_log!(
                "[articlepath]   candidate {candidate_index:#x} {}{candidate}",
                if candidate_index == index {
                    "<== MATCHES the builder  "
                } else {
                    ""
                }
            );
        }
    }
}

#[cfg(feature = "css_slot")]
#[inline(always)]
pub(crate) fn is_custom_article_data_key(definition: &CloneDefinition, key: u64) -> bool {
    let weapon_kind = (key & 0x1ffff) as i32;
    definition
        .articles
        .iter()
        .any(|article| article.base_weapon_kind == weapon_kind)
}

#[cfg(feature = "css_slot")]
unsafe fn article_context_kind() -> Option<i32> {
    active_construction_kind().or_else(|| crate::ARTICLE_SETUP_OWNER.active(current_thread_key()))
}

#[cfg(feature = "css_slot")]
const ARTICLE_DATA_NODE_KEY: usize = 0x20;

#[cfg(feature = "css_slot")]
unsafe fn article_data_cached(tree: *mut u8, key: u64) -> bool {
    if tree.is_null() {
        return false;
    }
    let header = tree as usize + 8;
    let mut node = core::ptr::read_volatile(header as *const usize);
    let mut found = header;
    while node != 0 {
        let node_key = core::ptr::read_volatile((node + ARTICLE_DATA_NODE_KEY) as *const u64);
        if node_key >= key {
            found = node;
        }
        node = core::ptr::read_volatile((node + if node_key < key { 8 } else { 0 }) as *const usize);
    }
    found != header && core::ptr::read_volatile((found + ARTICLE_DATA_NODE_KEY) as *const u64) <= key
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn tag_custom_article_cache_key(key: u64, site: &str) -> u64 {
    let Some(definition) = article_context_kind().and_then(clone_definition) else {
        return key;
    };
    if definition.articles.is_empty() || definition.article_namespace == 0 {
        return key;
    }
    let tagged = key | (u64::from(definition.article_namespace) << 33);
    let n = ARTICLE_CACHE_KEY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!("[articlecache] #{n} site={site} key={key:#x}->{tagged:#x}");
    }
    tagged
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17eea04), inline)]
pub(crate) unsafe fn custom_article_cache_key_single(ctx: &mut skyline::hooks::InlineCtx) {
    let key = ctx.registers[22].x();
    ctx.registers[22].set_x(tag_custom_article_cache_key(key, "single"));
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17eed48), inline)]
pub(crate) unsafe fn custom_article_cache_key_variant(ctx: &mut skyline::hooks::InlineCtx) {
    let key = ctx.registers[1].x();
    ctx.registers[1].set_x(tag_custom_article_cache_key(key, "variant"));
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17f0924), inline)]
pub(crate) unsafe fn custom_article_cache_key_direct(ctx: &mut skyline::hooks::InlineCtx) {
    let key = ctx.registers[21].x();
    ctx.registers[21].set_x(tag_custom_article_cache_key(key, "direct"));
}

#[cfg(feature = "css_slot")]
static ARTICLE_DATA_VANILLA_LOOKUP_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static ARTICLE_DATA_COPY_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const ARTICLE_DATA_COPY_REPORTS: u32 = 64;

#[cfg(feature = "css_slot")]
static ARTICLE_DATA_EMPTY_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const ADDED_RESOURCE_INDEX_FLOOR: u32 = 0x8_0000;

#[cfg(feature = "css_slot")]
const BARE_WEAPON_KIND_CEILING: u64 = 0x1000;

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17dded0))]
pub(crate) unsafe fn custom_article_data_cache_insert(
    tree: *mut u8,
    key: u64,
    search_index: *const u32,
    variant: u32,
    mode: u32,
) -> u32 {
    let definition = article_context_kind().and_then(clone_definition);
    let is_custom = definition
        .map(|definition| is_custom_article_data_key(definition, key))
        .unwrap_or(false);
    let effective_key = if let Some(definition) = definition.filter(|_| is_custom) {
        key | (u64::from(definition.article_namespace) << 36)
    } else {
        key
    };
    let index = if search_index.is_null() {
        u32::MAX
    } else {
        core::ptr::read_volatile(search_index)
    };
    let copy_kind = crate::kirby_copy::active_kirby_copy_kind();
    let minted = key < BARE_WEAPON_KIND_CEILING
        && crate::custom_articles::custom_weapon_name(key as i32).is_some();
    let guarded = minted || (key < BARE_WEAPON_KIND_CEILING && definition.is_some());
    if guarded && index != u32::MAX && !article_data_cached(tree, effective_key) {
        let (loads, stop, file_path) = crate::copy_model_probe::model_residency(index);
        if !loads {
            let n = ARTICLE_DATA_EMPTY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < ARTICLE_DATA_COPY_REPORTS {
                dbg_log!(
                    "[articlemiss] #{n} REFUSED key={key:#x} search={index:#x} file={file_path:#x} stop={stop} copy_kind={copy_kind:?}; inserting this record would store a null at +0x28 and the next lookup dereferences it at 0x17defc4, so the game own empty return at 0x17ddf00 is used instead"
                );
            }
            return 0;
        }
    }
    let slip = copy_kind.and_then(|_| {
        let n = ARTICLE_DATA_COPY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        (n < ARTICLE_DATA_COPY_REPORTS).then_some(n)
    });
    if let Some(n) = slip {
        dbg_log!(
            "[articlemiss] #{n} enter copy_kind={copy_kind:?} key={key:#x}->{effective_key:#x} custom={is_custom} search={index:#x} variant={variant} mode={mode}"
        );
    }
    let result = call_original!(tree, effective_key, search_index, variant, mode);
    if let Some(n) = slip {
        dbg_log!("[articlemiss] #{n} exit key={key:#x} ret={result}");
    }
    let added = index != u32::MAX && index >= ADDED_RESOURCE_INDEX_FLOOR;
    if is_custom || added || key < BARE_WEAPON_KIND_CEILING {
        let n = ARTICLE_DATA_CACHE_KEY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 160 {
            let tag = if effective_key != key {
                "tagged"
            } else if added {
                "UNTAGGED-ADDED"
            } else {
                "vanilla"
            };
            dbg_log!(
                "[articledata] #{n} {tag} key={key:#x}->{effective_key:#x} index={index:#x} variant={variant} mode={mode} ret={result}"
            );
        }
    }
    result
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn tag_custom_article_data_lookup(
    ctx: &mut skyline::hooks::InlineCtx,
    site: &str,
) {
    let key = ctx.registers[2].x();
    let Some(definition) = article_context_kind().and_then(clone_definition) else {
        if key < BARE_WEAPON_KIND_CEILING {
            let n = ARTICLE_DATA_VANILLA_LOOKUP_LOG
                .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 64 {
                dbg_log!("[articledata_lookup] #{n} site={site} VANILLA key={key:#x} untagged");
            }
        }
        return;
    };
    if !is_custom_article_data_key(definition, key) {
        return;
    }
    let tagged = key | (u64::from(definition.article_namespace) << 36);
    ctx.registers[2].set_x(tagged);
    let n = ARTICLE_DATA_CACHE_LOOKUP_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        dbg_log!("[articledata_lookup] #{n} site={site} key={key:#x}->{tagged:#x}");
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33a03c4), inline)]
pub(crate) unsafe fn custom_article_data_lookup_primary(ctx: &mut skyline::hooks::InlineCtx) {
    tag_custom_article_data_lookup(ctx, "primary");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33a9798), inline)]
pub(crate) unsafe fn custom_article_data_lookup_variant(ctx: &mut skyline::hooks::InlineCtx) {
    tag_custom_article_data_lookup(ctx, "variant");
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn log_article_data_result(
    ctx: &skyline::hooks::InlineCtx,
    stack_offset: usize,
    site: &str,
) {
    let Some(definition) = active_construction_kind().and_then(clone_definition) else {
        return;
    };
    if definition.articles.is_empty() {
        return;
    }
    let sp = ctx.sp.x() as *const u8;
    if sp.is_null() {
        return;
    }
    let object = core::ptr::read_unaligned(sp.add(stack_offset) as *const u64);
    let owner = core::ptr::read_unaligned(sp.add(stack_offset + 8) as *const u64);
    let key = ctx.registers[23].x();
    let n = ARTICLE_DATA_CACHE_RESULT_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        dbg_log!(
            "[articledata_result] #{n} site={site} key={key:#x} object={object:#x} owner={owner:#x}"
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33a03c8), inline)]
pub(crate) unsafe fn custom_article_data_result_primary(ctx: &mut skyline::hooks::InlineCtx) {
    log_article_data_result(ctx, 0x30, "primary");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x33a979c), inline)]
pub(crate) unsafe fn custom_article_data_result_variant(ctx: &mut skyline::hooks::InlineCtx) {
    log_article_data_result(ctx, 0x20, "variant");
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn handle_custom_base_module_name(
    ctx: &mut skyline::hooks::InlineCtx,
    destination: usize,
) {
    let raw_kind = ctx.registers[20].x() as i32;
    if let Some(definition) = clone_definition(raw_kind) {
        ctx.registers[destination].set_x(definition.base_resource_name_cstr.as_ptr() as u64);
    } else if let Some(kind) = active_resource_kind() {
        if crate::final_grafts::donor_of(kind).map(|donor| donor.kind) == Some(raw_kind) {
            return;
        }
        if let Some(definition) = clone_definition(kind) {
            ctx.registers[destination].set_x(definition.base_resource_name_cstr.as_ptr() as u64);
        }
    }
}

#[cfg(feature = "css_slot")]
macro_rules! declare_resource_name_hooks {
    ($install:ident, $handler:ident; $($name:ident($offset:expr, $dst:expr));* $(;)?) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                $handler(ctx, $dst);
            }
        )*

        pub(crate) fn $install() {
            skyline::install_hooks!($($name),*);
        }
    };
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_contextual_custom_resource_name_hooks, handle_custom_resource_name;
    custom_name_model_1(0x17e9aa4, 2);
    custom_name_model_2(0x17e9b94, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w1_custom_resource_name_hooks, handle_custom_resource_name_w1;
    custom_name_path_02(0x17df95c, 2);
    custom_name_path_03(0x17df888, 2);
    custom_name_path_18(0x17df920, 2);
    custom_name_path_19(0x17df934, 2);
    custom_name_path_20(0x17df8a8, 2);
    custom_name_path_21(0x17df948, 2);
    custom_name_path_22(0x17df970, 2);
    custom_name_path_23(0x17df984, 2);
    custom_name_path_24(0x17df8bc, 2);
    custom_name_path_25(0x17df8d0, 2);
    custom_name_path_26(0x17df8e4, 2);
    custom_name_path_27(0x17df998, 2);
    custom_name_path_28(0x17df9ac, 2);
    custom_name_path_29(0x17df9c0, 2);
    custom_name_path_30(0x17df8f8, 2);
    custom_name_path_31(0x17df90c, 2);
    custom_name_path_32(0x17df9d4, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w20_custom_resource_name_hooks, handle_custom_resource_name_w20;
    custom_name_path_04(0x17dfcfc, 2);
    custom_name_path_08(0x17e9094, 2);
    custom_name_path_09(0x17e9118, 2);
    custom_name_path_10(0x17e91b8, 2);
    custom_name_path_11(0x17e9238, 2);
    custom_name_path_12(0x17e9348, 2);
    custom_name_path_13(0x17e936c, 2);
    custom_name_path_14(0x17e9494, 2);
    custom_name_path_15(0x17e9600, 2);
    custom_name_path_17(0x17f0058, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w21_custom_resource_name_hooks, handle_custom_resource_name_w21;
    custom_name_path_05(0x17e0340, 2);
    custom_name_path_06(0x17e040c, 2);
    custom_name_path_16(0x17e9d58, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w19_custom_resource_name_hooks, handle_custom_resource_name_w19;
    custom_name_path_07(0x17e7558, 2);
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_w8_custom_resource_name_hooks, handle_custom_resource_name_w8;
    custom_name_motion(0x60c184, 2);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17df4f0), inline)]
pub(crate) unsafe fn custom_name_path_root(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[1].x() as i32;
    if let Some(definition) = clone_definition(kind) {
        ctx.registers[2].set_x(definition.resource_name_cstr.as_ptr() as u64);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) fn install_custom_resource_name_hooks() {
    install_custom_article_weapon_name_hooks();
    install_custom_article_scaled_weapon_name_hooks();
    install_custom_article_scaled_owner_name_hooks();
    install_custom_article_agent_gate_hooks();
    install_custom_article_gate_base_hooks();
    install_custom_article_source_data_sites();
    install_custom_article_source_kind_sites();
    skyline::install_hooks!(custom_article_kind_spec, custom_article_class_recache);
    crate::article_kind_facade::install();
    #[cfg(feature = "diag_article")]
    {
        install_module_190_probes();
        skyline::install_hooks!(module_190_factory);
    }
    install_article_motion_diagnostics();
    install_article_motion_scope_bridge();
    skyline::install_hooks!(article_status_agent_create,);
    skyline::install_hooks!(
        custom_article_weapon_record_base,
        custom_article_capability_index,
        custom_article_owner_category,
        custom_article_path_weapon_name,
        custom_article_owner_params,
        article_param_fetch_probe,
        article_lvd_parse_guard,
        article_init_owner_payload,
    );
    install_custom_article_owner_kind_hooks();
    install_custom_article_creator_owner_kind_hooks();
    if crate::smashline_articles::numbering_shared() {
        skyline::install_hooks!(shared_article_path_weapon_name);
    }
    install_contextual_custom_resource_name_hooks();
    install_w1_custom_resource_name_hooks();
    install_w19_custom_resource_name_hooks();
    install_w20_custom_resource_name_hooks();
    install_w21_custom_resource_name_hooks();
    install_w8_custom_resource_name_hooks();
    skyline::install_hooks!(
        custom_name_path_root,
        custom_article_path_probe,
        custom_article_owner_name,
        custom_article_weapon_name,
        custom_article_cache_key_single,
        custom_article_cache_key_variant,
        custom_article_cache_key_direct,
        custom_article_data_cache_insert,
        custom_article_data_lookup_primary,
        custom_article_data_lookup_variant,
        custom_article_data_result_primary,
        custom_article_data_result_variant,
        custom_effect_bank_load,
        custom_effect_bank_release
    );
}

#[cfg(feature = "css_slot")]
declare_resource_name_hooks! {
    install_custom_module_name_hook, handle_custom_base_module_name;
    custom_base_module_name(0x17e4bcc, 22);
}
