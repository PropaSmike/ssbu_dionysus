use core::sync::atomic::{AtomicI32, AtomicU16, Ordering};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Mutex, RwLock};

use crate::RESULT_OK;
use dionysus_api::{
    ERROR_ARTICLE_CAPACITY, ERROR_ARTICLE_OWNER, ERROR_ARTICLE_RESOURCE_CONFLICT,
    ERROR_ARTICLE_SOURCE, ERROR_CUSTOM_KIND, ERROR_REGISTRATION_CLOSED,
};

pub const FIRST_CUSTOM_WEAPON_KIND: i32 = 0x267;

pub(crate) const MAX_CUSTOM_ARTICLES: usize = 256;

const LOWERCASE_FIGHTER_NAMES: usize = 0x4f80e20;
const FIGHTER_NAME_COUNT: usize = 118;

const LOWERCASE_WEAPON_NAMES: usize = 0x5185bd0;
const WEAPON_OWNER_CATEGORIES: usize = 0x5186f08;
const WEAPON_OWNER_NAMES: usize = 0x5188240;
const WEAPON_OWNER_KINDS: usize = 0x455d7e4;
const WEAPON_NAME_COUNT: usize = 0x267;

const KIND_ROW_SPAN: usize = 2048;
const NO_ROW: u16 = 0;
const SMASHLINE_GENERATE_ID_FIGHTERS: i32 = 94;

static KIND_ROWS: [AtomicU16; KIND_ROW_SPAN] = [const { AtomicU16::new(NO_ROW) }; KIND_ROW_SPAN];
static ROW_KINDS: [AtomicI32; MAX_CUSTOM_ARTICLES] = [const { AtomicI32::new(-1) }; MAX_CUSTOM_ARTICLES];

fn kind_cell(kind: i32) -> Option<&'static AtomicU16> {
    let offset = usize::try_from(kind.checked_sub(FIRST_CUSTOM_WEAPON_KIND)?).ok()?;
    KIND_ROWS.get(offset)
}

pub(crate) fn row_of(kind: i32) -> Option<usize> {
    let row = kind_cell(kind)?.load(Ordering::Acquire);
    (row != NO_ROW).then(|| row as usize - 1)
}

pub(crate) fn kind_of_row(row: usize) -> Option<i32> {
    let kind = ROW_KINDS.get(row)?.load(Ordering::Acquire);
    (kind >= FIRST_CUSTOM_WEAPON_KIND).then_some(kind)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArticleDescriptor {
    pub weapon_id: i32,
    pub max_count: i32,
    pub on_init_callback: *const u8,
    pub on_fini_callback: *const u8,
    pub extra: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StaticArticleData {
    pub descriptors: *const ArticleDescriptor,
    pub count: usize,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ArticlePlacement {
    Static,
    KirbyCopy { target_kind: i32 },
}

struct CustomArticle {
    weapon_kind: i32,
    destination_kind: i32,
    source_owner_kind: i32,
    source_weapon_kind: i32,
    name: &'static [u8],
    resource_owner_name: &'static [u8],
    name_slot: usize,
    placement: ArticlePlacement,
    shared: bool,
    original_code: bool,
    original_kind: bool,
    param_keys: Option<(u64, u64)>,
    own_param_name: &'static [u8],
}

fn trimmed_text(bytes: &[u8]) -> Option<&str> {
    core::str::from_utf8(bytes.strip_suffix(&[0]).unwrap_or(bytes)).ok()
}

unsafe fn param_keys(source_weapon_kind: i32, resource_owner: &[u8], name: &[u8]) -> Option<(u64, u64)> {
    let source = table_string(LOWERCASE_WEAPON_NAMES, source_weapon_kind as usize)?.to_str().ok()?;
    let owner = trimmed_text(resource_owner)?;
    let name = trimmed_text(name)?;
    Some((
        dionysus_core::hash40(&format!("param_{source}")),
        dionysus_core::hash40(&format!("param_{owner}_{name}")),
    ))
}

pub fn own_param_remaps(resource_owner: &str) -> Vec<(u64, u64)> {
    let Ok(registry) = registry().read() else {
        return Vec::new();
    };
    let mut remaps: Vec<(u64, u64)> = Vec::new();
    for article in registry
        .iter()
        .filter(|article| article.original_code && article.placement == ArticlePlacement::Static)
    {
        if trimmed_text(article.resource_owner_name) != Some(resource_owner) {
            continue;
        }
        let Some((from, to)) = article.param_keys else {
            continue;
        };
        if !remaps.iter().any(|(known, _)| *known == from) {
            remaps.push((from, to));
        }
    }
    remaps
}

fn registry() -> &'static RwLock<Vec<CustomArticle>> {
    static REGISTRY: RwLock<Vec<CustomArticle>> = RwLock::new(Vec::new());
    &REGISTRY
}

struct KirbyCopyHeader {
    target_kind: i32,
    header: usize,
    descriptor_count: usize,
    header_slot: usize,
    weapon_kinds: &'static [i32],
}

fn kirby_copy_headers() -> &'static RwLock<Vec<KirbyCopyHeader>> {
    static HEADERS: RwLock<Vec<KirbyCopyHeader>> = RwLock::new(Vec::new());
    &HEADERS
}

struct KirbyCopyPreloadHeader {
    weapon_kinds: Vec<i32>,
    header: usize,
    descriptor_count: usize,
}

fn kirby_copy_preload_headers() -> &'static RwLock<Vec<KirbyCopyPreloadHeader>> {
    static HEADERS: RwLock<Vec<KirbyCopyPreloadHeader>> = RwLock::new(Vec::new());
    &HEADERS
}

