#![allow(clippy::missing_safety_doc)]

#[cfg(all(feature = "css_slot", not(feature = "clone_runtime")))]
compile_error!(
    "css_slot requires the clone_runtime load/init/namespace bridges \
     (enable `clone_runtime`, or use the default `beta` feature)"
);

pub use dionysus_api::{
    CloneArticleRegistrationV1, CloneRegistrationV1, SharedHookRegistrationV1, API_VERSION_V1,
    BACKEND_STATUS_COMPILED, BACKEND_STATUS_HOOKS_INSTALLED, BACKEND_STATUS_READY,
    BACKEND_STATUS_REGISTRATION_CLOSED, BACKEND_STATUS_RESOURCE_LAYOUT_PROVEN,
    BACKEND_STATUS_RESOURCE_LIFECYCLE_PROVEN, BACKEND_STATUS_STATIC_PREFLIGHT_OK,
    BACKEND_STATUS_STATIC_TABLES_READY, ERROR_ARTICLE, ERROR_BACKEND_UNAVAILABLE, ERROR_BASE_KIND,
    ERROR_COLOR_RANGE, ERROR_CUSTOM_KIND, ERROR_DUPLICATE, ERROR_HOOK_ABI, ERROR_HOOK_CONFLICT,
    ERROR_HOOK_INSTALL, ERROR_HOOK_PREFLIGHT, ERROR_HOOK_UNAVAILABLE, ERROR_NAME, ERROR_NAMESPACE,
    ERROR_NULL, ERROR_REGISTRATION_CLOSED, ERROR_SMASHLINE_REQUIRED, ERROR_STRUCT_SIZE,
    ERROR_UNSUPPORTED, ERROR_VERSION, FLAG_KIRBY_COPY_FULL_MODEL, FLAG_OWNS_PARAM_RESOURCES,
    KIND_AUTO, MAX_PROVEN_CUSTOM_KIND, RESULT_OK, SHARED_HOOK_STATUS_REGISTRATION_CLOSED,
    SMASHLINE_BRIDGE_VERSION_REQUIRED,
};
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock, RwLock};

mod game;
mod game_reloc_13_0_5;

#[cfg(feature = "native_table_backend")]
mod native_tables;

#[cfg(feature = "native_relocate")]
mod resource_relocation;

mod article_owners;
mod costume_slots;
mod hash40;
#[cfg(test)]
mod host_shims;
mod kind_ledger;
mod shared_hook_core;
mod shared_hooks;

#[cfg(feature = "diag_item_kind")]
mod item_re;

#[cfg(feature = "diag_item_categories")]
mod item_category_probe;
#[cfg(feature = "item_ui_backend")]
mod item_ui;
#[cfg(feature = "diag_item_ui")]
mod item_ui_probe;

#[cfg(all(feature = "diag_item_ui", feature = "item_ui_backend"))]
compile_error!("diag_item_ui and item_ui_backend hook the same sites; enable only one");

#[cfg(feature = "item_clone_backend")]
mod block_grid;
#[cfg(all(feature = "item_clone_backend", feature = "css_slot"))]
mod owner_effects;
#[cfg(feature = "item_clone_backend")]
mod item_clones;
#[cfg(feature = "item_clone_backend")]
mod item_spawn;
#[cfg(feature = "item_clone_backend")]
mod enemy_spawn;
#[cfg(feature = "item_clone_backend")]
mod enemy_clones;
#[cfg(feature = "item_clone_backend")]
mod enemy_packs;
#[cfg(feature = "item_clone_backend")]
mod enemy_drivers;
#[cfg(feature = "item_clone_backend")]
mod enemy_params;
#[cfg(feature = "item_clone_backend")]
mod item_common_tables;
#[cfg(feature = "item_clone_backend")]
mod item_generate;
mod fighter_packs;
mod item_packs;
mod mount_events;
mod staffroll;
#[cfg(feature = "item_clone_backend")]
mod item_params;
#[cfg(feature = "item_clone_backend")]
mod item_scripts;
#[cfg(feature = "item_clone_backend")]
mod item_slots;
#[cfg(feature = "item_clone_backend")]
mod item_status_tables;
#[cfg(feature = "diag_item_work")]
mod item_work_probe;
#[cfg(feature = "css_slot")]
#[cfg(feature = "css_slot")]
mod finalsmash_residency;
#[cfg(feature = "css_slot")]
mod append_group;
#[cfg(feature = "css_slot")]
mod entry_groups;
#[cfg(feature = "css_slot")]
mod fight_commands;

#[cfg(all(feature = "diag_item_kind", feature = "item_clone_backend"))]
compile_error!("diag_item_kind and item_clone_backend hook the same item lifecycle seams");

#[cfg(feature = "css_slot")]
const VANILLA_COSTUME_SLOTS: u8 = 8;

#[cfg(feature = "css_slot")]
mod article_item_order;
mod custom_articles;

#[cfg(feature = "css_slot")]
mod fighter_modules;
mod param_overrides;
mod upgrade;
mod stage_alts_bridge;
mod stage_backend;
#[cfg(feature = "css_slot")]
mod starlight_bridge;
mod stage_bounds;
mod stage_collision_probe;
mod stage_config_bridge;
#[cfg(feature = "css_slot")]
mod battle_music;
#[cfg(feature = "css_slot")]
mod battle_pause;
#[cfg(feature = "css_slot")]
mod menu_battle;
#[cfg(feature = "css_slot")]
mod menu_calls;
#[cfg(feature = "css_slot")]
mod menu_flow;
#[cfg(feature = "css_slot")]
mod menu_loading;
#[cfg(feature = "css_slot")]
mod menu_modes;
#[cfg(feature = "css_slot")]
mod menu_movie;
#[cfg(feature = "css_slot")]
mod menu_scripts;
#[cfg(feature = "css_slot")]
mod menu_sequences;
mod stage_move_floors;
mod stage_motions;
mod stage_gimmicks;
mod stage_csk_table;
mod stage_db_rows;
mod stage_dispatch;
mod stage_ledger;
mod stage_music;
mod stage_packs;
mod stage_pane_table;
#[cfg(feature = "stage_probe")]
mod stage_probe;
mod stage_registration;
mod stage_registry;
mod stage_relocation;
mod stage_sound;
#[cfg(feature = "stage_select_runtime")]
mod stage_resolve_probe;
mod stage_select_cap;
mod stage_select_page;
mod stage_select_ui;
mod stage_select_slice;
#[cfg(feature = "stage_relocate")]
mod stage_transaction;
mod text_patch;
mod thread_context;
mod clone_vtables;
mod smashline_vtables;
mod settings;
mod article_hurtboxes;
mod article_lvds;
mod weapon_records;
mod common_vtables;

#[cfg(feature = "css_slot")]
use skyline::nn::ro::LookupSymbol;

mod offsets;
use offsets::*;

pub const FIRST_CUSTOM_KIND: i32 = 118;

#[derive(Clone, Copy)]
struct CloneArticleDefinition {
    base_weapon_kind: i32,
    file_name: &'static str,
    file_name_cstr: &'static [u8],
}

#[derive(Clone, Copy)]
#[cfg_attr(not(feature = "native_table_backend"), allow(dead_code))]
struct CloneDefinition {
    kind: i32,
    base_kind: i32,
    ui_chara: &'static str,
    fighter_kind_name: &'static str,
    resource_name: &'static str,
    resource_name_cstr: &'static [u8],
    resource_name_upper_cstr: &'static [u8],
    resource_name_title_cstr: &'static [u8],
    base_resource_name: &'static str,
    base_resource_name_cstr: &'static [u8],
    color_start: u8,
    color_count: u8,
    copy_status_first: i32,
    copy_status_count: i32,
    effect_namespace: u32,
    article_namespace: u32,
    articles: &'static [CloneArticleDefinition],
    owns_param_resources: bool,
    kirby_copy_full_model: bool,
    css: Option<&'static CloneCssEntry>,
    param_file: &'static core::sync::atomic::AtomicU8,
    from_manifest: bool,
}

const KIRBY_COPY_FAMILY_POOL_FIRST: i32 = 0x37d + 19;
const KIRBY_COPY_FAMILY_POOL_END: i32 = 0x400 + 6;

#[cfg(feature = "css_slot")]
fn manifest_css(manifest: Option<&ManifestRegistration>) -> Option<&'static CloneCssEntry> {
    let css = manifest?.css.as_ref()?;
    Some(Box::leak(Box::new(CloneCssEntry {
        ui_name: css.ui_name,
        ui_series: css.ui_series,
        disp_order: css.disp_order,
        save_no: css.save_no,
        exhibit_year: css.exhibit_year,
        narration: css.narration,
    })))
}

#[cfg(not(feature = "css_slot"))]
fn manifest_css(_manifest: Option<&ManifestRegistration>) -> Option<&'static CloneCssEntry> {
    None
}

const PARAM_FILE_UNCHECKED: u8 = 0;
const PARAM_FILE_ABSENT: u8 = 1;
const PARAM_FILE_SHIPPED: u8 = 2;

#[cfg(feature = "css_slot")]
struct CloneCssEntry {
    ui_name: &'static str,
    ui_series: &'static str,
    disp_order: i8,
    save_no: i8,
    exhibit_year: i16,
    narration: Option<&'static str>,
}

pub(crate) struct ManifestRegistration {
    pub(crate) css: Option<ManifestCss>,
}

pub(crate) struct ManifestCss {
    pub(crate) ui_name: &'static str,
    pub(crate) ui_series: &'static str,
    pub(crate) disp_order: i8,
    pub(crate) save_no: i8,
    pub(crate) exhibit_year: i16,
    pub(crate) narration: Option<&'static str>,
}

#[cfg(not(feature = "css_slot"))]
struct CloneCssEntry;

impl CloneDefinition {
    fn ships_param_file(&self) -> bool {
        match self.param_file.load(core::sync::atomic::Ordering::Acquire) {
            PARAM_FILE_SHIPPED => return true,
            PARAM_FILE_ABSENT => return false,
            _ => {}
        }
        let path = format!("fighter/{}/param/vl.prc", self.resource_name);
        let shipped = fighter_modules::path_exists(&path);
        self.param_file.store(
            if shipped { PARAM_FILE_SHIPPED } else { PARAM_FILE_ABSENT },
            core::sync::atomic::Ordering::Release,
        );
        dbg_log_public(&format!(
            "[cloneparam] kind {} {} '{path}'; its param directories are {}",
            self.kind,
            if shipped { "ships" } else { "does not ship" },
            if shipped || self.owns_param_resources || self.article_namespace != 0 || !self.articles.is_empty() {
                "its own"
            } else {
                "the base fighter's"
            }
        ));
        shipped
    }

    fn ships_own_param_resources(&self) -> bool {
        self.owns_param_resources
            || self.article_namespace != 0
            || !self.articles.is_empty()
            || self.ships_param_file()
    }

    fn ships_own_ai_resources(&self) -> bool {
        self.owns_param_resources
    }

    #[cfg(feature = "css_slot")]
    fn css_color_count(&self) -> u8 {
        costume_slots::effective_color_count(
            self.fighter_kind_name,
            self.color_count,
            self.color_start,
        )
    }
}

static CLONE_DEFINITION_REGISTRY: OnceLock<RwLock<Vec<&'static CloneDefinition>>> = OnceLock::new();

fn clone_definitions() -> &'static RwLock<Vec<&'static CloneDefinition>> {
    CLONE_DEFINITION_REGISTRY.get_or_init(|| RwLock::new(Vec::new()))
}

#[inline(always)]
fn clone_definition(kind: i32) -> Option<&'static CloneDefinition> {
    clone_definitions()
        .read()
        .unwrap()
        .iter()
        .copied()
        .find(|definition| definition.kind == kind)
}

#[cfg(feature = "css_slot")]
pub(crate) fn clone_definition_from_name(name: &str) -> Option<&'static CloneDefinition> {
    clone_definitions()
        .read()
        .ok()?
        .iter()
        .copied()
        .find(|definition| definition.resource_name == name || definition.fighter_kind_name == name)
}

#[cfg(feature = "css_slot")]
fn clone_definition_from_ui(ui_chara: u64) -> Option<&'static CloneDefinition> {
    clone_definitions()
        .read()
        .unwrap()
        .iter()
        .copied()
        .find(|definition| hash40(definition.ui_chara) == ui_chara)
}

#[cfg(feature = "css_slot")]
const OFF_UI_FIGHTER_KIND_LOOKUP: usize = 0x326_2130;
#[cfg(feature = "css_slot")]
const OFF_UPDATE_SELECTED_FIGHTER: usize = 0x331_1190;
#[cfg(feature = "css_slot")]
const OFF_MATCH_ENTRY_EXPAND_OUTER_CALL: usize = 0x66_dd14;
#[cfg(feature = "css_slot")]
const OFF_MATCH_ENTRY_EXPAND_INNER_CALL: usize = 0x66_db84;
#[cfg(feature = "css_slot")]
const OFF_CONSTRUCTION_ROSTER_EXPAND_CALL: usize = 0x14e_c248;
#[cfg(feature = "css_slot")]
const OFF_FIGHTER_CHANGE_EXPAND_CALL: usize = 0x32c_a150;
#[cfg(feature = "css_slot")]
const OFF_FIGHTER_CHANGE_REQUEST_CALL: usize = 0x178_aae0;
#[cfg(feature = "css_slot")]
const OFF_TEAM_QUEUE_EXPAND_INPUT: usize = 0x249_376c;
#[cfg(feature = "css_slot")]
const OFF_RETRY_EXPAND_INPUT: usize = 0x150_4280;
#[cfg(feature = "css_slot")]
const OFF_TAG_TEAM_EXPAND_INPUT: usize = 0x157_0374;
#[cfg(feature = "css_slot")]
const OFF_CLASSIC_ROUTE_PATH: usize = 0x323_ad60;
#[cfg(feature = "css_slot")]
const OFF_CLASSIC_ROUTE_TITLE: usize = 0x1a1_9554;
#[cfg(feature = "css_slot")]
const OFF_CSS_ICON_COLUMN_BOUND: usize = 0x19f_1dec;
#[cfg(feature = "css_slot")]
const OFF_PLAYER_INFO_COSTUME_STORED: usize = 0x178_8380;
#[cfg(feature = "css_slot")]
const OFF_TEAM_MEMBER_COSTUME_STORED: usize = 0x249_3330;
#[cfg(feature = "css_slot")]
const OFF_KIND_DISPLAY_UI: usize = 0x138_c9b0;

static REGISTRY: OnceLock<RwLock<HashMap<i32, i32>>> = OnceLock::new();
static REGISTRATION_GATE: OnceLock<Mutex<()>> = OnceLock::new();
static REGISTRATION_CLOSED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
const REGISTRATION_INIT_WAIT_MILLIS: u32 = 5_000;

static REGISTRATION_TOO_EARLY_LOGGED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
static SMASHLINE_BRIDGE_VERSION: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

fn registry() -> &'static RwLock<HashMap<i32, i32>> {
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

fn registration_gate() -> &'static Mutex<()> {
    REGISTRATION_GATE.get_or_init(|| Mutex::new(()))
}

fn close_registration(site: &'static str) {
    if REGISTRATION_CLOSED.load(core::sync::atomic::Ordering::Acquire) {
        return;
    }
    let _guard = registration_gate().lock().unwrap();
    if !REGISTRATION_CLOSED.swap(true, core::sync::atomic::Ordering::AcqRel) {
        skyline::println!("[clone_engine] clone registration closed at {site}");
    }
}

pub fn register_clone_fighter(new_kind: i32, base_kind: i32) {
    let _guard = registration_gate().lock().unwrap();
    if REGISTRATION_CLOSED.load(core::sync::atomic::Ordering::Acquire) {
        skyline::println!(
            "[clone_engine] WARNING: legacy registration rejected for kind {new_kind}: registration is closed"
        );
        return;
    }
    if new_kind > clone_engine_max_custom_kind() {
        skyline::println!(
            "[clone_engine] WARNING: legacy registration rejected for kind {new_kind}: native backend capacity is unavailable"
        );
        return;
    }
    if new_kind < FIRST_CUSTOM_KIND {
        skyline::println!(
            "[clone_engine] WARNING: custom kind {new_kind} < FIRST_CUSTOM_KIND ({FIRST_CUSTOM_KIND}); it may collide with the vanilla/DLC roster"
        );
    }
    if !(0..=93).contains(&base_kind) {
        skyline::println!(
            "[clone_engine] WARNING: base kind {base_kind} is outside the primary roster (0..=93); the create_agent chain has no agent to build for it"
        );
    }
    if let Some(definition) = clone_definition(new_kind) {
        if definition.base_kind != base_kind {
            skyline::println!(
                "[clone_engine] WARNING: legacy registration rejected for kind {new_kind}: descriptor base {} != requested base {base_kind}",
                definition.base_kind
            );
            return;
        }
    }
    registry().write().unwrap().insert(new_kind, base_kind);
    skyline::println!("[clone_engine] registered clone kind {new_kind} -> base {base_kind}");
}

#[no_mangle]
pub extern "C" fn clone_engine_register(new_kind: i32, base_kind: i32) {
    register_clone_fighter(new_kind, base_kind);
}

#[no_mangle]
pub extern "C" fn clone_engine_api_version() -> u32 {
    API_VERSION_V1
}

fn compiled_capabilities() -> u64 {
    let mut capabilities = dionysus_api::CAP_SHARED_HOOKS | dionysus_api::CAP_MII_PARTS;
    #[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
    {
        capabilities |= dionysus_api::CAP_FIGHTER_IDENTITY
            | dionysus_api::CAP_SMASHLINE_BRIDGE
            | dionysus_api::CAP_FIGHTER_ARTICLES
            | dionysus_api::CAP_KIRBY_COPY
            | dionysus_api::CAP_PARAMCONFIG_BRIDGE;
    }
    #[cfg(feature = "item_clone_backend")]
    {
        capabilities |= dionysus_api::CAP_ITEM_IDENTITY
            | dionysus_api::CAP_ITEM_RESOURCES
            | dionysus_api::CAP_ITEM_PARAMS
            | dionysus_api::CAP_ITEM_ANIMCMD
            | dionysus_api::CAP_ITEM_STATUS;
    }
    #[cfg(feature = "item_ui_backend")]
    {
        capabilities |= dionysus_api::CAP_ITEM_TRAINING_UI;
    }
    #[cfg(feature = "stage_mint")]
    {
        capabilities |= dionysus_api::CAP_STAGE_MINT | dionysus_api::CAP_STAGE_CONFIG;
    }
    #[cfg(feature = "stage_select_runtime")]
    {
        capabilities |= dionysus_api::CAP_STAGE_SELECT_EXTENDED;
    }
    #[cfg(feature = "stage_slot")]
    {
        capabilities |= dionysus_api::CAP_STAGE_CSK;
    }
    #[cfg(feature = "css_slot")]
    {
        capabilities |= dionysus_api::CAP_GLOBAL_MOTIONS
            | dionysus_api::CAP_GRAB_RULES
            | dionysus_api::CAP_HUD
            | dionysus_api::CAP_FINAL_SMASH
            | dionysus_api::CAP_TETHER
            | dionysus_api::CAP_ENTRY_GROUPS
            | dionysus_api::CAP_ANY_BASE_GROUPS
            | dionysus_api::CAP_PARTNER_SWAP
            | dionysus_api::CAP_COMMAND_INPUTS
            | dionysus_api::CAP_MOVE_LIST_PAGES;
    }
    #[cfg(feature = "item_clone_backend")]
    {
        capabilities |= dionysus_api::CAP_RESEARCH_ITEM_FAMILIES;
    }
    capabilities
}

#[no_mangle]
pub extern "C" fn clone_engine_compiled_capabilities_v1() -> u64 {
    compiled_capabilities()
}

#[no_mangle]
pub extern "C" fn clone_engine_runtime_capabilities_v1() -> u64 {
    let mut capabilities = compiled_capabilities();
    if !param_overrides::available() {
        capabilities &= !dionysus_api::CAP_PARAMCONFIG_BRIDGE;
    }
    if shared_hooks::status() & dionysus_api::SHARED_HOOK_STATUS_READY == 0 {
        capabilities &= !dionysus_api::CAP_SHARED_HOOKS;
    }
    #[cfg(feature = "css_slot")]
    if !move_list::armed() {
        capabilities &= !dionysus_api::CAP_MOVE_LIST_PAGES;
    }
    if mii_parts::refused() {
        capabilities &= !dionysus_api::CAP_MII_PARTS;
    }
    #[cfg(feature = "item_clone_backend")]
    {
        let status = item_clones::clone_engine_item_backend_status();
        if status & dionysus_api::ITEM_BACKEND_STATUS_READY == 0 {
            capabilities &= !dionysus_api::CAP_ITEM_IDENTITY;
        }
        if status & dionysus_api::ITEM_BACKEND_STATUS_RESOURCE_ROUTER_READY == 0 {
            capabilities &= !dionysus_api::CAP_ITEM_RESOURCES;
        }
        if status & dionysus_api::ITEM_BACKEND_STATUS_PARAM_ROUTER_READY == 0 {
            capabilities &= !dionysus_api::CAP_ITEM_PARAMS;
        }
        if status & dionysus_api::ITEM_BACKEND_STATUS_CATEGORY_ROUTER_READY == 0 {
            capabilities &= !dionysus_api::CAP_ITEM_ANIMCMD;
        }
        if status & dionysus_api::ITEM_BACKEND_STATUS_STATUS_ROUTER_READY == 0 {
            capabilities &= !dionysus_api::CAP_ITEM_STATUS;
        }
        if status & dionysus_api::ITEM_BACKEND_STATUS_TRAINING_UI_READY == 0 {
            capabilities &= !dionysus_api::CAP_ITEM_TRAINING_UI;
        }
    }
    #[cfg(feature = "stage_mint")]
    {
        let ready = stage_registry::registry()
            .lock()
            .map(|registry| registry.capacities().can_mint())
            .unwrap_or(false);
        if !ready {
            capabilities &=
                !(dionysus_api::CAP_STAGE_MINT | dionysus_api::CAP_STAGE_CONFIG);
        }
    }
    #[cfg(feature = "stage_select_runtime")]
    if !stage_resolve_probe::ready() {
        capabilities &= !dionysus_api::CAP_STAGE_SELECT_EXTENDED;
    }
    capabilities
}

#[no_mangle]
pub extern "C" fn clone_engine_max_custom_kind() -> i32 {
    #[cfg(feature = "native_table_backend")]
    if native_tables::status() & BACKEND_STATUS_READY != 0 {
        return native_tables::last_supported_kind();
    }
    #[cfg(feature = "native_relocate")]
    return resource_relocation::last_usable_kind();
    #[allow(unreachable_code)]
    MAX_PROVEN_CUSTOM_KIND
}

#[no_mangle]
pub extern "C" fn clone_engine_native_backend_status() -> u32 {
    #[cfg(feature = "native_table_backend")]
    let mut status = native_tables::status();
    #[cfg(not(feature = "native_table_backend"))]
    let mut status = 0;
    if REGISTRATION_CLOSED.load(core::sync::atomic::Ordering::Acquire) {
        status |= BACKEND_STATUS_REGISTRATION_CLOSED;
    }
    status
}

unsafe fn read_registration_name(pointer: *const c_char) -> Result<String, i32> {
    if pointer.is_null() {
        return Err(ERROR_NAME);
    }
    let value = CStr::from_ptr(pointer).to_str().map_err(|_| ERROR_NAME)?;
    if value.is_empty()
        || value.len() > 127
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(ERROR_NAME);
    }
    Ok(value.to_owned())
}

fn leak_registration_name(value: String) -> (&'static str, &'static [u8]) {
    let text = Box::leak(value.clone().into_boxed_str());
    let mut bytes = value.into_bytes();
    bytes.push(0);
    let cstr = Box::leak(bytes.into_boxed_slice());
    (text, cstr)
}

fn resource_name_variants(value: &str) -> (String, String) {
    let upper = value.to_ascii_uppercase();
    let mut title = value.as_bytes().to_vec();
    title[0] = title[0].to_ascii_uppercase();
    (
        upper,
        String::from_utf8(title).expect("validated ASCII resource name"),
    )
}

#[cfg(feature = "native_relocate")]
const FIRST_ALLOCATABLE_KIND: i32 = resource_relocation::FIRST_USABLE_SLOT as i32;

