use super::*;

#[cfg(feature = "css_slot")]
struct CssNroTrace<'a>(&'a str);

#[cfg(feature = "css_slot")]
impl Drop for CssNroTrace<'_> {
    fn drop(&mut self) {
        skyline::println!("[nrotrace] engine-first EXIT name={}", self.0);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static CSS_SLOT_REGISTRATION_STARTED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "css_slot")]
pub(crate) static CSS_SLOT_REGISTERED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "css_slot")]
pub(crate) static CSK_GET_UI_CHARA_FROM_ENTRY: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

#[cfg(feature = "css_slot")]
pub(crate) const ENTRY_SELECTION_UNOBSERVED: i32 = i32::MIN;

#[cfg(feature = "css_slot")]
pub(crate) static CSS_CUSTOM_ENTRY_KINDS: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(ENTRY_SELECTION_UNOBSERVED) }; 8];

#[cfg(feature = "css_slot")]
static CSS_REPORTED_ENTRY_UI: [core::sync::atomic::AtomicU64; 8] =
    [const { core::sync::atomic::AtomicU64::new(0) }; 8];

#[cfg(feature = "css_slot")]
static ENTRY_RECOVERY_REFUSED_LOG: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
static RECORD_CLONE_BASE: [core::sync::atomic::AtomicI32; 8] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; 8];

#[cfg(feature = "css_slot")]
static RECORD_CLONE_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) fn entry_selection_observed(entry_id: u8) -> bool {
    if entry_id >= 8 {
        return false;
    }
    CSS_CUSTOM_ENTRY_KINDS[entry_id as usize].load(core::sync::atomic::Ordering::SeqCst)
        != ENTRY_SELECTION_UNOBSERVED
}

#[cfg(feature = "css_slot")]
pub(crate) const NO_OWNER_OBJECT: u32 = u32::MAX;

#[cfg(feature = "css_slot")]
pub(crate) static ENTRY_OWNER_OBJECT_ID: [core::sync::atomic::AtomicU32; 8] =
    [const { core::sync::atomic::AtomicU32::new(NO_OWNER_OBJECT) }; 8];

#[cfg(feature = "css_slot")]
pub(crate) fn record_entry_owner_object(entry_id: i32, object_id: u32) {
    if !(0..8).contains(&entry_id) || object_id == NO_OWNER_OBJECT {
        return;
    }
    ENTRY_OWNER_OBJECT_ID[entry_id as usize]
        .store(object_id, core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "css_slot")]
pub(crate) fn entry_of_owner_object(object_id: u32) -> Option<u8> {
    if object_id == NO_OWNER_OBJECT {
        return None;
    }
    ENTRY_OWNER_OBJECT_ID
        .iter()
        .position(|slot| slot.load(core::sync::atomic::Ordering::SeqCst) == object_id)
        .map(|entry| entry as u8)
}

#[cfg(feature = "css_slot")]
pub(crate) fn owner_object_table_is_populated() -> bool {
    ENTRY_OWNER_OBJECT_ID
        .iter()
        .any(|slot| slot.load(core::sync::atomic::Ordering::SeqCst) != NO_OWNER_OBJECT)
}

