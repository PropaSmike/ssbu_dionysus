#![allow(dead_code)]

pub const VANILLA_PLACES: u32 = 138;
pub const VANILLA_STAGE_IDS: u32 = 364;
pub const PLACE_ROW: usize = 0x28;
pub const PLACE_AUX_ROW: usize = 0x20;
pub const STAGE_ID_ROW: usize = 0x48;

#[cfg(not(feature = "diag_low_place"))]
pub const FIRST_MINTED_PLACE: u32 = VANILLA_PLACES;
#[cfg(not(feature = "diag_low_stage_id"))]
pub const FIRST_MINTED_STAGE_ID: u32 = VANILLA_STAGE_IDS;

#[cfg(feature = "diag_low_stage_id")]
pub const FIRST_MINTED_STAGE_ID: u32 = 326;

#[cfg(feature = "diag_low_place")]
pub const FIRST_MINTED_PLACE: u32 = 122;

pub fn hash40(text: &str) -> u64 {
    let lower = text.to_ascii_lowercase();
    ((lower.len() as u64) << 32) | crc32(lower.as_bytes()) as u64
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Normal = 0,
    Omega = 1,
    Battlefield = 2,
}

impl Form {
    fn prefix(self) -> &'static str {
        match self {
            Form::Normal => "",
            Form::Omega => "end_",
            Form::Battlefield => "battle_",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CloneStage {
    pub place_name: String,
    pub ships_battle_tree: bool,
    pub normal_in_battle_tree: bool,
    pub distribution: u64,
    pub dlc_owner: u64,
    pub resource_place: Option<String>,
    pub ui_name_id: Option<String>,
    pub aux_template: Option<u32>,
    pub forms: Vec<Form>,
}

impl CloneStage {
    pub fn new(place_name: &str) -> Self {
        Self {
            place_name: place_name.to_ascii_lowercase(),
            resource_place: None,
            ui_name_id: None,
            ships_battle_tree: false,
            normal_in_battle_tree: false,
            distribution: 2,
            dlc_owner: 0,
            aux_template: None,
            forms: vec![Form::Normal],
        }
    }

    pub fn name_id(&self) -> &str {
        self.ui_name_id.as_deref().unwrap_or(&self.place_name)
    }

    pub fn asset_place(&self) -> &str {
        self.resource_place.as_deref().unwrap_or(&self.place_name)
    }

    pub fn tree(&self, form: Form) -> &'static str {
        if self.ships_battle_tree && (form != Form::Normal || self.normal_in_battle_tree) {
            "battle"
        } else {
            "normal"
        }
    }

    pub fn root_path(&self, form: Form) -> String {
        format!("stage/{}/{}", self.asset_place(), self.tree(form))
    }

    pub fn stage_id_hashes(&self, form: Form) -> [u64; 7] {
        let place = self.asset_place();
        [
            hash40(&format!("{}{}", form.prefix(), place)),
            hash40(&self.root_path(form)),
            hash40(&format!("effect/stage/{place}")),
            hash40(&format!("sound/bank/stage/se_stage_{place}.nus3bank")),
            hash40(&format!("sound/sequence/stage/{place}.sqb")),
            hash40(&format!("sound/bank/stage/se_stage_{place}.nus3audio")),
            hash40(&format!("sound/bank/stage/se_stage_{place}.tonelabel")),
        ]
    }

    pub fn resources(&self, form: Form) -> StageResources {
        let hashes = self.stage_id_hashes(form);
        StageResources {
            stage_load_group_hash: hashes[1],
            effect_load_group_hash: hashes[2],
            nus3bank_path_hash: hashes[3],
            sqb_path_hash: hashes[4],
            nus3audio_path_hash: hashes[5],
            tonelabel_path_hash: hashes[6],
        }
    }

    pub fn resource_set(&self) -> StageResourceSet {
        StageResourceSet {
            normal: self.resources(Form::Normal),
            end: self.resources(Form::Omega),
            battle: self.resources(Form::Battlefield),
        }
    }

    pub fn stage_id_row(&self, stage_id: u32, place: u32, form: Form) -> [u8; STAGE_ID_ROW] {
        let mut row = [0u8; STAGE_ID_ROW];
        row[0x00..0x04].copy_from_slice(&stage_id.to_le_bytes());
        row[0x04..0x08].copy_from_slice(&place.to_le_bytes());
        row[0x08..0x10].copy_from_slice(&(form as u64).to_le_bytes());
        for (i, hash) in self.stage_id_hashes(form).iter().enumerate() {
            let at = 0x10 + i * 8;
            row[at..at + 8].copy_from_slice(&hash.to_le_bytes());
        }
        row
    }

    pub fn place_row(&self, place: u32, system: bool) -> [u8; PLACE_ROW] {
        let mut row = [0u8; PLACE_ROW];
        let name = hash40(&self.place_name);
        row[0x00..0x04].copy_from_slice(&place.to_le_bytes());
        row[0x04..0x08].copy_from_slice(&(system as u32).to_le_bytes());
        row[0x08..0x10].copy_from_slice(&name.to_le_bytes());
        row[0x10..0x18].copy_from_slice(&name.to_le_bytes());
        row[0x18..0x20].copy_from_slice(&self.distribution.to_le_bytes());
        row[0x20..0x28].copy_from_slice(&self.dlc_owner.to_le_bytes());
        row
    }

    pub fn place_aux_row(
        &self,
        place: u32,
        template: Option<&[u8; PLACE_AUX_ROW]>,
    ) -> [u8; PLACE_AUX_ROW] {
        let mut row = template.copied().unwrap_or([0u8; PLACE_AUX_ROW]);
        row[0x00..0x04].copy_from_slice(&place.to_le_bytes());
        row
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StageResources {
    pub stage_load_group_hash: u64,
    pub effect_load_group_hash: u64,
    pub nus3bank_path_hash: u64,
    pub sqb_path_hash: u64,
    pub nus3audio_path_hash: u64,
    pub tonelabel_path_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StageResourceSet {
    pub normal: StageResources,
    pub end: StageResources,
    pub battle: StageResources,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintedStageId {
    pub stage_id: u32,
    pub place: u32,
    pub form: Form,
    pub row: [u8; STAGE_ID_ROW],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub place_name: String,
    pub place: u32,
    pub place_row: [u8; PLACE_ROW],
    pub place_aux_row: [u8; PLACE_AUX_ROW],
    pub stage_ids: Vec<MintedStageId>,
}

#[derive(Debug)]
pub struct StageAllocator {
    next_place: u32,
    next_stage_id: u32,
    place_capacity: u32,
    stage_id_capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationError {
    OutOfPlaces { capacity: u32 },
    OutOfStageIds { capacity: u32, wanted: usize },
    NoForms,
    DuplicateForm(&'static str),
    EmptyName,
}

impl StageAllocator {
    pub fn new(place_capacity: u32, stage_id_capacity: u32) -> Self {
        Self {
            next_place: FIRST_MINTED_PLACE,
            next_stage_id: FIRST_MINTED_STAGE_ID,
            place_capacity,
            stage_id_capacity,
        }
    }

    pub fn places_remaining(&self) -> u32 {
        self.place_capacity.saturating_sub(self.next_place)
    }

    pub fn stage_ids_remaining(&self) -> u32 {
        self.stage_id_capacity.saturating_sub(self.next_stage_id)
    }

    pub fn allocate(&mut self, stage: &CloneStage) -> Result<InstallPlan, AllocationError> {
        if stage.place_name.is_empty() {
            return Err(AllocationError::EmptyName);
        }
        if stage.forms.is_empty() {
            return Err(AllocationError::NoForms);
        }
        for (i, form) in stage.forms.iter().enumerate() {
            if stage.forms[..i].contains(form) {
                return Err(AllocationError::DuplicateForm(match form {
                    Form::Normal => "Normal",
                    Form::Omega => "Omega",
                    Form::Battlefield => "Battlefield",
                }));
            }
        }
        if self.places_remaining() == 0 {
            return Err(AllocationError::OutOfPlaces {
                capacity: self.place_capacity,
            });
        }
        if (self.stage_ids_remaining() as usize) < stage.forms.len() {
            return Err(AllocationError::OutOfStageIds {
                capacity: self.stage_id_capacity,
                wanted: stage.forms.len(),
            });
        }

        let place = self.next_place;
        let mut stage_ids = Vec::with_capacity(stage.forms.len());
        for (i, &form) in stage.forms.iter().enumerate() {
            let stage_id = self.next_stage_id + i as u32;
            stage_ids.push(MintedStageId {
                stage_id,
                place,
                form,
                row: stage.stage_id_row(stage_id, place, form),
            });
        }
        self.next_place += 1;
        self.next_stage_id += stage.forms.len() as u32;

        Ok(InstallPlan {
            place_name: stage.place_name.clone(),
            place,
            place_row: stage.place_row(place, false),
            place_aux_row: stage.place_aux_row(place, None),
            stage_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHOTOSTAGE: u64 = 0x0000_000A_1E68_5186;
    const PHOTOSTAGE_ROW_326: [u64; 7] = [
        0x0000_000A_1E68_5186,
        0x0000_0017_B1B2_E674,
        0x0000_0017_ED8A_D1B2,
        0x0000_002D_4816_F51D,
        0x0000_0023_790D_AFD0,
        0x0000_002E_CB3B_B424,
        0x0000_002E_D5AA_21B1,
    ];
    const CASTLE64_ROW_6: [u64; 7] = [
        0x0000_0012_8222_4831,
        0x0000_001B_049A_3B7D,
        0x0000_001B_E493_5885,
        0x0000_0031_DE64_7DB4,
        0x0000_0027_2F3C_E666,
        0x0000_0032_64A7_DDE0,
        0x0000_0032_7A36_4875,
    ];
    const PLACE_5: [u8; PLACE_ROW] = [
        0x05, 0, 0, 0, 0, 0, 0, 0, 0xf4, 0xe4, 0x5a, 0xe0, 0x0c, 0, 0, 0, 0xf4, 0xe4, 0x5a, 0xe0,
        0x0c, 0, 0, 0, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    #[test]
    fn hash40_reproduces_a_shipped_hash() {
        assert_eq!(hash40("photostage"), PHOTOSTAGE);
        assert_eq!(hash40("PhotoStage"), PHOTOSTAGE, "must fold case");
        assert_eq!(hash40(""), 0);
    }

    #[test]
    fn rebuilds_the_shipped_photostage_row() {
        let stage = CloneStage::new("photostage");
        assert_eq!(stage.stage_id_hashes(Form::Normal), PHOTOSTAGE_ROW_326);
    }

    #[test]
    fn rebuilds_a_shipped_omega_row_that_uses_the_battle_tree() {
        let mut stage = CloneStage::new("mario_castle64");
        stage.ships_battle_tree = true;
        assert_eq!(stage.stage_id_hashes(Form::Omega), CASTLE64_ROW_6);
        let row = stage.stage_id_row(6, 3, Form::Omega);
        assert_eq!(u32::from_le_bytes(row[0..4].try_into().unwrap()), 6);
        assert_eq!(u32::from_le_bytes(row[4..8].try_into().unwrap()), 3);
        assert_eq!(u64::from_le_bytes(row[8..16].try_into().unwrap()), 1);
    }

    #[test]
    fn a_stage_without_a_battle_tree_keeps_the_normal_path() {
        let stage = CloneStage::new("mario_castle64");
        assert_eq!(stage.root_path(Form::Omega), "stage/mario_castle64/normal");
        assert_ne!(stage.stage_id_hashes(Form::Omega)[1], CASTLE64_ROW_6[1]);
    }

    #[test]
    fn place_row_matches_a_shipped_one() {
        let stage = CloneStage::new("zelda_hyrule");
        assert_eq!(
            stage.place_row(5, false),
            PLACE_5,
            "rebuilt place row differs from the image"
        );
    }

    #[test]
    fn allocation_starts_past_vanilla_and_is_monotonic() {
        let mut allocator = StageAllocator::new(192, 512);
        let mut stage = CloneStage::new("pumpkin_hill");
        stage.forms = vec![Form::Normal, Form::Omega, Form::Battlefield];
        let first = allocator.allocate(&stage).unwrap();
        assert_eq!(first.place, 138);
        assert_eq!(
            first
                .stage_ids
                .iter()
                .map(|s| s.stage_id)
                .collect::<Vec<_>>(),
            vec![364, 365, 366]
        );
        let second = allocator.allocate(&CloneStage::new("green_hill")).unwrap();
        assert_eq!(second.place, 139);
        assert_eq!(second.stage_ids[0].stage_id, 367);
    }

    #[test]
    fn vanilla_capacity_means_no_room_at_all() {
        let mut allocator = StageAllocator::new(VANILLA_PLACES, VANILLA_STAGE_IDS);
        assert_eq!(
            allocator.allocate(&CloneStage::new("pumpkin_hill")),
            Err(AllocationError::OutOfPlaces {
                capacity: VANILLA_PLACES
            })
        );
    }

    #[test]
    fn a_partial_fit_allocates_nothing() {
        let mut allocator = StageAllocator::new(192, 366);
        let mut stage = CloneStage::new("pumpkin_hill");
        stage.forms = vec![Form::Normal, Form::Omega, Form::Battlefield];
        assert_eq!(
            allocator.allocate(&stage),
            Err(AllocationError::OutOfStageIds {
                capacity: 366,
                wanted: 3
            })
        );
        assert_eq!(allocator.places_remaining(), 192 - 138);
        assert_eq!(allocator.stage_ids_remaining(), 2);
    }

    #[test]
    fn rejects_nonsense_declarations() {
        let mut allocator = StageAllocator::new(192, 512);
        assert_eq!(
            allocator.allocate(&CloneStage::new("")),
            Err(AllocationError::EmptyName)
        );
        let mut no_forms = CloneStage::new("x");
        no_forms.forms.clear();
        assert_eq!(allocator.allocate(&no_forms), Err(AllocationError::NoForms));
        let mut twice = CloneStage::new("x");
        twice.forms = vec![Form::Normal, Form::Normal];
        assert_eq!(
            allocator.allocate(&twice),
            Err(AllocationError::DuplicateForm("Normal"))
        );
    }

    #[test]
    fn resource_set_carries_the_shipped_photostage_paths() {
        let stage = CloneStage::new("photostage");
        let normal = stage.resource_set().normal;
        assert_eq!(normal.stage_load_group_hash, PHOTOSTAGE_ROW_326[1]);
        assert_eq!(normal.effect_load_group_hash, PHOTOSTAGE_ROW_326[2]);
        assert_eq!(normal.nus3bank_path_hash, PHOTOSTAGE_ROW_326[3]);
        assert_eq!(normal.sqb_path_hash, PHOTOSTAGE_ROW_326[4]);
        assert_eq!(normal.nus3audio_path_hash, PHOTOSTAGE_ROW_326[5]);
        assert_eq!(normal.tonelabel_path_hash, PHOTOSTAGE_ROW_326[6]);
    }

    #[test]
    fn every_form_is_populated_even_when_unrequested() {
        let stage = CloneStage::new("pumpkin_hill");
        let set = stage.resource_set();
        for form in [set.normal, set.end, set.battle] {
            assert_ne!(form, StageResources::default());
            assert_eq!(
                form.effect_load_group_hash,
                hash40("effect/stage/pumpkin_hill")
            );
        }
        assert_eq!(
            set.normal.stage_load_group_hash,
            set.end.stage_load_group_hash
        );
    }

    #[test]
    fn a_battle_tree_splits_the_forms() {
        let mut stage = CloneStage::new("pumpkin_hill");
        stage.ships_battle_tree = true;
        let set = stage.resource_set();
        assert_eq!(
            set.normal.stage_load_group_hash,
            hash40("stage/pumpkin_hill/normal")
        );
        assert_eq!(
            set.end.stage_load_group_hash,
            hash40("stage/pumpkin_hill/battle")
        );
        assert_eq!(
            set.battle.stage_load_group_hash,
            set.end.stage_load_group_hash
        );
    }

    #[test]
    fn a_battle_only_stage_loads_the_battle_tree_in_every_form() {
        let mut stage = CloneStage::new("final_rush");
        stage.ships_battle_tree = true;
        stage.normal_in_battle_tree = true;
        stage.forms = vec![Form::Normal, Form::Omega, Form::Battlefield];
        for form in [Form::Normal, Form::Omega, Form::Battlefield] {
            assert_eq!(stage.root_path(form), "stage/final_rush/battle");
        }
        stage.normal_in_battle_tree = false;
        assert_eq!(stage.tree(Form::Normal), "normal");
        assert_eq!(stage.tree(Form::Battlefield), "battle");
    }

    #[test]
    fn borrowing_a_place_moves_every_asset_path_and_no_identity() {
        let mut stage = CloneStage::new("template_stage");
        stage.resource_place = Some("battlefield".into());
        assert_eq!(stage.asset_place(), "battlefield");

        let borrowed = stage.stage_id_hashes(Form::Normal);
        let native = CloneStage::new("battlefield").stage_id_hashes(Form::Normal);
        assert_eq!(borrowed, native, "every asset path must be the donor's");

        assert_eq!(borrowed[0], hash40("battlefield"));
        assert_eq!(borrowed[1], hash40("stage/battlefield/normal"));

        let row = stage.place_row(138, false);
        assert_eq!(
            u64::from_le_bytes(row[0x08..0x10].try_into().unwrap()),
            hash40("template_stage")
        );
        assert_eq!(u32::from_le_bytes(row[0x00..0x04].try_into().unwrap()), 138);
    }

    #[test]
    fn an_unset_resource_place_changes_nothing() {
        let plain = CloneStage::new("pumpkin_hill");
        let mut explicit = CloneStage::new("pumpkin_hill");
        explicit.resource_place = Some("pumpkin_hill".into());
        for form in [Form::Normal, Form::Omega, Form::Battlefield] {
            assert_eq!(plain.stage_id_hashes(form), explicit.stage_id_hashes(form));
        }
    }

    #[test]
    fn aux_row_copies_its_template_and_only_reindexes() {
        let template = [0xAAu8; PLACE_AUX_ROW];
        let stage = CloneStage::new("pumpkin_hill");
        let row = stage.place_aux_row(140, Some(&template));
        assert_eq!(u32::from_le_bytes(row[0..4].try_into().unwrap()), 140);
        assert_eq!(&row[4..], &template[4..], "only the index may change");
    }
}