unsafe fn allocate_custom_kind(
    fighter_kind_name: *const c_char,
    resource_name: *const c_char,
) -> Result<i32, i32> {
    #[cfg(not(feature = "native_relocate"))]
    {
        let _ = (fighter_kind_name, resource_name);
        return Err(ERROR_BACKEND_UNAVAILABLE);
    }

    #[cfg(feature = "native_relocate")]
    {
        let wanted_fighter_kind = try_borrow_str(fighter_kind_name);
        let wanted_resource = try_borrow_str(resource_name);
        if wanted_fighter_kind.is_none() && wanted_resource.is_none() {
            return Err(ERROR_NAME);
        }

        let definitions = clone_definitions().read().unwrap();
        if let Some(existing) = definitions.iter().find(|definition| {
            wanted_fighter_kind == Some(definition.fighter_kind_name)
                || wanted_resource == Some(definition.resource_name)
        }) {
            return Ok(existing.kind);
        }

        let ceiling = clone_engine_max_custom_kind();
        let taken = |kind: i32| definitions.iter().any(|definition| definition.kind == kind);

        let identity = wanted_fighter_kind.or(wanted_resource).unwrap_or_default();
        let remembered = kind_ledger::reserved_kind(identity)
            .filter(|kind| (FIRST_ALLOCATABLE_KIND..=ceiling).contains(kind) && !taken(*kind));
        let candidate =
            remembered.or_else(|| (FIRST_ALLOCATABLE_KIND..=ceiling).find(|kind| !taken(*kind)));

        match candidate {
            Some(kind) => {
                skyline::println!(
                    "[clone_engine] allocated kind {kind} to {:?} (range {FIRST_ALLOCATABLE_KIND}..={ceiling}, {})",
                    wanted_resource.or(wanted_fighter_kind),
                    if remembered.is_some() {
                        "from ledger"
                    } else {
                        "new reservation"
                    }
                );
                kind_ledger::record(identity, kind);
                Ok(kind)
            }
            None => Err(ERROR_BACKEND_UNAVAILABLE),
        }
    }
}

unsafe fn try_borrow_str(pointer: *const c_char) -> Option<&'static str> {
    if pointer.is_null() {
        return None;
    }
    CStr::from_ptr(pointer).to_str().ok()
}

#[no_mangle]
pub extern "C" fn clone_engine_capacity_committed() -> u32 {
    #[cfg(feature = "native_relocate")]
    return u32::from(resource_relocation::capacity_committed());
    #[allow(unreachable_code)]
    0
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_kind_for_identity(name: *const c_char) -> i32 {
    let Some(name) = try_borrow_str(name) else {
        return ERROR_NULL;
    };
    clone_definitions()
        .read()
        .unwrap()
        .iter()
        .find(|definition| definition.fighter_kind_name == name || definition.resource_name == name)
        .map(|definition| definition.kind)
        .unwrap_or(ERROR_NAME)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_register_v1(registration: *const CloneRegistrationV1) -> i32 {
    register_v1_with(registration, None)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_fighter_kind_by_name_v1(name: *const c_char) -> i32 {
    let Some(name) = try_borrow_str(name) else {
        return ERROR_NULL;
    };
    if let Some(kind) = custom_articles::fighter_kind_from_str(name) {
        return kind;
    }
    clone_engine_kind_for_identity(name.as_ptr().cast())
}

#[no_mangle]
pub extern "C" fn clone_engine_on_mods_mounted_v1(callback: usize) -> i32 {
    mount_events::add_listener(callback)
}

#[no_mangle]
pub extern "C" fn clone_engine_kirby_copy_status_family_v1(
    kind: i32,
    first: *mut i32,
    count: *mut i32,
) -> i32 {
    let Some(definition) = clone_definitions()
        .read()
        .ok()
        .and_then(|definitions| definitions.iter().copied().find(|definition| definition.kind == kind))
    else {
        return ERROR_CUSTOM_KIND;
    };
    if definition.copy_status_count <= 0 {
        return ERROR_UNSUPPORTED;
    }
    if !first.is_null() {
        unsafe { first.write(definition.copy_status_first) };
    }
    if !count.is_null() {
        unsafe { count.write(definition.copy_status_count) };
    }
    RESULT_OK
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_article_kind_by_name_v1(
    kind: i32,
    name: *const c_char,
    kirby_copy: u32,
) -> i32 {
    let Some(name) = try_borrow_str(name) else {
        return ERROR_NULL;
    };
    if kirby_copy != 0 {
        return custom_articles::copy_article_kind_by_name(kind, name).unwrap_or(ERROR_NAME);
    }
    let Some(definition) = clone_definitions()
        .read()
        .ok()
        .and_then(|definitions| definitions.iter().copied().find(|definition| definition.kind == kind))
    else {
        return ERROR_CUSTOM_KIND;
    };
    custom_articles::article_kind_by_name(definition.resource_name, name).unwrap_or(ERROR_NAME)
}


unsafe fn item_vtable(kind: i32) -> Option<(*const usize, bool)> {
    if kind >= item_clones::FIRST_SPARSE_ITEM_KIND {
        return clone_vtables::clone_table(kind, clone_vtables::Space::Item).map(|table| (table as *const usize, true));
    }
    if kind < 0 {
        return None;
    }
    Some(((crate::game::addr(clone_vtables::ITEM_OBJECT_VTABLE)) as *const usize, false))
}

unsafe fn fighter_vtable(kind: i32) -> Option<(*const usize, bool)> {
    if clone_base(kind).is_some() {
        return clone_vtables::clone_table(kind, clone_vtables::Space::Fighter).map(|table| (table as *const usize, true));
    }
    if !(0..94).contains(&kind) {
        return None;
    }
    let class = *((crate::game::addr(FIGHTER_CLASS_TABLE) + kind as usize * 8) as *const u64);
    if class == 0 {
        return None;
    }
    let table = *(class as *const u64) as *const usize;
    (!table.is_null()).then_some((table, false))
}

unsafe fn weapon_vtable(kind: i32) -> Option<(*const usize, bool)> {
    if custom_articles::custom_weapon_source_kind(kind).is_some() {
        return clone_vtables::clone_table(kind, clone_vtables::Space::Weapon).map(|table| (table as *const usize, true));
    }
    if !(0..custom_articles::FIRST_CUSTOM_WEAPON_KIND).contains(&kind) {
        return None;
    }
    let class = clone_vtables::vanilla_weapon_class(kind);
    if class == 0 {
        return None;
    }
    let table = *(class as *const usize) as *const usize;
    (!table.is_null()).then_some((table, false))
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_vtable_entry_v1(
    kind: i32,
    index: u32,
    is_weapon: u32,
    want_pointer: u32,
) -> u64 {
    let Some(space) = clone_vtables::Space::from_code(is_weapon) else {
        return 0;
    };
    let table = match space {
        clone_vtables::Space::Fighter => fighter_vtable(kind),
        clone_vtables::Space::Weapon => weapon_vtable(kind),
        clone_vtables::Space::Item => item_vtable(kind),
    };
    let Some((table, private)) = table else {
        return 0;
    };
    if index as usize >= space.slots() {
        return 0;
    }
    let entry = table.add(index as usize);
    let text = game::raw_text_base();
    if want_pointer != 0 {
        if private || (entry as usize) < text {
            return 0;
        }
        return (entry as usize - text) as u64;
    }
    if private && clone_vtables::is_overridden(kind, space, index as usize) {
        return 0;
    }
    let function = *entry;
    if function < text || function >= text_end() {
        return 0;
    }
    (function - text) as u64
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_vtable_override_v1(
    kind: i32,
    index: u32,
    is_weapon: u32,
    function: u64,
) -> u64 {
    let Some(space) = clone_vtables::Space::from_code(is_weapon) else {
        return 0;
    };
    clone_vtables::set_entry(kind, index, space, function as usize, 0).unwrap_or(0) as u64
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_vtable_override_v2(
    kind: i32,
    index: u32,
    is_weapon: u32,
    function: u64,
    original_out: u64,
) -> u32 {
    let Some(space) = clone_vtables::Space::from_code(is_weapon) else {
        return 0;
    };
    u32::from(clone_vtables::set_entry(kind, index, space, function as usize, original_out as usize).is_some())
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_fighter_manifest_register_v1(
    label: *const c_char,
    text: *const c_char,
) -> i32 {
    let (Some(label), Some(text)) = (try_borrow_str(label), try_borrow_str(text)) else {
        return ERROR_NULL;
    };
    fighter_packs::register_text(label, text)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_item_manifest_register_v1(
    label: *const c_char,
    text: *const c_char,
) -> i32 {
    let (Some(label), Some(text)) = (try_borrow_str(label), try_borrow_str(text)) else {
        return ERROR_NULL;
    };
    item_packs::register_text(label, text)
}

#[no_mangle]
pub extern "C" fn clone_engine_fighter_manifest_kinds_v1(out: *mut i32, capacity: u32) -> i32 {
    let kinds = fighter_packs::loaded_kinds();
    if !out.is_null() {
        for (index, kind) in kinds.iter().take(capacity as usize).enumerate() {
            unsafe { out.add(index).write(*kind) };
        }
    }
    kinds.len() as i32
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_fighter_manifest_directory_v1(
    kind: i32,
    out: *mut u8,
    capacity: u32,
) -> i32 {
    let Some(directory) = fighter_packs::pack_directory(kind) else {
        return ERROR_CUSTOM_KIND;
    };
    let bytes = directory.as_bytes();
    if out.is_null() || (capacity as usize) <= bytes.len() {
        return bytes.len() as i32 + 1;
    }
    core::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
    out.add(bytes.len()).write(0);
    RESULT_OK
}

pub(crate) unsafe fn register_v1_with(
    registration: *const CloneRegistrationV1,
    manifest: Option<ManifestRegistration>,
) -> i32 {
    if registration.is_null() {
        return ERROR_NULL;
    }

    let api_version = core::ptr::read_unaligned(registration.cast::<u32>());
    let struct_size = core::ptr::read_unaligned(registration.cast::<u32>().add(1));
    if api_version != API_VERSION_V1 {
        return ERROR_VERSION;
    }
    if struct_size < core::mem::size_of::<CloneRegistrationV1>() as u32 {
        return ERROR_STRUCT_SIZE;
    }
    let mut registration = core::ptr::read(registration);
    let _registration_guard = registration_gate().lock().unwrap();
    if REGISTRATION_CLOSED.load(core::sync::atomic::Ordering::Acquire) {
        return ERROR_REGISTRATION_CLOSED;
    }

    smashline_vtables::install();

    #[cfg(feature = "native_table_backend")]
    if native_tables::status() & BACKEND_STATUS_STATIC_TABLES_READY == 0 {
        let waited = native_tables::ensure_static_tables(REGISTRATION_INIT_WAIT_MILLIS);
        let ready = native_tables::status() & BACKEND_STATUS_STATIC_TABLES_READY != 0;
        if !REGISTRATION_TOO_EARLY_LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
            skyline::println!(
                "[clone_engine] a registration arrived before the engine's own init: built the static tables from the caller's thread after {waited}ms, ready={ready}"
            );
        }
        if !ready {
            return ERROR_BACKEND_UNAVAILABLE;
        }
    }

    let automatic = registration.custom_kind == KIND_AUTO;
    if automatic {
        match allocate_custom_kind(registration.fighter_kind_name, registration.resource_name) {
            Ok(kind) => registration.custom_kind = kind,
            Err(error) => return error,
        }
    }

    if registration.custom_kind < FIRST_CUSTOM_KIND {
        return ERROR_CUSTOM_KIND;
    }
    if registration.custom_kind > clone_engine_max_custom_kind() {
        return ERROR_BACKEND_UNAVAILABLE;
    }
    if !(0..=93).contains(&registration.base_kind) {
        return ERROR_BASE_KIND;
    }
    if registration.color_count == 0
        || registration.color_count > 255
        || registration.color_start > 255
        || registration.color_start + registration.color_count > 256
    {
        return ERROR_COLOR_RANGE;
    }
    if registration.flags & !(FLAG_OWNS_PARAM_RESOURCES | FLAG_KIRBY_COPY_FULL_MODEL) != 0
        || registration.reserved.iter().any(|value| *value != 0)
    {
        return ERROR_UNSUPPORTED;
    }
    if registration.effect_namespace > 0x1000 {
        return ERROR_NAMESPACE;
    }
    if registration.copy_status_count < 0
        || (registration.copy_status_count == 0 && registration.copy_status_first >= 0)
        || (registration.copy_status_count > 0 && registration.copy_status_first < 0)
    {
        return ERROR_UNSUPPORTED;
    }
    if registration.article_count > 256
        || (registration.article_count > 0 && registration.articles.is_null())
        || (registration.article_count == 0 && registration.article_namespace != 0)
        || registration.article_namespace > 0x0fff
    {
        return ERROR_ARTICLE;
    }

    let ui_chara = match read_registration_name(registration.ui_chara) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let fighter_kind_name = match read_registration_name(registration.fighter_kind_name) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let resource_name = match read_registration_name(registration.resource_name) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let base_resource_name = match read_registration_name(registration.base_resource_name) {
        Ok(value) => value,
        Err(error) => return error,
    };

    let raw_articles = if registration.article_count == 0 {
        &[][..]
    } else {
        core::slice::from_raw_parts(registration.articles, registration.article_count as usize)
    };
    let mut article_kinds = HashSet::new();
    let mut article_names = Vec::with_capacity(raw_articles.len());
    for article in raw_articles {
        if article.reserved != 0
            || article.base_weapon_kind < 0
            || !article_kinds.insert(article.base_weapon_kind)
        {
            return ERROR_ARTICLE;
        }
        let name = match read_registration_name(article.file_name) {
            Ok(value) => value,
            Err(_) => return ERROR_ARTICLE,
        };
        article_names.push((article.base_weapon_kind, name));
    }

    let mut definitions = clone_definitions().write().unwrap();
    let mut bases = registry().write().unwrap();
    if let Some(existing) = definitions
        .iter()
        .copied()
        .find(|definition| definition.kind == registration.custom_kind)
    {
        let same_articles = existing.articles.len() == article_names.len()
            && existing
                .articles
                .iter()
                .zip(article_names.iter())
                .all(|(existing, requested)| {
                    existing.base_weapon_kind == requested.0 && existing.file_name == requested.1
                });
        let identical = existing.base_kind == registration.base_kind
            && existing.ui_chara == ui_chara
            && existing.fighter_kind_name == fighter_kind_name
            && existing.resource_name == resource_name
            && existing.base_resource_name == base_resource_name
            && u32::from(existing.color_start) == registration.color_start
            && u32::from(existing.color_count) == registration.color_count
            && existing.copy_status_first == registration.copy_status_first
            && existing.copy_status_count == registration.copy_status_count
            && (registration.effect_namespace == 0
                || existing.effect_namespace == registration.effect_namespace)
            && existing.kirby_copy_full_model
                == (registration.flags & FLAG_KIRBY_COPY_FULL_MODEL != 0)
            && same_articles;
        if !identical && !existing.from_manifest {
            return ERROR_DUPLICATE;
        }
        bases.insert(existing.kind, existing.base_kind);
        if identical {
            skyline::println!(
                "[clone_engine] API v1 accepted existing descriptor for kind {} ({})",
                existing.kind,
                existing.resource_name
            );
        } else {
            skyline::println!(
                "[clone_engine] {} already registered from fighter.toml as kind {}; the plugin's descriptor is ignored{}{}{}{}",
                existing.resource_name,
                existing.kind,
                if existing.base_kind != registration.base_kind { " (base differs)" } else { "" },
                if u32::from(existing.color_start) != registration.color_start
                    || u32::from(existing.color_count) != registration.color_count
                {
                    " (costumes differ)"
                } else {
                    ""
                },
                if existing.copy_status_first != registration.copy_status_first
                    || existing.copy_status_count != registration.copy_status_count
                {
                    " (kirby family differs)"
                } else {
                    ""
                },
                if !same_articles { " (articles differ)" } else { "" }
            );
        }
        return if automatic { existing.kind } else { RESULT_OK };
    }

    let effect_namespace_holder = |namespace: u32| {
        definitions
            .iter()
            .find(|definition| {
                definition.base_kind == registration.base_kind && definition.effect_namespace == namespace
            })
            .map(|definition| definition.resource_name)
    };
    let effect_namespace = {
        let asked = registration.effect_namespace;
        let holder = if asked == 0 { None } else { effect_namespace_holder(asked) };
        if asked != 0 && holder.is_none() {
            asked
        } else {
            match (1..=0x1000).find(|namespace| effect_namespace_holder(*namespace).is_none()) {
                Some(namespace) => {
                    if let Some(holder) = holder {
                        skyline::println!(
                            "[clone_engine] {resource_name} asked for effect namespace {asked}, which {holder} holds on base {}; it gets {namespace} instead (a runtime tag, nothing in the pack refers to it)",
                            registration.base_kind
                        );
                    }
                    namespace
                }
                None => return ERROR_NAMESPACE,
            }
        }
    };
    let article_namespace_holder = |namespace: u32| {
        definitions
            .iter()
            .find(|definition| !definition.articles.is_empty() && definition.article_namespace == namespace)
            .map(|definition| definition.resource_name)
    };
    let article_namespace = if registration.article_count == 0 {
        0
    } else {
        let asked = registration.article_namespace;
        let holder = if asked == 0 { None } else { article_namespace_holder(asked) };
        if asked != 0 && holder.is_none() {
            asked
        } else {
            match (1..=0x0fff).rev().find(|namespace| article_namespace_holder(*namespace).is_none()) {
                Some(namespace) => {
                    if let Some(holder) = holder {
                        skyline::println!(
                            "[clone_engine] {resource_name} asked for article namespace {asked}, which {holder} holds; it gets {namespace} instead (a runtime tag, nothing in the pack refers to it)"
                        );
                    }
                    namespace
                }
                None => return ERROR_NAMESPACE,
            }
        }
    };

    let taken = if bases.contains_key(&registration.custom_kind) {
        Some(format!("kind {} is already registered", registration.custom_kind))
    } else if let Some(other) = definitions.iter().find(|definition| {
        definition.ui_chara == ui_chara
            || definition.fighter_kind_name == fighter_kind_name
            || definition.resource_name == resource_name
    }) {
        Some(format!(
            "{} (kind {}) already uses ui_chara {ui_chara}, fighter kind name {fighter_kind_name} or resource name {resource_name}",
            other.resource_name, other.kind
        ))
    } else {
        definitions
            .iter()
            .find(|definition| {
                definition.base_kind == registration.base_kind && definition.effect_namespace == effect_namespace
            })
            .map(|other| {
                format!(
                    "{} (kind {}) already holds effect namespace {effect_namespace} on base {}",
                    other.resource_name, other.kind, registration.base_kind
                )
            })
    };
    if let Some(reason) = taken {
        skyline::println!("[clone_engine] {resource_name} refused (ERROR_DUPLICATE): {reason}");
        return ERROR_DUPLICATE;
    }

    let copy_status_first = if registration.copy_status_count > 0 {
        let count = registration.copy_status_count;
        let overlaps = |first: i32, definition: &CloneDefinition| {
            definition.copy_status_count > 0
                && first < definition.copy_status_first + definition.copy_status_count
                && definition.copy_status_first < first + count
        };
        if registration.copy_status_first == 0 {
            let mut candidate = KIRBY_COPY_FAMILY_POOL_FIRST;
            while candidate + count <= KIRBY_COPY_FAMILY_POOL_END
                && definitions.iter().any(|definition| overlaps(candidate, definition))
            {
                candidate += 1;
            }
            if candidate + count > KIRBY_COPY_FAMILY_POOL_END {
                skyline::println!(
                    "[clone_engine] {resource_name}: no free Kirby copy status family of {count} in {KIRBY_COPY_FAMILY_POOL_FIRST:#x}..{KIRBY_COPY_FAMILY_POOL_END:#x}"
                );
                return ERROR_DUPLICATE;
            }
            skyline::println!(
                "[clone_engine] {resource_name}: Kirby copy status family assigned at {candidate:#x} (+{count})"
            );
            candidate
        } else {
            if let Some(other) = definitions
                .iter()
                .find(|definition| overlaps(registration.copy_status_first, definition))
            {
                skyline::println!(
                    "[clone_engine] {resource_name}: Kirby copy status family {:#x}+{count} overlaps {}'s {:#x}+{}",
                    registration.copy_status_first,
                    other.resource_name,
                    other.copy_status_first,
                    other.copy_status_count
                );
                return ERROR_DUPLICATE;
            }
            registration.copy_status_first
        }
    } else {
        registration.copy_status_first
    };

    let (ui_chara, _) = leak_registration_name(ui_chara);
    let (fighter_kind_name, _) = leak_registration_name(fighter_kind_name);
    let (resource_name_upper, resource_name_title) = resource_name_variants(&resource_name);
    let (resource_name, resource_name_cstr) = leak_registration_name(resource_name);
    let (_, resource_name_upper_cstr) = leak_registration_name(resource_name_upper);
    let (_, resource_name_title_cstr) = leak_registration_name(resource_name_title);
    let (base_resource_name, base_resource_name_cstr) = leak_registration_name(base_resource_name);
    let articles = article_names
        .into_iter()
        .map(|(base_weapon_kind, file_name)| {
            let (file_name, file_name_cstr) = leak_registration_name(file_name);
            CloneArticleDefinition {
                base_weapon_kind,
                file_name,
                file_name_cstr,
            }
        })
        .collect::<Vec<_>>();
    let articles = Box::leak(articles.into_boxed_slice());
    let definition = Box::leak(Box::new(CloneDefinition {
        kind: registration.custom_kind,
        base_kind: registration.base_kind,
        ui_chara,
        fighter_kind_name,
        resource_name,
        resource_name_cstr,
        resource_name_upper_cstr,
        resource_name_title_cstr,
        base_resource_name,
        base_resource_name_cstr,
        color_start: registration.color_start as u8,
        color_count: registration.color_count as u8,
        copy_status_first,
        copy_status_count: registration.copy_status_count,
        effect_namespace,
        article_namespace,
        articles,
        owns_param_resources: registration.flags & FLAG_OWNS_PARAM_RESOURCES != 0,
        param_file: Box::leak(Box::new(core::sync::atomic::AtomicU8::new(PARAM_FILE_UNCHECKED))),
        kirby_copy_full_model: registration.flags & FLAG_KIRBY_COPY_FULL_MODEL != 0,
        css: manifest_css(manifest.as_ref()),
        from_manifest: manifest.is_some(),
    }));

    #[cfg(feature = "native_table_backend")]
    if !native_tables::publish_descriptor(definition) {
        return ERROR_BACKEND_UNAVAILABLE;
    }

    definitions.push(definition);
    bases.insert(definition.kind, definition.base_kind);
    skyline::println!(
        "[clone_engine] API v1 registered kind {} -> base {} resource={} colors={}..{} effects={} articles={} (namespace={})",
        definition.kind,
        definition.base_kind,
        definition.resource_name,
        definition.color_start,
        definition.color_start as u16 + definition.color_count as u16 - 1,
        definition.effect_namespace,
        definition.articles.len(),
        definition.article_namespace
    );
    if automatic {
        definition.kind
    } else {
        RESULT_OK
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_get_base_kind(custom_kind: i32) -> i32 {
    clone_base(custom_kind)
        .or_else(|| clone_definition(custom_kind).map(|definition| definition.base_kind))
        .unwrap_or(-1)
}

const FIGHTER_MANAGER_HOLDER: usize = 0x52b84f8;

#[no_mangle]
pub extern "C" fn clone_engine_fighter_manager_v1() -> usize {
    unsafe { core::ptr::read_volatile(crate::game::addr(FIGHTER_MANAGER_HOLDER) as *const usize) }
}

#[no_mangle]
pub extern "C" fn clone_engine_is_custom_kind(kind: i32) -> i32 {
    i32::from(clone_definition(kind).is_some())
}

#[no_mangle]
pub extern "C" fn clone_engine_fighter_name_for_kind_v1(kind: i32) -> *const core::ffi::c_char {
    match clone_definition(kind) {
        Some(definition) => definition.resource_name_cstr.as_ptr().cast(),
        None => core::ptr::null(),
    }
}

#[cfg(feature = "css_slot")]
#[no_mangle]
pub extern "C" fn clone_engine_weapon_name_for_kind_v1(
    weapon_kind: i32,
) -> *const core::ffi::c_char {
    match custom_articles::custom_weapon_name(weapon_kind) {
        Some(name) => name.as_ptr() as *const core::ffi::c_char,
        None => core::ptr::null(),
    }
}

#[cfg(feature = "css_slot")]
#[no_mangle]
pub extern "C" fn clone_engine_weapon_owner_name_for_kind_v1(
    weapon_kind: i32,
) -> *const core::ffi::c_char {
    match custom_articles::custom_weapon_owner_name(weapon_kind) {
        Some(name) => name.as_ptr() as *const core::ffi::c_char,
        None => core::ptr::null(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_shared_hook_v1(offset: u64, callback: usize) -> i32 {
    if callback == 0 || offset == 0 {
        return ERROR_NULL;
    }
    let _registration_guard = registration_gate().lock().unwrap();
    if REGISTRATION_CLOSED.load(core::sync::atomic::Ordering::Acquire) {
        return ERROR_REGISTRATION_CLOSED;
    }
    let Some(spec) = shared_hooks::legacy_spec(offset) else {
        return ERROR_HOOK_PREFLIGHT;
    };
    shared_hook_result(
        shared_hooks::register(game::raw_text_base(), relocated_hook_spec(spec), callback),
        offset,
        callback,
    )
}

fn shared_hook_result(result: shared_hooks::Register, offset: u64, callback: usize) -> i32 {
    match result {
        shared_hooks::Register::Ok => {
            skyline::println!(
                "[sharedhook] callback {callback:#x} joined the chain for {offset:#x}"
            );
            RESULT_OK
        }
        shared_hooks::Register::Duplicate => RESULT_OK,
        shared_hooks::Register::Full => dionysus_api::ERROR_HOOK_CAPACITY,
        shared_hooks::Register::Unavailable => ERROR_HOOK_UNAVAILABLE,
        shared_hooks::Register::Invalid => ERROR_HOOK_PREFLIGHT,
        shared_hooks::Register::InvalidAbi => ERROR_HOOK_ABI,
        shared_hooks::Register::Preflight => ERROR_HOOK_PREFLIGHT,
        shared_hooks::Register::Conflict => ERROR_HOOK_CONFLICT,
        shared_hooks::Register::InstallFailed => ERROR_HOOK_INSTALL,
    }
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_shared_hook_v2(
    registration: *const SharedHookRegistrationV1,
) -> i32 {
    if registration.is_null() {
        return ERROR_NULL;
    }
    let _registration_guard = registration_gate().lock().unwrap();
    if REGISTRATION_CLOSED.load(core::sync::atomic::Ordering::Acquire) {
        return ERROR_REGISTRATION_CLOSED;
    }
    let registration = &*registration;
    if registration.api_version != dionysus_api::API_VERSION_V1 {
        return ERROR_VERSION;
    }
    if (registration.struct_size as usize) < core::mem::size_of::<SharedHookRegistrationV1>() {
        return ERROR_STRUCT_SIZE;
    }
    if registration.callback == 0 || registration.offset == 0 {
        return ERROR_NULL;
    }
    if registration.flags != 0
        || registration.reserved_u32 != 0
        || registration.reserved.iter().any(|value| *value != 0)
    {
        return ERROR_UNSUPPORTED;
    }

    let spec = shared_hooks::HookSpec {
        offset: registration.offset,
        expected_opcodes: registration.expected_opcodes,
        abi: registration.abi,
        argument_count: registration.argument_count,
    };
    shared_hook_result(
        shared_hooks::register(game::raw_text_base(), relocated_hook_spec(spec), registration.callback),
        registration.offset,
        registration.callback,
    )
}

fn relocated_hook_spec(spec: shared_hooks::HookSpec) -> shared_hooks::HookSpec {
    let offset = spec.offset as usize;
    let mut expected_opcodes = spec.expected_opcodes;
    for (index, word) in expected_opcodes.iter_mut().enumerate() {
        *word = game::opcode(offset + index * 4, *word);
    }
    shared_hooks::HookSpec {
        offset: game::rel(offset) as u64,
        expected_opcodes,
        ..spec
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_shared_hook_status_v1() -> u32 {
    let mut status = shared_hooks::status();
    if REGISTRATION_CLOSED.load(core::sync::atomic::Ordering::Acquire) {
        status |= SHARED_HOOK_STATUS_REGISTRATION_CLOSED;
    }
    status
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_shared_hook_original_v1(
    offset: u64,
    args: *const [u64; 6],
) -> u64 {
    if args.is_null() || offset == 0 {
        return 0;
    }
    shared_hooks::original(game::raw_text_base(), game::rel(offset as usize) as u64, &*args)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_shared_hook_words_v1(offset: u64, words: *mut [u32; 4]) -> u32 {
    if words.is_null() || offset == 0 {
        return 0;
    }
    let _registration_guard = registration_gate().lock().unwrap();
    match shared_hooks::recorded_words(game::raw_text_base(), game::rel(offset as usize) as u64) {
        Some(found) => {
            *words = found;
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_add_costume_slots_v1(
    identity: *const c_char,
    highest_slot: i32,
) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        if identity.is_null() {
            return ERROR_NULL;
        }
        let Ok(identity) = core::ffi::CStr::from_ptr(identity).to_str() else {
            return ERROR_NAME;
        };
        if costume_slots::register(identity, highest_slot) {
            return RESULT_OK;
        }
        ERROR_COLOR_RANGE
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (identity, highest_slot);
        ERROR_UNSUPPORTED
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_entry_is_kind(entry_id: i32, expected_kind: i32) -> i32 {
    i32::from(clone_engine_get_entry_kind(entry_id) == expected_kind)
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_fighter_kind_v1(module_accessor: u64) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        if module_accessor != 0
            && smash::app::utility::get_category(
                &mut *(module_accessor as *mut smash::app::BattleObjectModuleAccessor),
            ) == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
        {
            if let Some(kind) = entry_groups::clone_of_boma(
                module_accessor as *mut smash::app::BattleObjectModuleAccessor,
            ) {
                return kind;
            }
        }
    }
    -1
}

#[no_mangle]
pub extern "C" fn clone_engine_get_entry_kind(entry_id: i32) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        if (0..8).contains(&entry_id) {
            return entry_custom_kind(entry_id as u8).unwrap_or(-1);
        }
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_clone_article_v1(
    source_owner: *const c_char,
    source_weapon_kind: i32,
    destination_owner: *const c_char,
    name: *const c_char,
) -> i32 {
    custom_articles::register(
        source_owner,
        source_weapon_kind,
        destination_owner,
        core::ptr::null(),
        name,
    )
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_clone_article_for_v1(
    source_owner: *const c_char,
    source_weapon_kind: i32,
    destination_owner: *const c_char,
    resource_owner: *const c_char,
    name: *const c_char,
) -> i32 {
    custom_articles::register(
        source_owner,
        source_weapon_kind,
        destination_owner,
        resource_owner,
        name,
    )
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_clone_copy_article_v1(
    target_kind: i32,
    source_owner: *const c_char,
    source_weapon_kind: i32,
    resource_owner: *const c_char,
    name: *const c_char,
) -> i32 {
    custom_articles::register_kirby_copy(
        target_kind,
        source_owner,
        source_weapon_kind,
        resource_owner,
        name,
    )
}

#[no_mangle]
pub extern "C" fn clone_engine_copy_article_index_v1(target_kind: i32, weapon_kind: i32) -> i32 {
    custom_articles::kirby_copy_index(target_kind, weapon_kind).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn clone_engine_log_v1(text: *const u8, len: usize) {
    if text.is_null() || len == 0 || len > 0x1000 {
        return;
    }
    let bytes = unsafe { core::slice::from_raw_parts(text, len) };
    let Ok(text) = core::str::from_utf8(bytes) else {
        return;
    };
    unsafe { dbg_out(text) };
    skyline::println!("{}", text);
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_article_index_v1(fighter_kind: i32, weapon_kind: i32) -> i32 {
    #[skyline::from_offset(crate::game::rel(OFF_STATIC_FIGHTER_DATA))]
    fn static_fighter_data(kind: i32) -> *const StaticFighterData;

    let blob = static_fighter_data(clone_base(fighter_kind).unwrap_or(fighter_kind));
    if blob.is_null() || (*blob).static_article_info.is_null() {
        return -1;
    }
    let table = *((*blob).static_article_info as *const custom_articles::StaticArticleData);
    custom_articles::index_of(&table, weapon_kind).unwrap_or(-1)
}

#[inline(always)]
fn clone_base(kind: i32) -> Option<i32> {
    #[cfg(not(any(
        feature = "diag_identity",
        feature = "diag_swap",
        feature = "diag_article"
    )))]
    if kind <= 93 {
        return None;
    }
    registry().read().unwrap().get(&kind).copied()
}

#[cfg(any(
    feature = "diag_identity",
    feature = "diag_swap",
    feature = "diag_article"
))]
macro_rules! diag_reroute_log {
    ($name:ident, $kind:expr, $base:expr) => {{
        static LOGGED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
        if !LOGGED.swap(true, core::sync::atomic::Ordering::Relaxed) {
            skyline::println!(
                "[clone_engine] REROUTE ({}): kind {} -> base {} (via chain)",
                stringify!($name),
                $kind,
                $base
            );
        }
    }};
}
#[cfg(not(any(
    feature = "diag_identity",
    feature = "diag_swap",
    feature = "diag_article"
)))]
macro_rules! diag_reroute_log {
    ($name:ident, $kind:expr, $base:expr) => {{}};
}

static PENDING_AGENT_KIND: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("pending_agent_kind");

static PENDING_WEAPON_KIND: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("pending_weapon_kind");

#[no_mangle]
pub extern "C" fn clone_engine_pending_agent_kind_v1() -> i32 {
    let kind = PENDING_AGENT_KIND
        .active(unsafe { current_thread_key() })
        .unwrap_or(-1);
    if kind >= 0 {
        static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            skyline::println!("[agentname] #{n} smashline asked; answering clone kind {kind}");
        }
    }
    kind
}

#[no_mangle]
pub extern "C" fn clone_engine_pending_weapon_kind_v1() -> i32 {
    let kind = PENDING_WEAPON_KIND
        .active(unsafe { current_thread_key() })
        .unwrap_or(-1);
    if kind >= 0 {
        static SEEN: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = SEEN.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            skyline::println!("[agentname] #{n} smashline asked; answering weapon kind {kind}");
        }
    }
    kind
}

#[cfg(feature = "css_slot")]
pub(crate) fn enter_pending_weapon_kind(
    kind: i32,
) -> crate::thread_context::ScopedKindGuard<'static> {
    PENDING_WEAPON_KIND.enter(unsafe { current_thread_key() }, kind)
}

pub(crate) fn pending_weapon_kind() -> Option<i32> {
    PENDING_WEAPON_KIND
        .active(unsafe { current_thread_key() })
        .filter(|kind| *kind >= 0)
}

macro_rules! nonshare_hook {
    ($name:ident, $off:expr) => {
        #[skyline::hook(offset = crate::game::rel($off))]
        unsafe fn $name(object: *mut u8, x1: u64, x2: u64) -> u64 {
            let kind_ptr = (object as usize + 0xC) as *mut i32;
            let kind = *kind_ptr;
            if let Some(base) = clone_base(kind) {
                diag_reroute_log!($name, kind, base);
                let _pending = PENDING_AGENT_KIND.enter(current_thread_key(), kind);
                *kind_ptr = base;
                let agent = call_original!(object, x1, x2);
                *kind_ptr = kind;
                return agent;
            }
            #[cfg(feature = "css_slot")]
            let _partner_name = entry_groups::hold_partner_agent_name(object as u64);
            call_original!(object, x1, x2)
        }
    };
}

nonshare_hook!(status_script_hook, OFF_STATUS);
nonshare_hook!(animcmd_game_hook, OFF_GAME);
nonshare_hook!(animcmd_effect_hook, OFF_EFFECT);
nonshare_hook!(animcmd_expression_hook, OFF_EXPRESSION);
nonshare_hook!(animcmd_sound_hook, OFF_SOUND);

macro_rules! share_hook {
    ($name:ident, $off:expr) => {
        #[skyline::hook(offset = crate::game::rel($off))]
        unsafe fn $name(kind: u64, x1: u64, x2: u64, x3: u64) -> u64 {
            if let Some(base) = clone_base(kind as i32) {
                diag_reroute_log!($name, kind as i32, base);
                let _pending = PENDING_AGENT_KIND.enter(current_thread_key(), kind as i32);
                let _agent_name = smashline_names::hold_clone_fighter_name(Some(kind as i32));
                return call_original!(base as u64, x1, x2, x3);
            }
            call_original!(kind, x1, x2, x3)
        }
    };
}

share_hook!(animcmd_game_share_hook, OFF_GAME_SHARE);
share_hook!(animcmd_effect_share_hook, OFF_EFFECT_SHARE);
share_hook!(animcmd_expression_share_hook, OFF_EXPRESSION_SHARE);
share_hook!(animcmd_sound_share_hook, OFF_SOUND_SHARE);

static TEXT_END: OnceLock<usize> = OnceLock::new();

fn text_end() -> usize {
    *TEXT_END.get_or_init(|| unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Rodata) as usize
    })
}

fn caller_is_outside_main_text(lr: usize) -> bool {
    if lr == 0 {
        return false;
    }
    let base = game::raw_text_base();
    let end = text_end();
    if base == 0 || end <= base {
        return false;
    }
    lr < base || lr >= end
}

#[cfg(any(
    feature = "clone_runtime",
    feature = "diag_article",
    feature = "diag_article_initspoof",
    feature = "true_kind",
    feature = "diag_pathtrace",
    feature = "diag_item_kind"
))]
#[cfg(any(
    feature = "clone_runtime",
    feature = "diag_article",
    feature = "diag_article_initspoof",
    feature = "true_kind",
    feature = "diag_pathtrace",
    feature = "diag_item_kind"
))]
pub(crate) fn dbg_out_public(s: &str) {
    unsafe { dbg_out(s) };
}

pub(crate) fn dbg_log_public(s: &str) {
    unsafe { dbg_out(s) };
    skyline::println!("{}", s);
}

#[cfg(not(target_arch = "aarch64"))]
unsafe fn dbg_out(_s: &str) {}

#[cfg(target_arch = "aarch64")]
unsafe fn dbg_out(s: &str) {
    let b = s.as_bytes();
    core::arch::asm!(
        "svc 0x27",
        inout("x0") b.as_ptr() => _,
        inout("x1") b.len() => _,
        clobber_abi("C"),
        options(nostack),
    );
}

macro_rules! dbg_log {
    ($($arg:tt)*) => {{
        let __s = format!($($arg)*);
        #[allow(unused_unsafe)]
        unsafe { dbg_out(&__s) };
        skyline::println!("{}", __s);
    }};
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn clone_kind_of_object(object: u64) -> Option<i32> {
    if object == 0 {
        return None;
    }
    let kind = core::ptr::read_volatile((object + 0xc) as *const i32);
    if !(0..FIRST_CUSTOM_KIND).contains(&kind) && clone_definition(kind).is_none() {
        return None;
    }
    let entry_id = core::ptr::read_volatile((object + 0x10) as *const i32);
    if !(0..8).contains(&entry_id) {
        return None;
    }
    let object_id = core::ptr::read_volatile((object + 0x8) as *const u32);
    if let Some(member) = entry_groups::same_base_member(entry_id, object_id) {
        return Some(member);
    }
    let clone_kind = entry_custom_kind(entry_id as u8)?;
    let definition = clone_definition(clone_kind)?;
    if definition.base_kind == kind {
        return Some(clone_kind);
    }
    entry_groups::member_of(clone_kind, kind)
}

macro_rules! clone_fighter_acmd_hooks {
    ($($name:ident($offset:expr, $category:expr);)*) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset))]
            unsafe fn $name(object: u64, boma: u64, lua_state: u64) -> u64 {
                let clone_kind = clone_kind_of_object(object);
                let agent = {
                    let _pending =
                        clone_kind.map(|kind| PENDING_AGENT_KIND.enter(current_thread_key(), kind));
                    let _agent_name = smashline_names::hold_clone_fighter_name(clone_kind);
                    call_original!(object, boma, lua_state)
                };
                if let Some(kind) = clone_kind {
                    final_grafts::on_acmd_agent($category, object, boma, lua_state, agent, kind);
                }
                agent
            }
        )*

        #[cfg(feature = "css_slot")]
        fn install_smashline_name_bridge_hooks() {
            skyline::install_hooks!($($name),*);
        }
    };
}