#[cfg(feature = "css_slot")]
pub(crate) fn cache_custom_entry_selection(entry_id: i32, kind: Option<i32>) {
    if !(0..8).contains(&entry_id) {
        return;
    }
    CSS_CUSTOM_ENTRY_KINDS[entry_id as usize]
        .store(kind.unwrap_or(-1), core::sync::atomic::Ordering::SeqCst);
    if let Some(kind) = kind {
        crate::staffroll::mark_used(kind);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) fn custom_entry_mask() -> u32 {
    let mut mask = 0u32;
    for (entry, kind) in CSS_CUSTOM_ENTRY_KINDS.iter().enumerate() {
        if kind.load(core::sync::atomic::Ordering::SeqCst) >= FIRST_CUSTOM_KIND {
            mask |= 1 << entry;
        }
    }
    mask
}

#[cfg(feature = "css_slot")]
pub(crate) fn entry_custom_kind(entry_id: u8) -> Option<i32> {
    if entry_id >= 8 {
        return None;
    }
    let kind = CSS_CUSTOM_ENTRY_KINDS[entry_id as usize].load(core::sync::atomic::Ordering::SeqCst);
    (kind >= FIRST_CUSTOM_KIND).then_some(kind)
}

#[cfg(feature = "css_slot")]
pub(crate) fn csk_entry_custom_kind(entry_id: u8) -> Option<i32> {
    type GetUiCharaFn = unsafe extern "C" fn(u32) -> u64;
    let address = CSK_GET_UI_CHARA_FROM_ENTRY.load(core::sync::atomic::Ordering::SeqCst);
    if address == 0 || entry_id >= 8 {
        return None;
    }
    let get_ui: GetUiCharaFn = unsafe { core::mem::transmute(address) };
    let ui_chara = unsafe { get_ui(entry_id as u32) } & 0x00ff_ffff_ffff;
    if ui_chara == 0 {
        return None;
    }
    clone_definition_from_ui(ui_chara).map(|definition| definition.kind)
}

#[cfg(feature = "css_slot")]
pub(crate) const ENTRY_RECORD_UI_CHARA: usize = 0x00;

#[cfg(feature = "css_slot")]
pub(crate) const ENTRY_RECORD_KIND: usize = 0x08;

#[cfg(feature = "css_slot")]
pub(crate) const ENTRY_RECORD_COSTUME: usize = 0x19;

#[cfg(feature = "css_slot")]
unsafe fn record_ui_chara(source: *const u8) -> u64 {
    let mut ui_chara = 0u64;
    for index in 0..5 {
        let byte = core::ptr::read_volatile(source.add(ENTRY_RECORD_UI_CHARA + index));
        ui_chara |= u64::from(byte) << (index * 8);
    }
    ui_chara
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn record_clone_definition(
    source: *const u8,
) -> Option<&'static CloneDefinition> {
    if source.is_null() {
        return None;
    }
    clone_definition_from_ui(record_ui_chara(source))
}

#[cfg(feature = "css_slot")]
fn costume_in_css_colours(definition: &CloneDefinition, costume: u8) -> bool {
    costume >= definition.color_start
        && u16::from(costume)
            < u16::from(definition.color_start) + u16::from(definition.css_color_count())
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn record_clone_kind(source: *const u8) -> Option<i32> {
    let definition = record_clone_definition(source)?;
    let costume = core::ptr::read_volatile(source.add(ENTRY_RECORD_COSTUME));
    costume_in_css_colours(definition, costume).then_some(definition.kind)
}

#[cfg(feature = "css_slot")]
unsafe fn follow_record(entry_id: u8, source: *const u8) {
    let Some(reported) = CSS_REPORTED_ENTRY_UI.get(entry_id as usize) else {
        return;
    };
    reported.store(record_ui_chara(source), core::sync::atomic::Ordering::SeqCst);
    cache_custom_entry_selection(entry_id as i32, record_clone_kind(source));
}

#[cfg(feature = "css_slot")]
unsafe fn rewritten_record_kind(
    entry_id: u8,
    source: *const u8,
    site: &'static str,
) -> Option<Option<i32>> {
    let reported = CSS_REPORTED_ENTRY_UI.get(entry_id as usize)?;
    let reported_ui = reported.load(core::sync::atomic::Ordering::SeqCst);
    let record_ui = record_ui_chara(source);
    if reported_ui == 0 || record_ui == 0 || record_ui == reported_ui {
        return None;
    }
    let kind = record_clone_kind(source);
    reported.store(record_ui, core::sync::atomic::Ordering::SeqCst);
    cache_custom_entry_selection(entry_id as i32, kind);
    dbg_log!(
        "[csscache] entry={entry_id} record ui {record_ui:#x} was written after the selection writer reported {reported_ui:#x}; the record decides kind {kind:?} site={site}"
    );
    Some(kind)
}

#[cfg(feature = "css_slot")]
unsafe fn record_clone_over_base(entry_id: u8, source: *const u8, input_kind: i32, site: &'static str) -> Option<i32> {
    let slot = RECORD_CLONE_BASE.get(entry_id as usize)?;
    let found = record_clone_definition(source).filter(|definition| {
        input_kind == definition.base_kind
            && costume_in_css_colours(definition, core::ptr::read_volatile(source.add(ENTRY_RECORD_COSTUME)))
    });
    let Some(definition) = found else {
        slot.store(-1, core::sync::atomic::Ordering::SeqCst);
        return None;
    };
    slot.store(input_kind, core::sync::atomic::Ordering::SeqCst);
    cache_custom_entry_selection(entry_id as i32, Some(definition.kind));
    if RECORD_CLONE_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
        crate::dbg_log_public(&format!(
            "[csscache] entry={entry_id} record names clone kind {} and the game resolved its base {input_kind} (no CSS pick, as in a replay); kind {} site={site}",
            definition.kind, definition.kind
        ));
    }
    Some(definition.kind)
}

#[cfg(feature = "css_slot")]
pub(crate) fn validated_entry_custom_kind(
    entry_id: u8,
    input_kind: i32,
    record: Option<*const u8>,
    site: &'static str,
) -> Option<i32> {
    if let Some(kind) =
        record.and_then(|source| unsafe { rewritten_record_kind(entry_id, source, site) })
    {
        return kind;
    }
    if let Some(kind) =
        record.and_then(|source| unsafe { record_clone_over_base(entry_id, source, input_kind, site) })
    {
        return Some(kind);
    }
    let Some(custom_kind) = entry_custom_kind(entry_id) else {
        if input_kind != 0 {
            return None;
        }
        if record.is_some() && entry_selection_observed(entry_id) {
            let n = ENTRY_RECOVERY_REFUSED_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            if n < 8 {
                dbg_log!(
                    "[csscache] entry={entry_id} REFUSED recovery site={site}: the CSS selection writer already reported this entry and it was not a clone, so its record holds a stale or vanilla selection"
                );
            }
            return None;
        }
        if let Some(source) = record {
            if let Some(definition) = unsafe { record_clone_definition(source) } {
                let costume = unsafe { core::ptr::read_volatile(source.add(ENTRY_RECORD_COSTUME)) };
                let in_range = costume_in_css_colours(definition, costume);
                if !in_range {
                    let n =
                        ENTRY_RECOVERY_REFUSED_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                    if n < 8 {
                        dbg_log!(
                            "[csscache] entry={entry_id} REFUSED recovery site={site}: ui_chara matches kind {} but costume {costume} is outside its CSS colours {}..{}, so the record holds the vanilla donor",
                            definition.kind,
                            definition.color_start,
                            u16::from(definition.color_start)
                                + u16::from(definition.css_color_count())
                        );
                    }
                    return None;
                }
                cache_custom_entry_selection(entry_id as i32, Some(definition.kind));
                dbg_log!(
                    "[csscache] recovered entry={entry_id} kind={} from record ui_chara costume={costume} in_range={in_range} site={site}",
                    definition.kind
                );
                return Some(definition.kind);
            }
        }
        let recovered = csk_entry_custom_kind(entry_id)?;
        cache_custom_entry_selection(entry_id as i32, Some(recovered));
        dbg_log!(
            "[csscache] recovered entry={entry_id} kind={recovered} from CSK entry->ui map site={site}"
        );
        return Some(recovered);
    };
    if input_kind == 0 || input_kind == custom_kind {
        return Some(custom_kind);
    }
    if record.is_none()
        && RECORD_CLONE_BASE
            .get(entry_id as usize)
            .is_some_and(|base| base.load(core::sync::atomic::Ordering::SeqCst) == input_kind)
    {
        return Some(custom_kind);
    }

    cache_custom_entry_selection(entry_id as i32, None);
    dbg_log!(
        "[csscache] stale entry={entry_id} cached_kind={custom_kind} current_kind={input_kind} site={site}; cleared"
    );
    None
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn lookup_symbol(name: &[u8]) -> Option<usize> {
    let mut address = 0usize;
    let result = LookupSymbol(&mut address as *mut usize, name.as_ptr());
    (result == 0 && address != 0).then_some(address)
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn register_csk_css_slot() {
    type AddEntryFn = unsafe extern "C" fn(*const core::ffi::c_void);

    let Some(add_chara_addr) = lookup_symbol(b"add_chara_db_entry_info\0") else {
        skyline::println!("[css118] CSK add_chara_db_entry_info is not resolvable");
        return;
    };
    let Some(add_layout_addr) = lookup_symbol(b"add_chara_layout_db_entry_info\0") else {
        skyline::println!("[css118] CSK add_chara_layout_db_entry_info is not resolvable");
        return;
    };

    let add_chara: AddEntryFn = core::mem::transmute(add_chara_addr);
    let add_layout: AddEntryFn = core::mem::transmute(add_layout_addr);

    let published = clone_definitions()
        .read()
        .unwrap()
        .iter()
        .copied()
        .filter(|definition| definition.css.is_some() && clone_base(definition.kind).is_some())
        .collect::<Vec<_>>();

    for definition in published {
        let css = definition.css.expect("filtered on css.is_some()");
        publish_css_slot_once(definition, css, add_chara, add_layout);
    }

    if let Some(get_ui_addr) = lookup_symbol(b"get_ui_chara_from_entry_id\0") {
        CSK_GET_UI_CHARA_FROM_ENTRY.store(get_ui_addr, core::sync::atomic::Ordering::SeqCst);
    } else {
        skyline::println!("[css118] WARNING: get_ui_chara_from_entry_id is not resolvable");
    }

    CSS_SLOT_REGISTERED.store(true, core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "css_slot")]
static CSS_PUBLISHED_KINDS: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new());

#[cfg(feature = "css_slot")]
unsafe fn publish_css_slot_once(
    definition: &'static CloneDefinition,
    css: &'static CloneCssEntry,
    add_chara: unsafe extern "C" fn(*const core::ffi::c_void),
    add_layout: unsafe extern "C" fn(*const core::ffi::c_void),
) -> bool {
    let Ok(mut published) = CSS_PUBLISHED_KINDS.lock() else {
        return false;
    };
    if published.contains(&definition.kind) {
        return false;
    }
    published.push(definition.kind);
    register_one_csk_css_slot(definition, css, add_chara, add_layout);
    true
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn publish_late_css_slot(kind: i32) {
    if !CSS_SLOT_REGISTRATION_STARTED.load(core::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let Some(definition) = clone_definition(kind) else {
        return;
    };
    let Some(css) = definition.css else {
        return;
    };
    if clone_base(kind).is_none() {
        return;
    }
    let (Some(add_chara_addr), Some(add_layout_addr)) = (
        lookup_symbol(b"add_chara_db_entry_info\0"),
        lookup_symbol(b"add_chara_layout_db_entry_info\0"),
    ) else {
        skyline::println!("[css118] {} registered after CSK's first pass but CSK is not resolvable; no CSS row", definition.resource_name);
        return;
    };
    type AddEntryFn = unsafe extern "C" fn(*const core::ffi::c_void);
    let add_chara: AddEntryFn = core::mem::transmute(add_chara_addr);
    let add_layout: AddEntryFn = core::mem::transmute(add_layout_addr);
    if publish_css_slot_once(definition, css, add_chara, add_layout) {
        skyline::println!("[css118] {} registered after CSK's first pass; its CSS row was published on registration", definition.resource_name);
    }
}

#[cfg(feature = "css_slot")]
const UI_TAG: u64 = 0xc1 << 56;

#[cfg(feature = "css_slot")]
const UI_HASH_MASK: u64 = 0x00ff_ffff_ffff;

#[cfg(feature = "css_slot")]
const UI_DATABASE_HOLDER: usize = 0x532_e730;

#[cfg(feature = "css_slot")]
const UI_DATABASE_TABLE: usize = 0x168;

#[cfg(feature = "css_slot")]
const OFF_UI_TAG_FROM_HASH: usize = 0x326_2bf0;

#[cfg(feature = "css_slot")]
const OFF_UI_CHARA_NAME: usize = 0x326_1d60;

#[cfg(feature = "css_slot")]
static CLASSIC_ROUTE_LOGGED: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());

#[cfg(feature = "css_slot")]
static CLASSIC_TITLE_LOGGED: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());

#[cfg(feature = "css_slot")]
unsafe fn classic_route_ui(ui: u64) -> u64 {
    if ui & !(UI_HASH_MASK | 0x00ff_ff00_0000_0000) != UI_TAG {
        return ui;
    }
    let Some(definition) = clone_definition_from_ui(ui & UI_HASH_MASK) else {
        return ui;
    };
    let Some(css) = definition.css else {
        return ui;
    };
    if crate::fighter_modules::path_exists(&format!("ui/param/standard/standard_route_{}.prc", css.ui_name)) {
        return ui;
    }
    let holder = core::ptr::read_volatile(crate::game::addr(UI_DATABASE_HOLDER) as *const usize);
    if holder == 0 {
        return ui;
    }
    let database = core::ptr::read_volatile((holder + 8) as *const usize);
    if database == 0 {
        return ui;
    }
    let table = core::ptr::read_volatile((database + UI_DATABASE_TABLE) as *const u64);
    if table == 0 {
        return ui;
    }
    let base_name = selectable_row(definition.base_resource_name);
    let tag: unsafe extern "C" fn(u64, u64) -> u64 = core::mem::transmute(crate::game::addr(OFF_UI_TAG_FROM_HASH));
    let base = tag(table, hash40(&format!("ui_chara_{base_name}")));
    if base & UI_HASH_MASK == 0 {
        return ui;
    }
    if let Ok(mut logged) = CLASSIC_ROUTE_LOGGED.lock() {
        if !logged.contains(&(ui & UI_HASH_MASK)) {
            logged.push(ui & UI_HASH_MASK);
            dbg_log!(
                "[classic] {} ships no standard_route_{}.prc, so Classic Mode runs {base_name}'s route",
                definition.ui_chara,
                css.ui_name
            );
        }
    }
    base
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_CLASSIC_ROUTE_PATH))]
pub(crate) unsafe fn classic_route_path_hook(ui: u64, a1: u64, a2: u64, a3: u64) -> u64 {
    call_original!(classic_route_ui(ui), a1, a2, a3)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_CLASSIC_ROUTE_TITLE), inline)]
