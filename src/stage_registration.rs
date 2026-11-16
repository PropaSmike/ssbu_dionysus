#[cfg(test)]
#[path = "stage_ledger.rs"]
mod stage_ledger;
#[cfg(not(test))]
use crate::stage_ledger::{hash40, CloneStage, StageResources};
use crate::stage_music::Music;
#[cfg(test)]
use stage_ledger::{hash40, CloneStage, StageResources};

#[derive(Debug, PartialEq, Eq)]
pub enum RegistrationError {
    DispOrderTooLarge(i32),
    NotSelectableAndNotHidden,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDbRequest {
    pub ui_stage_id: u64,
    pub name_id: String,
    pub stage_place_id: u64,
    pub secret_stage_place_id: u64,
    pub ui_series_id: u64,
    pub disp_order: i8,
    pub can_select: bool,
    pub is_dlc: bool,
    pub save_no: i16,
    pub bgm_set_id: u64,
    pub bgm_setting_no: u8,
    pub bgm_selector: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub db: StageDbRequest,
    pub asset_place: String,
    pub place_hash: u64,
    pub stage_hash: u64,
    pub normal: StageResources,
    pub end: StageResources,
    pub battle: StageResources,
    pub deferred_disp_order: Option<u16>,
}

pub fn plan(
    stage: &CloneStage,
    ui_series_id: u64,
    disp_order: i32,
    music: &Music,
) -> Result<Registration, RegistrationError> {
    if disp_order > u8::MAX as i32 {
        return Err(RegistrationError::DispOrderTooLarge(disp_order));
    }
    if disp_order < -1 {
        return Err(RegistrationError::NotSelectableAndNotHidden);
    }
    let deferred = (disp_order > i8::MAX as i32).then_some(disp_order as u16);
    let registered_order = if deferred.is_some() { -1 } else { disp_order };

    let place_hash = hash40(&stage.place_name);
    let stage_hash = hash40(&format!("ui_stage_{}", stage.place_name));
    let resources = stage.resource_set();

    Ok(Registration {
        db: StageDbRequest {
            ui_stage_id: stage_hash,
            name_id: stage.name_id().to_string(),
            stage_place_id: place_hash,
            secret_stage_place_id: place_hash,
            ui_series_id,
            disp_order: registered_order as i8,
            can_select: disp_order >= 0,
            is_dlc: stage.distribution == 1,
            save_no: -1,
            bgm_set_id: music.set_id,
            bgm_setting_no: music.setting_no,
            bgm_selector: music.selector,
        },
        asset_place: stage.asset_place().to_string(),
        place_hash,
        stage_hash,
        normal: resources.normal,
        end: resources.end,
        battle: resources.battle,
        deferred_disp_order: deferred,
    })
}

#[cfg(feature = "stage_slot")]
fn ui_stage_data(registration: &Registration) -> the_csk_collection_api::UiStageData {
    use the_csk_collection_api::{UiStageData, UiStageResources};

    fn resources(source: &StageResources) -> UiStageResources {
        UiStageResources {
            stage_load_group_hash: source.stage_load_group_hash,
            effect_load_group_hash: source.effect_load_group_hash,
            nus3bank_path_hash: source.nus3bank_path_hash,
            sqb_path_hash: source.sqb_path_hash,
            nus3audio_path_hash: source.nus3audio_path_hash,
            tonelabel_path_hash: source.tonelabel_path_hash,
        }
    }

    UiStageData {
        normal: resources(&registration.normal),
        end: resources(&registration.end),
        battle: resources(&registration.battle),
    }
}

#[cfg(feature = "stage_slot")]
struct Secret {
    icon: u64,
    icon_name: String,
    place_hash: u64,
    command_hash: u64,
}

#[cfg(feature = "stage_slot")]
static SECRETS: std::sync::Mutex<Vec<Secret>> = std::sync::Mutex::new(Vec::new());

#[cfg(feature = "stage_slot")]
static ROWS: std::sync::Mutex<Vec<Registration>> = std::sync::Mutex::new(Vec::new());

#[cfg(feature = "stage_slot")]
pub fn register_secret(registration: &Registration, base: &str, command: &str) {
    let Some(secret) = secret(registration, base, command) else {
        return;
    };
    let Ok(mut secrets) = SECRETS.lock() else {
        return;
    };
    if let Some(earlier) = secrets.iter().find(|earlier| earlier.icon == secret.icon) {
        skyline::println!(
            "[stagereg] {}: place {:#x} takes this icon's secret from place {:#x}; the last pack wins",
            secret.icon_name,
            secret.place_hash,
            earlier.place_hash
        );
    }
    let icon_row = ROWS
        .lock()
        .ok()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.stage_hash == secret.icon)
                .map(|row| stage_entry(row, Some(&secret)))
        });
    match icon_row {
        Some(entry) => the_csk_collection_api::add_stage_db_entry(&entry),
        None => register_vanilla_secret(&secret),
    }
    secrets.push(secret);
}

