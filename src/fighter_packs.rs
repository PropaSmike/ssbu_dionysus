use dionysus_core::fighter_toml::{FighterDeclaration, ParamDeclaration, ParamOp, ParamValue};
use dionysus_core::tether::Lasso;
use std::sync::{Mutex, OnceLock};

pub(crate) const MOD_ROOT: &str = "sd:/ultimate/mods";

pub(crate) struct LoadedPack {
    pub(crate) directory: &'static str,
    pub(crate) kind: i32,
    pub(crate) declaration: FighterDeclaration,
}

fn loaded() -> &'static Mutex<Vec<LoadedPack>> {
    static LOADED: OnceLock<Mutex<Vec<LoadedPack>>> = OnceLock::new();
    LOADED.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn pack_directory(kind: i32) -> Option<&'static str> {
    let packs = loaded().lock().ok()?;
    packs
        .iter()
        .find(|pack| pack.kind == kind)
        .map(|pack| pack.directory)
}

pub(crate) fn params_for_kind(kind: i32) -> Vec<ParamDeclaration> {
    loaded()
        .lock()
        .ok()
        .and_then(|packs| {
            packs
                .iter()
                .find(|pack| pack.kind == kind)
                .map(|pack| with_tether_params(&pack.declaration))
        })
        .unwrap_or_default()
}

fn with_tether_params(declaration: &FighterDeclaration) -> Vec<ParamDeclaration> {
    let mut params = declaration.params.clone();
    let Some(tether) = &declaration.tether else {
        return params;
    };
    for (name, value) in tether.params() {
        if params.iter().any(|param| param.name == name) {
            continue;
        }
        params.push(ParamDeclaration {
            name,
            slot: None,
            op: ParamOp::Set,
            value: match value {
                Lasso::Int(value) => ParamValue::Int(value),
                Lasso::Float(value) => ParamValue::Float(value as f64),
            },
        });
    }
    params
}

pub(crate) fn loaded_kinds() -> Vec<i32> {
    loaded()
        .lock()
        .map(|packs| packs.iter().map(|pack| pack.kind).collect())
        .unwrap_or_default()
}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
mod live {
    use super::*;
    use crate::{ManifestCss, ManifestRegistration};
    use dionysus_api::{
        CloneArticleRegistrationV1, CloneCopyMeshV1, CloneCopyModelV1, CloneCopyMotionV1,
        CloneGrabRuleV1, CloneRegistrationV1, API_VERSION_V1, COPY_MOTION_NO_EXTRA,
        COPY_MOTION_SET_ANIMATION_UNK, COPY_MOTION_SET_BLEND_FRAMES, COPY_MOTION_SET_CANCEL_FRAME,
        COPY_MOTION_SET_FLAGS, COPY_MOTION_SET_GAME_SCRIPT, COPY_MOTION_SET_NO_STOP_INTP,
        COPY_MOTION_SET_SCRIPTS, COPY_MOTION_SET_XLU, COPY_MOTION_VICTIM, FLAG_KIRBY_COPY_FULL_MODEL,
        FLAG_OWNS_PARAM_RESOURCES, GRAB_CATCH, GRAB_SET_THEN, GRAB_SET_VICTIM, KIND_AUTO,
    };
    use dionysus_core::fighter_toml::{
        parse_all, ArticleDeclaration, KirbyDeclaration, MotionDeclaration,
    };
    use dionysus_core::grab_rule::GrabRule;
    use std::ffi::CString;

