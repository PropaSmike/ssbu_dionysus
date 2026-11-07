pub const DEFAULT_TEXT: &str = "gliding = true\nstage_select_ui = \"vanilla\"\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageSelectUi {
    Vanilla,
    PageTurning,
    None,
}

impl StageSelectUi {
    pub fn name(self) -> &'static str {
        match self {
            StageSelectUi::Vanilla => "vanilla",
            StageSelectUi::PageTurning => "page_turning",
            StageSelectUi::None => "none",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub gliding: bool,
    pub stage_select_ui: StageSelectUi,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            gliding: true,
            stage_select_ui: StageSelectUi::Vanilla,
        }
    }
}

fn flag(value: &str) -> Option<bool> {
    let value = value.trim_matches('"').trim();
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

fn stage_select_ui(value: &str) -> Option<StageSelectUi> {
    let value = value.trim_matches('"').trim();
    [StageSelectUi::Vanilla, StageSelectUi::PageTurning, StageSelectUi::None]
        .into_iter()
        .find(|choice| value.eq_ignore_ascii_case(choice.name()))
}

pub fn parse(text: &str) -> (Settings, Vec<String>) {
    let mut settings = Settings::default();
    let mut problems = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            problems.push(format!("line {}: not key = value; ignored", index + 1));
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "gliding" => match flag(value) {
                Some(on) => settings.gliding = on,
                None => problems.push(format!("line {}: gliding = {value} is not true or false; ignored", index + 1)),
            },
            "stage_select_ui" => match stage_select_ui(value) {
                Some(choice) => settings.stage_select_ui = choice,
                None => problems.push(format!(
                    "line {}: stage_select_ui = {value} is not vanilla, page_turning or none; ignored",
                    index + 1
                )),
            },
            _ => problems.push(format!("line {}: unknown setting {key}; ignored", index + 1)),
        }
    }
    (settings, problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_file_turns_gliding_on() {
        assert_eq!(parse(DEFAULT_TEXT), (Settings::default(), vec![]));
        assert_eq!(parse(""), (Settings::default(), vec![]));
        assert!(Settings::default().gliding);
    }

    #[test]
    fn gliding_false_turns_it_off() {
        assert_eq!(parse("gliding = false\n").0.gliding, false);
        assert_eq!(parse("gliding=false # another mod glides\r\n").0.gliding, false);
        assert_eq!(parse("gliding = \"FALSE\"").0.gliding, false);
        assert_eq!(parse("# gliding = false\ngliding = true").0.gliding, true);
    }

    #[test]
    fn a_bad_line_is_reported_and_leaves_the_rest() {
        let (settings, problems) = parse("gliding = no\nspeed = 2\nhello\n");
        assert!(settings.gliding);
        assert_eq!(problems.len(), 3);
        assert!(problems[0].starts_with("line 1: gliding = no"));
        assert!(problems[1].contains("unknown setting speed"));
        assert!(problems[2].starts_with("line 3"));
        let (settings, problems) = parse("gliding = false\ngliding = maybe\n");
        assert!(!settings.gliding);
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn an_older_file_gets_the_vanilla_screen() {
        let (settings, problems) = parse("gliding = false\n");
        assert_eq!(settings.stage_select_ui, StageSelectUi::Vanilla);
        assert!(problems.is_empty());
    }

    #[test]
    fn stage_select_ui_takes_the_three_choices() {
        for (text, want) in [
            ("stage_select_ui = \"vanilla\"", StageSelectUi::Vanilla),
            ("stage_select_ui = \"page_turning\"", StageSelectUi::PageTurning),
            ("stage_select_ui = \"none\" # Ultimate S", StageSelectUi::None),
            ("stage_select_ui = NONE", StageSelectUi::None),
        ] {
            let (settings, problems) = parse(text);
            assert_eq!(settings.stage_select_ui, want, "{text}");
            assert!(problems.is_empty(), "{text}");
        }
    }

    #[test]
    fn a_bad_stage_select_ui_is_reported_and_keeps_the_default() {
        let (settings, problems) = parse("stage_select_ui = \"small\"\n");
        assert_eq!(settings.stage_select_ui, StageSelectUi::Vanilla);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("vanilla, page_turning or none"));
    }
}