unsafe fn table_string(table: usize, index: usize) -> Option<&'static CStr> {
    let slot = (crate::game::addr(table) + index * 8) as *const *const c_char;
    let pointer = core::ptr::read_volatile(slot);
    (!pointer.is_null()).then(|| CStr::from_ptr(pointer))
}

pub fn fighter_kind_from_name_prefix(name: &str) -> Option<i32> {
    let mut best: Option<(usize, i32)> = None;
    for index in 0..FIGHTER_NAME_COUNT {
        let Some(entry) = (unsafe { table_string(LOWERCASE_FIGHTER_NAMES, index) }) else {
            continue;
        };
        let Ok(entry) = entry.to_str() else {
            continue;
        };
        if entry.is_empty() || !name.starts_with(entry) {
            continue;
        }
        if best.is_none_or(|(len, _)| entry.len() > len) {
            best = Some((entry.len(), index as i32));
        }
    }
    best.map(|(_, kind)| kind)
}

pub fn fighter_kind_from_str(name: &str) -> Option<i32> {
    let owned = std::ffi::CString::new(name).ok()?;
    unsafe { fighter_kind_from_name(&owned) }
}

unsafe fn fighter_kind_from_name(name: &CStr) -> Option<i32> {
    (0..FIGHTER_NAME_COUNT)
        .find(|index| {
            table_string(LOWERCASE_FIGHTER_NAMES, *index).is_some_and(|entry| entry == name)
        })
        .map(|index| index as i32)
}

fn leak_cstr(value: &CStr) -> &'static [u8] {
    let mut bytes = value.to_bytes().to_vec();
    bytes.push(0);
    Vec::leak(bytes)
}

#[inline(always)]
pub fn is_custom_weapon_kind(weapon_kind: i32) -> bool {
    row_of(weapon_kind).is_some()
}

fn article_flag(weapon_kind: i32, flag: fn(&CustomArticle) -> bool) -> bool {
    if !is_custom_weapon_kind(weapon_kind) {
        return true;
    }
    registry()
        .read()
        .ok()
        .and_then(|registry| {
            registry
                .iter()
                .find(|article| article.weapon_kind == weapon_kind)
                .map(flag)
        })
        .unwrap_or(true)
}

pub fn runs_original_code(weapon_kind: i32) -> bool {
    article_flag(weapon_kind, |article| article.original_code)
}

pub fn shows_original_kind(weapon_kind: i32) -> bool {
    article_flag(weapon_kind, |article| article.original_kind)
}

pub fn always(_weapon_kind: i32) -> bool {
    true
}

pub fn set_original(weapon_kind: i32, code: bool, kind: bool) -> bool {
    let Ok(mut registry) = registry().write() else {
        return false;
    };
    let Some(article) = registry
        .iter_mut()
        .find(|article| article.weapon_kind == weapon_kind)
    else {
        return false;
    };
    article.original_code = code;
    article.original_kind = kind;
    true
}

pub fn is_shared_weapon_kind(weapon_kind: i32) -> bool {
    row_of(weapon_kind).is_some()
        && registry().read().ok().is_some_and(|registry| {
            registry
                .iter()
                .any(|article| article.weapon_kind == weapon_kind && article.shared)
        })
}

fn assets_present(weapon_kind: i32) -> bool {
    const FORCE_SOURCE_ASSETS: bool = false;
    if FORCE_SOURCE_ASSETS {
        static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
            crate::dbg_log_public(
                "[articleassets] BISECT: forcing the source's assets for every cloned article",
            );
        }
        return false;
    }

    static CACHE: std::sync::OnceLock<RwLock<Vec<(i32, bool)>>> = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(Vec::new()));

    if let Ok(cached) = cache.read() {
        if let Some((_, present)) = cached.iter().find(|(kind, _)| *kind == weapon_kind) {
            return *present;
        }
    }

    let Some((owner, name)) = registry().read().ok().and_then(|registry| {
        registry
            .iter()
            .find(|article| article.weapon_kind == weapon_kind)
            .map(|article| {
                (
                    String::from_utf8_lossy(article.resource_owner_name)
                        .trim_end_matches('\0')
                        .to_string(),
                    String::from_utf8_lossy(article.name)
                        .trim_end_matches('\0')
                        .to_string(),
                )
            })
    }) else {
        return false;
    };

    let path = format!("fighter/{owner}/motion/{name}/c00/motion_list.bin");
    let present = crate::fighter_modules::path_exists(&path);
    crate::dbg_log_public(&format!(
        "[articleassets] weapon kind {weapon_kind}: '{path}' {}",
        if present {
            "found; using the minted name"
        } else {
            "MISSING; falling back to the source's assets"
        }
    ));

    if let Ok(mut cached) = cache.write() {
        cached.push((weapon_kind, present));
    }
    present
}

