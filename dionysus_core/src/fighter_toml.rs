use crate::article_hurtbox::{Field, Hurtbox, MAX_PER_ARTICLE};
use crate::article_lvd::{ArticleLvd, LvdVertex};
use crate::grab_rule::{status_kind, GrabRule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamValue {
    Float(f64),
    Int(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamOp {
    Set,
    Mul,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDeclaration {
    pub name: String,
    pub slot: Option<i32>,
    pub op: ParamOp,
    pub value: ParamValue,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArticleDeclaration {
    pub name: String,
    pub from: Option<(String, String)>,
    pub base_article: Option<String>,
    pub kirby: bool,
    pub hurtboxes: Vec<Hurtbox>,
    pub lvd: Option<ArticleLvd>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MotionDeclaration {
    pub name: String,
    pub animation: String,
    pub template: Option<String>,
    pub game: Option<String>,
    pub sound: Option<String>,
    pub effect: Option<String>,
    pub expression: Option<String>,
    pub flags: Vec<String>,
    pub blend_frames: Option<u8>,
    pub cancel_frame: Option<u8>,
    pub xlu: Option<(u8, u8)>,
    pub no_stop_intp: Option<bool>,
    pub animation_unk: Option<u8>,
    pub no_extra: bool,
    pub victim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KirbyDeclaration {
    pub statuses: i32,
    pub first: Option<i32>,
    pub full_model: bool,
    pub model: Option<String>,
    pub motions: Vec<MotionDeclaration>,
    pub meshes: Vec<(String, bool)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FighterDeclaration {
    pub name: String,
    pub base: String,
    pub display_name: Option<String>,
    pub color_start: u32,
    pub color_count: u32,
    pub ui_chara: Option<String>,
    pub fighter_kind_name: Option<String>,
    pub resource_name: Option<String>,
    pub base_resource_name: Option<String>,
    pub series: Option<String>,
    pub disp_order: Option<i8>,
    pub save_no: Option<i8>,
    pub exhibit_year: Option<i16>,
    pub narration: Option<String>,
    pub staffroll: bool,
    pub item_hand: Option<crate::item_hand::ItemHand>,
    pub own_css: bool,
    pub jingle: Option<String>,
    pub owns_param_resources: bool,
    pub effect_namespace: u32,
    pub article_namespace: u32,
    pub articles: Vec<ArticleDeclaration>,
    pub kirby: Option<KirbyDeclaration>,
    pub global_motions: Vec<MotionDeclaration>,
    pub grabs: Vec<GrabRule>,
    pub hud: Option<String>,
    pub hud_layout: Option<String>,
    pub final_smash: Option<String>,
    pub tether: Option<crate::tether::Tether>,
    pub partner: Option<bool>,
    pub partner_with: Option<String>,
    pub partner_params: Vec<crate::partner_params::PartnerParam>,
    pub switch_with: Vec<String>,
    pub params: Vec<ParamDeclaration>,
    pub command_settings: crate::command_input::Settings,
    pub commands: Vec<crate::command_input::Command>,
    pub move_list: Option<crate::move_list::MoveList>,
}

pub const DEFAULT_COLOR_COUNT: u32 = 8;
pub const PARTNER_BASE: &str = "popo";

impl FighterDeclaration {
    pub fn ui_chara(&self) -> String {
        self.ui_chara
            .clone()
            .unwrap_or_else(|| format!("ui_chara_{}", self.name))
    }

    pub fn fighter_kind_name(&self) -> String {
        self.fighter_kind_name
            .clone()
            .unwrap_or_else(|| format!("fighter_kind_{}", self.name))
    }

    pub fn resource_name(&self) -> String {
        self.resource_name.clone().unwrap_or_else(|| self.name.clone())
    }

    pub fn base_resource_name(&self) -> String {
        self.base_resource_name
            .clone()
            .unwrap_or_else(|| self.base.clone())
    }

    pub fn display_name(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.name.clone())
    }

    pub fn has_partner(&self) -> bool {
        self.partner.unwrap_or(self.base == PARTNER_BASE && self.switch_with.is_empty())
    }

    pub fn series(&self) -> String {
        let series = self.series.as_deref().unwrap_or("");
        if series.starts_with("ui_series_") {
            series.to_string()
        } else {
            format!("ui_series_{series}")
        }
    }
}

impl MotionDeclaration {
    pub fn script(&self, prefix: &str, stem: Option<&str>) -> Option<String> {
        let explicit = match prefix {
            "game" => &self.game,
            "sound" => &self.sound,
            "effect" => &self.effect,
            _ => &self.expression,
        };
        explicit
            .clone()
            .or_else(|| stem.map(|stem| format!("{prefix}_{stem}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Fighter,
    Article,
    ArticleHurtbox,
    ArticleLvdVertex,
    Kirby,
    Motion,
    GlobalMotion,
    Grab,
    Mesh,
    Hud,
    Final,
    Tether,
    TetherBox,
    PartnerParams,
    Commands,
    Command,
    MoveList,
    Params { slot: Option<i32>, op: ParamOp },
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

fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

fn unquote(value: &str, line: usize) -> Result<String, ParseError> {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return Ok(value[1..value.len() - 1].to_string());
    }
    if value.starts_with('"') || value.ends_with('"') {
        return Err(error(line, "unterminated string"));
    }
    Err(error(line, "expected a quoted string"))
}

fn parse_bool(value: &str, line: usize, key: &str) -> Result<bool, ParseError> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(error(line, format!("{key} must be true or false"))),
    }
}

fn parse_int<T: core::str::FromStr>(value: &str, line: usize, key: &str) -> Result<T, ParseError> {
    let value = value.trim();
    let parsed = if let Some(hex) = value.strip_prefix("0x") {
        i64::from_str_radix(hex, 16)
            .ok()
            .and_then(|number| number.to_string().parse::<T>().ok())
    } else {
        value.parse::<T>().ok()
    };
    parsed.ok_or_else(|| {
        let whole = match value.strip_prefix("0x") {
            Some(hex) => i64::from_str_radix(hex, 16).is_ok(),
            None => value.parse::<i64>().is_ok(),
        };
        match whole {
            true => error(line, format!("{key} {value} is out of range")),
            false => error(line, format!("{key} must be a whole number")),
        }
    })
}

fn parse_list(value: &str, line: usize, key: &str) -> Result<Vec<String>, ParseError> {
    let value = value.trim();
    let Some(inner) = value.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) else {
        return Err(error(line, format!("{key} must be a [list]")));
    };
    inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            if item.starts_with('"') {
                unquote(item, line)
            } else {
                Ok(item.to_string())
            }
        })
        .collect()
}

fn header(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim();
    if let Some(inner) = trimmed
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
    {
        return Some((inner.trim().to_string(), true));
    }
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
        return Some((inner.trim().to_string(), false));
    }
    None
}

fn section_for(name: &str, array: bool, line: usize) -> Result<Section, ParseError> {
    let name = name.strip_prefix("fighter.").unwrap_or(name);
    match (name, array) {
        ("fighter", false) => Ok(Section::Fighter),
        ("article", true) => Ok(Section::Article),
        ("article.hurtbox", true) => Ok(Section::ArticleHurtbox),
        ("article.lvd_vertex", true) => Ok(Section::ArticleLvdVertex),
        ("kirby", false) => Ok(Section::Kirby),
        ("kirby.motion", true) => Ok(Section::Motion),
        ("kirby.mesh", true) => Ok(Section::Mesh),
        ("global_motion", true) => Ok(Section::GlobalMotion),
        ("grab", true) => Ok(Section::Grab),
        ("hud", false) => Ok(Section::Hud),
        ("final", false) => Ok(Section::Final),
        ("tether", false) => Ok(Section::Tether),
        ("tether.box", true) => Ok(Section::TetherBox),
        ("partner_params", false) => Ok(Section::PartnerParams),
        ("commands", false) => Ok(Section::Commands),
        ("command", true) => Ok(Section::Command),
        ("move_list", false) => Ok(Section::MoveList),
        ("params", false) => Ok(Section::Params {
            slot: None,
            op: ParamOp::Set,
        }),
        (rest, false) if rest.starts_with("params.") => {
            let mut slot = None;
            let mut op = ParamOp::Set;
            for part in rest["params.".len()..].split('.') {
                if part == "mul" {
                    op = ParamOp::Mul;
                } else if let Some(number) = part.strip_prefix('c') {
                    slot = Some(parse_int::<i32>(number, line, "costume")?);
                } else {
                    return Err(error(line, format!("unknown params table [{rest}]")));
                }
            }
            Ok(Section::Params { slot, op })
        }
        _ => Err(error(
            line,
            format!("unknown table [{}{name}{}]", if array { "[" } else { "" }, if array { "]" } else { "" }),
        )),
    }
}

pub fn parse(text: &str) -> Result<FighterDeclaration, ParseError> {
    let mut declaration = FighterDeclaration {
        color_count: DEFAULT_COLOR_COUNT,
        ..FighterDeclaration::default()
    };
    let mut section = Section::Fighter;
    let mut saw_costumes = false;
    for (index, raw) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some((name, array)) = header(line) {
            section = section_for(&name, array, line_number)?;
            match section {
                Section::Article => declaration.articles.push(ArticleDeclaration::default()),
                Section::ArticleHurtbox => match declaration.articles.last_mut() {
                    Some(article) => article.hurtboxes.push(Hurtbox::default()),
                    None => {
                        return Err(error(
                            line_number,
                            "[[article.hurtbox]] must follow the [[article]] it belongs to",
                        ))
                    }
                },
                Section::ArticleLvdVertex => match declaration.articles.last_mut() {
                    Some(article) => article
                        .lvd
                        .get_or_insert_with(ArticleLvd::default)
                        .vertices
                        .push(LvdVertex::new(UNSET_VERTEX, "")),
                    None => {
                        return Err(error(
                            line_number,
                            "[[article.lvd_vertex]] must follow the [[article]] it belongs to",
                        ))
                    }
                },
                Section::Kirby => {
                    declaration.kirby.get_or_insert_with(KirbyDeclaration::default);
                }
                Section::Motion => declaration
                    .kirby
                    .get_or_insert_with(KirbyDeclaration::default)
                    .motions
                    .push(MotionDeclaration::default()),
                Section::GlobalMotion => declaration
                    .global_motions
                    .push(MotionDeclaration::default()),
                Section::Grab => declaration.grabs.push(GrabRule::new(-1)),
                Section::Command => declaration.commands.push(crate::command_input::Command::new("", "")),
                Section::Tether => {
                    declaration.tether.get_or_insert_with(Default::default);
                }
                Section::MoveList => {
                    declaration.move_list.get_or_insert_with(Default::default);
                }
                Section::TetherBox => declaration
                    .tether
                    .get_or_insert_with(Default::default)
                    .boxes
                    .push(crate::tether::TetherBox::new([0.0; 2], [0.0; 2])),
                Section::Mesh => declaration
                    .kirby
                    .get_or_insert_with(KirbyDeclaration::default)
                    .meshes
                    .push((String::new(), true)),
                _ => {}
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(error(line_number, "expected key = value"));
        };
        let key = key.trim();
        let value = value.trim();
        match section {
            Section::Fighter => match key {
                "name" => declaration.name = unquote(value, line_number)?,
                "base" => declaration.base = unquote(value, line_number)?,
                "display_name" => declaration.display_name = Some(unquote(value, line_number)?),
                "costumes" | "color_count" => {
                    declaration.color_count = parse_int(value, line_number, key)?;
                    saw_costumes = true;
                }
                "color_start" => declaration.color_start = parse_int(value, line_number, key)?,
                "ui_chara" => declaration.ui_chara = Some(unquote(value, line_number)?),
                "fighter_kind_name" => {
                    declaration.fighter_kind_name = Some(unquote(value, line_number)?)
                }
                "resource_name" => declaration.resource_name = Some(unquote(value, line_number)?),
                "base_resource_name" => {
                    declaration.base_resource_name = Some(unquote(value, line_number)?)
                }
                "series" => declaration.series = Some(unquote(value, line_number)?),
                "disp_order" => declaration.disp_order = Some(parse_int(value, line_number, key)?),
                "save_no" => declaration.save_no = Some(parse_int(value, line_number, key)?),
                "exhibit_year" => {
                    declaration.exhibit_year = Some(parse_int(value, line_number, key)?)
                }
                "narration" => declaration.narration = Some(unquote(value, line_number)?),
                "staffroll" => declaration.staffroll = parse_bool(value, line_number, key)?,
                "item_hand" => {
                    let text = unquote(value, line_number)?;
                    declaration.item_hand = Some(crate::item_hand::ItemHand::parse(&text).ok_or_else(|| {
                        error(line_number, "item_hand must be \"left\" or \"right\"")
                    })?);
                }
                "partner" if value.starts_with('"') => {
                    declaration.partner_with = Some(unquote(value, line_number)?);
                    declaration.partner = Some(true);
                }
                "partner" => declaration.partner = Some(parse_bool(value, line_number, key)?),
                "switch" => declaration.switch_with = parse_list(value, line_number, key)?,
                "css" => declaration.own_css = !parse_bool(value, line_number, key)?,
                "jingle" => declaration.jingle = Some(unquote(value, line_number)?),
                "owns_param_resources" => {
                    declaration.owns_param_resources = parse_bool(value, line_number, key)?
                }
                "effect_namespace" => {
                    declaration.effect_namespace = parse_int(value, line_number, key)?
                }
                "article_namespace" => {
                    declaration.article_namespace = parse_int(value, line_number, key)?
                }
                _ => return Err(error(line_number, format!("unknown key {key}"))),
            },
            Section::Article => {
                let article = declaration.articles.last_mut().expect("pushed at header");
                match key {
                    "name" => article.name = unquote(value, line_number)?,
                    "from" => {
                        let from = unquote(value, line_number)?;
                        let Some((owner, weapon)) = from.split_once('/') else {
                            return Err(error(line_number, "from must be \"fighter/weapon\""));
                        };
                        if owner.is_empty() || weapon.is_empty() {
                            return Err(error(line_number, "from must be \"fighter/weapon\""));
                        }
                        article.from = Some((owner.to_string(), weapon.to_string()));
                    }
                    "base_article" => article.base_article = Some(unquote(value, line_number)?),
                    "kirby" => article.kirby = parse_bool(value, line_number, key)?,
                    "lvd" => {
                        article.lvd.get_or_insert_with(ArticleLvd::default).file =
                            unquote(value, line_number)?
                    }
                    _ => return Err(error(line_number, format!("unknown article key {key}"))),
                }
            }
            Section::ArticleHurtbox => {
                let hurtbox = declaration
                    .articles
                    .last_mut()
                    .and_then(|article| article.hurtboxes.last_mut())
                    .expect("pushed at header");
                hurtbox_key(hurtbox, key, value, line_number)?;
            }
            Section::ArticleLvdVertex => {
                let vertex = declaration
                    .articles
                    .last_mut()
                    .and_then(|article| article.lvd.as_mut())
                    .and_then(|lvd| lvd.vertices.last_mut())
                    .expect("pushed at header");
                match key {
                    "vertex" => vertex.vertex = parse_int(value, line_number, key)?,
                    "joint" => vertex.joint = unquote(value, line_number)?,
                    "offset" => vertex.offset = Some(parse_vector(value, line_number, key)?),
                    "joint_left" => vertex.joint_left = Some(unquote(value, line_number)?),
                    _ => return Err(error(line_number, format!("unknown lvd_vertex key {key}"))),
                }
            }
            Section::Kirby => {
                let kirby = declaration.kirby.as_mut().expect("inserted at header");
                match key {
                    "statuses" => kirby.statuses = parse_int(value, line_number, key)?,
                    "first" => kirby.first = Some(parse_int(value, line_number, key)?),
                    "full_model" => kirby.full_model = parse_bool(value, line_number, key)?,
                    "model" => kirby.model = Some(unquote(value, line_number)?),
                    _ => return Err(error(line_number, format!("unknown kirby key {key}"))),
                }
            }
            Section::Motion => {
                let motion = declaration
                    .kirby
                    .as_mut()
                    .and_then(|kirby| kirby.motions.last_mut())
                    .expect("pushed at header");
                motion_key(motion, key, value, line_number)?;
            }
            Section::GlobalMotion => {
                let motion = declaration
                    .global_motions
                    .last_mut()
                    .expect("pushed at header");
                motion_key(motion, key, value, line_number)?;
            }
            Section::Grab => {
                let grab = declaration.grabs.last_mut().expect("pushed at header");
                grab_key(grab, key, value, line_number)?;
            }
            Section::Mesh => {
                let mesh = declaration
                    .kirby
                    .as_mut()
                    .and_then(|kirby| kirby.meshes.last_mut())
                    .expect("pushed at header");
                match key {
                    "name" => mesh.0 = unquote(value, line_number)?,
                    "visible" => mesh.1 = parse_bool(value, line_number, key)?,
                    _ => return Err(error(line_number, format!("unknown mesh key {key}"))),
                }
            }
            Section::Hud => match key {
                "from" => {
                    let donor = unquote(value, line_number)?;
                    crate::hud::check(&donor).map_err(|message| error(line_number, message))?;
                    declaration.hud = Some(donor);
                }
                "layout" => {
                    let path = unquote(value, line_number)?;
                    crate::hud::check_layout(&path).map_err(|message| error(line_number, message))?;
                    declaration.hud_layout = Some(path);
                }
                _ => return Err(error(line_number, format!("unknown hud key {key}"))),
            },
            Section::Final => match key {
                "from" => {
                    let donor = unquote(value, line_number)?;
                    crate::final_smash::check(&donor).map_err(|message| error(line_number, message))?;
                    declaration.final_smash = Some(donor);
                }
                _ => return Err(error(line_number, format!("unknown final key {key}"))),
            },
            Section::Tether => match key {
                "from" => {
                    let donor = unquote(value, line_number)?;
                    crate::tether::check(&donor).map_err(|message| error(line_number, message))?;
                    declaration.tether.as_mut().expect("inserted at header").from = Some(donor);
                }
                _ => return Err(error(line_number, format!("unknown tether key {key}"))),
            },
            Section::TetherBox => {
                let shape = declaration
                    .tether
                    .as_mut()
                    .and_then(|tether| tether.boxes.last_mut())
                    .expect("pushed at header");
                match key {
                    "p1" => shape.p1 = parse_pair(value, line_number, key)?,
                    "p2" => shape.p2 = parse_pair(value, line_number, key)?,
                    _ => return Err(error(line_number, format!("unknown tether.box key {key}"))),
                }
            }
            Section::Commands => commands_key(&mut declaration.command_settings, key, value, line_number)?,
            Section::Command => command_key(
                declaration.commands.last_mut().expect("pushed at header"),
                key,
                value,
                line_number,
            )?,
            Section::MoveList => {
                let list = declaration.move_list.as_mut().expect("inserted at header");
                match key {
                    "pages" => list.pages = Some(parse_int(value, line_number, key)?),
                    "image_pages" => {
                        list.image_pages = Some(
                            parse_list(value, line_number, key)?
                                .iter()
                                .map(|page| parse_int(page, line_number, "an image page"))
                                .collect::<Result<Vec<u8>, _>>()?,
                        )
                    }
                    _ => return Err(error(line_number, format!("unknown move_list key {key}"))),
                }
            }
            Section::PartnerParams => declaration.partner_params.push(
                crate::partner_params::resolve(key, value).map_err(|message| error(line_number, message))?,
            ),
            Section::Params { slot, op } => {
                let value = value.trim();
                let parsed = if value.contains('.') || value.contains('e') {
                    value
                        .parse::<f64>()
                        .map(ParamValue::Float)
                        .map_err(|_| error(line_number, format!("{key} must be a number")))?
                } else {
                    ParamValue::Int(parse_int(value, line_number, key)?)
                };
                if op == ParamOp::Mul && matches!(parsed, ParamValue::Int(_)) {
                    return Err(error(line_number, format!("{key}: a multiplier must be a decimal")));
                }
                declaration.params.push(ParamDeclaration {
                    name: key.to_string(),
                    slot,
                    op,
                    value: parsed,
                });
            }
        }
    }
    finish(declaration, saw_costumes)
}

fn finish(mut declaration: FighterDeclaration, saw_costumes: bool) -> Result<FighterDeclaration, ParseError> {
    if declaration.name.is_empty() {
        return Err(error(0, "name is required"));
    }
    if declaration.base.is_empty() {
        return Err(error(0, "base is required"));
    }
    if !valid_name(&declaration.name) {
        return Err(error(0, "name must be lowercase letters, digits and underscores"));
    }
    if declaration.color_count == 0 || declaration.color_start + declaration.color_count > 256 {
        return Err(error(0, "costumes must be 1 to 256 including color_start"));
    }
    let _ = saw_costumes;
    for article in &declaration.articles {
        if article.name.is_empty() {
            return Err(error(0, "an article needs a name"));
        }
        if article.from.is_none() && article.base_article.is_none() {
            return Err(error(
                0,
                format!("article {} needs from = \"fighter/weapon\" or base_article", article.name),
            ));
        }
        if article.from.is_some() && article.base_article.is_some() {
            return Err(error(
                0,
                format!("article {} has both from and base_article", article.name),
            ));
        }
        if let Some(lvd) = &article.lvd {
            check_article_lvd(article, lvd)?;
        }
        if article.hurtboxes.is_empty() {
            continue;
        }
        if article.from.is_none() {
            return Err(error(
                0,
                format!("article {}: hurtboxes need from = \"fighter/weapon\"", article.name),
            ));
        }
        if article.hurtboxes.len() > MAX_PER_ARTICLE {
            return Err(error(
                0,
                format!("article {}: at most {MAX_PER_ARTICLE} hurtboxes", article.name),
            ));
        }
        for hurtbox in &article.hurtboxes {
            hurtbox
                .check()
                .map_err(|message| error(0, format!("article {}: {message}", article.name)))?;
        }
    }
    if let Some(kirby) = &declaration.kirby {
        if kirby.statuses < 0 {
            return Err(error(0, "kirby statuses must not be negative"));
        }
        if kirby.first.is_some_and(|first| first <= 0) {
            return Err(error(0, "kirby first must be a positive status kind"));
        }
        for motion in &kirby.motions {
            if motion.name.is_empty() || motion.animation.is_empty() {
                return Err(error(0, "a kirby motion needs name and animation"));
            }
            if motion.victim {
                return Err(error(0, format!("kirby motion {}: victim is for global motions", motion.name)));
            }
        }
        for (mesh, _) in &kirby.meshes {
            if mesh.is_empty() {
                return Err(error(0, "a kirby mesh needs a name"));
            }
        }
    }
    for motion in &declaration.global_motions {
        if motion.name.is_empty() {
            return Err(error(0, "a global motion needs a name"));
        }
        if motion.animation.is_empty() && motion.template.is_none() && !motion.victim {
            return Err(error(
                0,
                format!(
                    "global motion {} needs a template, an animation or victim = true",
                    motion.name
                ),
            ));
        }
    }
    let start = crate::status_starts::status_start(&declaration.base);
    let base = declaration.base.clone();
    let own = |status: i32| -> Result<i32, ParseError> {
        if status < 0 || status & crate::status_starts::OWN_STATUS == 0 {
            return Ok(status);
        }
        start
            .map(|start| start + (status & !crate::status_starts::OWN_STATUS))
            .ok_or_else(|| error(0, format!("own statuses need a vanilla base; {base} has no status table the engine knows")))
    };
    for grab in &mut declaration.grabs {
        grab.status = own(grab.status)?;
        if let Some(then) = grab.then {
            grab.then = Some(own(then)?);
        }
    }
    for grab in &declaration.grabs {
        if grab.status < 0 {
            return Err(error(0, "a grab needs status, the grabber status its CATCH box belongs to"));
        }
    }
    if let Some(tether) = &declaration.tether {
        tether.check().map_err(|message| error(0, message))?;
    }
    if let Some(list) = &declaration.move_list {
        list.check().map_err(|message| error(0, message))?;
    }
    check_commands(&declaration)?;
    if declaration.switch_with.len() > 2 {
        return Err(error(
            0,
            format!("{} switches with at most two other fighters", declaration.name),
        ));
    }
    if !declaration.switch_with.is_empty() && matches!(declaration.base.as_str(), "popo" | "nana") {
        return Err(error(0, format!("{} is on the Ice Climbers, who cannot switch", declaration.name)));
    }
    if declaration.switch_with.iter().any(|member| *member == declaration.name) {
        return Err(error(0, format!("{} cannot switch with itself", declaration.name)));
    }
    if declaration.partner_with.as_deref() == Some(declaration.name.as_str()) {
        return Err(error(
            0,
            format!("{} cannot name itself as its partner; partner = true gives it a copy of itself", declaration.name),
        ));
    }
    if declaration.partner == Some(true) && !declaration.switch_with.is_empty() {
        return Err(error(
            0,
            format!("{} has both a partner and a switch group; a player slot holds one or the other", declaration.name),
        ));
    }
    if !declaration.partner_params.is_empty() && !declaration.has_partner() {
        return Err(error(0, format!("{} has partner_params but no partner", declaration.name)));
    }
    Ok(declaration)
}

const UNSET_VERTEX: u32 = u32::MAX;

fn check_article_lvd(article: &ArticleDeclaration, lvd: &ArticleLvd) -> Result<(), ParseError> {
    let fail = |message: String| Err(error(0, format!("article {}: {message}", article.name)));
    if article.from.is_none() {
        return fail("lvd needs from = \"fighter/weapon\"".to_string());
    }
    if lvd.file.trim().is_empty() {
        return fail("[[article.lvd_vertex]] needs lvd = \"file.lvd\" on its article".to_string());
    }
    if lvd.vertices.iter().any(|vertex| vertex.vertex == UNSET_VERTEX) {
        return fail("every [[article.lvd_vertex]] needs vertex".to_string());
    }
    match lvd.check() {
        Ok(()) => Ok(()),
        Err(message) => fail(message),
    }
}

fn parse_status(value: &str, line: usize, key: &str) -> Result<i32, ParseError> {
    let value = value.trim();
    let status = if value.starts_with('"') {
        let name = unquote(value, line)?;
        if let Some(index) = crate::status_starts::own_status(&name) {
            if key == "victim" {
                return Err(error(line, "victim is a status of the fighter caught; own N is for status and then"));
            }
            return Ok(crate::status_starts::OWN_STATUS | index);
        }
        status_kind(&name).ok_or_else(|| {
            error(
                line,
                format!("{key}: {name:?} is not a known status name; write own N for one of your own, or its number"),
            )
        })?
    } else {
        parse_int::<i32>(value, line, key)?
    };
    if status < 0 {
        return Err(error(line, format!("{key} must not be negative")));
    }
    Ok(status)
}

fn grab_key(grab: &mut GrabRule, key: &str, value: &str, line_number: usize) -> Result<(), ParseError> {
    match key {
        "status" => grab.status = parse_status(value, line_number, key)?,
        "victim" => grab.victim = Some(parse_status(value, line_number, key)?),
        "then" => grab.then = Some(parse_status(value, line_number, key)?),
        "node" => grab.node = Some(unquote(value, line_number)?),
        "offset" => {
            let parts = parse_list(value, line_number, key)?;
            if parts.len() != 3 {
                return Err(error(line_number, "offset must be [x, y, z]"));
            }
            for (slot, part) in grab.offset.iter_mut().zip(&parts) {
                *slot = part
                    .parse::<f32>()
                    .map_err(|_| error(line_number, "offset must be [x, y, z] numbers"))?;
            }
        }
        "catch" => grab.catch = parse_bool(value, line_number, key)?,
        _ => return Err(error(line_number, format!("unknown grab key {key}"))),
    }
    Ok(())
}

fn commands_key(
    settings: &mut crate::command_input::Settings,
    key: &str,
    value: &str,
    line: usize,
) -> Result<(), ParseError> {
    let frames = |value: &str| -> Result<u8, ParseError> {
        let frames: u8 = parse_int(value, line, key)?;
        if frames == 0 {
            return Err(error(line, format!("{key} must be at least 1")));
        }
        Ok(frames)
    };
    match key {
        "vanilla" => settings.vanilla = parse_bool(value, line, key)?,
        "special_command_life_max" => settings.life = Some(frames(value)?),
        "super_special_command_life_max" => settings.super_life = Some(frames(value)?),
        "special_command_ab_button_life" => settings.button_life = Some(frames(value)?),
        "special_command_neutral_threshold" => {
            let threshold = value
                .parse::<f32>()
                .ok()
                .filter(|threshold| (0.0..1.0).contains(threshold))
                .ok_or_else(|| error(line, format!("{key} must be a number from 0 to below 1")))?;
            settings.neutral_threshold = Some(threshold);
        }
        "special_command_stick_range" => settings.stick_range = Some(parse_stick_range(value, line, key)?),
        _ => return Err(error(line, format!("unknown commands key {key}"))),
    }
    Ok(())
}

fn command_key(
    command: &mut crate::command_input::Command,
    key: &str,
    value: &str,
    line: usize,
) -> Result<(), ParseError> {
    match key {
        "name" => command.name = unquote(value, line)?,
        "input" => {
            command.input = if value.starts_with('"') { unquote(value, line)? } else { value.to_string() }
        }
        "button" => {
            command.button = crate::command_input::Button::parse(&unquote(value, line)?)
                .map_err(|message| error(line, message))?
        }
        "super" => command.super_command = parse_bool(value, line, key)?,
        "life" => command.life = Some(parse_int(value, line, key)?),
        "neutral_resets" => command.neutral_resets = parse_bool(value, line, key)?,
        "reverse" => command.reverse = parse_bool(value, line, key)?,
        "cpu" => {
            command.cpu = value
                .parse::<f32>()
                .ok()
                .filter(|chance| (0.0..=1.0).contains(chance))
                .ok_or_else(|| error(line, format!("{key} must be a chance from 0 to 1")))?
        }
        _ => return Err(error(line, format!("unknown command key {key}"))),
    }
    Ok(())
}

fn parse_stick_range(value: &str, line: usize, key: &str) -> Result<crate::command_input::StickRange, ParseError> {
    let value = value.trim();
    let Some(inner) = value.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) else {
        return Err(error(line, format!("{key} must be a list of 7 [min, max] pairs")));
    };
    let mut pairs = Vec::new();
    let mut rest = inner.trim();
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            return Err(error(line, format!("{key}: a pair is missing its ]")));
        };
        pairs.push(parse_pair(&rest[open..open + close + 1], line, key)?);
        rest = &rest[open + close + 1..];
    }
    let range: crate::command_input::StickRange = pairs
        .try_into()
        .map_err(|_| error(line, format!("{key} must have exactly {} [min, max] pairs", crate::command_input::SECTORS)))?;
    crate::command_input::check_stick_range(&range).map_err(|message| error(line, message))?;
    Ok(range)
}

fn check_commands(declaration: &FighterDeclaration) -> Result<(), ParseError> {
    if declaration.commands.len() > crate::command_input::MAX_COMMANDS {
        return Err(error(0, format!("at most {} commands", crate::command_input::MAX_COMMANDS)));
    }
    for (index, command) in declaration.commands.iter().enumerate() {
        command.check().map_err(|message| error(0, message))?;
        if !valid_name(&command.name) {
            return Err(error(0, format!("command {}: name must be lowercase letters, digits and underscores", command.name)));
        }
        if declaration.commands[..index].iter().any(|other| other.name == command.name) {
            return Err(error(0, format!("command {} is declared twice", command.name)));
        }
    }
    Ok(())
}

fn parse_pair(value: &str, line: usize, key: &str) -> Result<[f32; 2], ParseError> {
    let parts = parse_list(value, line, key)?;
    if parts.len() != 2 {
        return Err(error(line, format!("{key} must be [x, y]")));
    }
    let mut out = [0.0f32; 2];
    for (slot, part) in out.iter_mut().zip(&parts) {
        *slot = part
            .parse::<f32>()
            .map_err(|_| error(line, format!("{key} must be [x, y] numbers")))?;
    }
    Ok(out)
}

fn parse_vector(value: &str, line: usize, key: &str) -> Result<[f32; 3], ParseError> {
    let parts = parse_list(value, line, key)?;
    if parts.len() != 3 {
        return Err(error(line, format!("{key} must be [x, y, z]")));
    }
    let mut out = [0.0f32; 3];
    for (slot, part) in out.iter_mut().zip(&parts) {
        *slot = part
            .parse::<f32>()
            .map_err(|_| error(line, format!("{key} must be [x, y, z] numbers")))?;
    }
    Ok(out)
}

fn parse_field(field: Field, value: &str, line: usize) -> Result<i32, ParseError> {
    let value = value.trim();
    if value.starts_with('"') {
        let name = unquote(value, line)?;
        return field.value(&name).ok_or_else(|| {
            error(
                line,
                format!("{}: {name:?} is not one of {}", field.key(), field.names()),
            )
        });
    }
    parse_int::<i32>(value, line, field.key())
}

fn hurtbox_key(hurtbox: &mut Hurtbox, key: &str, value: &str, line_number: usize) -> Result<(), ParseError> {
    match key {
        "node" => hurtbox.node = unquote(value, line_number)?,
        "size" => {
            hurtbox.size = value
                .trim()
                .parse::<f32>()
                .map_err(|_| error(line_number, "size must be a number"))?
        }
        "offset" => hurtbox.offset = parse_vector(value, line_number, key)?,
        "offset2" => hurtbox.offset2 = Some(parse_vector(value, line_number, key)?),
        "part" => hurtbox.part = parse_field(Field::Part, value, line_number)?,
        "height" => hurtbox.height = parse_field(Field::Height, value, line_number)?,
        "status" => hurtbox.status = parse_field(Field::Status, value, line_number)?,
        "shape" => hurtbox.shape = Some(parse_field(Field::Shape, value, line_number)?),
        _ => return Err(error(line_number, format!("unknown hurtbox key {key}"))),
    }
    Ok(())
}

fn motion_key(
    motion: &mut MotionDeclaration,
    key: &str,
    value: &str,
    line_number: usize,
) -> Result<(), ParseError> {
    match key {
        "name" => motion.name = unquote(value, line_number)?,
        "animation" => motion.animation = unquote(value, line_number)?,
        "template" => motion.template = Some(unquote(value, line_number)?),
        "scripts" => {
            let stem = unquote(value, line_number)?;
            motion.game.get_or_insert_with(|| format!("game_{stem}"));
            motion.sound.get_or_insert_with(|| format!("sound_{stem}"));
            motion.effect.get_or_insert_with(|| format!("effect_{stem}"));
            motion
                .expression
                .get_or_insert_with(|| format!("expression_{stem}"));
        }
        "game" => motion.game = Some(unquote(value, line_number)?),
        "sound" => motion.sound = Some(unquote(value, line_number)?),
        "effect" => motion.effect = Some(unquote(value, line_number)?),
        "expression" => motion.expression = Some(unquote(value, line_number)?),
        "flags" => motion.flags = parse_list(value, line_number, key)?,
        "loop" => {
            if parse_bool(value, line_number, key)? {
                motion.flags.push("loop".to_string());
            }
        }
        "blend_frames" => motion.blend_frames = Some(parse_int(value, line_number, key)?),
        "cancel_frame" => motion.cancel_frame = Some(parse_int(value, line_number, key)?),
        "xlu" => {
            let pair = parse_list(value, line_number, key)?;
            if pair.len() != 2 {
                return Err(error(line_number, "xlu must be [start, end]"));
            }
            motion.xlu = Some((
                parse_int(&pair[0], line_number, key)?,
                parse_int(&pair[1], line_number, key)?,
            ));
        }
        "no_stop_intp" => motion.no_stop_intp = Some(parse_bool(value, line_number, key)?),
        "animation_unk" => motion.animation_unk = Some(parse_int(value, line_number, key)?),
        "no_extra" => motion.no_extra = parse_bool(value, line_number, key)?,
        "victim" => motion.victim = parse_bool(value, line_number, key)?,
        _ => return Err(error(line_number, format!("unknown motion key {key}"))),
    }
    Ok(())
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub fn parse_all(text: &str) -> Result<Vec<FighterDeclaration>, ParseError> {
    let blocks = crate::manifest::blocks(text, "fighter")
        .map_err(|line| error(line, "a key before the first [[fighter]] belongs to nothing"))?;
    blocks
        .iter()
        .map(|block| {
            parse(&block.text).map_err(|mut failure| {
                if failure.line > 0 {
                    failure.line += block.first_line - 1;
                }
                failure
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KAWASHIMA: &str = r#"
[fighter]
name = "kawashima"          # files under fighter/kawashima
base = "marth"
display_name = "Kawashima"
costumes = 1
series = "tekken"
disp_order = 40
narration = "vc_narration_characall_kawashima"

[[article]]
name = "coin"
from = "simon/cross"

[[article]]
name = "cshot"
base_article = "cshot"

[kirby]
statuses = 4
full_model = true

[[kirby.motion]]
name = "specialnstart"
animation = "kawashima_specialnstart"
scripts = "kawashima_specialnstart"

[[kirby.motion]]
name = "specialnloop"
animation = "kawashima_specialnloop"
scripts = "kawashima_specialnloop"
loop = true
xlu = [3, 9]

[[kirby.mesh]]
name = "hat_visor"
visible = false

[params]
run_speed_max = 1.645
attack_combo_max = 3

[params.mul]
jump_y = 1.1

[params.c07]
scale = 0.95
"#;

    #[test]
    fn the_worked_example_parses_into_every_table() {
        let fighter = parse(KAWASHIMA).unwrap();
        assert_eq!(fighter.name, "kawashima");
        assert_eq!(fighter.base, "marth");
        assert_eq!(fighter.color_count, 1);
        assert_eq!(fighter.color_start, 0);
        assert_eq!(fighter.ui_chara(), "ui_chara_kawashima");
        assert_eq!(fighter.fighter_kind_name(), "fighter_kind_kawashima");
        assert_eq!(fighter.resource_name(), "kawashima");
        assert_eq!(fighter.base_resource_name(), "marth");
        assert_eq!(fighter.series(), "ui_series_tekken");
        assert_eq!(fighter.disp_order, Some(40));
        assert_eq!(fighter.narration.as_deref(), Some("vc_narration_characall_kawashima"));
        assert_eq!(fighter.articles.len(), 2);
        assert_eq!(
            fighter.articles[0].from,
            Some(("simon".to_string(), "cross".to_string()))
        );
        assert_eq!(fighter.articles[1].base_article.as_deref(), Some("cshot"));
        let kirby = fighter.kirby.as_ref().unwrap();
        assert_eq!(kirby.statuses, 4);
        assert!(kirby.full_model);
        assert_eq!(kirby.motions.len(), 2);
        assert_eq!(
            kirby.motions[0].script("game", None).as_deref(),
            Some("game_kawashima_specialnstart")
        );
        assert_eq!(
            kirby.motions[0].script("expression", None).as_deref(),
            Some("expression_kawashima_specialnstart")
        );
        assert_eq!(kirby.motions[1].flags, vec!["loop".to_string()]);
        assert_eq!(kirby.motions[1].xlu, Some((3, 9)));
        assert_eq!(kirby.meshes, vec![("hat_visor".to_string(), false)]);
        assert_eq!(fighter.params.len(), 4);
        assert_eq!(fighter.params[0].value, ParamValue::Float(1.645));
        assert_eq!(fighter.params[1].value, ParamValue::Int(3));
        assert_eq!(fighter.params[2].op, ParamOp::Mul);
        assert_eq!(fighter.params[3].slot, Some(7));
    }

    #[test]
    fn the_flat_form_needs_no_fighter_header_and_defaults_to_eight_costumes() {
        let fighter = parse("name = \"wawa\"\nbase = \"samus\"\n").unwrap();
        assert_eq!(fighter.color_count, DEFAULT_COLOR_COUNT);
        assert_eq!(fighter.series(), "ui_series_");
        assert!(fighter.kirby.is_none());
        assert!(fighter.articles.is_empty());
    }

    #[test]
    fn a_missing_name_or_base_is_refused() {
        assert_eq!(parse("base = \"samus\"\n").unwrap_err().message, "name is required");
        assert_eq!(parse("name = \"wawa\"\n").unwrap_err().message, "base is required");
    }

    #[test]
    fn an_unknown_key_names_its_line() {
        let failure = parse("name = \"wawa\"\nbase = \"samus\"\ncolour = 3\n").unwrap_err();
        assert_eq!(failure.line, 3);
        assert_eq!(failure.message, "unknown key colour");
    }

    #[test]
    fn a_number_past_its_range_says_so() {
        let failure = parse("name = \"wawa\"\nbase = \"samus\"\ndisp_order = 128\n").unwrap_err();
        assert_eq!(failure.line, 3);
        assert_eq!(failure.message, "disp_order 128 is out of range");
        let failure = parse("name = \"wawa\"\nbase = \"samus\"\ndisp_order = forty\n").unwrap_err();
        assert_eq!(failure.message, "disp_order must be a whole number");
    }

    #[test]
    fn item_hand_takes_left_or_right() {
        let fighter = parse("name = \"wawa\"\nbase = \"luigi\"\nitem_hand = \"left\"\n").unwrap();
        assert_eq!(fighter.item_hand, Some(crate::item_hand::ItemHand::Left));
        assert_eq!(parse("name = \"wawa\"\nbase = \"luigi\"\n").unwrap().item_hand, None);
        let failure = parse("name = \"wawa\"\nbase = \"luigi\"\nitem_hand = \"havel\"\n").unwrap_err();
        assert_eq!(failure.line, 3);
        assert_eq!(failure.message, "item_hand must be \"left\" or \"right\"");
    }

    #[test]
    fn an_unknown_table_is_refused() {
        let failure = parse("name = \"wawa\"\nbase = \"samus\"\n[sound]\nbank = 1\n").unwrap_err();
        assert_eq!(failure.line, 3);
        assert!(failure.message.starts_with("unknown table"));
    }

    #[test]
    fn article_hurtboxes_parse_names_and_numbers() {
        let text = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"rock\"\nfrom = \"pacman/firehydrant\"\n[[article.hurtbox]]\nnode = \"hip\"\nsize = 4\noffset = [0, 4, 0]\noffset2 = [0, 9, 0]\npart = \"COLLISION_PART_HEAD\"\nheight = 2\nstatus = \"xlu\"\n[[article.hurtbox]]\nnode = \"top\"\nsize = 1.5\nshape = \"aabb\"\n";
        let fighter = parse(text).unwrap();
        let boxes = &fighter.articles[0].hurtboxes;
        assert_eq!(boxes.len(), 2);
        assert_eq!(boxes[0].size, 4.0);
        assert_eq!(boxes[0].offset2, Some([0.0, 9.0, 0.0]));
        assert_eq!((boxes[0].part, boxes[0].height, boxes[0].status), (4, 2, 2));
        assert_eq!(boxes[0].resolved_shape(), 2);
        assert_eq!(boxes[1].resolved_shape(), 1);
    }

    #[test]
    fn article_hurtboxes_are_refused_where_the_engine_cannot_serve_them() {
        let head = "name = \"wawa\"\nbase = \"samus\"\n";
        let orphan = format!("{head}[[article.hurtbox]]\nnode = \"hip\"\nsize = 1\n");
        assert!(parse(&orphan).unwrap_err().message.contains("must follow"));
        let renamed = format!("{head}[[article]]\nname = \"x\"\nbase_article = \"cshot\"\n[[article.hurtbox]]\nnode = \"hip\"\nsize = 1\n");
        assert!(parse(&renamed).unwrap_err().message.contains("need from"));
        let sizeless = format!("{head}[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\n[[article.hurtbox]]\nnode = \"hip\"\n");
        assert!(parse(&sizeless).unwrap_err().message.contains("size"));
        let unknown = format!("{head}[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\n[[article.hurtbox]]\nnode = \"hip\"\nsize = 1\nshape = \"cube\"\n");
        assert!(parse(&unknown).unwrap_err().message.contains("sphere, aabb, capsule"));
    }

    #[test]
    fn article_lvd_parses_the_file_and_its_vertices() {
        let text = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"plank\"\nfrom = \"koopajr/cannonball\"\nlvd = \"plank.lvd\"\n[[article.lvd_vertex]]\nvertex = 0\njoint = \"plankl\"\noffset = [0, 6, 4]\n[[article.lvd_vertex]]\nvertex = 1\njoint = \"plankr\"\njoint_left = \"plankl\"\n";
        let fighter = parse(text).unwrap();
        let lvd = fighter.articles[0].lvd.as_ref().unwrap();
        assert_eq!(lvd.file, "plank.lvd");
        assert_eq!(
            lvd.vertices,
            vec![
                LvdVertex::new(0, "plankl").offset(0.0, 6.0, 4.0),
                LvdVertex::new(1, "plankr").joint_left("plankl"),
            ]
        );
    }

    #[test]
    fn article_lvd_is_refused_where_the_engine_cannot_place_it() {
        let head = "name = \"wawa\"\nbase = \"samus\"\n";
        let orphan = format!("{head}[[article.lvd_vertex]]\nvertex = 0\njoint = \"a\"\n");
        assert!(parse(&orphan).unwrap_err().message.contains("must follow"));
        let renamed = format!("{head}[[article]]\nname = \"x\"\nbase_article = \"cshot\"\nlvd = \"x.lvd\"\n[[article.lvd_vertex]]\nvertex = 0\njoint = \"a\"\n");
        assert!(parse(&renamed).unwrap_err().message.contains("lvd needs from"));
        let fileless = format!("{head}[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\n[[article.lvd_vertex]]\nvertex = 0\njoint = \"a\"\n");
        assert!(parse(&fileless).unwrap_err().message.contains("needs lvd ="));
        let vertexless = format!("{head}[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\nlvd = \"x.lvd\"\n[[article.lvd_vertex]]\njoint = \"a\"\n");
        assert!(parse(&vertexless).unwrap_err().message.contains("needs vertex"));
        let alone = format!("{head}[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\nlvd = \"x.lvd\"\n");
        assert!(parse(&alone).unwrap_err().message.contains("at least one lvd_vertex"));
        let pathed = format!("{head}[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\nlvd = \"c00/x.lvd\"\n[[article.lvd_vertex]]\nvertex = 0\njoint = \"a\"\n");
        assert!(parse(&pathed).unwrap_err().message.contains("file name"));
    }

    #[test]
    fn a_move_list_names_its_pages_and_which_show_pictures() {
        let head = "name = \"wawa\"\nbase = \"demon\"\n";
        let fighter = parse(&format!("{head}[move_list]\npages = 6\nimage_pages = [0, 5]\n")).unwrap();
        let list = fighter.move_list.unwrap();
        assert_eq!((list.pages, list.image_pages), (Some(6), Some(vec![0, 5])));
        assert_eq!(parse(head).unwrap().move_list, None);
        let past = parse(&format!("{head}[move_list]\npages = 2\nimage_pages = [2]\n")).unwrap_err();
        assert!(past.message.contains("past the last page"), "{}", past.message);
        let none = parse(&format!("{head}[move_list]\nimage_pages = [0]\n")).unwrap_err();
        assert!(none.message.contains("needs pages"), "{}", none.message);
        let stray = parse(&format!("{head}[move_list]\ncount = 3\n")).unwrap_err();
        assert!(stray.message.contains("unknown move_list key count"), "{}", stray.message);
    }

    #[test]
    fn a_hud_names_a_donor_the_engine_drives() {
        let head = "name = \"wawa\"\nbase = \"samus\"\n";
        let fighter = parse(&format!("{head}[hud]\nfrom = \"pickel\"\n")).unwrap();
        assert_eq!(fighter.hud.as_deref(), Some("pickel"));
        assert_eq!(parse(head).unwrap().hud, None);
        let unknown = parse(&format!("{head}[hud]\nfrom = \"murabito\"\n")).unwrap_err();
        assert_eq!(unknown.line, 4);
        assert!(unknown.message.contains("\"murabito\""), "{}", unknown.message);
        let stray = parse(&format!("{head}[hud]\nmeter = \"jack\"\n")).unwrap_err();
        assert!(stray.message.contains("unknown hud key meter"), "{}", stray.message);
    }

    #[test]
    fn a_hud_layout_is_an_arc_path_beside_or_without_a_donor() {
        let head = "name = \"wawa\"\nbase = \"samus\"\n";
        let own = parse(&format!("{head}[hud]\nlayout = \"ui/layout/hud/wawa/layout.arc\"\n")).unwrap();
        assert_eq!(own.hud_layout.as_deref(), Some("ui/layout/hud/wawa/layout.arc"));
        assert_eq!(own.hud, None);
        let both = parse(&format!(
            "{head}[hud]\nfrom = \"jack\"\nlayout = \"ui/layout/hud/wawa/layout.arc\"\n"
        ))
        .unwrap();
        assert_eq!((both.hud.as_deref(), both.hud_layout.is_some()), (Some("jack"), true));
        let bad = parse(&format!("{head}[hud]\nlayout = \"Wawa.bflyt\"\n")).unwrap_err();
        assert_eq!(bad.line, 4);
        assert!(bad.message.contains("ending in .arc"), "{}", bad.message);
    }

    #[test]
    fn a_switch_lists_one_or_two_other_members_on_any_base_but_the_ice_climbers() {
        let pyra = "[fighter]\nname = \"wawa\"\nbase = \"eflame\"\n";
        let fighter = parse(&format!("{pyra}switch = [\"wawab\"]\n")).unwrap();
        assert_eq!(fighter.switch_with, vec!["wawab".to_string()]);
        assert!(parse(pyra).unwrap().switch_with.is_empty());
        let donkey = "[fighter]\nname = \"wawa\"\nbase = \"donkey\"\n";
        let free = parse(&format!("{donkey}switch = [\"wawab\", \"wawac\"]\n")).unwrap();
        assert_eq!(free.switch_with.len(), 2);
        let message = parse(&format!("{donkey}switch = [\"b\", \"c\", \"d\"]\n")).unwrap_err().message;
        assert!(message.contains("at most two other fighters"), "{message}");
        let popo = "[fighter]\nname = \"wawa\"\nbase = \"popo\"\npartner = false\n";
        let message = parse(&format!("{popo}switch = [\"wawab\"]\n")).unwrap_err().message;
        assert!(message.contains("Ice Climbers"), "{message}");
        assert!(parse(&format!("{pyra}switch = [\"wawa\"]\n")).is_err());
    }

    #[test]
    fn an_ice_climbers_clone_has_a_partner_unless_it_opts_out() {
        let popo = "[fighter]
name = \"wawa\"
base = \"popo\"
";
        assert!(parse(popo).unwrap().has_partner());
        assert!(!parse(&format!("{popo}partner = false
")).unwrap().has_partner());
        let donkey = "[fighter]
name = \"wawa\"
base = \"donkey\"
";
        assert!(!parse(donkey).unwrap().has_partner());
        let copy = parse(&format!("{donkey}partner = true
")).unwrap();
        assert!(copy.has_partner() && copy.partner_with.is_none());
        let other = parse(&format!("{donkey}partner = \"wawab\"
")).unwrap();
        assert!(other.has_partner());
        assert_eq!(other.partner_with.as_deref(), Some("wawab"));
        let message = parse(&format!("{donkey}partner = \"wawa\"
")).unwrap_err().message;
        assert!(message.contains("cannot name itself"), "{message}");
        let pyra = "[fighter]
name = \"wawa\"
base = \"eflame\"
switch = [\"wawab\"]
";
        let message = parse(&format!("{pyra}partner = true
")).unwrap_err().message;
        assert!(message.contains("both a partner and a switch group"), "{message}");
    }

    #[test]
    fn partner_params_name_the_ice_climbers_words() {
        let head = "[fighter]\nname = \"wawa\"\nbase = \"donkey\"\npartner = true\n";
        let fighter = parse(&format!("{head}\n[partner_params]\nnana_delay = 20\njostle_data_nana.speed_z = 0.5\n")).unwrap();
        assert_eq!(fighter.partner_params.len(), 2);
        assert_eq!(fighter.partner_params[0].offset, 0x140);
        assert_eq!(fighter.partner_params[0].bits, 20);
        assert_eq!(fighter.partner_params[1].offset, 0x108);
        let failure = parse(&format!("{head}\n[partner_params]\nnana_speed = 2.0\n")).unwrap_err();
        assert_eq!(failure.line, 7);
        assert!(failure.message.contains("not one of the Ice Climbers' partner params"), "{}", failure.message);
        let alone = "[fighter]\nname = \"wawa\"\nbase = \"donkey\"\n\n[partner_params]\nnana_delay = 20\n";
        assert_eq!(parse(alone).unwrap_err().message, "wawa has partner_params but no partner");
    }

    #[test]
    fn commands_take_the_vanilla_tuning_names_and_their_own_inputs() {
        let head = "[fighter]\nname = \"wawa\"\nbase = \"mario\"\n";
        let range = "[[-20.0, 20.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0]]";
        let text = format!(
            "{head}\n[commands]\nvanilla = true\nspecial_command_life_max = 12\n\
             special_command_neutral_threshold = 0.25\nspecial_command_stick_range = {range}\n\
             \n[[command]]\nname = \"geyser\"\ninput = \"21416\"\nbutton = \"special\"\nsuper = true\nreverse = true\ncpu = 0.3\n\
             \n[[command]]\nname = \"dash\"\ninput = 656\nbutton = \"none\"\nlife = 8\n"
        );
        let fighter = parse(&text).unwrap();
        let settings = fighter.command_settings;
        assert!(settings.vanilla);
        assert_eq!(settings.life, Some(12));
        assert_eq!(settings.neutral_threshold, Some(0.25));
        assert_eq!(settings.stick_range.unwrap()[3], [110.0, 160.0]);
        assert_eq!(fighter.commands.len(), 2);
        assert_eq!(fighter.commands[0].input, "21416");
        assert!(fighter.commands[0].super_command && fighter.commands[0].reverse);
        assert_eq!(fighter.commands[0].cpu, 0.3);
        assert_eq!(fighter.commands[1].button, crate::command_input::Button::None);
        assert_eq!(fighter.commands[1].life, Some(8));
        let no_button = format!("{head}\n[[command]]\nname = \"a\"\ninput = \"656\"\nbutton = \"none\"\ncpu = 0.5\n");
        assert!(parse(&no_button).unwrap_err().message.contains("cpu needs a button"));
        let odds = format!("{head}\n[[command]]\nname = \"a\"\ninput = \"236\"\ncpu = 2\n");
        assert!(parse(&odds).unwrap_err().message.contains("chance from 0 to 1"));
        let twice = format!("{head}\n[[command]]\nname = \"a\"\ninput = \"236\"\n\n[[command]]\nname = \"a\"\ninput = \"214\"\n");
        assert_eq!(parse(&twice).unwrap_err().message, "command a is declared twice");
        let bad = format!("{head}\n[[command]]\nname = \"a\"\ninput = \"2236\"\n");
        assert!(parse(&bad).unwrap_err().message.contains("repeats 2"));
        let short = format!("{head}\n[commands]\nspecial_command_stick_range = [[-20.0, 20.0]]\n");
        assert!(parse(&short).unwrap_err().message.contains("exactly 7"));
        let zero = format!("{head}\n[commands]\nspecial_command_life_max = 0\n");
        assert!(parse(&zero).unwrap_err().message.contains("at least 1"));
        let unknown = format!("{head}\n[[command]]\nname = \"a\"\ninput = \"236\"\nstrength = 3\n");
        assert_eq!(parse(&unknown).unwrap_err().message, "unknown command key strength");
    }

    #[test]
    fn a_tether_names_a_donor_and_its_own_ledge_boxes() {
        let head = "[fighter]\nname = \"wawa\"\nbase = \"donkey\"\n";
        let text = format!("{head}\n[tether]\nfrom = \"lucas\"\n\n[[tether.box]]\np1 = [50.0, 40.0]\np2 = [0, -30]\n");
        let tether = parse(&text).unwrap().tether.unwrap();
        assert_eq!(tether.from.as_deref(), Some("lucas"));
        assert_eq!(tether.boxes[0].words(), [50.0, 40.0, 0.0, -30.0]);
        assert!(parse(&format!("{head}\n[tether]\n")).unwrap().tether.unwrap().from.is_none());
        assert!(parse(&format!("{head}\n[tether]\nfrom = \"mario\"\n")).is_err());
        assert!(parse(&format!("{head}\n[[tether.box]]\np1 = [1.0]\n")).is_err());
        assert_eq!(parse(head).unwrap().tether, None);
    }

    #[test]
    fn a_final_smash_names_a_donor_with_nothing_left_to_build() {
        let head = "name = \"wawa\"\nbase = \"donkey\"\n";
        let marth = parse(&format!("{head}[final]\nfrom = \"marth\"\n")).unwrap();
        assert_eq!(marth.final_smash.as_deref(), Some("marth"));
        assert_eq!(parse(head).unwrap().final_smash, None);
        let joker = parse(&format!("{head}[final]\nfrom = \"jack\"\n")).unwrap_err();
        assert_eq!(joker.line, 4);
        assert!(joker.message.contains("cannot borrow yet"), "{}", joker.message);
        let stray = parse(&format!("{head}[final]\ndonor = \"marth\"\n")).unwrap_err();
        assert!(stray.message.contains("unknown final key donor"), "{}", stray.message);
    }

    #[test]
    fn an_article_needs_a_source_or_a_base_article_but_not_both() {
        let neither = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"x\"\n";
        assert!(parse(neither).unwrap_err().message.contains("needs from"));
        let both = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"x\"\nfrom = \"mario/fireball\"\nbase_article = \"cshot\"\n";
        assert!(parse(both).unwrap_err().message.contains("both"));
        let bad_from = "name = \"wawa\"\nbase = \"samus\"\n[[article]]\nname = \"x\"\nfrom = \"fireball\"\n";
        assert_eq!(parse(bad_from).unwrap_err().line, 5);
    }

    #[test]
    fn a_global_motion_takes_the_kirby_motion_keys_and_needs_a_template_or_an_animation() {
        let fighter = parse(
            "name = \"bluster\"\nbase = \"donkey\"\n\
             [[global_motion]]\nname = \"catched_bluster\"\ntemplate = \"catched_ganon\"\n\
             [[global_motion]]\nname = \"bluster_victim\"\nanimation = \"d05blustervictim.nuanmb\"\nscripts = \"blustervictim\"\nloop = true\n",
        )
        .unwrap();
        assert!(fighter.kirby.is_none());
        assert_eq!(fighter.global_motions.len(), 2);
        assert_eq!(fighter.global_motions[0].template.as_deref(), Some("catched_ganon"));
        assert!(fighter.global_motions[0].animation.is_empty());
        assert_eq!(
            fighter.global_motions[1].script("sound", None).as_deref(),
            Some("sound_blustervictim")
        );
        assert_eq!(fighter.global_motions[1].flags, vec!["loop".to_string()]);

        let neither = parse("name = \"a\"\nbase = \"b\"\n[[global_motion]]\nname = \"x\"\n").unwrap_err();
        assert!(neither.message.contains("needs a template, an animation or victim = true"));
        let bad_key = parse("name = \"a\"\nbase = \"b\"\n[[global_motion]]\nname = \"x\"\nframes = 3\n").unwrap_err();
        assert_eq!((bad_key.line, bad_key.message.as_str()), (5, "unknown motion key frames"));
    }

    #[test]
    fn a_victim_global_motion_needs_nothing_else_and_is_refused_on_kirby() {
        let fighter = parse(
            "name = \"bluster\"\nbase = \"donkey\"\n[[global_motion]]\nname = \"thrown_bluster_f\"\nvictim = true\n",
        )
        .unwrap();
        assert!(fighter.global_motions[0].victim);
        assert!(fighter.global_motions[0].animation.is_empty());
        assert!(fighter.global_motions[0].template.is_none());

        let kirby = parse(
            "name = \"a\"\nbase = \"b\"\n[kirby]\nstatuses = 1\n[[kirby.motion]]\nname = \"x\"\nanimation = \"x.nuanmb\"\nvictim = true\n",
        )
        .unwrap_err();
        assert!(kirby.message.contains("victim is for global motions"));
    }

    #[test]
    fn a_fighter_block_takes_its_own_global_motions() {
        let fighters = parse_all(
            "[[fighter]]\nname = \"a\"\nbase = \"mario\"\n[[fighter.global_motion]]\nname = \"catched_a\"\ntemplate = \"catched_ganon\"\n\
             [[fighter]]\nname = \"b\"\nbase = \"luigi\"\n",
        )
        .unwrap();
        assert_eq!(fighters[0].global_motions.len(), 1);
        assert!(fighters[1].global_motions.is_empty());
    }

    #[test]
    fn costume_range_is_bounded_like_the_engine() {
        assert!(parse("name = \"a\"\nbase = \"b\"\ncostumes = 0\n").is_err());
        assert!(parse("name = \"a\"\nbase = \"b\"\ncostumes = 256\n").is_ok());
        assert!(parse("name = \"a\"\nbase = \"b\"\ncolor_start = 8\ncostumes = 249\n").is_err());
        assert!(parse("name = \"a\"\nbase = \"b\"\ncolor_start = 8\ncostumes = 248\n").is_ok());
    }

    #[test]
    fn a_multiplier_must_be_a_decimal_and_hex_is_a_whole_number() {
        let failure = parse("name = \"a\"\nbase = \"b\"\n[params.mul]\njump_y = 2\n").unwrap_err();
        assert_eq!(failure.line, 4);
        let fighter = parse("name = \"a\"\nbase = \"b\"\n[kirby]\nfirst = 0x400\nstatuses = 6\n").unwrap();
        assert_eq!(fighter.kirby.unwrap().first, Some(0x400));
    }

    #[test]
    fn names_are_folder_names() {
        assert!(parse("name = \"Wawa\"\nbase = \"samus\"\n").is_err());
        assert!(parse("name = \"wawa-2\"\nbase = \"samus\"\n").is_err());
        assert!(parse("name = \"wawa_2\"\nbase = \"samus\"\n").is_ok());
    }

    #[test]
    fn several_fighters_are_blocks_with_prefixed_tables_and_absolute_lines() {
        let text = "[[fighter]]\nname = \"a\"\nbase = \"mario\"\n[[fighter.article]]\nname = \"ball\"\nfrom = \"mario/fireball\"\n[fighter.kirby]\nstatuses = 2\n\n[[fighter]]\nname = \"b\"\nbase = \"luigi\"\nbogus = 1\n";
        let failure = parse_all(text).unwrap_err();
        assert_eq!(failure.line, 13);
        let fixed = text.replace("bogus = 1\n", "");
        let fighters = parse_all(&fixed).unwrap();
        assert_eq!(fighters.len(), 2);
        assert_eq!(fighters[0].articles[0].name, "ball");
        assert_eq!(fighters[0].kirby.as_ref().unwrap().statuses, 2);
        assert_eq!(fighters[1].base, "luigi");
    }

    #[test]
    fn a_key_before_the_first_block_is_refused_at_its_line() {
        let failure = parse_all("name = \"lost\"\n[[fighter]]\nname = \"a\"\nbase = \"mario\"\n").unwrap_err();
        assert_eq!(failure.line, 1);
    }

    #[test]
    fn a_grab_takes_status_names_or_numbers() {
        let fighter = parse(
            "name = \"a\"\nbase = \"mario\"\n\
             [[grab]]\nstatus = \"SPECIAL_S\"\nvictim = \"CATCHED_GANON\"\nthen = 0x1e2\n\
             [[grab]]\nstatus = 0x1de\nvictim = 0xbd\nnode = \"\"\ncatch = true\noffset = [0.0, 2.5, -1]\n",
        )
        .unwrap();
        assert_eq!(fighter.grabs.len(), 2);
        let choke = &fighter.grabs[0];
        assert_eq!((choke.status, choke.victim, choke.then), (0x1dd, Some(0xbe), Some(0x1e2)));
        assert_eq!(choke.node.as_deref(), Some("throw"));
        assert!(!choke.catch);
        let dive = &fighter.grabs[1];
        assert_eq!(dive.node.as_deref(), Some(""));
        assert!(dive.catch);
        assert_eq!(dive.offset, [0.0, 2.5, -1.0]);
        assert_eq!(dive.then, None);
    }

    #[test]
    fn own_statuses_count_from_the_end_of_the_bases_table() {
        let fighter = parse(
            "name = \"a\"\nbase = \"mario\"\n[[grab]]\nstatus = \"own 0\"\nvictim = \"CATCHED_GANON\"\nthen = \"own 2\"\n",
        )
        .unwrap();
        assert_eq!((fighter.grabs[0].status, fighter.grabs[0].then), (0x1e3, Some(0x1e5)));
        let lucina = parse("name = \"a\"\nbase = \"lucina\"\n[[grab]]\nstatus = \"SPECIAL_S\"\nthen = \"own 1\"\n").unwrap();
        assert_eq!(lucina.grabs[0].then, crate::status_starts::status_start("marth").map(|start| Some(start + 1)).unwrap());
        let victim = parse("name = \"a\"\nbase = \"mario\"\n[[grab]]\nstatus = 1\nvictim = \"own 0\"\n").unwrap_err();
        assert!(victim.message.contains("victim"));
        let unknown = parse("name = \"a\"\nbase = \"b\"\n[[grab]]\nstatus = \"own 0\"\n").unwrap_err();
        assert!(unknown.message.contains("vanilla base"));
    }

    #[test]
    fn a_grab_needs_a_status_and_refuses_what_it_cannot_read() {
        let missing = parse("name = \"a\"\nbase = \"b\"\n[[grab]]\nvictim = 0xbe\n").unwrap_err();
        assert!(missing.message.contains("status"));
        let unknown = parse("name = \"a\"\nbase = \"b\"\n[[grab]]\nstatus = \"SPECIAL_Q\"\n").unwrap_err();
        assert_eq!(unknown.line, 4);
        assert!(unknown.message.contains("own N"));
        let bad_key = parse("name = \"a\"\nbase = \"b\"\n[[grab]]\nstatus = 1\nhold = true\n").unwrap_err();
        assert_eq!(bad_key.line, 5);
        let short = parse("name = \"a\"\nbase = \"b\"\n[[grab]]\nstatus = 1\noffset = [1.0, 2.0]\n").unwrap_err();
        assert!(short.message.contains("[x, y, z]"));
        let negative = parse("name = \"a\"\nbase = \"b\"\n[[grab]]\nstatus = -3\n").unwrap_err();
        assert!(negative.message.contains("negative"));
    }

    #[test]
    fn a_fighter_block_takes_its_own_grabs() {
        let fighters = parse_all(
            "[[fighter]]\nname = \"a\"\nbase = \"mario\"\n[[fighter.grab]]\nstatus = \"ATTACK\"\nvictim = \"CAPTURE_PULLED\"\nthen = \"CATCH_PULL\"\n\
             [[fighter]]\nname = \"b\"\nbase = \"luigi\"\n",
        )
        .unwrap();
        assert_eq!(fighters[0].grabs.len(), 1);
        assert_eq!(fighters[0].grabs[0].then, Some(0x38));
        assert!(fighters[1].grabs.is_empty());
    }
}