#[cfg(feature = "stage_slot")]
fn secret(registration: &Registration, base: &str, command: &str) -> Option<Secret> {
    use dionysus_core::stage_secret;

    let Some(command_name) = stage_secret::command_name(command) else {
        skyline::println!("[stagereg] secret_of {base}: command {command:?} is not l, r or l_r; refused");
        return None;
    };
    let icon_name = stage_secret::ui_stage_name(base);
    let icon = hash40(&icon_name);
    if icon == registration.stage_hash {
        skyline::println!("[stagereg] secret_of {base}: a stage cannot be its own secret; refused");
        return None;
    }
    if let Some((place, existing)) = stage_secret::existing_secret(base) {
        skyline::println!(
            "[stagereg] secret_of {base}: replaces its own secret ({place} on {existing})"
        );
    }
    skyline::println!(
        "[stagereg] {icon_name}: holding {command} on its icon plays place {:#x}",
        registration.place_hash
    );
    Some(Secret {
        icon,
        icon_name,
        place_hash: registration.place_hash,
        command_hash: hash40(command_name),
    })
}

#[cfg(feature = "stage_slot")]
fn register_vanilla_secret(secret: &Secret) {
    use the_csk_collection_api::{Hash40Type, StageDatabaseEntry};

    let entry = StageDatabaseEntry {
        ui_stage_id: secret.icon,
        secret_stage_place_id: Hash40Type::Overwrite(secret.place_hash),
        secret_command_id: Hash40Type::Overwrite(secret.command_hash),
        secret_command_id_joycon: Hash40Type::Overwrite(secret.command_hash),
        ..Default::default()
    };
    the_csk_collection_api::add_stage_db_entry(&entry);
}

#[cfg(feature = "stage_slot")]
fn stage_entry(
    registration: &Registration,
    secret: Option<&Secret>,
) -> the_csk_collection_api::StageDatabaseEntry {
    use the_csk_collection_api::{
        BoolType, CStrCSK, Hash40Map, Hash40Type, ShortType, SignedByteType, StageDatabaseEntry,
        StringType, UnsignedByteType,
    };

    StageDatabaseEntry {
        ui_stage_id: registration.db.ui_stage_id,
        clone_from_ui_stage_id: None,
        name_id: StringType::Overwrite(CStrCSK::new(&registration.db.name_id)),
        save_no: ShortType::Overwrite(registration.db.save_no),
        ui_series_id: Hash40Type::Overwrite(registration.db.ui_series_id),
        can_select: BoolType::Overwrite(registration.db.can_select),
        disp_order: SignedByteType::Overwrite(registration.db.disp_order),
        stage_place_id: Hash40Type::Overwrite(registration.db.stage_place_id),
        secret_stage_place_id: Hash40Type::Overwrite(
            secret.map_or(registration.db.secret_stage_place_id, |secret| secret.place_hash),
        ),
        can_demo: BoolType::Overwrite(false),
        is_8player_stage: BoolType::Overwrite(false),
        is_usable_flag: BoolType::Overwrite(true),
        is_usable_amiibo: BoolType::Overwrite(true),
        secret_command_id: Hash40Type::Overwrite(secret.map_or(0, |secret| secret.command_hash)),
        secret_command_id_joycon: Hash40Type::Overwrite(
            secret.map_or(0, |secret| secret.command_hash),
        ),
        bgm_set_id: Hash40Type::Overwrite(registration.db.bgm_set_id),
        bgm_setting_no: UnsignedByteType::Overwrite(registration.db.bgm_setting_no),
        bgm_selector: BoolType::Overwrite(registration.db.bgm_selector),
        is_dlc: BoolType::Overwrite(registration.db.is_dlc),
        is_patch: BoolType::Overwrite(crate::ui_pictures::stage_reads_patch_tree(
            &registration.db.name_id,
        )),
        dlc_chara_id: Hash40Type::Overwrite(0),
        extra_hash_maps: Hash40Map::default(),
    }
}

