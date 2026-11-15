pub const COMMANDS: [(&str, &str); 3] = [
    ("l", "define_secret_command_l"),
    ("r", "define_secret_command_r"),
    ("l_r", "define_secret_command_l_r"),
];
pub const DEFAULT_COMMAND: &str = "l";

const UI_STAGE_EXCEPTIONS: [(&str, &str); 18] = [
    ("battlefield", "ui_stage_battle_field"),
    ("battlefield_l", "ui_stage_battle_field_l"),
    ("battlefield_s", "ui_stage_battle_field_s"),
    ("_75m", "ui_stage_75m"),
    ("bonusgame", "ui_stage_bonus_game"),
    ("settingstage", "ui_stage_setting_stage"),
    ("shamfight", "ui_stage_sham_fight"),
    ("campaignmap", "ui_stage_campaign_map"),
    ("bossstage_ganonboss", "ui_stage_boss_ganon"),
    ("bossstage_rathalos", "ui_stage_boss_rathalos"),
    ("bossstage_marx", "ui_stage_boss_marx"),
    ("bossstage_dracula", "ui_stage_boss_dracula"),
    ("bossstage_galleom", "ui_stage_boss_galleom"),
    ("bossstage_final1", "ui_stage_boss_final"),
    ("bossstage_final2", "ui_stage_boss_final2"),
    ("bossstage_final3", "ui_stage_boss_final3"),
    ("sp_edit", "ui_stage_edit"),
    ("homeruncontest", "ui_stage_homerun"),
];

const VANILLA_SECRETS: [(&str, &str, &str); 2] = [
    ("punchoutsb", "punchoutw", "define_secret_command_l"),
    ("pickel_world", "pickel_world", "define_secret_command_extra"),
];

pub fn command_name(command: &str) -> Option<&'static str> {
    let command = command.trim().to_ascii_lowercase();
    COMMANDS
        .iter()
        .find(|(short, _)| *short == command)
        .map(|(_, name)| *name)
}

pub fn ui_stage_name(stage: &str) -> String {
    let stage = stage.trim().to_ascii_lowercase();
    if stage.starts_with("ui_stage_") {
        return stage;
    }
    UI_STAGE_EXCEPTIONS
        .iter()
        .find(|(place, _)| *place == stage)
        .map(|(_, id)| id.to_string())
        .unwrap_or_else(|| format!("ui_stage_{stage}"))
}

pub fn existing_secret(stage: &str) -> Option<(&'static str, &'static str)> {
    let wanted = ui_stage_name(stage);
    VANILLA_SECRETS
        .iter()
        .find(|(place, _, _)| ui_stage_name(place) == wanted)
        .map(|(_, secret, command)| (*secret, *command))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_name_their_vanilla_stage_ids() {
        assert_eq!(ui_stage_name("battlefield"), "ui_stage_battle_field");
        assert_eq!(ui_stage_name("Battlefield"), "ui_stage_battle_field");
        assert_eq!(ui_stage_name("mario_castle64"), "ui_stage_mario_castle64");
        assert_eq!(ui_stage_name("ui_stage_punchoutsb"), "ui_stage_punchoutsb");
        assert_eq!(ui_stage_name("my_clone"), "ui_stage_my_clone");
    }

    #[test]
    fn commands_are_the_ones_the_game_reads() {
        assert_eq!(command_name("l"), Some("define_secret_command_l"));
        assert_eq!(command_name("L_R"), Some("define_secret_command_l_r"));
        assert_eq!(command_name("extra"), None);
        assert_eq!(command_name("zl"), None);
        assert_eq!(command_name(DEFAULT_COMMAND), Some("define_secret_command_l"));
    }

    #[test]
    fn rows_that_already_lead_somewhere_are_known() {
        assert_eq!(existing_secret("punchoutsb"), Some(("punchoutw", "define_secret_command_l")));
        assert_eq!(existing_secret("ui_stage_pickel_world"), Some(("pickel_world", "define_secret_command_extra")));
        assert_eq!(existing_secret("battlefield"), None);
    }
}