#[cfg(feature = "css_slot")]
clone_fighter_acmd_hooks! {
    clone_fighter_acmd_game(0x64c310, 0);
    clone_fighter_acmd_effect(0x64c930, 1);
    clone_fighter_acmd_expression(0x64cf50, 2);
    clone_fighter_acmd_sound(0x64d570, 3);
}

#[cfg(all(feature = "css_slot", feature = "diag_pocket"))]
static EFFECT_REQ_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "css_slot", feature = "diag_pocket"))]
unsafe fn effect_request_probe(tag: &str, boma: u64, hash: u64) {
    if boma == 0 || EFFECT_REQ_LOG.load(core::sync::atomic::Ordering::Relaxed) >= 48 {
        return;
    }
    let accessor = boma as *mut smash::app::BattleObjectModuleAccessor;
    let category = smash::app::utility::get_category(&mut *accessor);
    let kind = smash::app::utility::get_kind(&mut *accessor);

    let (owner_kind, owner_entry) =
        if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_WEAPON {
            let owner_id = smash::app::lua_bind::WorkModule::get_int(
                accessor,
                *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_ACTIVATE_FOUNDER_ID,
            );
            if !smash::app::sv_battle_object::is_active(owner_id as u32) {
                return;
            }
            let owner = smash::app::sv_battle_object::module_accessor(owner_id as u32);
            (
                smash::app::utility::get_kind(&mut *owner),
                smash::app::lua_bind::WorkModule::get_int(
                    owner,
                    *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                ) as i32,
            )
        } else if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER {
            (
                kind,
                smash::app::lua_bind::WorkModule::get_int(
                    accessor,
                    *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
                ) as i32,
            )
        } else {
            return;
        };

    let owner_clone = (0..8)
        .contains(&owner_entry)
        .then(|| entry_custom_kind(owner_entry as u8))
        .flatten();
    if owner_clone.is_none() && custom_articles::custom_weapon_source_kind(kind).is_none() {
        return;
    }
    let n = EFFECT_REQ_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 48 {
        dbg_log!(
            "[effectreq] #{n} {tag} cat={category} kind={kind} owner_kind={owner_kind} owner_entry={owner_entry} owner_clone={owner_clone:?} hash={hash:#x}"
        );
    }
}

#[cfg(all(feature = "css_slot", feature = "diag_pocket"))]
#[skyline::hook(offset = crate::game::rel(0x20176b0), inline)]
unsafe fn effect_req_probe(ctx: &mut skyline::hooks::InlineCtx) {
    effect_request_probe("req", ctx.registers[0].x(), ctx.registers[1].x());
}

#[cfg(all(feature = "css_slot", feature = "diag_pocket"))]
#[skyline::hook(offset = crate::game::rel(0x2017730), inline)]
unsafe fn effect_req_follow_probe(ctx: &mut skyline::hooks::InlineCtx) {
    effect_request_probe("req_follow", ctx.registers[0].x(), ctx.registers[1].x());
}

#[cfg(all(feature = "css_slot", feature = "diag_pocket"))]
static POCKET_NAME_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(all(feature = "css_slot", feature = "diag_pocket"))]
#[skyline::hook(offset = crate::game::rel(0x17e0840))]
unsafe fn weapon_name_owner_tables(a0: u64, kind: i32, a2: u64, a3: u64, a4: u64, a5: u64) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let source = custom_articles::custom_weapon_source_kind(kind);
    if source.is_none() && kind < custom_articles::FIRST_CUSTOM_WEAPON_KIND {
        return call_original!(a0, kind, a2, a3, a4, a5);
    }
    let n = POCKET_NAME_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        dbg_log!(
            "[pocketname] #{n} kind={kind} source={source:?} caller=@{:#x}",
            game::offset_of(lr)
        );
    }
    call_original!(a0, kind, a2, a3, a4, a5)
}

#[cfg(feature = "css_slot")]
macro_rules! article_animcmd_agent_hooks {
    ($($name:ident($offset:expr, $tag:expr);)*) => {
        $(
            #[cfg(feature = "css_slot")]
            #[skyline::hook(offset = crate::game::rel($offset))]
            unsafe fn $name(object: *mut u8, boma: *mut u8, lua_state: *mut u8) -> *mut u8 {
                if object.is_null() {
                    return call_original!(object, boma, lua_state);
                }
                let kind_field = object.add(0xc) as *mut i32;
                let kind = core::ptr::read_volatile(kind_field);
                let Some(source) = custom_articles::custom_weapon_source_kind(kind) else {
                    return call_original!(object, boma, lua_state);
                };
                core::ptr::write_volatile(kind_field, source);
                let agent = {
                    let _pending = crate::enter_pending_weapon_kind(kind);
                    let _names = smashline_names::hold_clone_weapon_names(source, kind);
                    call_original!(object, boma, lua_state)
                };
                core::ptr::write_volatile(kind_field, kind);
                let n = ARTICLE_ANIMCMD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                if n < 24 {
                    dbg_log!(
                        "[articleagent] {} creator: kind {kind} presented as {source}, agent {:#x}",
                        $tag,
                        agent as usize
                    );
                }
                agent
            }
        )*

        #[cfg(feature = "css_slot")]
        fn install_article_animcmd_agent_hooks() {
            skyline::install_hooks!($($name),*);
        }
    };
}

#[cfg(feature = "css_slot")]
static ARTICLE_ANIMCMD_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
article_animcmd_agent_hooks! {
    article_game_agent_create(0x33acde0, "game");
    article_effect_agent_create(0x33add40, "effect");
    article_sound_agent_create(0x33aeca0, "sound");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x64bbd0))]
unsafe fn clone_fighter_status_create(object: u64, boma: u64, lua_state: u64) -> u64 {
    let kind_field = (object + 0xc) as *mut i32;
    let kind = if object == 0 {
        -1
    } else {
        core::ptr::read_volatile(kind_field)
    };
    let spoof = clone_definition(kind).map(|definition| definition.base_kind);
    let status_clone_kind = clone_kind_of_object(object);
    let _pending =
        status_clone_kind.map(|clone_kind| PENDING_AGENT_KIND.enter(current_thread_key(), clone_kind));
    let _agent_name = smashline_names::hold_clone_fighter_name(status_clone_kind);
    if let Some(base) = spoof {
        core::ptr::write_volatile(kind_field, base);
    }
    let result = call_original!(object, boma, lua_state);
    if spoof.is_some() {
        core::ptr::write_volatile(kind_field, kind);
    }
    common_vtables::on_fighter_agent(object, result);
    final_grafts::on_status_agent(object, boma, lua_state, result, status_clone_kind);
    tether::on_status_agent(boma, status_clone_kind);
    result
}

