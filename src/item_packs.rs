#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemPackDeclaration {
    pub base_kind: i32,
    pub base_item: Option<String>,
    pub resource_name: String,
    pub agent_name: Option<String>,
    pub ui_id: Option<String>,
    pub training_order: i32,
    pub spawn_per: Option<i32>,
    pub spawn_min: i32,
    pub spawn_max: i32,
    pub spawn_from: Vec<String>,
    pub members: Vec<String>,
    pub common: Vec<(String, ItemPackValue)>,
    pub owner_params: Vec<(String, String, ItemPackValue)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemPackValue {
    Float(f64),
    Int(i32),
}

impl Eq for ItemPackValue {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemPackSection {
    Item,
    Common,
    OwnerParams,
}

fn names(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|name| name.trim().trim_matches('"').trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

fn parse_number(value: &str) -> Option<ItemPackValue> {
    let value = value.trim();
    if value.contains('.') || value.contains('e') {
        value.parse::<f64>().ok().map(ItemPackValue::Float)
    } else if let Some(hex) = value.strip_prefix("0x") {
        i32::from_str_radix(hex, 16).ok().map(ItemPackValue::Int)
    } else {
        value.parse::<i32>().ok().map(ItemPackValue::Int)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ItemPackError {
    Malformed { line: usize },
    BadValue { line: usize, key: String },
    MissingResourceName,
    MissingBaseKind,
    UnknownBaseItem { name: String },
}

pub const SPAWN_COUNT_DEFAULT: i32 = 1;

impl ItemPackDeclaration {
    pub fn spawn_count(&self) -> (i32, i32) {
        let min = self.spawn_min.max(0);
        (min, self.spawn_max.max(min))
    }

    pub fn agent(&self) -> &str {
        self.agent_name.as_deref().unwrap_or(&self.resource_name)
    }

    pub fn ui(&self) -> String {
        self.ui_id
            .clone()
            .unwrap_or_else(|| format!("ui_item_{}", self.resource_name))
    }
}

fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quoted = false;
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b'#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

pub fn parse(text: &str) -> Result<ItemPackDeclaration, ItemPackError> {
    let mut declaration = ItemPackDeclaration {
        spawn_min: SPAWN_COUNT_DEFAULT,
        spawn_max: SPAWN_COUNT_DEFAULT,
        ..ItemPackDeclaration::default()
    };
    let mut saw_base = false;
    let mut section = ItemPackSection::Item;
    for (index, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && !line.starts_with("[[") {
            let table = line[1..line.len() - 1].trim();
            let table = table.strip_prefix("item.").unwrap_or(table);
            section = match table {
                "item" => ItemPackSection::Item,
                "common" => ItemPackSection::Common,
                "owner_params" => ItemPackSection::OwnerParams,
                _ => return Err(ItemPackError::Malformed { line: index + 1 }),
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ItemPackError::Malformed { line: index + 1 });
        };
        let key = key.trim();
        let value = value.trim();
        let unquoted = value.trim_matches('"');
        let bad = || ItemPackError::BadValue {
            line: index + 1,
            key: key.to_string(),
        };
        match section {
            ItemPackSection::Common => {
                let Some(number) = parse_number(value) else {
                    return Err(bad());
                };
                declaration.common.push((key.to_string(), number));
                continue;
            }
            ItemPackSection::OwnerParams => {
                let Some((owner, field)) = key.split_once('.') else {
                    return Err(bad());
                };
                let Some(number) = parse_number(value) else {
                    return Err(bad());
                };
                declaration
                    .owner_params
                    .push((owner.trim().to_string(), field.trim().to_string(), number));
                continue;
            }
            ItemPackSection::Item => {}
        }
        match key {
            "base_kind" => {
                declaration.base_kind = value.parse::<i32>().map_err(|_| bad())?;
                saw_base = true;
            }
            "base_item" => declaration.base_item = Some(unquoted.to_string()),
            "resource_name" => declaration.resource_name = unquoted.to_string(),
            "agent_name" => declaration.agent_name = Some(unquoted.to_string()),
            "ui_id" => declaration.ui_id = Some(unquoted.to_string()),
            "training_order" => {
                declaration.training_order = value.parse::<i32>().map_err(|_| bad())?
            }
            "spawn_per" => declaration.spawn_per = Some(value.parse::<i32>().map_err(|_| bad())?),
            "spawn_min" => declaration.spawn_min = value.parse::<i32>().map_err(|_| bad())?,
            "spawn_max" => declaration.spawn_max = value.parse::<i32>().map_err(|_| bad())?,
            "spawn_from" => declaration.spawn_from = names(value),
            "members" => declaration.members = names(value),
            _ => {}
        }
    }
    if declaration.resource_name.is_empty() {
        return Err(ItemPackError::MissingResourceName);
    }
    let named = declaration.base_item.as_deref().and_then(dionysus_core::item_kinds::item_kind);
    match (named, saw_base, declaration.base_item.as_deref()) {
        (Some(kind), _, _) => declaration.base_kind = kind,
        (None, true, _) => {}
        (None, false, Some(name)) => return Err(ItemPackError::UnknownBaseItem { name: name.to_string() }),
        (None, false, None) => return Err(ItemPackError::MissingBaseKind),
    }
    Ok(declaration)
}

pub fn parse_all(text: &str) -> Result<Vec<ItemPackDeclaration>, ItemPackError> {
    let blocks = dionysus_core::manifest::blocks(text, "item")
        .map_err(|line| ItemPackError::Malformed { line })?;
    blocks
        .iter()
        .map(|block| {
            parse(&block.text).map_err(|error| match error {
                ItemPackError::Malformed { line } => ItemPackError::Malformed {
                    line: line + block.first_line - 1,
                },
                ItemPackError::BadValue { line, key } => ItemPackError::BadValue {
                    line: line + block.first_line - 1,
                    key,
                },
                other => other,
            })
        })
        .collect()
}

#[cfg(all(not(test), feature = "item_clone_backend"))]
mod live {
    use super::*;
    use dionysus_api::{
        CloneItemFamilyMemberV2, CloneItemFamilyRegistrationV2, CloneItemRegistrationV1,
        CloneItemUiRegistrationV1, API_VERSION_V1, API_VERSION_V2, ITEM_UI_FLAG_TRAINING, KIND_AUTO,
    };
    use std::ffi::CString;

    const MOD_ROOT: &str = "sd:/ultimate/mods";

    const MAX_KIND_PROBES: usize = 8;

    static NEXT_KIND: core::sync::atomic::AtomicI32 =
        core::sync::atomic::AtomicI32::new(crate::item_clones::FIRST_SPARSE_ITEM_KIND);

    fn register_probing(directory: &str, declaration: &ItemPackDeclaration) -> Option<i32> {
        if !declaration.members.is_empty() {
            return register_family(directory, declaration);
        }
        for attempt in 0..MAX_KIND_PROBES {
            let kind = NEXT_KIND.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            match register(directory, declaration, kind) {
                RegisterOutcome::Registered => return Some(kind),
                RegisterOutcome::KindTaken if attempt + 1 < MAX_KIND_PROBES => continue,
                RegisterOutcome::KindTaken => {
                    skyline::println!(
                        "[itempack] {directory}: no free item kind in {MAX_KIND_PROBES} tries; {} skipped",
                        declaration.resource_name
                    );
                    return None;
                }
                RegisterOutcome::Refused => return None,
            }
        }
        None
    }

    pub fn register_text(label: &str, text: &str) -> i32 {
        let parsed = match parse_all(text) {
            Ok(parsed) => parsed,
            Err(error) => {
                skyline::println!("[itempack] {label} (plugin manifest) is not readable: {error:?}");
                return dionysus_api::ERROR_MANIFEST;
            }
        };
        let mut first = dionysus_api::ERROR_MANIFEST;
        for declaration in &parsed {
            if let Some(kind) = register_probing(label, declaration) {
                if first < 0 {
                    first = kind;
                }
            }
        }
        first
    }

    enum RegisterOutcome {
        Registered,
        KindTaken,
        Refused,
    }

    pub fn load_all() {
        let Ok(entries) = std::fs::read_dir(MOD_ROOT) else {
            return;
        };
        let mut declarations: Vec<(String, String)> = Vec::new();
        for entry in entries.flatten() {
            let directory = entry.path();
            if !directory.is_dir() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(directory.join("item.toml")) else {
                continue;
            };
            if !crate::mod_folders::enabled(&directory) {
                continue;
            }
            declarations.push((
                directory
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                text,
            ));
        }
        declarations.sort_by(|a, b| a.0.cmp(&b.0));
        if declarations.is_empty() {
            return;
        }
        skyline::println!(
            "[itempack] {} item.toml pack(s) under {MOD_ROOT}",
            declarations.len()
        );
        for (directory, text) in declarations {
            let parsed = match parse_all(&text) {
                Ok(parsed) => parsed,
                Err(error) => {
                    skyline::println!(
                        "[itempack] {directory}/item.toml is not readable: {error:?}; skipped"
                    );
                    continue;
                }
            };
            if parsed.len() > 1 {
                skyline::println!("[itempack] {directory}/item.toml: {} items", parsed.len());
            }
            for declaration in &parsed {
                register_probing(&directory, declaration);
            }
        }
    }

    fn register(
        directory: &str,
        declaration: &ItemPackDeclaration,
        public_kind: i32,
    ) -> RegisterOutcome {
        let (Ok(resource), Ok(agent), Ok(ui)) = (
            CString::new(declaration.resource_name.as_str()),
            CString::new(declaration.agent()),
            CString::new(declaration.ui()),
        ) else {
            skyline::println!("[itempack] {directory}: names must not contain NUL; skipped");
            return RegisterOutcome::Refused;
        };

        let registration = CloneItemRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneItemRegistrationV1>() as u32,
            item_kind: public_kind,
            base_item_kind: declaration.base_kind,
            resource_name: resource.as_ptr(),
            agent_name: agent.as_ptr(),
            flags: 0,
            reserved_u32: 0,
            reserved: [0; 4],
        };
        if let Some(owner) = declaration
            .base_item
            .as_deref()
            .and_then(crate::custom_articles::fighter_kind_from_name_prefix)
        {
            crate::item_clones::remember_base_item_owner(declaration.base_kind, owner);
            skyline::println!(
                "[itempack] {directory}: base item {} belongs to fighter kind {owner}; its params                  will be loaded even when that fighter is absent",
                declaration.base_item.as_deref().unwrap_or("?")
            );
        }
        let result = unsafe { crate::item_clones::clone_engine_register_item_v1(&registration) };
        if result != 0 {
            skyline::println!(
                "[itempack] {directory}: REFUSED public={public_kind:#x} base={} result={result}",
                declaration.base_kind
            );
            return if result == dionysus_api::ERROR_DUPLICATE {
                RegisterOutcome::KindTaken
            } else {
                RegisterOutcome::Refused
            };
        }

        let ui_registration = CloneItemUiRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneItemUiRegistrationV1>() as u32,
            item_kind: public_kind,
            flags: ITEM_UI_FLAG_TRAINING,
            ui_id: ui.as_ptr(),
            training_order: declaration.training_order,
            rules_order: 0,
            reserved: [0; 4],
        };
        #[cfg(feature = "item_ui_backend")]
        let ui_result =
            unsafe { crate::item_ui::clone_engine_register_item_ui_v1(&ui_registration) };
        #[cfg(not(feature = "item_ui_backend"))]
        let ui_result = {
            let _ = &ui_registration;
            i32::MIN
        };
        skyline::println!(
            "[itempack] {directory}: public={public_kind:#x} base={} ({}) resource={} ui={} ui_result={ui_result}",
            declaration.base_kind,
            declaration.base_item.as_deref().unwrap_or("?"),
            declaration.resource_name,
            declaration.ui()
        );
        register_spawns(directory, declaration, public_kind);
        register_tables(directory, declaration, public_kind);
        RegisterOutcome::Registered
    }

    fn register_family(directory: &str, declaration: &ItemPackDeclaration) -> Option<i32> {
        let mut listed: Vec<&str> = declaration.members.iter().map(String::as_str).collect();
        if listed.first() != Some(&declaration.resource_name.as_str()) {
            listed.insert(0, declaration.resource_name.as_str());
        }
        let base = declaration.base_item.clone().unwrap_or_else(|| declaration.base_kind.to_string());
        let Ok(strings) = listed.iter().map(|name| CString::new(*name)).collect::<Result<Vec<_>, _>>() else {
            skyline::println!("[itempack] {directory}: names must not contain NUL; skipped");
            return None;
        };
        let members: Vec<CloneItemFamilyMemberV2> = strings
            .iter()
            .map(|name| CloneItemFamilyMemberV2 {
                item_kind: KIND_AUTO,
                flags: 0,
                resource_name: name.as_ptr(),
                agent_name: name.as_ptr(),
                reserved: [0; 2],
            })
            .collect();
        let registration = CloneItemFamilyRegistrationV2 {
            api_version: API_VERSION_V2,
            struct_size: core::mem::size_of::<CloneItemFamilyRegistrationV2>() as u32,
            base_owner_kind: declaration.base_kind,
            member_count: members.len() as u32,
            member_struct_size: core::mem::size_of::<CloneItemFamilyMemberV2>() as u32,
            flags: 0,
            members: members.as_ptr(),
            reserved: [0; 4],
        };
        let result = unsafe { crate::item_clones::clone_engine_register_item_family_v2(&registration) };
        if result != 0 {
            let expected = unsafe { crate::item_clones::native_family_len(declaration.base_kind) };
            match expected {
                Some(count) if count != listed.len() => skyline::println!(
                    "[itempack] {directory}: family on {base} REFUSED: its family has {count} members, members names {}",
                    listed.len()
                ),
                _ => skyline::println!("[itempack] {directory}: family on {base} REFUSED result={result}"),
            }
            return None;
        }
        let kind = crate::item_clones::item_kind_for_identity(&declaration.resource_name)?;
        skyline::println!(
            "[itempack] {directory}: family on {base} ({}) first kind {kind:#x}",
            listed.join(", ")
        );
        if declaration.spawn_per.is_some() {
            skyline::println!("[itempack] {directory}: spawn keys are for single items; the family appears as its base does");
        }
        register_tables(directory, declaration, kind);
        Some(kind)
    }

    fn register_tables(directory: &str, declaration: &ItemPackDeclaration, public_kind: i32) {
        for (field, value) in &declaration.common {
            let hash = dionysus_core::hash40(field);
            let result = match value {
                ItemPackValue::Float(value) => {
                    crate::item_clones::clone_engine_item_common_set(public_kind, hash, *value as f32)
                }
                ItemPackValue::Int(value) => {
                    crate::item_clones::clone_engine_item_common_set_i32(public_kind, hash, *value)
                }
            };
            skyline::println!("[itempack] {directory}: [common] {field} result={result}");
        }
        for (owner, field, value) in &declaration.owner_params {
            let Some(owner_kind) = crate::custom_articles::fighter_kind_from_str(owner) else {
                skyline::println!(
                    "[itempack] {directory}: [owner_params] {owner}.{field}: {owner} is not a fighter"
                );
                continue;
            };
            let Some(word) = dionysus_core::owner_param_words::words_of(owner_kind)
                .iter()
                .find(|word| word.path() == *field || word.field_name() == field)
            else {
                skyline::println!(
                    "[itempack] {directory}: [owner_params] {owner}.{field}: {owner} has no such param"
                );
                continue;
            };
            let offset = u32::from(word.offset);
            let result = match (word.kind, value) {
                (dionysus_core::owner_param_words::WordKind::F32, ItemPackValue::Float(value)) => {
                    crate::item_clones::clone_engine_item_owner_param_set_f32(public_kind, owner_kind, offset, *value as f32)
                }
                (dionysus_core::owner_param_words::WordKind::F32, ItemPackValue::Int(value)) => {
                    crate::item_clones::clone_engine_item_owner_param_set_f32(public_kind, owner_kind, offset, *value as f32)
                }
                (_, ItemPackValue::Int(value)) => {
                    crate::item_clones::clone_engine_item_owner_param_set_i32(public_kind, owner_kind, offset, *value)
                }
                (_, ItemPackValue::Float(value)) => {
                    crate::item_clones::clone_engine_item_owner_param_set_i32(public_kind, owner_kind, offset, *value as i32)
                }
            };
            skyline::println!(
                "[itempack] {directory}: [owner_params] {owner}.{field} (offset {offset:#x}) result={result}"
            );
        }
    }

    fn register_spawns(directory: &str, declaration: &ItemPackDeclaration, public_kind: i32) {
        let Some(per) = declaration.spawn_per else {
            return;
        };
        let (min, max) = declaration.spawn_count();
        let mut generators = vec![String::from("item_genid_random")];
        generators.extend(
            declaration
                .spawn_from
                .iter()
                .map(|container| format!("item_kind_{container}")),
        );
        for generator in generators {
            let result = crate::item_generate::register(
                public_kind,
                dionysus_core::hash40(&generator),
                per,
                min,
                max,
                dionysus_core::item_generate::VARIATION_AUTO,
            );
            skyline::println!(
                "[itempack] {directory}: spawn in {generator} per={per} count={min}..{max} result={result}"
            );
        }
    }
}

#[cfg(all(not(test), feature = "item_clone_backend"))]
pub(crate) fn load_all() {
    live::load_all();
}

#[cfg(not(all(not(test), feature = "item_clone_backend")))]
pub(crate) fn load_all() {}

#[cfg(all(not(test), feature = "item_clone_backend"))]
pub(crate) fn register_text(label: &str, text: &str) -> i32 {
    live::register_text(label, text)
}

#[cfg(not(all(not(test), feature = "item_clone_backend")))]
pub(crate) fn register_text(_label: &str, _text: &str) -> i32 {
    dionysus_api::ERROR_UNSUPPORTED
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_PACK: &str = r#"
# Example, built on Killing Edge.
base_kind     = 63
base_item     = "killsword"
resource_name = "wawa"
"#;

    #[test]
    fn reads_a_minimal_pack() {
        let declaration = parse(EXAMPLE_PACK).unwrap();
        assert_eq!(declaration.base_kind, 63);
        assert_eq!(declaration.resource_name, "wawa");
        assert_eq!(declaration.base_item.as_deref(), Some("killsword"));
    }

    #[test]
    fn the_agent_defaults_to_the_resource_namespace() {
        assert_eq!(parse(EXAMPLE_PACK).unwrap().agent(), "wawa");
    }

    #[test]
    fn the_ui_id_defaults_to_the_shipped_convention() {
        assert_eq!(parse(EXAMPLE_PACK).unwrap().ui(), "ui_item_wawa");
    }

    #[test]
    fn an_explicit_agent_and_ui_id_win() {
        let declaration = parse(
            "base_kind = 64\nresource_name = \"bonk\"\nagent_name = \"other\"\nui_id = \"ui_item_custom\"\n",
        )
        .unwrap();
        assert_eq!(declaration.agent(), "other");
        assert_eq!(declaration.ui(), "ui_item_custom");
    }

    #[test]
    fn a_pack_without_a_base_is_refused() {
        assert_eq!(
            parse("resource_name = \"wawa\"\n"),
            Err(ItemPackError::MissingBaseKind)
        );
    }

    #[test]
    fn the_base_item_alone_names_the_base() {
        let declaration = parse("base_item = \"pickelobject\"\nresource_name = \"wawa\"\n").unwrap();
        assert_eq!(declaration.base_kind, 0x1ae);
        assert_eq!(
            parse("base_item = \"nothing\"\nresource_name = \"wawa\"\n"),
            Err(ItemPackError::UnknownBaseItem { name: "nothing".to_string() })
        );
        let declaration = parse("base_kind = 64\nbase_item = \"killsword\"\nresource_name = \"wawa\"\n").unwrap();
        assert_eq!(declaration.base_kind, 63);
        let declaration = parse("base_kind = 64\nbase_item = \"a label\"\nresource_name = \"wawa\"\n").unwrap();
        assert_eq!(declaration.base_kind, 64);
    }

    #[test]
    fn a_family_lists_its_members() {
        let declaration =
            parse("base_item = \"galleom\"\nresource_name = \"boss\"\nmembers = [\"boss\", \"boss_missile\", \"boss_wave\"]\n")
                .unwrap();
        assert_eq!(declaration.base_kind, 367);
        assert_eq!(declaration.members, ["boss", "boss_missile", "boss_wave"]);
        let plain = parse("base_item = \"galleom\"\nresource_name = \"boss\"\nmembers = \"boss_missile, boss_wave\"\n").unwrap();
        assert_eq!(plain.members, ["boss_missile", "boss_wave"]);
    }

    #[test]
    fn a_pack_without_a_namespace_is_refused() {
        assert_eq!(
            parse("base_kind = 63\n"),
            Err(ItemPackError::MissingResourceName)
        );
    }

    #[test]
    fn spawn_keys_default_to_one_at_a_time_and_no_natural_spawn() {
        let declaration = parse(EXAMPLE_PACK).unwrap();
        assert_eq!(declaration.spawn_per, None);
        assert_eq!(declaration.spawn_count(), (1, 1));
        assert!(declaration.spawn_from.is_empty());

        let declaration = parse(
            "base_kind = 63\nresource_name = \"wawa\"\nspawn_per = 30\nspawn_max = 2\nspawn_from = \"box, barrel,capsule\"\n",
        )
        .unwrap();
        assert_eq!(declaration.spawn_per, Some(30));
        assert_eq!(declaration.spawn_count(), (1, 2));
        assert_eq!(declaration.spawn_from, vec!["box", "barrel", "capsule"]);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let declaration =
            parse("base_kind = 63\nresource_name = \"wawa\"\nfuture_key = \"whatever\"\n").unwrap();
        assert_eq!(declaration.resource_name, "wawa");
    }

    #[test]
    fn a_comment_inside_a_quoted_value_is_data() {
        let declaration =
            parse("base_kind = 63\nresource_name = \"wawa\"\nbase_item = \"a # b\"\n").unwrap();
        assert_eq!(declaration.base_item.as_deref(), Some("a # b"));
    }

    #[test]
    fn a_non_numeric_base_kind_names_its_line() {
        assert_eq!(
            parse("base_kind = \"killsword\"\nresource_name = \"wawa\"\n"),
            Err(ItemPackError::BadValue {
                line: 1,
                key: "base_kind".to_string()
            })
        );
    }

    #[test]
    fn a_line_that_is_not_a_pair_is_malformed() {
        assert_eq!(
            parse("base_kind = 63\nresource_name\n"),
            Err(ItemPackError::Malformed { line: 2 })
        );
    }
}
