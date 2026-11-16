use crate::hash40;

pub const HASH_MASK: u64 = 0xff_ffff_ffff;
pub const FIRST_CUSTOM_PAGE: u32 = 0x08;
pub const LAST_CUSTOM_PAGE: u32 = 0x7f;
pub const MODE_PAGE: u32 = 0x7f;
pub const SEQUENCE_PAGE: u32 = 0x7e;
pub const MAX_MODES: usize = 32;
pub const MAX_MODE_ID: usize = 32;
pub const MAX_MODE_NAME: usize = 64;
pub const MAX_MODE_DESCRIPTION: usize = 256;
pub const MAX_MODE_ORDER: u64 = 0x7fff_ffff;
pub const MAX_SEQUENCES: usize = 64;
pub const MAX_PRELOADS: usize = 64;
pub const MAX_LAYOUTS: usize = 16;
pub const PACK_LAYOUT_PRIORITY: u64 = 0x10100;
pub const OVER_SIDE_PANEL_PRIORITY: u64 = 0x23000;
pub const MOVIE_PRIORITY: u64 = OVER_SIDE_PANEL_PRIORITY - 0x100;
pub const UNDER_MOVIE_PRIORITY: u64 = MOVIE_PRIORITY - 0x40;
pub const MAX_PATCHES: usize = 64;
pub const MAX_MODULE_NAME: usize = 200;
pub const MAX_PATCH_DEPTH: usize = 8;
pub const MAIN_MENU: &str = "main_menu";
pub const GALLERY_SCENE: &str = "gallery";
pub const GALLERY_HOST_LAYOUT: u64 = 0x39_d730_bdfa;
pub const GALLERY_RUNS_AS: u32 = 0x0405_0000;
pub const MAX_GALLERY_TEXT: usize = 1024;
pub const MAX_GALLERY_SOURCES: usize = 256;
pub const MAX_GALLERY_COSTUME: u64 = 255;
pub const MAIN_MENU_LAYOUT: u64 = 0x2d_0e6e_5ffe;
pub const MAX_CALL_ARGS: usize = 8;
pub const MAX_CALLS: usize = 64;
pub const MAX_CALL_NAME: usize = 63;
pub const LUA_TNIL: u32 = 0x00;
pub const LUA_TBOOLEAN: u32 = 0x01;
pub const LUA_TNUMFLT: u32 = 0x03;
pub const LUA_TNUMINT: u32 = 0x13;
pub const LUA_TSHRSTR: u32 = 0x04;
pub const LUA_TLNGSTR: u32 = 0x14;
pub const SCENE_NAME_CAPACITY: usize = 64;
pub const SCENE_HASH_SEED: u32 = 0x811c_9dc5;
pub const SCENE_HASH_FACTOR: u32 = 0x89;
pub const MENU_SCENE: &str = "MenuSequenceScene";
pub const BATTLE_SCENE: &str = "IngameCallSequenceScene";

pub const SCENES: &[&str] = &[
    "MenuSequenceScene",
    "OptionSequenceScene",
    "HelpSequenceScene",
    "IngameCallSequenceScene",
    "UnlockSequenceScene",
    "LocalSequenceScene",
    "MeleeNormalSequenceScene",
    "MeleeAllBattleSequenceScene",
    "MeleeTeamCompeSequenceScene",
    "MeleeTournamentSequenceScene",
    "SpiritsBoardSequenceScene",
    "SpiritsCampaignSequenceScene",
    "SpiritsCampaignFinalStageSequenceScene",
    "SpiritsBoardSpecialSequenceScene",
    "SpiritsOrganizationSequenceScene",
    "StandardSequenceScene",
    "TrainingSequenceScene",
    "KumiteSequenceScene",
    "HomeRunContestSequenceScene",
    "CharaMakeSequenceScene",
    "AmiiboSequenceScene",
    "ChallengerRoomSequenceScene",
    "OnlineMeleeAnySequenceScene",
    "OnlineMeleeParallelSequenceScene",
    "OnlineMeleeRoomSequenceScene",
    "OnlineWatchSequenceScene",
    "SoundTestSequenceScene",
    "CollectionTipsSequenceScene",
    "CollectionShopSequenceScene",
    "CollectionFighterSequenceScene",
    "CollectionCountSequenceScene",
    "CollectionAnniversarySequenceScene",
    "CollectionPlayingSequenceScene",
    "CollectionCardSequenceScene",
    "CollectionReplaySequenceScene",
    "CollectionVideoSequenceScene",
    "CollectionGetterSequenceScene",
    "CollectionPresentSequenceScene",
    "CollectionMovieSequenceScene",
    "MeleeVrSequenceScene",
    "StageMakeSequenceScene",
    "VideoEditSequenceScene",
    "OnlineMeleeTournamentSequenceScene",
    "OnlineShareSequenceScene",
    "OnlineWatchFullSequenceScene",
    "OnlineMeleeConventionSequenceScene",
];

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneName {
    pub hash: u32,
    pub len: u32,
    pub text: [u8; SCENE_NAME_CAPACITY],
}

impl SceneName {
    pub fn new(name: &str) -> Self {
        let bytes = name.as_bytes();
        let len = bytes.len().min(SCENE_NAME_CAPACITY - 1);
        let mut text = [0u8; SCENE_NAME_CAPACITY];
        text[..len].copy_from_slice(&bytes[..len]);
        let hash = text[..len].iter().fold(SCENE_HASH_SEED, |hash, &byte| {
            hash.wrapping_mul(SCENE_HASH_FACTOR) ^ u32::from(byte)
        });
        SceneName {
            hash,
            len: len as u32,
            text,
        }
    }
}

pub const BATTLE_PLAYERS: usize = 8;
pub const BATTLE_PLAYER_TYPES: u32 = 4;
pub const MAX_START_DAMAGE: u32 = 999;
pub const MAX_BATTLE_COLOR: u32 = 255;
pub const BATTLE_RULE_KINDS: u32 = 3;
pub const MAX_RULE_STOCK: u32 = 99;
pub const MAX_RULE_SECONDS: u32 = 5940;
pub const MAX_RULE_ITEMS: u32 = 5;
pub const CHARA_TAG: u64 = 0xc1;
pub const STAGE_TAG: u64 = 0x69;

pub fn tagged(tag: u64, hash: u64) -> u64 {
    (tag << 56) | (hash & HASH_MASK)
}

pub fn scene_index(name: &str) -> Option<u8> {
    SCENES.iter().position(|known| *known == name).map(|index| index as u8)
}

pub const SEQUENCE_NAMES: &[(&str, u32)] = &[
    ("title", 0x0100_0000),
    ("title_demo", 0x0101_0000),
    ("melee", 0x0200_0000),
    ("melee_normal", 0x0201_0000),
    ("melee_team_compe", 0x0202_0000),
    ("melee_special", 0x0203_0000),
    ("melee_special_various", 0x0203_0100),
    ("melee_special_suddendeath", 0x0203_0200),
    ("melee_special_all_battle", 0x0203_0300),
    ("melee_tournament", 0x0204_0000),
    ("spirits", 0x0300_0000),
    ("spirits_challenge", 0x0301_0000),
    ("spirits_canpaign", 0x0302_0000),
    ("spirits_formation", 0x0303_0000),
    ("spirits_challenge_special", 0x0304_0000),
    ("other", 0x0400_0000),
    ("other_standard", 0x0401_0000),
    ("other_kumite", 0x0402_0000),
    ("other_kumite_100", 0x0402_0100),
    ("other_kumite_allstar", 0x0402_0200),
    ("other_kumite_cruel", 0x0402_0300),
    ("other_homerun", 0x0403_0000),
    ("other_homerun_single", 0x0403_0100),
    ("other_homerun_coop", 0x0403_0200),
    ("other_homerun_everyone", 0x0403_0300),
    ("other_training", 0x0404_0000),
    ("other_chara_make", 0x0405_0000),
    ("other_stage_make", 0x0406_0000),
    ("other_amiibo", 0x0407_0000),
    ("other_challenger", 0x0408_0000),
    ("other_vr", 0x0409_0000),
    ("collection", 0x0500_0000),
    ("collection_replay", 0x0501_0000),
    ("collection_replay_data", 0x0501_0100),
    ("collection_replay_video", 0x0501_0200),
    ("collection_replay_edit", 0x0501_0300),
    ("collection_cleargetter", 0x0502_0000),
    ("collection_sound", 0x0503_0000),
    ("collection_smaknowledge", 0x0504_0000),
    ("collection_movie", 0x0505_0000),
    ("collection_record", 0x0506_0000),
    ("collection_record_count", 0x0506_0100),
    ("collection_record_memorial", 0x0506_0200),
    ("collection_record_fighter", 0x0506_0300),
    ("collection_record_card", 0x0506_0400),
    ("collection_record_playing", 0x0506_0500),
    ("collection_shop", 0x0507_0000),
    ("collection_present", 0x0508_0000),
    ("option", 0x0600_0000),
    ("option_melee", 0x0601_0000),
    ("option_button", 0x0602_0000),
    ("option_picture", 0x0603_0000),
    ("option_sound", 0x0604_0000),
    ("option_online", 0x0605_0000),
    ("option_language", 0x0606_0000),
    ("online", 0x0700_0000),
    ("online_melee", 0x0701_0000),
    ("online_melee_anyone", 0x0701_0100),
    ("online_melee_room", 0x0701_0200),
    ("online_melee_tournament", 0x0701_0300),
    ("online_melee_anyone_vip", 0x0701_0400),
    ("online_melee_wait_parallel", 0x0701_0600),
    ("online_melee_convention", 0x0701_0800),
    ("online_watch", 0x0702_0000),
    ("online_share", 0x0707_0000),
];