#[no_mangle]
pub extern "C" fn clone_engine_copy_common_vtable_override_v2(
    kind: i32,
    slot: u32,
    function: usize,
    original_out: usize,
) -> u32 {
    #[cfg(feature = "css_slot")]
    {
        match common_vtables::register_copy(kind, slot, function, original_out) {
            Ok(_) => {
                dbg_log_public(&format!(
                    "[clone_engine] copy common vtable: kind {kind} slot {slot} ({}) taken; it runs on KIRBY while he holds this ability",
                    dionysus_core::common_vtable::slot_name(slot).unwrap_or("?"),
                ));
                1
            }
            Err(()) => {
                dbg_log_public(&format!(
                    "[clone_engine] copy common vtable: kind {kind} slot {slot} REFUSED; a copy hook needs a clone kind and a system line slot"
                ));
                0
            }
        }
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (kind, slot, function, original_out);
        0
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_common_vtable_override_v2(
    kind: i32,
    slot: u32,
    space: u32,
    function: usize,
    original_out: usize,
) -> u32 {
    #[cfg(feature = "css_slot")]
    {
        match common_vtables::register_followed(kind, slot, space, function, original_out) {
            Ok(_) => 1,
            Err(()) => {
                dbg_log_public(&format!(
                    "[clone_engine] common vtable: kind {kind} slot {slot} space {space} REFUSED"
                ));
                0
            }
        }
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (kind, slot, space, function, original_out);
        0
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_common_vtable_override_v1(
    kind: i32,
    slot: u32,
    space: u32,
    function: usize,
) -> usize {
    #[cfg(feature = "css_slot")]
    {
        match common_vtables::register(kind, slot, space, function) {
            Ok(previous) => {
                dbg_log_public(&format!(
                    "[clone_engine] common vtable: {} kind {kind} slot {slot} ({}) taken",
                    if space == common_vtables::SPACE_WEAPON { "weapon" } else { "fighter" },
                    dionysus_core::common_vtable::slot_name(slot).unwrap_or("?"),
                ));
                previous.unwrap_or(0)
            }
            Err(()) => {
                dbg_log_public(&format!(
                    "[clone_engine] common vtable: kind {kind} slot {slot} space {space} REFUSED"
                ));
                0
            }
        }
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (kind, slot, space, function);
        0
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_param_override_v1(
    kind: i32,
    slot: i32,
    param_type: u64,
    param_hash: u64,
    op: u32,
    value: f64,
) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        let key = (param_type, param_hash);
        let slots = [slot];
        let row_op = match op {
            param_overrides::OP_SET => Some(dionysus_core::fighter_param_row::Op::Set(value)),
            param_overrides::OP_MUL => Some(dionysus_core::fighter_param_row::Op::Mul(value)),
            _ => None,
        };
        let available = param_overrides::available();
        let (row, forward) = match row_op {
            Some(row_op) if param_type == dionysus_core::fighter_param_thrown::PARAM_THROWN => {
                let recorded = fighter_param_thrown::record(kind, slot, param_hash, row_op);
                (recorded.message, !recorded.recorded)
            }
            Some(_) if !available => (String::from("row: not recorded, ParamConfig missing"), true),
            Some(row_op) if param_type == dionysus_core::fighter_common_copies::COMMON => {
                let recorded = fighter_common_copies::record(kind, slot, param_hash, row_op);
                let forward = !recorded.recorded || op == param_overrides::OP_SET;
                (recorded.message, forward)
            }
            Some(row_op) => {
                let recorded = fighter_param_rows::record(kind, slot, param_type, param_hash, row_op);
                (recorded.message, !recorded.accessor_set_deferred)
            }
            None => (String::from("row: unknown op"), true),
        };
        let accepted = if forward {
            unsafe { param_overrides::push_to_param_config(kind, &slots, key, op, value) }
        } else {
            available || param_type == dionysus_core::fighter_param_thrown::PARAM_THROWN
        };
        let n = PARAM_REGISTER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < PARAM_REGISTER_LOG_LIMIT {
            dbg_log!(
                "[paramreg] #{n} kind={kind} slot={slot} key=({param_type:#x},{param_hash:#x}) op={op} value={value} paramconfig={accepted} {row}"
            );
        }
        i32::from(!accepted)
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (kind, slot, param_type, param_hash, op, value);
        -1
    }
}

#[no_mangle]
pub extern "C" fn clone_engine_param_int_override_v1(
    kind: i32,
    slot: i32,
    param_type: u64,
    param_hash: u64,
    value: i32,
) -> i32 {
    #[cfg(feature = "css_slot")]
    {
        let slots = [slot];
        let accepted = unsafe {
            param_overrides::push_int_to_param_config(kind, &slots, (param_type, param_hash), value)
        };
        let op = dionysus_core::fighter_param_row::Op::SetInt(value);
        let row = if !accepted {
            String::from("row: not recorded")
        } else if param_type == dionysus_core::fighter_common_copies::COMMON {
            fighter_common_copies::record(kind, slot, param_hash, op).message
        } else if param_type == dionysus_core::fighter_param_thrown::PARAM_THROWN {
            fighter_param_thrown::record(kind, slot, param_hash, op).message
        } else {
            fighter_param_rows::record(kind, slot, param_type, param_hash, op).message
        };
        let n = PARAM_REGISTER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < PARAM_REGISTER_LOG_LIMIT {
            dbg_log!(
                "[paramreg] #{n} int kind={kind} slot={slot} key=({param_type:#x},{param_hash:#x}) value={value} paramconfig={accepted} {row}"
            );
        }
        i32::from(!accepted)
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = (kind, slot, param_type, param_hash, value);
        -1
    }
}

#[cfg(feature = "css_slot")]
static PARAM_CONTEXT: thread_context::ThreadReentrancyFlag =
    thread_context::ThreadReentrancyFlag::new("param_context");

#[cfg(feature = "css_slot")]
static PARAM_BRACKETS_INSTALLED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "css_slot")]
static PARAM_KIND_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static PARAM_CONFIG_TEXT: [core::sync::atomic::AtomicUsize; 2] =
    [core::sync::atomic::AtomicUsize::new(0), core::sync::atomic::AtomicUsize::new(0)];

#[cfg(feature = "css_slot")]
fn record_param_config_text() {
    let mut address = 0usize;
    let found = unsafe {
        skyline::nn::ro::LookupSymbol(&mut address as *mut usize, b"FighterParamModule__has_kind\0".as_ptr())
    } == 0
        && address != 0;
    if !found {
        dbg_log_public(&String::from(
            "[parambridge] ParamConfig is not loaded; clone-kind ParamConfig values have no reader",
        ));
        return;
    }
    let (start, end) = unsafe { final_grafts::module_range(address) };
    if start == 0 {
        return;
    }
    PARAM_CONFIG_TEXT[0].store(start, core::sync::atomic::Ordering::Release);
    PARAM_CONFIG_TEXT[1].store(end, core::sync::atomic::Ordering::Release);
    dbg_log_public(&format!(
        "[parambridge] ParamConfig's code is {start:#x}..{end:#x}: get_kind answers a clone's kind to callers there, bracket or not"
    ));
}

#[cfg(feature = "css_slot")]
fn caller_is_param_config(lr: usize) -> bool {
    let start = PARAM_CONFIG_TEXT[0].load(core::sync::atomic::Ordering::Acquire);
    let end = PARAM_CONFIG_TEXT[1].load(core::sync::atomic::Ordering::Acquire);
    start != 0 && lr >= start && lr < end
}

#[cfg(feature = "css_slot")]
static PARAM_REGISTER_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
const PARAM_REGISTER_LOG_LIMIT: u32 = 160;

#[cfg(feature = "css_slot")]
unsafe fn param_getter_native(module: u64, member: usize, slot: usize) -> Option<(u64, u64)> {
    let object = if member == 0 {
        module
    } else {
        core::ptr::read_volatile((module as usize + member) as *const u64)
    };
    if object < 0x1000 {
        return None;
    }
    let vtable = core::ptr::read_volatile(object as usize as *const u64);
    if vtable < 0x1000 {
        return None;
    }
    let target = core::ptr::read_volatile((vtable as usize + slot) as *const u64);
    if target < 0x1000 {
        return None;
    }
    Some((object, target))
}

#[cfg(feature = "css_slot")]
unsafe fn install_param_getter_bracket(
    offset: usize,
    expected: &[u32; dionysus_core::hook_site::PROLOGUE_WORDS],
    handler: usize,
    tag: &str,
) {
    let site = game::addr(offset);
    let mut live = [0u32; dionysus_core::hook_site::PROLOGUE_WORDS];
    for (index, word) in live.iter_mut().enumerate() {
        *word = core::ptr::read_volatile((site + index * 4) as *const u32);
    }
    let mut expected = *expected;
    for (index, word) in expected.iter_mut().enumerate() {
        *word = game::opcode(offset + index * 4, *word);
    }
    if live != expected {
        let why = match dionysus_core::hook_site::relocation_hazard(&live) {
            Some(hazard) => hazard.reason(),
            None => "the thunk does not read as vanilla",
        };
        dbg_log_public(&format!(
            "[parambridge] {tag} at {offset:#x} left alone: {why} (live {live:#010x?}); a pack's ParamConfig calls on this getter keep the vanilla path"
        ));
        return;
    }
    let words = [
        0x5800_0051u32,
        0xd61f_0220,
        handler as u32,
        (handler >> 32) as u32,
    ];
    if text_patch::write_words(site, &words) {
        dbg_log_public(&format!(
            "[parambridge] {tag} at {offset:#x} bridged by a 16 byte trampoline to {handler:#x}, so the thunk after it keeps its first instruction"
        ));
        return;
    }
    dbg_log_public(&format!(
        "[parambridge] {tag} at {offset:#x} REFUSED by the page; this getter keeps the vanilla path"
    ));
}

#[cfg(feature = "css_slot")]
macro_rules! param_getter_brackets {
    ($($name:ident($offset:expr, $member:expr, $slot:expr, $words:expr) -> $ret:ty;)*) => {
        $(
            #[cfg(feature = "css_slot")]
            unsafe extern "C" fn $name(module: u64, param_type: u64, param_hash: u64) -> $ret {
                let _param_context = PARAM_CONTEXT.enter(current_thread_key());
                let Some((object, target)) = param_getter_native(module, $member, $slot) else {
                    return <$ret>::default();
                };
                let native: unsafe extern "C" fn(u64, u64, u64) -> $ret =
                    core::mem::transmute(target);
                native(object, param_type, param_hash)
            }
        )*

        #[cfg(feature = "css_slot")]
        fn install_param_getter_brackets() {
            $(
                unsafe {
                    install_param_getter_bracket(
                        $offset,
                        &$words,
                        $name as *const () as usize,
                        stringify!($name),
                    );
                }
            )*
        }
    };
}

#[cfg(feature = "css_slot")]
param_getter_brackets! {
    param_config_int_bracket(0x4e53a0, 0x38, 0x20,
        [0xf940_1c00, 0xf940_0008, 0xf940_1103, 0xd61f_0060]) -> i32;
    param_config_int64_bracket(0x4e53b0, 0, 0x260,
        [0xf940_0008, 0xf941_3103, 0xd61f_0060, 0x0000_0000]) -> i64;
    param_config_float_bracket(0x4e53e0, 0x38, 0x30,
        [0xf940_1c00, 0xf940_0008, 0xf940_1903, 0xd61f_0060]) -> f32;
}

#[cfg(feature = "css_slot")]
pub(crate) fn ensure_param_getter_brackets_installed() {
    if PARAM_BRACKETS_INSTALLED
        .compare_exchange(
            false,
            true,
            core::sync::atomic::Ordering::AcqRel,
            core::sync::atomic::Ordering::Acquire,
        )
        .is_ok()
    {
        install_param_getter_brackets();
        record_param_config_text();
        dbg_log_public(&String::from(
            "[parambridge] param getter brackets installed at the first fighter construction, outside every ParamConfig instance's hooks: a pack's own ParamConfig calls with its clone kind apply to the clone",
        ));
    }
}

#[cfg(feature = "css_slot")]
unsafe fn param_override_slot(module: u64) -> i32 {
    if module == 0 {
        return param_overrides::ANY_SLOT;
    }
    let boma = core::ptr::read_volatile((module + 8) as *const u64);
    if boma == 0 {
        return param_overrides::ANY_SLOT;
    }
    if smash::app::utility::get_category(
        &mut *(boma as *mut smash::app::BattleObjectModuleAccessor),
    ) != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        return param_overrides::ANY_SLOT;
    }
    smash::app::lua_bind::WorkModule::get_int(
        boma as *mut smash::app::BattleObjectModuleAccessor,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_COLOR,
    ) as i32
}

#[cfg(feature = "css_slot")]
unsafe fn param_override_kind(module: u64) -> Option<i32> {
    if module == 0 {
        return None;
    }
    let boma = core::ptr::read_volatile((module + 8) as *const u64);
    if boma == 0 {
        return None;
    }
    if smash::app::utility::get_category(
        &mut *(boma as *mut smash::app::BattleObjectModuleAccessor),
    ) != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        return None;
    }
    let entry_id = smash::app::lua_bind::WorkModule::get_int(
        boma as *mut smash::app::BattleObjectModuleAccessor,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    ) as i32;
    if !(0..8).contains(&entry_id) {
        return None;
    }
    entry_groups::clone_of_boma(boma as *mut smash::app::BattleObjectModuleAccessor)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x15cdff0))]
unsafe fn utility_get_kind_hook(boma: u64) -> i32 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let kind = call_original!(boma);

    if boma != 0
        && smash::app::utility::get_category(&mut *(boma as *mut smash::app::BattleObjectModuleAccessor))
            == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        let entry = smash::app::lua_bind::WorkModule::get_int(
            boma as *mut smash::app::BattleObjectModuleAccessor,
            *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
        ) as i32;
        record_entry_fighter_boma(entry, boma as usize);
    }

    let thread = current_thread_key();
    if !PARAM_CONTEXT.is_active(thread) && !caller_is_param_config(lr) {
        return kind;
    }
    if !caller_is_outside_main_text(lr) {
        return kind;
    }
    if boma == 0 || clone_definition(kind).is_some() {
        return kind;
    }
    if smash::app::utility::get_category(
        &mut *(boma as *mut smash::app::BattleObjectModuleAccessor),
    ) != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        return kind;
    }
    let entry_id = smash::app::lua_bind::WorkModule::get_int(
        boma as *mut smash::app::BattleObjectModuleAccessor,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    ) as i32;
    if !(0..8).contains(&entry_id) {
        return kind;
    }
    let Some(true_kind) = entry_custom_kind(entry_id as u8) else {
        return kind;
    };
    let object_id = core::ptr::read_volatile((boma as usize + BOMA_BATTLE_OBJECT_ID) as *const u32);
    let true_kind = entry_groups::same_base_member(entry_id, object_id).unwrap_or(true_kind);
    let Some(definition) = clone_definition(true_kind) else {
        return kind;
    };
    if definition.base_kind != kind && entry_groups::partner_of(true_kind) != Some(kind) {
        return entry_groups::member_of(true_kind, kind).unwrap_or(kind);
    }
    let n = PARAM_KIND_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 12 {
        let object_id =
            core::ptr::read_volatile((boma as usize + BOMA_BATTLE_OBJECT_ID) as *const u32);
        dbg_log!(
            "[paramkind] #{n} param-context get_kind {kind}->{true_kind} entry={entry_id} object={object_id:#x} caller={lr:#x}"
        );
    }
    true_kind
}

#[no_mangle]
pub extern "C" fn clone_engine_article_owner_kind_v1(module_accessor: u64) -> i32 {
    #[cfg(feature = "css_slot")]
    unsafe {
        if module_accessor == 0 {
            return -1;
        }
        let boma = module_accessor as *mut smash::app::BattleObjectModuleAccessor;
        let category = smash::app::utility::get_category(&mut *boma);
        if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER {
            return fighter_clone_kind(boma).unwrap_or(-1);
        }
        if category != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_WEAPON {
            return -1;
        }

        let object_id = core::ptr::read_volatile(
            (module_accessor as usize + BOMA_BATTLE_OBJECT_ID) as *const u32,
        );
        let weapon_kind = smash::app::utility::get_kind(&mut *boma);

        let linked = weapon_owner_clone_kind(
            boma,
            *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER,
            true,
        )
        .or_else(|| {
            weapon_owner_clone_kind(
                boma,
                *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_ACTIVATE_FOUNDER_ID,
                true,
            )
        });
        if let Some(kind) = linked {
            let first = !article_owners::is_tracked(object_id);
            article_owners::remember(object_id, weapon_kind, kind);
            if first {
                let n = ARTICLE_REMEMBER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                if n < 12 {
                    dbg_log!(
                        "[artowner] #{n} remember obj={object_id:#x} cat={} wkind={weapon_kind} clone={kind} tracked={}",
                        object_id >> 28,
                        article_owners::len()
                    );
                }
            }
            return kind;
        }

        let claimed = pocket_claim_kind(boma, weapon_kind);
        if article_owners::len() != 0 {
            let n = ARTICLE_RECALL_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 12 {
                dbg_log!(
                    "[artowner] #{n} lookup obj={object_id:#x} cat={} wkind={weapon_kind} links=none claim={claimed:?} tracked={}",
                    object_id >> 28,
                    article_owners::len()
                );
            }
        }
        claimed.unwrap_or(-1)
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = module_accessor;
        -1
    }
}

#[cfg(feature = "css_slot")]
const BOMA_BATTLE_OBJECT_ID: usize = 8;

#[cfg(feature = "css_slot")]
static ARTICLE_RECALL_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static ARTICLE_REMEMBER_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
fn is_pocketer_kind(kind: i32) -> bool {
    kind == *smash::lib::lua_const::FIGHTER_KIND_MURABITO
        || kind == *smash::lib::lua_const::FIGHTER_KIND_SHIZUE
}

const POCKET_SLOT_EMPTY: i32 = 0x5000_0000;

fn pocket_slot_is_empty(kind: i32) -> bool {
    kind == POCKET_SLOT_EMPTY || kind < 0
}

#[cfg(test)]
mod pocket_tests {
    use super::{is_pocketer_kind, pocket_slot_is_empty};

    #[test]
    fn an_empty_pocket_is_battle_object_id_invalid() {
        assert_eq!(super::POCKET_SLOT_EMPTY, 0x5000_0000);
        assert_eq!(
            super::POCKET_SLOT_EMPTY,
            *smash::lib::lua_const::BATTLE_OBJECT_ID_INVALID
        );
        assert!(pocket_slot_is_empty(super::POCKET_SLOT_EMPTY));
    }

    #[test]
    fn neither_zero_nor_a_real_kind_reads_as_empty() {
        assert!(!pocket_slot_is_empty(0));
        assert!(pocket_slot_is_empty(-1));
    }

    #[test]
    fn item_kind_assist_is_zero_and_must_not_read_as_empty() {
        assert_eq!(*smash::lib::lua_const::ITEM_KIND_ASSIST, 0);
        assert!(!pocket_slot_is_empty(
            *smash::lib::lua_const::ITEM_KIND_ASSIST
        ));
    }

    #[test]
    fn a_real_clone_base_kind_is_never_empty() {
        assert!(!pocket_slot_is_empty(0x3F));
        assert!(!pocket_slot_is_empty(0x45));
    }

    #[test]
    fn both_pocketers_are_recognised() {
        assert!(is_pocketer_kind(
            *smash::lib::lua_const::FIGHTER_KIND_MURABITO
        ));
        assert!(is_pocketer_kind(
            *smash::lib::lua_const::FIGHTER_KIND_SHIZUE
        ));
    }

    #[test]
    fn isabelle_inherits_villagers_status_and_work_space() {
        use smash::lib::lua_const::*;
        assert_eq!(
            *FIGHTER_SHIZUE_INSTANCE_WORK_ID_INT_START,
            *FIGHTER_MURABITO_INSTANCE_WORK_ID_INT_TERM
        );
        assert_eq!(
            *FIGHTER_SHIZUE_INSTANCE_WORK_ID_FLOAT_START,
            *FIGHTER_MURABITO_INSTANCE_WORK_ID_FLOAT_TERM
        );
        assert!(*FIGHTER_SHIZUE_STATUS_KIND_PREV >= *FIGHTER_MURABITO_STATUS_KIND_NUM - 1);
        assert!(
            *FIGHTER_MURABITO_INSTANCE_WORK_ID_INT_SPECIAL_N_OBJECT_KIND
                < *FIGHTER_SHIZUE_INSTANCE_WORK_ID_INT_START
        );
        assert!(
            super::MURABITO_STATUS_SPECIAL_N_TAKE_OUT < *FIGHTER_SHIZUE_STATUS_KIND_SPECIAL_S_START
        );
    }
}

#[cfg(feature = "css_slot")]
const POCKET_OBJECT_KIND: i32 = 0x1000_00C5;

#[cfg(feature = "css_slot")]
static POCKET_CLAIM_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
unsafe fn pocket_claim_kind(
    weapon: *mut smash::app::BattleObjectModuleAccessor,
    weapon_kind: i32,
) -> Option<i32> {
    let owner_id = smash::app::lua_bind::WorkModule::get_int(
        weapon,
        *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER,
    ) as u32;
    if owner_id == *smash::lib::lua_const::BATTLE_OBJECT_ID_INVALID as u32
        || !smash::app::sv_battle_object::is_active(owner_id)
    {
        return None;
    }
    let owner = smash::app::sv_battle_object::module_accessor(owner_id);
    if owner.is_null()
        || smash::app::utility::get_category(&mut *owner)
            != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        return None;
    }
    if !is_pocketer_kind(smash::app::utility::get_kind(&mut *owner)) {
        return None;
    }
    if smash::app::lua_bind::WorkModule::get_int(owner, POCKET_OBJECT_KIND) != weapon_kind {
        return None;
    }

    if let Some(kind) = witnessed_pocket_kind(owner) {
        return Some(kind);
    }

    let claimant = clone_claiming_weapon_kind(weapon_kind);
    if let Some(kind) = claimant {
        let n = POCKET_CLAIM_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            dbg_log!("[artowner] #{n} pocket claim wkind={weapon_kind} -> clone {kind}");
        }
    }
    claimant
}

#[cfg(feature = "css_slot")]
static POCKET_LATCH: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; 8];

#[cfg(feature = "css_slot")]
static POCKET_PREVIOUS: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(0) }; 8];

#[cfg(feature = "css_slot")]
static POCKET_WATCH_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static POCKET_WATCH_INSTALLED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "css_slot")]
unsafe extern "C" fn pocket_watch_tick(agent: &mut smash::lua2cpp::L2CFighterBase) {
    let agent = agent as *mut smash::lua2cpp::L2CFighterBase as u64;
    let object = core::ptr::read_volatile((agent + 0x38) as *const u64);
    if object == 0 {
        return;
    }
    if !is_pocketer_kind(core::ptr::read_volatile((object + 0xc) as *const i32)) {
        return;
    }
    let entry = core::ptr::read_volatile((object + 0x10) as *const i32);
    if !(0..8).contains(&entry) {
        return;
    }
    let boma = core::ptr::read_volatile((agent + 0x40) as *const u64)
        as *mut smash::app::BattleObjectModuleAccessor;
    if boma.is_null() {
        return;
    }

    article_owners::sweep(|object_id| smash::app::sv_battle_object::is_active(object_id));

    item_clones::sweep_live(|object_id| smash::app::sv_battle_object::is_active(object_id));
    POCKET_OWNER_ID[entry as usize].store(
        core::ptr::read_volatile((boma as usize + BOMA_BATTLE_OBJECT_ID) as *const u32),
        core::sync::atomic::Ordering::Relaxed,
    );
    let (have, pickable) = item_clones::pocket_contact_object_ids(boma as *mut u8);
    let contact = [
        have,
        pickable,
        POCKET_PREVIOUS_HAVE[entry as usize].swap(have, core::sync::atomic::Ordering::Relaxed),
        POCKET_PREVIOUS_PICKABLE[entry as usize]
            .swap(pickable, core::sync::atomic::Ordering::Relaxed),
    ];
    let held = smash::app::lua_bind::WorkModule::get_int(boma, POCKET_OBJECT_KIND);
    pocket_item_tick(boma, entry as usize, held, contact);
    let previous =
        POCKET_PREVIOUS[entry as usize].swap(held, core::sync::atomic::Ordering::Relaxed);
    if held == previous {
        return;
    }

    if pocket_slot_is_empty(held) {
        POCKET_LATCH[entry as usize].store(-1, core::sync::atomic::Ordering::Relaxed);
        return;
    }

    let owner = article_owners::recently_died(held).unwrap_or(-1);
    POCKET_LATCH[entry as usize].store(owner, core::sync::atomic::Ordering::Relaxed);
    let n = POCKET_WATCH_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 12 {
        dbg_log!("[artowner] #{n} pocket watch entry={entry} took wkind={held} -> clone {owner}");
    }
}

#[cfg(feature = "css_slot")]
const MURABITO_STATUS_SPECIAL_N_TAKE_OUT: i32 = 0x1E4;

#[cfg(feature = "css_slot")]
static POCKET_ITEM_LATCH: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; 8];

#[cfg(feature = "css_slot")]
static POCKET_ITEM_PREVIOUS: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(0) }; 8];

#[cfg(feature = "css_slot")]
static POCKET_ITEM_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const POCKET_TICKET_MAX_AGE: u32 = 30;

#[cfg(feature = "css_slot")]
static POCKET_TICKET_AGE: [core::sync::atomic::AtomicU32; 8] =
    [const { core::sync::atomic::AtomicU32::new(0) }; 8];

