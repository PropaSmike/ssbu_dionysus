pub const PATCH_ROWS: &[&str] = &[
    "ui_chara_sandbag",
    "ui_chara_packun",
    "ui_chara_jack",
    "ui_chara_brave",
    "ui_chara_buddy",
    "ui_chara_dolly",
    "ui_chara_master",
    "ui_chara_tantan",
    "ui_chara_pickel",
    "ui_chara_edge",
    "ui_chara_element",
    "ui_chara_flame_first",
    "ui_chara_light_first",
    "ui_chara_flame_only",
    "ui_chara_light_only",
    "ui_chara_demon",
    "ui_chara_trail",
];

fn numbered<'a>(file_name: &'a str, prefix: &str) -> Option<&'a str> {
    let stem = file_name.strip_suffix(".bntx")?;
    let (number, rest) = stem.strip_prefix(prefix)?.split_once('_')?;
    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) || rest.is_empty() {
        return None;
    }
    Some(rest)
}

pub fn chara_name(file_name: &str) -> Option<&str> {
    let (name, costume) = numbered(file_name, "chara_")?.rsplit_once('_')?;
    if name.is_empty() || costume.len() < 2 || !costume.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(name)
}

pub fn stage_name(file_name: &str) -> Option<&str> {
    numbered(file_name, "stage_")
}

pub fn row_patch_flag(set: Option<bool>, base_patch: bool, in_replace: bool, in_patch: bool) -> Option<bool> {
    let wanted = match (in_replace, in_patch) {
        (false, true) => true,
        (true, false) => false,
        _ => return None,
    };
    (set.unwrap_or(base_patch) != wanted).then_some(wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_names_its_fighter() {
        assert_eq!(chara_name("chara_0_mecha_00.bntx"), Some("mecha"));
        assert_eq!(chara_name("chara_13_eflame_first_07.bntx"), Some("eflame_first"));
        assert_eq!(chara_name("chara_3_jack_120.bntx"), Some("jack"));
        assert_eq!(chara_name("chara_0_mecha.bntx"), None);
        assert_eq!(chara_name("chara_x_mecha_00.bntx"), None);
        assert_eq!(chara_name("stage_0_mecha_00.bntx"), None);
        assert_eq!(chara_name("chara_0_mecha_00.nutexb"), None);
    }

    #[test]
    fn an_icon_names_its_stage() {
        assert_eq!(stage_name("stage_0_animal_city.bntx"), Some("animal_city"));
        assert_eq!(stage_name("stage_1_75m.bntx"), Some("75m"));
        assert_eq!(stage_name("stage_4_ff_cave.bntx"), Some("ff_cave"));
        assert_eq!(stage_name("stage_x_ff_cave.bntx"), None);
        assert_eq!(stage_name("stage_2_.bntx"), None);
        assert_eq!(stage_name("chara_0_ff_cave.bntx"), None);
    }

    #[test]
    fn the_row_reads_the_tree_that_has_the_pictures() {
        assert_eq!(row_patch_flag(Some(false), false, false, true), Some(true));
        assert_eq!(row_patch_flag(None, true, true, false), Some(false));
        assert_eq!(row_patch_flag(None, true, false, true), None);
        assert_eq!(row_patch_flag(Some(false), true, true, false), None);
    }

    #[test]
    fn no_pictures_or_both_trees_leave_the_row_alone() {
        assert_eq!(row_patch_flag(Some(false), false, false, false), None);
        assert_eq!(row_patch_flag(Some(false), true, true, true), None);
        assert_eq!(row_patch_flag(None, true, false, false), None);
    }
}