pub(crate) unsafe fn classic_route_title_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let ui = ctx.registers[19].x();
    let route = classic_route_ui(ui);
    let table = ctx.registers[20].x();
    let name = ctx.registers[23].x();
    if route == ui || table == 0 || name == 0 {
        return;
    }
    let ui_chara_name: unsafe extern "C" fn(u64, u64, u64) = core::mem::transmute(crate::game::addr(OFF_UI_CHARA_NAME));
    ui_chara_name(name, table, route);
    if let Ok(mut logged) = CLASSIC_TITLE_LOGGED.lock() {
        if !logged.contains(&(ui & UI_HASH_MASK)) {
            logged.push(ui & UI_HASH_MASK);
            let text = core::ffi::CStr::from_ptr((name as usize + 8) as *const core::ffi::c_char).to_string_lossy();
            let clone = clone_definition_from_ui(ui & UI_HASH_MASK).map_or("a clone", |definition| definition.ui_chara);
            dbg_log!("[classic] {clone}'s route preview shows mnu_standard_sub_{text}, the title of the route it runs");
        }
    }
}

#[cfg(feature = "css_slot")]
fn selectable_row(base_resource_name: &str) -> &str {
    match base_resource_name {
        "popo" | "nana" => "ice_climber",
        "eflame" => "flame_first",
        "elight" => "light_first",
        "pzenigame" | "pfushigisou" | "plizardon" => "ptrainer",
        other => other,
    }
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn register_one_csk_css_slot(
    definition: &'static CloneDefinition,
    css: &'static CloneCssEntry,
    add_chara: unsafe extern "C" fn(*const core::ffi::c_void),
    add_layout: unsafe extern "C" fn(*const core::ffi::c_void),
) {
    use the_csk_collection_api::{
        BoolType, CStrCSK, CharacterDatabaseEntry, CharacterLayoutDatabaseEntry, Hash40Map,
        Hash40Type, IntType, ShortType, SignedByteType, StringType, UnsignedByteMap,
        UnsignedByteType,
    };

    type AllowOnlineFn = unsafe extern "C" fn(u64);

    let ui_chara = hash40(definition.ui_chara);
    let custom_kind_hash = hash40(definition.fighter_kind_name);
    let base_ui = hash40(&format!("ui_chara_{}", definition.base_resource_name));
    let selectable_base_ui = hash40(&format!("ui_chara_{}", selectable_row(definition.base_resource_name)));
    let base_characall = match css.narration {
        Some(narration) => hash40(narration),
        None => hash40(&format!(
            "vc_narration_characall_{}",
            definition.base_resource_name
        )),
    };
    if let Some(narration) = css.narration {
        type AddNarrationFn = unsafe extern "C" fn(*mut i8) -> bool;
        if let Some(add_narration_addr) = lookup_symbol(b"add_narration_characall_entry\0") {
            let add_narration: AddNarrationFn = core::mem::transmute(add_narration_addr);
            let mut owned = narration.as_bytes().to_vec();
            owned.push(0);
            let added = add_narration(owned.as_mut_ptr().cast::<i8>());
            skyline::println!(
                "[css118] narration {narration} for {}: {}",
                definition.resource_name,
                if added { "added" } else { "refused by CSK" }
            );
        }
    }

    let mut ui_fallbacks = HashMap::new();
    let mut ui_indices = HashMap::new();

    let color_count = definition.css_color_count();
    for color in definition.color_start..definition.color_start + color_count {
        ui_fallbacks.insert(
            hash40(&format!("characall_label_c{color:02}")),
            Hash40Type::Overwrite(base_characall),
        );
        ui_fallbacks.insert(
            hash40(&format!("characall_label_article_c{color:02}")),
            Hash40Type::Overwrite(0),
        );
        ui_indices.insert(
            hash40(&format!("c{color:02}_index")),
            UnsignedByteType::Overwrite(0),
        );
        ui_indices.insert(
            hash40(&format!("n{color:02}_index")),
            UnsignedByteType::Overwrite(0),
        );
        ui_indices.insert(
            hash40(&format!("c{color:02}_group")),
            UnsignedByteType::Overwrite(0),
        );
    }
    ui_indices.insert(
        hash40("color_start_index"),
        UnsignedByteType::Overwrite(definition.color_start),
    );
    ui_fallbacks.insert(
        hash40("original_ui_chara_hash"),
        Hash40Type::Overwrite(base_ui),
    );

    let entry = CharacterDatabaseEntry {
        ui_chara_id: ui_chara,
        clone_from_ui_chara_id: Some(selectable_base_ui),
        name_id: StringType::Overwrite(CStrCSK::new(css.ui_name)),
        fighter_kind: Hash40Type::Overwrite(custom_kind_hash),
        fighter_kind_corps: Hash40Type::Overwrite(custom_kind_hash),
        ui_series_id: Hash40Type::Overwrite(hash40(css.ui_series)),
        fighter_type: Hash40Type::Overwrite(hash40("fighter_type_normal")),
        alt_chara_id: Hash40Type::Overwrite(0x2302_d482a),
        exhibit_year: ShortType::Overwrite(css.exhibit_year),
        ext_skill_page_num: SignedByteType::Overwrite(0),
        is_img_ext_skill_page0: BoolType::Overwrite(false),
        is_img_ext_skill_page1: BoolType::Overwrite(false),
        is_img_ext_skill_page2: BoolType::Overwrite(false),
        disp_order: SignedByteType::Overwrite(
            crate::chara_orders::player_disp_order(ui_chara).unwrap_or(css.disp_order),
        ),
        save_no: SignedByteType::Overwrite(css.save_no),
        skill_list_order: SignedByteType::Overwrite(css.disp_order),
        chara_count: SignedByteType::Overwrite(1),
        can_select: BoolType::Overwrite(true),
        is_usable_soundtest: BoolType::Overwrite(true),
        is_called_pokemon: BoolType::Overwrite(false),
        is_mii: BoolType::Overwrite(false),
        is_boss: BoolType::Overwrite(false),
        is_hidden_boss: BoolType::Overwrite(false),
        is_dlc: BoolType::Overwrite(false),
        is_patch: BoolType::Overwrite(
            crate::ui_pictures::chara_patch_flag(css.ui_name, Some(false), Some(selectable_base_ui))
                .unwrap_or(false),
        ),
        is_plural_message: BoolType::Overwrite(false),
        is_plural_narration: BoolType::Overwrite(false),
        is_article: BoolType::Overwrite(false),
        extra_flags: IntType::Overwrite(0),
        has_multiple_face: BoolType::Overwrite(false),
        result_pf0: BoolType::Overwrite(true),
        result_pf1: BoolType::Overwrite(true),
        result_pf2: BoolType::Overwrite(true),
        color_num: UnsignedByteType::Overwrite(color_count),
        extra_index_maps: UnsignedByteMap::Overwrite(ui_indices),
        extra_hash_maps: Hash40Map::Overwrite(ui_fallbacks),
        shop_item_tag: Hash40Type::Overwrite(0x2302_d482a),
        ..Default::default()
    };
    add_chara(&entry as *const _ as *const core::ffi::c_void);

    for color in definition.color_start..definition.color_start + color_count {
        let custom_layout = format!("{}_{color:02}", definition.ui_chara);
        let base_layout = format!(
            "ui_chara_{}_{:02}",
            definition.base_resource_name,
            color % VANILLA_COSTUME_SLOTS
        );
        let layout = CharacterLayoutDatabaseEntry {
            ui_layout_id: hash40(&custom_layout),
            clone_from_ui_layout_id: Some(hash40(&base_layout)),
            ui_chara_id: Hash40Type::Overwrite(ui_chara),
            chara_color: UnsignedByteType::Overwrite(color),
            ..Default::default()
        };
        add_layout(&layout as *const _ as *const core::ffi::c_void);
    }

    if let Some(allow_addr) = lookup_symbol(b"allow_ui_chara_hash_online\0") {
        let allow_online: AllowOnlineFn = core::mem::transmute(allow_addr);
        allow_online(ui_chara);
    }

    skyline::println!(
        "[css118] registered {} with CSK name_id={}, colors={}..{} (color_start_index {}) -> {} -> kind {}; fighter/{}/",
        definition.ui_chara,
        css.ui_name,
        definition.color_start,
        definition.color_start + definition.color_count - 1,
        definition.color_start,
        definition.fighter_kind_name,
        definition.kind,
        definition.resource_name
    );
}

#[cfg(feature = "css_slot")]
pub(crate) fn css_slot_nro_hook(info: &skyline::nro::NroInfo) {
    let _trace = if info.name == "samus" || info.name == "kirby" {
        skyline::println!("[nrotrace] engine-first ENTER name={}", info.name);
        Some(CssNroTrace(info.name))
    } else {
        None
    };

    if !POCKET_WATCH_INSTALLED.load(core::sync::atomic::Ordering::SeqCst)
        && unsafe { lookup_symbol(b"smashline_install_line_callback\0") }.is_some()
        && !POCKET_WATCH_INSTALLED.swap(true, core::sync::atomic::Ordering::SeqCst)
    {
        unsafe { install_pocket_watch() };
    }

    if CSS_SLOT_REGISTERED.load(core::sync::atomic::Ordering::SeqCst)
        || CSS_SLOT_REGISTRATION_STARTED.load(core::sync::atomic::Ordering::SeqCst)
    {
        return;
    }

    let symbols_ready = unsafe {
        lookup_symbol(b"add_chara_db_entry_info\0").is_some()
            && lookup_symbol(b"add_chara_layout_db_entry_info\0").is_some()
    };
    if !symbols_ready
        || CSS_SLOT_REGISTRATION_STARTED.swap(true, core::sync::atomic::Ordering::SeqCst)
    {
        return;
    }

    skyline::println!(
        "[css118] CSK API became resolvable on post-CSK NRO event '{}'; registering now",
        info.name
    );
    unsafe { register_csk_css_slot() };
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_CSS_ICON_COLUMN_BOUND), inline)]
pub(crate) unsafe fn css_icon_column_bound_hook(ctx: &mut skyline::hooks::InlineCtx) {
    const LAST_NATIVE_COLUMN: i32 = 12;

    let column = ctx.registers[13].x() as u32 as i32;
    if column > LAST_NATIVE_COLUMN {
        ctx.registers[13].set_x(LAST_NATIVE_COLUMN as u64);
        static CLAMP_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = CLAMP_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 8 {
            dbg_log!(
                "[cssgrid] icon column {column} uses native column {LAST_NATIVE_COLUMN} animation delay"
            );
        }
    }
}

