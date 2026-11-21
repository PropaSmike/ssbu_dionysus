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

fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

pub fn chara_parts(file_name: &str) -> Option<(u32, &str, u32)> {
    let stem = file_name.strip_suffix(".bntx")?;
    let (slot, rest) = stem.strip_prefix("chara_")?.split_once('_')?;
    let (name, costume) = rest.rsplit_once('_')?;
    if !digits(slot) || name.is_empty() || costume.len() < 2 || !digits(costume) {
        return None;
    }
    Some((slot.parse().ok()?, name, costume.parse().ok()?))
}

pub fn chara_name(file_name: &str) -> Option<&str> {
    chara_parts(file_name).map(|(_, name, _)| name)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreePictures {
    pub slots: std::collections::BTreeSet<u32>,
    pub costumes: std::collections::BTreeSet<u32>,
    pub files: usize,
}

impl TreePictures {
    pub fn add(&mut self, slot: u32, costume: u32) {
        self.slots.insert(slot);
        self.costumes.insert(costume);
        self.files += 1;
    }

    fn describe(&self) -> String {
        if self.files == 0 {
            return "none".to_string();
        }
        let slots: Vec<String> = self.slots.iter().map(u32::to_string).collect();
        let first = *self.costumes.iter().next().unwrap_or(&0);
        let last = *self.costumes.iter().next_back().unwrap_or(&0);
        let costumes = if self.costumes.len() == 1 {
            format!("costume {first:02}")
        } else if (last - first) as usize + 1 == self.costumes.len() {
            format!("costumes {first:02} to {last:02}")
        } else {
            let listed: Vec<String> = self.costumes.iter().map(|costume| format!("{costume:02}")).collect();
            format!("costumes {}", listed.join(", "))
        };
        format!("chara_{} with {costumes} ({} files)", slots.join(", "), self.files)
    }
}

fn tree_name(patch: bool) -> &'static str {
    if patch {
        "ui/replace_patch/chara"
    } else {
        "ui/replace/chara"
    }
}

pub fn picture_report(name: &str, replace: &TreePictures, patch: &TreePictures, reads_patch: bool, base_patch: bool) -> String {
    let mut line = format!(
        "[pictures] {name}: {} has {}; {} has {}; its row reads {}",
        tree_name(false),
        replace.describe(),
        tree_name(true),
        patch.describe(),
        tree_name(reads_patch)
    );
    if reads_patch != base_patch {
        line += &format!(", not its base's {}", tree_name(base_patch));
    }
    if replace.files > 0 && patch.files > 0 {
        line += &format!(
            ". The ones under {} never show: keep them all in one tree",
            tree_name(!reads_patch)
        );
    } else if replace.files + patch.files == 0 {
        line += &format!(
            ". No chara_<N>_{name}_<costume>.bntx in an enabled pack, so none show (pictures shared through config.json are not counted)"
        );
    }
    line
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
    fn a_picture_gives_its_slot_and_costume() {
        assert_eq!(chara_parts("chara_13_eflame_first_07.bntx"), Some((13, "eflame_first", 7)));
        assert_eq!(chara_parts("chara_3_jack_120.bntx"), Some((3, "jack", 120)));
        assert_eq!(chara_parts("chara_3_jack_7.bntx"), None);
        assert_eq!(chara_parts("chara__jack_07.bntx"), None);
    }

    fn pictures(files: &[(u32, u32)]) -> TreePictures {
        let mut tree = TreePictures::default();
        for &(slot, costume) in files {
            tree.add(slot, costume);
        }
        tree
    }

    #[test]
    fn the_report_names_both_trees_and_the_one_the_row_reads() {
        let replace = pictures(&[(0, 0), (0, 1), (1, 0), (1, 1)]);
        assert_eq!(
            picture_report("wawa", &replace, &TreePictures::default(), false, false),
            "[pictures] wawa: ui/replace/chara has chara_0, 1 with costumes 00 to 01 (4 files); \
             ui/replace_patch/chara has none; its row reads ui/replace/chara"
        );
        let patch = pictures(&[(3, 120), (3, 127)]);
        assert_eq!(
            picture_report("wawa", &TreePictures::default(), &patch, true, false),
            "[pictures] wawa: ui/replace/chara has none; ui/replace_patch/chara has chara_3 with costumes 120, 127 \
             (2 files); its row reads ui/replace_patch/chara, not its base's ui/replace/chara"
        );
    }

    #[test]
    fn the_report_warns_about_split_or_missing_pictures() {
        let replace = pictures(&[(1, 0), (3, 0)]);
        let patch = pictures(&[(0, 0), (2, 0), (4, 0)]);
        let split = picture_report("wawa", &replace, &patch, false, false);
        assert!(split.contains("chara_1, 3 with costume 00 (2 files)"), "{split}");
        assert!(split.ends_with(". The ones under ui/replace_patch/chara never show: keep them all in one tree"), "{split}");
        let none = picture_report("wawa", &TreePictures::default(), &TreePictures::default(), false, false);
        assert!(none.ends_with(". No chara_<N>_wawa_<costume>.bntx in an enabled pack, so none show \
                                (pictures shared through config.json are not counted)"), "{none}");
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
