use crate::article_hurtbox::Hurtbox;
use crate::article_lvd::ArticleLvd;
use crate::grab_rule::GrabRule;

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn float(value: f64) -> String {
    let text = format!("{value:?}");
    if text.contains('.') || text.contains('e') {
        text
    } else {
        format!("{text}.0")
    }
}

pub struct Motion {
    lines: Vec<String>,
}

impl Motion {
    pub fn new(name: &str, animation: &str) -> Self {
        Self {
            lines: vec![
                format!("name = {}", quoted(name)),
                format!("animation = {}", quoted(animation)),
            ],
        }
    }

    pub fn from_template(name: &str, template: &str) -> Self {
        Self {
            lines: vec![
                format!("name = {}", quoted(name)),
                format!("template = {}", quoted(template)),
            ],
        }
    }

    pub fn victim(name: &str) -> Self {
        Self {
            lines: vec![format!("name = {}", quoted(name)), "victim = true".to_string()],
        }
    }

    fn text(mut self, key: &str, value: &str) -> Self {
        self.lines.push(format!("{key} = {}", quoted(value)));
        self
    }

    pub fn animation(self, animation: &str) -> Self {
        self.text("animation", animation)
    }

    fn number(mut self, key: &str, value: impl core::fmt::Display) -> Self {
        self.lines.push(format!("{key} = {value}"));
        self
    }

    pub fn template(self, template: &str) -> Self {
        self.text("template", template)
    }

    pub fn scripts(self, stem: &str) -> Self {
        self.text("scripts", stem)
    }

    pub fn game(self, script: &str) -> Self {
        self.text("game", script)
    }

    pub fn sound(self, script: &str) -> Self {
        self.text("sound", script)
    }

    pub fn effect(self, script: &str) -> Self {
        self.text("effect", script)
    }

    pub fn expression(self, script: &str) -> Self {
        self.text("expression", script)
    }

    pub fn flags(mut self, flags: &[&str]) -> Self {
        let list: Vec<String> = flags.iter().map(|flag| quoted(flag)).collect();
        self.lines.push(format!("flags = [{}]", list.join(", ")));
        self
    }

    pub fn blend_frames(self, frames: u8) -> Self {
        self.number("blend_frames", frames)
    }

    pub fn cancel_frame(self, frame: u8) -> Self {
        self.number("cancel_frame", frame)
    }

    pub fn xlu(mut self, start: u8, end: u8) -> Self {
        self.lines.push(format!("xlu = [{start}, {end}]"));
        self
    }

    pub fn no_stop_intp(self, value: bool) -> Self {
        self.number("no_stop_intp", value)
    }

    pub fn animation_unk(self, value: u8) -> Self {
        self.number("animation_unk", value)
    }

    pub fn no_extra(self) -> Self {
        self.number("no_extra", true)
    }
}

pub struct Grab {
    rule: GrabRule,
}

impl Grab {
    pub fn new(status: i32) -> Self {
        Self {
            rule: GrabRule::new(status),
        }
    }

    pub fn victim(mut self, status: i32) -> Self {
        self.rule.victim = Some(status);
        self
    }

    pub fn then(mut self, status: i32) -> Self {
        self.rule.then = Some(status);
        self
    }

    pub fn node(mut self, joint: &str) -> Self {
        self.rule.node = Some(joint.to_string());
        self
    }

    pub fn unpinned(mut self) -> Self {
        self.rule.node = Some(String::new());
        self
    }

    pub fn offset(mut self, x: f32, y: f32, z: f32) -> Self {
        self.rule.offset = [x, y, z];
        self
    }

    pub fn catch(mut self) -> Self {
        self.rule.catch = true;
        self
    }

    pub fn rule(&self) -> &GrabRule {
        &self.rule
    }

    fn lines(&self) -> Vec<String> {
        let rule = &self.rule;
        let mut lines = vec![format!("status = {:#x}", rule.status)];
        if let Some(victim) = rule.victim {
            lines.push(format!("victim = {victim:#x}"));
        }
        if let Some(then) = rule.then {
            lines.push(format!("then = {then:#x}"));
        }
        if let Some(node) = &rule.node {
            lines.push(format!("node = {}", quoted(node)));
        }
        if rule.offset != [0.0; 3] {
            let [x, y, z] = rule.offset.map(|value| float(f64::from(value)));
            lines.push(format!("offset = [{x}, {y}, {z}]"));
        }
        if rule.catch {
            lines.push("catch = true".to_string());
        }
        lines
    }
}

fn push_lvd_vertices(out: &mut String, lvd: &ArticleLvd) {
    for vertex in &lvd.vertices {
        out.push_str("\n[[article.lvd_vertex]]\n");
        for line in vertex.lines() {
            out.push_str(&line);
            out.push('\n');
        }
    }
}

const GAME_SWITCH_BASES: [&str; 5] = ["eflame", "elight", "pzenigame", "pfushigisou", "plizardon"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    GlobalMotions,
    Grabs,
    Hud,
    Tether,
    MoveList,
    Commands,
    EntryGroups,
    AnyBaseGroups,
}

impl Feature {
    pub const ALL: [Feature; 8] = [
        Feature::GlobalMotions,
        Feature::Grabs,
        Feature::Hud,
        Feature::Tether,
        Feature::MoveList,
        Feature::Commands,
        Feature::EntryGroups,
        Feature::AnyBaseGroups,
    ];

    pub fn describe(self) -> &'static str {
        match self {
            Feature::GlobalMotions => "global motions",
            Feature::Grabs => "command grabs",
            Feature::Hud => "the HUD",
            Feature::Tether => "the tether",
            Feature::MoveList => "move list pages",
            Feature::Commands => "command inputs",
            Feature::EntryGroups => "the partner or switch group",
            Feature::AnyBaseGroups => "partners and switch groups on this base",
        }
    }

    pub fn of_table(header: &str) -> Option<Feature> {
        let name = header.trim().trim_start_matches('[').trim_end_matches(']').trim();
        match name.split('.').next().unwrap_or("") {
            "global_motion" => Some(Feature::GlobalMotions),
            "grab" => Some(Feature::Grabs),
            "hud" => Some(Feature::Hud),
            "tether" => Some(Feature::Tether),
            "move_list" => Some(Feature::MoveList),
            "commands" | "command" => Some(Feature::Commands),
            "partner_params" => Some(Feature::AnyBaseGroups),
            _ => None,
        }
    }
}