#[cfg(feature = "css_slot")]
const FIGHTER_CHANGE_LOOKUP_RETURNS: [usize; 3] = [0x330_57d4, 0x330_5888, 0x249_331c];

#[cfg(feature = "css_slot")]
const UI_FROM_KIND_SCAN_RETURN: usize = 0x326_398c;

#[cfg(feature = "css_slot")]
static UI_FROM_KIND_SCAN_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
const FIGHTER_CHANGE_PLAYER_UPDATE_RETURNS: [usize; 3] = [0x330_5500, 0x330_58d0, 0x330_5a44];

#[cfg(feature = "css_slot")]
const PLAYER_INFO_IN_MATCH_SETTINGS: usize = 0x5b0;

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_UI_FIGHTER_KIND_LOOKUP))]
pub(crate) unsafe fn ui_fighter_kind_lookup_hook(database: *mut u8, encoded_hash: u64) -> i32 {
    let lr: usize;
    core::arch::asm!("mov {}, x30", out(reg) lr, options(nomem, nostack, preserves_flags));
    close_registration("CSS fighter-kind lookup");
    let kind = call_original!(database, encoded_hash);
    if kind != -1 {
        return kind;
    }
    let site = crate::game::offset_of(lr);
    let fighter_change = FIGHTER_CHANGE_LOOKUP_RETURNS.contains(&site);
    if !fighter_change && site != UI_FROM_KIND_SCAN_RETURN {
        return kind;
    }
    let Some(definition) = clone_definition_from_ui(encoded_hash & 0x00ff_ffff_ffff) else {
        return kind;
    };
    if fighter_change {
        dbg_log!(
            "[csschange] fighter change keeps {} selected as kind {} (lookup return @{:#x})",
            definition.ui_chara,
            definition.kind,
            site
        );
    } else if UI_FROM_KIND_SCAN_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
        dbg_log!(
            "[cssui] the kind->ui row scan reads {} as kind {}, so a clone's kind finds its own row (HUD after a team swap, results)",
            definition.ui_chara,
            definition.kind
        );
    }
    definition.kind
}