pub fn registered_article_name(weapon_kind: i32) -> Option<&'static [u8]> {
    registry()
        .read()
        .ok()?
        .iter()
        .find(|article| article.weapon_kind == weapon_kind)
        .map(|article| article.name)
}

pub fn custom_weapon_name(weapon_kind: i32) -> Option<&'static [u8]> {
    if !is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    if runs_original_code(weapon_kind) && !assets_present(weapon_kind) {
        return source_weapon_name(weapon_kind).map(|name| name.to_bytes_with_nul());
    }
    registry()
        .read()
        .ok()?
        .iter()
        .find(|article| article.weapon_kind == weapon_kind)
        .map(|article| article.name)
}

pub fn weapon_kind_from_names(owner: &str, weapon: &str) -> Option<i32> {
    (0..WEAPON_NAME_COUNT).find_map(|index| {
        let owner_entry = unsafe { table_string(WEAPON_OWNER_NAMES, index) }?;
        let weapon_entry = unsafe { table_string(LOWERCASE_WEAPON_NAMES, index) }?;
        (owner_entry.to_bytes().eq_ignore_ascii_case(owner.as_bytes())
            && weapon_entry.to_bytes().eq_ignore_ascii_case(weapon.as_bytes()))
        .then_some(index as i32)
    })
}

pub fn weapon_kinds_of_owner(owner_kind: i32) -> Vec<i32> {
    (0..WEAPON_NAME_COUNT)
        .filter(|index| {
            let slot = (crate::game::addr(WEAPON_OWNER_KINDS) + index * 4) as *const i32;
            let stored: i32 = unsafe { core::ptr::read_volatile(slot) };
            stored == owner_kind
        })
        .map(|index| index as i32)
        .collect()
}

pub fn article_kind_by_name(resource_owner: &str, name: &str) -> Option<i32> {
    let registry = registry().read().ok()?;
    registry
        .iter()
        .find(|article| {
            article.placement == ArticlePlacement::Static
                && article.resource_owner_name.strip_suffix(&[0]) == Some(resource_owner.as_bytes())
                && article.name.strip_suffix(&[0]) == Some(name.as_bytes())
        })
        .map(|article| article.weapon_kind)
}

pub fn copy_article_kind_by_name(target_kind: i32, name: &str) -> Option<i32> {
    let registry = registry().read().ok()?;
    registry
        .iter()
        .find(|article| {
            article.placement == ArticlePlacement::KirbyCopy { target_kind }
                && article.name.strip_suffix(&[0]) == Some(name.as_bytes())
        })
        .map(|article| article.weapon_kind)
}

pub fn weapon_name_table_bias(weapon_kind: i32) -> Option<u64> {
    if !is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    let slot = if assets_present(weapon_kind) {
        registry()
            .read()
            .ok()?
            .iter()
            .find(|article| article.weapon_kind == weapon_kind)
            .map(|article| article.name_slot)?
    } else {
        let source = custom_weapon_source_kind(weapon_kind)?;
        crate::game::addr(LOWERCASE_WEAPON_NAMES) + source as usize * 8
    };
    Some((slot as u64).wrapping_sub((weapon_kind as u64).wrapping_mul(8)))
}

pub fn agent_names(weapon_kind: i32) -> Option<(&'static [u8], &'static [u8])> {
    if !is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    registry()
        .read()
        .ok()?
        .iter()
        .find(|article| article.weapon_kind == weapon_kind)
        .map(|article| (article.resource_owner_name, article.name))
}

pub fn custom_weapon_owner_name(weapon_kind: i32) -> Option<&'static [u8]> {
    if !is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    if runs_original_code(weapon_kind) && !assets_present(weapon_kind) {
        return source_weapon_owner_name(weapon_kind).map(|name| name.to_bytes_with_nul());
    }
    registry()
        .read()
        .ok()?
        .iter()
        .find(|article| article.weapon_kind == weapon_kind)
        .map(|article| article.resource_owner_name)
}

pub fn resource_owner_of(weapon_kind: i32) -> Option<&'static [u8]> {
    registry()
        .read()
        .ok()?
        .iter()
        .find(|article| article.weapon_kind == weapon_kind)
        .map(|article| article.resource_owner_name)
}

pub fn custom_weapon_source_kind(weapon_kind: i32) -> Option<i32> {
    if !is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    registry()
        .read()
        .ok()?
        .iter()
        .find(|article| article.weapon_kind == weapon_kind)
        .map(|article| article.source_weapon_kind)
}

pub fn is_source_of_custom_weapon(weapon_kind: i32) -> bool {
    registry()
        .read()
        .ok()
        .map(|articles| articles.iter().any(|article| article.source_weapon_kind == weapon_kind))
        .unwrap_or(false)
}

pub fn param_source_owner_kind(weapon_kind: i32) -> Option<i32> {
    if !runs_original_code(weapon_kind) {
        return None;
    }
    let source_owner = source_weapon_owner_kind(weapon_kind)?;
    crate::fighter_params::kind_params_have_real_data(source_owner).then_some(source_owner)
}

pub fn param_lookup_weapon_name(weapon_kind: i32) -> Option<&'static [u8]> {
    if !runs_original_code(weapon_kind) {
        return registry()
            .read()
            .ok()?
            .iter()
            .find(|article| article.weapon_kind == weapon_kind)
            .map(|article| article.own_param_name);
    }
    if param_source_owner_kind(weapon_kind).is_some() {
        return source_weapon_name(weapon_kind).map(|name| name.to_bytes_with_nul());
    }
    custom_weapon_name(weapon_kind)
}