    fn leak(text: &str) -> &'static str {
        Box::leak(text.to_string().into_boxed_str())
    }

    fn cstring(directory: &str, what: &str, text: &str) -> Option<CString> {
        match CString::new(text) {
            Ok(value) => Some(value),
            Err(_) => {
                skyline::println!("[fighterpack] {directory}: {what} must not contain NUL");
                None
            }
        }
    }

    pub fn load_all() {
        let Ok(entries) = std::fs::read_dir(MOD_ROOT) else {
            return;
        };
        let mut manifests: Vec<(String, String)> = Vec::new();
        for entry in entries.flatten() {
            let directory = entry.path();
            if !directory.is_dir() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(directory.join("fighter.toml")) else {
                continue;
            };
            if !crate::mod_folders::enabled(&directory) {
                continue;
            }
            manifests.push((
                directory
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                text,
            ));
        }
        manifests.sort_by(|a, b| a.0.cmp(&b.0));
        if manifests.is_empty() {
            return;
        }
        skyline::println!(
            "[fighterpack] {} fighter.toml pack(s) under {MOD_ROOT}",
            manifests.len()
        );
        for (directory, text) in manifests {
            let parsed = match parse_all(&text) {
                Ok(parsed) => parsed,
                Err(error) => {
                    skyline::println!(
                        "[fighterpack] {directory}/fighter.toml line {}: {}; the pack is not registered",
                        error.line,
                        error.message
                    );
                    continue;
                }
            };
            if parsed.len() > 1 {
                skyline::println!("[fighterpack] {directory}/fighter.toml: {} fighters", parsed.len());
            }
            let directory: &'static str = leak(&directory);
            for declaration in parsed {
                register(directory, declaration);
            }
        }
    }

    fn directory_shipping(resource: &str) -> Option<&'static str> {
        let entries = std::fs::read_dir(MOD_ROOT).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.join("fighter").join(resource).is_dir() && crate::mod_folders::enabled(&path) {
                return Some(leak(&entry.file_name().to_string_lossy()));
            }
        }
        None
    }

    pub fn register_text(label: &str, text: &str) -> i32 {
        let parsed = match parse_all(text) {
            Ok(parsed) => parsed,
            Err(error) => {
                skyline::println!(
                    "[fighterpack] {label} (plugin manifest) line {}: {}; not registered",
                    error.line,
                    error.message
                );
                return dionysus_api::ERROR_MANIFEST;
            }
        };
        if parsed.is_empty() {
            return dionysus_api::ERROR_MANIFEST;
        }
        let mut first = dionysus_api::ERROR_MANIFEST;
        for declaration in parsed {
            let directory = directory_shipping(&declaration.resource_name()).unwrap_or_else(|| leak(label));
            if let Some(kind) = register(directory, declaration) {
                if first < 0 {
                    first = kind;
                }
            }
        }
        first
    }

    fn register(directory: &'static str, mut declaration: FighterDeclaration) -> Option<i32> {
        let Some(base_kind) = crate::custom_articles::fighter_kind_from_str(&declaration.base) else {
            skyline::println!(
                "[fighterpack] {directory}: base {:?} is not a fighter name; the pack is not registered",
                declaration.base
            );
            return None;
        };
        let name = declaration.name.clone();
        let (Some(ui_chara), Some(fighter_kind_name), Some(resource_name), Some(base_resource_name)) = (
            cstring(directory, "ui_chara", &declaration.ui_chara()),
            cstring(directory, "fighter_kind_name", &declaration.fighter_kind_name()),
            cstring(directory, "name", &declaration.resource_name()),
            cstring(directory, "base", &declaration.base_resource_name()),
        ) else {
            return None;
        };

        let mut base_article_names: Vec<CString> = Vec::new();
        let mut base_articles: Vec<CloneArticleRegistrationV1> = Vec::new();
        for article in declaration
            .articles
            .iter()
            .filter(|article| article.base_article.is_some())
        {
            let weapon = article.base_article.as_deref().unwrap_or_default();
            let Some(weapon_kind) =
                crate::custom_articles::weapon_kind_from_names(&declaration.base, weapon)
            else {
                skyline::println!(
                    "[fighterpack] {directory}: {} has no article {weapon:?}; the pack is not registered",
                    declaration.base
                );
                return None;
            };
            let Some(file_name) = cstring(directory, "article name", &article.name) else {
                return None;
            };
            base_article_names.push(file_name);
            base_articles.push(CloneArticleRegistrationV1 {
                base_weapon_kind: weapon_kind,
                reserved: 0,
                file_name: base_article_names.last().map(|name| name.as_ptr()).unwrap(),
            });
        }

        let mut flags = 0;
        if declaration.owns_param_resources {
            flags |= FLAG_OWNS_PARAM_RESOURCES;
        }
        let kirby = declaration.kirby.clone();
        if kirby.as_ref().is_some_and(|kirby| kirby.full_model) {
            flags |= FLAG_KIRBY_COPY_FULL_MODEL;
        }
        let (copy_status_first, copy_status_count) = match &kirby {
            Some(kirby) if kirby.statuses > 0 => (kirby.first.unwrap_or(0), kirby.statuses),
            _ => (-1, 0),
        };

        let registration = CloneRegistrationV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneRegistrationV1>() as u32,
            custom_kind: KIND_AUTO,
            base_kind,
            ui_chara: ui_chara.as_ptr(),
            fighter_kind_name: fighter_kind_name.as_ptr(),
            resource_name: resource_name.as_ptr(),
            base_resource_name: base_resource_name.as_ptr(),
            color_start: declaration.color_start,
            color_count: declaration.color_count,
            copy_status_first,
            copy_status_count,
            article_namespace: declaration.article_namespace,
            effect_namespace: declaration.effect_namespace,
            articles: if base_articles.is_empty() {
                core::ptr::null()
            } else {
                base_articles.as_ptr()
            },
            article_count: base_articles.len() as u32,
            flags,
            reserved: [0; 4],
        };
        let manifest = ManifestRegistration {
            css: (!declaration.own_css).then(|| ManifestCss {
                ui_name: leak(&declaration.display_name()),
                ui_series: leak(&declaration.series()),
                disp_order: declaration.disp_order.unwrap_or(-1),
                save_no: declaration.save_no.unwrap_or(0),
                exhibit_year: declaration.exhibit_year.unwrap_or(0),
                narration: declaration.narration.as_deref().map(leak),
            }),
        };
        let result = unsafe { crate::register_v1_with(&registration, Some(manifest)) };
        if result < 0 {
            skyline::println!(
                "[fighterpack] {directory}: {name} REFUSED by the engine (result {result}); the pack is not registered"
            );
            return None;
        }
        let kind = result;
        skyline::println!(
            "[fighterpack] {directory}: {name} is kind {kind} on {} ({base_kind}), costumes {}..{}, {} base article(s){}",
            declaration.base,
            declaration.color_start,
            declaration.color_start + declaration.color_count - 1,
            base_articles.len(),
            if declaration.own_css {
                "; the pack publishes its own CSS row"
            } else {
                ""
            }
        );
        if !declaration.own_css {
            unsafe { crate::css_registration::publish_late_css_slot(kind) };
        }

        for article in declaration
            .articles
            .iter()
            .filter(|article| article.from.is_some())
        {
            mint_article(directory, kind, &declaration, article);
        }
        if let Some(kirby) = &kirby {
            register_kirby(directory, kind, kirby);
        }
        register_global_motions(directory, kind, &declaration.global_motions);
        if declaration.has_partner() {
            crate::entry_groups::register_partner(kind, declaration.partner_with.clone(), declaration.partner_params.clone());
        } else if declaration.partner == Some(false) {
            crate::entry_groups::decline_partner(kind);
        }
        crate::entry_groups::register_switch(kind, declaration.switch_with.clone());
        register_grabs(directory, kind, &declaration.grabs);
        match crate::fight_commands::register(kind, &declaration.command_settings, &declaration.commands) {
            Ok(Some(summary)) => skyline::println!("[fighterpack] {directory}: {name} commands: {summary}"),
            Ok(None) => {}
            Err(message) => skyline::println!("[fighterpack] {directory}: {name} commands REFUSED: {message}"),
        }
        if let Some(donor) = &declaration.hud {
            match crate::hud_grafts::register(kind, &declaration.ui_chara(), donor) {
                Ok(()) => skyline::println!(
                    "[fighterpack] {directory}: {name} borrows the {donor} HUD"
                ),
                Err(message) => skyline::println!(
                    "[fighterpack] {directory}: {name} hud REFUSED: {message}"
                ),
            }
        }
        if let Some(donor) = &declaration.final_smash {
            match crate::final_grafts::register(kind, donor) {
                Ok(_) => {
                    skyline::println!("[fighterpack] {directory}: {name} borrows {donor}'s Final Smash");
                    if crate::mount_events::mounted() {
                        crate::global_motions::install();
                    }
                }
                Err(message) => skyline::println!(
                    "[fighterpack] {directory}: {name} final smash REFUSED: {message}"
                ),
            }
        }
        if let Some(tether) = &declaration.tether {
            match crate::tether::register(kind, tether) {
                Ok(boxes) => skyline::println!(
                    "[fighterpack] {directory}: {name} tethers like {}: air grab on, {boxes} ledge box(es)",
                    tether.from.as_deref().unwrap_or("its own [params]")
                ),
                Err(message) => skyline::println!(
                    "[fighterpack] {directory}: {name} tether REFUSED: {message}"
                ),
            }
        }
        if let Some(choice) = declaration.item_hand {
            match crate::item_hand::register(kind, &declaration.base, choice) {
                Ok(()) => skyline::println!(
                    "[fighterpack] {directory}: {name} holds items in the {} hand",
                    choice.name()
                ),
                Err(message) => skyline::println!(
                    "[fighterpack] {directory}: {name} item_hand REFUSED: {message}"
                ),
            }
        }
        if let Some(list) = &declaration.move_list {
            match crate::move_list::register(&declaration.ui_chara(), list) {
                Ok(pages) => skyline::println!(
                    "[fighterpack] {directory}: {name}'s move list has {pages} extra page(s)"
                ),
                Err(message) => skyline::println!(
                    "[fighterpack] {directory}: {name} move list REFUSED: {message}"
                ),
            }
        }
        if let Some(path) = &declaration.hud_layout {
            match crate::hud_layouts::register(kind, path) {
                Ok(()) => skyline::println!("[fighterpack] {directory}: {name} draws its own HUD layout {path}"),
                Err(message) => skyline::println!(
                    "[fighterpack] {directory}: {name} hud layout REFUSED: {message}"
                ),
            }
        }
        if declaration.jingle.is_some() {
            skyline::println!(
                "[fighterpack] {directory}: jingle is accepted but not served by this engine yet"
            );
        }
        if declaration.staffroll {
            crate::staffroll::install(
                kind,
                directory,
                &declaration.resource_name(),
                &declaration.base_resource_name(),
            );
        }
        let from_files =
            crate::param_files::fighter_params(directory, &declaration.resource_name(), &declaration.params);
        let from_vl = crate::param_files::fighter_vl(directory, &declaration.resource_name(), &declaration.params);
        declaration.params.extend(from_files);
        declaration.params.extend(from_vl);
        crate::cpu_rows::load_file(directory, &declaration.resource_name(), kind);
        if let Ok(mut packs) = loaded().lock() {
            packs.push(LoadedPack {
                directory,
                kind,
                declaration,
            });
        }
        Some(kind)
    }

    fn set_original(directory: &str, minted: i32, article: &ArticleDeclaration) {
        let mut code = article.runs_original_code();
        let kind = article.shows_original_kind();
        if code && kind {
            return;
        }
        if !code && !crate::smashline_articles::numbering_shared() {
            skyline::println!(
                "[fighterpack] {directory}: article {} keeps its source's code: original_code = false needs Smashline 2 on game 13.0.5",
                article.name
            );
            code = true;
        }
        crate::custom_articles::set_original(minted, code, kind);
        skyline::println!(
            "[fighterpack] {directory}: article {} runs {} and shows the game {}",
            article.name,
            if code { "its source's code under its own scripts" } else { "only its own scripts" },
            if kind { "its source's weapon kind" } else { "its own weapon kind" }
        );
    }

    fn mint_article(
        directory: &str,
        kind: i32,
        declaration: &FighterDeclaration,
        article: &ArticleDeclaration,
    ) {
        let Some((owner, weapon)) = article.from.as_ref() else {
            return;
        };
        let Some(weapon_kind) = crate::custom_articles::weapon_kind_from_names(owner, weapon) else {
            skyline::println!(
                "[fighterpack] {directory}: no weapon {owner}/{weapon} for article {}; skipped",
                article.name
            );
            return;
        };
        let resource_owner = if article.kirby {
            "kirby".to_string()
        } else {
            declaration.resource_name()
        };
        let (Some(owner_c), Some(clone_c), Some(name_c)) = (
            cstring(directory, "article owner", owner),
            cstring(directory, "name", &resource_owner),
            cstring(directory, "article name", &article.name),
        ) else {
            return;
        };
        let minted = if article.kirby {
            unsafe {
                crate::custom_articles::register_kirby_copy(
                    kind,
                    owner_c.as_ptr(),
                    weapon_kind,
                    clone_c.as_ptr(),
                    name_c.as_ptr(),
                )
            }
        } else {
            unsafe {
                crate::custom_articles::register(
                    owner_c.as_ptr(),
                    weapon_kind,
                    clone_c.as_ptr(),
                    core::ptr::null(),
                    name_c.as_ptr(),
                )
            }
        };
        if minted < 0 {
            skyline::println!(
                "[fighterpack] {directory}: article {} from {owner}/{weapon} REFUSED (result {minted})",
                article.name
            );
        } else {
            skyline::println!(
                "[fighterpack] {directory}: article {} from {owner}/{weapon} is weapon kind {minted}{}",
                article.name,
                if article.kirby { " (Kirby copy)" } else { "" }
            );
            set_original(directory, minted, article);
            if !article.hurtboxes.is_empty() {
                match crate::article_hurtboxes::register(minted, &article.hurtboxes) {
                    Ok(count) => skyline::println!(
                        "[fighterpack] {directory}: article {} carries {count} hurtbox(es): {}",
                        article.name,
                        article
                            .hurtboxes
                            .iter()
                            .map(|hurtbox| hurtbox.describe())
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                    Err(message) => skyline::println!(
                        "[fighterpack] {directory}: article {} hurtboxes REFUSED: {message}",
                        article.name
                    ),
                }
            }
            if let Some(lvd) = &article.lvd {
                match crate::article_lvds::register(minted, lvd) {
                    Ok(()) => skyline::println!(
                        "[fighterpack] {directory}: article {} carries platform {}",
                        article.name,
                        lvd.describe()
                    ),
                    Err(message) => skyline::println!(
                        "[fighterpack] {directory}: article {} platform REFUSED: {message}",
                        article.name
                    ),
                }
            }
        }
    }

    fn register_kirby(directory: &str, kind: i32, kirby: &KirbyDeclaration) {
        if let Some(model) = &kirby.model {
            if let Some(model_c) = cstring(directory, "kirby model", model) {
                let registration = CloneCopyModelV1 {
                    api_version: API_VERSION_V1,
                    struct_size: core::mem::size_of::<CloneCopyModelV1>() as u32,
                    fighter_kind: kind,
                    reserved0: 0,
                    directory: model_c.as_ptr(),
                    reserved: [0; 4],
                };
                let result =
                    unsafe { crate::kirby_copy::clone_engine_clone_copy_model_v1(&registration) };
                skyline::println!("[fighterpack] {directory}: kirby model {model} result={result}");
            }
        }
        for (mesh, visible) in &kirby.meshes {
            let Some(mesh_c) = cstring(directory, "kirby mesh", mesh) else {
                continue;
            };
            let registration = CloneCopyMeshV1 {
                api_version: API_VERSION_V1,
                struct_size: core::mem::size_of::<CloneCopyMeshV1>() as u32,
                fighter_kind: kind,
                visible: u32::from(*visible),
                mesh: mesh_c.as_ptr(),
                reserved: [0; 4],
            };
            let result =
                unsafe { crate::kirby_copy::clone_engine_clone_copy_mesh_default_v1(&registration) };
            skyline::println!("[fighterpack] {directory}: kirby mesh {mesh} result={result}");
        }
        for motion in &kirby.motions {
            if let Some(result) = submit_motion(
                directory,
                kind,
                "kirby motion",
                motion,
                crate::kirby_motions::clone_engine_clone_copy_motion_v1,
            ) {
                skyline::println!(
                    "[fighterpack] {directory}: kirby motion {} result={result}",
                    motion.name
                );
            }
        }
    }

    fn register_global_motions(directory: &str, kind: i32, motions: &[MotionDeclaration]) {
        for motion in motions {
            if let Some(result) = submit_motion(
                directory,
                kind,
                "global motion",
                motion,
                crate::global_motions::clone_engine_global_motion_v1,
            ) {
                skyline::println!(
                    "[fighterpack] {directory}: global motion {} result={result}",
                    motion.name
                );
            }
        }
    }

    fn register_grabs(directory: &str, kind: i32, grabs: &[GrabRule]) {
        for grab in grabs {
            let node = match grab.node.as_deref() {
                Some(node) => match cstring(directory, "grab node", node) {
                    Some(node) => Some(node),
                    None => continue,
                },
                None => None,
            };
            let mut present = 0;
            if grab.victim.is_some() {
                present |= GRAB_SET_VICTIM;
            }
            if grab.then.is_some() {
                present |= GRAB_SET_THEN;
            }
            if grab.catch {
                present |= GRAB_CATCH;
            }
            let registration = CloneGrabRuleV1 {
                api_version: API_VERSION_V1,
                struct_size: core::mem::size_of::<CloneGrabRuleV1>() as u32,
                fighter_kind: kind,
                present,
                status: grab.status,
                victim: grab.victim.unwrap_or(-1),
                then: grab.then.unwrap_or(-1),
                reserved0: 0,
                node: node.as_ref().map_or(core::ptr::null(), |node| node.as_ptr()),
                offset: grab.offset,
                reserved1: 0,
                reserved: [0; 4],
            };
            let result = unsafe { crate::grab_rules::clone_engine_grab_rule_v1(&registration) };
            if result < 0 {
                skyline::println!("[fighterpack] {directory}: grab {} result={result}", grab.describe());
            }
        }
    }

    fn submit_motion(
        directory: &str,
        kind: i32,
        what: &str,
        motion: &MotionDeclaration,
        submit: unsafe extern "C" fn(*const CloneCopyMotionV1) -> i32,
    ) -> Option<i32> {
        let owned = |value: &str| cstring(directory, what, value);
        let (Some(name), Some(animation)) = (owned(&motion.name), owned(&motion.animation)) else {
            return None;
        };
        let template = motion.template.as_deref().and_then(owned);
        let game = motion.script("game", None).as_deref().and_then(owned);
        let sound = motion.script("sound", None).as_deref().and_then(owned);
        let effect = motion.script("effect", None).as_deref().and_then(owned);
        let expression = motion.script("expression", None).as_deref().and_then(owned);
        let mut present = 0u32;
        let mut motion_flags = 0u16;
        if game.is_some() {
            present |= COPY_MOTION_SET_GAME_SCRIPT;
        }
        if sound.is_some() && effect.is_some() && expression.is_some() {
            present |= COPY_MOTION_SET_SCRIPTS;
        }
        if !motion.flags.is_empty() {
            present |= COPY_MOTION_SET_FLAGS;
            for flag in &motion.flags {
                motion_flags |= match flag.as_str() {
                    "turn" => 1 << 0,
                    "loop" => 1 << 1,
                    "move" => 1 << 2,
                    "fix_trans" => 1 << 3,
                    "fix_rot" => 1 << 4,
                    "fix_scale" => 1 << 5,
                    other => {
                        skyline::println!(
                            "[fighterpack] {directory}: {what} {} has unknown flag {other:?}",
                            motion.name
                        );
                        0
                    }
                };
            }
        }
        if motion.blend_frames.is_some() {
            present |= COPY_MOTION_SET_BLEND_FRAMES;
        }
        if motion.xlu.is_some() {
            present |= COPY_MOTION_SET_XLU;
        }
        if motion.cancel_frame.is_some() {
            present |= COPY_MOTION_SET_CANCEL_FRAME;
        }
        if motion.no_stop_intp.is_some() {
            present |= COPY_MOTION_SET_NO_STOP_INTP;
        }
        if motion.animation_unk.is_some() {
            present |= COPY_MOTION_SET_ANIMATION_UNK;
        }
        if motion.no_extra {
            present |= COPY_MOTION_NO_EXTRA;
        }
        if motion.victim {
            present |= COPY_MOTION_VICTIM;
        }
        let null = core::ptr::null();
        let pointer = |value: &Option<CString>| value.as_ref().map_or(null, |value| value.as_ptr());
        let (xlu_start, xlu_end) = motion.xlu.unwrap_or((0, 0));
        let registration = CloneCopyMotionV1 {
            api_version: API_VERSION_V1,
            struct_size: core::mem::size_of::<CloneCopyMotionV1>() as u32,
            fighter_kind: kind,
            present,
            name: name.as_ptr(),
            template: pointer(&template),
            animation: animation.as_ptr(),
            game_script: pointer(&game),
            script_expression: pointer(&expression),
            script_sound: pointer(&sound),
            script_effect: pointer(&effect),
            motion_flags,
            blend_frames: motion.blend_frames.unwrap_or(0),
            xlu_start,
            xlu_end,
            cancel_frame: motion.cancel_frame.unwrap_or(0),
            no_stop_intp: u8::from(motion.no_stop_intp.unwrap_or(false)),
            animation_unk: motion.animation_unk.unwrap_or(0),
            reserved: [0; 4],
        };
        Some(unsafe { submit(&registration) })
    }

    pub fn apply_params() {
        for kind in loaded_kinds() {
            let params = params_for_kind(kind);
            if params.is_empty() {
                continue;
            }
            if !crate::param_overrides::available() {
                skyline::println!(
                    "[fighterpack] kind {kind}: {} [params] entries need ParamConfig, which is not loaded",
                    params.len()
                );
                continue;
            }
            let mut applied = 0;
            for param in &params {
                let (name, sub) = match param.name.split_once('.') {
                    Some((name, sub)) => (name, dionysus_core::prcxml::key(sub)),
                    None => (param.name.as_str(), 0),
                };
                let slot = param.slot.unwrap_or(crate::param_overrides::ANY_SLOT);
                let result = match (&param.value, param.op) {
                    (dionysus_core::fighter_toml::ParamValue::Int(value), _) => {
                        crate::clone_engine_param_int_override_v1(
                            kind,
                            slot,
                            dionysus_core::prcxml::key(name),
                            sub,
                            *value,
                        )
                    }
                    (dionysus_core::fighter_toml::ParamValue::Float(value), op) => {
                        crate::clone_engine_param_override_v1(
                            kind,
                            slot,
                            dionysus_core::prcxml::key(name),
                            sub,
                            match op {
                                dionysus_core::fighter_toml::ParamOp::Set => {
                                    crate::param_overrides::OP_SET
                                }
                                dionysus_core::fighter_toml::ParamOp::Mul => {
                                    crate::param_overrides::OP_MUL
                                }
                            },
                            *value,
                        )
                    }
                };
                if result == 0 {
                    applied += 1;
                } else {
                    skyline::println!(
                        "[fighterpack] kind {kind}: [params] {} was declined (result {result})",
                        param.name
                    );
                }
            }
            skyline::println!("[fighterpack] kind {kind}: {applied} of {} [params] applied", params.len());
        }
    }
}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
pub(crate) fn load_all() {
    live::load_all();
}

#[cfg(not(all(not(test), feature = "native_relocate", feature = "css_slot")))]
pub(crate) fn load_all() {}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
pub(crate) fn apply_params() {
    live::apply_params();
}

#[cfg(all(not(test), feature = "native_relocate", feature = "css_slot"))]
pub(crate) fn register_text(label: &str, text: &str) -> i32 {
    live::register_text(label, text)
}

#[cfg(not(all(not(test), feature = "native_relocate", feature = "css_slot")))]
pub(crate) fn register_text(_label: &str, _text: &str) -> i32 {
    dionysus_api::ERROR_UNSUPPORTED
}

#[cfg(not(all(not(test), feature = "native_relocate", feature = "css_slot")))]
pub(crate) fn apply_params() {}