#[cfg(feature = "stage_slot")]
pub fn register(registration: Registration) {
    if crate::ui_pictures::stage_reads_patch_tree(&registration.db.name_id) {
        skyline::println!(
            "[stagereg] {}: stage-select icons only under ui/replace_patch/stage, so its row reads that tree",
            registration.db.name_id
        );
    }
    let entry = {
        let secrets = SECRETS.lock().ok();
        let secret = secrets.as_ref().and_then(|secrets| {
            secrets
                .iter()
                .rev()
                .find(|secret| secret.icon == registration.stage_hash)
        });
        stage_entry(&registration, secret)
    };

    let data = ui_stage_data(&registration);

    the_csk_collection_api::add_ui_stage_db_resources_entry(
        registration.place_hash,
        registration.stage_hash,
        &data,
    );
    the_csk_collection_api::add_stage_db_entry(&entry);

    crate::stage_sound::schedule(&registration.asset_place);
    crate::stage_alts_bridge::arm();

    skyline::println!(
        "[stagereg] registered {:#x} (place {:#x}) with CSK, disp_order {}, bgm {:#x} column {}{}",
        registration.stage_hash,
        registration.place_hash,
        registration.db.disp_order,
        registration.db.bgm_set_id,
        registration.db.bgm_setting_no,
        if registration.db.bgm_selector {
            ", album selector"
        } else {
            ""
        }
    );
    if let Ok(mut rows) = ROWS.lock() {
        rows.push(registration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage_music::{resolve, MusicRequest};
    use stage_ledger::Form;

    fn declared(name: &str, battle_tree: bool) -> CloneStage {
        let mut stage = CloneStage::new(name);
        stage.ships_battle_tree = battle_tree;
        stage.forms = vec![Form::Normal, Form::Omega, Form::Battlefield];
        stage
    }

    #[test]
    fn carries_the_hashes_the_image_proved() {
        let stage = declared("photostage", false);
        let registration = plan(&stage, 0, 0, &Music::default()).unwrap();
        assert_eq!(
            registration.normal.effect_load_group_hash,
            hash40("effect/stage/photostage")
        );
        assert_eq!(registration.place_hash, hash40("photostage"));
        assert_eq!(registration.stage_hash, hash40("ui_stage_photostage"));
    }

    #[test]
    fn a_value_past_csk_registers_hidden_and_defers() {
        let stage = declared("pumpkin_hill", false);
        let deferred = plan(&stage, 0, 200, &Music::default()).unwrap();
        assert_eq!(deferred.db.disp_order, -1);
        assert_eq!(deferred.deferred_disp_order, Some(200));
        assert!(deferred.db.can_select);
    }

    #[test]
    fn a_value_csk_can_carry_is_not_deferred() {
        let stage = declared("pumpkin_hill", false);
        let direct = plan(&stage, 0, 127, &Music::default()).unwrap();
        assert_eq!(direct.db.disp_order, 127);
        assert_eq!(direct.deferred_disp_order, None);
    }

    #[test]
    fn refuses_past_what_any_backend_can_express() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(
            plan(&stage, 0, 256, &Music::default()),
            Err(RegistrationError::DispOrderTooLarge(256))
        );
    }

    #[test]
    fn minus_one_hides_the_stage_and_clears_can_select() {
        let stage = declared("pumpkin_hill", false);
        let hidden = plan(&stage, 0, -1, &Music::default()).unwrap();
        assert_eq!(hidden.db.disp_order, -1);
        assert!(!hidden.db.can_select);
        let shown = plan(&stage, 0, 0, &Music::default()).unwrap();
        assert!(shown.db.can_select);
    }

    #[test]
    fn refuses_a_negative_that_is_not_the_hidden_sentinel() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(
            plan(&stage, 0, -7, &Music::default()),
            Err(RegistrationError::NotSelectableAndNotHidden)
        );
    }

    #[test]
    fn the_resolved_music_reaches_the_row() {
        let stage = declared("pumpkin_hill", false);
        let music = resolve(
            &MusicRequest {
                set: Some("sonic".to_string()),
                setting_no: Some(1),
                selector: Some(true),
            },
            None,
        )
        .unwrap()
        .music;
        let registration = plan(&stage, 0, 0, &music).unwrap();
        assert_eq!(
            registration.db.bgm_set_id,
            crate::stage_ledger::hash40("bgmsonic")
        );
        assert_eq!(registration.db.bgm_setting_no, 1);
        assert!(registration.db.bgm_selector);
    }

    #[test]
    fn an_unset_music_row_stays_the_silent_default() {
        let stage = declared("pumpkin_hill", false);
        let registration = plan(&stage, 0, 0, &Music::default()).unwrap();
        assert_eq!(registration.db.bgm_set_id, 0);
        assert_eq!(registration.db.bgm_setting_no, 0);
        assert!(!registration.db.bgm_selector);
    }

    #[test]
    fn a_minted_stage_never_claims_a_save_slot() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(plan(&stage, 0, 0, &Music::default()).unwrap().db.save_no, -1);
    }

    #[test]
    fn the_label_is_the_enum_name_not_the_place_name() {
        let mut stage = declared("template_stage", false);
        stage.ui_name_id = Some("TemplateStage".into());
        assert_eq!(plan(&stage, 0, 118, &Music::default()).unwrap().db.name_id, "TemplateStage");
    }

    #[test]
    fn an_unset_label_still_falls_back_to_the_place_name() {
        let stage = declared("pumpkin_hill", false);
        assert_eq!(plan(&stage, 0, 0, &Music::default()).unwrap().db.name_id, "pumpkin_hill");
    }

    #[test]
    fn the_battle_tree_reaches_the_registration() {
        let stage = declared("pumpkin_hill", true);
        let registration = plan(&stage, 0, 0, &Music::default()).unwrap();
        assert_eq!(
            registration.end.stage_load_group_hash,
            hash40("stage/pumpkin_hill/battle")
        );
        assert_ne!(
            registration.normal.stage_load_group_hash,
            registration.end.stage_load_group_hash
        );
    }
}