pub fn source_weapon_name(weapon_kind: i32) -> Option<&'static CStr> {
    let source = custom_weapon_source_kind(weapon_kind)?;
    unsafe { table_string(LOWERCASE_WEAPON_NAMES, source as usize) }
}

pub fn source_weapon_owner_name(weapon_kind: i32) -> Option<&'static CStr> {
    let source = custom_weapon_source_kind(weapon_kind)?;
    unsafe { table_string(WEAPON_OWNER_NAMES, source as usize) }
}

pub fn fighter_name(fighter_kind: i32) -> Option<&'static CStr> {
    if fighter_kind < 0 || fighter_kind as usize >= FIGHTER_NAME_COUNT {
        return None;
    }
    unsafe { table_string(LOWERCASE_FIGHTER_NAMES, fighter_kind as usize) }
}

pub fn source_weapon_owner_kind(weapon_kind: i32) -> Option<i32> {
    let source = custom_weapon_source_kind(weapon_kind)?;
    if source < 0 || source as usize >= WEAPON_NAME_COUNT {
        return None;
    }
    let slot = (crate::game::addr(WEAPON_OWNER_KINDS) + source as usize * 4) as *const i32;
    Some(unsafe { core::ptr::read_volatile(slot) })
}

pub fn custom_weapon_owner_category(weapon_kind: i32) -> Option<*const c_char> {
    let source = custom_weapon_source_kind(weapon_kind)?;
    unsafe { table_string(WEAPON_OWNER_CATEGORIES, source as usize) }
        .map(|category| category.as_ptr())
}

const PARAM_RECORD_BASE: u32 = 0x60;
const PARAM_RECORD_STRIDE: u32 = 0x38;
const PARAM_RECORD_PAYLOAD: u32 = 0x10;
const PAYLOAD_SCAN_WORDS: usize = 0x100;
const TEXT_LIMIT: usize = 0x39c7e90;

pub fn callback_payload_offsets(callback: *const u8, owner_kind: i32) -> Vec<u32> {
    let mut offsets = Vec::new();
    if callback.is_null() || owner_kind < 0 {
        return offsets;
    }
    let payload = PARAM_RECORD_BASE + owner_kind as u32 * PARAM_RECORD_STRIDE + PARAM_RECORD_PAYLOAD;
    if payload % 8 != 0 || payload >= 0x8000 {
        return offsets;
    }
    let Some(relative) = (callback as usize).checked_sub(crate::game::raw_text_base()) else {
        return offsets;
    };
    if relative + PAYLOAD_SCAN_WORDS * 4 >= TEXT_LIMIT {
        return offsets;
    }
    let words = unsafe { core::slice::from_raw_parts(callback as *const u32, PAYLOAD_SCAN_WORDS) };

    let is_ldr64 = |word: u32| word & 0xFFC0_0000 == 0xF940_0000;
    let mut holder = None;
    for word in words.iter().copied() {
        if is_ldr64(word) && (word >> 10) & 0xFFF == payload / 8 {
            holder = Some(word & 0x1F);
            continue;
        }
        let Some(base) = holder else { continue };
        if is_ldr64(word) && (word >> 5) & 0x1F == base {
            let offset = ((word >> 10) & 0xFFF) * 8;
            if !offsets.contains(&offset) {
                offsets.push(offset);
            }
        }
    }
    offsets
}

pub fn vanilla_weapon_owner_kind(weapon_kind: i32) -> Option<i32> {
    if weapon_kind < 0 || weapon_kind as usize >= WEAPON_NAME_COUNT {
        return None;
    }
    let slot = (crate::game::addr(WEAPON_OWNER_KINDS) + weapon_kind as usize * 4) as *const i32;
    let owner = unsafe { core::ptr::read_volatile(slot) };
    (owner >= 0).then_some(owner)
}

pub fn resource_owners_for_source_owner(source_owner_kind: i32) -> Vec<&'static [u8]> {
    let Ok(registry) = registry().read() else {
        return Vec::new();
    };
    let mut owners: Vec<&'static [u8]> = Vec::new();
    for article in registry
        .iter()
        .filter(|article| article.source_owner_kind == source_owner_kind)
    {
        if !owners.contains(&article.resource_owner_name) {
            owners.push(article.resource_owner_name);
        }
    }
    owners
}

pub fn inherited_child_owner(weapon_kind: i32) -> Option<(i32, i32)> {
    if is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    let owner = vanilla_weapon_owner_kind(weapon_kind)?;
    let registry = registry().read().ok()?;
    let mut candidates = registry.iter().filter(|article| {
        article.source_owner_kind == owner && article.source_weapon_kind != weapon_kind
    });
    let first = candidates.clone().next()?;
    let live = candidates.find(|article| {
        crate::fighter_params::kind_params_present(article.destination_kind)
            && !crate::fighter_params::kind_params_are_foreign(article.destination_kind)
    });
    Some((owner, live.unwrap_or(first).destination_kind))
}