#[cfg(feature = "css_slot")]
static KIND_DISPLAY_UI_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_KIND_DISPLAY_UI))]
pub(crate) unsafe fn kind_display_ui_hook(kind: i32) -> u64 {
    if !(-128..=-2).contains(&kind) {
        return call_original!(kind);
    }
    let unwrapped = kind + 256;
    let Some(definition) = clone_definition(unwrapped) else {
        return call_original!(kind);
    };
    if KIND_DISPLAY_UI_LOG.fetch_add(1, core::sync::atomic::Ordering::Relaxed) < 8 {
        dbg_log!(
            "[cssui] HUD asked for kind {kind} read from an i8 player kind; answering for {} as kind {unwrapped}",
            definition.ui_chara
        );
    }
    call_original!(unwrapped)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_UPDATE_SELECTED_FIGHTER))]
pub(crate) unsafe fn update_selected_fighter_hook(
    records: *mut u8,
    entry_id: i32,
    selection: *mut u8,
) {
    close_registration("CSS selection writer");
    let selection_ui = if selection.is_null() {
        0
    } else {
        core::ptr::read_volatile(selection.add(0x18) as *const u64) & 0x00ff_ffff_ffff
    };
    let selection_definition = clone_definition_from_ui(selection_ui);
    if !selection.is_null() {
        cache_custom_entry_selection(
            entry_id,
            selection_definition.map(|definition| definition.kind),
        );
        if (0..8).contains(&entry_id) {
            CSS_REPORTED_ENTRY_UI[entry_id as usize]
                .store(selection_ui, core::sync::atomic::Ordering::SeqCst);
        }
    }
    let selected_mask = custom_entry_mask();

    if let Some(definition) = selection_definition {
        with_resource_context(definition.kind, || {
            call_original!(records, entry_id, selection)
        });
    } else {
        call_original!(records, entry_id, selection);
    }

    if records.is_null() || !(0..=5).contains(&entry_id) {
        return;
    }

    let record = records.add(entry_id as usize * 0x20);
    let kind_ptr = record as *mut i32;
    let native_kind = core::ptr::read_volatile(kind_ptr);

    if let Some(definition) = selection_definition {
        core::ptr::write_volatile(kind_ptr, definition.kind);
        dbg_log!(
            "[cssselect] entry={entry_id} selection_ui={selection_ui:#x} mask={selected_mask:#04x} kind {native_kind}->{} record={:#x}",
            definition.kind,
            record as usize
        );
    } else {
        static TRACE_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
        let n = TRACE_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        if n < 24 {
            dbg_log!(
                "[cssselect] #{n} entry={entry_id} selection_ui={selection_ui:#x} mask={selected_mask:#04x} native_kind={native_kind}"
            );
        }
    }
}