#[cfg(feature = "css_slot")]
static POCKET_OWNER_ID: [core::sync::atomic::AtomicU32; 8] = [const {
    core::sync::atomic::AtomicU32::new(article_probes::INVALID_BATTLE_OBJECT_ID)
}; 8];

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn pocket_release_claim(base_kind: i32) -> Option<i32> {
    for entry in 0..8usize {
        let latched = POCKET_ITEM_LATCH[entry].load(core::sync::atomic::Ordering::Relaxed);
        if latched < 0 || item_clones::clone_base_kind(latched) != Some(base_kind) {
            continue;
        }
        let owner_id = POCKET_OWNER_ID[entry].load(core::sync::atomic::Ordering::Relaxed);
        if owner_id == article_probes::INVALID_BATTLE_OBJECT_ID
            || !smash::app::sv_battle_object::is_active(owner_id)
        {
            continue;
        }
        let boma = smash::app::sv_battle_object::module_accessor(owner_id);
        if boma.is_null()
            || smash::app::lua_bind::StatusModule::status_kind(boma)
                != MURABITO_STATUS_SPECIAL_N_TAKE_OUT
        {
            continue;
        }
        POCKET_ITEM_LATCH[entry].store(-1, core::sync::atomic::Ordering::Relaxed);
        POCKET_TICKET_AGE[entry].store(0, core::sync::atomic::Ordering::Relaxed);
        return Some(latched);
    }
    None
}

#[cfg(not(feature = "css_slot"))]
pub(crate) unsafe fn pocket_release_claim(base_kind: i32) -> Option<i32> {
    let _ = base_kind;
    None
}

#[cfg(feature = "css_slot")]
static POCKET_PREVIOUS_HAVE: [core::sync::atomic::AtomicU32; 8] =
    [const { core::sync::atomic::AtomicU32::new(u32::MAX) }; 8];

#[cfg(feature = "css_slot")]
static POCKET_PREVIOUS_PICKABLE: [core::sync::atomic::AtomicU32; 8] =
    [const { core::sync::atomic::AtomicU32::new(u32::MAX) }; 8];

#[cfg(feature = "css_slot")]
const POCKET_CANDIDATES: usize = 8;

#[cfg(feature = "css_slot")]
static POCKET_CANDIDATE_OBJECT: [[core::sync::atomic::AtomicU32; POCKET_CANDIDATES]; 8] = [const {
    [const { core::sync::atomic::AtomicU32::new(u32::MAX) }; POCKET_CANDIDATES]
}; 8];

#[cfg(feature = "css_slot")]
static POCKET_CANDIDATE_KIND: [[core::sync::atomic::AtomicI32; POCKET_CANDIDATES]; 8] =
    [const { [const { core::sync::atomic::AtomicI32::new(-1) }; POCKET_CANDIDATES] }; 8];

#[cfg(feature = "css_slot")]
static POCKET_CANDIDATE_COUNT: [core::sync::atomic::AtomicUsize; 8] =
    [const { core::sync::atomic::AtomicUsize::new(0) }; 8];

#[cfg(feature = "css_slot")]
fn arm_pocket_candidates(entry: usize, base_kind: i32) -> usize {
    let mut census = [(u32::MAX, -1i32); POCKET_CANDIDATES];
    let count = item_clones::live_clones_with_base(base_kind, &mut census);
    for (index, slot) in census.iter().enumerate() {
        let (object_id, public_kind) = if index < count {
            *slot
        } else {
            (u32::MAX, -1)
        };
        POCKET_CANDIDATE_OBJECT[entry][index].store(object_id, core::sync::atomic::Ordering::Relaxed);
        POCKET_CANDIDATE_KIND[entry][index].store(public_kind, core::sync::atomic::Ordering::Relaxed);
    }
    POCKET_CANDIDATE_COUNT[entry].store(count, core::sync::atomic::Ordering::Release);
    count
}

#[cfg(feature = "css_slot")]
fn clear_pocket_candidates(entry: usize) {
    POCKET_CANDIDATE_COUNT[entry].store(0, core::sync::atomic::Ordering::Release);
}

#[cfg(feature = "css_slot")]
unsafe fn poll_pocket_candidates(entry: usize) -> Option<i32> {
    let count = POCKET_CANDIDATE_COUNT[entry].load(core::sync::atomic::Ordering::Acquire);
    for index in 0..count {
        let object_id =
            POCKET_CANDIDATE_OBJECT[entry][index].load(core::sync::atomic::Ordering::Relaxed);
        if object_id == u32::MAX || smash::app::sv_battle_object::is_active(object_id) {
            continue;
        }
        POCKET_CANDIDATE_OBJECT[entry][index].store(u32::MAX, core::sync::atomic::Ordering::Relaxed);
        let public_kind =
            POCKET_CANDIDATE_KIND[entry][index].load(core::sync::atomic::Ordering::Relaxed);
        return (public_kind >= 0).then_some(public_kind);
    }
    None
}

#[cfg(feature = "css_slot")]
unsafe fn pocket_item_tick(
    boma: *mut smash::app::BattleObjectModuleAccessor,
    entry: usize,
    held: i32,
    contact: [u32; 4],
) {
    let previous = POCKET_ITEM_PREVIOUS[entry].swap(held, core::sync::atomic::Ordering::Relaxed);
    let latched = POCKET_ITEM_LATCH[entry].load(core::sync::atomic::Ordering::Relaxed);

    if pocket_slot_is_empty(held) {
        clear_pocket_candidates(entry);
        if latched < 0 {
            return;
        }
        if !pocket_slot_is_empty(previous) {
            POCKET_ITEM_LATCH[entry].store(-1, core::sync::atomic::Ordering::Relaxed);
            POCKET_TICKET_AGE[entry].store(0, core::sync::atomic::Ordering::Relaxed);
            if !item_clones::pocket_ticket_pending(latched) {
                item_clones::queue_pocket_ticket(latched);
            }
            return;
        }
        let age = POCKET_TICKET_AGE[entry].fetch_add(1, core::sync::atomic::Ordering::Relaxed) + 1;
        if age >= POCKET_TICKET_MAX_AGE {
            POCKET_ITEM_LATCH[entry].store(-1, core::sync::atomic::Ordering::Relaxed);
            item_clones::drop_pocket_ticket(latched);
        }
        return;
    }

    if held != previous {
        POCKET_TICKET_AGE[entry].store(0, core::sync::atomic::Ordering::Relaxed);
        let contacted = contact
            .iter()
            .find_map(|&object_id| item_clones::clone_kind_of_object(object_id, held));
        let vanished = if contacted.is_some() {
            None
        } else {
            item_clones::claim_retired_clone(held)
        };
        let clone = contacted.or(vanished);
        POCKET_ITEM_LATCH[entry].store(clone.unwrap_or(-1), core::sync::atomic::Ordering::Relaxed);
        match clone {
            Some(public_kind) => {
                clear_pocket_candidates(entry);
                if POCKET_ITEM_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 16 {
                    dbg_log!("[pocketclone] pocketed clone {public_kind:#x} (base {held:#x})");
                }
            }
            None if arm_pocket_candidates(entry, held) > 0 => {
                if POCKET_ITEM_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 16 {
                    dbg_log!(
                        "[pocketclone] a clone item on base {held:#x} is live but the pocketed object could not be identified; it will come back as the base item"
                    );
                }
            }
            None => {}
        }
        return;
    }

    if latched < 0 {
        let Some(public_kind) = poll_pocket_candidates(entry) else {
            return;
        };
        POCKET_ITEM_LATCH[entry].store(public_kind, core::sync::atomic::Ordering::Relaxed);
        clear_pocket_candidates(entry);
        if POCKET_ITEM_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 16 {
            dbg_log!("[pocketclone] pocketed clone {public_kind:#x} (base {held:#x}), identified late");
        }
        return;
    }

    let status = smash::app::lua_bind::StatusModule::status_kind(boma);
    let releasing = status == MURABITO_STATUS_SPECIAL_N_TAKE_OUT;
    if !releasing {
        item_clones::drop_pocket_ticket(latched);
        return;
    }
    if item_clones::pocket_ticket_pending(latched) {
        return;
    }
    item_clones::queue_pocket_ticket(latched);
}

#[cfg(feature = "css_slot")]
unsafe fn witnessed_pocket_kind(boma: *mut smash::app::BattleObjectModuleAccessor) -> Option<i32> {
    let entry = smash::app::lua_bind::WorkModule::get_int(
        boma,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    ) as i32;
    if !(0..8).contains(&entry) {
        return None;
    }
    match POCKET_LATCH[entry as usize].load(core::sync::atomic::Ordering::Relaxed) {
        -1 => None,
        kind => Some(kind),
    }
}

#[cfg(feature = "css_slot")]
unsafe fn install_pocket_watch() {
    let Some(address) = lookup_symbol(b"smashline_install_line_callback\0") else {
        skyline::println!(
            "[artowner] smashline_install_line_callback is not resolvable; pocketed clone articles will fall back to registration data"
        );
        return;
    };
    type InstallLineCallback = unsafe extern "C" fn(u64, i32, *const ());
    let install: InstallLineCallback = core::mem::transmute(address);
    install(
        hash40("fighter"),
        STATUS_LINE_MAIN,
        pocket_watch_tick as *const (),
    );
    skyline::println!("[artowner] pocket watch installed on the wildcard fighter agent");
}

#[cfg(feature = "css_slot")]
const STATUS_LINE_MAIN: i32 = 1;

#[cfg(feature = "css_slot")]
fn clone_claiming_weapon_kind(weapon_kind: i32) -> Option<i32> {
    if let Some(kind) = article_owners::sole_owner_of_kind(weapon_kind) {
        return Some(kind);
    }
    let mut claimant = None;
    for entry in 0..8u8 {
        let Some(clone) = entry_custom_kind(entry) else {
            continue;
        };
        let Some(definition) = clone_definition(clone) else {
            continue;
        };
        if !definition
            .articles
            .iter()
            .any(|article| article.base_weapon_kind == weapon_kind)
        {
            continue;
        }
        match claimant {
            None => claimant = Some(clone),
            Some(existing) if existing == clone => {}
            Some(_) => return None,
        }
    }
    claimant
}

#[cfg(feature = "css_slot")]
static POCKET_HOLDER_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[no_mangle]
pub extern "C" fn clone_engine_pocket_holder_kind_v1(module_accessor: u64) -> i32 {
    #[cfg(feature = "css_slot")]
    unsafe {
        if module_accessor == 0 {
            return -1;
        }
        let boma = module_accessor as *mut smash::app::BattleObjectModuleAccessor;
        if smash::app::utility::get_category(&mut *boma)
            != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
        {
            return -1;
        }
        if !is_pocketer_kind(smash::app::utility::get_kind(&mut *boma)) {
            return -1;
        }
        if let Some(kind) = witnessed_pocket_kind(boma) {
            return kind;
        }
        let held = smash::app::lua_bind::WorkModule::get_int(boma, POCKET_OBJECT_KIND);
        if held <= 0 {
            return -1;
        }
        let Some(kind) = clone_claiming_weapon_kind(held) else {
            return -1;
        };
        let n = POCKET_HOLDER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            dbg_log!("[artowner] #{n} pocket holder holds wkind={held} -> clone {kind}");
        }
        kind
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = module_accessor;
        -1
    }
}

#[cfg(feature = "css_slot")]
unsafe fn fighter_clone_kind(boma: *mut smash::app::BattleObjectModuleAccessor) -> Option<i32> {
    entry_groups::clone_of_boma(boma)
}

#[cfg(feature = "css_slot")]
unsafe fn weapon_owner_clone_kind(
    boma: *mut smash::app::BattleObjectModuleAccessor,
    owner_work_id: i32,
    allow_kirby_copy: bool,
) -> Option<i32> {
    if active_vanilla_construction_kind().is_some() {
        return None;
    }
    let owner_id = smash::app::lua_bind::WorkModule::get_int(boma, owner_work_id) as u32;
    if owner_id == *smash::lib::lua_const::BATTLE_OBJECT_ID_INVALID as u32
        || !smash::app::sv_battle_object::is_active(owner_id)
    {
        return None;
    }
    let owner = smash::app::sv_battle_object::module_accessor(owner_id);
    if owner.is_null() {
        return None;
    }
    if smash::app::utility::get_category(&mut *owner)
        != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER
    {
        return None;
    }
    if css_registration::owner_object_table_is_populated() {
        if let Some(entry) = css_registration::entry_of_owner_object(owner_id) {
            return entry_groups::clone_for_object(entry, smash::app::utility::get_kind(&mut *owner), owner_id);
        }
    }
    if let Some(kind) = fighter_clone_kind(owner) {
        return Some(kind);
    }
    if !allow_kirby_copy {
        return None;
    }
    kirby_copied_clone_kind(owner as u64)
}

#[cfg(feature = "css_slot")]
static OWNER_RESOLVE_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static GENERATOR_OWNER_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static ARTICLE_GENERATOR_OWNER: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("article_generator_owner");

#[cfg(feature = "css_slot")]
pub(crate) const GENERATOR_NOT_A_CLONE: i32 = i32::MAX;

#[cfg(feature = "css_slot")]
pub(crate) static ENTRY_FIGHTER_BOMA: [core::sync::atomic::AtomicUsize; 8] =
    [const { core::sync::atomic::AtomicUsize::new(0) }; 8];

#[cfg(feature = "css_slot")]
unsafe fn record_entry_fighter_boma(entry_id: i32, boma: usize) {
    if !(0..8).contains(&entry_id) || boma == 0 {
        return;
    }
    ENTRY_FIGHTER_BOMA[entry_id as usize].store(boma, core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "css_slot")]
unsafe fn entry_of_fighter_boma(candidate: usize) -> Option<usize> {
    if candidate == 0 {
        return None;
    }
    ENTRY_FIGHTER_BOMA
        .iter()
        .position(|slot| slot.load(core::sync::atomic::Ordering::SeqCst) == candidate)
}


#[cfg(feature = "css_slot")]
pub(crate) unsafe fn entry_of_fighter_boma_public(candidate: usize) -> Option<usize> {
    entry_of_fighter_boma(candidate)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn scan_stack_for_owner(sp: usize, span: usize) -> String {
    if sp == 0 {
        return "nosp".to_string();
    }
    let mut hits: Vec<String> = Vec::new();
    let mut offset = 0usize;
    while offset < span {
        let value = core::ptr::read_volatile((sp + offset) as *const usize);
        if let Some(entry) = entry_of_fighter_boma(value) {
            hits.push(format!("+{offset:#x}=>entry{entry}"));
            if hits.len() >= 8 {
                break;
            }
        }
        offset += 8;
    }
    if hits.is_empty() {
        "none".to_string()
    } else {
        hits.join(",")
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn scan_for_owner_pointer_public(
    label: &str,
    base: usize,
    span: usize,
) -> String {
    scan_for_owner_pointer(label, base, span)
}

#[cfg(feature = "css_slot")]
unsafe fn scan_for_owner_pointer(label: &str, base: usize, span: usize) -> String {
    if base == 0 {
        return format!("{label}=null");
    }
    let mut hits: Vec<String> = Vec::new();
    let mut offset = 0usize;
    while offset < span {
        let value = core::ptr::read_volatile((base + offset) as *const usize);
        if let Some(entry) = entry_of_fighter_boma(value) {
            hits.push(format!("+{offset:#x}=>entry{entry}"));
            if hits.len() >= 8 {
                break;
            }
        }
        offset += 8;
    }
    if hits.is_empty() {
        format!("{label}: none")
    } else {
        format!("{label}: {}", hits.join(","))
    }
}

#[cfg(feature = "css_slot")]
unsafe fn describe_owner(boma: *mut smash::app::BattleObjectModuleAccessor, work_id: i32) -> String {
    let owner_id = smash::app::lua_bind::WorkModule::get_int(boma, work_id) as u32;
    if owner_id == *smash::lib::lua_const::BATTLE_OBJECT_ID_INVALID as u32 {
        return "invalid".to_string();
    }
    if !smash::app::sv_battle_object::is_active(owner_id) {
        return format!("id={owner_id:#x} inactive");
    }
    let owner = smash::app::sv_battle_object::module_accessor(owner_id);
    if owner.is_null() {
        return format!("id={owner_id:#x} null");
    }
    let category = smash::app::utility::get_category(&mut *owner);
    if category != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER {
        return format!("id={owner_id:#x} category={category}");
    }
    let native = smash::app::utility::get_kind(&mut *owner);
    let entry = smash::app::lua_bind::WorkModule::get_int(
        owner,
        *smash::lib::lua_const::FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID,
    ) as i32;
    format!(
        "id={owner_id:#x} boma={:#x} native={native} entry={entry} maps_to={:?}",
        owner as usize,
        (0..8).contains(&entry).then(|| entry_custom_kind(entry as u8)).flatten()
    )
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn article_owner_kind_by_entry(module_accessor: u64) -> Option<i32> {
    if module_accessor == 0 {
        return None;
    }
    let boma = module_accessor as *mut smash::app::BattleObjectModuleAccessor;
    let category = smash::app::utility::get_category(&mut *boma);
    if category == *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_FIGHTER {
        return fighter_clone_kind(boma);
    }
    if category != *smash::lib::lua_const::BATTLE_OBJECT_CATEGORY_WEAPON {
        return None;
    }
    if let Some(generator) = ARTICLE_GENERATOR_OWNER.active(current_thread_key()) {
        let resolved = clone_definition(generator).map(|_| generator);
        let n = GENERATOR_OWNER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 {
            dbg_log!(
                "[ownerresolve] generated article boma={module_accessor:#x} takes its generator's owner {resolved:?} (its own link is not set yet)"
            );
        }
        return resolved;
    }
    let resolved = weapon_owner_clone_kind(
        boma,
        *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER,
        false,
    )
    .or_else(|| {
        weapon_owner_clone_kind(
            boma,
            *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_ACTIVATE_FOUNDER_ID,
            false,
        )
    });

    let article_kind = smash::app::utility::get_kind(&mut *boma);
    if custom_articles::custom_weapon_source_kind(article_kind).is_some()
        || custom_articles::source_weapon_owner_kind(article_kind).is_some()
        || resolved.is_some()
    {
        let n = OWNER_RESOLVE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 40 {
            dbg_log!(
                "[ownerresolve] #{n} article={article_kind} boma={module_accessor:#x} link[{}] founder[{}] -> {resolved:?}",
                describe_owner(boma, *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_LINK_OWNER),
                describe_owner(
                    boma,
                    *smash::lib::lua_const::WEAPON_INSTANCE_WORK_ID_INT_ACTIVATE_FOUNDER_ID
                )
            );
            dbg_log!(
                "[ownerscan] #{n} known{:?} {} {}",
                ENTRY_FIGHTER_BOMA
                    .iter()
                    .map(|slot| slot.load(core::sync::atomic::Ordering::SeqCst))
                    .take(4)
                    .collect::<Vec<_>>(),
                scan_for_owner_pointer("boma", module_accessor as usize, 0x200),
                scan_for_owner_pointer(
                    "obj",
                    module_accessor.wrapping_sub(0x150) as usize,
                    0x300
                )
            );
        }
    }
    resolved
}

#[cfg(feature = "css_slot")]
unsafe fn kirby_copied_clone_kind(owner_boma: u64) -> Option<i32> {
    let (flag, _copy_kind, _target_entry, target_kind) =
        crate::kirby_copy::kirby_copy_work_snapshot(owner_boma)?;
    if flag == 0 {
        return None;
    }
    let kind = target_kind?;
    clone_definition(kind).map(|_| kind)
}

#[cfg(feature = "diag_article")]
static RESOLVER_LOG_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "diag_article")]
static GETTER_LOG_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[skyline::hook(offset = crate::game::rel(OFF_FIGHTER_CLASS_RESOLVER))]
unsafe fn fighter_class_resolver_hook(kind: i32) -> u64 {
    #[cfg(feature = "diag_article")]
    {
        let n = RESOLVER_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 {
            dbg_log!("[resolver] #{n} kind={kind}");
        }
    }
    let resolved = match clone_base(kind) {
        Some(base) => {
            diag_reroute_log!(fighter_class_resolver_hook, kind, base);
            if let Some(class) = clone_vtables::fighter_class_for(kind, base) {
                return smashline_vtables::class_for(kind, class) as u64;
            }
            base
        }
        None => {
            #[cfg(feature = "css_slot")]
            if !CONSTRUCTION_CONTEXT.is_idle() {
                if let Some(clone) = active_construction_kind() {
                    if clone_base(clone) == Some(kind) {
                        if let Some(class) = clone_vtables::fighter_class_for(clone, kind) {
                            return smashline_vtables::class_for(clone, class) as u64;
                        }
                    }
                }
            }
            kind
        }
    };
    if resolved != kind && (0..94).contains(&resolved) {
        let slot = crate::game::addr(FIGHTER_CLASS_TABLE) + resolved as usize * 8;
        return *(slot as *const u64);
    }
    call_original!(kind)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StaticFighterData {
    id: i32,
    static_article_info: *const u8,
    rest: [u64; 9],
}

const NATIVE_FIGHTER_KIND_MAX: i32 = 0x74;

static GETTER_ENTRY_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

static NATIVE_KIND_CLAMP_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[skyline::hook(offset = crate::game::rel(OFF_STATIC_FIGHTER_DATA))]
unsafe fn static_fighter_data_hook(kind: i32) -> *const StaticFighterData {
    {
        let n = GETTER_ENTRY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 32 {
            dbg_log_public(&format!("[getter] #{n} enter kind={kind}"));
        }
    }
    #[cfg(feature = "diag_article")]
    let seq = GETTER_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    #[cfg(feature = "diag_article")]
    if seq < 24 {
        dbg_log!("[getter] #{seq} ENTER kind={kind}");
    }
    if let Some(base) = clone_base(kind) {
        diag_reroute_log!(static_fighter_data_hook, kind, base);
        #[cfg(feature = "diag_article_baseblob")]
        {
            let blob: *const StaticFighterData = call_original!(base);
            #[cfg(feature = "diag_article")]
            if seq < 24 {
                dbg_log!(
                    "[getter] #{seq} REMAP kind={kind} -> base {base}, blob={:#x} art={:#x}",
                    blob as usize,
                    (*blob).static_article_info as usize
                );
            }
            return blob;
        }
        #[cfg(not(feature = "diag_article_baseblob"))]
        {
            let template = if kind > NATIVE_FIGHTER_KIND_MAX { base } else { kind };
            if template != kind {
                let n = NATIVE_KIND_CLAMP_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                if n < 8 {
                    dbg_log_public(&format!(
                        "[getter] kind {kind} is past the native getter's own bound {NATIVE_FIGHTER_KIND_MAX}; taking base {base}'s blob as the template instead of asking 0x64b730 for a kind it rejects"
                    ));
                }
            }
            if template > NATIVE_FIGHTER_KIND_MAX {
                return core::ptr::null();
            }
            let own: *const StaticFighterData = call_original!(template);
            if own.is_null() {
                return own;
            }
            #[cfg_attr(feature = "diag_article_nopatch", allow(unused_mut))]
            let mut patched = *own;
            #[cfg_attr(feature = "diag_article_nopatch", allow(unused_mut))]
            let mut base_blob = 0usize;
            #[cfg(not(feature = "diag_article_nopatch"))]
            {
                let base_data: *const StaticFighterData = call_original!(base);
                if base_data.is_null() {
                    return own;
                }
                let base_data = mask_shared_articles(base, base_data, false);
                patched.static_article_info = (*base_data).static_article_info;
                base_blob = base_data as usize;
            }
            let Ok(mut blobs) = remapped_fighter_blobs().lock() else {
                return Box::into_raw(Box::new(patched));
            };
            if let Some((_, _, _, cached)) = blobs
                .iter()
                .find(|(cached_kind, own_blob, cached_base, _)| {
                    *cached_kind == kind && *own_blob == own as usize && *cached_base == base_blob
                })
            {
                return *cached as *const StaticFighterData;
            }
            let created = Box::into_raw(Box::new(patched));
            blobs.push((kind, own as usize, base_blob, created as usize));
            return created;
        }
    }
    if kind < 0 || kind > NATIVE_FIGHTER_KIND_MAX {
        let n = NATIVE_KIND_CLAMP_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            dbg_log_public(&format!(
                "[getter] kind {kind} has no clone base and is outside 0..={NATIVE_FIGHTER_KIND_MAX}; the native getter would assert on it, so returning no blob"
            ));
        }
        return core::ptr::null();
    }
    let blob: *const StaticFighterData = call_original!(kind);
    #[cfg(feature = "diag_article")]
    if seq < 24 {
        dbg_log!(
            "[getter] #{seq} PASS kind={kind}, blob={:#x} art={:#x}",
            blob as usize,
            if blob.is_null() {
                0
            } else {
                (*blob).static_article_info as usize
            }
        );
    }
    #[cfg(feature = "css_slot")]
    if active_vanilla_construction_kind() == Some(kind) {
        let n = VANILLA_ARTICLE_SKIP_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 16 {
            dbg_log_public(&format!(
                "[article] fighter kind {kind} is the real base fighter under construction; leaving its own article table alone"
            ));
        }
        return mask_shared_articles(kind, blob, true);
    }
    append_custom_articles(kind, blob, &mut |source_kind| {
        if source_kind < 0 || source_kind > NATIVE_FIGHTER_KIND_MAX {
            return core::ptr::null();
        }
        call_original!(source_kind)
    })
}

#[cfg(feature = "css_slot")]
static VANILLA_ARTICLE_SKIP_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static KIRBY_COPY_NAME_RECORDS: OnceLock<RwLock<Vec<(i32, usize)>>> = OnceLock::new();

#[cfg(feature = "css_slot")]
fn custom_kirby_copy_name_record(kind: i32) -> Option<usize> {
    let definition = clone_definition(kind)?;
    let records = KIRBY_COPY_NAME_RECORDS.get_or_init(|| RwLock::new(Vec::new()));
    let mut records = records.write().unwrap();
    if let Some((_, record)) = records.iter().find(|(candidate, _)| *candidate == kind) {
        return Some(*record);
    }

    let mut name = format!("copy_{}_fitkirby", definition.resource_name).into_bytes();
    name.push(0);
    let name = Box::leak(name.into_boxed_slice()).as_ptr() as usize;
    let empty = b"\0".as_ptr() as usize;
    let mut slots = [name, empty, empty, empty];
    for (index, extra) in kirby_copy::copy_models(kind)
        .iter()
        .take(dionysus_api::MAX_COPY_MODELS)
        .enumerate()
    {
        let mut bytes = extra.clone().into_bytes();
        bytes.push(0);
        slots[index + 1] = Box::leak(bytes.into_boxed_slice()).as_ptr() as usize;
        dbg_log!("[kirbynative] kind={kind} copy model slot {} = '{extra}'", index + 1);
    }
    let record = Box::leak(Box::new(slots)).as_ptr() as usize;
    records.push((kind, record));
    Some(record)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_KIRBY_COPY_RECORD_NAME), inline)]