#[cfg(not(test))]
pub fn resolve_music(
    place_name: &str,
    request: &crate::stage_music::MusicRequest,
    donor_place: Option<&str>,
) -> Option<Music> {
    match crate::stage_music::resolve(request, donor_place) {
        Ok(resolution) => {
            for note in &resolution.notes {
                skyline::println!("[stagemusic] {place_name}: {note}");
            }
            Some(resolution.music)
        }
        Err(crate::stage_music::MusicError::ColumnOutOfRange(value)) => {
            skyline::println!(
                "[stagemusic] refused {place_name}: bgm_setting_no {value} is outside 0..15"
            );
            None
        }
    }
}

#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn clone_engine_register_stage(
    place_name: *const core::ffi::c_char,
    name_id: *const core::ffi::c_char,
    ships_battle_tree: bool,
    ui_series_id: u64,
    disp_order: i32,
) -> i32 {
    if place_name.is_null() {
        return -1;
    }
    let name = match core::ffi::CStr::from_ptr(place_name).to_str() {
        Ok(name) if !name.is_empty() => name,
        _ => return -2,
    };

    let mut stage = CloneStage::new(name);
    stage.ships_battle_tree = ships_battle_tree;
    if !name_id.is_null() {
        match core::ffi::CStr::from_ptr(name_id).to_str() {
            Ok(label) if !label.is_empty() => stage.ui_name_id = Some(label.to_string()),
            _ => return -2,
        }
    }

    let mut request = crate::stage_music::MusicRequest::default();
    let mut donor = stage.place_name.clone();
    if let Ok(registry) = crate::stage_registry::registry().lock() {
        if let Some(minted) = registry.by_name(&stage.place_name) {
            if minted.asset_place != stage.place_name {
                stage.resource_place = Some(minted.asset_place.clone());
                skyline::println!(
                    "[stagereg] {name} registers with assets from {}, as minted",
                    minted.asset_place
                );
            }
            request = minted.music.clone();
            donor = minted.behaviour_place.clone();
        }
    }
    let music = match resolve_music(name, &request, Some(&donor)) {
        Some(music) => music,
        None => return -6,
    };

    let claimed = crate::stage_registry::registry()
        .lock()
        .map(|mut registry| registry.claim_row(&stage.place_name))
        .unwrap_or(false);
    if !claimed {
        skyline::println!("[stagereg] {name} already has a grid row; not registering a second");
        return 0;
    }

    match plan(&stage, ui_series_id, disp_order, &music) {
        Ok(_registration) => {
            skyline::println!(
                "[stagereg] {name} label will be nam_stg1_{}",
                stage.name_id()
            );
            #[cfg(feature = "stage_slot")]
            {
                if let Some(wanted) = _registration.deferred_disp_order {
                    crate::stage_db_rows::request_disp_order(_registration.stage_hash, wanted);
                    skyline::println!(
                        "[stagereg] {name} registered hidden; disp_order {wanted} queued for the                          row backend (CSK's SignedByteType stops at 127)"
                    );
                }
                register(_registration);
                0
            }
            #[cfg(not(feature = "stage_slot"))]
            {
                skyline::println!(
                    "[stagereg] refused {name}: this engine build has no stage backend \
                     (rebuild with --features stage_slot)"
                );
                -3
            }
        }
        Err(RegistrationError::DispOrderTooLarge(value)) => {
            skyline::println!(
                "[stagereg] refused {name}: disp_order {value} exceeds 127, which is all \
                 CSK's SignedByteType can carry. Author the row into ui_stage_db.prc with \
                 tools/stage_disp_order.py instead."
            );
            -4
        }
        Err(RegistrationError::NotSelectableAndNotHidden) => {
            skyline::println!(
                "[stagereg] refused {name}: disp_order {disp_order} is negative but not the \
                 -1 hidden sentinel"
            );
            -5
        }
    }
}