#[cfg(feature = "css_slot")]
unsafe fn bridged_entry_kind(
    source: *const u8,
    input_kind: i32,
    site: &'static str,
    counter: &core::sync::atomic::AtomicU32,
) -> Option<i32> {
    if source.is_null() {
        return None;
    }
    let entry_id = core::ptr::read_volatile(source.add(0xa7));
    if entry_id >= 8 {
        return None;
    }

    let selected_kind = validated_entry_custom_kind(entry_id, input_kind, Some(source), site);
    let selected_mask = custom_entry_mask();
    let n = counter.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    if let Some(custom_kind) = selected_kind {
        dbg_log!(
            "[cssentry] {site} #{n} entry={entry_id} mask={selected_mask:#04x} expander kind {input_kind}->{custom_kind} source={:#x}",
            source as usize
        );
    } else if n < 24 {
        dbg_log!(
            "[cssentry] {site} #{n} entry={entry_id} mask={selected_mask:#04x} expander kind={input_kind} passthrough"
        );
    }
    selected_kind
}

#[cfg(feature = "css_slot")]
pub(crate) unsafe fn bridge_custom_entry_expander_input(
    ctx: &mut skyline::hooks::InlineCtx,
    site: &'static str,
    counter: &core::sync::atomic::AtomicU32,
) {
    let source = ctx.registers[20].x() as *const u8;
    let input_kind = ctx.registers[0].x() as i32;
    if let Some(custom_kind) = bridged_entry_kind(source, input_kind, site, counter) {
        ctx.registers[0].set_x(custom_kind as u64);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static CSS_ENTRY_OUTER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static CSS_ENTRY_INNER_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static CSS_ROSTER_ENTRY_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
pub(crate) static CSS_VANILLA_ENTRY_KINDS: [core::sync::atomic::AtomicI32; 8] = [
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
pub(crate) fn vanilla_kind_in_match(kind: i32) -> bool {
    CSS_VANILLA_ENTRY_KINDS
        .iter()
        .any(|slot| slot.load(core::sync::atomic::Ordering::SeqCst) == kind)
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_MATCH_ENTRY_EXPAND_OUTER_CALL), inline)]
pub(crate) unsafe fn match_entry_expand_outer_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    bridge_custom_entry_expander_input(ctx, "outer@66dd14", &CSS_ENTRY_OUTER_COUNT);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_MATCH_ENTRY_EXPAND_INNER_CALL), inline)]
pub(crate) unsafe fn match_entry_expand_inner_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    bridge_custom_entry_expander_input(ctx, "inner@66db84", &CSS_ENTRY_INNER_COUNT);
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_CONSTRUCTION_ROSTER_EXPAND_CALL), inline)]
pub(crate) unsafe fn construction_roster_expand_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let entry_id = ctx.registers[21].x() as u8;
    if entry_id >= 8 {
        return;
    }

    let input_kind = ctx.registers[0].x() as i32;
    let selected_kind = validated_entry_custom_kind(entry_id, input_kind, None, "roster@14ec248");
    let selected_mask = custom_entry_mask();
    CSS_VANILLA_ENTRY_KINDS[entry_id as usize].store(
        if selected_kind.is_some() { -1 } else { input_kind },
        core::sync::atomic::Ordering::SeqCst,
    );
    crate::finalsmash_residency::refresh_suppression();
    let n = CSS_ROSTER_ENTRY_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if let Some(roster_kind) = selected_kind.and_then(crate::entry_groups::roster_kind) {
        ctx.registers[23].set_x(roster_kind);
    }
    if let Some(custom_kind) = selected_kind {
        ctx.registers[0].set_x(custom_kind as u64);
        dbg_log!(
            "[cssroster] #{n} entry={entry_id} mask={selected_mask:#04x} expander kind {input_kind}->{custom_kind}"
        );
    } else if n < 24 {
        dbg_log!(
            "[cssroster] #{n} entry={entry_id} mask={selected_mask:#04x} expander kind={input_kind} passthrough"
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static CSS_CHANGE_EXPAND_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);
#[cfg(feature = "css_slot")]
pub(crate) static CSS_CHANGE_REQUEST_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_FIGHTER_CHANGE_EXPAND_CALL), inline)]
pub(crate) unsafe fn fighter_change_expand_call_hook(ctx: &mut skyline::hooks::InlineCtx) {
    if let Ok(entry) = u8::try_from(ctx.registers[22].x()) {
        crate::entry_groups::unswap(entry, "as the fighter changes");
    }
    let settings = ctx.registers[25].x() as *const u8;
    let input_kind = ctx.registers[0].x() as i32;
    if settings.is_null() || input_kind < 0 {
        return;
    }
    let source = settings.add(PLAYER_INFO_IN_MATCH_SETTINGS);
    if let Some(custom_kind) =
        bridged_entry_kind(source, input_kind, "change@32ca150", &CSS_CHANGE_EXPAND_COUNT)
    {
        ctx.registers[0].set_x(custom_kind as u64);
        if let Some(roster_kind) = crate::entry_groups::roster_kind(custom_kind) {
            ctx.registers[21].set_x(roster_kind);
        }
    }
}

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_FIGHTER_CHANGE_REQUEST_CALL), inline)]
pub(crate) unsafe fn fighter_change_request_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let input_kind = ctx.registers[2].x() as i32;
    let frame = ctx.registers[29].x() as *const usize;
    if input_kind < 0 || frame.is_null() {
        return;
    }
    let caller = crate::game::offset_of(core::ptr::read_volatile(frame.add(1)));
    if !FIGHTER_CHANGE_PLAYER_UPDATE_RETURNS.contains(&caller) {
        return;
    }
    let source = ctx.registers[19].x() as *const u8;
    if let Some(custom_kind) =
        bridged_entry_kind(source, input_kind, "change@178aae0", &CSS_CHANGE_REQUEST_COUNT)
    {
        ctx.registers[2].set_x(custom_kind as u64);
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static TEAM_QUEUE_EXPAND_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_TEAM_QUEUE_EXPAND_INPUT), inline)]
pub(crate) unsafe fn team_queue_expand_input_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let entry = u8::try_from(ctx.registers[26].x()).ok().filter(|entry| *entry < 8);
    if let Some(entry) = entry {
        crate::entry_groups::unswap(entry, "as the next team member comes in");
    }
    let settings = ctx.registers[25].x() as *const u8;
    if settings.is_null() {
        return;
    }
    let source = settings.add(PLAYER_INFO_IN_MATCH_SETTINGS);
    if let Some(entry) = entry {
        follow_record(entry, source);
    }
    let Some(custom_kind) = record_clone_kind(source) else {
        return;
    };
    let input_kind = ctx.registers[0].x() as i32;
    ctx.registers[0].set_x(custom_kind as u64);
    if let Some(roster_kind) = crate::entry_groups::roster_kind(custom_kind) {
        ctx.registers[22].set_x(roster_kind);
    }
    let n = TEAM_QUEUE_EXPAND_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!(
            "[cssentry] queue@249376c #{n} next team member kind {input_kind}->{custom_kind} source={:#x}",
            source as usize
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static TAG_TEAM_EXPAND_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_TAG_TEAM_EXPAND_INPUT), inline)]
pub(crate) unsafe fn tag_team_expand_input_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let entry = u8::try_from(ctx.registers[28].x()).ok().filter(|entry| *entry < 8);
    if let Some(entry) = entry {
        crate::entry_groups::unswap(entry, "as the next squad member tags in");
    }
    let settings = ctx.registers[23].x() as *const u8;
    if settings.is_null() {
        return;
    }
    let source = settings.add(PLAYER_INFO_IN_MATCH_SETTINGS);
    if let Some(entry) = entry {
        follow_record(entry, source);
    }
    let Some(custom_kind) = record_clone_kind(source) else {
        return;
    };
    let input_kind = ctx.registers[0].x() as i32;
    ctx.registers[0].set_x(custom_kind as u64);
    core::ptr::write_volatile(source.add(ENTRY_RECORD_KIND) as *mut i8, custom_kind as i8);
    if let Some(roster_kind) = crate::entry_groups::roster_kind(custom_kind) {
        ctx.registers[24].set_x(roster_kind);
    }
    let n = TAG_TEAM_EXPAND_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!(
            "[cssentry] tag@1570374 #{n} entry {entry:?} next squad member kind {input_kind}->{custom_kind} source={:#x}",
            source as usize
        );
    }
}