unsafe fn kirby_copy_record_name(ctx: &mut skyline::hooks::InlineCtx) {
    let Some(kind) = active_clone_slot_kind() else {
        return;
    };
    let Some(record) = custom_kirby_copy_name_record(kind) else {
        return;
    };
    let before = ctx.registers[20].x();
    ctx.registers[20].set_x(record as u64);
    let n = KIRBY_NATIVE_RECORD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 48 {
        dbg_log!("[kirbynative] #{n} names kind={kind} record={before:#x}->{record:#x}");
    }
}

#[cfg(feature = "css_slot")]
unsafe fn kirby_copy_record_base_name(ctx: &mut skyline::hooks::InlineCtx, site: &str) {
    let Some(kind) = active_clone_slot_kind() else {
        return;
    };
    let Some(definition) = clone_definition(kind) else {
        return;
    };
    let before = ctx.registers[2].x();
    let name = definition.base_resource_name_cstr.as_ptr() as u64;
    ctx.registers[2].set_x(name);
    let n = KIRBY_NATIVE_RECORD_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 48 {
        dbg_log!(
            "[kirbynative] #{n} {site} kind={kind} base={} name={before:#x}->{name:#x}",
            definition.base_resource_name
        );
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_KIRBY_COPY_RECORD_BODY_NAME), inline)]
unsafe fn kirby_copy_record_body_name(ctx: &mut skyline::hooks::InlineCtx) {
    kirby_copy_record_base_name(ctx, "bodymotion");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_KIRBY_COPY_RECORD_SOUND_NAME), inline)]
unsafe fn kirby_copy_record_sound_name(ctx: &mut skyline::hooks::InlineCtx) {
    kirby_copy_record_base_name(ctx, "sound");
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_KIRBY_COPY_RECORD_CREATOR))]
unsafe fn kirby_copy_record_creator_probe(sub: u64, args: *const i32, x2: u64) -> u64 {
    let lr: usize;
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("mov {}, x30", out(reg) lr);
    #[cfg(not(target_arch = "aarch64"))]
    {
        lr = 0;
    }
    let n = KIRBY_RECORD_CREATOR_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let (kind, color) = if args.is_null() {
        (-1, -1)
    } else {
        (
            core::ptr::read_unaligned(args),
            core::ptr::read_unaligned(args.add(1)),
        )
    };
    if n < 32 {
        dbg_log!(
            "[kirbycreator] #{n} ENTER kind={kind} color={color} sub={sub:#x} caller=@{:#x}",
            game::offset_of(lr)
        );
    }
    if let Some(clone_kind) = active_clone_slot_kind().filter(|_| !args.is_null()) {
        let _ = custom_kirby_copy_name_record(clone_kind);
        let variant_key = core::ptr::read_unaligned(args.add(2));
        let clone_args = [clone_kind, color, variant_key];
        let result = call_original!(sub, clone_args.as_ptr(), x2);
        let record = kirby_record_find(sub, clone_kind);
        let pair_mask = kirby_record_model_pair_mask(record);
        let color_record = if record != 0 && (0..8).contains(&color) {
            record as usize + color as usize * KIRBY_RECORD_COLOR_STRIDE
        } else {
            0
        };
        let (model, motion) = if color_record != 0 {
            (
                *((color_record + 0x20) as *const u64),
                *((color_record + 0xc0) as *const u64),
            )
        } else {
            (0, 0)
        };
        if n < 32 {
            dbg_log!(
                "[kirbycreator] #{n} NATIVE kind={kind}(base)->{clone_kind} color={color} variant={variant_key:#x} record={record:#x} model={model:#x} motion={motion:#x} colors={pair_mask:#x} ret={result:#x}"
            );
        }
        return result;
    }

    let result = call_original!(sub, args, x2);
    if n < 32 {
        dbg_log!("[kirbycreator] #{n} EXIT kind={kind} color={color} ret={result:#x}");
    }
    result
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_KIRBY_COPY_MEMBER_BUILDER))]
unsafe fn kirby_copy_member_builder_probe(
    sub: u64,
    kind: u32,
    color: u32,
    variant: i32,
    member_base: u64,
    name: *const u8,
    resource_type: u32,
    flags: u32,
) {
    let n = KIRBY_MEMBER_BUILDER_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 32 {
        let text = if name.is_null() {
            String::from("<null>")
        } else {
            core::ffi::CStr::from_ptr(name as *const core::ffi::c_char)
                .to_string_lossy()
                .into_owned()
        };
        dbg_log!(
            "[kirbymember] #{n} ENTER kind={kind} color={color} variant={variant} name='{text}' type={resource_type} flags={flags:#x} member_base={member_base:#x} clone_slot={:?}",
            active_clone_slot_kind()
        );
    }
    call_original!(
        sub,
        kind,
        color,
        variant,
        member_base,
        name,
        resource_type,
        flags
    );
    if n < 32 {
        dbg_log!("[kirbymember] #{n} EXIT kind={kind} color={color}");
    }
}

fn article_owner_loaded(weapon_kind: i32) -> bool {
    let Some(owner) = custom_articles::resource_owner_of(weapon_kind) else {
        return true;
    };
    let owner = core::str::from_utf8(owner.split(|byte| *byte == 0).next().unwrap_or(owner));
    let Some(definition) = owner.ok().and_then(clone_definition_from_name) else {
        return true;
    };
    clone_kind_in_match(definition.kind)
}

fn clone_kind_in_match(kind: i32) -> bool {
    #[cfg(feature = "css_slot")]
    {
        CSS_CUSTOM_ENTRY_KINDS
            .iter()
            .any(|entry| entry.load(core::sync::atomic::Ordering::SeqCst) == kind)
    }
    #[cfg(not(feature = "css_slot"))]
    {
        let _ = kind;
        true
    }
}

unsafe fn append_custom_articles(
    kind: i32,
    blob: *const StaticFighterData,
    original: &mut dyn FnMut(i32) -> *const StaticFighterData,
) -> *const StaticFighterData {
    if blob.is_null() {
        return blob;
    }
    let blob = mask_shared_articles(kind, blob, false);

    let copy_source_owners = custom_articles::prime_kirby_copy_headers(|source_kind| {
        let source = original(source_kind);
        if source.is_null() || (*source).static_article_info.is_null() {
            return None;
        }
        Some(*((*source).static_article_info as *const custom_articles::StaticArticleData))
    });
    for owner in copy_source_owners {
        fighter_modules::request(owner);
    }

    let mut appended = custom_articles::descriptors_for(kind, |source_kind| {
        let source = original(source_kind);
        if source.is_null() || (*source).static_article_info.is_null() {
            return None;
        }
        Some(*((*source).static_article_info as *const custom_articles::StaticArticleData))
    });
    appended.retain(|descriptor| article_owner_loaded(descriptor.weapon_id));
    if appended.is_empty() {
        return blob;
    }

    let appended_kinds: Vec<i32> = appended.iter().map(|descriptor| descriptor.weapon_id).collect();
    if let Some(cached) = patched_article_blobs().lock().ok().and_then(|blobs| {
        blobs
            .iter()
            .find(|(cached_kind, cached_blob, kinds, _)| {
                *cached_kind == kind && *cached_blob == blob as usize && *kinds == appended_kinds
            })
            .map(|(_, _, _, patched)| *patched)
    }) {
        return cached as *const StaticFighterData;
    }

    for descriptor in appended.iter() {
        let Some(owner) = custom_articles::source_weapon_owner_kind(descriptor.weapon_id) else {
            continue;
        };
        fighter_modules::request(owner);
    }

    let mut descriptors: Vec<custom_articles::ArticleDescriptor> = Vec::new();
    let existing = (*blob).static_article_info as *const custom_articles::StaticArticleData;
    if !existing.is_null() && !(*existing).descriptors.is_null() {
        descriptors.extend_from_slice(core::slice::from_raw_parts(
            (*existing).descriptors,
            (*existing).count,
        ));
    }
    descriptors.extend_from_slice(&appended);

    let count = descriptors.len();
    let article_info = Box::leak(Box::new(custom_articles::StaticArticleData {
        descriptors: Vec::leak(descriptors).as_ptr(),
        count,
    }));
    let mut patched = *blob;
    patched.static_article_info = article_info as *const _ as *const u8;
    dbg_log!(
        "[article] fighter kind {kind} article table {} -> {count} entries",
        count - appended.len()
    );
    {
        static TABLE_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        if TABLE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 6 {
            let rows: Vec<String> = core::slice::from_raw_parts(
                (*article_info).descriptors,
                (*article_info).count,
            )
            .iter()
            .map(|d| {
                format!(
                    "{{weapon {} max {} init {:#x} fini {:#x}}}",
                    d.weapon_id,
                    d.max_count,
                    game::offset_of(d.on_init_callback as usize),
                    game::offset_of(d.on_fini_callback as usize)
                )
            })
            .collect();
            dbg_log!("[articletable] fighter kind {kind}: {}", rows.join(" "));
        }
    }
    let patched = Box::leak(Box::new(patched)) as *const StaticFighterData;
    if let Ok(mut blobs) = patched_article_blobs().lock() {
        blobs.push((kind, blob as usize, appended_kinds, patched as usize));
    }
    patched
}

unsafe fn mask_shared_articles(
    kind: i32,
    blob: *const StaticFighterData,
    vanilla: bool,
) -> *const StaticFighterData {
    if blob.is_null() || (*blob).static_article_info.is_null() {
        return blob;
    }
    let info = (*blob).static_article_info as *const custom_articles::StaticArticleData;
    if (*info).descriptors.is_null() || (*info).count == 0 {
        return blob;
    }
    let rows = core::slice::from_raw_parts((*info).descriptors, (*info).count);
    let mut masked: Vec<usize> = Vec::new();
    let mut kept: Vec<i32> = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        if !custom_articles::is_shared_weapon_kind(row.weapon_id) {
            continue;
        }
        if vanilla || custom_articles::is_kirby_copy_weapon(row.weapon_id) || !article_owner_loaded(row.weapon_id) {
            masked.push(index);
        } else {
            kept.push(row.weapon_id);
        }
    }
    if masked.is_empty() && kept.is_empty() {
        return blob;
    }
    let Ok(mut blobs) = masked_article_blobs().lock() else {
        return blob;
    };
    if let Some((_, _, _, result)) = blobs
        .iter()
        .find(|(cached_kind, cached_blob, cached_mask, _)| {
            *cached_kind == kind && *cached_blob == blob as usize && *cached_mask == masked
        })
    {
        return *result as *const StaticFighterData;
    }
    for weapon_kind in kept.iter() {
        if let Some(owner) = custom_articles::source_weapon_owner_kind(*weapon_kind) {
            fighter_modules::request(owner);
        }
    }
    let result = if masked.is_empty() {
        blob
    } else {
        let own_weapon = rows
            .iter()
            .find(|row| {
                (0..custom_articles::FIRST_CUSTOM_WEAPON_KIND).contains(&row.weapon_id) && row.extra & 1 == 0
            })
            .map(|row| row.weapon_id);
        let mut descriptors = rows.to_vec();
        for index in masked.iter() {
            let row = &mut descriptors[*index];
            if let Some(stand_in) = own_weapon.or_else(|| custom_articles::custom_weapon_source_kind(row.weapon_id)) {
                row.weapon_id = stand_in;
            }
            row.max_count = 0;
        }
        let count = descriptors.len();
        let article_info = Box::leak(Box::new(custom_articles::StaticArticleData {
            descriptors: Vec::leak(descriptors).as_ptr(),
            count,
        }));
        let mut patched = *blob;
        patched.static_article_info = article_info as *const _ as *const u8;
        Box::leak(Box::new(patched)) as *const StaticFighterData
    };
    dbg_log_public(&format!(
        "[article] fighter kind {kind}{}: {} Smashline-numbered article(s) kept, {} emptied",
        if vanilla { " (vanilla)" } else { "" },
        kept.len(),
        masked.len()
    ));
    blobs.push((kind, blob as usize, masked, result as usize));
    result
}

fn masked_article_blobs() -> &'static Mutex<Vec<(i32, usize, Vec<usize>, usize)>> {
    static BLOBS: OnceLock<Mutex<Vec<(i32, usize, Vec<usize>, usize)>>> = OnceLock::new();
    BLOBS.get_or_init(|| Mutex::new(Vec::new()))
}

fn patched_article_blobs() -> &'static Mutex<Vec<(i32, usize, Vec<i32>, usize)>> {
    static BLOBS: OnceLock<Mutex<Vec<(i32, usize, Vec<i32>, usize)>>> = OnceLock::new();
    BLOBS.get_or_init(|| Mutex::new(Vec::new()))
}

fn remapped_fighter_blobs() -> &'static Mutex<Vec<(i32, usize, usize, usize)>> {
    static BLOBS: OnceLock<Mutex<Vec<(i32, usize, usize, usize)>>> = OnceLock::new();
    BLOBS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "true_kind")]
static AUX_DATA_REMAP_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[skyline::hook(offset = crate::game::rel(OFF_FIGHTER_AUX_DATA_INIT))]
unsafe fn fighter_aux_data_init_hook(record: *mut u8, source: *mut u8, kind: i32) {
    if let Some(base) = clone_base(kind) {
        #[cfg(feature = "true_kind")]
        {
            let n = AUX_DATA_REMAP_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 32 {
                dbg_log!(
                    "[auxkind] #{n} remap kind {kind}->{base} record={:#x} source={:#x}",
                    record as usize,
                    source as usize
                );
            }
        }
        let row = fighter_param_rows::enter_for_clone(kind);
        let result = call_original!(record, source, base);
        fighter_param_rows::leave(row);
        fighter_param_rows::note_aux_data_init(kind);
        return result;
    }
    call_original!(record, source, kind)
}

#[cfg(feature = "true_kind")]
static BOUNDARY_REMAP_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[skyline::hook(offset = crate::game::rel(OFF_FIGHTER_BOUNDARY_PARAMS), inline)]
unsafe fn fighter_boundary_params_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[0].x() as i32;
    if let Some(base) = clone_base(kind) {
        ctx.registers[0].set_x(base as u64);
        #[cfg(feature = "true_kind")]
        {
            let n = BOUNDARY_REMAP_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 32 {
                dbg_log!("[boundkind] #{n} remap kind {kind}->{base} for 0x6797b0");
            }
        }
    }
}

#[cfg(feature = "true_kind")]
static AI_KIND_REMAP_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

unsafe fn remap_fighter_ai_kind(ctx: &mut skyline::hooks::InlineCtx, phase: &str) {
    let kind = ctx.registers[8].x() as i32;
    let base = clone_base(kind);
    if let Some(base) = base {
        ctx.registers[8].set_x(base as u64);
    }
    #[cfg(feature = "true_kind")]
    {
        let n = AI_KIND_REMAP_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 64 {
            let out = base.unwrap_or(kind);
            dbg_log!("[aikind] #{n} phase={phase} raw_kind={kind} out_kind={out}");
        }
    }
}

static AI_SETUP_LOGGED: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[skyline::hook(offset = crate::game::rel(OFF_AI_SETUP))]
unsafe fn fighter_ai_setup_hook(ai: *mut u8, path: *const u8) -> u64 {
    if !ai.is_null() {
        let field = ai.add(AI_KIND) as *mut i32;
        let kind = core::ptr::read_volatile(field);
        if let Some(base) = clone_base(kind) {
            core::ptr::write_volatile(field, base);
            if AI_SETUP_LOGGED.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 16 {
                skyline::println!(
                    "[clone_engine] kind {kind}'s computer player loads and reads the AI data of its base {base}"
                );
            }
        }
    }
    call_original!(ai, path)
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_PROFILE_KIND_BOUND), inline)]
unsafe fn fighter_ai_profile_kind_hook(ctx: &mut skyline::hooks::InlineCtx) {
    remap_fighter_ai_kind(ctx, "profile");
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_AGENT_KIND_BOUND), inline)]
unsafe fn fighter_ai_agent_kind_hook(ctx: &mut skyline::hooks::InlineCtx) {
    remap_fighter_ai_kind(ctx, "agents");
}

#[cfg(feature = "true_kind")]
static AI_DATA_KIND_REMAP_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

unsafe fn remap_ai_data_kind(ctx: &mut skyline::hooks::InlineCtx, register: usize, phase: &str) {
    let kind = ctx.registers[register].x() as i32;
    let base = clone_base(kind);
    if let Some(base) = base {
        ctx.registers[register].set_x(base as u64);
    }
    #[cfg(feature = "true_kind")]
    if base.is_some() {
        let n = AI_DATA_KIND_REMAP_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 48 {
            dbg_log!(
                "[aidata] #{n} phase={phase} raw_kind={kind} out_kind={}",
                base.unwrap()
            );
        }
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_ATTACK_LIST_KIND_BOUND), inline)]
unsafe fn fighter_ai_attack_list_kind_hook(ctx: &mut skyline::hooks::InlineCtx) {
    remap_ai_data_kind(ctx, 0, "attack_list");
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_ATTACK_DATA_KIND_BOUND), inline)]
unsafe fn fighter_ai_attack_data_kind_hook(ctx: &mut skyline::hooks::InlineCtx) {
    remap_ai_data_kind(ctx, 0, "attack_data");
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_PARAM_FLOAT_KIND_BOUND), inline)]
unsafe fn fighter_ai_param_float_kind_hook(ctx: &mut skyline::hooks::InlineCtx) {
    remap_ai_data_kind(ctx, 8, "param_float");
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_PARAM_INT_KIND_BOUND), inline)]
unsafe fn fighter_ai_param_int_kind_hook(ctx: &mut skyline::hooks::InlineCtx) {
    remap_ai_data_kind(ctx, 8, "param_int");
}

const OFF_FIGHTER_INIT_OBJ: usize = 0x6079d0;