fn raw_features(raw: &str) -> Vec<Feature> {
    let mut found = Vec::new();
    for line in raw.lines() {
        if !line.trim_start().starts_with('[') {
            continue;
        }
        if let Some(feature) = Feature::of_table(line) {
            if !found.contains(&feature) {
                found.push(feature);
            }
        }
    }
    found
}

pub struct Manifest {
    name: String,
    base: String,
    fighter: Vec<String>,
    entry: Vec<String>,
    entry_feature: Option<Feature>,
    articles: Vec<Vec<String>>,
    article_hurtboxes: Vec<(String, Hurtbox)>,
    article_lvds: Vec<(String, ArticleLvd)>,
    kirby: Vec<String>,
    motions: Vec<Motion>,
    meshes: Vec<(String, bool)>,
    global_motions: Vec<Motion>,
    grabs: Vec<Grab>,
    hud: Option<String>,
    hud_layout: Option<String>,
    tether: Option<crate::tether::Tether>,
    move_list: Option<crate::move_list::MoveList>,
    params: Vec<(Option<u32>, bool, String, String)>,
    partner_params: Vec<(String, String)>,
    command_settings: crate::command_input::Settings,
    commands: Vec<crate::command_input::Command>,
    raw: Vec<String>,
}

impl Manifest {
    pub fn new(name: &str, base: &str) -> Self {
        Self {
            name: name.to_string(),
            base: base.to_ascii_lowercase(),
            fighter: vec![
                format!("name = {}", quoted(name)),
                format!("base = {}", quoted(base)),
            ],
            entry: Vec::new(),
            entry_feature: None,
            articles: Vec::new(),
            article_hurtboxes: Vec::new(),
            article_lvds: Vec::new(),
            kirby: Vec::new(),
            motions: Vec::new(),
            meshes: Vec::new(),
            global_motions: Vec::new(),
            grabs: Vec::new(),
            hud: None,
            hud_layout: None,
            tether: None,
            move_list: None,
            params: Vec::new(),
            partner_params: Vec::new(),
            command_settings: crate::command_input::Settings::default(),
            commands: Vec::new(),
            raw: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn text(mut self, key: &str, value: &str) -> Self {
        self.fighter.push(format!("{key} = {}", quoted(value)));
        self
    }

    fn number(mut self, key: &str, value: impl core::fmt::Display) -> Self {
        self.fighter.push(format!("{key} = {value}"));
        self
    }

    pub fn costumes(self, count: u32) -> Self {
        self.number("costumes", count)
    }

    pub fn color_start(self, first: u32) -> Self {
        self.number("color_start", first)
    }

    pub fn series(self, series: &str) -> Self {
        self.text("series", series)
    }

    pub fn disp_order(self, order: i8) -> Self {
        self.number("disp_order", order)
    }

    pub fn save_no(self, save_no: i8) -> Self {
        self.number("save_no", save_no)
    }

    pub fn exhibit_year(self, year: i16) -> Self {
        self.number("exhibit_year", year)
    }

    pub fn narration(self, label: &str) -> Self {
        self.text("narration", label)
    }

    pub fn name_label(self, label: &str) -> Self {
        self.text("display_name", label)
    }

    pub fn staffroll(self) -> Self {
        self.number("staffroll", true)
    }

    pub fn item_hand(self, hand: &str) -> Self {
        self.text("item_hand", hand)
    }

    pub fn cpu_ai(self, fighter: &str) -> Self {
        self.text("cpu_ai", fighter)
    }

    fn entry_line(mut self, line: String, any_base: bool) -> Self {
        self.entry.push(line);
        if any_base {
            self.entry_feature = Some(Feature::AnyBaseGroups);
        } else if self.entry_feature.is_none() {
            self.entry_feature = Some(Feature::EntryGroups);
        }
        self
    }

    pub fn partner(self, enabled: bool) -> Self {
        let any_base = enabled && self.base != "popo";
        self.entry_line(format!("partner = {enabled}"), any_base)
    }

    pub fn partner_with(self, fighter: &str) -> Self {
        self.entry_line(format!("partner = {}", quoted(fighter)), true)
    }

    pub fn switch_with(self, members: &[&str]) -> Self {
        let list = members.iter().map(|member| format!("\"{member}\"")).collect::<Vec<_>>().join(", ");
        let any_base = !GAME_SWITCH_BASES.contains(&self.base.as_str());
        self.entry_line(format!("switch = [{list}]"), any_base)
    }

    pub fn features(&self) -> Vec<Feature> {
        let mut used = Vec::new();
        let mut add = |feature: Feature| {
            if !used.contains(&feature) {
                used.push(feature);
            }
        };
        if !self.global_motions.is_empty() {
            add(Feature::GlobalMotions);
        }
        if !self.grabs.is_empty() {
            add(Feature::Grabs);
        }
        if self.hud.is_some() || self.hud_layout.is_some() {
            add(Feature::Hud);
        }
        if self.tether.is_some() {
            add(Feature::Tether);
        }
        if self.move_list.is_some() {
            add(Feature::MoveList);
        }
        if !self.command_settings.is_empty() || !self.commands.is_empty() {
            add(Feature::Commands);
        }
        if let Some(feature) = self.entry_feature {
            add(feature);
        }
        if !self.partner_params.is_empty() {
            add(Feature::AnyBaseGroups);
        }
        for raw in &self.raw {
            for feature in raw_features(raw) {
                add(feature);
            }
        }
        used
    }

    pub fn own_css(self) -> Self {
        self.number("css", false)
    }

    pub fn jingle(self, name: &str) -> Self {
        self.text("jingle", name)
    }

    pub fn ui_chara(self, ui_chara: &str) -> Self {
        self.text("ui_chara", ui_chara)
    }

    pub fn fighter_kind_name(self, name: &str) -> Self {
        self.text("fighter_kind_name", name)
    }

    pub fn resource_name(self, name: &str) -> Self {
        self.text("resource_name", name)
    }

    pub fn base_resource_name(self, name: &str) -> Self {
        self.text("base_resource_name", name)
    }

    pub fn owns_param_resources(self) -> Self {
        self.number("owns_param_resources", true)
    }

    pub fn effect_namespace(self, namespace: u32) -> Self {
        self.number("effect_namespace", namespace)
    }

    pub fn article_namespace(self, namespace: u32) -> Self {
        self.number("article_namespace", namespace)
    }

    pub fn article(mut self, name: &str, from: &str) -> Self {
        self.articles.push(vec![
            format!("name = {}", quoted(name)),
            format!("from = {}", quoted(from)),
        ]);
        self
    }

    pub fn base_article(mut self, name: &str, base_article: &str) -> Self {
        self.articles.push(vec![
            format!("name = {}", quoted(name)),
            format!("base_article = {}", quoted(base_article)),
        ]);
        self
    }

    pub fn kirby_article(mut self, name: &str, from: &str) -> Self {
        self.articles.push(vec![
            format!("name = {}", quoted(name)),
            format!("from = {}", quoted(from)),
            "kirby = true".to_string(),
        ]);
        self
    }

    pub fn article_hurtbox(mut self, article: &str, hurtbox: Hurtbox) -> Self {
        self.article_hurtboxes.push((article.to_string(), hurtbox));
        self
    }

    pub fn article_lvd(mut self, article: &str, lvd: ArticleLvd) -> Self {
        self.article_lvds.push((article.to_string(), lvd));
        self
    }

    pub fn article_original_code(self, article: &str, on: bool) -> Self {
        self.article_line(article, format!("original_code = {on}"))
    }

    pub fn article_original_kind(self, article: &str, on: bool) -> Self {
        self.article_line(article, format!("original_kind = {on}"))
    }

    fn article_line(mut self, article: &str, line: String) -> Self {
        let name = format!("name = {}", quoted(article));
        match self.articles.iter_mut().find(|lines| lines.first() == Some(&name)) {
            Some(lines) => lines.push(line),
            None => self.articles.push(vec![name, line]),
        }
        self
    }

    pub fn kirby(mut self, statuses: i32) -> Self {
        self.kirby.push(format!("statuses = {statuses}"));
        self
    }

    pub fn kirby_first(mut self, first: i32) -> Self {
        self.kirby.push(format!("first = {first}"));
        self
    }

    pub fn kirby_full_model(mut self) -> Self {
        self.kirby.push("full_model = true".to_string());
        self
    }

    pub fn kirby_model(mut self, model: &str) -> Self {
        self.kirby.push(format!("model = {}", quoted(model)));
        self
    }

    pub fn kirby_mesh(mut self, name: &str, visible: bool) -> Self {
        self.meshes.push((name.to_string(), visible));
        self
    }

    pub fn kirby_motion(mut self, motion: Motion) -> Self {
        self.motions.push(motion);
        self
    }

    pub fn global_motion(mut self, motion: Motion) -> Self {
        self.global_motions.push(motion);
        self
    }

    pub fn grab(mut self, grab: Grab) -> Self {
        self.grabs.push(grab);
        self
    }

    pub fn hud(mut self, donor: &str) -> Self {
        self.hud = Some(donor.to_string());
        self
    }

    pub fn hud_layout(mut self, path: &str) -> Self {
        self.hud_layout = Some(path.to_string());
        self
    }

    pub fn tether(mut self, donor: &str) -> Self {
        self.tether.get_or_insert_with(Default::default).from = Some(donor.to_string());
        self
    }

    pub fn move_list(mut self, pages: u8) -> Self {
        self.move_list.get_or_insert_with(Default::default).pages = Some(pages);
        self
    }

    pub fn move_list_images(mut self, pages: &[u8]) -> Self {
        self.move_list.get_or_insert_with(Default::default).image_pages = Some(pages.to_vec());
        self
    }

    pub fn tether_box(mut self, p1: [f32; 2], p2: [f32; 2]) -> Self {
        self.tether
            .get_or_insert_with(Default::default)
            .boxes
            .push(crate::tether::TetherBox::new(p1, p2));
        self
    }

    fn push_param(mut self, costume: Option<u32>, mul: bool, name: &str, value: String) -> Self {
        self.params.push((costume, mul, name.to_string(), value));
        self
    }

    pub fn param(self, name: &str, value: f64) -> Self {
        self.push_param(None, false, name, float(value))
    }

    pub fn param_int(self, name: &str, value: i32) -> Self {
        self.push_param(None, false, name, value.to_string())
    }

    pub fn param_mul(self, name: &str, value: f64) -> Self {
        self.push_param(None, true, name, float(value))
    }

    pub fn param_at(self, costume: u32, name: &str, value: f64) -> Self {
        self.push_param(Some(costume), false, name, float(value))
    }

    pub fn param_int_at(self, costume: u32, name: &str, value: i32) -> Self {
        self.push_param(Some(costume), false, name, value.to_string())
    }

    pub fn param_mul_at(self, costume: u32, name: &str, value: f64) -> Self {
        self.push_param(Some(costume), true, name, float(value))
    }

    pub fn params(mut self, values: &[(&str, f64)]) -> Self {
        for (name, value) in values {
            self = self.param(name, *value);
        }
        self
    }

    pub fn params_int(mut self, values: &[(&str, i32)]) -> Self {
        for (name, value) in values {
            self = self.param_int(name, *value);
        }
        self
    }

    pub fn params_mul(mut self, values: &[(&str, f64)]) -> Self {
        for (name, value) in values {
            self = self.param_mul(name, *value);
        }
        self
    }

    pub fn partner_param(mut self, name: &str, value: f64) -> Self {
        self.partner_params.push((name.to_string(), float(value)));
        self
    }

    pub fn partner_param_int(mut self, name: &str, value: i32) -> Self {
        self.partner_params.push((name.to_string(), value.to_string()));
        self
    }

    pub fn vanilla_commands(mut self, on: bool) -> Self {
        self.command_settings.vanilla = on;
        self
    }

    pub fn special_command_life_max(mut self, frames: u8) -> Self {
        self.command_settings.life = Some(frames);
        self
    }

    pub fn super_special_command_life_max(mut self, frames: u8) -> Self {
        self.command_settings.super_life = Some(frames);
        self
    }

    pub fn special_command_ab_button_life(mut self, frames: u8) -> Self {
        self.command_settings.button_life = Some(frames);
        self
    }

    pub fn special_command_neutral_threshold(mut self, threshold: f32) -> Self {
        self.command_settings.neutral_threshold = Some(threshold);
        self
    }

    pub fn special_command_stick_range(mut self, range: crate::command_input::StickRange) -> Self {
        self.command_settings.stick_range = Some(range);
        self
    }

    pub fn command(mut self, command: crate::command_input::Command) -> Self {
        self.commands.push(command);
        self
    }

    pub fn raw(mut self, toml: &str) -> Self {
        self.raw.push(toml.to_string());
        self
    }

    pub fn to_toml(&self) -> String {
        self.to_toml_with(|_| true)
    }

    pub fn to_toml_with(&self, keep: impl Fn(Feature) -> bool) -> String {
        let mut out = String::from("[fighter]\n");
        for line in &self.fighter {
            out.push_str(line);
            out.push('\n');
        }
        if self.entry_feature.is_some_and(&keep) {
            for line in &self.entry {
                out.push_str(line);
                out.push('\n');
            }
        }
        let mut placed = vec![false; self.article_hurtboxes.len()];
        let mut lvd_placed = vec![false; self.article_lvds.len()];
        for article in &self.articles {
            out.push_str("\n[[article]]\n");
            for line in article {
                out.push_str(line);
                out.push('\n');
            }
            let named = |name: &str| article.first() == Some(&format!("name = {}", quoted(name)));
            let lvd = self
                .article_lvds
                .iter()
                .enumerate()
                .find(|(index, (name, _))| !lvd_placed[*index] && named(name))
                .map(|(index, (_, lvd))| (index, lvd));
            if let Some((index, lvd)) = lvd {
                lvd_placed[index] = true;
                out.push_str(&format!("lvd = {}\n", quoted(&lvd.file)));
            }
            for (index, (name, hurtbox)) in self.article_hurtboxes.iter().enumerate() {
                if placed[index] || !named(name) {
                    continue;
                }
                placed[index] = true;
                out.push_str("\n[[article.hurtbox]]\n");
                for line in hurtbox.lines() {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            if let Some((_, lvd)) = lvd {
                push_lvd_vertices(&mut out, lvd);
            }
        }
        for (index, (name, hurtbox)) in self.article_hurtboxes.iter().enumerate() {
            if placed[index] {
                continue;
            }
            out.push_str(&format!("\n[[article]]\nname = {}\n\n[[article.hurtbox]]\n", quoted(name)));
            for line in hurtbox.lines() {
                out.push_str(&line);
                out.push('\n');
            }
        }
        for (index, (name, lvd)) in self.article_lvds.iter().enumerate() {
            if lvd_placed[index] {
                continue;
            }
            out.push_str(&format!(
                "\n[[article]]\nname = {}\nlvd = {}\n",
                quoted(name),
                quoted(&lvd.file)
            ));
            push_lvd_vertices(&mut out, lvd);
        }
        if !self.kirby.is_empty() || !self.motions.is_empty() || !self.meshes.is_empty() {
            out.push_str("\n[kirby]\n");
            for line in &self.kirby {
                out.push_str(line);
                out.push('\n');
            }
            for motion in &self.motions {
                out.push_str("\n[[kirby.motion]]\n");
                for line in &motion.lines {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            for (name, visible) in &self.meshes {
                out.push_str(&format!("\n[[kirby.mesh]]\nname = {}\nvisible = {visible}\n", quoted(name)));
            }
        }
        for motion in self.global_motions.iter().filter(|_| keep(Feature::GlobalMotions)) {
            out.push_str("\n[[global_motion]]\n");
            for line in &motion.lines {
                out.push_str(line);
                out.push('\n');
            }
        }
        for grab in self.grabs.iter().filter(|_| keep(Feature::Grabs)) {
            out.push_str("\n[[grab]]\n");
            for line in grab.lines() {
                out.push_str(&line);
                out.push('\n');
            }
        }
        let hud = keep(Feature::Hud);
        if hud && (self.hud.is_some() || self.hud_layout.is_some()) {
            out.push_str("\n[hud]\n");
        }
        if let Some(donor) = self.hud.as_ref().filter(|_| hud) {
            out.push_str(&format!("from = {}\n", quoted(donor)));
        }
        if let Some(path) = self.hud_layout.as_ref().filter(|_| hud) {
            out.push_str(&format!("layout = {}\n", quoted(path)));
        }
        if let Some(tether) = self.tether.as_ref().filter(|_| keep(Feature::Tether)) {
            out.push_str("\n[tether]\n");
            if let Some(donor) = &tether.from {
                out.push_str(&format!("from = {}\n", quoted(donor)));
            }
            for shape in &tether.boxes {
                out.push_str(&format!(
                    "\n[[tether.box]]\np1 = [{}, {}]\np2 = [{}, {}]\n",
                    float(shape.p1[0] as f64),
                    float(shape.p1[1] as f64),
                    float(shape.p2[0] as f64),
                    float(shape.p2[1] as f64)
                ));
            }
        }
        if let Some(list) = self.move_list.as_ref().filter(|_| keep(Feature::MoveList)) {
            out.push_str("\n[move_list]\n");
            if let Some(pages) = list.pages {
                out.push_str(&format!("pages = {pages}\n"));
            }
            if let Some(images) = &list.image_pages {
                let pages: Vec<String> = images.iter().map(|page| page.to_string()).collect();
                out.push_str(&format!("image_pages = [{}]\n", pages.join(", ")));
            }
        }
        let mut tables: Vec<(Option<u32>, bool)> = Vec::new();
        for (costume, mul, _, _) in &self.params {
            if !tables.contains(&(*costume, *mul)) {
                tables.push((*costume, *mul));
            }
        }
        tables.sort();
        for (costume, mul) in tables {
            let mut header = String::from("params");
            if let Some(costume) = costume {
                header.push_str(&format!(".c{costume:02}"));
            }
            if mul {
                header.push_str(".mul");
            }
            out.push_str(&format!("\n[{header}]\n"));
            for (param_costume, param_mul, name, value) in &self.params {
                if *param_costume == costume && *param_mul == mul {
                    out.push_str(&format!("{name} = {value}\n"));
                }
            }
        }
        if !self.partner_params.is_empty() && keep(Feature::AnyBaseGroups) {
            out.push_str("\n[partner_params]\n");
            for (name, value) in &self.partner_params {
                out.push_str(&format!("{name} = {value}\n"));
            }
        }
        let commands = keep(Feature::Commands);
        if commands && !self.command_settings.is_empty() {
            out.push_str("\n[commands]\n");
            for line in self.command_settings.toml_lines() {
                out.push_str(&line);
                out.push('\n');
            }
        }
        for command in self.commands.iter().filter(|_| commands) {
            out.push_str("\n[[command]]\n");
            for line in command.toml_lines() {
                out.push_str(&line);
                out.push('\n');
            }
        }
        for raw in self.raw.iter().filter(|raw| raw_features(raw).into_iter().all(&keep)) {
            out.push('\n');
            out.push_str(raw);
            if !raw.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }
}

pub trait ItemBase {
    fn line(&self) -> String;
}

impl ItemBase for i32 {
    fn line(&self) -> String {
        format!("base_kind = {self}")
    }
}

impl ItemBase for &str {
    fn line(&self) -> String {
        format!("base_item = {}", quoted(self))
    }
}

pub struct ItemManifest {
    resource_name: String,
    item: Vec<String>,
    common: Vec<String>,
    owner_params: Vec<String>,
    raw: Vec<String>,
}

impl ItemManifest {
    pub fn new(resource_name: &str, base: impl ItemBase) -> Self {
        Self {
            resource_name: resource_name.to_string(),
            item: vec![
                base.line(),
                format!("resource_name = {}", quoted(resource_name)),
            ],
            common: Vec::new(),
            owner_params: Vec::new(),
            raw: Vec::new(),
        }
    }

    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }

    fn text(mut self, key: &str, value: &str) -> Self {
        self.item.push(format!("{key} = {}", quoted(value)));
        self
    }

    fn number(mut self, key: &str, value: impl core::fmt::Display) -> Self {
        self.item.push(format!("{key} = {value}"));
        self
    }

    pub fn base_item(self, name: &str) -> Self {
        self.text("base_item", name)
    }

    pub fn members(mut self, names: &[&str]) -> Self {
        let listed: Vec<String> = names.iter().map(|name| quoted(name)).collect();
        self.item.push(format!("members = [{}]", listed.join(", ")));
        self
    }

    pub fn agent_name(self, name: &str) -> Self {
        self.text("agent_name", name)
    }

    pub fn ui_id(self, ui_id: &str) -> Self {
        self.text("ui_id", ui_id)
    }

    pub fn training_order(self, order: i32) -> Self {
        self.number("training_order", order)
    }

    pub fn spawn_per(self, per: i32) -> Self {
        self.number("spawn_per", per)
    }

    pub fn spawn_range(self, min: i32, max: i32) -> Self {
        self.number("spawn_min", min).number("spawn_max", max)
    }

    pub fn spawn_from(mut self, names: &[&str]) -> Self {
        self.item.push(format!("spawn_from = {}", quoted(&names.join(","))));
        self
    }

    pub fn common(mut self, field: &str, value: f64) -> Self {
        self.common.push(format!("{field} = {}", float(value)));
        self
    }

    pub fn common_int(mut self, field: &str, value: i32) -> Self {
        self.common.push(format!("{field} = {value}"));
        self
    }

    pub fn owner_param(mut self, owner: &str, field: &str, value: f64) -> Self {
        self.owner_params.push(format!("{owner}.{field} = {}", float(value)));
        self
    }

    pub fn owner_param_int(mut self, owner: &str, field: &str, value: i32) -> Self {
        self.owner_params.push(format!("{owner}.{field} = {value}"));
        self
    }

    pub fn raw(mut self, toml: &str) -> Self {
        self.raw.push(toml.to_string());
        self
    }

    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        for line in &self.item {
            out.push_str(line);
            out.push('\n');
        }
        if !self.common.is_empty() {
            out.push_str("\n[common]\n");
            for line in &self.common {
                out.push_str(line);
                out.push('\n');
            }
        }
        if !self.owner_params.is_empty() {
            out.push_str("\n[owner_params]\n");
            for line in &self.owner_params {
                out.push_str(line);
                out.push('\n');
            }
        }
        for raw in &self.raw {
            out.push('\n');
            out.push_str(raw);
            if !raw.ends_with('\n') {
                out.push('\n');
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_renders_the_tables_the_engine_reads() {
        let text = Manifest::new("bluster", "donkey")
            .costumes(8)
            .narration("vc_narration_characall_bluster")
            .article("cannonballcloned", "koopajr/cannonball")
            .kirby_article("blustercannonball", "koopajr/cannonball")
            .kirby(1)
            .kirby_first(0x410)
            .kirby_motion(Motion::new("bluster_special_n", "blusterd00specialn.nuanmb").template("donkey_special_n"))
            .param("dash_speed", 2.0)
            .param_int("jump_squat_frame", 3)
            .param_mul("common.shield_size", 3.0)
            .param_at(3, "weight", 100.0)
            .to_toml();
        let expected = "[fighter]\nname = \"bluster\"\nbase = \"donkey\"\ncostumes = 8\nnarration = \"vc_narration_characall_bluster\"\n\n[[article]]\nname = \"cannonballcloned\"\nfrom = \"koopajr/cannonball\"\n\n[[article]]\nname = \"blustercannonball\"\nfrom = \"koopajr/cannonball\"\nkirby = true\n\n[kirby]\nstatuses = 1\nfirst = 1040\n\n[[kirby.motion]]\nname = \"bluster_special_n\"\nanimation = \"blusterd00specialn.nuanmb\"\ntemplate = \"donkey_special_n\"\n\n[params]\ndash_speed = 2.0\njump_squat_frame = 3\n\n[params.mul]\ncommon.shield_size = 3.0\n\n[params.c03]\nweight = 100.0\n";
        assert_eq!(text, expected);
    }

    #[test]
    fn the_rendered_text_parses_back_into_the_same_declaration() {
        let text = Manifest::new("bluster", "donkey")
            .costumes(8)
            .series("donkey")
            .disp_order(2)
            .article("cannonballcloned", "koopajr/cannonball")
            .kirby_article("blustercannonball", "koopajr/cannonball")
            .kirby(1)
            .kirby_first(0x410)
            .kirby_motion(Motion::new("bluster_special_n", "blusterd00specialn.nuanmb").template("donkey_special_n"))
            .params(&[("walk_accel_mul", 0.105), ("dash_speed", 2.0)])
            .params_int(&[("jump_squat_frame", 3)])
            .params_mul(&[("common.shield_size", 3.0)])
            .to_toml();
        let parsed = crate::fighter_toml::parse_all(&text).expect("the builder writes what the parser reads");
        assert_eq!(parsed.len(), 1);
        let fighter = &parsed[0];
        assert!(!fighter.own_css);
        assert!(crate::fighter_toml::parse_all(&Manifest::new("a", "mario").own_css().to_toml()).unwrap()[0].own_css);
        assert_eq!(fighter.name, "bluster");
        assert_eq!(fighter.base, "donkey");
        assert_eq!(fighter.color_count, 8);
        assert_eq!(fighter.disp_order, Some(2));
        assert_eq!(fighter.articles.len(), 2);
        assert!(fighter.articles[1].kirby);
        let kirby = fighter.kirby.as_ref().expect("kirby table");
        assert_eq!((kirby.statuses, kirby.first), (1, Some(0x410)));
        assert_eq!(kirby.motions[0].template.as_deref(), Some("donkey_special_n"));
        assert_eq!(fighter.params.len(), 4);
        assert_eq!(fighter.params[2].name, "jump_squat_frame");
        assert!(matches!(fighter.params[2].value, crate::fighter_toml::ParamValue::Int(3)));
        assert!(matches!(fighter.params[3].op, crate::fighter_toml::ParamOp::Mul));
    }

    #[test]
    fn global_motions_render_after_kirby_and_parse_back() {
        let text = Manifest::new("bluster", "donkey")
            .kirby_motion(Motion::new("bluster_special_n", "blusterd00specialn.nuanmb"))
            .global_motion(Motion::from_template("catched_bluster", "catched_ganon"))
            .global_motion(
                Motion::from_template("thrown_bluster", "thrown_f")
                    .animation("d05thrownbluster.nuanmb")
                    .scripts("thrownbluster"),
            )
            .param("dash_speed", 2.0)
            .to_toml();
        assert!(text.contains(
            "\n[[global_motion]]\nname = \"catched_bluster\"\ntemplate = \"catched_ganon\"\n"
        ));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.kirby.as_ref().unwrap().motions.len(), 1);
        assert_eq!(fighter.global_motions.len(), 2);
        assert_eq!(fighter.global_motions[1].animation, "d05thrownbluster.nuanmb");
        assert_eq!(
            fighter.global_motions[1].script("effect", None).as_deref(),
            Some("effect_thrownbluster")
        );
        assert_eq!(fighter.params.len(), 1);
    }

    #[test]
    fn a_victim_global_motion_renders_and_parses_back() {
        let text = Manifest::new("bluster", "donkey")
            .global_motion(Motion::victim("thrown_bluster_f").animation("e01thrownblusterf.nuanmb"))
            .global_motion(Motion::victim("thrown_bluster_f_dx").scripts("thrownblusterf"))
            .to_toml();
        assert!(text.contains("\n[[global_motion]]\nname = \"thrown_bluster_f\"\nvictim = true\n"));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.global_motions.len(), 2);
        assert!(fighter.global_motions.iter().all(|motion| motion.victim));
        assert_eq!(fighter.global_motions[0].animation, "e01thrownblusterf.nuanmb");
        assert!(fighter.global_motions[1].animation.is_empty());
    }

    #[test]
    fn commands_render_and_parse_back() {
        use crate::command_input::{Button, Command};
        let kazuya = [[-20.0, 20.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0]];
        let text = Manifest::new("wawa", "mario")
            .vanilla_commands(true)
            .special_command_life_max(12)
            .special_command_neutral_threshold(0.2)
            .special_command_stick_range(kazuya)
            .command(Command::new("geyser", "21416").button(Button::Special).super_command().reverse().cpu(0.3))
            .command(Command::new("dash", "656").button(Button::None).life(8).neutral_resets(false))
            .to_toml();
        assert!(text.contains("\n[commands]\nvanilla = true\nspecial_command_life_max = 12\nspecial_command_neutral_threshold = 0.2\n"), "{text}");
        assert!(text.contains("\n[[command]]\nname = \"geyser\"\ninput = \"21416\"\nbutton = \"special\"\nsuper = true\nreverse = true\ncpu = 0.3\n"), "{text}");
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert!(fighter.command_settings.vanilla);
        assert_eq!(fighter.command_settings.life, Some(12));
        assert_eq!(fighter.command_settings.neutral_threshold, Some(0.2));
        assert_eq!(fighter.command_settings.stick_range, Some(kazuya));
        assert_eq!(fighter.commands[0], Command::new("geyser", "21416").button(Button::Special).super_command().reverse().cpu(0.3));
        assert_eq!(fighter.commands[1], Command::new("dash", "656").button(Button::None).life(8).neutral_resets(false));
    }

    #[test]
    fn cpu_ai_renders_and_parses_back() {
        let text = Manifest::new("wawa", "luigi").cpu_ai("ryu").to_toml();
        assert!(text.contains("cpu_ai = \"ryu\"\n"));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.cpu_ai.as_deref(), Some("ryu"));
    }

    #[test]
    fn item_hand_renders_and_parses_back() {
        let text = Manifest::new("wawa", "luigi").item_hand("left").to_toml();
        assert!(text.contains("item_hand = \"left\"\n"));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.item_hand, Some(crate::item_hand::ItemHand::Left));
    }

    #[test]
    fn grabs_render_and_parse_back() {
        let text = Manifest::new("bluster", "donkey")
            .grab(Grab::new(0x1dd).victim(0xbe).then(0x1e2))
            .grab(Grab::new(0x1de).victim(0xbd).unpinned().catch().offset(0.0, 2.5, 0.0))
            .grab(Grab::new(0x27).victim(0x41).then(0x38).node("haver"))
            .to_toml();
        assert!(text.contains("\n[[grab]]\nstatus = 0x1dd\nvictim = 0xbe\nthen = 0x1e2\nnode = \"throw\"\n"));
        assert!(text.contains("offset = [0.0, 2.5, 0.0]\ncatch = true\n"));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.grabs.len(), 3);
        assert_eq!(&fighter.grabs[0], Grab::new(0x1dd).victim(0xbe).then(0x1e2).rule());
        assert_eq!(
            &fighter.grabs[1],
            Grab::new(0x1de).victim(0xbd).unpinned().catch().offset(0.0, 2.5, 0.0).rule()
        );
        assert_eq!(fighter.grabs[2].node.as_deref(), Some("haver"));
    }

    #[test]
    fn article_hurtboxes_render_under_their_article_and_parse_back() {
        use crate::article_hurtbox::Hurtbox;
        let rock = Hurtbox::new("hip", 4.0).offset(0.0, 4.0, 0.0).offset2(0.0, 9.0, 0.0);
        let text = Manifest::new("wawa", "samus")
            .article("beam", "samus/cshot")
            .article("rock", "pacman/firehydrant")
            .article_hurtbox("rock", rock.clone())
            .article_hurtbox("rock", Hurtbox::new("top", 2.0).part(4))
            .to_toml();
        assert!(text.contains("from = \"pacman/firehydrant\"\n\n[[article.hurtbox]]\nnode = \"hip\"\n"));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert!(fighter.articles[0].hurtboxes.is_empty());
        assert_eq!(fighter.articles[1].hurtboxes.len(), 2);
        assert_eq!(fighter.articles[1].hurtboxes[0], rock);
        assert_eq!(fighter.articles[1].hurtboxes[1].part, 4);
    }

    #[test]
    fn article_original_keys_render_under_their_article_and_parse_back() {
        let text = Manifest::new("wawa", "link")
            .article("arrow", "link/bowarrow")
            .article("ring", "link/boomerang")
            .article_original_code("ring", false)
            .article_original_kind("ring", true)
            .to_toml();
        assert!(text.contains("from = \"link/boomerang\"\noriginal_code = false\noriginal_kind = true\n"));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert!(fighter.articles[0].runs_original_code());
        assert!(fighter.articles[0].shows_original_kind());
        assert!(!fighter.articles[1].runs_original_code());
        assert!(fighter.articles[1].shows_original_kind());
    }

    #[test]
    fn an_article_lvd_renders_under_its_article_and_parses_back() {
        use crate::article_hurtbox::Hurtbox;
        let plank = ArticleLvd::new("plank.lvd")
            .vertex_at(0, "top", 0.0, 6.0, -4.0)
            .vertex_facing(1, "plankr", "plankl");
        let text = Manifest::new("wawa", "samus")
            .article("plank", "koopajr/cannonball")
            .article_hurtbox("plank", Hurtbox::new("rot", 3.0))
            .article_lvd("plank", plank.clone())
            .to_toml();
        assert!(text.contains("from = \"koopajr/cannonball\"\nlvd = \"plank.lvd\"\n"));
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.articles[0].lvd.as_ref(), Some(&plank));
        assert_eq!(fighter.articles[0].hurtboxes.len(), 1);
    }

    #[test]
    fn two_articles_keep_their_own_lvds() {
        let pearl = ArticleLvd::new("pearl.lvd").vertex_at(0, "top", 0.0, 2.0, 0.0);
        let block = ArticleLvd::new("forge.lvd")
            .vertex_at(0, "top", -8.0, 8.0, 0.0)
            .vertex_at(1, "top", 8.0, 8.0, 0.0);
        let text = Manifest::new("herobrine", "mewtwo")
            .article("pearl", "mario/fireball")
            .article("block", "pickel/forge")
            .article_lvd("pearl", pearl.clone())
            .article_lvd("block", block.clone())
            .to_toml();
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        let lvd_of = |name: &str| {
            fighter.articles.iter().find(|article| article.name == name).and_then(|article| article.lvd.clone())
        };
        assert_eq!(lvd_of("pearl"), Some(pearl), "{text}");
        assert_eq!(lvd_of("block"), Some(block), "{text}");
    }

    #[test]
    fn a_hud_renders_its_own_table_and_parses_back() {
        let text = Manifest::new("wawa", "samus")
            .hud("cloud")
            .param("walk_speed_max", 1.5)
            .to_toml();
        assert!(text.contains("\n[hud]\nfrom = \"cloud\"\n"), "{text}");
        let fighters = crate::fighter_toml::parse_all(&text).unwrap();
        assert_eq!(fighters[0].hud.as_deref(), Some("cloud"));
        assert_eq!(fighters[0].params.len(), 1);
        let both = Manifest::new("wawa", "samus")
            .hud("jack")
            .hud_layout("ui/layout/hud/wawa/layout.arc")
            .to_toml();
        assert!(
            both.contains("\n[hud]\nfrom = \"jack\"\nlayout = \"ui/layout/hud/wawa/layout.arc\"\n"),
            "{both}"
        );
        let own = &crate::fighter_toml::parse_all(&both).unwrap()[0];
        assert_eq!(own.hud_layout.as_deref(), Some("ui/layout/hud/wawa/layout.arc"));
    }

    #[test]
    fn a_tether_renders_its_donor_and_boxes_and_parses_back() {
        let text = Manifest::new("wawa", "donkey")
            .tether("lucas")
            .tether_box([50.0, 40.0], [0.0, -30.0])
            .to_toml();
        assert!(text.contains("\n[tether]\nfrom = \"lucas\"\n"), "{text}");
        let fighters = crate::fighter_toml::parse_all(&text).unwrap();
        let tether = fighters[0].tether.clone().unwrap();
        assert_eq!(tether.from.as_deref(), Some("lucas"));
        assert_eq!(tether.boxes[0].words(), [50.0, 40.0, 0.0, -30.0]);
    }

    #[test]
    fn a_move_list_renders_and_parses_back() {
        let text = Manifest::new("wawa", "demon").move_list(6).move_list_images(&[0, 4]).to_toml();
        assert!(text.contains("\n[move_list]\npages = 6\nimage_pages = [0, 4]\n"), "{text}");
        let list = crate::fighter_toml::parse_all(&text).unwrap()[0].move_list.clone().unwrap();
        assert_eq!((list.pages, list.image_pages), (Some(6), Some(vec![0, 4])));
    }

    #[test]
    fn a_switch_group_renders_and_parses_back() {
        let text = Manifest::new("wawa", "pzenigame").switch_with(&["wawab", "wawac"]).to_toml();
        assert!(text.contains("switch = [\"wawab\", \"wawac\"]"), "{text}");
        let fighters = crate::fighter_toml::parse_all(&text).unwrap();
        assert_eq!(fighters[0].switch_with, vec!["wawab".to_string(), "wawac".to_string()]);
    }

    #[test]
    fn features_name_every_table_an_older_engine_would_refuse() {
        let manifest = Manifest::new("wawa", "mario")
            .hud("mario")
            .tether("link")
            .move_list(2)
            .partner(true)
            .raw("[[global_motion]]\nname = \"x\"\n");
        let used = manifest.features();
        for feature in [Feature::Hud, Feature::Tether, Feature::MoveList, Feature::AnyBaseGroups, Feature::GlobalMotions] {
            assert!(used.contains(&feature), "{feature:?} in {used:?}");
        }
        assert!(!used.contains(&Feature::Grabs));
        assert_eq!(Manifest::new("wawa", "popo").partner(false).features(), vec![Feature::EntryGroups]);
        assert_eq!(Manifest::new("wawa", "pzenigame").switch_with(&["a"]).features(), vec![Feature::EntryGroups]);
        assert_eq!(Manifest::new("wawa", "mario").switch_with(&["a"]).features(), vec![Feature::AnyBaseGroups]);
        assert!(Manifest::new("wawa", "mario").costumes(8).features().is_empty());
    }

    #[test]
    fn a_left_out_feature_drops_only_its_own_tables() {
        let manifest = Manifest::new("wawa", "mario")
            .hud("mario")
            .tether("link")
            .partner_with("wawab")
            .raw("[[global_motion]]\nname = \"x\"\n")
            .raw("[kirby]\nstatuses = 1\n");
        let text = manifest.to_toml_with(|feature| feature != Feature::Tether && feature != Feature::GlobalMotions);
        assert!(text.contains("[hud]"), "{text}");
        assert!(text.contains("partner = \"wawab\""), "{text}");
        assert!(text.contains("[kirby]"), "{text}");
        assert!(!text.contains("[tether]"), "{text}");
        assert!(!text.contains("global_motion"), "{text}");
        let text = manifest.to_toml_with(|feature| feature != Feature::AnyBaseGroups);
        assert!(!text.contains("partner"), "{text}");
        assert_eq!(manifest.to_toml(), manifest.to_toml_with(|_| true));
        assert!(crate::fighter_toml::parse_all(&manifest.to_toml_with(|_| false)).is_ok());
    }

    #[test]
    fn a_partner_switch_renders_and_parses_back() {
        let text = Manifest::new("wawa", "popo").partner(false).to_toml();
        assert!(text.contains("partner = false"), "{text}");
        assert!(!crate::fighter_toml::parse_all(&text).unwrap()[0].has_partner());
        let text = Manifest::new("wawa", "popo").to_toml();
        assert!(crate::fighter_toml::parse_all(&text).unwrap()[0].has_partner());
        let text = Manifest::new("wawa", "donkey").partner_with("wawab").to_toml();
        assert!(text.contains("partner = \"wawab\""), "{text}");
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.partner_with.as_deref(), Some("wawab"));
        let text = Manifest::new("wawa", "donkey").partner(true).to_toml();
        assert!(crate::fighter_toml::parse_all(&text).unwrap()[0].has_partner());
    }

    #[test]
    fn partner_params_render_their_own_table_and_parse_back() {
        let text = Manifest::new("wawa", "donkey")
            .partner(true)
            .partner_param_int("nana_delay", 20)
            .partner_param("nana_opt_dst", 40.0)
            .to_toml();
        assert!(text.contains("\n[partner_params]\nnana_delay = 20\nnana_opt_dst = 40.0\n"), "{text}");
        let fighter = &crate::fighter_toml::parse_all(&text).unwrap()[0];
        assert_eq!(fighter.partner_params[1].offset, 0x13c);
        assert_eq!(fighter.partner_params[1].bits, 40.0f32.to_bits());
    }

    #[test]
    fn an_lvd_for_an_undeclared_article_fails_the_parse() {
        let text = Manifest::new("wawa", "samus")
            .article_lvd("ghost", ArticleLvd::new("g.lvd").vertex(0, "a"))
            .to_toml();
        let message = crate::fighter_toml::parse_all(&text).unwrap_err().message;
        assert!(message.contains("ghost"), "{message}");
    }

    #[test]
    fn a_hurtbox_for_an_undeclared_article_fails_the_parse() {
        use crate::article_hurtbox::Hurtbox;
        let text = Manifest::new("wawa", "samus")
            .article_hurtbox("ghost", Hurtbox::new("hip", 1.0))
            .to_toml();
        let message = crate::fighter_toml::parse_all(&text).unwrap_err().message;
        assert!(message.contains("ghost"), "{message}");
    }

    #[test]
    fn floats_always_carry_a_point_and_strings_are_escaped() {
        assert_eq!(float(2.0), "2.0");
        assert_eq!(float(0.07337), "0.07337");
        assert_eq!(quoted("a\"b"), "\"a\\\"b\"");
        let text = ItemManifest::new("blusterblock", 430)
            .base_item("pickelobject")
            .owner_param_int("pickel", "life", 600)
            .owner_param("pickel", "auto_damage", 0.0)
            .to_toml();
        assert_eq!(
            text,
            "base_kind = 430\nresource_name = \"blusterblock\"\nbase_item = \"pickelobject\"\n\n[owner_params]\npickel.life = 600\npickel.auto_damage = 0.0\n"
        );
        assert_eq!(
            ItemManifest::new("wawa", "killsword").to_toml(),
            "base_item = \"killsword\"\nresource_name = \"wawa\"\n"
        );
        assert_eq!(
            ItemManifest::new("boss", "galleom").members(&["boss", "boss_missile"]).to_toml(),
            "base_item = \"galleom\"\nresource_name = \"boss\"\nmembers = [\"boss\", \"boss_missile\"]\n"
        );
    }
}