#[cfg(feature = "css_slot")]
pub(crate) static RETRY_EXPAND_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "css_slot")]
#[skyline::hook(offset = crate::game::rel(OFF_RETRY_EXPAND_INPUT), inline)]
pub(crate) unsafe fn retry_expand_input_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let entry = u8::try_from(ctx.registers[22].x()).ok().filter(|entry| *entry < 8);
    if let Some(entry) = entry {
        crate::entry_groups::unswap(entry, "as the match restarts");
    }
    let settings = ctx.registers[20].x() as *const u8;
    if settings.is_null() {
        return;
    }
    let source = settings.add(PLAYER_INFO_IN_MATCH_SETTINGS);
    if let Some(entry) = entry {
        follow_record(entry, source);
    }
    let Some(custom_kind) = record_clone_kind(source) else {
        return;
    };
    let input_kind = ctx.registers[0].x() as i32;
    ctx.registers[0].set_x(custom_kind as u64);
    if let Some(roster_kind) = crate::entry_groups::roster_kind(custom_kind) {
        ctx.registers[27].set_x(roster_kind);
    }
    let n = RETRY_EXPAND_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 24 {
        dbg_log!(
            "[cssentry] retry@1504280 #{n} entry {entry:?} rebuilt with kind {input_kind}->{custom_kind} source={:#x}",
            source as usize
        );
    }
}

#[cfg(any(feature = "diag_article_initspoof", feature = "clone_runtime"))]
pub(crate) fn base_name_hash(base: i32) -> Option<u64> {
    let name = match base {
        0 => "mario",
        1 => "donkey",
        2 => "link",
        3 => "samus",
        4 => "samusd",
        5 => "yoshi",
        6 => "kirby",
        7 => "fox",
        8 => "pikachu",
        _ => return None,
    };
    Some(hash40(&format!("fighter_kind_{name}")))
}