#[cfg(any(
    feature = "diag_article_initspoof",
    feature = "true_kind",
    feature = "css_slot",
    feature = "native_table_backend"
))]
fn hash40(s: &str) -> u64 {
    let mut crc: u32 = 0xffff_ffff;
    for &b in s.as_bytes() {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    ((s.len() as u64) << 32) | (!crc) as u64
}

#[cfg(feature = "css_slot")]
static RESOURCE_CONTEXT: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("resource_context");

#[cfg(feature = "css_slot")]
static CONSTRUCTION_CONTEXT: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("construction_context");

#[cfg(feature = "css_slot")]
static VANILLA_CONSTRUCTION_CONTEXT: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("vanilla_construction");

unsafe fn current_thread_key() -> usize {
    skyline::nn::os::GetCurrentThread() as usize
}

#[cfg(feature = "css_slot")]
unsafe fn active_resource_kind() -> Option<i32> {
    let kind = RESOURCE_CONTEXT.active(current_thread_key())?;
    clone_definition(kind).map(|_| kind)
}

#[cfg(all(feature = "diag_trail", feature = "css_slot"))]
pub(crate) unsafe fn active_resource_kind_for_diagnostics() -> Option<i32> {
    active_resource_kind()
}

#[cfg(feature = "css_slot")]
unsafe fn with_resource_context<R>(kind: i32, callback: impl FnOnce() -> R) -> R {
    if clone_definition(kind).is_none() {
        return callback();
    }
    RESOURCE_CONTEXT.scope(current_thread_key(), kind, callback)
}

#[cfg(feature = "css_slot")]
unsafe fn active_construction_kind() -> Option<i32> {
    let kind = CONSTRUCTION_CONTEXT.active(current_thread_key())?;
    clone_definition(kind).map(|_| kind)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn active_construction_kind_public() -> Option<i32> {
    active_construction_kind()
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn active_vanilla_construction_kind() -> Option<i32> {
    let kind = VANILLA_CONSTRUCTION_CONTEXT.active(current_thread_key())?;
    clone_definition(kind).is_none().then_some(kind)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn with_vanilla_construction_context<R>(
    kind: i32,
    callback: impl FnOnce() -> R,
) -> R {
    if kind < 0 || clone_definition(kind).is_some() {
        return callback();
    }
    VANILLA_CONSTRUCTION_CONTEXT.scope(current_thread_key(), kind, callback)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn with_construction_context_public<R>(
    kind: i32,
    callback: impl FnOnce() -> R,
) -> R {
    with_construction_context(kind, callback)
}

#[cfg(feature = "css_slot")]
unsafe fn with_construction_context<R>(kind: i32, callback: impl FnOnce() -> R) -> R {
    if clone_definition(kind).is_none() {
        return callback();
    }
    CONSTRUCTION_CONTEXT.scope(current_thread_key(), kind, callback)
}

#[cfg(feature = "css_slot")]
unsafe fn handle_custom_resource_name(ctx: &mut skyline::hooks::InlineCtx, destination: usize) {
    let Some(kind) = active_resource_kind() else {
        return;
    };
    let Some(definition) = clone_definition(kind) else {
        return;
    };
    ctx.registers[destination].set_x(definition.resource_name_cstr.as_ptr() as u64);
}

#[cfg(feature = "css_slot")]
unsafe fn handle_custom_resource_name_from_register(
    ctx: &mut skyline::hooks::InlineCtx,
    destination: usize,
    kind_register: usize,
) -> Option<i32> {
    let raw_kind = ctx.registers[kind_register].x() as i32;
    if let Some(definition) = clone_definition(raw_kind) {
        ctx.registers[destination].set_x(definition.resource_name_cstr.as_ptr() as u64);
        return Some(raw_kind);
    }

    handle_custom_resource_name(ctx, destination);
    None
}

#[cfg(feature = "css_slot")]
unsafe fn handle_custom_resource_name_w1(ctx: &mut skyline::hooks::InlineCtx, destination: usize) {
    handle_custom_resource_name_from_register(ctx, destination, 1);
}

#[cfg(feature = "css_slot")]
unsafe fn handle_custom_resource_name_w19(ctx: &mut skyline::hooks::InlineCtx, destination: usize) {
    handle_custom_resource_name_from_register(ctx, destination, 19);
}

#[cfg(feature = "css_slot")]
unsafe fn handle_custom_resource_name_w20(ctx: &mut skyline::hooks::InlineCtx, destination: usize) {
    handle_custom_resource_name_from_register(ctx, destination, 20);
}

#[cfg(feature = "css_slot")]
unsafe fn handle_custom_resource_name_w21(ctx: &mut skyline::hooks::InlineCtx, destination: usize) {
    handle_custom_resource_name_from_register(ctx, destination, 21);
}

#[cfg(feature = "css_slot")]
static CUSTOM_MOTION_NAME_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
unsafe fn handle_custom_resource_name_w8(ctx: &mut skyline::hooks::InlineCtx, destination: usize) {
    let raw_kind = ctx.registers[8].x() as i32;
    let object = ctx.registers[19].x() as usize;
    let entry_id = if object != 0 {
        *((object + 0x10) as *const i32)
    } else {
        -1
    };
    let recovered_kind = if (0..8).contains(&entry_id) {
        entry_custom_kind(entry_id as u8)
    } else {
        None
    };
    let resolved_kind = clone_definition(raw_kind)
        .map(|_| raw_kind)
        .or_else(|| clone_kind_of_object(object as u64))
        .or(recovered_kind)
        .or_else(|| active_resource_kind());

    if let Some(kind) = resolved_kind {
        let definition = clone_definition(kind).unwrap();
        ctx.registers[destination].set_x(definition.resource_name_cstr.as_ptr() as u64);
        let n = CUSTOM_MOTION_NAME_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            dbg_log!(
                "[motionname] #{n} raw_kind={raw_kind} entry={entry_id} recovered={recovered_kind:?} \
                 true_kind={kind} namespace={} common_merge=0x60c184",
                definition.resource_name
            );
        }
    }
}

#[cfg(feature = "css_slot")]
static ARTICLE_OWNER_NAME_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static ARTICLE_WEAPON_NAME_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static ARTICLE_PATH_RESOLVE_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static ARTICLE_CACHE_KEY_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static ARTICLE_DATA_CACHE_KEY_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static ARTICLE_DATA_CACHE_LOOKUP_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static ARTICLE_DATA_CACHE_RESULT_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static CUSTOM_EFFECT_BANK_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static CUSTOM_EFFECT_BANK_MISS_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
static PENDING_EFFECT_THREADS: [core::sync::atomic::AtomicUsize; 8] = [
    core::sync::atomic::AtomicUsize::new(0),
    core::sync::atomic::AtomicUsize::new(0),
    core::sync::atomic::AtomicUsize::new(0),
    core::sync::atomic::AtomicUsize::new(0),
    core::sync::atomic::AtomicUsize::new(0),
    core::sync::atomic::AtomicUsize::new(0),
    core::sync::atomic::AtomicUsize::new(0),
    core::sync::atomic::AtomicUsize::new(0),
];
#[cfg(feature = "css_slot")]
static PENDING_EFFECT_KINDS: [core::sync::atomic::AtomicI32; 8] = [
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
];
#[cfg(feature = "css_slot")]
static PENDING_EFFECT_VANILLA_INDEX: [core::sync::atomic::AtomicI32; 8] = [
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
    core::sync::atomic::AtomicI32::new(-1),
];

#[cfg(feature = "css_slot")]
unsafe fn arm_pending_effect_kind(kind: i32, vanilla_index: i32) {
    let thread = current_thread_key();
    if thread == 0 || clone_definition(kind).is_none() {
        return;
    }

    for index in 0..PENDING_EFFECT_THREADS.len() {
        let owner = PENDING_EFFECT_THREADS[index].load(core::sync::atomic::Ordering::Acquire);
        if owner == thread {
            PENDING_EFFECT_KINDS[index].store(kind, core::sync::atomic::Ordering::Release);
            PENDING_EFFECT_VANILLA_INDEX[index]
                .store(vanilla_index, core::sync::atomic::Ordering::Release);
            return;
        }
        if owner == 0
            && PENDING_EFFECT_THREADS[index]
                .compare_exchange(
                    0,
                    thread,
                    core::sync::atomic::Ordering::AcqRel,
                    core::sync::atomic::Ordering::Relaxed,
                )
                .is_ok()
        {
            PENDING_EFFECT_KINDS[index].store(kind, core::sync::atomic::Ordering::Release);
            PENDING_EFFECT_VANILLA_INDEX[index]
                .store(vanilla_index, core::sync::atomic::Ordering::Release);
            return;
        }
    }
    dbg_log!("[effectbank] WARNING no pending-context slot for true_kind={kind}");
}

#[cfg(feature = "css_slot")]
unsafe fn take_pending_effect_kind(handle: u32) -> Option<(i32, i32)> {
    let thread = current_thread_key();
    if thread == 0 {
        return None;
    }

    for index in 0..PENDING_EFFECT_THREADS.len() {
        if PENDING_EFFECT_THREADS[index].load(core::sync::atomic::Ordering::Acquire) == thread {
            let kind = PENDING_EFFECT_KINDS[index].load(core::sync::atomic::Ordering::Acquire);
            let definition = clone_definition(kind)?;
            let base_handle = 0x300 + definition.base_kind as u32;
            if handle < base_handle || (handle - base_handle) % 2000 != 0 {
                return None;
            }
            let vanilla_index =
                PENDING_EFFECT_VANILLA_INDEX[index].load(core::sync::atomic::Ordering::Acquire);
            PENDING_EFFECT_KINDS[index].store(-1, core::sync::atomic::Ordering::Release);
            PENDING_EFFECT_VANILLA_INDEX[index].store(-1, core::sync::atomic::Ordering::Release);
            PENDING_EFFECT_THREADS[index].store(0, core::sync::atomic::Ordering::Release);
            return Some((kind, vanilla_index));
        }
    }
    None
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x355f8f0))]
unsafe fn custom_effect_bank_load(manager: *mut u64, handle: u32, search_index: *const u32) -> u32 {
    {
        static ENTRY_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = ENTRY_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            let index = if search_index.is_null() {
                u32::MAX
            } else {
                *search_index
            };
            dbg_log!("[effectbank] enter #{n} handle={handle:#x} search_index={index:#x}");
        }
    }
    #[cfg(feature = "item_clone_backend")]
    if owner_effects::is_fighter_handle(handle) {
        owner_effects::remember_manager(manager as usize);
    }
    let pending = take_pending_effect_kind(handle);
    let Some(kind) = active_construction_kind().or(pending.map(|p| p.0)) else {
        if (0x300..0x400).contains(&handle) {
            let n = CUSTOM_EFFECT_BANK_MISS_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 16 {
                dbg_log!("[effectbank] base-load #{n} handle={handle:#x} no clone context");
            }
        }
        return call_original!(manager, handle, search_index);
    };
    let Some(definition) = clone_definition(kind) else {
        return call_original!(manager, handle, search_index);
    };
    let vanilla_index = pending
        .map(|p| p.1)
        .filter(|index| *index >= 0 && *index != RESOURCE_INDEX_NOT_FOUND);

    const FIGHTER_EFFECT_HANDLE_BASE: u32 = 0x300;
    const TRANSPLANT_HANDLE_STRIDE: u32 = 2000;
    let base_handle = FIGHTER_EFFECT_HANDLE_BASE + definition.base_kind as u32;
    let transplanted = handle >= base_handle && (handle - base_handle) % TRANSPLANT_HANDLE_STRIDE == 0;
    let bank_owner = if transplanted { effect_bank_owner(definition) } else { None };
    if transplanted && bank_owner.is_none() {
        return match vanilla_index {
            Some(index) => {
                let index = index as u32;
                call_original!(manager, handle, &index as *const u32)
            }
            None => call_original!(manager, handle, search_index),
        };
    }
    let mapped_handle = match bank_owner {
        Some(owner) => {
            FIGHTER_EFFECT_HANDLE_BASE
                + owner.base_kind as u32
                + (handle - base_handle)
                + TRANSPLANT_HANDLE_STRIDE * owner.effect_namespace
        }
        None => handle,
    };

    let n = CUSTOM_EFFECT_BANK_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        let index = if search_index.is_null() {
            u32::MAX
        } else {
            *search_index
        };
        dbg_log!(
            "[effectbank] #{n} true_kind={} base={} handle={:#x}->{:#x} search_index={:#x} namespace={}",
            definition.kind,
            definition.base_kind,
            handle,
            mapped_handle,
            index,
            definition.resource_name
        );
    }
    let result = call_original!(manager, mapped_handle, search_index);
    if n < 24 {
        dbg_log!("[effectbank] #{n} load-result={result} mapped_handle={mapped_handle:#x}");
    }
    if mapped_handle != handle && effect_bank_refcount(manager as usize, mapped_handle) > 0 {
        note_mapped_effect_bank(manager as usize, handle, mapped_handle);
    }
    if mapped_handle != handle {
        match vanilla_index {
            Some(index) => {
                let index_u32 = index as u32;
                let base_result = call_original!(manager, handle, &index_u32 as *const u32);
                if n < 24 {
                    dbg_log!(
                        "[effectbank] #{n} base-dual-load result={base_result} handle={handle:#x} index={index_u32:#x}"
                    );
                }
            }
            None => {
                if n < 24 {
                    dbg_log!("[effectbank] #{n} base-dual-load SKIPPED no vanilla index");
                }
            }
        }
    }
    result
}

#[cfg(feature = "css_slot")]
unsafe fn effect_bank_refcount(manager: usize, handle: u32) -> u32 {
    const BANK_MUTEX: usize = 0x19498;
    const SLOTS: usize = 0x18000;
    const SLOT_SIZE: usize = 0xa8;
    const SLOT_REFCOUNT: usize = 0x98;
    if manager == 0 {
        return 0;
    }
    let mutex_lock: extern "C" fn(u64) = core::mem::transmute(crate::game::addr(OFF_STD_MUTEX_LOCK));
    let mutex_unlock: extern "C" fn(u64) = core::mem::transmute(crate::game::addr(OFF_STD_MUTEX_UNLOCK));
    mutex_lock((manager + BANK_MUTEX) as u64);
    let refcount = effect_bank_find(manager, handle)
        .map(|slot| core::ptr::read_volatile((manager + SLOTS + slot as usize * SLOT_SIZE + SLOT_REFCOUNT) as *const u32))
        .unwrap_or(0);
    mutex_unlock((manager + BANK_MUTEX) as u64);
    refcount
}

#[cfg(feature = "css_slot")]
unsafe fn effect_bank_find(manager: usize, handle: u32) -> Option<u32> {
    const BUCKETS: usize = 0x193b0;
    const BUCKET_COUNT: usize = 0x193b8;
    let count = core::ptr::read_volatile((manager + BUCKET_COUNT) as *const usize);
    let buckets = core::ptr::read_volatile((manager + BUCKETS) as *const usize);
    if count == 0 || buckets == 0 {
        return None;
    }
    let key = handle as usize;
    let bucket_of = |hash: usize| if count & (count - 1) == 0 { hash & (count - 1) } else { hash % count };
    let index = bucket_of(key);
    let before = core::ptr::read_volatile((buckets + index * 8) as *const usize);
    if before == 0 {
        return None;
    }
    let mut node = core::ptr::read_volatile(before as *const usize);
    let mut steps = 0;
    while node != 0 && steps < 4096 {
        let hash = core::ptr::read_volatile((node + 8) as *const usize);
        if hash == key {
            if core::ptr::read_volatile((node + 0x10) as *const u32) == handle {
                return Some(core::ptr::read_volatile((node + 0x14) as *const u32));
            }
        } else if bucket_of(hash) != index {
            return None;
        }
        node = core::ptr::read_volatile(node as *const usize);
        steps += 1;
    }
    None
}

#[cfg(feature = "css_slot")]
unsafe fn arc_file_info_indice(hash: u64) -> Option<u32> {
    const ARC_SERVICE: usize = 0x5331f20;
    const SERVICE_ARC: usize = 0x78;
    const ARC_FILE_BUCKETS: usize = 0x50;
    const ARC_HASH_TO_PATH: usize = 0x58;
    const ARC_FILE_PATHS: usize = 0x60;
    const FILE_PATH_STRIDE: usize = 0x20;
    let service = core::ptr::read_volatile(crate::game::addr(ARC_SERVICE) as *const usize);
    if service == 0 {
        return None;
    }
    let holder = core::ptr::read_volatile((service + SERVICE_ARC) as *const usize);
    if holder == 0 {
        return None;
    }
    let arc = core::ptr::read_volatile(holder as *const usize);
    if arc == 0 {
        return None;
    }
    let buckets = core::ptr::read_volatile((arc + ARC_FILE_BUCKETS) as *const usize);
    let lookup = core::ptr::read_volatile((arc + ARC_HASH_TO_PATH) as *const usize);
    let paths = core::ptr::read_volatile((arc + ARC_FILE_PATHS) as *const usize);
    if buckets == 0 || lookup == 0 || paths == 0 {
        return None;
    }
    let bucket_count = core::ptr::read_volatile((buckets + 4) as *const u32) as u64;
    if bucket_count == 0 {
        return None;
    }
    let bucket = buckets + 8 + (hash % bucket_count) as usize * 8;
    let start = core::ptr::read_volatile(bucket as *const u32) as usize;
    let count = core::ptr::read_volatile((bucket + 4) as *const u32) as usize;
    let (mut low, mut high) = (0, count);
    while low < high {
        let middle = (low + high) / 2;
        let entry = core::ptr::read_volatile((lookup + (start + middle) * 8) as *const u64);
        let key = entry & 0xff_ffff_ffff;
        if key == hash {
            let path = core::ptr::read_volatile(
                (paths + (entry >> 40) as usize * FILE_PATH_STRIDE) as *const u64,
            );
            return Some((path >> 40) as u32);
        }
        if key < hash {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    None
}

#[cfg(feature = "css_slot")]
unsafe fn own_effect_file(definition: &CloneDefinition) -> Option<u32> {
    static KNOWN: OnceLock<Mutex<Vec<(i32, Option<u32>)>>> = OnceLock::new();
    let known = KNOWN.get_or_init(|| Mutex::new(Vec::new()));
    if let Some(&(_, file)) = known.lock().unwrap().iter().find(|entry| entry.0 == definition.kind) {
        return file;
    }
    let path = |name: &str| hash40(&format!("effect/fighter/{name}/ef_{name}.eff"));
    let own_file = arc_file_info_indice(path(definition.resource_name));
    let base_file = arc_file_info_indice(path(definition.base_resource_name));
    let file = own_file.filter(|file| Some(*file) != base_file);
    known.lock().unwrap().push((definition.kind, file));
    file
}

#[cfg(feature = "css_slot")]
unsafe fn effect_bank_owner(definition: &'static CloneDefinition) -> Option<&'static CloneDefinition> {
    let Some(file) = own_effect_file(definition) else {
        tell_effect_bank(definition, "shares the base's effect file; only the base bank loads");
        return None;
    };
    let candidates: Vec<&'static CloneDefinition> = clone_definitions().read().unwrap().clone();
    let sharing: Vec<&'static CloneDefinition> = candidates
        .into_iter()
        .filter(|candidate| own_effect_file(candidate) == Some(file))
        .collect();
    let owner = sharing.iter().copied().min_by_key(|candidate| candidate.kind).unwrap_or(definition);
    let others: Vec<&str> = sharing
        .iter()
        .filter(|candidate| candidate.kind != definition.kind)
        .map(|candidate| candidate.resource_name)
        .collect();
    if others.is_empty() {
        tell_effect_bank(definition, "ships its own effect file; it loads beside the base's");
    } else {
        tell_effect_bank(
            definition,
            &format!("shares one effect file with {}; one bank serves them", others.join(", ")),
        );
    }
    Some(owner)
}

#[cfg(feature = "css_slot")]
fn tell_effect_bank(definition: &CloneDefinition, message: &str) {
    static TOLD: OnceLock<Mutex<Vec<i32>>> = OnceLock::new();
    let mut told = TOLD.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    if !told.contains(&definition.kind) {
        told.push(definition.kind);
        dbg_log_public(&format!("[effectbank] {} {message}", definition.resource_name));
    }
}

#[cfg(feature = "css_slot")]
fn mapped_effect_banks() -> &'static Mutex<Vec<(usize, u32, u32, u32)>> {
    static BANKS: OnceLock<Mutex<Vec<(usize, u32, u32, u32)>>> = OnceLock::new();
    BANKS.get_or_init(|| Mutex::new(Vec::new()))
}

#[cfg(feature = "css_slot")]
fn note_mapped_effect_bank(manager: usize, base: u32, mapped: u32) {
    let mut banks = mapped_effect_banks().lock().unwrap();
    match banks.iter_mut().find(|bank| bank.0 == manager && bank.1 == base && bank.2 == mapped) {
        Some(bank) => bank.3 += 1,
        None => banks.push((manager, base, mapped, 1)),
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x3563720))]
unsafe fn custom_effect_bank_release(manager: *mut u64, handle: i32) {
    call_original!(manager, handle);
    let address = manager as usize;
    let released = handle as u32;
    {
        let mut banks = mapped_effect_banks().lock().unwrap();
        if let Some(bank) = banks.iter_mut().find(|bank| bank.0 == address && bank.2 == released) {
            bank.3 = bank.3.saturating_sub(1);
        }
        banks.retain(|bank| bank.3 > 0);
        if !banks.iter().any(|bank| bank.0 == address && bank.1 == released) {
            return;
        }
    }
    if effect_bank_refcount(address, released) > 0 {
        return;
    }
    let owed: Vec<(u32, u32)> = {
        let mut banks = mapped_effect_banks().lock().unwrap();
        let owed = banks
            .iter()
            .filter(|bank| bank.0 == address && bank.1 == released)
            .map(|bank| (bank.2, bank.3))
            .collect();
        banks.retain(|bank| !(bank.0 == address && bank.1 == released));
        owed
    };
    for (mapped, count) in owed {
        let mut freed = 0;
        while freed < count && effect_bank_refcount(address, mapped) > 0 {
            call_original!(manager, mapped as i32);
            freed += 1;
        }
        dbg_log!("[effectbank] base {released:#x} released, mapped {mapped:#x} released {freed} of {count}");
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static ARTICLE_OWNER_OVERRIDE: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("article_owner_override");

#[cfg(feature = "css_slot")]
pub(crate) static ARTICLE_SETUP_OWNER: crate::thread_context::ThreadScopedKind =
    crate::thread_context::ThreadScopedKind::new("article_setup_owner");

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17e0a4c), inline)]
unsafe fn custom_article_owner_name(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[26].x() as i32;
    if let Some(owner) = custom_articles::custom_weapon_owner_name(weapon_kind) {
        ctx.registers[25].set_x(owner.as_ptr() as u64);
        return;
    }
    if let Some(definition) =
        ARTICLE_OWNER_OVERRIDE.active(current_thread_key()).and_then(clone_definition)
    {
        ctx.registers[25].set_x(definition.resource_name_cstr.as_ptr() as u64);
        return;
    }
    let Some(kind) = active_construction_kind() else {
        return;
    };
    let Some(definition) = clone_definition(kind) else {
        return;
    };
    if !definition.ships_own_param_resources() {
        return;
    }
    ctx.registers[25].set_x(definition.resource_name_cstr.as_ptr() as u64);
    let n = ARTICLE_OWNER_NAME_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!(
            "[articleowner] #{n} true_kind={} base={} namespace={} get_file=0x17e0a4c",
            definition.kind,
            definition.base_resource_name,
            definition.resource_name
        );
    }
}

#[cfg(feature = "css_slot")]
fn custom_article_definition(
    kind: i32,
    weapon_kind: i32,
) -> Option<&'static CloneArticleDefinition> {
    clone_definition(kind)?
        .articles
        .iter()
        .find(|article| article.base_weapon_kind == weapon_kind)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(0x17e098c), inline)]
unsafe fn custom_article_weapon_name(ctx: &mut skyline::hooks::InlineCtx) {
    let weapon_kind = ctx.registers[23].x() as i32;
    if let Some(name) = custom_articles::custom_weapon_name(weapon_kind) {
        ctx.registers[22].set_x(name.as_ptr() as u64);
        return;
    }
    if let Some(copy_kind) = active_kirby_copy_kind() {
        let n = ARTICLE_WEAPON_NAME_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            dbg_log!(
                "[articlefile] #{n} kirby-copy of kind={copy_kind} weapon_kind={weapon_kind:#x}: keeping the BASE namespace"
            );
        }
        return;
    }
    let Some(kind) = active_construction_kind() else {
        return;
    };
    let Some(article) = custom_article_definition(kind, weapon_kind) else {
        return;
    };
    let name = article.file_name_cstr;
    ctx.registers[22].set_x(name.as_ptr() as u64);
    let n = ARTICLE_WEAPON_NAME_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!(
            "[articlefile] #{n} true_kind={kind} weapon_kind={weapon_kind:#x} name={}",
            article.file_name
        );
    }
}

mod article_bridges;
use article_bridges::*;

#[cfg(feature = "css_slot")]
mod copy_model_probe;

#[cfg(feature = "css_slot")]
mod kirby_copy;
#[cfg(feature = "css_slot")]
use kirby_copy::*;

#[cfg(feature = "kirby_copy_motions")]
mod kirby_motions;
#[cfg(feature = "css_slot")]
mod global_motions;
#[cfg(feature = "css_slot")]
mod grab_rules;
#[cfg(feature = "css_slot")]
mod hud_grafts;
#[cfg(feature = "css_slot")]
mod hud_layouts;
#[cfg(feature = "css_slot")]
mod move_list;
mod mii_parts;
#[cfg(feature = "css_slot")]
mod boss_gauge;
#[cfg(feature = "css_slot")]
mod fighter_change;
#[cfg(feature = "css_slot")]
mod final_grafts;
#[cfg(feature = "css_slot")]
mod tether;

#[cfg(feature = "css_slot")]
mod css_registration;
#[cfg(feature = "css_slot")]
use css_registration::*;
#[cfg(feature = "css_slot")]
mod csk_row_bridge;
#[cfg(feature = "css_slot")]
mod chara_orders;
#[cfg(feature = "css_slot")]
mod arc_files;

#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
mod fighter_params;
#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
mod fighter_common_copies;
mod fighter_param_rows;
#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
mod fighter_param_ai;
#[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
mod fighter_param_thrown;
#[cfg(feature = "true_kind")]
mod load_pipeline;
#[cfg(feature = "true_kind")]
use load_pipeline::*;

mod article_probes;
use article_probes::*;

mod smashline_names;
mod smashline_articles;

fn smashline_bridge_version() -> u32 {
    let version = SMASHLINE_BRIDGE_VERSION.load(core::sync::atomic::Ordering::Acquire);
    if version != 0 {
        return version;
    }
    report_smashline_name_support();
    common_vtables::note_smashline();
    SMASHLINE_BRIDGE_VERSION.load(core::sync::atomic::Ordering::Acquire)
}

fn report_smashline_name_support() {
    let mut address = 0usize;
    let result = unsafe {
        skyline::nn::ro::LookupSymbol(
            &mut address as *mut usize,
            b"smashline_clone_engine_bridge_version\0".as_ptr(),
        )
    };
    if result != 0 || address == 0 {
        SMASHLINE_BRIDGE_VERSION.store(0, core::sync::atomic::Ordering::Release);
        skyline::println!(
            "[clone_engine] smashline: stock build; agent names come from the engine's name slots"
        );
        return;
    }
    let version = unsafe { core::mem::transmute::<usize, extern "C" fn() -> u32>(address)() };
    SMASHLINE_BRIDGE_VERSION.store(version, core::sync::atomic::Ordering::Release);
    skyline::println!(
        "[clone_engine] smashline: bridge v{version} (required v{SMASHLINE_BRIDGE_VERSION_REQUIRED})"
    );
}

#[no_mangle]
pub extern "C" fn clone_engine_smashline_bridge_version() -> u32 {
    SMASHLINE_BRIDGE_VERSION.load(core::sync::atomic::Ordering::Acquire)
}