pub fn custom_weapon_owner_kind(weapon_kind: i32) -> Option<i32> {
    if !is_custom_weapon_kind(weapon_kind) {
        return None;
    }
    registry()
        .read()
        .ok()?
        .iter()
        .find(|article| article.weapon_kind == weapon_kind)
        .map(|article| article.destination_kind)
}

pub fn descriptors_for(
    fighter_kind: i32,
    mut source_articles: impl FnMut(i32) -> Option<StaticArticleData>,
) -> Vec<ArticleDescriptor> {
    let Ok(registry) = registry().read() else {
        return Vec::new();
    };

    let mut appended = Vec::new();
    for article in registry.iter().filter(|article| {
        article.destination_kind == fighter_kind
            && article.placement == ArticlePlacement::Static
            && !article.shared
    }) {
        let Some(table) = source_articles(article.source_owner_kind) else {
            continue;
        };
        let Some(mut descriptor) = find_descriptor(&table, article.source_weapon_kind) else {
            continue;
        };
        descriptor.weapon_id = article.weapon_kind;
        appended.push(descriptor);
    }
    appended
}

pub fn prime_kirby_copy_headers(
    mut source_articles: impl FnMut(i32) -> Option<StaticArticleData>,
) -> Vec<i32> {
    let registrations = {
        let Ok(registry) = registry().read() else {
            return Vec::new();
        };
        registry
            .iter()
            .filter_map(|article| match article.placement {
                ArticlePlacement::KirbyCopy { target_kind } => Some((
                    target_kind,
                    article.weapon_kind,
                    article.source_owner_kind,
                    article.source_weapon_kind,
                )),
                ArticlePlacement::Static => None,
            })
            .collect::<Vec<_>>()
    };

    let mut source_owners = Vec::new();
    let mut target_kinds = Vec::new();
    for (target_kind, _, source_owner_kind, _) in registrations.iter().copied() {
        if !source_owners.contains(&source_owner_kind) {
            source_owners.push(source_owner_kind);
        }
        if !target_kinds.contains(&target_kind) {
            target_kinds.push(target_kind);
        }
    }

    for target_kind in target_kinds {
        if kirby_copy_headers().read().ok().is_some_and(|headers| {
            headers
                .iter()
                .any(|header| header.target_kind == target_kind)
        }) {
            continue;
        }

        let target_articles = registrations
            .iter()
            .copied()
            .filter(|(target, _, _, _)| *target == target_kind)
            .collect::<Vec<_>>();
        let base_seeded;
        let mut descriptors = Vec::with_capacity(target_articles.len());
        let mut weapon_kinds = Vec::with_capacity(target_articles.len());
        let mut complete = true;

        if let Some(base_kind) = crate::clone_definition(target_kind).map(|d| d.base_kind) {
            if let Some(base_table) = source_articles(base_kind) {
                if !base_table.descriptors.is_null() && base_table.count <= 256 {
                    let base_descriptors = unsafe {
                        core::slice::from_raw_parts(base_table.descriptors, base_table.count)
                    };
                    for descriptor in base_descriptors {
                        descriptors.push(*descriptor);
                    }
                }
            }
        }
        base_seeded = descriptors.len();

        for (_, weapon_kind, source_owner_kind, source_weapon_kind) in target_articles {
            let Some(table) = source_articles(source_owner_kind) else {
                crate::dbg_log_public(&format!(
                    "[copyarticle] target={target_kind} source fighter {source_owner_kind} has no article table"
                ));
                complete = false;
                break;
            };
            let Some(mut descriptor) = find_descriptor(&table, source_weapon_kind) else {
                crate::dbg_log_public(&format!(
                    "[copyarticle] target={target_kind} source weapon {source_weapon_kind} is absent"
                ));
                complete = false;
                break;
            };
            descriptor.weapon_id = weapon_kind;
            descriptors.push(descriptor);
            weapon_kinds.push(weapon_kind);
        }

        if !complete || descriptors.len() == base_seeded {
            continue;
        }

        let total_count = descriptors.len();
        let minted_count = total_count - base_seeded;
        let full: &'static [ArticleDescriptor] = Vec::leak(descriptors);

        let header = Box::leak(Box::new(StaticArticleData {
            descriptors: unsafe { full.as_ptr().add(base_seeded) },
            count: minted_count,
        })) as *const StaticArticleData as usize;

        let callback_header = Box::leak(Box::new(StaticArticleData {
            descriptors: full.as_ptr(),
            count: total_count,
        })) as *const StaticArticleData as usize;
        let header_slot = Box::leak(Box::new(callback_header)) as *mut usize as usize;

        let descriptor_count = minted_count;
        let weapon_kinds: &'static [i32] = Vec::leak(weapon_kinds);

        if let Ok(mut headers) = kirby_copy_headers().write() {
            if headers
                .iter()
                .all(|existing| existing.target_kind != target_kind)
            {
                headers.push(KirbyCopyHeader {
                    target_kind,
                    header,
                    descriptor_count,
                    header_slot,
                    weapon_kinds,
                });
                crate::dbg_log_public(&format!(
                    "[copyarticle] primed target={target_kind} pool={header:#x} minted={minted_count} callback={callback_header:#x} indexed={total_count} (base={base_seeded})"
                ));
            }
        }
    }

    source_owners
}