#[cfg(feature = "diag_article_initspoof")]
pub(crate) unsafe fn manager_chain() -> (usize, usize, u64) {
    let mgr = *((crate::game::addr(0x5323680)) as *const usize);
    if mgr == 0 {
        return (0, 0, 0);
    }
    let sub = *((mgr + 0x68) as *const usize);
    if sub == 0 {
        return (mgr, 0, 0);
    }
    (mgr, sub, *((sub + 0x1d238) as *const u64))
}

#[cfg(any(feature = "diag_article_initspoof", feature = "clone_runtime"))]
pub(crate) unsafe fn entry_kind_slot(entry_id: i32, kind: i32) -> Option<(*mut i32, usize, u32)> {
    let t1 = *((crate::game::addr(0x52b84f8)) as *const usize);
    if t1 == 0 {
        return None;
    }
    let t2 = *(t1 as *const usize);
    if t2 == 0 {
        return None;
    }
    let per = *((t2 + entry_id as usize * 8 + 0x20) as *const usize);
    if per == 0 {
        return None;
    }
    let count = *((per + 0x14) as *const u32);
    if count == 0 || count > 8 {
        return None;
    }
    let arr = (per + 0x18) as *mut i32;
    for i in 0..count as usize {
        if *arr.add(i) == kind {
            return Some((arr.add(i), i, count));
        }
    }
    None
}

#[cfg(feature = "diag_article_initspoof")]
pub(crate) static INIT_LOG_COUNT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "diag_article_initspoof")]
#[skyline::hook(offset = crate::game::rel(OFF_FIGHTER_INIT_OBJ))]
pub(crate) unsafe fn fighter_init_object_data_hook(
    object: *mut u8,
    id: u32,
    kind: i32,
    entry_id: i32,
    name_hash: u64,
) {
    let n = INIT_LOG_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    if n < 64 {
        let (mgr, sub, mtx) = manager_chain();
        dbg_log!(
            "[initspoof] #{n} ENTER kind={kind} entry={entry_id} id={id:#x} \
             name={name_hash:#x} mgr={mgr:#x} sub={sub:#x} mtx={mtx:#x}"
        );
    }
    #[cfg(feature = "true_kind")]
    if n < 4 {
        log_entry_installer_vt(n, entry_id);
    }
    if let Some(base) = clone_base(kind) {
        let spoof_hash = match base_name_hash(base) {
            Some(h) => h,
            None => {
                dbg_log!("[initspoof] WARNING: no name hash for base {base}; spoofing kind only");
                name_hash
            }
        };
        dbg_log!(
            "[initspoof] #{n} SPOOF kind {kind} -> base {base}, name {name_hash:#x} -> {spoof_hash:#x}"
        );
        let patched = entry_kind_slot(entry_id, kind);
        match patched {
            Some((_, idx, count)) => dbg_log!(
                "[initspoof] #{n} KINDARR entry={entry_id} count={count} idx={idx}: {kind} -> {base}"
            ),
            None => dbg_log!(
                "[initspoof] #{n} WARNING: kind {kind} not in entry {entry_id} kind array; \
                 search at 0x607a5c will miss (w5=-1 -> null-mutex crash)"
            ),
        }
        if let Some((slot, _, _)) = patched {
            *slot = base;
        }
        #[cfg(feature = "css_slot")]
        with_resource_context(kind, || {
            call_original!(object, id, base, entry_id, spoof_hash)
        });
        #[cfg(not(feature = "css_slot"))]
        call_original!(object, id, base, entry_id, spoof_hash);
        if let Some((slot, _, _)) = patched {
            *slot = kind;
        }
    } else {
        call_original!(object, id, kind, entry_id, name_hash);
    }
    if n < 64 {
        let (_, sub, mtx) = manager_chain();
        dbg_log!("[initspoof] #{n} EXIT kind={kind} sub={sub:#x} mtx={mtx:#x}");
    }
}

#[cfg(test)]
mod owner_object_tests {
    use super::*;

    fn reset() {
        for slot in ENTRY_OWNER_OBJECT_ID.iter() {
            slot.store(NO_OWNER_OBJECT, core::sync::atomic::Ordering::SeqCst);
        }
        for slot in CSS_CUSTOM_ENTRY_KINDS.iter() {
            slot.store(ENTRY_SELECTION_UNOBSERVED, core::sync::atomic::Ordering::SeqCst);
        }
    }

    #[test]
    fn an_unpopulated_table_claims_nothing() {
        reset();
        assert!(!owner_object_table_is_populated());
        assert_eq!(entry_of_owner_object(0x0), None);
        assert_eq!(entry_of_owner_object(NO_OWNER_OBJECT), None);
    }

    #[test]
    fn each_owner_object_maps_to_its_own_entry() {
        reset();
        record_entry_owner_object(0, 0x0);
        record_entry_owner_object(1, 0x10000);
        assert!(owner_object_table_is_populated());
        assert_eq!(entry_of_owner_object(0x0), Some(0));
        assert_eq!(entry_of_owner_object(0x10000), Some(1));
        assert_eq!(entry_of_owner_object(0x20000), None);
    }

    #[test]
    fn a_second_fighter_does_not_inherit_the_first_entrys_clone() {
        reset();
        record_entry_owner_object(0, 0x0);
        record_entry_owner_object(1, 0x10000);
        cache_custom_entry_selection(0, Some(122));

        let mecha = entry_of_owner_object(0x0).and_then(entry_custom_kind);
        let samus = entry_of_owner_object(0x10000).and_then(entry_custom_kind);
        assert_eq!(mecha, Some(122));
        assert_eq!(
            samus, None,
            "the non-clone fighter must not resolve to the clone in entry 0"
        );
    }

    #[test]
    fn out_of_range_entries_and_sentinels_are_ignored() {
        reset();
        record_entry_owner_object(-1, 0x5);
        record_entry_owner_object(8, 0x6);
        record_entry_owner_object(0, NO_OWNER_OBJECT);
        assert!(!owner_object_table_is_populated());
    }

    #[test]
    fn a_reused_entry_takes_the_newest_owner() {
        reset();
        record_entry_owner_object(0, 0x0);
        record_entry_owner_object(0, 0x30000);
        assert_eq!(entry_of_owner_object(0x30000), Some(0));
        assert_eq!(entry_of_owner_object(0x0), None);
    }
}