#[cfg(not(test))]
#[skyline::main(name = "dionysus")]
pub fn main() {
    upgrade::stop_if_old_plugin_installed();
    upgrade::carry_old_folder();
    skyline::println!("[dionysus] init (SSBU {})", game::version().name());
    report_smashline_name_support();
    common_vtables::note_smashline();
    unsafe { smashline_vtables::install() };
    settings::get();

    #[cfg(feature = "stage_relocate")]
    stage_transaction::apply_select_cap("early");
    #[cfg(feature = "stage_relocate")]
    stage_select_cap::install_runtime_refresh();
    #[cfg(not(feature = "native_relocate"))]
    skyline::println!(
        "[clone_engine] A/B BUILD: native_relocate=OFF; fixed legacy kinds only (expect no [resreloc] lines)"
    );

    #[cfg(feature = "diag_item_kind")]
    item_re::install();

    #[cfg(feature = "diag_item_categories")]
    item_category_probe::install();
    #[cfg(feature = "diag_item_ui")]
    item_ui_probe::install();

    #[cfg(feature = "item_clone_backend")]
    item_spawn::install();
    #[cfg(feature = "item_clone_backend")]
    enemy_spawn::install();
    #[cfg(feature = "item_clone_backend")]
    enemy_clones::install();
    #[cfg(feature = "item_clone_backend")]
    enemy_drivers::install();
    #[cfg(feature = "item_clone_backend")]
    enemy_params::install();
    #[cfg(feature = "item_clone_backend")]
    item_slots::install();
    #[cfg(feature = "item_clone_backend")]
    item_clones::install();
    #[cfg(feature = "item_clone_backend")]
    block_grid::install();
    #[cfg(feature = "css_slot")]
    finalsmash_residency::install();
    #[cfg(feature = "css_slot")]
    article_item_order::install();
    #[cfg(feature = "css_slot")]
    append_group::install();
    #[cfg(feature = "css_slot")]
    entry_groups::install();
    #[cfg(feature = "css_slot")]
    fight_commands::install();
    #[cfg(feature = "css_slot")]
    copy_model_probe::install();
    #[cfg(feature = "css_slot")]
    hud_grafts::install();
    #[cfg(feature = "css_slot")]
    hud_layouts::install();
    #[cfg(feature = "css_slot")]
    move_list::install();
    #[cfg(feature = "css_slot")]
    boss_gauge::install();
    #[cfg(feature = "css_slot")]
    fighter_change::install();
    #[cfg(feature = "css_slot")]
    final_grafts::install_hooks();
    #[cfg(feature = "css_slot")]
    tether::install();
    #[cfg(feature = "item_clone_backend")]
    if item_slots::ready() {
        item_clones::mark_resource_router_ready();
    }

    #[cfg(feature = "item_ui_backend")]
    item_ui::install();

    #[cfg(feature = "kirby_copy_motions")]
    kirby_motions::install();

    #[cfg(feature = "css_slot")]
    fighter_packs::load_all();
    mount_events::subscribe();

    #[cfg(feature = "item_clone_backend")]
    item_packs::load_all();

    mii_parts::load_all();

    #[cfg(feature = "item_clone_backend")]
    enemy_packs::load_all();

    #[cfg(feature = "item_clone_backend")]
    item_params::install();
    #[cfg(feature = "item_clone_backend")]
    if item_params::ready() {
        item_clones::mark_param_router_ready();
    }
    #[cfg(feature = "item_clone_backend")]
    item_generate::install();

    #[cfg(feature = "item_clone_backend")]
    item_scripts::install();
    #[cfg(feature = "item_clone_backend")]
    if item_slots::ready() && item_params::ready() && item_scripts::ready() {
        item_clones::mark_category_router_ready();
    }

    #[cfg(feature = "stage_page")]
    stage_select_page::install();

    #[cfg(feature = "stage_slice")]
    stage_pane_table::install();
    #[cfg(feature = "stage_slice")]
    stage_select_slice::install();

    stage_config_bridge::install();

    #[cfg(feature = "stage_select_runtime")]
    stage_resolve_probe::install();

    #[cfg(feature = "stage_mint_places")]
    stage_dispatch::install();
    stage_collision_probe::install();

    #[cfg(feature = "stage_mint")]
    stage_packs::load_all();

    #[cfg(feature = "css_slot")]
    menu_sequences::load_all();

    #[cfg(feature = "css_slot")]
    costume_slots::scan_mods();
    #[cfg(feature = "css_slot")]
    {
        skyline::install_hook!(css_icon_column_bound_hook);
        skyline::println!(
            "[clone_engine] CSS grid overflow guard active: columns above 12 reuse the last native icon-animation delay"
        );
    }

    match shared_hooks::initialize() {
        Ok(()) => skyline::println!("[sharedhook] dispatch self-test passed"),
        Err(reason) => skyline::println!(
            "[sharedhook] DISPATCH SELF-TEST FAILED ({reason}); registration is disabled"
        ),
    }

    #[cfg(feature = "native_table_backend")]
    native_tables::initialize();

    #[cfg(feature = "native_relocate")]
    resource_relocation::start();

    #[cfg(feature = "stage_probe")]
    stage_probe::run();

    #[cfg(feature = "demo")]
    register_clone_fighter(FIRST_CUSTOM_KIND, 0);

    #[cfg(feature = "diag_identity")]
    {
        registry().write().unwrap().insert(0, 0);
        skyline::println!(
            "[clone_engine] DIAG identity: kind 0 -> base 0 (via chain). Mario MUST behave 100% like vanilla."
        );
    }

    #[cfg(feature = "diag_swap")]
    {
        registry().write().unwrap().insert(1, 0);
        skyline::println!(
            "[clone_engine] DIAG swap: kind 1 (Donkey) -> base 0 (Mario, via chain). Expect DK to run Mario's agents."
        );
    }

    #[cfg(all(feature = "diag_article", not(feature = "true_kind")))]
    {
        registry().write().unwrap().insert(1, 0);
        skyline::println!(
            "[clone_engine] DIAG article: kind 1 (Donkey) -> base 0 (Mario) + generate_article probe + resolver remap. Play DK vs Mario; DK's [article_probe] lines should now show ret != 0 (Mario articles on DK)."
        );
        skyline::println!(
            "[clone_engine] registry self-check: clone_base(1) = {:?}",
            clone_base(1)
        );
    }

    #[cfg(feature = "true_kind")]
    {
        skyline::println!(
            "[clone_engine] true-kind runtime active; waiting for content-plugin registrations"
        );
    }

    #[cfg(feature = "diag_alloc_selftest")]
    unsafe {
        let identity = b"fighter_kind_alloc_selftest\0";
        let resource = b"alloc_selftest\0";
        let request = CloneRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneRegistrationV1>() as u32,
            custom_kind: KIND_AUTO,
            base_kind: 0,
            ui_chara: b"alloc_selftest\0".as_ptr().cast(),
            fighter_kind_name: identity.as_ptr().cast(),
            resource_name: resource.as_ptr().cast(),
            base_resource_name: b"mario\0".as_ptr().cast(),
            color_start: 0,
            color_count: 1,
            copy_status_first: -1,
            copy_status_count: 0,
            article_namespace: 0,
            effect_namespace: 0,
            articles: core::ptr::null(),
            article_count: 0,
            flags: 0,
            reserved: [0; 4],
        };
        let first = clone_engine_register_v1(&request);
        let again = clone_engine_register_v1(&request);
        let looked_up = clone_engine_kind_for_identity(identity.as_ptr().cast());
        let ceiling = clone_engine_max_custom_kind();
        skyline::println!(
            "[allocself] allocate={first} repeat={again} lookup={looked_up} ceiling={ceiling} committed_at_boot={}",
            clone_engine_capacity_committed()
        );
        let allocated_in_range = first >= FIRST_ALLOCATABLE_KIND && first <= ceiling;
        skyline::println!(
            "[allocself] {} in_range={allocated_in_range} idempotent={} lookup_matches={}",
            if allocated_in_range && first == again && first == looked_up {
                "PASS"
            } else {
                "FAIL"
            },
            first == again,
            first == looked_up
        );
    }

    skyline::install_hooks!(
        status_script_hook,
        animcmd_game_hook,
        animcmd_effect_hook,
        animcmd_expression_hook,
        animcmd_sound_hook,
        animcmd_game_share_hook,
        animcmd_effect_share_hook,
        animcmd_expression_share_hook,
        animcmd_sound_share_hook,
        fighter_class_resolver_hook,
        static_fighter_data_hook,
        fighter_aux_data_init_hook,
        fighter_boundary_params_hook,
        fighter_ai_setup_hook,
        fighter_ai_profile_kind_hook,
        fighter_ai_agent_kind_hook,
        fighter_ai_attack_list_kind_hook,
        fighter_ai_attack_data_kind_hook,
        fighter_ai_param_float_kind_hook,
        fighter_ai_param_int_kind_hook,
    );

    skyline::println!(
        "[clone_engine] installed 20 kind-spoof hooks (9 create_agent dispatchers + fighter-class resolver + static fighter data + auxiliary record + boundary parameters + CPU setup/profile/action/mode gates + four AI data selectors)"
    );

    #[cfg(any(feature = "diag_article", feature = "css_slot"))]
    {
        skyline::install_hook!(generate_article_probe);
        skyline::println!(
            "[clone_engine] installed generate_article probe hook (css_slot builds log only tracked Kirby clone-copy BOMA)"
        );
    }

    #[cfg(feature = "css_slot")]
    {
        skyline::install_hooks!(article_probes::observer_purge_refcount_probe);
        skyline::println!(
            "[clone_engine] installed the observer purge refcount probe at 0x37ae244; tag [purge], and [purge] UNDERFLOW means a listener node is being destroyed twice"
        );
        skyline::install_hooks!(kirby_article_init_probe, kirby_article_init_guard,);
        skyline::println!(
            "[clone_engine] kirbyinit: copied-article init guard at 0xba3e2c (LOAD-BEARING) plus a              bounded probe at 0xba3e24. 0xba3df0 tail-branches to              ArticleDescriptor.on_init_callback via the BASE fighter's table; the guard serves the              published Kirby-copy header's callback when that yields null. Tag [kirbyinit]."
        );
    }

    #[cfg(all(feature = "css_slot", feature = "diag_kirby_copy"))]
    {
        skyline::install_hooks!(
            generate_article_enable_probe,
            article_creator_dispatch_probe,
            article_custom_creator_probe,
            article_base_creator_probe,
            shoot_article_probe,
            shoot_exist_article_probe
        );
        if !unsafe { article_probes::install_article_removal_stubs() } {
            skyline::println!(
                "[clone_engine] WARNING could not place the article removal stubs; those two probes are inert"
            );
        }
        skyline::println!(
            "[clone_engine] installed tracked-Kirby ArticleModule operation/lifecycle probes (diag_kirby_copy)"
        );
    }

    #[cfg(feature = "diag_article_initspoof")]
    {
        skyline::install_hook!(fighter_init_object_data_hook);
        skyline::println!(
            "[clone_engine] installed fighter_initialize_object_data bracket spoof (0x6079d0)"
        );
    }

    #[cfg(feature = "true_kind")]
    {
        #[cfg(not(feature = "css_slot"))]
        {
            skyline::install_hook!(entry_block_install_hook);
            skyline::install_hook!(entry_lifecycle_hook);
        }
        skyline::install_hook!(kind_expander_hook);
        #[cfg(not(feature = "css_slot"))]
        skyline::println!(
            "[clone_engine] v33: use one final kind-0 resource registration while retaining construction kind 118 at 0x14ec24c; keep all proven init/auxiliary/boundary base bridges. Expected tags: [loadfinal] single,[initbridge],[auxkind],[boundkind]."
        );
        #[cfg(feature = "css_slot")]
        {
            install_custom_resource_name_hooks();
            install_custom_kirby_copy_hooks();
            install_smashline_name_bridge_hooks();
            install_article_animcmd_agent_hooks();
            #[cfg(feature = "diag_pocket")]
            {
                skyline::install_hook!(weapon_name_owner_tables);
                skyline::install_hooks!(effect_req_probe, effect_req_follow_probe);
            }
            skyline::install_hook!(utility_get_kind_hook);
            skyline::install_hook!(clone_fighter_status_create);
            install_custom_module_name_hook();
            skyline::install_hook!(ui_fighter_kind_lookup_hook);
            skyline::install_hook!(kind_display_ui_hook);
            skyline::install_hook!(update_selected_fighter_hook);
            skyline::install_hook!(match_entry_expand_outer_call_hook);
            skyline::install_hook!(match_entry_expand_inner_call_hook);
            skyline::install_hook!(construction_roster_expand_call_hook);
            skyline::install_hook!(fighter_change_expand_call_hook);
            skyline::install_hook!(fighter_change_request_hook);
            skyline::install_hook!(team_queue_expand_input_hook);
            skyline::install_hook!(retry_expand_input_hook);
            skyline::install_hook!(tag_team_expand_input_hook);
            skyline::install_hook!(classic_route_path_hook);
            skyline::install_hook!(classic_route_title_hook);
            unsafe { csk_row_bridge::install() };
            #[cfg(feature = "diag_load_barrier")]
            {
                skyline::install_hook!(load_pipeline::load_barrier_poll_probe);
                skyline::install_hook!(load_pipeline::match_setup_entry_probe);
                skyline::install_hook!(load_pipeline::setup_trace_1);
                skyline::install_hook!(load_pipeline::setup_trace_2);
                skyline::install_hook!(load_pipeline::setup_trace_3);
                skyline::install_hook!(load_pipeline::setup_trace_4);
                skyline::install_hook!(load_pipeline::setup_trace_5);
                skyline::install_hook!(load_pipeline::setup_trace_6);
                skyline::println!(
                    "[clone_engine] diag_load_barrier: match-setup entry probe at 0x14e58f0 [setupentry] + state-machine probe at 0x14e94d4 [loadbar]. Both read-only; 0x14e94d4 proven to fire in a working match and never in a Kirby one, so these bracket the stall."
                );
            }
            skyline::nro::add_hook(css_slot_nro_hook)
                .expect("clone_engine: libnro_hook is required for the CSK CSS slot");
            skyline::nro::add_hook(kirby_copy_family_nro_hook).expect(
                "clone_engine: libnro_hook is required for the Kirby copy dispatcher range",
            );
            if !unsafe { kirby_copy::install_status_interrupt_stub() } {
                skyline::println!(
                    "[kirbyfam] WARNING could not place the status-interrupt stub; copy status routing is inert"
                );
            }
            skyline::println!(
                "[clone_engine] kirby copy FAMILY route: ONE hook on StatusModule::set_status_kind_interrupt (0x2087740), claimed only for callers inside lua2cpp_kirby's copy dispatcher 0x236ce0..0x239efc - so every per-fighter branch routes and NO Kirby NRO code is patched. Routes an ARMED clone's copied entry to its descriptor-owned status family. Unarmed/unregistered = fully native. Reserve hook 0xAD0 intentionally absent: sv_set_status_func self-grows the status vector (decoded 2026-07-18). Tag [kirbyfam]."
            );
            skyline::println!(
                "[clone_engine] installed descriptor-driven per-thread resource-name hooks"
            );
            skyline::println!(
                "[clone_engine] installed true-kind Kirby copy bridges: native ability tables use each clone's base kind while models use copy_<resource>_fitkirby"
            );
            skyline::println!(
                "[clone_engine] kirbyreg: load-time kirbycopy dir registrar 0x17effe0 bridges custom kinds to their base (fix for the Kirby+kind119 fighter/none/kirbycopy miss APPCRASH); parent 0x17efb80 observed. Tag [kirbyreg]."
            );
            skyline::println!(
                "[clone_engine] wpn probes + resource-slot BRIDGE: per-fighter preload 0x17eeae0 + loader continuation 0x607e44/0x607e74/0x607f28/0x607f98 bracketed; slot walker 0x17f1aa0 remaps raw clone kinds to their base (fix for the Kirby+kind119 APPCRASH pinned at slot enter w2=0x77). Tag [wpn]."
            );
            skyline::println!(
                "[clone_engine] kirby copy RECORD MECHANISM v14: clone slot registration now runs Nintendo's complete creator under the true kind, with a descriptor-owned four-name record and base-owned bodymotion/sound. Native list nodes and teardown replace the old one-member approximation. Tags [kirbynative]/[kirbyrec]/[kirbycreator]."
            );
            skyline::println!(
                "[clone_engine] kirby copy OUTCOME probes: copy_ability_reset 0xb96770 entry (caller LR, catches any post-grant canceller incl. plugin opffs) + Kirby per-fighter-frame 0xb97b30 copy-state tracker (flag 0x20000102 / int 0x100000FC, change-only). Tag [copytrack]."
            );
            skyline::println!(
                "[clone_engine] effect-bank DUAL-LOAD: clone transplant (base+2000) now also loads the VANILLA base eff under the original 0x300+kind handle (vanilla index resolved context-free at type-20) so Kirby-copy and real-base consumers see real samus effects. Tag [effectbank] base-dual-load."
            );
        }
        skyline::install_hook!(kind_validity_gate_hook);
        skyline::println!(
            "[clone_engine] v15: installed kind-validity gate trace+spoof (0x65dd70): kind 118 traced REAL+SPOOF(kind->0) for the first 8 calls then silently spoofed, kind>0x75 traced up to 32 calls, everything else passthrough"
        );
        skyline::install_hook!(resmgr_insert_hook);
        skyline::println!(
            "[clone_engine] v49: installed read-only resource-cache trace at 0x17d1d70; logs caller, returned map census, and active byte for Mario/custom kinds after the native call. Tag [resmgr49]."
        );
        skyline::install_hook!(load_dispatch_kind_hook);
        skyline::println!(
            "[clone_engine] v33: load-dispatch 0x17e5c00 PASSES kind 118 through unchanged so construction can retain the true kind. Tag [loaddisp]."
        );
        skyline::install_hook!(path_builder_remap_hook);
        #[cfg(not(feature = "css_slot"))]
        skyline::println!(
            "[clone_engine] v33: path-builder 0x17df460 remaps w1 118->0 so the true-118 load resolves fighter/mario assets without changing [load_obj+0x58]. Tag [pathb]."
        );
        #[cfg(feature = "css_slot")]
        skyline::println!(
            "[clone_engine] path-builder preserves true custom kinds; thread-scoped name hooks resolve their independent resource roots. Tag [pathb]."
        );
        #[cfg(feature = "css_slot")]
        skyline::install_hook!(custom_native_register_probe);
        #[cfg(feature = "css_slot")]
        skyline::println!(
            "[clone_engine] native custom-kind module verifier uses each descriptor's base module while asset paths retain the custom resource name. Tag [regkind]."
        );
        skyline::install_hook!(load_final_register_call_hook);
        #[cfg(not(feature = "css_slot"))]
        skyline::println!(
            "[clone_engine] v33: final BL callback mutates saved w1 118->0; Skyline's trampoline executes 0x17e4940 exactly once as kind 0. Construction identity remains independently 118. Tag [loadfinal]."
        );
        #[cfg(feature = "css_slot")]
        skyline::println!(
            "[clone_engine] v46: final BL callback keeps key 118; Skyline's trampoline executes 0x17e4940 once after its module-name fallback was aliased at boot. Tag [loadfinal]."
        );
        #[cfg(feature = "css_slot")]
        skyline::install_hooks!(
            custom_group_ready_probe,
            custom_entry_ready_probe,
            custom_base_ready_probe,
        );
        #[cfg(feature = "css_slot")]
        skyline::println!(
            "[clone_engine] v46: retained custom-object-only readiness probes at group 0x17eb180, entry 0x17e4790, and base 0x17e28c0 with startup-only sampling. Observation only; tag [ready118]."
        );
        #[cfg(not(feature = "diag_article_initspoof"))]
        skyline::install_hooks!(fighter_init_kind_bridge, load_pipeline::fighter_init_member_index);
        #[cfg(feature = "css_slot")]
        fighter_param_rows::install_hooks();
        #[cfg(feature = "css_slot")]
        fighter_param_ai::install_hooks();
        fighter_common_copies::install_hooks();
        fighter_param_thrown::install_hooks();
        #[cfg(feature = "css_slot")]
        skyline::install_hooks!(fighter_scoped_resource_path_hook, fighter_camera_set_hook);
        #[cfg(feature = "css_slot")]
        skyline::install_hook!(load_pipeline::camera_animation_base_fallback);
        #[cfg(all(feature = "diag_trail", feature = "css_slot"))]
        skyline::install_hook!(load_pipeline::trail_nutexb_probe);
        #[cfg(all(feature = "diag_trail", feature = "css_slot"))]
        skyline::install_hook!(load_pipeline::trail_request_probe);
        #[cfg(all(feature = "diag_trail", feature = "css_slot"))]
        skyline::install_hook!(load_pipeline::trail_directory_probe);
        #[cfg(feature = "css_slot")]
        skyline::install_hooks!(
            load_pipeline::victory_camera_name_hook_a,
            load_pipeline::victory_camera_name_hook_b,
        );
        #[cfg(feature = "css_slot")]
        skyline::install_hook!(load_pipeline::camera_record_value_guard);
        #[cfg(feature = "css_slot")]
        skyline::install_hook!(load_pipeline::victory_camera_kind_hook);
        skyline::install_hook!(load_pipeline::post_init_work_query_probe);
        #[cfg(all(feature = "clone_runtime", feature = "css_slot"))]
        skyline::install_hook!(model_path_namespace_hook);
        skyline::println!(
            "[clone_engine] v61: corrected filename/container/texture/dictionary clone_mario UI identities and restored CSK's supported custom name_id; retain the one-costume kind-118 model bridge. Tags [uiasset61]/[modelpath54]/[css118]."
        );
    }

    #[cfg(all(feature = "diag_pathtrace", not(feature = "true_kind")))]
    {
        skyline::install_hook!(path_builder_trace_hook);
        skyline::println!(
            "[clone_engine] v15: installed canonical path-builder trace (0x17df460): kind>0x75 calls always logged, first 24 kind<=0x75 calls logged, pure observation (no spoof)"
        );
    }

    report_clone_runtime_hooks();
    skyline::println!(
        "[clone_engine] capabilities compiled={:#018x} runtime={:#018x}",
        clone_engine_compiled_capabilities_v1(),
        clone_engine_runtime_capabilities_v1(),
    );
}

fn report_clone_runtime_hooks() {
    #[cfg(feature = "clone_runtime")]
    {
        const LOAD_BEARING: &[&str] = &[
            "fighter_init_kind_bridge(0x6079d0) kind+name+kind-array bridge",
            "fighter_scoped_resource_path_hook(0x17e88d0) namespace + base fallback",
            "fighter_params(0x70c580) per-instance vl.prc payload for clone/base coexistence",
            "fighter_param_rows(0x6867e0, 0x60b6b0, per-frame 0x3a84e0 + Fighter phases 107/109/110/111-115 at 0x614630 0x6164a0 0x616580 0x619810 0x619850 0x619890 0x61a0a0 0x3a8bc0; aux record under 0x34af10) fighter_param + fighter_param_motion row swap for direct readers",
            "fighter_common_copies(accessor+0x2758 param object; binders 0x736a90 common 0x782c00 item 0x7741f0 etc 0x798540 power_up 0x761f70 effect) per-fighter copies of the six shared param files patched for clones; kind bind 0x77d100 re-patches the detached power_up object at param object+0x1b10",
            "fighter_param_ai(AI fighter update 0x34b310 for rosetta inkling brave) the AI's per-fighter pass reads a clone's own vl.prc outside the fighter's phases",
            "fighter_param_thrown(thunk 0x20a8000 + lua call 0x20a81b4 for thrown_offset 0x720f90; entry hooks 0x7210e0 donkey 0x7211a0 ridley 0x721380 gaogaen 0x721240 diddy 0x7212e0 mii 0x721430 demon_command 0x7214d0 demon_special_lw) hold offsets adjusted for clones at the accessor",
            "model_path_namespace_hook(0x17e9a00) MODEL namespace",
            "path_builder_remap_hook(0x17df460) path namespace",
            "load_dispatch_kind_hook(0x17e5c00) load-dispatch kind",
            "load_final_register_call_hook resource registration kind",
            "kind_validity_gate_hook(0x65dd70) kind validity",
            "clone_fighter_status_create(0x64bbd0) Smashline status agent name scope",
            "clone_fighter_acmd_game(0x64c310) Smashline game agent name scope",
            "clone_fighter_acmd_effect(0x64c930) Smashline effect agent name scope",
            "clone_fighter_acmd_expression(0x64cf50) Smashline expression agent name scope",
            "clone_fighter_acmd_sound(0x64d570) Smashline sound agent name scope",
        ];
        skyline::println!(
            "[clone_engine] clone_runtime: {} load-bearing hooks active (NOT diagnostics - \
             never gate these on a diag_* feature):",
            LOAD_BEARING.len()
        );
        for hook in LOAD_BEARING {
            skyline::println!("[clone_engine]   * {hook}");
        }
    }
    #[cfg(not(feature = "clone_runtime"))]
    skyline::println!(
        "[clone_engine] WARNING: built without `clone_runtime` - kind and resource-namespace \
         bridges are ABSENT; clones will wear their base fighter's assets"
    );
}