pub fn kirby_copy_header(target_kind: i32) -> Option<(usize, usize)> {
    let headers = kirby_copy_headers().read().ok()?;
    let header = headers
        .iter()
        .find(|header| header.target_kind == target_kind)?;
    Some((header.header, header.descriptor_count))
}

pub unsafe fn kirby_copy_header_without(header: usize, skipped: &[usize]) -> usize {
    static MASKED: Mutex<Vec<(usize, Vec<usize>, usize)>> = Mutex::new(Vec::new());
    let Ok(mut cache) = MASKED.lock() else {
        return header;
    };
    if let Some((_, _, masked)) = cache
        .iter()
        .find(|(source, rows, _)| *source == header && rows.as_slice() == skipped)
    {
        return *masked;
    }
    let table = &*(header as *const StaticArticleData);
    let mut descriptors = core::slice::from_raw_parts(table.descriptors, table.count).to_vec();
    for index in skipped {
        descriptors[*index].max_count = 0;
    }
    let count = descriptors.len();
    let masked = Box::leak(Box::new(StaticArticleData {
        descriptors: Vec::leak(descriptors).as_ptr(),
        count,
    })) as *const StaticArticleData as usize;
    cache.push((header, skipped.to_vec(), masked));
    masked
}

pub fn kirby_copy_header_slot(target_kind: i32) -> Option<usize> {
    kirby_copy_headers()
        .read()
        .ok()?
        .iter()
        .find(|header| header.target_kind == target_kind)
        .map(|header| header.header_slot)
}

pub unsafe fn kirby_copy_dynamic_preload_header(
    target_present: impl Fn(i32) -> bool,
) -> Option<(usize, usize)> {
    let (dynamic_descriptors, weapon_kinds) = {
        let headers = kirby_copy_headers().read().ok()?;
        let mut descriptors = Vec::new();
        let mut kinds = Vec::new();

        for entry in headers.iter() {
            if !target_present(entry.target_kind) {
                continue;
            }
            let header = entry.header as *const StaticArticleData;
            if header.is_null() || (*header).count > 256 {
                continue;
            }
            let source = (*header).descriptors;
            if source.is_null() && (*header).count != 0 {
                continue;
            }

            for descriptor in core::slice::from_raw_parts(source, (*header).count) {
                if !entry.weapon_kinds.contains(&descriptor.weapon_id) {
                    continue;
                }
                if !kinds.contains(&descriptor.weapon_id) {
                    kinds.push(descriptor.weapon_id);
                    descriptors.push(*descriptor);
                }
            }
        }
        (descriptors, kinds)
    };

    if dynamic_descriptors.is_empty() {
        return None;
    }

    if let Ok(headers) = kirby_copy_preload_headers().read() {
        if let Some(cached) = headers
            .iter()
            .find(|cached| cached.weapon_kinds == weapon_kinds)
        {
            return Some((cached.header, cached.descriptor_count));
        }
    }

    let descriptor_count = dynamic_descriptors.len();
    let descriptors = Vec::leak(dynamic_descriptors);
    let header = Box::leak(Box::new(StaticArticleData {
        descriptors: descriptors.as_ptr(),
        count: descriptor_count,
    })) as *const StaticArticleData as usize;

    if let Ok(mut headers) = kirby_copy_preload_headers().write() {
        if let Some(cached) = headers
            .iter()
            .find(|cached| cached.weapon_kinds == weapon_kinds)
        {
            return Some((cached.header, cached.descriptor_count));
        }
        headers.push(KirbyCopyPreloadHeader {
            weapon_kinds,
            header,
            descriptor_count,
        });
    }

    crate::dbg_log_public(&format!(
        "[copyarticle] dynamic preload table count={descriptor_count} header={header:#x}"
    ));
    Some((header, descriptor_count))
}

pub fn is_kirby_copy_weapon(weapon_kind: i32) -> bool {
    registry().read().map_or(false, |registry| {
        registry.iter().any(|article| {
            article.weapon_kind == weapon_kind
                && matches!(article.placement, ArticlePlacement::KirbyCopy { .. })
        })
    })
}

pub fn kirby_copy_index(target_kind: i32, weapon_kind: i32) -> Option<i32> {
    let registry = registry().read().ok()?;
    registry
        .iter()
        .filter_map(|article| match article.placement {
            ArticlePlacement::KirbyCopy {
                target_kind: article_target,
            } if article_target == target_kind => Some(article.weapon_kind),
            _ => None,
        })
        .position(|candidate| candidate == weapon_kind)
        .map(|index| index as i32)
}

pub fn index_of(table: &StaticArticleData, weapon_kind: i32) -> Option<i32> {
    if table.descriptors.is_null() || table.count == 0 {
        return None;
    }
    let descriptors = unsafe { core::slice::from_raw_parts(table.descriptors, table.count) };
    descriptors
        .iter()
        .position(|descriptor| descriptor.weapon_id == weapon_kind)
        .map(|position| position as i32)
}

fn find_descriptor(table: &StaticArticleData, weapon_kind: i32) -> Option<ArticleDescriptor> {
    if table.descriptors.is_null() || table.count == 0 {
        return None;
    }
    let descriptors = unsafe { core::slice::from_raw_parts(table.descriptors, table.count) };
    descriptors
        .iter()
        .find(|descriptor| descriptor.weapon_id == weapon_kind)
        .copied()
}