pub fn vanilla_code(name: &str) -> Option<u32> {
    let name = name.trim().to_ascii_lowercase();
    let name = name.strip_prefix("sequence_").unwrap_or(&name);
    SEQUENCE_NAMES
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, code)| *code)
}

pub const BOSSES: &[(&str, i32)] = &[
    ("masterhand", 352),
    ("crazyhand", 361),
    ("galleom", 367),
    ("ganonboss", 370),
    ("dracula", 373),
    ("dracula2", 379),
    ("marx", 384),
    ("lioleusboss", 392),
    ("kiila", 399),
    ("darz", 410),
];

pub fn call_hash(text: &str) -> i64 {
    (hash40(&text.trim().to_ascii_lowercase()) & HASH_MASK) as i64
}

pub fn boss_kind(hash: i64) -> Option<i32> {
    BOSSES
        .iter()
        .find(|(name, _)| call_hash(name) == hash)
        .map(|(_, kind)| *kind)
}

pub const PLAYER_TYPES: &[&str] = &["human", "cpu", "amiibo", "none"];
pub const RULE_KINDS: &[&str] = &["time", "stock", "stamina"];

pub fn named(value: i64, names: &[&str]) -> i64 {
    names
        .iter()
        .position(|name| call_hash(name) == value)
        .map_or(value, |index| index as i64)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sequence {
    pub code: u32,
    pub runs_as: u32,
    pub scene: Option<u8>,
    pub flow: Flow,
    pub id: u64,
}

pub const SMASH: u32 = 0x0201_0000;
pub const STEP_SELECTORS: i32 = 0x13;
const FLOW_SKIP_RULES: u32 = 1;
const FLOW_SKIP_STAGE_SELECT: u32 = 2;
const FLOW_MENU_AFTER_BATTLE: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flow {
    pub skip_rules: bool,
    pub skip_stage_select: bool,
    pub menu_after_battle: bool,
}

impl Flow {
    pub fn is_empty(self) -> bool {
        self == Flow::default()
    }

    pub fn bits(self) -> u32 {
        (if self.skip_rules { FLOW_SKIP_RULES } else { 0 })
            | (if self.skip_stage_select { FLOW_SKIP_STAGE_SELECT } else { 0 })
            | (if self.menu_after_battle { FLOW_MENU_AFTER_BATTLE } else { 0 })
    }

    pub fn from_bits(bits: u32) -> Flow {
        Flow {
            skip_rules: bits & FLOW_SKIP_RULES != 0,
            skip_stage_select: bits & FLOW_SKIP_STAGE_SELECT != 0,
            menu_after_battle: bits & FLOW_MENU_AFTER_BATTLE != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowHost {
    pub runs_as: u32,
    pub steps: usize,
    pub exit: i32,
    pub rules: i32,
    pub stage_select: i32,
    pub fighter_select: i32,
    pub battle: i32,
    pub after_battle: i32,
}

pub const SMASH_FLOW: FlowHost = FlowHost {
    runs_as: SMASH,
    steps: 10,
    exit: 1,
    rules: 2,
    stage_select: 4,
    fighter_select: 5,
    battle: 6,
    after_battle: 7,
};

pub fn flow_host(runs_as: u32) -> Option<FlowHost> {
    (runs_as == SMASH).then_some(SMASH_FLOW)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepLayout {
    pub next: usize,
    pub selector: usize,
    pub current: usize,
}

pub fn step_layout(steps: usize) -> StepLayout {
    let next = 0x30 + steps * 0x10;
    let selector = next + STEP_SELECTORS as usize * 4;
    StepLayout {
        next,
        selector,
        current: selector + 4,
    }
}

pub fn redirect(host: &FlowHost, flow: Flow, current: i32, next: i32) -> Option<i32> {
    if next == current {
        return None;
    }
    if flow.menu_after_battle && current == host.after_battle && next != host.exit {
        return Some(host.exit);
    }
    let backward = current == host.stage_select || current == host.fighter_select;
    let mut target = next;
    for _ in 0..2 {
        target = if target == host.rules && flow.skip_rules {
            if backward {
                host.exit
            } else {
                host.stage_select
            }
        } else if target == host.stage_select && flow.skip_stage_select {
            if backward {
                host.rules
            } else {
                host.fighter_select
            }
        } else {
            target
        };
    }
    (target != next).then_some(target)
}

fn parse_flow(skip: &[String], after_battle: Option<&str>, line: usize) -> Result<Flow, MenuError> {
    let mut flow = Flow::default();
    for screen in skip {
        match screen.trim().to_ascii_lowercase().as_str() {
            "rules" => flow.skip_rules = true,
            "stage_select" => flow.skip_stage_select = true,
            _ => return Err(MenuError::BadValue { line, key: "skip".to_string() }),
        }
    }
    match after_battle.map(|text| text.trim().to_ascii_lowercase()) {
        None => {}
        Some(text) if text == "menu" => flow.menu_after_battle = true,
        Some(text) if text == "vanilla" => {}
        Some(_) => return Err(MenuError::BadValue { line, key: "after_battle".to_string() }),
    }
    Ok(flow)
}

fn check_flow(flow: Flow, runs_as: Option<u32>, scene: Option<u8>, line: usize) -> Result<(), MenuError> {
    if flow.is_empty() {
        return Ok(());
    }
    match runs_as {
        Some(runs_as) if flow_host(runs_as).is_some() && scene.is_none() => Ok(()),
        _ => Err(MenuError::NoFlow {
            line,
            runs_as: runs_as.unwrap_or(0),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreloadFile {
    pub hash: u64,
    pub script: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preload {
    pub after: u64,
    pub files: Vec<PreloadFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub script: Option<String>,
    pub button: Option<String>,
    pub runs_as: Option<u32>,
    pub scene: Option<u8>,
    pub order: u32,
    pub flow: Flow,
}

impl Mode {
    pub fn button_root(&self) -> Option<String> {
        self.button.as_deref().and_then(layout_root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GalleryItem {
    pub gallery: String,
    pub fighter: String,
    pub costume: u8,
    pub name: String,
    pub text: String,
    pub group: String,
    pub sources: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuDeclaration {
    pub sequences: Vec<Sequence>,
    pub sequence_ids: Vec<String>,
    pub preloads: Vec<Preload>,
    pub main_menu_layouts: Vec<u64>,
    pub over_side_panel: Vec<u64>,
    pub under_movies: Vec<u64>,
    pub patches: Vec<Patch>,
    pub modes: Vec<Mode>,
    pub galleries: Vec<String>,
    pub gallery_music: Vec<(String, String)>,
    pub gallery_items: Vec<GalleryItem>,
    pub gallery_layouts: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptRoot {
    #[default]
    Script,
    ScriptPatch,
}

impl ScriptRoot {
    pub fn parse(text: &str) -> Option<ScriptRoot> {
        match text.trim().to_ascii_lowercase().as_str() {
            "script" => Some(ScriptRoot::Script),
            "script_patch" => Some(ScriptRoot::ScriptPatch),
            _ => None,
        }
    }

    pub fn path(self) -> &'static str {
        match self {
            ScriptRoot::Script => "ui/script/",
            ScriptRoot::ScriptPatch => "ui/script_patch/",
        }
    }

    pub fn from_path(path: &[u8]) -> Option<ScriptRoot> {
        [ScriptRoot::Script, ScriptRoot::ScriptPatch]
            .into_iter()
            .find(|root| root.path().as_bytes() == path)
    }
}

pub fn script_hash(root: ScriptRoot, module: &str) -> u64 {
    hash40(&format!("{}{}.lc", root.path(), module.to_ascii_lowercase()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub module: String,
    pub script: String,
    pub root: ScriptRoot,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MenuError {
    Malformed { line: usize },
    UnknownTable { line: usize, table: String },
    BadValue { line: usize, key: String },
    Missing { line: usize, key: &'static str },
    NotCustom { line: usize, code: u32 },
    NotVanilla { line: usize, code: u32 },
    UnknownScene { line: usize, scene: String },
    UnknownSequenceScene { line: usize, scene: String },
    NotLayout { line: usize, file: String },
    NotModule { line: usize, name: String },
    ModeCode { line: usize, code: u32 },
    NotModeId { line: usize, id: String },
    NoFlow { line: usize, runs_as: u32 },
    UnknownSequence { line: usize, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NotCustom,
    NotVanilla,
    Taken(u32),
    Full,
    Loop,
    ModeCode,
    SameMode,
    SameId,
}

impl Refusal {
    pub fn code(self) -> i32 {
        match self {
            Refusal::NotCustom => -1,
            Refusal::NotVanilla => -2,
            Refusal::Taken(_) => -3,
            Refusal::Full => -4,
            Refusal::Loop => -5,
            Refusal::ModeCode => -6,
            Refusal::SameMode => -7,
            Refusal::SameId => -8,
        }
    }
}

pub fn lua_integer(tag: u32, raw: u64) -> Option<i64> {
    match tag {
        LUA_TNUMINT => Some(raw as i64),
        LUA_TNUMFLT => {
            let value = f32::from_bits(raw as u32);
            value.is_finite().then_some(value as i64)
        }
        LUA_TBOOLEAN => Some(i64::from(raw as u32 != 0)),
        LUA_TNIL => Some(0),
        _ => None,
    }
}

pub fn is_lua_string(tag: u32) -> bool {
    tag == LUA_TSHRSTR || tag == LUA_TLNGSTR
}

pub fn valid_call_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_CALL_NAME
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.')
}

pub fn is_custom(code: u32) -> bool {
    (FIRST_CUSTOM_PAGE..=LAST_CUSTOM_PAGE).contains(&(code >> 24))
}

pub fn is_vanilla(code: u32) -> bool {
    (1..FIRST_CUSTOM_PAGE).contains(&(code >> 24))
}

pub fn is_mode_code(code: u32) -> bool {
    code >> 24 == MODE_PAGE
}

pub fn is_engine_code(code: u32) -> bool {
    code >> 24 == MODE_PAGE || code >> 24 == SEQUENCE_PAGE
}

pub fn mode_code(index: usize) -> u32 {
    MODE_PAGE << 24 | ((index as u32 + 1) & 0xff) << 16
}

pub fn mode_of_code(code: u32) -> Option<usize> {
    if !is_mode_code(code) || code & 0xffff != 0 {
        return None;
    }
    ((code >> 16) & 0xff).checked_sub(1).map(|index| index as usize)
}

pub fn valid_mode_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_MODE_ID
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub fn layout_root(path: &str) -> Option<String> {
    let path = path.trim().to_ascii_lowercase();
    let folder = path.strip_suffix("/layout.arc")?;
    let name = folder.rsplit('/').next()?;
    (!name.is_empty()).then(|| name.to_string())
}

pub fn module_name(name: &str) -> Option<String> {
    let name = name.trim().to_ascii_lowercase();
    let valid = !name.is_empty()
        && name.len() <= MAX_MODULE_NAME
        && !name.starts_with('/')
        && !name.ends_with('/')
        && !name.ends_with(".lc")
        && !name.starts_with("ui/")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'/' | b'-' | b'.'));
    valid.then_some(name)
}

pub fn file(path: &str) -> PreloadFile {
    let path = path.trim().to_ascii_lowercase();
    PreloadFile {
        hash: hash40(&path),
        script: path.ends_with(".lc"),
    }
}

enum Value {
    Number(u64),
    Text(String),
    List(Vec<String>),
    Flag(bool),
}

enum Table {
    Sequence,
    Preload,
    Layout,
    Patch,
    Mode,
    Gallery,
    GalleryItem,
}

struct Pending {
    table: Table,
    line: usize,
    code: Option<u32>,
    runs_as: Option<u32>,
    sequence_scene: Option<String>,
    after: Option<String>,
    files: Vec<String>,
    scene: Option<String>,
    file: Option<String>,
    over_side_panel: bool,
    under_movies: bool,
    module: Option<String>,
    script: Option<String>,
    root: Option<String>,
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    button: Option<String>,
    order: Option<u64>,
    skip: Vec<String>,
    after_battle: Option<String>,
    gallery: Option<String>,
    fighter: Option<String>,
    costume: Option<u64>,
    text: Option<String>,
    group: Option<String>,
    sources: Option<String>,
    music: Option<String>,
}

fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in line.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if quoted => escaped = true,
            b'"' => quoted = !quoted,
            b'#' if !quoted => return &line[..index],
            _ => {}
        }
    }
    line
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn parse_value(raw: &str) -> Option<Value> {
    let raw = raw.trim();
    if let Some(inner) = raw.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let mut items = Vec::new();
        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            items.push(item.strip_prefix('"')?.strip_suffix('"')?.to_string());
        }
        return Some(Value::List(items));
    }
    if let Some(text) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return Some(Value::Text(unescape(text)));
    }
    match raw {
        "true" => return Some(Value::Flag(true)),
        "false" => return Some(Value::Flag(false)),
        _ => {}
    }
    let digits = raw.replace('_', "");
    let number = match digits.strip_prefix("0x").or_else(|| digits.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => digits.parse::<u64>().ok(),
    };
    number.map(Value::Number)
}

fn finish(block: Pending, out: &mut MenuDeclaration) -> Result<(), MenuError> {
    let line = block.line;
    match block.table {
        Table::Sequence => {
            let name = block.id.map(|id| id.trim().to_string()).unwrap_or_default();
            if !name.is_empty() && !valid_mode_id(&name) {
                return Err(MenuError::NotModeId { line, id: name });
            }
            let id = if name.is_empty() { 0 } else { call_hash(&name) as u64 };
            let code = match block.code {
                Some(code) => code,
                None if id != 0 => 0,
                None => return Err(MenuError::Missing { line, key: "id or code" }),
            };
            let runs_as = block.runs_as.ok_or(MenuError::Missing { line, key: "runs_as" })?;
            if code != 0 && !is_custom(code) {
                return Err(MenuError::NotCustom { line, code });
            }
            if is_engine_code(code) {
                return Err(MenuError::ModeCode { line, code });
            }
            if !is_vanilla(runs_as) {
                return Err(MenuError::NotVanilla { line, code: runs_as });
            }
            let scene = match block.sequence_scene {
                Some(name) => Some(
                    scene_index(&name).ok_or(MenuError::UnknownSequenceScene { line, scene: name })?,
                ),
                None => None,
            };
            let flow = parse_flow(&block.skip, block.after_battle.as_deref(), line)?;
            check_flow(flow, Some(runs_as), scene, line)?;
            out.sequences.push(Sequence { code, runs_as, scene, flow, id });
            out.sequence_ids.push(name);
        }
        Table::Preload => {
            let after = block.after.ok_or(MenuError::Missing { line, key: "after" })?;
            if block.files.is_empty() {
                return Err(MenuError::Missing { line, key: "files" });
            }
            out.preloads.push(Preload {
                after: file(&after).hash,
                files: block.files.iter().map(|path| file(path)).collect(),
            });
        }
        Table::Layout => {
            let scene = block.scene.ok_or(MenuError::Missing { line, key: "scene" })?;
            let path = block.file.ok_or(MenuError::Missing { line, key: "file" })?;
            if scene != MAIN_MENU && scene != GALLERY_SCENE {
                return Err(MenuError::UnknownScene { line, scene });
            }
            if !path.trim().to_ascii_lowercase().ends_with("/layout.arc") {
                return Err(MenuError::NotLayout { line, file: path });
            }
            let hash = file(&path).hash;
            if scene == GALLERY_SCENE {
                out.gallery_layouts.push(hash);
                return Ok(());
            }
            out.main_menu_layouts.push(hash);
            if block.over_side_panel {
                out.over_side_panel.push(hash);
            }
            if block.under_movies {
                out.under_movies.push(hash);
            }
        }
        Table::Patch => {
            let module = block.module.ok_or(MenuError::Missing { line, key: "module" })?;
            let script = block.script.ok_or(MenuError::Missing { line, key: "script" })?;
            let module = module_name(&module).ok_or(MenuError::NotModule { line, name: module })?;
            let script = module_name(&script).ok_or(MenuError::NotModule { line, name: script })?;
            let root = match block.root {
                Some(text) => ScriptRoot::parse(&text).ok_or(MenuError::BadValue {
                    line,
                    key: "root".to_string(),
                })?,
                None => ScriptRoot::Script,
            };
            out.patches.push(Patch { module, script, root });
        }
        Table::Mode => {
            let id = block.id.ok_or(MenuError::Missing { line, key: "id" })?;
            let id = id.trim().to_string();
            if !valid_mode_id(&id) {
                return Err(MenuError::NotModeId { line, id });
            }
            let name = block.name.ok_or(MenuError::Missing { line, key: "name" })?;
            let name = name.trim().to_string();
            if name.is_empty() || name.len() > MAX_MODE_NAME {
                return Err(MenuError::BadValue { line, key: "name".to_string() });
            }
            let description = block.description.unwrap_or_default().trim().to_string();
            if description.len() > MAX_MODE_DESCRIPTION {
                return Err(MenuError::BadValue { line, key: "description".to_string() });
            }
            let script = match block.script {
                Some(script) => Some(module_name(&script).ok_or(MenuError::NotModule { line, name: script })?),
                None => None,
            };
            let button = match block.file.or(block.button) {
                Some(path) => {
                    let path = path.trim().to_ascii_lowercase();
                    if layout_root(&path).is_none() {
                        return Err(MenuError::NotLayout { line, file: path });
                    }
                    Some(path)
                }
                None => None,
            };
            if let Some(runs_as) = block.runs_as {
                if !is_vanilla(runs_as) {
                    return Err(MenuError::NotVanilla { line, code: runs_as });
                }
            }
            let scene = match block.sequence_scene {
                Some(name) => {
                    if block.runs_as.is_none() {
                        return Err(MenuError::Missing { line, key: "runs_as" });
                    }
                    Some(scene_index(&name).ok_or(MenuError::UnknownSequenceScene { line, scene: name })?)
                }
                None => None,
            };
            if script.is_none() && block.runs_as.is_none() {
                return Err(MenuError::Missing { line, key: "script or runs_as" });
            }
            let flow = parse_flow(&block.skip, block.after_battle.as_deref(), line)?;
            check_flow(flow, block.runs_as, scene, line)?;
            out.modes.push(Mode {
                id,
                name,
                description,
                script,
                button,
                runs_as: block.runs_as,
                scene,
                order: block.order.unwrap_or(0) as u32,
                flow,
            });
        }
        Table::Gallery => {
            let id = block.id.ok_or(MenuError::Missing { line, key: "id" })?.trim().to_string();
            if !valid_mode_id(&id) {
                return Err(MenuError::NotModeId { line, id });
            }
            let name = block.name.ok_or(MenuError::Missing { line, key: "name" })?.trim().to_string();
            if name.is_empty() || name.len() > MAX_MODE_NAME {
                return Err(MenuError::BadValue { line, key: "name".to_string() });
            }
            let description = block.description.unwrap_or_default().trim().to_string();
            if description.len() > MAX_MODE_DESCRIPTION {
                return Err(MenuError::BadValue { line, key: "description".to_string() });
            }
            if let Some(music) = block.music {
                let music = music.trim().to_ascii_lowercase();
                if music.is_empty() || music.len() > MAX_MODE_NAME || !music.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
                    return Err(MenuError::BadValue { line, key: "music".to_string() });
                }
                out.gallery_music.push((id.clone(), music));
            }
            out.galleries.push(id.clone());
            out.modes.push(Mode {
                id,
                name,
                description,
                script: None,
                button: None,
                runs_as: Some(GALLERY_RUNS_AS),
                scene: None,
                order: block.order.unwrap_or(0) as u32,
                flow: Flow::default(),
            });
        }
        Table::GalleryItem => {
            let gallery = block.gallery.ok_or(MenuError::Missing { line, key: "gallery" })?.trim().to_string();
            if !valid_mode_id(&gallery) {
                return Err(MenuError::NotModeId { line, id: gallery });
            }
            let fighter = block.fighter.ok_or(MenuError::Missing { line, key: "fighter" })?;
            let fighter = fighter.trim().to_ascii_lowercase();
            if fighter.is_empty() || !fighter.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
                return Err(MenuError::BadValue { line, key: "fighter".to_string() });
            }
            let fighter = if fighter.starts_with("ui_chara_") { fighter } else { format!("ui_chara_{fighter}") };
            let name = block.name.ok_or(MenuError::Missing { line, key: "name" })?.trim().to_string();
            if name.is_empty() || name.len() > MAX_MODE_NAME {
                return Err(MenuError::BadValue { line, key: "name".to_string() });
            }
            let text = block.text.unwrap_or_default().trim().replace("\\n", "\n");
            if text.len() > MAX_GALLERY_TEXT {
                return Err(MenuError::BadValue { line, key: "text".to_string() });
            }
            let group = block.group.unwrap_or_default().trim().to_string();
            if group.len() > MAX_MODE_NAME {
                return Err(MenuError::BadValue { line, key: "group".to_string() });
            }
            let sources = block.sources.unwrap_or_default().trim().replace("\\n", "\n");
            if sources.len() > MAX_GALLERY_SOURCES {
                return Err(MenuError::BadValue { line, key: "sources".to_string() });
            }
            out.gallery_items.push(GalleryItem {
                gallery,
                fighter,
                costume: block.costume.unwrap_or(0) as u8,
                name,
                text,
                group,
                sources,
            });
        }
    }
    Ok(())
}

pub fn parse(text: &str) -> Result<MenuDeclaration, MenuError> {
    let mut out = MenuDeclaration::default();
    let mut pending: Option<Pending> = None;
    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
            if let Some(done) = pending.take() {
                finish(done, &mut out)?;
            }
            let table = match name.trim() {
                "sequence" => Table::Sequence,
                "preload" => Table::Preload,
                "layout" => Table::Layout,
                "patch" => Table::Patch,
                "mode" => Table::Mode,
                "gallery" => Table::Gallery,
                "gallery_item" => Table::GalleryItem,
                other => {
                    return Err(MenuError::UnknownTable {
                        line: number,
                        table: other.to_string(),
                    })
                }
            };
            pending = Some(Pending {
                table,
                line: number,
                code: None,
                runs_as: None,
                sequence_scene: None,
                after: None,
                files: Vec::new(),
                scene: None,
                file: None,
                over_side_panel: false,
                under_movies: false,
                module: None,
                script: None,
                root: None,
                id: None,
                name: None,
                description: None,
                button: None,
                order: None,
                skip: Vec::new(),
                after_battle: None,
                gallery: None,
                fighter: None,
                costume: None,
                text: None,
                group: None,
                sources: None,
                music: None,
            });
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(MenuError::Malformed { line: number });
        };
        let Some(block) = pending.as_mut() else {
            return Err(MenuError::Malformed { line: number });
        };
        let key = key.trim();
        let bad = || MenuError::BadValue {
            line: number,
            key: key.to_string(),
        };
        let value = parse_value(value).ok_or_else(bad)?;
        match (&block.table, key, value) {
            (Table::Sequence, "code", Value::Number(n)) if n <= u64::from(u32::MAX) => {
                block.code = Some(n as u32)
            }
            (Table::Sequence, "runs_as", Value::Number(n)) if n <= u64::from(u32::MAX) => {
                block.runs_as = Some(n as u32)
            }
            (Table::Sequence | Table::Mode, "runs_as", Value::Text(text)) => {
                let code = vanilla_code(&text).ok_or_else(|| MenuError::UnknownSequence {
                    line: number,
                    name: text.trim().to_string(),
                })?;
                block.runs_as = Some(code)
            }
            (Table::Sequence, "id", Value::Text(text)) => block.id = Some(text),
            (Table::Sequence, "scene", Value::Text(text)) => block.sequence_scene = Some(text.trim().to_string()),
            (Table::Sequence | Table::Mode, "skip", Value::List(items)) => block.skip = items,
            (Table::Sequence | Table::Mode, "after_battle", Value::Text(text)) => block.after_battle = Some(text),
            (Table::Preload, "after", Value::Text(text)) => block.after = Some(text),
            (Table::Preload, "files", Value::List(items)) => block.files = items,
            (Table::Layout, "scene", Value::Text(text)) => block.scene = Some(text.trim().to_ascii_lowercase()),
            (Table::Layout, "file", Value::Text(text)) => block.file = Some(text),
            (Table::Layout, "over_side_panel", Value::Flag(flag)) => block.over_side_panel = flag,
            (Table::Layout, "under_movies", Value::Flag(flag)) => block.under_movies = flag,
            (Table::Patch, "module", Value::Text(text)) => block.module = Some(text),
            (Table::Patch, "script", Value::Text(text)) => block.script = Some(text),
            (Table::Patch, "root", Value::Text(text)) => block.root = Some(text),
            (Table::Mode, "id", Value::Text(text)) => block.id = Some(text),
            (Table::Mode, "name", Value::Text(text)) => block.name = Some(text),
            (Table::Mode, "description", Value::Text(text)) => block.description = Some(text),
            (Table::Mode, "script", Value::Text(text)) => block.script = Some(text),
            (Table::Mode, "button", Value::Text(text)) => block.button = Some(text),
            (Table::Mode, "runs_as", Value::Number(n)) if n <= u64::from(u32::MAX) => {
                block.runs_as = Some(n as u32)
            }
            (Table::Mode, "scene", Value::Text(text)) => block.sequence_scene = Some(text.trim().to_string()),
            (Table::Mode, "order", Value::Number(n)) if n <= MAX_MODE_ORDER => block.order = Some(n),
            (Table::Gallery, "id", Value::Text(text)) => block.id = Some(text),
            (Table::Gallery, "name", Value::Text(text)) => block.name = Some(text),
            (Table::Gallery, "description", Value::Text(text)) => block.description = Some(text),
            (Table::Gallery, "order", Value::Number(n)) if n <= MAX_MODE_ORDER => block.order = Some(n),
            (Table::Gallery, "music", Value::Text(text)) => block.music = Some(text),
            (Table::GalleryItem, "gallery", Value::Text(text)) => block.gallery = Some(text),
            (Table::GalleryItem, "fighter", Value::Text(text)) => block.fighter = Some(text),
            (Table::GalleryItem, "costume", Value::Number(n)) if n <= MAX_GALLERY_COSTUME => block.costume = Some(n),
            (Table::GalleryItem, "name", Value::Text(text)) => block.name = Some(text),
            (Table::GalleryItem, "text", Value::Text(text)) => block.text = Some(text),
            (Table::GalleryItem, "group", Value::Text(text)) => block.group = Some(text),
            (Table::GalleryItem, "sources", Value::Text(text)) => block.sources = Some(text),
            (Table::Sequence, "id" | "code" | "runs_as" | "scene" | "skip" | "after_battle", _)
            | (Table::Preload, "after" | "files", _)
            | (Table::Layout, "scene" | "file" | "over_side_panel" | "under_movies", _)
            | (Table::Patch, "module" | "script" | "root", _)
            | (
                Table::Mode,
                "id" | "name" | "description" | "script" | "button" | "runs_as" | "scene" | "order" | "skip" | "after_battle",
                _,
            )
            | (Table::Gallery, "id" | "name" | "description" | "order" | "music", _)
            | (Table::GalleryItem, "gallery" | "fighter" | "costume" | "name" | "text" | "group" | "sources", _) => {
                return Err(bad())
            }
            _ => {}
        }
    }
    if let Some(done) = pending.take() {
        finish(done, &mut out)?;
    }
    Ok(out)
}

#[derive(Debug, Default)]
pub struct Registry {
    sequences: Vec<Sequence>,
    preloads: Vec<Preload>,
    main_menu_layouts: Vec<u64>,
    over_side_panel: Vec<u64>,
    under_movies: Vec<u64>,
    patches: Vec<Patch>,
    modes: Vec<Mode>,
}

impl Registry {
    pub fn add_sequence(&mut self, sequence: Sequence) -> Result<u32, Refusal> {
        if is_engine_code(sequence.code) {
            return Err(Refusal::ModeCode);
        }
        if sequence.id != 0 {
            if let Some(existing) = self.sequences.iter().find(|known| known.id == sequence.id) {
                let same = existing.runs_as == sequence.runs_as
                    && existing.scene == sequence.scene
                    && (sequence.code == 0 || sequence.code == existing.code);
                return if same { Ok(existing.code) } else { Err(Refusal::SameId) };
            }
        }
        let mut sequence = sequence;
        if sequence.code == 0 && sequence.id != 0 {
            let given = self
                .sequences
                .iter()
                .filter(|known| known.code >> 24 == SEQUENCE_PAGE)
                .count() as u32;
            sequence.code = SEQUENCE_PAGE << 24 | (given + 1) << 16;
        }
        self.insert_sequence(sequence).map(|()| sequence.code)
    }

    pub fn code_of(&self, id: u64) -> Option<u32> {
        if id == 0 {
            return None;
        }
        if let Some(known) = self.sequences.iter().find(|known| known.id == id) {
            return Some(known.code);
        }
        let index = self.modes.iter().position(|mode| call_hash(&mode.id) as u64 == id)?;
        self.mode_sequence(index)
    }

    fn insert_sequence(&mut self, sequence: Sequence) -> Result<(), Refusal> {
        if !is_custom(sequence.code) {
            return Err(Refusal::NotCustom);
        }
        if !is_vanilla(sequence.runs_as) {
            return Err(Refusal::NotVanilla);
        }
        if let Some(existing) = self.sequences.iter().find(|known| known.code == sequence.code) {
            if existing.runs_as == sequence.runs_as && existing.scene == sequence.scene {
                return Ok(());
            }
            return Err(Refusal::Taken(existing.runs_as));
        }
        if self.sequences.len() >= MAX_SEQUENCES {
            return Err(Refusal::Full);
        }
        self.sequences.push(sequence);
        Ok(())
    }

    pub fn add_preload(&mut self, preload: Preload) -> Result<(), Refusal> {
        if self.preloads.len() >= MAX_PRELOADS {
            return Err(Refusal::Full);
        }
        self.preloads.push(preload);
        Ok(())
    }

    pub fn add_main_menu_layout(&mut self, hash: u64) -> Result<(), Refusal> {
        let hash = hash & HASH_MASK;
        if hash == MAIN_MENU_LAYOUT || self.main_menu_layouts.contains(&hash) {
            return Ok(());
        }
        if self.main_menu_layouts.len() >= MAX_LAYOUTS {
            return Err(Refusal::Full);
        }
        self.main_menu_layouts.push(hash);
        Ok(())
    }

    pub fn main_menu_layouts(&self) -> &[u64] {
        &self.main_menu_layouts
    }

    pub fn keep_main_menu_layouts(&mut self, keep: impl Fn(u64) -> bool) -> Vec<u64> {
        let (kept, dropped): (Vec<u64>, Vec<u64>) =
            self.main_menu_layouts.iter().partition(|hash| keep(**hash));
        self.main_menu_layouts = kept;
        dropped
    }

    pub fn cover_side_panel(&mut self, hash: u64) {
        let hash = hash & HASH_MASK;
        if !self.over_side_panel.contains(&hash) {
            self.over_side_panel.push(hash);
        }
    }

    pub fn under_movies(&mut self, hash: u64) {
        let hash = hash & HASH_MASK;
        if !self.under_movies.contains(&hash) {
            self.under_movies.push(hash);
        }
    }

    pub fn layout_priorities(&self, first_index: u64) -> Vec<u64> {
        self.main_menu_layouts
            .iter()
            .enumerate()
            .map(|(offset, hash)| {
                let index = first_index + offset as u64;
                if self.under_movies.contains(hash) {
                    UNDER_MOVIE_PRIORITY + index
                } else if self.over_side_panel.contains(hash) {
                    OVER_SIDE_PANEL_PRIORITY + index
                } else {
                    PACK_LAYOUT_PRIORITY + index
                }
            })
            .collect()
    }

    pub fn add_patch(&mut self, patch: Patch) -> Result<(), Refusal> {
        if patch.root == ScriptRoot::Script && patch.module == patch.script {
            return Err(Refusal::Loop);
        }
        if self.patches.contains(&patch) {
            return Ok(());
        }
        if self.patches.len() >= MAX_PATCHES {
            return Err(Refusal::Full);
        }
        self.patches.push(patch);
        Ok(())
    }

    pub fn patches_for(&self, root: ScriptRoot, module: &str) -> Vec<String> {
        self.patches
            .iter()
            .filter(|patch| patch.root == root && patch.module.eq_ignore_ascii_case(module))
            .map(|patch| patch.script.clone())
            .collect()
    }

    pub fn patched_script(&self, hash: u64) -> Option<(ScriptRoot, String)> {
        let hash = hash & HASH_MASK;
        self.patches
            .iter()
            .find(|patch| script_hash(patch.root, &patch.module) == hash)
            .map(|patch| (patch.root, patch.module.clone()))
    }

    pub fn has_flows(&self) -> bool {
        self.sequences.iter().any(|sequence| !sequence.flow.is_empty())
    }

    pub fn has_patches(&self) -> bool {
        !self.patches.is_empty()
    }

    pub fn runs_as(&self, code: u32) -> Option<u32> {
        self.sequence(code).map(|known| known.runs_as)
    }

    pub fn sequence(&self, code: u32) -> Option<Sequence> {
        self.sequences.iter().find(|known| known.code == code).copied()
    }

    pub fn has_preloads(&self) -> bool {
        !self.preloads.is_empty()
    }

    pub fn files_after(&self, hash: u64) -> Vec<PreloadFile> {
        let hash = hash & HASH_MASK;
        let mut files: Vec<PreloadFile> = self
            .preloads
            .iter()
            .filter(|preload| preload.after == hash)
            .flat_map(|preload| preload.files.iter().copied())
            .collect();
        if hash == MAIN_MENU_LAYOUT {
            files.extend(self.main_menu_layouts.iter().map(|&layout| PreloadFile {
                hash: layout,
                script: false,
            }));
        }
        files
    }

    pub fn wants_requests(&self) -> bool {
        !self.preloads.is_empty() || !self.main_menu_layouts.is_empty()
    }

    pub fn sequence_count(&self) -> usize {
        self.sequences.len()
    }

    pub fn add_mode(&mut self, mode: Mode) -> Result<(), Refusal> {
        if self.modes.iter().any(|known| known.id == mode.id) {
            return Err(Refusal::SameMode);
        }
        if self.modes.len() >= MAX_MODES {
            return Err(Refusal::Full);
        }
        self.modes.push(mode);
        Ok(())
    }

    pub fn settle_modes(&mut self) -> Vec<(usize, Refusal)> {
        self.modes.sort_by_key(|mode| mode.order);
        let wanted: Vec<(usize, Sequence)> = self
            .modes
            .iter()
            .enumerate()
            .filter_map(|(index, mode)| {
                mode.runs_as.map(|runs_as| {
                    (
                        index,
                        Sequence {
                            code: mode_code(index),
                            runs_as,
                            scene: mode.scene,
                            flow: mode.flow,
                            id: 0,
                        },
                    )
                })
            })
            .collect();
        let mut refused = Vec::new();
        for (index, sequence) in wanted {
            if let Err(refusal) = self.insert_sequence(sequence) {
                refused.push((index, refusal));
            }
        }
        refused
    }

    pub fn modes(&self) -> &[Mode] {
        &self.modes
    }

    pub fn mode_sequence(&self, index: usize) -> Option<u32> {
        let mode = self.modes.get(index)?;
        mode.runs_as?;
        let code = mode_code(index);
        self.sequence(code).map(|_| code)
    }

    pub fn preload_count(&self) -> usize {
        self.preloads.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACK: &str = r#"
[[sequence]]
code = 0x0f01_0000
runs_as = 0x04040000   # Training

[[preload]]
after = "ui/script/menu/main/main_menu_layer_other.lc"
files = ["ui/script/menu/main/my_hello.lc", "UI/Layout/Menu/Title/Title/layout.arc"]
"#;

    #[test]
    fn a_pack_declares_sequences_and_preloads() {
        let parsed = parse(PACK).unwrap();
        assert_eq!(
            parsed.sequences,
            vec![Sequence {
                code: 0x0f01_0000,
                runs_as: 0x0404_0000,
                scene: None,
                flow: Flow::default(),
                id: 0,
            }]
        );
        assert_eq!(parsed.preloads.len(), 1);
        let preload = &parsed.preloads[0];
        assert_eq!(preload.after, 0x2c_5acc_b787);
        assert_eq!(preload.files[0].script, true);
        assert_eq!(preload.files[1].script, false);
        assert_eq!(preload.files[1].hash, hash40("ui/layout/menu/title/title/layout.arc"));
    }

    const MODES: &str = r#"
[[mode]]
id = "story"
name = "Story"
description = "A long adventure"
script = "menu/story/story_mode"
button = "ui/layout/menu/story/story_button/layout.arc"
order = 5

[[mode]]
id = "quick_duel"
name = "Quick Duel"
runs_as = 0x02010000
scene = "IngameCallSequenceScene"
"#;

    #[test]
    fn a_pack_declares_modes() {
        let parsed = parse(MODES).unwrap();
        assert_eq!(parsed.modes.len(), 2);
        let story = &parsed.modes[0];
        assert_eq!(story.id, "story");
        assert_eq!(story.script.as_deref(), Some("menu/story/story_mode"));
        assert_eq!(story.button_root().as_deref(), Some("story_button"));
        assert_eq!(story.order, 5);
        assert_eq!(story.runs_as, None);
        let duel = &parsed.modes[1];
        assert_eq!(duel.runs_as, Some(0x0201_0000));
        assert_eq!(duel.scene, scene_index(BATTLE_SCENE));
        assert_eq!(duel.description, "");
    }

    #[test]
    fn a_mode_needs_an_id_a_name_and_something_to_do() {
        assert_eq!(
            parse("[[mode]]\nid = \"Bad Id\"\nname = \"x\"\nruns_as = 0x02010000\n"),
            Err(MenuError::NotModeId { line: 1, id: "Bad Id".to_string() })
        );
        assert_eq!(
            parse("[[mode]]\nid = \"a\"\nruns_as = 0x02010000\n"),
            Err(MenuError::Missing { line: 1, key: "name" })
        );
        assert_eq!(
            parse("[[mode]]\nid = \"a\"\nname = \"A\"\n"),
            Err(MenuError::Missing { line: 1, key: "script or runs_as" })
        );
        assert_eq!(
            parse("[[mode]]\nid = \"a\"\nname = \"A\"\nscript = \"m/a\"\nscene = \"IngameCallSequenceScene\"\n"),
            Err(MenuError::Missing { line: 1, key: "runs_as" })
        );
    }

    #[test]
    fn manual_codes_stay_off_the_mode_page() {
        assert_eq!(
            parse("[[sequence]]\ncode = 0x7f010000\nruns_as = 0x04040000\n"),
            Err(MenuError::ModeCode { line: 1, code: 0x7f01_0000 })
        );
        let mut registry = Registry::default();
        assert_eq!(
            registry.add_sequence(Sequence { code: 0x7f01_0000, runs_as: 0x0404_0000, scene: None, flow: Flow::default(), id: 0 }),
            Err(Refusal::ModeCode)
        );
        assert_eq!(
            parse("[[sequence]]\ncode = 0x7e010000\nruns_as = 0x04040000\n"),
            Err(MenuError::ModeCode { line: 1, code: 0x7e01_0000 })
        );
    }

    #[test]
    fn runs_as_takes_the_games_sequence_names() {
        let text = "[[sequence]]\nid = \"my_button\"\nruns_as = \"other_training\"\n";
        assert_eq!(parse(text).unwrap().sequences[0].runs_as, 0x0404_0000);
        let text = "[[mode]]\nid = \"a\"\nname = \"A\"\nruns_as = \"SEQUENCE_MELEE_NORMAL\"\n";
        assert_eq!(parse(text).unwrap().modes[0].runs_as, Some(SMASH));
        assert_eq!(
            parse("[[sequence]]\nid = \"b\"\nruns_as = \"training\"\n"),
            Err(MenuError::UnknownSequence { line: 3, name: "training".to_string() })
        );
        assert_eq!(vanilla_code("other_homerun_everyone"), Some(0x0403_0300));
        assert_eq!(vanilla_code("none"), None);
        for (index, (name, code)) in SEQUENCE_NAMES.iter().enumerate() {
            assert!(is_vanilla(*code), "{name}");
            assert!(SEQUENCE_NAMES[index + 1..].iter().all(|(other, _)| other != name), "{name}");
        }
    }

    #[test]
    fn a_sequence_with_an_id_gets_a_code_of_its_own() {
        let parsed = parse("[[sequence]]\nid = \"vs_cpu\"\nruns_as = \"melee_normal\"\n\n[[sequence]]\nid = \"practice\"\nruns_as = \"other_training\"\n").unwrap();
        let mut registry = Registry::default();
        assert_eq!(registry.add_sequence(parsed.sequences[0]), Ok(0x7e01_0000));
        assert_eq!(registry.add_sequence(parsed.sequences[1]), Ok(0x7e02_0000));
        assert_eq!(registry.add_sequence(parsed.sequences[0]), Ok(0x7e01_0000));
        assert_eq!(registry.runs_as(0x7e02_0000), Some(0x0404_0000));
        assert_eq!(registry.code_of(call_hash("practice") as u64), Some(0x7e02_0000));
        assert_eq!(registry.code_of(call_hash("PRACTICE") as u64), Some(0x7e02_0000));
        assert_eq!(registry.code_of(call_hash("nothing") as u64), None);
        let other = parse("[[sequence]]\nid = \"practice\"\nruns_as = \"other_standard\"\n").unwrap();
        assert_eq!(registry.add_sequence(other.sequences[0]), Err(Refusal::SameId));
        let manual = parse("[[sequence]]\nid = \"old\"\ncode = 0x0f010000\nruns_as = \"other_training\"\n").unwrap();
        assert_eq!(registry.add_sequence(manual.sequences[0]), Ok(0x0f01_0000));
        assert_eq!(registry.code_of(call_hash("old") as u64), Some(0x0f01_0000));
        assert_eq!(
            parse("[[sequence]]\nruns_as = \"other_training\"\n"),
            Err(MenuError::Missing { line: 1, key: "id or code" })
        );
        assert_eq!(
            parse("[[sequence]]\nid = \"Bad\"\nruns_as = \"other_training\"\n"),
            Err(MenuError::NotModeId { line: 1, id: "Bad".to_string() })
        );
        for mode in parse(MODES).unwrap().modes {
            registry.add_mode(mode).unwrap();
        }
        assert!(registry.settle_modes().is_empty());
        assert_eq!(registry.code_of(call_hash("quick_duel") as u64), Some(0x7f01_0000));
    }

    #[test]
    fn lua_names_hash_like_the_game() {
        assert_eq!(call_hash("ui_chara_mario"), 0xe_daf3_c863);
        assert_eq!(call_hash(" UI_Stage_Battle_Field "), 0x15_4b5c_4392);
        assert_eq!(boss_kind(call_hash("galleom")), Some(367));
        assert_eq!(boss_kind(call_hash("mario")), None);
        assert_eq!(named(call_hash("cpu"), PLAYER_TYPES), 1);
        assert_eq!(named(call_hash("Stamina"), RULE_KINDS), 2);
        assert_eq!(named(3, PLAYER_TYPES), 3);
    }

    #[test]
    fn modes_sort_by_order_and_get_their_own_codes() {
        let mut registry = Registry::default();
        for mode in parse(MODES).unwrap().modes {
            registry.add_mode(mode).unwrap();
        }
        let again = parse(MODES).unwrap().modes.remove(0);
        assert_eq!(registry.add_mode(again), Err(Refusal::SameMode));
        assert!(registry.settle_modes().is_empty());
        assert_eq!(registry.modes()[0].id, "quick_duel");
        assert_eq!(registry.modes()[1].id, "story");
        assert_eq!(registry.mode_sequence(0), Some(0x7f01_0000));
        assert_eq!(registry.mode_sequence(1), None);
        assert_eq!(registry.runs_as(0x7f01_0000), Some(0x0201_0000));
        assert_eq!(mode_of_code(0x7f01_0000), Some(0));
        assert_eq!(mode_of_code(0x7f02_0000), Some(1));
        assert_eq!(mode_of_code(0x7f00_0000), None);
        assert_eq!(mode_of_code(0x0f01_0000), None);
    }

    #[test]
    fn the_main_menu_paths_hash_like_the_game() {
        assert_eq!(file("ui/layout/menu/main_menu/main_menu/layout.arc").hash, 0x2d_0e6e_5ffe);
        assert_eq!(file("ui/script/menu/main/main_menu.lc").hash, 0x20_e09f_c3fc);
    }

    #[test]
    fn a_custom_code_must_leave_the_vanilla_pages_alone() {
        let text = "[[sequence]]\ncode = 0x04040000\nruns_as = 0x04040000\n";
        assert_eq!(
            parse(text),
            Err(MenuError::NotCustom {
                line: 1,
                code: 0x0404_0000
            })
        );
        let text = "[[sequence]]\ncode = 0x0f010000\nruns_as = 0x0f020000\n";
        assert_eq!(
            parse(text),
            Err(MenuError::NotVanilla {
                line: 1,
                code: 0x0f02_0000
            })
        );
    }

    #[test]
    fn missing_and_mistyped_keys_name_themselves() {
        assert_eq!(
            parse("[[sequence]]\ncode = 0x0f010000\n"),
            Err(MenuError::Missing {
                line: 1,
                key: "runs_as"
            })
        );
        assert_eq!(
            parse("[[preload]]\nafter = 3\n"),
            Err(MenuError::BadValue {
                line: 2,
                key: "after".to_string()
            })
        );
        assert_eq!(
            parse("[[menu]]\n"),
            Err(MenuError::UnknownTable {
                line: 1,
                table: "menu".to_string()
            })
        );
        assert_eq!(parse("code = 1\n"), Err(MenuError::Malformed { line: 1 }));
    }

    #[test]
    fn the_registry_maps_codes_and_refuses_clashes() {
        let mut registry = Registry::default();
        let training = Sequence {
            code: 0x0f01_0000,
            runs_as: 0x0404_0000,
            scene: None,
            flow: Flow::default(),
            id: 0,
        };
        assert_eq!(registry.add_sequence(training), Ok(0x0f01_0000));
        assert_eq!(registry.add_sequence(training), Ok(0x0f01_0000));
        assert_eq!(
            registry.add_sequence(Sequence {
                code: 0x0f01_0000,
                runs_as: 0x0401_0000,
                scene: None,
                flow: Flow::default(),
                id: 0,
            }),
            Err(Refusal::Taken(0x0404_0000))
        );
        assert_eq!(registry.runs_as(0x0f01_0000), Some(0x0404_0000));
        assert_eq!(registry.runs_as(0x0404_0000), None);
        assert_eq!(
            registry.add_sequence(Sequence {
                code: 0x0101_0000,
                runs_as: 0x0404_0000,
                scene: None,
                flow: Flow::default(),
                id: 0,
            }),
            Err(Refusal::NotCustom)
        );
    }

    #[test]
    fn an_unserved_main_menu_layout_is_dropped_and_the_rest_keep_their_order() {
        let first = hash40("ui/layout/menu/a/a/layout.arc");
        let missing = hash40("ui/layout/menu/b/b/layout.arc");
        let last = hash40("ui/layout/menu/c/c/layout.arc");
        let mut registry = Registry::default();
        for hash in [first, missing, last] {
            registry.add_main_menu_layout(hash).unwrap();
        }
        registry.cover_side_panel(last);
        let dropped = registry.keep_main_menu_layouts(|hash| hash != missing);
        assert_eq!(dropped, vec![missing]);
        assert_eq!(registry.main_menu_layouts(), &[first, last]);
        assert_eq!(
            registry.layout_priorities(6),
            vec![PACK_LAYOUT_PRIORITY + 6, OVER_SIDE_PANEL_PRIORITY + 7]
        );
        assert!(!registry.files_after(MAIN_MENU_LAYOUT).iter().any(|file| file.hash == missing));
    }

    #[test]
    fn a_main_menu_layout_loads_beside_the_menu_layout() {
        let text = "[[layout]]\nscene = \"main_menu\"\nfile = \"ui/layout/menu/title/title/layout.arc\"\n";
        let parsed = parse(text).unwrap();
        let title = hash40("ui/layout/menu/title/title/layout.arc");
        assert_eq!(parsed.main_menu_layouts, vec![title]);
        let mut registry = Registry::default();
        registry.add_main_menu_layout(title).unwrap();
        registry.add_main_menu_layout(title).unwrap();
        registry.add_main_menu_layout(MAIN_MENU_LAYOUT).unwrap();
        assert_eq!(registry.main_menu_layouts(), &[title]);
        assert_eq!(
            registry.files_after(MAIN_MENU_LAYOUT),
            vec![PreloadFile {
                hash: title,
                script: false
            }]
        );
        assert!(registry.wants_requests());
        assert_eq!(
            parse("[[layout]]\nscene = \"title\"\nfile = \"a/layout.arc\"\n"),
            Err(MenuError::UnknownScene {
                line: 1,
                scene: "title".to_string()
            })
        );
        assert_eq!(
            parse("[[layout]]\nscene = \"main_menu\"\nfile = \"a/b.lc\"\n"),
            Err(MenuError::NotLayout {
                line: 1,
                file: "a/b.lc".to_string()
            })
        );
    }

    #[test]
    fn a_layout_over_the_side_panel_draws_above_it() {
        let text = "[[layout]]\nscene = \"main_menu\"\nfile = \"a/layout.arc\"\n\n[[layout]]\nscene = \"main_menu\"\nfile = \"b/layout.arc\"\nover_side_panel = true\n";
        let parsed = parse(text).unwrap();
        let (a, b) = (hash40("a/layout.arc"), hash40("b/layout.arc"));
        assert_eq!(parsed.main_menu_layouts, vec![a, b]);
        assert_eq!(parsed.over_side_panel, vec![b]);
        let mut registry = Registry::default();
        for &layout in &parsed.main_menu_layouts {
            registry.add_main_menu_layout(layout).unwrap();
        }
        registry.cover_side_panel(b);
        assert_eq!(registry.layout_priorities(6), vec![PACK_LAYOUT_PRIORITY + 6, OVER_SIDE_PANEL_PRIORITY + 7]);
        assert!(matches!(
            parse("[[layout]]\nscene = \"main_menu\"\nfile = \"a/layout.arc\"\nover_side_panel = 1\n"),
            Err(MenuError::BadValue { line: 4, .. })
        ));
    }

    #[test]
    fn under_movie_layouts_draw_between_the_side_panel_and_movies() {
        let text = "[[layout]]\nscene = \"main_menu\"\nfile = \"a/layout.arc\"\nunder_movies = true\nover_side_panel = true\n\n[[layout]]\nscene = \"main_menu\"\nfile = \"b/layout.arc\"\nover_side_panel = true\n";
        let parsed = parse(text).unwrap();
        let mut registry = Registry::default();
        for hash in &parsed.main_menu_layouts {
            registry.add_main_menu_layout(*hash).unwrap();
        }
        for hash in &parsed.over_side_panel {
            registry.cover_side_panel(*hash);
        }
        for hash in &parsed.under_movies {
            registry.under_movies(*hash);
        }
        let priorities = registry.layout_priorities(6);
        assert_eq!(priorities, vec![UNDER_MOVIE_PRIORITY + 6, OVER_SIDE_PANEL_PRIORITY + 7]);
        assert!(priorities[0] < MOVIE_PRIORITY && MOVIE_PRIORITY < priorities[1]);
    }

    #[test]
    fn scene_names_hash_like_the_router() {
        let menu = SceneName::new(MENU_SCENE);
        assert_eq!(menu.len, 17);
        assert_eq!(&menu.text[..17], b"MenuSequenceScene");
        assert_eq!(menu.text[17], 0);
        assert_eq!(menu.hash, 0xd1fb_0b8f);
        assert_eq!(SceneName::new("").hash, SCENE_HASH_SEED);
        assert_eq!(SceneName::new(&"x".repeat(80)).len, 63);
        assert_eq!(core::mem::size_of::<SceneName>(), 0x48);
        assert_eq!(SCENES.len(), 46);
        assert_eq!(scene_index("IngameCallSequenceScene"), Some(3));
    }

    #[test]
    fn a_sequence_may_name_its_scene() {
        let text = "[[sequence]]\ncode = 0x0f020000\nruns_as = 0x04040000\nscene = \"IngameCallSequenceScene\"\n";
        let parsed = parse(text).unwrap();
        assert_eq!(parsed.sequences[0].scene, Some(3));
        assert_eq!(
            parse("[[sequence]]\ncode = 0x0f020000\nruns_as = 0x04040000\nscene = \"BattleScene\"\n"),
            Err(MenuError::UnknownSequenceScene {
                line: 1,
                scene: "BattleScene".to_string()
            })
        );
    }

    #[test]
    fn selections_carry_their_tag_over_the_hash() {
        let mario = hash40("ui_chara_mario");
        assert_eq!(tagged(CHARA_TAG, mario), 0xc100_000e_daf3_c863);
        assert_eq!(tagged(STAGE_TAG, mario | 0xffff_0000_0000_0000) >> 56, 0x69);
    }

    #[test]
    fn lua_arguments_become_integers() {
        assert_eq!(lua_integer(LUA_TNUMINT, (-5i64) as u64), Some(-5));
        assert_eq!(lua_integer(LUA_TNUMFLT, u64::from(3.75f32.to_bits())), Some(3));
        assert_eq!(lua_integer(LUA_TNUMFLT, u64::from(f32::NAN.to_bits())), None);
        assert_eq!(lua_integer(LUA_TBOOLEAN, 1), Some(1));
        assert_eq!(lua_integer(LUA_TBOOLEAN, 0), Some(0));
        assert_eq!(lua_integer(LUA_TNIL, 99), Some(0));
        assert_eq!(lua_integer(LUA_TSHRSTR, 0), None);
        assert!(is_lua_string(LUA_TSHRSTR) && is_lua_string(LUA_TLNGSTR) && !is_lua_string(LUA_TNUMINT));
    }

    #[test]
    fn call_names_are_plain_dotted_words() {
        assert!(valid_call_name("story.level_count"));
        assert!(valid_call_name("engine.active_sequence"));
        assert!(!valid_call_name(""));
        assert!(!valid_call_name("bad name"));
        assert!(!valid_call_name(&"x".repeat(MAX_CALL_NAME + 1)));
    }

    #[test]
    fn a_patch_names_a_module_and_the_script_that_replaces_it() {
        let text = "[[patch]]\nmodule = \"menu/main/main_menu_layer_other\"\nscript = \"Menu/Story/story_other\"\n";
        let parsed = parse(text).unwrap();
        assert_eq!(
            parsed.patches,
            vec![Patch {
                module: "menu/main/main_menu_layer_other".to_string(),
                script: "menu/story/story_other".to_string(),
                root: ScriptRoot::Script,
            }]
        );
        let text = "[[patch]]\nmodule = \"common/stage_select_actor3\"\nscript = \"menu/pack/stage\"\nroot = \"script_patch\"\n";
        assert_eq!(parse(text).unwrap().patches[0].root, ScriptRoot::ScriptPatch);
        assert_eq!(
            parse("[[patch]]\nmodule = \"menu/a\"\nscript = \"menu/b\"\nroot = \"patch\"\n"),
            Err(MenuError::BadValue {
                line: 1,
                key: "root".to_string()
            })
        );
        for name in ["", "ui/script/menu/a", "menu/a.lc", "/menu/a", "menu a"] {
            let text = format!("[[patch]]\nmodule = \"{name}\"\nscript = \"menu/b\"\n");
            assert_eq!(
                parse(&text),
                Err(MenuError::NotModule {
                    line: 1,
                    name: name.to_string()
                })
            );
        }
        assert_eq!(
            parse("[[patch]]\nmodule = \"menu/a\"\n"),
            Err(MenuError::Missing { line: 1, key: "script" })
        );
    }

    #[test]
    fn patches_chain_in_load_order() {
        let mut registry = Registry::default();
        let first = Patch {
            module: "menu/main/main_menu_layer_other".to_string(),
            script: "menu/a/patch".to_string(),
            root: ScriptRoot::Script,
        };
        let second = Patch {
            module: "menu/main/main_menu_layer_other".to_string(),
            script: "menu/b/patch".to_string(),
            root: ScriptRoot::Script,
        };
        registry.add_patch(first.clone()).unwrap();
        registry.add_patch(second).unwrap();
        registry.add_patch(first).unwrap();
        assert_eq!(
            registry.patches_for(ScriptRoot::Script, "Menu/Main/main_menu_layer_other"),
            vec!["menu/a/patch".to_string(), "menu/b/patch".to_string()]
        );
        assert!(registry.patches_for(ScriptRoot::Script, "menu/main/main_menu_layer_top").is_empty());
        assert!(registry
            .patches_for(ScriptRoot::ScriptPatch, "menu/main/main_menu_layer_other")
            .is_empty());
        assert_eq!(
            registry.add_patch(Patch {
                module: "menu/a".to_string(),
                script: "menu/a".to_string(),
                root: ScriptRoot::Script,
            }),
            Err(Refusal::Loop)
        );
        assert_eq!(
            registry.add_patch(Patch {
                module: "common/a".to_string(),
                script: "common/a".to_string(),
                root: ScriptRoot::ScriptPatch,
            }),
            Ok(())
        );
    }

    #[test]
    fn the_step_machine_layout_matches_smash_and_training() {
        assert_eq!(
            step_layout(10),
            StepLayout {
                next: 0xd0,
                selector: 0x11c,
                current: 0x120
            }
        );
        assert_eq!(
            step_layout(7),
            StepLayout {
                next: 0xa0,
                selector: 0xec,
                current: 0xf0
            }
        );
    }

    #[test]
    fn a_skipped_screen_is_stepped_over_both_ways() {
        let host = SMASH_FLOW;
        let rules = Flow { skip_rules: true, ..Flow::default() };
        assert_eq!(redirect(&host, rules, 0, 2), Some(4));
        assert_eq!(redirect(&host, rules, 4, 2), Some(1));
        assert_eq!(redirect(&host, rules, 4, 5), None);
        let stage = Flow { skip_stage_select: true, ..Flow::default() };
        assert_eq!(redirect(&host, stage, 2, 4), Some(5));
        assert_eq!(redirect(&host, stage, 5, 4), Some(2));
        assert_eq!(redirect(&host, stage, 7, 4), Some(5));
        let both = Flow {
            skip_rules: true,
            skip_stage_select: true,
            menu_after_battle: false,
        };
        assert_eq!(redirect(&host, both, 0, 2), Some(5));
        assert_eq!(redirect(&host, both, 5, 4), Some(1));
        assert_eq!(redirect(&host, both, 5, 6), None);
        let menu = Flow { menu_after_battle: true, ..Flow::default() };
        assert_eq!(redirect(&host, menu, 7, 4), Some(1));
        assert_eq!(redirect(&host, menu, 7, 1), None);
        assert_eq!(redirect(&host, menu, 6, 7), None);
        assert_eq!(redirect(&host, Flow::default(), 0, 2), None);
        assert_eq!(redirect(&host, both, 4, 4), None);
    }

    #[test]
    fn a_flow_needs_smash_and_no_direct_scene() {
        let text = "[[sequence]]\ncode = 0x0f010000\nruns_as = 0x02010000\nskip = [\"rules\", \"Stage_Select\"]\nafter_battle = \"menu\"\n";
        assert_eq!(
            parse(text).unwrap().sequences[0].flow,
            Flow {
                skip_rules: true,
                skip_stage_select: true,
                menu_after_battle: true
            }
        );
        let text = "[[sequence]]\ncode = 0x0f010000\nruns_as = 0x04040000\nskip = [\"stage_select\"]\n";
        assert_eq!(
            parse(text),
            Err(MenuError::NoFlow {
                line: 1,
                runs_as: 0x0404_0000
            })
        );
        let text = "[[sequence]]\ncode = 0x0f010000\nruns_as = 0x02010000\nscene = \"IngameCallSequenceScene\"\nafter_battle = \"menu\"\n";
        assert!(matches!(parse(text), Err(MenuError::NoFlow { .. })));
        let text = "[[sequence]]\ncode = 0x0f010000\nruns_as = 0x02010000\nskip = [\"fighters\"]\n";
        assert_eq!(
            parse(text),
            Err(MenuError::BadValue {
                line: 1,
                key: "skip".to_string()
            })
        );
        let text = "[[mode]]\nid = \"duel\"\nname = \"Duel\"\nruns_as = 0x02010000\nskip = [\"rules\"]\n";
        let mut registry = Registry::default();
        for mode in parse(text).unwrap().modes {
            registry.add_mode(mode).unwrap();
        }
        assert!(registry.settle_modes().is_empty());
        let code = registry.mode_sequence(0).unwrap();
        assert!(registry.sequence(code).unwrap().flow.skip_rules);
        for flow in [Flow::default(), Flow { skip_rules: true, ..Flow::default() }, Flow {
            skip_rules: true,
            skip_stage_select: true,
            menu_after_battle: true,
        }] {
            assert_eq!(Flow::from_bits(flow.bits()), flow);
        }
    }

    #[test]
    fn a_scene_script_is_found_by_the_hash_the_game_loads_it_by() {
        assert_eq!(script_hash(ScriptRoot::Script, "menu/main/main_menu_common"), 0x27_0cca_08a6);
        assert_eq!(
            script_hash(ScriptRoot::ScriptPatch, "common/stage_select_actor3"),
            0x2d_7aa1_a517
        );
        assert_eq!(ScriptRoot::from_path(b"ui/script_patch/"), Some(ScriptRoot::ScriptPatch));
        assert_eq!(ScriptRoot::from_path(b"ui/scripts/"), None);
        let mut registry = Registry::default();
        registry
            .add_patch(Patch {
                module: "common/stage_select_actor3".to_string(),
                script: "menu/pack/stage".to_string(),
                root: ScriptRoot::ScriptPatch,
            })
            .unwrap();
        assert_eq!(
            registry.patched_script(0xff00_002d_7aa1_a517),
            Some((ScriptRoot::ScriptPatch, "common/stage_select_actor3".to_string()))
        );
        assert_eq!(registry.patched_script(0x27_0cca_08a6), None);
    }

    #[test]
    fn preloads_follow_their_trigger_file_only() {
        let mut registry = Registry::default();
        for preload in parse(PACK).unwrap().preloads {
            registry.add_preload(preload).unwrap();
        }
        assert_eq!(registry.files_after(0x2c_5acc_b787).len(), 2);
        assert_eq!(registry.files_after(0xff00_002c_5acc_b787).len(), 2);
        assert!(registry.files_after(0x20_e09f_c3fc).is_empty());
    }

    #[test]
    fn a_gallery_is_a_mode_on_the_mii_screen_with_its_items() {
        let parsed = parse(concat!(
            "[[layout]]\nscene = \"gallery\"\nfile = \"ui/layout/menu/x/y/layout.arc\"\n",
            "[[gallery]]\nid = \"shapes\"\nname = \"Shapes\"\norder = 4\n",
            "[[gallery_item]]\ngallery = \"shapes\"\nfighter = \"viewerstand\"\ncostume = 8\nname = \"Gem\"\ntext = \"One\\nTwo\"\n",
            "[[gallery_item]]\ngallery = \"shapes\"\nfighter = \"ui_chara_mario\"\nname = \"Mario\"\n",
        ))
        .unwrap();
        assert_eq!(parsed.galleries, vec!["shapes".to_string()]);
        assert!(parsed.gallery_music.is_empty());
        assert_eq!(parsed.gallery_layouts.len(), 1);
        assert!(parsed.main_menu_layouts.is_empty());
        assert_eq!(parsed.modes[0].runs_as, Some(GALLERY_RUNS_AS));
        assert_eq!(parsed.modes[0].order, 4);
        assert_eq!(parsed.gallery_items[0].fighter, "ui_chara_viewerstand");
        assert_eq!(parsed.gallery_items[0].costume, 8);
        assert_eq!(parsed.gallery_items[0].text, "One\nTwo");
        assert_eq!(parsed.gallery_items[1].fighter, "ui_chara_mario");
        assert_eq!(parsed.gallery_items[1].costume, 0);
        assert_eq!(parsed.gallery_items[1].group, "");
        assert_eq!(parsed.gallery_items[1].sources, "");
    }

    #[test]
    fn quoted_text_keeps_its_escaped_quotes_and_backslashes() {
        let parsed = parse(concat!(
            "[[gallery_item]]\ngallery = \"shapes\"\nfighter = \"mario\"\nname = \"M\"\n",
            "text = \"called \\\"green mustache guy.\\\" # not a comment\\nnext\\\\line\" # a comment\n",
        ))
        .unwrap();
        assert_eq!(parsed.gallery_items[0].text, "called \"green mustache guy.\" # not a comment\nnext\\line");
    }

    #[test]
    fn a_gallery_may_name_its_music() {
        let parsed = parse("[[gallery]]\nid = \"shapes\"\nname = \"Shapes\"\nmusic = \" UI_BGM_My_Song \"\n").unwrap();
        assert_eq!(parsed.gallery_music, vec![("shapes".to_string(), "ui_bgm_my_song".to_string())]);
        assert!(matches!(
            parse("[[gallery]]\nid = \"shapes\"\nname = \"Shapes\"\nmusic = \"bad name\"\n"),
            Err(MenuError::BadValue { .. })
        ));
    }

    #[test]
    fn a_gallery_item_may_carry_a_group_and_its_sources() {
        let parsed = parse(concat!(
            "[[gallery_item]]\ngallery = \"shapes\"\nfighter = \"mario\"\nname = \"M\"\n",
            "group = \" Series \"\nsources = \"First\\nSecond\"\n",
        ))
        .unwrap();
        assert_eq!(parsed.gallery_items[0].group, "Series");
        assert_eq!(parsed.gallery_items[0].sources, "First\nSecond");
        let long = format!(
            "[[gallery_item]]\ngallery = \"a\"\nfighter = \"mario\"\nname = \"M\"\ngroup = \"{}\"\n",
            "x".repeat(65)
        );
        assert!(matches!(parse(&long), Err(MenuError::BadValue { .. })));
    }

    #[test]
    fn a_gallery_item_needs_its_keys_and_a_costume_in_range() {
        assert!(matches!(
            parse("[[gallery_item]]\nfighter = \"mario\"\nname = \"M\"\n"),
            Err(MenuError::Missing { key: "gallery", .. })
        ));
        assert!(matches!(
            parse("[[gallery_item]]\ngallery = \"a\"\nfighter = \"mario\"\nname = \"M\"\ncostume = 256\n"),
            Err(MenuError::BadValue { .. })
        ));
    }
}
