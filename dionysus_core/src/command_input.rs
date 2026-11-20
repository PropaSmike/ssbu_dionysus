use crate::hash::hash40;

pub const SECTORS: usize = 7;
pub const MAX_STEPS: usize = 12;
pub const MAX_COMMANDS: usize = 32;

pub type StickRange = [[f32; 2]; SECTORS];

pub const VANILLA_STICK_RANGE: StickRange = [
    [-32.5, 32.5],
    [32.5, 57.5],
    [57.5, 122.5],
    [122.5, 153.5],
    [32.5, 57.5],
    [57.5, 122.5],
    [122.5, 153.5],
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    pub life: u8,
    pub super_life: u8,
    pub button_life: u8,
    pub neutral_threshold: f32,
    pub precede: u8,
}

pub const VANILLA: Tuning = Tuning {
    life: 10,
    super_life: 20,
    button_life: 3,
    neutral_threshold: 0.2,
    precede: 7,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Settings {
    pub vanilla: bool,
    pub life: Option<u8>,
    pub super_life: Option<u8>,
    pub button_life: Option<u8>,
    pub neutral_threshold: Option<f32>,
    pub stick_range: Option<StickRange>,
}

impl Settings {
    pub fn is_empty(&self) -> bool {
        *self == Settings::default()
    }

    pub fn common_overrides(&self) -> Vec<(&'static str, f64)> {
        let mut out = Vec::new();
        if let Some(life) = self.life {
            out.push(("special_command_life_max", f64::from(life)));
        }
        if let Some(life) = self.super_life {
            out.push(("super_special_command_life_max", f64::from(life)));
        }
        if let Some(life) = self.button_life {
            out.push(("special_command_ab_button_life", f64::from(life)));
        }
        if let Some(threshold) = self.neutral_threshold {
            out.push(("special_command_neutral_threshold", f64::from(threshold)));
        }
        out
    }

    pub fn toml_lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.vanilla {
            out.push("vanilla = true".to_string());
        }
        for (name, frames) in [
            ("special_command_life_max", self.life),
            ("super_special_command_life_max", self.super_life),
            ("special_command_ab_button_life", self.button_life),
        ] {
            if let Some(frames) = frames {
                out.push(format!("{name} = {frames}"));
            }
        }
        if let Some(threshold) = self.neutral_threshold {
            out.push(format!("special_command_neutral_threshold = {threshold:?}"));
        }
        if let Some(range) = self.stick_range {
            let pairs: Vec<String> = range.iter().map(|[min, max]| format!("[{min:?}, {max:?}]")).collect();
            out.push(format!("special_command_stick_range = [{}]", pairs.join(", ")));
        }
        out
    }

    pub fn tuning(&self, base: Tuning) -> Tuning {
        Tuning {
            life: self.life.unwrap_or(base.life),
            super_life: self.super_life.unwrap_or(base.super_life),
            button_life: self.button_life.unwrap_or(base.button_life),
            neutral_threshold: self.neutral_threshold.unwrap_or(base.neutral_threshold),
            precede: base.precede,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Special,
    Attack,
    Either,
    None,
}

impl Button {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "special" => Ok(Button::Special),
            "attack" => Ok(Button::Attack),
            "either" => Ok(Button::Either),
            "none" => Ok(Button::None),
            _ => Err(format!("button {text:?} must be special, attack, either or none")),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Button::Special => "special",
            Button::Attack => "attack",
            Button::Either => "either",
            Button::None => "none",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub name: String,
    pub input: String,
    pub button: Button,
    pub super_command: bool,
    pub life: Option<u8>,
    pub neutral_resets: bool,
    pub reverse: bool,
    pub cpu: f32,
}

impl Command {
    pub fn new(name: &str, input: &str) -> Self {
        Self {
            name: name.to_string(),
            input: input.to_string(),
            button: Button::Either,
            super_command: false,
            life: None,
            neutral_resets: true,
            reverse: false,
            cpu: 0.0,
        }
    }

    pub fn button(mut self, button: Button) -> Self {
        self.button = button;
        self
    }

    pub fn super_command(mut self) -> Self {
        self.super_command = true;
        self
    }

    pub fn life(mut self, frames: u8) -> Self {
        self.life = Some(frames);
        self
    }

    pub fn neutral_resets(mut self, resets: bool) -> Self {
        self.neutral_resets = resets;
        self
    }

    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    pub fn cpu(mut self, chance: f32) -> Self {
        self.cpu = chance;
        self
    }

    pub fn hash(&self) -> u64 {
        hash40(&self.name)
    }

    pub fn toml_lines(&self) -> Vec<String> {
        let mut out = vec![format!("name = {:?}", self.name), format!("input = {:?}", self.input)];
        if self.button != Button::Either {
            out.push(format!("button = {:?}", self.button.name()));
        }
        if self.super_command {
            out.push("super = true".to_string());
        }
        if let Some(life) = self.life {
            out.push(format!("life = {life}"));
        }
        if !self.neutral_resets {
            out.push("neutral_resets = false".to_string());
        }
        if self.reverse {
            out.push("reverse = true".to_string());
        }
        if self.cpu != 0.0 {
            out.push(format!("cpu = {:?}", self.cpu));
        }
        out
    }

    pub fn steps(&self) -> Result<Vec<u8>, String> {
        parse_input(&self.input)
    }

    pub fn check(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("a command needs a name".to_string());
        }
        self.steps().map_err(|message| format!("command {}: {message}", self.name))?;
        if self.life == Some(0) {
            return Err(format!("command {}: life must be at least 1", self.name));
        }
        if !(0.0..=1.0).contains(&self.cpu) {
            return Err(format!("command {}: cpu is a chance from 0 to 1", self.name));
        }
        if self.cpu > 0.0 && self.button == Button::None {
            return Err(format!(
                "command {}: cpu needs a button, a CPU gets the command when it presses it",
                self.name
            ));
        }
        Ok(())
    }

    pub fn limit(&self, tuning: &Tuning) -> u8 {
        self.life.unwrap_or(if self.super_command { tuning.super_life } else { tuning.life })
    }
}

pub fn parse_input(text: &str) -> Result<Vec<u8>, String> {
    let mut steps = Vec::new();
    for character in text.chars() {
        let Some(digit) = character.to_digit(10).filter(|digit| (1..=9).contains(digit)) else {
            return Err(format!("input {text:?} must be numpad digits 1 to 9"));
        };
        let digit = digit as u8;
        if steps.last() == Some(&digit) {
            return Err(format!("input {text:?} repeats {digit}; put 5 between two of the same direction"));
        }
        steps.push(digit);
    }
    if steps.is_empty() {
        return Err("input is empty".to_string());
    }
    if steps[0] == 5 || steps[steps.len() - 1] == 5 {
        return Err(format!("input {text:?} must start and end on a direction, not 5"));
    }
    if steps.len() > MAX_STEPS {
        return Err(format!("input {text:?} has more than {MAX_STEPS} directions"));
    }
    Ok(steps)
}

pub fn check_stick_range(range: &StickRange) -> Result<(), String> {
    for (index, [min, max]) in range.iter().enumerate() {
        if !min.is_finite() || !max.is_finite() || min > max {
            return Err(format!("special_command_stick_range[{index}] needs min <= max"));
        }
        if !(-180.0..=180.0).contains(min) || !(-180.0..=180.0).contains(max) {
            return Err(format!("special_command_stick_range[{index}] must stay within -180..180 degrees"));
        }
    }
    Ok(())
}

pub fn direction(x: f32, y: f32, neutral_threshold: f32, range: &StickRange) -> u8 {
    let length = (x * x + y * y).sqrt();
    if length <= neutral_threshold {
        return 5;
    }
    let angle = (x / length).atan2(y / length).to_degrees();
    let [up, up_right, right, down_right, up_left, left, down_left] = *range;
    if angle >= up[0] && angle <= up[1] {
        8
    } else if angle > up_right[0] && angle <= up_right[1] {
        9
    } else if angle > right[0] && angle <= right[1] {
        6
    } else if angle > down_right[0] && angle <= down_right[1] {
        3
    } else if angle < -up_left[0] && angle >= -up_left[1] {
        7
    } else if angle < -left[0] && angle >= -left[1] {
        4
    } else if angle < -down_left[0] && angle >= -down_left[1] {
        1
    } else {
        2
    }
}

pub const fn mirror(direction: u8) -> u8 {
    match direction {
        1 => 3,
        3 => 1,
        4 => 6,
        6 => 4,
        7 => 9,
        9 => 7,
        other => other,
    }
}

pub fn relative(direction: u8, lr: f32) -> u8 {
    if lr < 0.0 {
        mirror(direction)
    } else {
        direction
    }
}

const fn components(direction: u8) -> Option<(u8, u8)> {
    match direction {
        1 => Some((2, 4)),
        3 => Some((2, 6)),
        7 => Some((8, 4)),
        9 => Some((8, 6)),
        _ => None,
    }
}

fn skippable(steps: &[u8], at: usize) -> bool {
    if at == 0 || at + 1 >= steps.len() {
        return false;
    }
    let Some((a, b)) = components(steps[at]) else {
        return false;
    };
    let (before, after) = (steps[at - 1], steps[at + 1]);
    (before == a && after == b) || (before == b && after == a)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Progress {
    next: u8,
    timer: u8,
    lr: f32,
}

const IDLE: Progress = Progress { next: 0, timer: 0, lr: 0.0 };

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ready {
    frames: u8,
    lr: f32,
}

const SPENT: Ready = Ready { frames: 0, lr: 0.0 };

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frame {
    pub direction: u8,
    pub facing: f32,
    pub attack: bool,
    pub special: bool,
    pub frozen: bool,
    pub roll: Option<f32>,
}

impl Frame {
    fn presses(&self, button: Button) -> bool {
        match button {
            Button::Special => self.special,
            Button::Attack => self.attack,
            Button::Either => self.special || self.attack,
            Button::None => false,
        }
    }
}

pub fn wants_roll(commands: &[(Command, Vec<u8>)]) -> bool {
    commands.iter().any(|(command, _)| command.cpu > 0.0)
}

#[derive(Clone, Debug)]
pub struct Recognizer {
    progress: [[Progress; 2]; MAX_COMMANDS],
    ready: [Ready; MAX_COMMANDS],
    attack_window: u8,
    special_window: u8,
    held: (bool, bool),
}

impl Default for Recognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Recognizer {
    pub const fn new() -> Self {
        Self {
            progress: [[IDLE; 2]; MAX_COMMANDS],
            ready: [SPENT; MAX_COMMANDS],
            attack_window: 0,
            special_window: 0,
            held: (false, false),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn ready(&self, index: usize) -> Option<f32> {
        self.ready.get(index).filter(|ready| ready.frames > 0).map(|ready| ready.lr)
    }

    pub fn clear(&mut self, index: usize) {
        if let Some(ready) = self.ready.get_mut(index) {
            *ready = SPENT;
        }
    }

    fn pressed(&self, button: Button) -> bool {
        match button {
            Button::Special => self.special_window > 0,
            Button::Attack => self.attack_window > 0,
            Button::Either => self.special_window > 0 || self.attack_window > 0,
            Button::None => true,
        }
    }

    pub fn update(&mut self, commands: &[(Command, Vec<u8>)], tuning: &Tuning, frame: Frame) -> u32 {
        if !frame.frozen {
            for ready in self.ready.iter_mut() {
                ready.frames = ready.frames.saturating_sub(1);
            }
        }
        if frame.attack {
            self.attack_window = tuning.button_life.max(1);
        }
        if frame.special {
            self.special_window = tuning.button_life.max(1);
        }
        let fresh = Frame { attack: frame.attack && !self.held.0, special: frame.special && !self.held.1, ..frame };
        self.held = (frame.attack, frame.special);
        let mut recognised = 0u32;
        let mut chances = 0.0f32;
        for (index, (command, steps)) in commands.iter().enumerate().take(MAX_COMMANDS) {
            let variants = if command.reverse { 2 } else { 1 };
            let limit = command.limit(tuning);
            let mut done = None;
            for variant in 0..variants {
                let facing = if variant == 0 { frame.facing } else { -frame.facing };
                let complete = step(&mut self.progress[index][variant], steps, command.neutral_resets, frame.direction, facing);
                if complete && done.is_none() && self.pressed(command.button) {
                    done = Some(self.progress[index][variant].lr);
                }
                let progress = &mut self.progress[index][variant];
                if progress.next > 0 && !frame.frozen {
                    progress.timer = progress.timer.saturating_add(1);
                    if progress.timer > limit {
                        *progress = IDLE;
                    }
                }
            }
            if let Some(roll) = frame.roll.filter(|_| command.cpu > 0.0 && fresh.presses(command.button)) {
                if done.is_none() && roll >= chances && roll < chances + command.cpu {
                    done = Some(frame.facing);
                }
                chances += command.cpu;
            }
            if let Some(lr) = done {
                self.progress[index] = [IDLE; 2];
                self.ready[index] = Ready { frames: tuning.precede.saturating_add(1), lr };
                recognised |= 1 << index;
            }
        }
        self.attack_window = self.attack_window.saturating_sub(1);
        self.special_window = self.special_window.saturating_sub(1);
        recognised
    }
}

fn step(progress: &mut Progress, steps: &[u8], neutral_resets: bool, direction: u8, facing: f32) -> bool {
    let length = steps.len() as u8;
    if progress.next == 0 {
        if relative(direction, facing) == steps[0] {
            *progress = Progress { next: 1, timer: 0, lr: facing };
        }
        return progress.next == length;
    }
    if progress.next >= length {
        return true;
    }
    let read = relative(direction, progress.lr);
    let at = progress.next as usize;
    if read == steps[at] {
        progress.next += 1;
        progress.timer = 0;
    } else if skippable(steps, at) && read == steps[at + 1] {
        progress.next += 2;
        progress.timer = 0;
    } else if direction == 5 && neutral_resets && steps[at - 1] != 5 {
        *progress = IDLE;
    }
    progress.next >= length
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compiled(commands: &[Command]) -> Vec<(Command, Vec<u8>)> {
        commands.iter().map(|command| (command.clone(), command.steps().unwrap())).collect()
    }

    fn run(recognizer: &mut Recognizer, commands: &[(Command, Vec<u8>)], inputs: &[(u8, bool)], facing: f32) -> Vec<u32> {
        inputs
            .iter()
            .map(|(direction, special)| {
                recognizer.update(
                    commands,
                    &VANILLA,
                    Frame { direction: *direction, facing, special: *special, ..Frame::default() },
                )
            })
            .collect()
    }

    fn last(recognizer: &mut Recognizer, commands: &[(Command, Vec<u8>)], inputs: &[(u8, bool)]) -> u32 {
        *run(recognizer, commands, inputs, 1.0).last().unwrap()
    }

    fn held(first: u8, then: u8, frames: usize, finish: u8) -> Vec<(u8, bool)> {
        let mut inputs = vec![(first, false)];
        inputs.extend(std::iter::repeat((then, false)).take(frames));
        inputs.push((finish, true));
        inputs
    }

    #[test]
    fn the_stick_reads_like_the_game_with_its_sectors_and_threshold() {
        let r = &VANILLA_STICK_RANGE;
        assert_eq!(direction(0.0, 1.0, 0.2, r), 8);
        assert_eq!(direction(1.0, 0.0, 0.2, r), 6);
        assert_eq!(direction(-1.0, 0.0, 0.2, r), 4);
        assert_eq!(direction(0.0, -1.0, 0.2, r), 2);
        assert_eq!(direction(0.7, 0.7, 0.2, r), 9);
        assert_eq!(direction(0.7, -0.7, 0.2, r), 3);
        assert_eq!(direction(-0.7, -0.7, 0.2, r), 1);
        assert_eq!(direction(-0.7, 0.7, 0.2, r), 7);
        assert_eq!(direction(0.1, 0.1, 0.2, r), 5);
        assert_eq!(direction(0.15, -0.95, 0.2, r), 2);
        let kazuya: StickRange = [[-20.0, 20.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0]];
        let (x, y) = (25.0f32.to_radians().sin(), 25.0f32.to_radians().cos());
        assert_eq!(direction(x, y, 0.2, r), 8);
        assert_eq!(direction(x, y, 0.2, &kazuya), 9);
        assert!(check_stick_range(&kazuya).is_ok());
        assert!(check_stick_range(&[[10.0, 5.0]; SECTORS]).is_err());
    }

    #[test]
    fn settings_name_the_vanilla_common_fields_they_override() {
        let settings = Settings { life: Some(12), neutral_threshold: Some(0.25), ..Settings::default() };
        assert_eq!(
            settings.common_overrides(),
            vec![("special_command_life_max", 12.0), ("special_command_neutral_threshold", 0.25f32 as f64)]
        );
        let tuning = settings.tuning(VANILLA);
        assert_eq!((tuning.life, tuning.super_life, tuning.button_life), (12, 20, 3));
        assert!(Settings::default().is_empty());
        assert!(!Settings { vanilla: true, ..Settings::default() }.is_empty());
    }

    #[test]
    fn inputs_parse_as_numpad_and_refuse_what_cannot_be_read() {
        assert_eq!(parse_input("236").unwrap(), vec![2, 3, 6]);
        assert_eq!(parse_input("656").unwrap(), vec![6, 5, 6]);
        assert!(parse_input("2236").is_err());
        assert!(parse_input("5236").is_err());
        assert!(parse_input("23a").is_err());
        assert!(parse_input("").is_err());
        assert!(Command::new("", "236").check().is_err());
        let mut zero = Command::new("hadoken", "236");
        zero.life = Some(0);
        assert!(zero.check().is_err());
    }

    #[test]
    fn a_quarter_circle_and_a_button_is_read_relative_to_facing() {
        let commands = compiled(&[Command::new("hadoken", "236")]);
        let mut recognizer = Recognizer::new();
        assert_eq!(run(&mut recognizer, &commands, &[(2, false), (3, false), (6, true)], 1.0), vec![0, 0, 1]);
        assert_eq!(recognizer.ready(0), Some(1.0));
        let mut recognizer = Recognizer::new();
        assert_eq!(*run(&mut recognizer, &commands, &[(2, false), (1, false), (4, true)], -1.0).last().unwrap(), 1);
        assert_eq!(recognizer.ready(0), Some(-1.0));
    }

    #[test]
    fn the_diagonal_between_two_cardinals_may_be_skipped_but_not_a_cardinal() {
        let commands = compiled(&[Command::new("hadoken", "236"), Command::new("dp", "623")]);
        assert_eq!(last(&mut Recognizer::new(), &commands, &[(2, false), (6, true)]), 1);
        assert_eq!(last(&mut Recognizer::new(), &commands, &[(6, false), (3, false), (6, true)]) & 2, 0);
        assert_eq!(last(&mut Recognizer::new(), &commands, &[(6, false), (2, false), (3, true)]) & 2, 2);
    }

    #[test]
    fn each_step_must_come_within_the_life_and_a_super_gets_the_super_life() {
        let normal = compiled(&[Command::new("hadoken", "236")]);
        assert_eq!(last(&mut Recognizer::new(), &normal, &held(2, 3, 12, 6)), 0);
        assert_eq!(last(&mut Recognizer::new(), &normal, &held(2, 3, 9, 6)), 1);
        let mut geyser = Command::new("geyser", "236");
        geyser.super_command = true;
        assert_eq!(last(&mut Recognizer::new(), &compiled(&[geyser.clone()]), &held(2, 3, 12, 6)), 1);
        geyser.life = Some(4);
        assert_eq!(last(&mut Recognizer::new(), &compiled(&[geyser]), &held(2, 3, 6, 6)), 0);
    }

    #[test]
    fn holding_the_first_direction_keeps_the_command_armed_like_the_game() {
        let commands = compiled(&[Command::new("hadoken", "236")]);
        assert_eq!(last(&mut Recognizer::new(), &commands, &held(2, 2, 30, 6)), 1);
    }

    #[test]
    fn the_button_may_come_a_few_frames_early_or_later_within_the_step_life() {
        let commands = compiled(&[Command::new("hadoken", "236")]);
        assert_eq!(last(&mut Recognizer::new(), &commands, &[(2, false), (3, true), (6, false)]), 1);
        let too_early = [(8, true), (5, false), (5, false), (2, false), (3, false), (6, false)];
        assert_eq!(run(&mut Recognizer::new(), &commands, &too_early, 1.0).iter().sum::<u32>(), 0);
        let late = [(2, false), (3, false), (6, false), (6, false), (6, false), (6, true)];
        assert_eq!(last(&mut Recognizer::new(), &commands, &late), 1);
    }

    #[test]
    fn neutral_cancels_unless_the_command_allows_it_or_asks_for_it() {
        let broken = [(2, false), (5, false), (3, false), (6, true)];
        let strict = compiled(&[Command::new("hadoken", "236")]);
        assert_eq!(run(&mut Recognizer::new(), &strict, &broken, 1.0).iter().sum::<u32>(), 0);
        let mut lenient = Command::new("hadoken", "236");
        lenient.neutral_resets = false;
        assert_eq!(last(&mut Recognizer::new(), &compiled(&[lenient]), &broken), 1);
        let mut dash = Command::new("dash", "656");
        dash.button = Button::None;
        let dash = compiled(&[dash]);
        assert_eq!(last(&mut Recognizer::new(), &dash, &[(6, false), (5, false), (6, false)]), 1);
        let held_neutral = [(6, false), (6, false), (5, false), (5, false), (6, false)];
        assert_eq!(last(&mut Recognizer::new(), &dash, &held_neutral), 1);
    }

    #[test]
    fn a_reversible_command_reads_mirrored_and_reports_which_way() {
        let mut geyser = Command::new("geyser", "21416");
        geyser.reverse = true;
        let commands = compiled(&[geyser.clone()]);
        let plain = [(2, false), (1, false), (4, false), (1, false), (6, true)];
        let mut recognizer = Recognizer::new();
        assert_eq!(last(&mut recognizer, &commands, &plain), 1);
        assert_eq!(recognizer.ready(0), Some(1.0));
        let mirrored = [(2, false), (3, false), (6, false), (3, false), (4, true)];
        let mut recognizer = Recognizer::new();
        assert_eq!(last(&mut recognizer, &commands, &mirrored), 1);
        assert_eq!(recognizer.ready(0), Some(-1.0));
        geyser.reverse = false;
        assert_eq!(run(&mut Recognizer::new(), &compiled(&[geyser]), &mirrored, 1.0).iter().sum::<u32>(), 0);
    }

    #[test]
    fn hitstop_freezes_the_step_timers_and_a_used_command_can_be_cleared() {
        let commands = compiled(&[Command::new("hadoken", "236")]);
        let mut recognizer = Recognizer::new();
        let _ = recognizer.update(&commands, &VANILLA, Frame { direction: 2, facing: 1.0, ..Frame::default() });
        let _ = recognizer.update(&commands, &VANILLA, Frame { direction: 3, facing: 1.0, ..Frame::default() });
        for _ in 0..30 {
            let _ = recognizer.update(&commands, &VANILLA, Frame { direction: 3, facing: 1.0, frozen: true, ..Frame::default() });
        }
        let bits = recognizer.update(&commands, &VANILLA, Frame { direction: 6, facing: 1.0, special: true, ..Frame::default() });
        assert_eq!(bits, 1);
        for _ in 0..=VANILLA.precede {
            assert!(recognizer.ready(0).is_some());
            let _ = recognizer.update(&commands, &VANILLA, Frame { direction: 5, facing: 1.0, ..Frame::default() });
        }
        assert!(recognizer.ready(0).is_none());
        let _ = run(&mut recognizer, &commands, &[(2, false), (3, false), (6, true)], 1.0);
        assert!(recognizer.ready(0).is_some());
        recognizer.clear(0);
        assert!(recognizer.ready(0).is_none());
    }

    #[test]
    fn a_cpu_gets_one_command_per_press_by_its_chance() {
        let commands = compiled(&[
            Command::new("geyser", "21416").button(Button::Special).cpu(0.25),
            Command::new("kick", "236").button(Button::Attack).cpu(0.5),
            Command::new("hadoken", "236").cpu(0.5),
        ]);
        assert!(wants_roll(&commands));
        let press = |roll: f32, special: bool| {
            let mut recognizer = Recognizer::new();
            let frame = Frame { direction: 5, facing: -1.0, special, attack: !special, roll: Some(roll), ..Frame::default() };
            let bits = recognizer.update(&commands, &VANILLA, frame);
            (bits, recognizer.ready(bits.trailing_zeros() as usize))
        };
        assert_eq!(press(0.1, true), (1, Some(-1.0)));
        assert_eq!(press(0.6, true).0, 4);
        assert_eq!(press(0.9, true).0, 0);
        assert_eq!(press(0.3, false).0, 2);
        assert_eq!(press(0.7, false).0, 4);
        let held = Frame { direction: 5, facing: 1.0, roll: Some(0.0), ..Frame::default() };
        assert_eq!(Recognizer::new().update(&commands, &VANILLA, held), 0);
        let mut recognizer = Recognizer::new();
        let twice = Frame { direction: 5, facing: 1.0, special: true, roll: Some(0.1), ..Frame::default() };
        assert_eq!(recognizer.update(&commands, &VANILLA, twice), 1);
        recognizer.clear(0);
        assert_eq!(recognizer.update(&commands, &VANILLA, twice), 0);
        assert!(Command::new("dash", "656").button(Button::None).cpu(0.5).check().is_err());
        assert!(Command::new("kick", "236").cpu(1.5).check().is_err());
        assert!(!wants_roll(&compiled(&[Command::new("kick", "236")])));
    }
}