pub unsafe fn register(
    source_owner: *const c_char,
    source_weapon_kind: i32,
    destination_owner: *const c_char,
    resource_owner: *const c_char,
    name: *const c_char,
) -> i32 {
    register_inner(
        source_owner,
        source_weapon_kind,
        destination_owner,
        resource_owner,
        name,
        ArticlePlacement::Static,
    )
}

pub unsafe fn register_kirby_copy(
    target_kind: i32,
    source_owner: *const c_char,
    source_weapon_kind: i32,
    resource_owner: *const c_char,
    name: *const c_char,
) -> i32 {
    if crate::clone_definition(target_kind).is_none() {
        return ERROR_CUSTOM_KIND;
    }
    register_inner(
        source_owner,
        source_weapon_kind,
        b"kirby\0".as_ptr() as *const c_char,
        resource_owner,
        name,
        ArticlePlacement::KirbyCopy { target_kind },
    )
}

unsafe fn share_with_smashline(
    owner_kind: i32,
    resource_owner: &'static [u8],
    name: &'static [u8],
    source_weapon_kind: i32,
) -> Result<i32, i32> {
    let resource_text = resource_owner.strip_suffix(&[0]).unwrap_or(resource_owner);
    let name_text = name.strip_suffix(&[0]).unwrap_or(name);
    let decline = |why: String, code: i32| {
        skyline::println!(
            "[article] DECLINED '{}': {why}",
            String::from_utf8_lossy(name_text)
        );
        code
    };
    if !(0..SMASHLINE_GENERATE_ID_FIGHTERS).contains(&owner_kind) {
        return Err(decline(
            format!(
                "Smashline 2 adds articles only to fighter kinds below {SMASHLINE_GENERATE_ID_FIGHTERS}, and this table is fighter kind {owner_kind}'s"
            ),
            ERROR_ARTICLE_OWNER,
        ));
    }
    let Some(owner) = table_string(LOWERCASE_FIGHTER_NAMES, owner_kind as usize) else {
        return Err(decline(format!("fighter kind {owner_kind} has no name"), ERROR_ARTICLE_OWNER));
    };
    let source_owner = table_string(WEAPON_OWNER_NAMES, source_weapon_kind as usize);
    if source_owner.and_then(|owner| fighter_kind_from_name(owner)).is_none() {
        return Err(decline(
            format!("source weapon {source_weapon_kind}'s owner is not a fighter name Smashline can resolve"),
            ERROR_ARTICLE_SOURCE,
        ));
    }
    let (Ok(owner_str), Ok(resource_str), Ok(name_str)) = (
        owner.to_str(),
        core::str::from_utf8(resource_text),
        core::str::from_utf8(name_text),
    ) else {
        return Err(decline("owner or name is not UTF-8".to_string(), ERROR_ARTICLE_OWNER));
    };
    let article = format!("{resource_str}_{name_str}");
    if let Some(vanilla) = weapon_kind_from_names(owner_str, &article) {
        return Err(decline(
            format!("vanilla weapon {vanilla} is already called {owner_str}_{article}"),
            ERROR_ARTICLE_RESOURCE_CONFLICT,
        ));
    }
    let Some(shared) =
        crate::smashline_articles::clone_weapon(owner_str.as_bytes(), article.as_bytes(), source_weapon_kind)
    else {
        return Err(decline("Smashline 2 did not answer".to_string(), ERROR_ARTICLE_OWNER));
    };
    skyline::println!(
        "[article] smashline numbered '{owner_str}_{article}' as weapon kind {} (generate id {} on fighter kind {owner_kind})",
        shared.weapon_kind,
        shared.generate_id
    );
    Ok(shared.weapon_kind)
}

