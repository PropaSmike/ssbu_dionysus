# Dionysus API

Reference for packs that ship code. File-only packs use `fighter.toml`,
`item.toml` and `stage.toml`; see the [wiki](wiki/GETTING_STARTED.md). The
pre-`fighter.toml` calls are in [DEPRECATED.md](DEPRECATED.md).

Templates: [fighter](../custom_fighters/template_v2/),
[item](../custom_items/template_v2/).

```toml
[dependencies]
dionysus_api = { git = "https://github.com/PropaSmike/ssbu_dionysus", tag = "<release>" }
```

Use the tag of the Dionysus release you installed; releases from before the
rename name the crate `clone_engine_api`. Without Dionysus every call returns
`Error::EngineUnavailable`.

## Fighters

### Registration by code

```rust
use dionysus_api::v2::{fighter, hook, slot, Manifest};
use smash::hash40;
use smashline::{Agent, Main, Priority};

// WAWA: the handle the plugin uses everywhere. "wawa": the identity, the same
// string as Manifest::new's name.
fighter!(WAWA, "wawa");

const COSTUMES: u8 = 8; // how many costumes the pack ships (c00..), used everywhere below

#[skyline::main(name = "wawa")]
pub fn main() {
    let manifest = Manifest::new(
        "wawa",  // name: folder under fighter/, Smashline agent name, identity
        "samus", // base: the vanilla fighter it is built on
    )
    .costumes(COSTUMES as u32)      // the costume count
    .own_css()                      // the plugin publishes the select entry itself (publish_css below)
    .article("beam", "samus/cshot") // article "beam" (fighter/wawa/model/beam), copied from Samus's "cshot"
    .kirby(2);                      // reserve two status numbers for Kirby's copy
    let Ok(kind) = WAWA.register(manifest) else {
        return; // the fault is in the log
    };
    skyline::println!("[wawa] registered as fighter kind {kind}");

    publish_css();

    // values: name one ("group.field" inside a group), then .set, .mul or .int; see Parameters
    WAWA.param("param_special_hi.y_spd_air").set(1.5); // a vl.prc value, pushed to ParamConfig
    WAWA.param("jump_squat_frame").int(4);             // a fighter_param attribute, through the engine
    WAWA.param("weight").mul(1.02);                    // times the base's value

    Agent::new("wawa")                                              // scripts under the clone's own name
        .game_acmd("game_attack11", game_attack11, Priority::Default) // script name, your function, priority
        .on_line(Main, wawa_frame)                                  // once per frame
        .install();
    Agent::new("wawa_beam")                                         // the article's scripts and statuses: "<fighter>_<article>"
        .status(Main, *WEAPON_SAMUS_CSHOT_STATUS_KIND_FLY, beam_main) // status line, a status kind of the source weapon or a new number, your function
        .install();
    Agent::new("kirby")
        .status(Main, WAWA.kirby_status(0), kirby_special_n) // kirby_status(0): the first reserved number
        .install();
    WAWA.arm_kirby(); // after the Kirby statuses are installed
    dionysus_api::install_hooks!(on_link_event); // every #[hook] in the crate
}

// slot: which vtable entry. class: the class object the game passes first;
// object: the fighter; event: the entry's own argument.
#[hook(slot = slot::fighter::ON_LINK_EVENT)]
unsafe fn on_link_event(class: u64, object: *mut smash::app::BattleObject, event: u64) -> u64 {
    call_original!(class, object, event) // the base's entry
}
```