unsafe fn register_inner(
    source_owner: *const c_char,
    source_weapon_kind: i32,
    destination_owner: *const c_char,
    resource_owner: *const c_char,
    name: *const c_char,
    placement: ArticlePlacement,
) -> i32 {
    if source_owner.is_null() || destination_owner.is_null() || name.is_null() {
        return dionysus_api::ERROR_NULL;
    }
    let source_owner = CStr::from_ptr(source_owner);
    let destination_owner = CStr::from_ptr(destination_owner);
    let name = CStr::from_ptr(name);

    let Some(source_owner_kind) = fighter_kind_from_name(source_owner) else {
        return ERROR_ARTICLE_OWNER;
    };
    let resolve = |owner: &CStr| -> Option<(i32, Vec<u8>)> {
        if let Some(kind) = fighter_kind_from_name(owner) {
            return Some((kind, owner.to_bytes_with_nul().to_vec()));
        }
        let definition = owner
            .to_str()
            .ok()
            .and_then(crate::clone_definition_from_name)?;
        let mut namespace = definition.resource_name.as_bytes().to_vec();
        namespace.push(0);
        Some((definition.base_kind, namespace))
    };

    let Some((destination_kind, default_namespace)) = resolve(destination_owner) else {
        return ERROR_ARTICLE_OWNER;
    };

    let resource_owner = if resource_owner.is_null() {
        default_namespace
    } else {
        let requested = CStr::from_ptr(resource_owner);
        let Some((_, namespace)) = resolve(requested) else {
            return ERROR_ARTICLE_OWNER;
        };
        if namespace != default_namespace {
            skyline::println!(
                "[article] NOTE '{}': files under 'fighter/{}/' but the table is {}'s, whose own \
                 root is 'fighter/{}/'. Those files must be declared in a `new-dir-files` group \
                 the match loads, or the resource load will fault.",
                name.to_string_lossy(),
                String::from_utf8_lossy(&namespace).trim_end_matches('\0'),
                destination_owner.to_string_lossy(),
                String::from_utf8_lossy(&default_namespace).trim_end_matches('\0'),
            );
        }
        namespace
    };
    if !(0..WEAPON_NAME_COUNT as i32).contains(&source_weapon_kind) {
        return ERROR_ARTICLE_SOURCE;
    }

    let Ok(mut registry) = registry().write() else {
        return ERROR_ARTICLE_CAPACITY;
    };

    if let Some(existing) = registry.iter().find(|article| {
        article.source_owner_kind == source_owner_kind
            && article.source_weapon_kind == source_weapon_kind
            && article.destination_kind == destination_kind
            && article.name == name.to_bytes_with_nul()
            && article.placement == placement
    }) {
        return existing.weapon_kind;
    }

    if let ArticlePlacement::KirbyCopy { target_kind } = placement {
        if kirby_copy_headers().read().ok().is_some_and(|headers| {
            headers
                .iter()
                .any(|header| header.target_kind == target_kind)
        }) {
            return ERROR_REGISTRATION_CLOSED;
        }
    }

    if let Some(existing) = registry.iter().find(|article| {
        article.resource_owner_name == resource_owner.as_slice()
            && article.name == name.to_bytes_with_nul()
    }) {
        skyline::println!(
            "[article] DECLINED '{}': weapon kind {} already owns 'fighter/{}/…/{}/', and two \
             weapon kinds cannot share one article directory. Give this one its own name.",
            name.to_string_lossy(),
            existing.weapon_kind,
            String::from_utf8_lossy(existing.resource_owner_name).trim_end_matches('\0'),
            name.to_string_lossy(),
        );
        return ERROR_ARTICLE_RESOURCE_CONFLICT;
    }

    if registry.len() >= MAX_CUSTOM_ARTICLES {
        return ERROR_ARTICLE_CAPACITY;
    }

    let row = registry.len();
    let leaked_name = leak_cstr(name);
    let resource_owner_name: &'static [u8] = Box::leak(resource_owner.into_boxed_slice());
    let (weapon_kind, shared) = if crate::smashline_articles::numbering_shared() {
        match share_with_smashline(
            destination_kind,
            resource_owner_name,
            leaked_name,
            source_weapon_kind,
        ) {
            Ok(kind) => (kind, true),
            Err(code) => return code,
        }
    } else {
        (FIRST_CUSTOM_WEAPON_KIND + row as i32, false)
    };
    let Some(cell) = kind_cell(weapon_kind) else {
        skyline::println!(
            "[article] DECLINED '{}': weapon kind {weapon_kind} is past the engine's article span ({} kinds from {FIRST_CUSTOM_WEAPON_KIND})",
            name.to_string_lossy(),
            KIND_ROW_SPAN
        );
        return ERROR_ARTICLE_CAPACITY;
    };
    if let Some(taken) = row_of(weapon_kind) {
        skyline::println!(
            "[article] DECLINED '{}': weapon kind {weapon_kind} already belongs to article row {taken}",
            name.to_string_lossy()
        );
        return ERROR_ARTICLE_RESOURCE_CONFLICT;
    }
    let name_slot = Box::leak(Box::new(leaked_name.as_ptr())) as *const *const u8 as usize;
    let param_keys = param_keys(source_weapon_kind, resource_owner_name, leaked_name);
    let own_param_name: &'static [u8] = Box::leak(
        format!(
            "{}_{}\0",
            trimmed_text(resource_owner_name).unwrap_or_default(),
            trimmed_text(leaked_name).unwrap_or_default()
        )
        .into_bytes()
        .into_boxed_slice(),
    );
    registry.push(CustomArticle {
        weapon_kind,
        destination_kind,
        source_owner_kind,
        source_weapon_kind,
        name: leaked_name,
        resource_owner_name,
        name_slot,
        placement,
        shared,
        original_code: true,
        original_kind: true,
        param_keys,
        own_param_name,
    });
    ROW_KINDS[row].store(weapon_kind, Ordering::Release);
    cell.store(row as u16 + 1, Ordering::Release);

    let article = registry.last().expect("just pushed");
    let placement_name = match placement {
        ArticlePlacement::Static => "static",
        ArticlePlacement::KirbyCopy { .. } => "kirby-copy",
    };
    skyline::println!(
        "[article] {} -> {} as '{}' = weapon kind {weapon_kind} (source weapon \
         {source_weapon_kind}, placement {placement_name}, table fighter kind {destination_kind}, files under 'fighter/{}/')",
        source_owner.to_string_lossy(),
        destination_owner.to_string_lossy(),
        name.to_string_lossy(),
        String::from_utf8_lossy(article.resource_owner_name).trim_end_matches('\0'),
    );
    let _ = RESULT_OK;
    weapon_kind
}