`register` goes first; everything else needs the kind. It returns the kind
or `Error::Engine(ERROR_MANIFEST)`; the log names the fault. Values are param
calls after it ([Parameters](#parameters)); the manifest's own `param` methods
are in [DEPRECATED.md](DEPRECATED.md#manifest-params).

| `Manifest` method | Meaning |
|---|---|
| `new(name, base)` | `name`: folder under `fighter/`, Smashline name, identity. `base`: vanilla fighter name. |
| `costumes(n)`, `color_start(n)` | Costume range. Default 8 from 0. |
| `name_label(s)`, `series(s)`, `disp_order(n)`, `save_no(n)`, `exhibit_year(n)` | Character select entry. `name_label` is the suffix of your `nam_chr*_00_<x>` labels; defaults to `name`. |
| `narration(label)` | Announcer call, `vc_narration_characall_<name>`. |
| `staffroll()` | Serve `standard/staffroll/texture/standard_staffroll_<name>.nutexb` from the pack. |
| `item_hand(hand)` | `"left"` or `"right"`: the hand that holds, throws and swings items. See [Item hand](wiki/FIGHTERS.md#item-hand). |
| `cpu_ai(fighter)` | the fighter whose CPU AI yours uses, such as `"ryu"`. See [CPU AI](wiki/FIGHTERS.md#cpu-ai). |
| `own_css()` | Do not publish the select entry; the pack's CSK code does, with `hash40("fighter_kind_<name>")` and `hash40("ui_chara_<name>")`. |
| `jingle(name)` | Accepted, not served yet. |
| `ui_chara(s)`, `fighter_kind_name(s)`, `resource_name(s)`, `base_resource_name(s)` | Override the names derived from `name` and `base`. |
| `owns_param_resources()`, `effect_namespace(n)`, `article_namespace(n)` | Leave out. |
| `article(name, "fighter/weapon")` | Copy that vanilla weapon into `fighter/<name>/model/<article>`, scripts under `<name>_<article>`. |
| `base_article(name, "weapon")` | Rename one of the base's own articles into that folder. |
| `article_original_code(article, on)`, `article_original_kind(article, on)` | Whether an `article` runs its source's scripts and shows the game its source's weapon kind. See [Articles](wiki/ARTICLES_AND_KIRBY.md#the-sources-code). |
| `kirby_article(name, "fighter/weapon")` | An article for Kirby's copy, files under `fighter/kirby/model/<name>/`, scripts under `kirby_<name>`. Ship a `cNN` folder for each costume and list it under `fighter/<clone>/kirbycopy/cNN`: the copied fighter's costume picks the folder, and without a loaded model the article is skipped. |
| `kirby(statuses)`, `kirby_first(n)` | Reserve that many copy status numbers; the engine picks them and `kirby_status(n)` reads them. `kirby_first` pins the first, only for code that hard-codes them. |
| `kirby_full_model()`, `kirby_model(dir)`, `kirby_mesh(name, visible)` | Full-body copy; one extra model folder under `fighter/kirby/model/`; a mesh's starting visibility (up to 16). |
| `kirby_motion(Motion)` | A copy animation: `Motion::new(name, "file.nuanmb")` with `.template(motion)`, `.scripts(stem)` or `.game/.sound/.effect/.expression(script)`, `.flags(&[..])`, `.blend_frames(n)`, `.cancel_frame(n)`, `.xlu(start, end)`, `.no_stop_intp(b)`, `.animation_unk(n)`, `.no_extra()`. File: `fighter/kirby/motion/<name>body/c00/`, listed under `fighter/<name>/kirbycopy/cNN/bodymotion`. Name must start with `<name>`. |
| `global_motion(Motion)` | Add a motion to every fighter, clones and every costume included. `Motion::from_template(name, template)` copies an existing motion from each fighter's own list; `.animation(file)` plays your file instead, for fighters that ship it; the `kirby_motion` options override the rest. `Motion::victim(name)` is the opponent's half of a throw: no animation, so the grabber's plays. A fighter that already has the name keeps its own. See [Motions on every fighter](wiki/FIGHTERS.md#motions-on-every-fighter). |
| `grab(Grab)` | Let a CATCH box catch outside the grab statuses. `Grab::new(status)` names the grabber status the box is out in; `.victim(status)` limits it to one victim status, `.then(status)` is the grabber's status once caught, `.node(joint)` holds the victim there (default `throw`), `.unpinned()` leaves it free, `.offset(x, y, z)`, `.catch()` calls `CatchModule::set_catch`. See [Command grabs](wiki/FIGHTERS.md#command-grabs). |
| `hud(donor)` | Show that fighter's HUD by this fighter's portrait: `"jack"`, `"cloud"`, `"inkling"`, `"brave"`, `"littlemac"`, `"shulk"`, `"reflet"`, `"pickel"`, `"trail"`, `"dolly"` or `"demon"`. Drive it with the `Fighter`'s `hud`. See [Borrowed HUDs](wiki/FIGHTERS.md#borrowed-huds). |
| `hud_layout(path)` | Draw the pack's own HUD layout, shipped at that arc path, on this fighter's damage card. Drive it with the `Fighter`'s `hud`. See [Your own HUD](wiki/FIGHTERS.md#your-own-hud). |
| `tether(donor)` | Turn the air tether on and take that fighter's tether timings and ledge boxes: `"samus"`, `"samusd"`, `"szerosuit"`, `"younglink"`, `"toonlink"`, `"lucas"` or `"luigi"`. See [Tethers](wiki/FIGHTERS.md#tethers). |
| `tether_box([x, y], [x, y])` | One ledge box of your own, two corners relative to the fighter. Replaces the donor's boxes. |
| `move_list(pages)`, `move_list_images(&[..])` | Extra move list pages, up to 32, and the ones that show a picture, counted from 0. See [Move list pages](wiki/FIGHTERS.md#move-list-pages). |
| `raw(toml)` | Append any table the builder lacks. |
| `to_toml()` | The text the engine receives. |

`v2::register_fighter_manifest(label, text)` registers that text directly.

### Character select entry

CSK's own call, as in a one-slot mod. Two fields name the clone instead of
the base, and the manifest says `own_css()` (`css = false` in
`fighter.toml`):

```rust
use the_csk_collection_api::*;

fn publish_css() {
    let ui_chara = hash40("ui_chara_wawa"); // the clone's UI id: ui_chara_<name>
    let mut indices = HashMap::new();
    let mut hashes = HashMap::new();
    for color in 0..COSTUMES {              // one set per costume the pack ships
        indices.insert(hash40(&format!("c{color:02}_index")), UnsignedByteType::Overwrite(color)); // costume index
        indices.insert(hash40(&format!("n{color:02}_index")), UnsignedByteType::Overwrite(color)); // name index
        indices.insert(hash40(&format!("c{color:02}_group")), UnsignedByteType::Overwrite(0));     // costume group
        hashes.insert(
            hash40(&format!("characall_label_c{color:02}")),          // announcer call for this costume
            Hash40Type::Overwrite(hash40("vc_narration_characall_wawa")),
        );
    }
    hashes.insert(hash40("original_ui_chara_hash"), Hash40Type::Overwrite(hash40("ui_chara_samus"))); // the base's UI id

    allow_ui_chara_hash_online(ui_chara); // let the entry be picked online
    add_chara_db_entry_info(CharacterDatabaseEntry {
        ui_chara_id: ui_chara,                                                 // the clone's UI id
        clone_from_ui_chara_id: Some(hash40("ui_chara_samus")),                // copy every other field from the base's entry
        name_id: StringType::Overwrite(CStrCSK::new("wawa")),                  // suffix of the nam_chr*_00_<x> labels in msg_name.xmsbt
        fighter_kind: Hash40Type::Overwrite(hash40("fighter_kind_wawa")),      // the clone's identity: fighter_kind_<name>
        fighter_kind_corps: Hash40Type::Overwrite(hash40("fighter_kind_wawa")), // same
        ui_series_id: Hash40Type::Overwrite(hash40("ui_series_metroid")),      // series icon
        disp_order: SignedByteType::Optional(Some(40)),                        // position on the select screen
        color_num: UnsignedByteType::Overwrite(COSTUMES),                      // number of costumes
        shop_item_tag: Hash40Type::Overwrite(hash40("-1")),                    // the base's DLC fields hide the entry; clear them
        alt_chara_id: Hash40Type::Overwrite(hash40("-1")),
        save_no: SignedByteType::Overwrite(0),
        is_dlc: BoolType::Overwrite(false),
        is_patch: BoolType::Overwrite(false),
        extra_index_maps: UnsignedByteMap::Overwrite(indices),
        extra_hash_maps: Hash40Map::Overwrite(hashes),
        ..Default::default()
    });
    for color in 0..COSTUMES {              // one layout per costume; the base has only 8, so wrap
        add_chara_layout_db_entry_info(CharacterLayoutDatabaseEntry {
            ui_layout_id: hash40(&format!("ui_chara_wawa_{color:02}")),                       // layout id: ui_chara_<name>_<costume>
            clone_from_ui_layout_id: Some(hash40(&format!("ui_chara_samus_{:02}", color % 8))), // copy the base's layout for that costume
            ui_chara_id: Hash40Type::Overwrite(ui_chara),                                     // the clone's UI id
            chara_color: UnsignedByteType::Overwrite(color),                                  // the costume
            ..Default::default()
        });
    }
}
```

`fighter_kind` and `fighter_kind_corps` are `hash40("fighter_kind_<name>")`,
`ui_chara_id` is `hash40("ui_chara_<name>")`. The rest is CSK's.

Without `own_css()` the engine publishes an entry itself from `name_label`,
`series`, `disp_order`, `save_no`, `exhibit_year` and `narration`. Do not do
both; CSK keeps the last one written.

`disp_order` is a place on the vanilla select screen: the same number as
a vanilla fighter puts the clone right after it. A player's `disp_order` for a
clone wins over the pack's, whichever loads first: in any
`mods:/database/*.json` (`character_database_entries`), or in the
`ui/param/database/ui_chara_db.prc` the game loads, with every mod's `.prc`,
`.prcx` and `.prcxml` applied. The JSON wins over the file. When the player's
table renumbers the vanilla fighters, a clone it does not list moves with the
vanilla fighter its number pointed at. A partial JSON entry is merged into the
clone's full row.

### `fighter.toml`

The same declaration as a file beside `config.json`, read at boot. The plugin
then skips `register`; `WAWA.ready()` is already true. If both exist the
file wins.

```toml
name = "wawa"             # folder under fighter/, Smashline agent name, identity
base = "samus"            # the vanilla fighter it is built on
costumes = 8              # costumes c00..c07
css = false               # the plugin publishes the CSK entry

[[article]]
name = "beam"             # article folder fighter/wawa/model/beam, scripts under "wawa_beam"
from = "samus/cshot"      # the vanilla weapon it is copied from

[kirby]
statuses = 2              # status numbers reserved for Kirby's copy

[params.mul]
weight = 1.02             # multiply the base's value
```

With no plugin, drop `css = false` and add `series`, `disp_order`,
`display_name`, `narration`; the engine publishes the entry.

| Key | `Manifest` method |
|---|---|
| `name`, `base`, `costumes`, `color_start`, `series`, `disp_order`, `save_no`, `exhibit_year`, `narration`, `staffroll`, `item_hand`, `cpu_ai`, `jingle`, `ui_chara`, `fighter_kind_name`, `resource_name`, `base_resource_name` | Same name. |
| `display_name` | `name_label`. |
| `css = false` | `own_css()`. |
| `[[article]]` with `name`, `from` or `base_article`, `kirby = true` | `article`, `base_article`, `kirby_article`. |
| `[[article]]` `original_code`, `original_kind` | `article_original_code(article, on)`, `article_original_kind(article, on)`. See [Articles](wiki/ARTICLES_AND_KIRBY.md#the-sources-code). |
| `[[article.hurtbox]]` with `node`, `size`, `offset`, `offset2`, `part`, `height`, `status`, `shape` | `article_hurtbox(article, Hurtbox)`. See [Articles](wiki/ARTICLES_AND_KIRBY.md#hurtboxes). |
| `[[article]]` `lvd`, `[[article.lvd_vertex]]` with `vertex`, `joint`, `offset`, `joint_left` | `article_lvd(article, ArticleLvd)`. See [Articles](wiki/ARTICLES_AND_KIRBY.md#platforms-on-any-article). |
| `[kirby]` with `statuses`, `first`, `full_model`, `model`; `[[kirby.mesh]]` with `name`, `visible` | `kirby`, `kirby_first`, `kirby_full_model`, `kirby_model`, `kirby_mesh`. |
| `[[kirby.motion]]` with `name`, `animation`, `template`, `scripts` or `game`/`sound`/`effect`/`expression`, `flags` or `loop`, `blend_frames`, `cancel_frame`, `xlu`, `no_stop_intp`, `animation_unk`, `no_extra` | `kirby_motion`. |
| `[[global_motion]]` with `name`, then `template`, `animation`, `victim = true` or a mix, plus any `[[kirby.motion]]` key | `global_motion`. |
| `[[grab]]` with `status`, `victim`, `then`, `node`, `offset`, `catch` | `grab`. A status is a number or a name: `ATTACK`, `ATTACK_DASH`, `ATTACK_S3`, `ATTACK_HI3`, `ATTACK_LW3`, `ATTACK_S4`, `ATTACK_HI4`, `ATTACK_LW4`, `ATTACK_AIR`, `SPECIAL_N`, `SPECIAL_S`, `SPECIAL_HI`, `SPECIAL_LW`, `CATCH`, `CATCH_PULL`, `CATCH_DASH`, `CATCH_DASH_PULL`, `CATCH_TURN`, `CATCH_WAIT`, `THROW`, `CAPTURE_PULLED`, `CAPTURE_WAIT`, `CATCHED_GANON`, `CATCHED_AIR_GANON`, `CLUNG_GANON`, `CLUNG_CAPTAIN`, `KOOPA_DIVED`, `SHOULDERED_DONKEY_START`, `SWALLOWED`, `BITTEN_WARIO_START`, `CLUNG_DIDDY`, `CAPTURE_YOSHI`, `CATCHED_REFLET`, `CATCHED_RIDLEY`, `MIIFIGHTER_SUPLEX_AIR_CAPTURED`, `CAPTURE_JACK_WIRE`, `DOLLY_SUPER_SPECIAL2_CAPTURE`, `CAPTURE_MASTER_SWORD`, `DEMON_DIVED`. `status` and `then` also take `"own N"`: your fighter's own status N, counted from 0 past the base's (`own_status(N)` in code). |
| `[hud]` with `from`, `layout` | `hud`, `hud_layout`. |
| `[tether]` with `from`, `[[tether.box]]` with `p1`, `p2` | `tether`, `tether_box`. |
| `[move_list]` with `pages`, `image_pages` | `move_list`, `move_list_images`. See [Move list pages](wiki/FIGHTERS.md#move-list-pages). |
| `partner` (`true`, `false` or a fighter's name), `[partner_params]` | `partner`, `partner_with`, `partner_param`, `partner_param_int`. See [Partner](wiki/FIGHTERS.md#partner). |
| `switch` | `switch_with`. See [Switching](wiki/FIGHTERS.md#switching). |
| `[commands]` with `vanilla`, `special_command_life_max`, `super_special_command_life_max`, `special_command_ab_button_life`, `special_command_neutral_threshold`, `special_command_stick_range`; `[[command]]` with `name`, `input`, `button`, `super`, `life`, `neutral_resets`, `reverse`, `cpu` | `vanilla_commands`, the `special_command_*` and `super_special_command_life_max` methods, `command(Command)`. See [Command inputs](wiki/FIGHTERS.md#command-inputs). |
| `[params]`, `[params.mul]`, `[params.cNN]`, `[params.cNN.mul]` | `param*`. A decimal is a float, a whole number an integer. |

Several fighters in one file: `[[fighter]]` blocks, with `[[fighter.article]]`,
`[fighter.kirby]` and so on.

### `Fighter`

| Method | Meaning |
|---|---|
| `kind()` | The kind this boot. Never store it. |
| `ready()` | Registered. |
| `is(x)` | `x` is this fighter. `x`: `&L2CFighterCommon`, `&L2CAgentBase`, `&L2CWeaponCommon`, `*mut BattleObject`, `*mut Fighter`, `*mut Weapon`, `*mut BattleObjectModuleAccessor`, `u64`. |
| `owns(x)` | `x` is this fighter or one of its weapons or articles. |
| `entries()`, `in_match()` | Match slots it occupies. |
| `hud(x, Hud::...)` | Change the borrowed HUD: `Gauge(v)`, `State(on)`, `Select(i)`, `Slot(i, v)`, `MenuOpen([MenuCommand; 4])`, `MenuCursor(row)`, `MenuDecide(row)`, `MenuClose`. False when that HUD has no such call. See [Borrowed HUDs](wiki/FIGHTERS.md#borrowed-huds). Change your own layout: `Play(animation)`, `Frame(animation, frame)`, `Show(part, on)`, `Text(part, text)`. False when the layout on screen has no such part. See [Your own HUD](wiki/FIGHTERS.md#your-own-hud). |
| `article("name")` | `.status(line, status, f)`, `.install()`, `.weapon_kind()`, `.index()`, `.spawn(boma)`, `.set_vtable_entry(n, f)`, `.set_common_vtable_entry(n, f)`. Read `index()` right before use; never fall back to 0. `.status` installs through Smashline on `<fighter>_<article>`; new code writes statuses with `smashline::Agent` there. |
| `copy_article("name")` | Same, for a `kirby_article`. |
| `param("name")`, `param("group.field")` | `.slot(n)`, then `.set(v)`, `.mul(v)` or `.int(v)` ([Parameters](#parameters)). |
| `kirby_family()`, `kirby_status(n)`, `arm_kirby()` | The reserved statuses. Arm after the `Agent::new("kirby")` statuses are installed. |
| `own_status(n)` | Status number `n` of your fighter's own, counted from 0 past the base's statuses. Call after `register`; -1 before. |
| `set_vtable_entry(n, f)` | Write one vtable entry; returns the previous. |
| `set_common_vtable_entry(n, f)`, `set_copy_common_vtable_entry(n, f, &ORIGINAL)` | [Common vtables](#common-vtables). |
| `register(manifest)` | Above. A table the installed Dionysus cannot read (global motions, grabs, HUD, tether, move list, command inputs, partner and switch groups) is left out and logged; the rest registers. |

Compare fighters with `is` and `owns`; `utility::get_kind` answers the base.

Under Smashline 2.0 or later, Smashline numbers article weapon kinds, shared with
other mods' `clone_weapon` calls. Read `weapon_kind()` and `index()` at runtime;
never assume `615 + n`. A built article answers `utility::get_kind` with its
source's kind; `article_kind(boma)` gives its own (see
[Articles](wiki/ARTICLES_AND_KIRBY.md#articles)). A base fighter of kind 94 or above takes no articles there.

### Scripts

Smashline, under your own name:

```rust
smashline::Agent::new("wawa")                                            // the clone's name, never the base's
    .game_acmd("game_attack11", attack11, smashline::Priority::Default)  // script name, your function, priority
    .status(smashline::Pre, *FIGHTER_STATUS_KIND_SPECIAL_N, special_n_pre) // status line, status kind, your function
    .on_line(smashline::Main, fighter_frame)                             // once per frame
    .install();
```

Smashline 2.0's `replace_fighter_vtable`, `replace_fighter_vtable_costume` and
`get_original_fighter_vtable` take a clone's kind. A costume listed by a
`_costume` call runs only the entries that list it; the others run only the
entries with no list. `get_original_fighter_vtable` answers the clone's own
entry, `#[hook(slot = ...)]` included. Only these vtable calls claim a costume
here: `set_costume` on a script agent does not. Call them after `register`.

`replace_weapon_vtable` takes an article's kind (`article("name").weapon_kind()`)
and writes the article's own copy. `get_original_weapon_vtable` answers the
source weapon's entry.

### Mount event

`v2::on_mods_mounted(callback)`: runs once ARCropolis has mounted the mods.

## `#[hook]`

```rust
// slot: the fighter vtable entry to replace, by name.
// class: the class object the game passes first; fighter: the fighter; log: the entry's own argument.
#[hook(slot = slot::fighter::ON_SEARCH)]
unsafe fn on_search(class: u64, fighter: *mut smash::app::Fighter, log: u64) -> u64 {
    call_original!(class, fighter, log) // the base's entry, with the same arguments
}

// article: which of the fighter's articles; slot: a weapon vtable entry.
#[hook(slot = slot::weapon::ON_ATTACK, article = "fireball")]
unsafe fn fireball_hit(class: u64, weapon: *mut smash::app::Weapon, log: u32) -> u32 {
    call_original!(class, weapon, log)
}

// item: the item! static; slot: an item vtable entry. item: the item object (no class object first).
#[hook(slot = slot::item::UPDATE_9, item = BLOCK)]
unsafe fn block_update(item: *mut smash::app::BattleObject) {
    call_original!(item);
    smash::app::lua_bind::ModelModule::set_scale((*item).module_accessor, 1.5); // after the original: the frame's last word
}

// offset: address of a game function in main; me: the parameter that must be this fighter's,
// otherwise the call goes straight to the original.
#[hook(offset = 0x33bd9c0, me = weapon)]
unsafe fn weapon_hit(vtable: u64, weapon: *mut smash::app::Weapon, log: u32) -> u32 {
    call_original!(vtable, weapon, log)
}
```

Declare the entry's real return type: the game reads it, and a hook declared
with none hands back whatever the register holds.

| Attribute | Meaning |
|---|---|
| `slot = slot::fighter::NAME` | Replace that entry in the fighter's own vtable copy. Runs for this fighter only. First parameter is the class object, the fighter second. Names: `slot::fighter::*` (146), `slot::weapon::*` (104). |
| `slot = slot::weapon::NAME, article = "name"` | Same, in the article's own copy. |
| `slot = slot::item::NAME, item = ITEM` | Same, in the item's own copy (181 entries). First parameter is the item object. `INITIALIZE(item, id, kind, flag, record)` at every spawn, `kind` is the base's; `START(item)` after it; `UPDATE_1..UPDATE_9(item)` each frame in order, statuses run inside them. Unnamed entries are `UNKn`. |
| `common_slot = slot::common::NAME` | Replace that entry in the agent's own copy of the shared `L2CFighterCommon` vtable. First parameter is the agent. Slots 10 and 12 return `L2CValue`. Add `article = "name"` for `L2CWeaponCommon`, `item = ITEM` for an item agent (slots 0 to 9). |
| `copy_slot = slot::common::NAME` | Replace that entry on Kirby, while he holds this fighter's copy ability. Slots 10 to 14 only, and it stands alone. |
| `offset = 0x...` | Hook game code that is not a vtable entry, shared with other packs: each call goes to the first hook that takes it, in load order, then to the game. Up to six integer or pointer parameters, integer, pointer or no return. Every hook on one address declares the same parameter count. |
| `me = <parameter>` | Offset form only: skip the call unless that parameter is this fighter's. `of = OTHER` names another fighter. Without it the hook takes every call, and hooks loaded after it on that address never run. |
| `expect = [w0, w1, w2, w3]` | Offset form: the words expected at the address. Without it the address must be an untouched function start, or one another pack already hooks through the engine. |

`install_hooks!(a, b, ...)` installs them, after registration. A hook on the
base's function through plain `#[skyline::hook]` also runs for the clone;
`replace` is not supported.

`v2::get_agent_virtual_function(kind, index, is_weapon, get_ptr)`: the
tutorials' signature, returns 0 instead of aborting; `get_ptr = true` on a
clone kind is 0.

## Common vtables

Every fighter shares one `L2CFighterCommon` vtable, every weapon one
`L2CWeaponCommon`, every item one `L2CAgentGeneratedBase`. An entry written
there is written for the whole roster, so two packs writing the same entry
overwrite each other. `common_slot` writes it in a copy private to this
clone's agent, and it runs for this clone only.

```rust
use smash::lib::L2CValue;

// common_slot: the shared agent entry to replace, by name.
// The agent object itself is the first parameter, not a separate class object.
#[hook(common_slot = slot::common::SYS_LINE_SYSTEM_INIT)]
unsafe fn system_init(agent: u64) -> L2CValue {
    call_original!(agent) // the entry this replaced, vanilla or another pack's
}

// article: the entry goes in that article's L2CWeaponCommon copy instead.
#[hook(common_slot = slot::common::SUB_BEGIN_ADDED_LINES, article = "fireball")]
unsafe fn fireball_lines(agent: u64, lines: L2CValue) -> u64 {
    call_original!(agent, lines)
}

// item: the entry goes in that item's own agent vtable copy, slots 0 to 9.
// The parameters after the agent are the entry's own; match the real signature.
#[hook(common_slot = slot::common::START_COROUTINE, item = BLOCK)]
unsafe fn block_start(agent: u64, index: i32, name: u64, state: *mut i32) -> u32 {
    call_original!(agent, index, name, state)
}
```

`SYS_LINE_SYSTEM_INIT` (10) and `SYS_LINE_STATUS_END_CONTROL` (12) return an
`L2CValue` through x8: declare `-> L2CValue` with skyline-smash's type and
return what `call_original!` gives back. 10 runs once, as the agent starts; 12
at the end of every status. `SUB_BEGIN_ADDED_LINES` (11), `SUB_END_ADDED_LINES`
(13) and `RESET` (14) return no struct: declare `-> u64`. 11 also takes an
`L2CValue`. The macro checks both: a missing `L2CValue` on 10 or 12 lets the
original write to wherever x8 last pointed, and one on 11, 13 or 14 makes the
hook write through an x8 the game never set.

`slot::common::*`, `COUNT` entries:

| Slot | Name | Present on |
|---|---|---|
| 0 | `DESTRUCTOR` | all three |
| 1 | `DELETER` | all three |
| 2 | `COROUTINE_YIELD` | all three |
| 3 | `START_COROUTINE` | all three |
| 4 | `RESUME_COROUTINE` | all three |
| 5 | `GET_UNUSED_COROUTINE_INDEX` | all three |
| 6 | `CLEAN_COROUTINE` | all three |
| 7 | `SET_COROUTINE_RELEASE_CONTROL` | all three |
| 8 | `IS_COROUTINE_RELEASE_CONTROL` | all three |
| 9 | `SET_STATUS_SCRIPTS` | all three |
| 10 | `SYS_LINE_SYSTEM_INIT` | fighter, weapon |
| 11 | `SUB_BEGIN_ADDED_LINES` | fighter, weapon |
| 12 | `SYS_LINE_STATUS_END_CONTROL` | fighter, weapon |
| 13 | `SUB_END_ADDED_LINES` | fighter, weapon |
| 14 | `RESET` | fighter, weapon |

An item agent shares the first **ten** of those slots, `SET_STATUS_SCRIPTS`
included. Slots 10 to 14 are refused for an item: those indices exist, but on
an item they are the item class's own generated virtuals, not the system lines
the names describe.

Item agent vtables are per item class and run from 10 to 118 slots, so the
engine sizes the private copy from the class's real count rather than from the
shared prefix. An agent whose count it cannot establish is refused and logged,
because repointing to a short table would leave every slot above the copy
reading past its end. Every member of an Assist Trophy, Pokemon or boss family
is an item clone and gets its own copy too.

Smashline gives each fighter and weapon object its own copy of the same
vtable, and the engine writes into that copy rather than replacing it, so a
`common_slot` hook and a Smashline common hook compose. If the agent still
wears the shared vtable when the engine first looks, it waits and writes once
Smashline has made the copy; an agent that never gets one is refused and
logged.

Item agents get the engine's own copy. Smashline inspects every agent, items
included, so the engine keeps its copies inside its own module, where
Smashline treats them as vanilla vtables. The log states where the copies live
and whether Smashline will accept them.

### Kirby copy abilities

A fighter's `common_slot` hooks never reach Kirby: the object running the
copied ability is Kirby's, a vanilla kind. `copy_slot` registers the entry
Kirby runs while he holds this fighter's ability, and it is deliberately
separate, because the status lines run on every Kirby status and silent
inheritance would run a clone's code through all of Kirby's ordinary
behaviour.

```rust
// copy_slot: the entry Kirby runs while he holds this fighter's copy ability.
// Slots 10 to 14 only. No slot, common_slot or offset alongside it.
#[hook(copy_slot = slot::common::SYS_LINE_STATUS_END_CONTROL)]
unsafe fn kirby_status_end(agent: u64) -> L2CValue {
    call_original!(agent) // Kirby's own entry, also when no copy is held
}
```

A `copy_slot` hook on `SYS_LINE_SYSTEM_INIT` does not fire for a copy: Kirby's
runs when his agent starts, before he can hold one.

`agent` is Kirby's agent, so read Kirby's modules from it, not this fighter's.
The routing is decided per call and per object, so two Kirbys holding two
different clones' abilities each get their own.

### By code

| Function | Meaning |
|---|---|
| `Fighter::set_common_vtable_entry(n, f)` | One common entry for this fighter. |
| `Fighter::set_copy_common_vtable_entry(n, f, &ORIGINAL)` | The Kirby copy entry. `ORIGINAL` is a `static AtomicUsize` the engine fills with the entry replaced; returns whether it was taken. |
| `Article::set_common_vtable_entry(n, f)` | In that article's `L2CWeaponCommon` copy. |
| `Item::set_common_vtable_entry(n, f)` | In that item's copy, slots 0 to 9. |
| `v2::set_common_vtable_entry(kind, n, f)` | By kind, fighter space. |
| `v2::set_common_vtable_entry_in(kind, n, space, f)` | `v2::SPACE_FIGHTER`, `v2::SPACE_WEAPON`, `v2::SPACE_ITEM`. |

These return the entry previously registered for that slot, or `None` if it
was free. That is a registration, not the vanilla function: use the `#[hook]`
form when you want `call_original!`.

## Parameters

Every param call has one shape: name the value, then `.set(v)`, `.mul(factor)`
or `.int(v)`, and `.label("...")` or `.hash("...")` for a field that holds a
name. `"group.field"` reaches a field inside a group (`.sub("field")` does the
same). Each returns `false` when the value is refused.

| Call | Sets |
|---|---|
| `WAWA.param("name")` | a fighter's values, below |
| `WAWA.cpu_param("name")`, `v2::cpu_param(kind, "name")` | its CPU's row ([below](#cpus)) |
| `ITEM.param("key")`, `v2::item_param(kind, "key")` | an item's params of its own |
| `ITEM.common("field")`, `v2::item_common(kind, "field")` | its row of `item/common/param/param.prc` ([Items](#item)) |
| `ITEM.owner_param("fighter", "field")`, `v2::item_owner_param(kind, "fighter", "field")` | its owner fighter's `vl.prc` values, for this item only |
| `enemy::param(kind, "key")`, `enemy::common(kind, "field")` | an enemy's params of its own, its row of `enemy/common/common.prc` ([Enemies](#enemies)) |
| `stage::param(place, "key")`, `stage::sound(place, "field")`, `stage::cpu_area(place, "edge")` | a stage's params of its own, its sound row, its CPU box ([Stages](#stages)) |

`.mul` scales the base's value; stages have none to scale.

A fighter's values take two syntaxes, by where the game reads them.

| Values | Syntax |
|---|---|
| `vl.prc`: `param_special_*`, `param_private`, `fly_data`, `air_lasso_data`, everything under the fighter's own file | ParamConfig |
| `fighter_param.prc` fields (the attributes: `walk_speed_max`, `jump_y`, `scale`, `weight`, ...) | engine |
| `fighter_param_motion.prc` fields (`param_motion`) | engine |
| `fighter/common/param/` files: `common.prc`, `item.prc`, `etc.prc`, `power_up.prc`, `effect.prc`, `sound.prc` (`common`) | engine |
| `fighter_param_thrown.prc` (`param_thrown`) | engine |
| lists and arrays inside `vl.prc` (`hit_data`, `hit_target`, `map_coll_data`, `cliff_hang_data`, ...) | your own `vl.prc` file |
| lists inside the common files, `battle_object.prc`, `spirits.prc`, the match-level fields of the common files | cannot be changed per clone |

### ParamConfig

`vl.prc` values, read through `WorkModule::get_param_*`, which ParamConfig
hooks. Its calls, with the clone's kind where the base kind was; `vec![-1]`
is every costume. The `param_config` crate carries ParamConfig itself, so
the calls work whether or not `libparam_config.nro` is installed.

```rust
let kind = WAWA.kind().get().unwrap();                 // the clone's kind, in place of *FIGHTER_KIND_SAMUS
let slots: Vec<i32> = (0..COSTUMES as i32).collect();  // costumes; vec![-1] is all of them

// (kind, costumes, (param table, field), value)
param_config::update_float(kind, slots.clone(), (hash40("param_special_hi"), hash40("y_spd_air")), 1.5);
// a top-level field has no table: second hash is 0
param_config::update_int(kind, slots.clone(), (hash40("jump_squat_frame"), 0), 4);
// multiply the base's float value
param_config::update_attribute_mul(kind, slots.clone(), (hash40("param_special_n"), hash40("fireball_speed_mul")), 1.15);
// multiply the base's integer value
param_config::update_int_mul(kind, vec![-1], (hash40("landing_frame"), 0), 0.5);

// (kind, costumes): Kirby cannot copy these costumes
param_config::disable_kirby_copy(kind, vec![-1]);
let beam = WAWA.article("beam").weapon_kind().unwrap(); // the article's weapon kind; 0 means every weapon the fighter owns
// (kind, costumes, weapon kind, behaviour): what Kirby's inhale does to the article
param_config::set_kirby_inhale_behavior(kind, vec![-1], beam, param_config::POCKET_BEHAVIOR_MISFIRE);
// what Villager's pocket does to it
param_config::set_villager_pocket_behavior(kind, vec![-1], beam, param_config::POCKET_BEHAVIOR_DELETE);
// same as POCKET_BEHAVIOR_MISFIRE
param_config::disable_villager_pocket(kind, vec![-1], beam);
// what Rosalina's pull does to it
param_config::set_rosetta_pull_behavior(kind, vec![-1], beam, param_config::POCKET_BEHAVIOR_IGNORE);
// (weapon kind, use type): the article's use type
param_config::set_article_use_type(beam, *ARTICLE_USETYPE_FINAL);
```

Behaviours: `POCKET_BEHAVIOR_ORIGINAL`, `IGNORE`, `DELETE`, `MISFIRE`.

`dionysus_api::param_*` send the same keys to ParamConfig, for a pack
without the `param_config` crate: `param_disable_kirby_copy(kind)`,
`param_kirby_inhale_behavior(kind, weapon_kind, behavior)`,
`param_villager_pocket_behavior`, `param_disable_villager_pocket`,
`param_rosetta_pull_behavior`, `param_article_use_type(weapon_kind,
use_type)`, and for one costume `param_disable_kirby_copy_slot(kind, slot)`,
`param_kirby_inhale_behavior_slot(kind, slot, weapon_kind, behavior)`,
`param_villager_pocket_behavior_slot`, `param_disable_villager_pocket_slot`,
`param_rosetta_pull_behavior_slot`.

### Engine

The four groups below are not read through `get_param_*`, or not only: the
game reads `fighter_param.prc` and `fighter_param_motion.prc` rows through
`FighterParamAccessor2` and, for most fields, straight out of the row
(`scale` is never read any other way); the six common files are per-fighter
copies read directly; `fighter_param_thrown.prc` has no field names. A
ParamConfig entry for these applies to some reads and not others. The engine
writes the clone's row, copies and results, and pushes `set` values to
ParamConfig too, so one call covers every read.

```rust
// param(name) or param("group.field"), .slot(n) for one costume,
// then .set(value), .mul(factor) or .int(value)
WAWA.param("walk_speed_max").set(1.2);                         // fighter_param.prc field: no group
WAWA.param("scale").set(1.1);                                  // read once at construction, engine only
WAWA.param("param_motion.escape_air_slide_distance").set(60.0); // fighter_param_motion.prc field
WAWA.param("param_motion.flip").int(0);                        // an integer field
WAWA.param("common.shield_max").set(20.0);                     // any of the six common files
WAWA.param("common.shield_reset").set(7.5);                    // set with shield_max, the game does not clamp it
WAWA.param("param_thrown.offset_f_y").set(8.0);                // where a fighter this clone throws sits: forward throw, Y
WAWA.param("param_thrown.held_offset").mul(1.2);               // where the clone sits when thrown: whole vector, mul only
```

```toml
[params]
walk_speed_max = 1.2
param_motion.flip = 0                 # a whole number is an integer
common.shield_max = 20.0

[params.mul]
param_thrown.held_offset = 1.2

[params.c03]
param_special_hi.y_spd_air = 2.0      # costume 3 only; a vl.prc value, pushed to ParamConfig
```

| Group | Field | Notes |
|---|---|---|
| none: a `fighter_param.prc` field | the field | Every costume only. |
| `param_motion` | a `fighter_param_motion.prc` field | |
| `common` | a top-level field of `common.prc`, `item.prc`, `etc.prc`, `power_up.prc`, `effect.prc`, `sound.prc` | Lists inside them and the ~40 match-level fields (handicap, team attack, finish camera, area wind, `power_up_point_min`) stay global. |
| `param_thrown` | holder: `offset`, `offset_f`, `offset_b`, `offset_hi`, `offset_lw`; victim: `held_offset`; each also `_x`, `_y`, `_z` | Whole vector: `mul` only. Component: `set` or `mul`. Every costume only. |

### CPUs

A clone's CPU reads its base's row (or its `cpu_ai` fighter's) of
`fighter/common/ai/param/fighter_param.prc` (recovery reach, attack ranges,
Final Smash range) unless you give it values:

```rust
// cpu_param(name) then .set(value), .mul(factor) or .int(value), like param
WAWA.cpu_param("width").set(12.0);                      // a number field
WAWA.cpu_param("jump_high").mul(1.2);                   // times the base's value
WAWA.cpu_param("num_of_attack_123").int(2);             // a whole-number field
WAWA.cpu_param("jump_sp_u_c0").int(0);                  // an on/off field: 0 or 1
v2::cpu_param(kind, "range_long").set(45.0);            // by kind, for a fighter registered by fighter.toml
```

Each returns `false` when the kind is unknown or the field is not one the row
can take. Or ship `fighter/<name>/ai/param/fighter_param.prcxml`: its first
`base` row, whatever its index or `name_hash`. Code sets over the file.
`final_type` is a label and stays the base's. Set values before the battle:
one set during it waits for the next, unless the clone already had values
when it began.

### Lists and arrays

Neither syntax keys into a list. `vl.prc` lists (`hit_data`, `hit_target`,
`map_coll_data`, `cliff_hang_data`, `jostle_map_coll_data`,
`virtual_node_data_hit`, ...) come from your own
`fighter/<name>/param/vl.prc`, declared under every costume in
`config.json`; a costume that misses it falls back to the base's whole file.
Lists inside the common files, `battle_object.prc` and `spirits.prc` cannot
be changed per clone.

## Items

### Registration by code

```rust
use dionysus_api::v2::{item, ItemManifest};
use dionysus_api::ItemStatusLine;

// WAWA: the handle. "wawa": the resource name, same as ItemManifest::new's.
item!(WAWA, "wawa");

#[skyline::main(name = "wawa_item")]
pub fn main() {
    let manifest = ItemManifest::new(
        "wawa",         // resource name: files under item/wawa, script name
        "pickelobject", // the vanilla item it is built on, here Steve's block
    )
    .spawn_per(30); // natural drop weight
    if WAWA.register(manifest).is_err() {
        return; // the fault is in the log
    }
    WAWA.common("throw_speed_mul").set(0.75);    // a field of item/common/param/param.prc
    WAWA.owner_param("pickel", "life").int(600); // owner fighter (Steve), a field of its vl.prc
    WAWA.status(ItemStatusLine::Init, "THROW", throw_init); // status line, status name of the base item, your function
}
```

| `ItemManifest` method | Meaning |
|---|---|
| `new(resource_name, base)` | Files under `item/<resource_name>`; the vanilla item it is built on, by name (`"killsword"`) or kind number. |
| `base_item(name)` | Names the base in the log when `new` got a number. |
| `members(&[..])` | Clones an Assist Trophy, Pokemon or boss family: every member's resource name in the base family's order, this item first. See [Items](wiki/ITEMS.md#assist-trophies-pokemon-and-bosses). |
| `agent_name(name)` | Script name. Default `resource_name`. |
| `ui_id(id)` | Training menu id. Default `ui_item_<resource_name>`. |
| `training_order(n)` | Position in the Training list. |
| `spawn_per(weight)` | Natural drops. Vanilla items sit at 20..50, capsules at 100. Follows the base item's item switch. |
| `spawn_range(min, max)` | How many appear at once. Default 1, 1. |
| `spawn_from(&["box", "barrel"])` | Container drops: `box`, `barrel`, `capsule`, `carrierbox`, `kusudama`, `sandbag`, `grass`. |
| `raw(toml)`, `to_toml()` | As for `Manifest`. |

`v2::register_item_manifest(label, text)` registers that text directly. Values are
param calls after `register` ([Parameters](#parameters)); the manifest's own
`common` and `owner_param` methods are in [DEPRECATED.md](DEPRECATED.md#manifest-params).

### `item.toml`

```toml
base_item     = "pickelobject"      # the vanilla item it is built on, here Steve's block
resource_name = "wawa"              # files under item/wawa, script name
agent_name    = "wawa"              # optional: script name, default resource_name
ui_id         = "ui_item_wawa"      # optional: Training menu id, default ui_item_<resource_name>
training_order = 0                  # optional: position in the Training list
spawn_per  = 30                     # optional: natural drop weight; none without it
spawn_min  = 1                      # optional: how many appear at once
spawn_max  = 2
spawn_from = "box, barrel, capsule" # optional: containers that can hold it

[common]
throw_speed_mul = 0.75              # a float field of item/common/param/param.prc
life = 300                          # a whole number is an integer field

[owner_params]
pickel.life = 600                   # owner fighter, then a field of its vl.prc
pickel.auto_damage = 0
```

Keys map to the methods above by name. A decimal is a float, a whole number
an integer. Several items: `[[item]]` blocks (engines after `0.2.1-beta.1`).

### `Item`

| Method | Meaning |
|---|---|
| `kind()`, `ready()`, `is(x)` | As for `Fighter`. |
| `status(line, "NAME", f)` | Status callback by name, registered at startup. Lines: `Setting`, `JointSrt`, `Init`, `Update`, `Coroutine`, `Exit`. Statuses are the base item's; an unknown name is refused. |
| `owner_param("fighter", "field")` | A field of that fighter's `vl.prc`, for this item only: `.set(v)`, `.int(v)` or `.mul(factor)`. `v2::item_owner_param(kind, "fighter", "field")` by kind. |
| `common("field")` | One field of its row of `item/common/param/param.prc`: `.set(v)`, `.int(v)` or `.mul(factor)` for a number, `.label("item_shield_kind_lost")` for a kind field, `.hash("top")` for a bone or motion name. `v2::item_common(kind, "field")` by kind. |
| `vtable_entry(n)`, `set_vtable_entry(n, f)` | The item's own vtable copy. |
| `set_common_vtable_entry(n, f)` | Its own agent vtable copy, slots 0 to 9. See [Common vtables](#common-vtables). |
| `param("key")` | A key of the item's own params (`"group.key"` for a group): `.set(v)`, `.mul(factor)`, `.int(v)`. `v2::item_param(kind, "key")` by kind. Read with `app::item::self_param_float`. See [Params of your own](wiki/PARAMETERS.md#params-of-your-own). |
| `register(manifest)` | Above. |

Spawning from code:

```rust
ItemModule::have_item(
    boma,                                            // the fighter that gets the item
    smash::app::ItemKind(WAWA.kind().get().unwrap()), // the clone's kind, wrapped in the game's type
    0, 0, false, false,                              // the game's own arguments, as for a vanilla item
);
```

`born_item` and `attach_item` take the kind the same way.
`ItemModule::get_have_item_kind` answers the base kind; use `item_kind_held`.

| Function | Meaning |
|---|---|
| `item_kind_for_identity(resource_name)` | The kind a registered name landed on. |
| `item_base_kind(kind)` | Its vanilla base. |
| `is_item_kind(kind)` | Registered by the engine. |
| `item_resource_name(kind)` | Its file root. |
| `item_kind_from_object(object)`, `item_kind_from_boma(boma)` | Kind of a live item. Unsafe. |
| `item_kind_held(boma, index)`, `item_kind_pickable(boma)` | What a fighter holds or could pick up. Unsafe. |
| `item_common_has(hash)` | Whether a common field is supported. |
| `item_backend_status()` | `ITEM_BACKEND_STATUS_*` readiness bits. |

Common fields: `have_kind`, `size_kind`, `hit_kind`, `shield_kind`,
`reflect_kind`, `flip_type`, `bound_flag`, `scale_type`, `camera_kind`,
`eatable`, `paintable`, `ai_pri`, `thrown_rot_kind`, `thrown_node`,
`clung_node`, `captured_motion`, ... `group` cannot be set. The older
`item_common_set*` calls are in [DEPRECATED.md](DEPRECATED.md#items).

`members = [...]` in `item.toml`, or `ItemManifest::members`, registers a
family with no other code. Item families (`register_item_family`,
`item_category`, `item_family_owner`, `item_family_member_index`,
`item_spawn_source`, `item_parent_kind`) clone
Assist Trophies, Pokemon and bosses with their members. Give each member
`KIND_AUTO` and read its kind back with `item_kind_for_identity(name)`; the
item template registers one. A member ships its files under its base's top
folder, `<root>/<resource>/` with `<root>` being `assist`, `pokemon` or `boss`,
with every folder in `config.json`'s `new-dir-infos` and `new-dir-files`, as an
item does under `item/`; without them it plays with the base's model and motions.
Its `motion_list.bin` names its own folder at offset 8
(`hash40("<root>/<resource>/motion/body/c00")`); a copy of the base's list
plays the base's animations. An Assist or Pokemon loads into the next resource
container of its category when the base holds the first. `item::spawn` loads a clone's own resources
before creating it; the game alone loads them only while creating the first
item of the kind, which then plays with the base's motions.

## Enemies

Ultimate's stage enemies (Yellow Devil, Dark Emperor, Flying Men, Metal Face,
Nabbit) on any stage, and new enemy kinds made from them:
`dionysus_api::enemy`. The enemy template is a complete pack.

```rust
use dionysus_api::enemy;

let kind = enemy::clone(enemy::NABBIT, "my_enemy").unwrap();      // base, name: files under enemy/my_enemy
enemy::drive(kind, Some(my_driver), enemy::statuses(&[0])).unwrap(); // your driver runs its statuses; status 0 stays the base's
enemy::arm(enemy::kinds(&[kind])).unwrap();                          // loaded in every battle from the next one on
let request = enemy::spawn(kind, 0.0, 20.0, 0.0, 1.0).unwrap();   // kind, x, y, z, facing (1.0 right, -1.0 left)
let object = enemy::start(request);                               // on a later frame: Some(battle object id) once it is made
```

| Function | Meaning |
|---|---|
| `enemy::arm(kinds)` | Load these kinds (`enemy::kinds(&[kind, ...])`) in every battle from the next one on; 0 turns it off. On the five stages with an enemy of their own, only that enemy loads. |
| `enemy::spawn(kind, x, y, z, lr)` | Ask for an enemy; returns a request number. It is made on a later frame. |
| `enemy::start(request)` | Bring it out; `None` until it is made. |
| `enemy::object(index)` | The battle object id in enemy object `index` (below `enemy::OBJECTS`), live or not. |
| `enemy::clone(base, name)` | A new kind that is `base` in all it does and loads `enemy/<name>/model`, `motion`, `param` and `script`, `effect/enemy/<name>` and `sound/bank/enemy/se_enemy_<name>.*`. Kinds 5 to 31; a name registered again answers its kind. |
| `enemy::drive(kind, Some(driver), native)` | Every status the clone's class asks for, except those in `native` (`enemy::statuses(&[0, ...])`), runs `driver(object, module_accessor, status)` every frame instead of the base's logic. Damage and grabs stay the base's. `None` gives the kind back. |
| `enemy::drive_now(object)` | End what the game is doing with a driven enemy (a hit's reaction) and hand it to the driver at once. |
| `enemy::class_status(object)` | Its class status (each base numbers its own); `None` for no enemy. |
| `enemy::remove(object)` | Take it off the stage. |
| `enemy::common(kind, "field")` | One field of the clone's own row of `enemy/common/common.prc` (`hp`, `weight`, `egg_scale`, `capture_frame`, ...): `.set(v)`, `.int(v)` or `.mul(factor)`, `.hash("damagestart")` for `capture_motion`. The rest stays the base's. Labels (`category`, `size_category`, ...) are refused. |
| `enemy::param(kind, "key")` | A key of the clone's own params (`"group.key"` for a group): `.set(v)`, `.mul(factor)`, `.int(v)`. Read with `WorkModule::get_param_float(boma, hash40("param"), hash40("key"))`; `enemy/<name>/param/param.prcxml` sets the same. |
| `enemy::set_vtable_entry(kind, slot, function)` | Replace one entry of the clone's 77-slot class vtable; the base's own enemies keep the game's. Set it before the battle; a null function gives the slot back. |
| `enemy::original_entry(kind, slot)` | The entry it replaced, to call from your function; `None` until the first enemy of the clone is built. |
| `enemy::set_object_vtable_entry(kind, slot, function)` | Replace one entry of the clone's copy of the enemy object's 163-slot vtable, otherwise one table for every enemy kind. |
| `enemy::original_object_entry(slot)` | The game's entry for that slot, to call from your function. |

Slots in `enemy::slot`: `JOINT_ROTATION`, `DAMAGE`, `CHANGE_STATUS` and
`REQUEST_STATUS` (`fn(instance: usize, status: i32)`), `INSTALL_STATUS`,
`INSTALL_REQUEST`; the others by number, below `slot::COUNT`. A driven clone's
hook runs before its driver's routing. A clone brought out gets its first
status through `REQUEST_STATUS` when driven and `CHANGE_STATUS` when not.

Object slots in `enemy::object_slot`: `INITIALIZE`, `START`, `UPDATE_1` to
`UPDATE_9` (`fn(object: *mut BattleObject)`, the same `BattleObject` entries
as `slot::item`'s); the others by number, below `object_slot::COUNT`.

Kinds: `YELLOW_DEVIL`, `DARK_EMPEROR`, `FLYING_MAN`, `METAL_FACE`, `NABBIT`. A
Nabbit clone reads its own parameters from
`enemy/<name>/model/body/c00/param.prc`. Its row of `enemy/common/common.prc`
can come from `enemy/<name>/common.prcxml` (the first `enemy_param_table`
row, whatever its index or `kind`); `enemy::common` sets over it.

An `enemy.toml` beside `config.json` declares the clone without code:

```toml
name = "my_enemy"   # files under enemy/my_enemy/, effect/enemy/my_enemy/
base = "nabbit"     # yellow_devil, dark_emperor, flying_man, metal_face or nabbit
```

- The engine clones it at boot; `enemy::clone(base, name)` then answers the
  same kind. Several enemies: `[[enemy]]` blocks.
- The game's own enemy logic runs only on its home stage: elsewhere Nabbit
  waits out of sight and only Flying Man fights. The plugin spawns the enemy
  and drives it, as the enemy template does; the Workbench writes that code.
- Its `motion_list.bin` names its own folder at offset 8
  (`hash40("enemy/<name>/motion/body/c00")`), or it plays the base's animations.
- The game's tests for one enemy (Yellow Devil, Dark Emperor, Flying Man),
  CPU fighters and fighter scripts (`utility::get_kind`) see a clone as its
  base. A plugin's `utility::get_kind` answers the clone's own kind.

## Stages

`stage.toml` registers stages; see [Stages](wiki/STAGES.md). A stage plugin
holds code and registers nothing. The old calls are in
[DEPRECATED.md](DEPRECATED.md#stages).

A cloned stage's rows, by its `place` name, from any plugin. The field names
are the prcxml files' (a trailing `_` is optional):

```rust
use dionysus_api::stage;

stage::sound("my_stage", "reverb_type_").int(2);            // 0 to 10, see Stages
stage::sound("my_stage", "scale_").set(1.2);
stage::sound("my_stage", "offset_").set(-15.0);
stage::cpu_area("my_stage", "x0").set(-80.0);               // the Normal form's box: x0, y0, x1, y1,
stage::cpu_area("my_stage", "y0").set(-5.0);                // used once all four are known
stage::cpu_area("my_stage", "x1").set(80.0);
stage::cpu_area("my_stage", "y1").set(40.0);
stage::param("my_stage", "wind").set(0.5);                  // a key of its own: .set or .int
let wind = stage::param_float("wind");                      // the stage being played: Some(0.5), None if unset
stage::set_items_start("my_stage", true).unwrap();          // like items_start = "all"
stage::set_item("my_stage", "heart", false).unwrap();       // like items_off; true like items_on
```

They set over `stage.toml` and the stage's prcxml files; the last call for an
item wins. The item calls switch items on and off; they are not params.

## Menus

`menu.toml` covers sequences, layouts, preloads, menu script patches and galleries; see
[Menus](wiki/MENUS.md).

| Function | Meaning |
|---|---|
| `menu::register_sequence(code, runs_as)` | A main menu button with `code` starts the vanilla mode `runs_as`; `menu::vanilla_code("other_training")` gives a mode's code by the game's name. |
| `menu::active_sequence()` | The custom code that started the last mode, `None` after a vanilla one. |
| `menu::register_call(name, handler)` | Answers `prg_func.engine_call(name, ...)` in main menu Lua. |
| `menu::battle_player(slot, kind, ui_chara, pad)` | Player selection for the next battle (`PlayerType::Human`, `Cpu`, `Amiibo`, `None`). |
| `menu::battle_stage(ui_stage)` | Stage selection for the next battle. |
| `menu::battle_damage(slot, percent)` | Starting damage for the next battle (`battle_player` resets it to 0). |
| `menu::battle_color(slot, color)` | Costume for the next battle, 0 to 255 (`battle_player` resets it to 0). |
| `menu::battle_rules(Rules { kind, stock, seconds, items })` | Rules for the next battle: `RuleKind::Time`, `Stock` or `Stamina`, stocks, time limit (0 = none), item frequency (0 = none). |
| `menu::battle_pause(allowed)` | Whether the players may pause the next battle; `battle_rules` turns it on. |
| `menu::battle_stock(slot)` | Stocks a player has left in the running battle. |
| `menu::next_battle()` | Ends the running battle and starts a new one from the current selections. |
| `menu::end_battle()` | Ends the running battle and returns to the main menu. |
| `menu::battle_frame()` | Battle frames so far, counted by the HUD whoever is on stage. |
| `menu::change_fighter(slot, ui_chara, color)` | Swap a player's fighter in the running battle; call it on the KO and the new fighter comes in at the rebirth. |
| `menu::mode_sequence(id)` | The code the engine gave the `[[mode]]` or `[[sequence]]` named `id`; compare it with `active_sequence()`. |
| `menu::battle_boss(Some(kind))` | Make the next battle a boss battle against boss item `kind` (a native boss or a boss family clone) on its stage; `None` turns it off. |
| `menu::battle_boss_object()` | The boss's battle object id once the game made it. |
| `menu::battle_boss_finished()` | The game counts the boss defeated. |
| `menu::viewer_begin(slot)` | Readies viewer slot 0 or 1 on the Model viewer screen and hides its side panel. |
| `menu::viewer_state(slot)` | `ViewerState::Loading`, `Built` or `Shown`. |
| `menu::viewer_end()` | Brings the side panel back. |
| `menu::gallery_item_count()` | Items in the open gallery. |
| `menu::gallery_item_costume(index)` | Costume of item `index`, 0 to 255. |

The viewer and gallery calls return `Error::Engine(-1)` on a screen with no viewer, with no gallery open or for an index past the last item; call them from a registered call. See [Model viewer](wiki/MENUS.md#model-viewer).

A handler is `unsafe extern "C" fn(args: *const i64, count: usize) -> i64`.

## Mii hats and outfits

`mii.toml` only; see [Mii hats and outfits](wiki/MII_PARTS.md). Check
`runtime_capabilities() & CAP_MII_PARTS`.

## Spirits

`spirit.toml` only, spirits and DLC spirit boards; see [Spirits](wiki/SPIRITS.md).
Check `runtime_capabilities() & CAP_SPIRITS`.

## Battle HUD

| Function | Meaning |
|---|---|
| `hud_layout_every(path)` | Draw the layout at that arc path on every player's damage card whose fighter has no layout of its own. Call before the battle. |
| `hud_entry(entry, HudAction::...)` | `Play`, `Frame`, `Show`, `Text`, `Width` on player `entry`'s layout. |
| `hud_screen_layout(name, path, x, y, scale)` | Draw a layout (a pack's or the game's) on the battle HUD at a screen place and scale, not on a card; several names may share a path. Call before the battle. |
| `hud_screen(name, HudAction::...)` | The same calls on screen layout `name`. |
| `boss_gauge_battle()` | Give the next battle the game's boss HP bar. Call before the battle loads. |
| `boss_gauge(item, name)` | Put an item on that bar under `name`; returns its row (two at most). The bar shows the item's `ITEM_INSTANCE_WORK_FLOAT_HP` of `_HP_MAX`. |
| `boss_gauge_name(row, name)` | Rename a row. |
| `boss_gauge_hide(hide)` | Leave the game's boss bar out of the next battle's HUD, for a mod that draws its own. |

See [On every player's card](wiki/FIGHTERS.md#on-every-players-card).

## Engine state

| Function | Meaning |
|---|---|
| `api_version()` | Installed engine version. |
| `compiled_capabilities()`, `runtime_capabilities()` | `CAP_*` bits built in, and passing this boot. `register` already leaves out what the installed Dionysus lacks; use these only to branch your own code. |
| `feature_capability(Feature)` | The `CAP_*` bit of a manifest feature. |
| `max_custom_kind()` | Highest fighter kind accepted. |
| `kind_for_identity("fighter_kind_<name>")` | The kind given to that identity. |
| `smashline_bridge_version()`, `smashline_compatible()` | Smashline build found. |
| `native_backend_status()` | Diagnostic flags. |
| `log(&str)`, `elog!(..)` | Engine log. |
| `fighter_manager()` | The game's `FighterManager`, for `lua_bind::FighterManager` calls such as `start_finalbg`. |

Registration closes when the roster is built: `ERROR_REGISTRATION_CLOSED`.

## Errors

`Error::Engine(code)`:

| Code | Meaning |
|---|---|
| `ERROR_BACKEND_UNAVAILABLE` | Not in this build. |
| `ERROR_REGISTRATION_CLOSED` | Registered after the roster was built. |
| `ERROR_DUPLICATE` | Identity or kind taken; the log says what by whom. |
| `ERROR_NAMESPACE` | No effect or article namespace left. |
| `ERROR_ARTICLE_RESOURCE_CONFLICT` | Two articles in one folder, or, under Smashline 2.0, the name of a vanilla article. |
| `ERROR_ARTICLE_OWNER` | Owner not found, or, under Smashline 2.0, a base fighter of kind 94 or above. |
| `ERROR_HOOK_PREFLIGHT` | Game code did not match the hook's expectation. |
| `ERROR_HOOK_ABI` | Hook signature not supported. |
| `ERROR_ITEM_UI_METADATA` | Training menu data malformed. |
| `ERROR_ITEM_UI_UNAVAILABLE` | Item UI layer unavailable. |
| `ERROR_MANIFEST` | Manifest did not parse or registered nothing. |
| `ERROR_SMASHLINE_REQUIRED` | Retired. |

Other constants in `dionysus_api`.

## C ABI

Exports are `clone_engine_*`; the Rust client is the authoritative list.
Every structure is `#[repr(C)]` with `api_version` and `struct_size` first
and zeroed reserved fields.
