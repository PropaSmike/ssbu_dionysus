# Fighters

## Declaring the fighter

In the plugin, before anything else:

```rust
use dionysus_api::v2::{fighter, Manifest};

fighter!(MY_FIGHTER, "my_fighter");

let manifest = Manifest::new("my_fighter", "mario")
    .costumes(8)
    .own_css();
let Ok(kind) = MY_FIGHTER.register(manifest) else { return };
```

Or as a `fighter.toml` beside the pack's `config.json`, in which case the
plugin (if any) skips `register`:

```toml
name = "my_fighter"
base = "mario"
costumes = 8
css = false
```

Every key and method is in the [API reference](../API.md#fighters).

## Character select entry

Your CSK call stays as it is. Two fields name the clone:

```rust
add_chara_db_entry_info(CharacterDatabaseEntry {
    ui_chara_id: hash40("ui_chara_my_fighter"),
    clone_from_ui_chara_id: Some(hash40("ui_chara_mario")),
    fighter_kind: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),
    fighter_kind_corps: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),
    ..
});
```

The full call is in the [API reference](../API.md#character-select-entry).
A pack with no plugin drops `css = false` and adds `display_name`, `series`,
`disp_order` to `fighter.toml`; the engine publishes the entry.

Select-screen pictures (`chara_<N>_<name_id>_<costume>.bntx`) go under
`ui/replace/chara/chara_<N>/` or, as the DLC fighters' do,
`ui/replace_patch/chara/chara_<N>/`. A fighter reads one tree (`is_patch` in
its row): the engine points yours at the tree that has your pictures, so keep
them in one.

Classic Mode plays `ui/param/standard/standard_route_<name_label>.prc` when
your pack ships one, and the base's route otherwise. The route preview titles
yours `mnu_standard_sub_<name_label>` (ship it in `ui/message/msg_menu.xmsbt`)
and the base's route with the base's title.

## Stats

ParamConfig's calls stay as they are, with `kind` where the base kind was:

```rust
param_config::update_float(kind, vec![-1], (hash40("param_special_hi"), hash40("y_spd_air")), 1.5);
```

See [Stats and parameters](PARAMETERS.md).

## Scripts

Smashline scripts go under your own fighter name, never the base's:

```rust
smashline::Agent::new("my_fighter")
    .game_acmd("game_attack11", game_attack11, smashline::Priority::Default)
    .status(smashline::Exec, FIGHTER_STATUS_KIND_WAIT, wait_exec)
    .on_line(smashline::Main, fighter_frame)
    .install();
```

`MY_FIGHTER.is(x)` and `MY_FIGHTER.owns(x)` are the checks for a plain
`#[skyline::hook]`; Smashline scripts under `"my_fighter"` need none.

## Motions on every fighter

A grab or a throw plays a motion on the opponent, so every fighter needs it. A
global motion adds one to every fighter's motion list, clones and every costume
included:

```rust
use dionysus_api::v2::Motion;

Manifest::new("my_fighter", "mario")
    .global_motion(Motion::from_template("catched_my_fighter", "catched_ganon"))
    .global_motion(Motion::victim("thrown_my_fighter_f").animation("e01thrownmyfighterf.nuanmb"))
```

```toml
[[global_motion]]
name = "catched_my_fighter"
template = "catched_ganon"

[[global_motion]]
name = "thrown_my_fighter_f"
victim = true
animation = "e01thrownmyfighterf.nuanmb"
```

- `template` copies a motion each fighter already has. A fighter without it is
  skipped; one that already has the name keeps its own.
- `animation` replaces it for fighters that ship that file in their motion
  folders (for vanilla, all of c00 to c07).
- `victim = true` is the opponent's half of a throw: no animation, so the
  opponent plays the grabber's file. Ship it for your fighter only.

Play it with `MotionModule::change_motion` on an opponent your grab is holding;
a grabbed pose on a fighter nobody holds crashes.
`custom_fighters/template_v2/src/throws.rs` does this for a forward throw. A
plugin with no fighter calls `dionysus_api::global_motion` or
`global_victim_motion`. Every key is in the [API reference](../API.md#fighters).

## Command grabs

A CATCH box only catches when the grabber accepts. The game accepts by itself
in its grab statuses (`CATCH`, `CATCH_DASH`, `CATCH_TURN`); anywhere else the
victim slips out. A grab rule accepts it in your own status:

```rust
use dionysus_api::v2::Grab;

Manifest::new("my_fighter", "mario")
    .grab(Grab::new(*FIGHTER_STATUS_KIND_SPECIAL_S)
        .victim(*FIGHTER_STATUS_KIND_CATCHED_GANON)
        .then(MY_STATUS_SPECIAL_S_CATCH))
```

```toml
[[grab]]
status = "SPECIAL_S"        # the grabber status the CATCH box is out in
victim = "CATCHED_GANON"    # the status the box gives the victim; any if left out
then = "own 0"              # the grabber status once caught: your fighter's first own status
```

- The victim is held at the `throw` joint. `node = "hip"` picks another,
  `node = ""` leaves it free, as in Ganondorf's Dark Dive. `offset = [x, y, z]`
  moves it.
- `catch = true` calls the grabber's `CatchModule::set_catch(victim)`, for a `then`
  status that does not.
- The victim plays the motion its status names (`catched_ganon` for
  `CATCHED_GANON`) from your motion list: list it with your animation, plus
  `_dx` and `_girl`. Every other fighter's entry is empty, so they all play
  yours.
- `victim = "CAPTURE_PULLED"` with `then = "CATCH_PULL"` makes the move an
  ordinary grab with your pummel and throws.
  `custom_fighters/template_v2` does this on side special.
- `then` starts before the victim is linked; read `LINK_NO_CAPTURE` from
  `Main` on.

A status is a number or a name from the
[API reference](../API.md#fighters). A plugin with no manifest calls
`dionysus_api::grab(kind, &Grab)`.

## Borrowed HUDs

Show another fighter's HUD by your portrait and drive it from your code.

```rust
use dionysus_api::v2::{Hud, MenuCommand};

Manifest::new("my_fighter", "mario")
    .hud("pickel")

MY_FIGHTER.hud(fighter, Hud::Slot(0, 12.0)); // 12 dirt
```

```toml
[hud]
from = "pickel"
```

| `from` | Shows | Calls |
|---|---|---|
| `jack` | Rebellion gauge | `Gauge(v)` |
| `cloud` | Limit gauge, charged look at 1 | `Gauge(v)` |
| `inkling` | Ink tank | `Gauge(v)` |
| `littlemac` | KO meter, "KO" at 1 | `Gauge(v)` |
| `brave` | MP bar (`v` x 100) and the command menu | `Gauge(v)`, `MenuOpen`, `MenuCursor(row)`, `MenuDecide(row)`, `MenuClose` |
| `shulk` | Monado Art and its timer | `Select(art)`, then `Gauge(v)` for the timer |
| `reflet` | A tome and its uses left | `Slot(tome, v)` |
| `pickel` | Material bar | `Slot(material, count)` |
| `trail` | Spell | `Select(spell)` |
| `dolly` | GO! | `State(on)` |
| `demon` | Rage | `State(on)` |

`v` runs from 0 (empty) to 1 (full).

- Shulk's arts: 0 Jump, 1 Speed, 2 Shield, 3 Buster, 4 Smash. The timer bar
  hides at 0 and at 1.
- Robin's tomes: 0 Levin Sword, 1 Elwind, 2 Thunder, 3 Arcfire, 4 Nosferatu.
- Steve's materials: 0 dirt, 1 wood, 2 stone, 3 iron, 4 gold, 5 redstone,
  6 diamond; a count is a whole number up to 999.
- Sora's spells: 0 Firaga, 1 Blizzaga, 2 Thundaga.

Hero's menu has four rows, counted from 0 by `MenuCursor` and `MenuDecide`:

```rust
MY_FIGHTER.hud(fighter, Hud::MenuOpen([
    MenuCommand::new(2, 8.0),             // Sizz, 8 MP
    MenuCommand::new(14, 8.0),            // Snooze
    MenuCommand::new(5, 37.0).unusable(), // Kaboom, greyed out
    MenuCommand::new(1, 7.0),             // Heal
]));
```

Commands: 0 blank, 1 Heal, 2 Sizz, 3 Sizzle, 4 Bang, 5 Kaboom, 6 Whack,
7 Thwack, 8 Magic Burst, 9 Kamikazee, 10 Kaclang, 11 Acceleratle, 12 Oomph,
13 Bounce, 14 Snooze, 15 Hocus Pocus, 16 Zoom, 17 Flame Slash,
18 Kacrackle Slash, 19 Metal Slash, 20 Hatchet Man, 21 Psyche Up.

- Only the picture is borrowed. It starts empty or off, the meter lives in
  your code, and the donor's moves stay with the donor.
- A call that HUD has no use for returns false.
- Without the `Fighter` handle: `dionysus_api::hud(boma, &HudAction::Gauge(0.5))`.
- `custom_fighters/template_v2/src/hud.rs` fills Joker's gauge with damage.

## Your own HUD

Draw your own layout on your damage card. It is a new file in your pack, so
no vanilla file is replaced.

```rust
Manifest::new("my_fighter", "mario")
    .hud_layout("ui/layout/hud/my_fighter/layout.arc")

MY_FIGHTER.hud(fighter, Hud::Play("charge_in"));        // play an animation
MY_FIGHTER.hud(fighter, Hud::Frame("gauge", 30.0));     // hold one at a frame
MY_FIGHTER.hud(fighter, Hud::Show("icon_fire", false)); // hide a part
MY_FIGHTER.hud(fighter, Hud::Text("count", "12"));      // set a text box
```

```toml
[hud]
layout = "ui/layout/hud/my_fighter/layout.arc"   # can go with from
```

The file is a layout archive like the game's
`ui/layout/info/info_melee/info_melee/layout.arc`:

| Path in the archive | Holds |
|---|---|
| `blyt/<layout>.bflyt` | The layout: 1920 x 1080, drawn from the center. |
| `anim/<layout>_<animation>.bflan` | One file per animation. |
| `timg/<any>.bntx` | Its textures. |
| `info/layout.info` | What the game builds, see below. |

`info/layout.info` lists `<layout>.bflyt`, then its animation names, each
followed by a zero byte, then one more zero byte. Parts follow the same way
(name without `.bflyt`). Two zero bytes end the file. For `charge.bflyt`
with `charge_in` and `gauge`:

```text
charge.bflyt\0charge_in\0gauge\0\0\0\0
```

- (0, 0) is the left edge of the percent on the damage card, +x right, +y up.
  The layout moves, scales, fades and hides with the card.
- The root pane belongs to the engine; hide a part under it instead.
- Each player using the fighter gets its own copy.
- `Show`, `Frame` and `Text` sent before the HUD appears apply when it does;
  `Play` does not.
- A missing file, part or animation returns false and is logged.

### On every player's card

A plugin that is not a fighter can draw one layout on every player's damage
card, vanilla fighters included, and drive it by player slot:

```rust
dionysus_api::hud_layout_every("ui/layout/hud/my_mode/layout.arc")?; // before the battle
unsafe { dionysus_api::hud_entry(0, &HudAction::Show("gauge", true)) }?;
```

- A fighter with a layout of its own keeps it.
- Same file format and calls as above; the slot is the match entry, from 0.

## Packs written before `fighter.toml`

They register with `CloneRegistration` and `allocate`, and check `is_kind`
in every callback. Still works: [Deprecated: the long form](../DEPRECATED.md).

## Tethers

A tether grab needs nothing from the engine: put your `CATCH` box on the
`throw` bone and let your catch animations carry it out. The rope is an
article (`lucas/himohebi`, `samus/gbeam`, ...).

The air tether and its ledge grab come from a donor:

```rust
Manifest::new("my_fighter", "mario")
    .tether("lucas")
    .tether_box([60.0, 48.0], [5.0, -34.0])
```

```toml
[tether]
from = "lucas"

[[tether.box]]
p1 = [60.0, 48.0]
p2 = [5.0, -34.0]
```

- `from` turns the air tether on and copies that fighter's `air_lasso_data`
  and ledge boxes: `samus`, `samusd`, `szerosuit`, `younglink`, `toonlink`,
  `lucas`, `luigi` (no ledge boxes). Your `[params]` win over the donor's.
- A box is two corners relative to your fighter. Yours replace the donor's.
- Ship the donor's `<donor>_vl.prc` in `fighter/<your name>/param/` for its
  articles' params. ParamConfig must be installed.

Port the donor's `AIR_LASSO` statuses; they call the common ones. Weapon
statuses: `SHOOT` 1, `EXHAUST` 2, `REWIND` 3, `HANG_REWIND` 6, `REACH` 7.
Cliff hang data 1 is the tether, 2 the hang.

| donor | articles | `status_AirLasso` | hang joint |
| --- | --- | --- | --- |
| `samus`, `samusd` | `gbeam` | `(gbeam, NONE, 1, true)` | `gbeam27` |
| `younglink`, `toonlink` | `hookshot`, `hookshothand` | `(hookshot, hookshothand, 1, true)` | `hookshot22` |
| `szerosuit` | `whip2` | `(whip2, NONE, 1, true)` | `plasmawhip9` |
| `lucas` | `himohebi`, `himohebi2` | `(himohebi2, NONE, 1, true)` | `joint8` |
| `luigi` | `obakyumu`, `plunger` | `(NONE, NONE, 0, true)` | no hang |

| status | main |
| --- | --- |
| `AIR_LASSO_REACH` | `status_AirLassoReach(7)` |
| `AIR_LASSO_HANG` | `status_AirLassoHang(hash40(joint), 2, hash40("rot"))` |
| `AIR_LASSO_REWIND` | `status_AirLassoRewind(6)` |
| `AIR_LASSO_FAILURE` | `status_AirLassoFailure(f, hash40("air_catch"), 3)`, `f` calls `sub_air_lasso_failure_uniq(3)` |
| `AIR_LASSO_LANDING` | `status_AirLassoLanding(2)` |

- Port the donor's `aircatch` scripts, which spawn, shoot, rewind and
  remove the articles, and the rest of its status code: Samus and Lucas
  set `AIR_LASSO_REACH_WORK_INT_ARTICLE_STATUS` in the reach end, the Links
  respawn the pair in the reach, Zero Suit lays material motions over the
  whip, Lucas flips when hanging facing left, Luigi lands his own way.
- `FIGHTER_STATUS_AIR_LASSO_FLAG_CHECK` is the ledge search window: frames
  0 to 5, frame 7 only for Lucas.
- Your motion list needs `air_catch`, `air_catch_pose`, `air_catch_hang`,
  `air_catch_hit` and `air_catch_landing`. Stretch animations shorter than
  the donor's with `FT_MOTION_RATE` (game frames per motion frame) and scale
  `air_lasso_data.shoot_end_frame` the same way.
- Lucas's rope aims by `air_catch_pose`: the frame shown is the angle in
  degrees from straight up between your `ShoulderR` and the ledge, and the
  rope leaves `HaveR` the way that frame points. It hangs only within
  `hang_dist` (6) of the ledge. Copy his `g04aircatchpose` arm onto your
  right arm.
- `custom_fighters/template_v2/src/tether.rs` has every status, blank, with
  Samus's calls in comments; the tether is commented out in its `lib.rs`.
  The Workbench's Tether box writes `[tether]` and the same blank statuses
  with the chosen donor's calls.

## Glide

Brawl's glide. The game still runs the five glide statuses (`GLIDE_START`,
`GLIDE`, `GLIDE_ATTACK`, `GLIDE_END`, `GLIDE_LANDING`);
`dionysus_api::glide` adds the way in and the flight.

```rust
use dionysus_api::glide::{self, Glide};

const GLIDE: Glide = Glide::META_KNIGHT;

unsafe extern "C" fn glide_entry(fighter: &mut L2CFighterCommon) {
    glide::frame(fighter, &GLIDE);
}
unsafe extern "C" fn glide_init(fighter: &mut L2CFighterCommon) -> L2CValue {
    glide::glide_init(fighter, &GLIDE)
}
unsafe extern "C" fn glide_exec(fighter: &mut L2CFighterCommon) -> L2CValue {
    glide::glide_exec(fighter, &GLIDE)
}
// glide_start_init / glide_start_exec the same way

if glide::enabled() {
    agent.on_line(Main, glide_entry);
    agent.status(Init, *FIGHTER_STATUS_KIND_GLIDE_START, glide_start_init);
    agent.status(Exec, *FIGHTER_STATUS_KIND_GLIDE_START, glide_start_exec);
    agent.status(Init, *FIGHTER_STATUS_KIND_GLIDE, glide_init);
    agent.status(Exec, *FIGHTER_STATUS_KIND_GLIDE, glide_exec);
}
```

- Players with another glide mod set `gliding = false` in
  `sd:/ultimate/dionysus/settings.toml`, which the engine writes on its
  first boot (`gliding = true`). `glide::enabled()` is then false, so install
  nothing glide-related; `glide::frame` does nothing either way.

- Hold jump through the top of your midair jump. Forward or down dives and
  gains speed, back or up climbs and loses it; out of speed the glide ends.
  Jump or special ends it, attack is the glide attack. One glide per airtime.
- `Glide::PIT`, `Glide::META_KNIGHT` and `Glide::CHARIZARD` are their Brawl
  values; `Glide::from_brawl([f32; 20])` takes any Brawl glide block.
  `.button_frame(n)` is how long jump must be held (8).
- Motions: `glide_start`, `glide`, `glide_attack`, `glide_end`,
  `glide_landing`, scripted like any other. A `glide` of 180 frames or more
  plays as Brawl's pose sweep (frame 90 minus the glide angle).
- The glide and the float start from the same input, so a fighter takes one.
  `custom_fighters/template_v2/src/glide.rs` is the glide, installed in place
  of the float. The Workbench's Glide box writes this code.

## Float

Peach's float on any base, with her rules and values.

```rust
use dionysus_api::float::{self, Float};

let start = MY_FIGHTER.own_status(0); // the first status number past the base's own
float::register("my_fighter", Float::PEACH, start);
let mut agent = Agent::new("my_fighter");
agent.on_start(float::on_start);
agent.on_line(Main, float::frame);
agent.status(Pre, start, float::start_pre);
agent.status(Init, start, float::start_init);
agent.status(Main, start, float::start_main);
agent.status(End, start, float::end);
agent.status(Pre, start + 1, float::float_pre);
agent.status(Main, start + 1, float::float_main);
agent.status(End, start + 1, float::end);
agent.status(Pre, *FIGHTER_STATUS_KIND_ATTACK_AIR, float::attack_air_pre);
agent.status(Main, *FIGHTER_STATUS_KIND_ATTACK_AIR, float::attack_air_main);
agent.install();
```

- Hold jump past the top of a jump, or hold down and jump on the ground.
  Letting go falls. Aerials keep the float going. Landing gives it back; a
  hit or a ledge does not.
- The float takes your first two own statuses, so yours start at
  `MY_FIGHTER.own_status(2)`. Call `own_status` after `register`.
- `Float::PEACH` takes `.frames(n)` (150), `.start_y(y)` (ground float
  height, 1.5), `.water_height(h)` (14), `.motions(start, hover)`
  (`fuwafuwa_start`, `fuwafuwa`; `fall` without them) and `.item(kind)` (the
  held item down special throws, like her turnip).
- An aerial status of your own calls `float::attack_air_pre` and
  `float::attack_air_main`.
- The ground float looks for the ground 6 below the first jump frame. A base
  whose first jump frame rises higher floats where the jump leaves it.
- The Workbench's Float box writes this code;
  `custom_fighters/template_v2/src/float.rs` has it on Mario.

## Command inputs

The command inputs Ryu, Ken, Terry and Kazuya read, on any fighter, and your
own.

```rust
Manifest::new("my_fighter", "mario")
    .vanilla_commands(true)
    .special_command_life_max(12)
    .command(Command::new("geyser", "21416").button(Button::Special).super_command().reverse().cpu(0.3))
```

```toml
[commands]
vanilla = true                            # the game's own commands
special_command_life_max = 12             # frames between two directions (10)
super_special_command_life_max = 24       # the same for super commands (20)
special_command_ab_button_life = 4        # frames a button press counts (3)
special_command_neutral_threshold = 0.25  # stick distance that reads as neutral (0.2)
special_command_stick_range = [[-20.0, 20.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0], [20.0, 70.0], [70.0, 110.0], [110.0, 160.0]]

[[command]]
name = "geyser"
input = "21416"        # numpad, read for the way you face
button = "special"     # special, attack, either (the default) or none
super = true           # waits super_special_command_life_max per direction
reverse = true         # also read mirrored, like Terry's _R commands
cpu = 0.3              # a CPU's special press reads it 30% of the time
```

```rust
use dionysus_api::fight;

let cat4 = ControlModule::get_command_flag_cat(fighter.module_accessor, 3);
if cat4 & *FIGHTER_PAD_CMD_CAT4_FLAG_SPECIAL_N_COMMAND != 0 { /* 236 + A or B */ }

if let Some(lr) = fight::command_lr(fighter, "geyser") {
    fight::clear_command(fighter, "geyser");
    // lr is the way it was input for
}
```

- `vanilla = true` fills the game's 27 `FIGHTER_PAD_CMD_CAT4_FLAG_*`
  commands on any base, with `get_special_command_lr` and
  `clear_command_one` as for Ryu.
- The tuning keys are the game's `common.prc` names and apply to the game's
  commands and yours. `special_command_stick_range` is Kazuya's list: seven
  ranges in degrees from straight up (up, up-forward, forward, down-forward,
  then the same three backward); past the last the stick reads as down.
- Each direction must come within the life of the one before, and the
  button from a few frames before the last direction until the life runs
  out. Hitstop pauses the count. A diagonal between two directions may be
  skipped (236 also reads as 26); `5` asks for neutral.
- `[[command]]` also takes `life = n` and `neutral_resets = false` (going
  back to neutral midway no longer cancels it).
- `fight::command` is true for 8 frames after the input, `command_lr` gives
  the way it was input for, `clear_command` uses it up. Up to 32 commands.
- `lr` is the way the fighter faced at the first direction. A back input on
  the ground turns it around, so it may face the other way by the end.
- CPUs enter the game's commands only on a Ryu, Ken or Terry
  base. `cpu` gives them your command when they press its button (not with
  `button = "none"`); a press reads at most one command, in order.
- On an older Dionysus, `register` leaves the command tables out and logs it;
  a `fighter.toml` with them needs a release that has them.
- `custom_fighters/template_v2/src/fight.rs` throws the fireball on
  41236 + special.

### Motions and facing in code

```rust
use dionysus_api::fight::{self, Motion};

unsafe extern "C" fn every_frame(fighter: &mut L2CFighterCommon) {
    fight::track(fighter);
    fight::face_opponent(fighter);
}

// in a special's status, on its first frame:
if fight::motion(fighter, Motion::QuarterForward) {
    // 236 + B: the stronger version
}
```

- `track` reads the stick every frame. `motion` asks for one motion,
  `recognise` returns the first match, `forget` clears the history (do it
  after using a motion).
- `QuarterForward` 236, `QuarterBack` 214, `DragonForward` 623,
  `DragonBack` 421, `HalfForward` 41236, `HalfBack` 63214, `DownDown` 22. A
  motion counts when it ended in the last 4 frames and took at most 15
  frames (24 for half circles, 16 for 22).
- `face_opponent` turns you toward the opponent in a 1-on-1 on the ground:
  in wait, crouch, landing and shield, and on the first frame of jabs,
  tilts, specials and grabs. `opponent` and `opponent_lr` return the
  opponent.

## Partner

A partner that follows you the way Nana follows Popo.

```rust
Manifest::new("my_fighter", "mario").partner(true)             // a copy of your fighter
Manifest::new("my_fighter", "mario").partner_with("my_other")  // another of your fighters
```

```toml
partner = true            # a copy of your fighter
partner = "my_other"      # another of your fighters
partner = false           # none: the default, except on base popo
```

- An Ice Climbers clone has Nana in your files unless `partner = false`,
  paired moves included.
- The partner repeats your moves `nana_delay` frames later and catches up
  when it falls behind. Its KO keeps your stock and it returns with your
  next one; your KO takes it along.
- A copy has your files, `[params]` and scripts. On base popo Nana is faster
  so she can catch up: a `mul` keeps that gap, a `set` gives her your value.
- A named partner is its own fighter, on any base. On your base, Kirby
  copies you whichever of the two it swallows.
- Your portrait plays you with the partner following; the named partner's
  portrait plays it with you following, under your `[partner_params]`.
  `partner = false` on the partner plays it alone from its portrait.
- A fighter has a partner or a switch group, not both.

```rust
use dionysus_api::partner;

let boma = fighter.module_accessor;
if partner::is_partner(boma) { /* only the partner */ }
if partner::distance(boma).is_some_and(|far| far > 60.0) { /* separated */ }
```

- `is_partner`, `has_partner`, `of` (the other one's module accessor) and
  `distance`. `has_partner` and `distance` count the partner only while it
  is in play; `of` gives it either way. `MY_FIGHTER.is(fighter)` is true for
  both.
- Other rules are yours to script, for example a partner KO that costs a
  stock.

### Swapping who you play

`partner::swap(boma)`, called from either one, hands your controller to the
partner; the one you played follows.

- The one you play leads: its KO costs the stock, the P1 tag is over it, and
  the other one `is_partner`. A KO keeps the swap. A new match, a Training
  reset and Stage Builder's Reset Stage or Change Fighter bring back the one
  you picked.
- False while the partner is KO'd or waiting, within 0.3 seconds of the last
  swap, and for an Ice Climbers clone.
- Check `runtime_capabilities() & CAP_PARTNER_SWAP` for older engines.

### How the partner follows

`[partner_params]` gives your pair its own Nana values; other Ice Climbers in
the match keep theirs.

```rust
Manifest::new("my_fighter", "mario")
    .partner(true)
    .partner_param_int("nana_delay", 12)
    .partner_param("nana_opt_dst", 30.0)
```

```toml
[partner_params]
nana_delay = 12
nana_opt_dst = 30.0
jostle_data_nana.speed_z = 0.5
```

- Keys: Popo's `param_private` values (`nana_delay`, `nana_opt_dst`,
  `nana_pos_x_offset`, ...) and the `jostle_data_nana` and
  `jostle_data_nana_small` tables. An unknown key refuses the whole file.
- Off base popo only `nana_delay` (frames behind you, 15 at most) and
  `nana_opt_dst` (the distance it keeps) change anything.

## Switching

Two or three of your fighters share one player slot and switch, like Pyra
and Mythra. Each is its own fighter with its own files and scripts; one
lists the others:

```rust
Manifest::new("my_fighter", "mario").switch_with(&["my_other_fighter", "my_third_fighter"])
```

```toml
switch = ["my_other_fighter", "my_third_fighter"]   # on bases pikachu and luigi
```

- One fighter per base, any base but the Ice Climbers. Pyra and Mythra
  (`eflame`, `elight`) and the Trainer's three (`pzenigame`, `pfushigisou`,
  `plizardon`) switch only within their own set.
- Each keeps its portrait. Picking one starts as it and the others follow in
  list order (`my_other_fighter`, then `my_third_fighter`, then
  `my_fighter`). A KO costs one stock; a KO'd fighter comes back as itself.
- On the Pyra and Mythra or Trainer set, down special switches, as does a
  change to `FIGHTER_STATUS_KIND_SPECIAL_LW` from another move.
- On other bases call `dionysus_api::switch::next(boma)` from a move of
  your choice. The next fighter takes over where the last stood, facing the
  same way, in wait. False outside a group or mid-switch.
- A fighter on a Pyra, Mythra or Trainer base without `switch` plays alone:
  down special does nothing, and Charizard keeps his tail fire.

## Item hand

`item_hand = "left"` or `"right"` (`Manifest::item_hand`) picks the hand your
fighter holds items in. The base keeps its own, in the same match too.

- Held items, thrown items and the hitboxes of swung items (the
  `commonhave` joint) move to `HaveL` or `HaveR`.
- A base whose hold data names both hands (Link, Toon Link, Hero, Byleth,
  Steve) has the two swapped.
- A base that holds items in its mouth (Pikachu, Pichu, Squirtle, Mewtwo,
  Duck Hunt) holds them in the chosen hand.
- The base's animations still close its own hand around the grip. Animate
  the other hand for a clean grip.
- Every base but Ivysaur has both bones; on `pfushigisou` the key is
  refused. A skeleton of your own needs the bone.
- The Workbench's Item hand box (Identity tab, fighter) shows the base's hand
  and each bone that changes.

## CPU AI

`cpu_ai = "ryu"` (`Manifest::cpu_ai`) makes a CPU fight with another
fighter's AI instead of the base's. Any fighter with CPU AI works; see
[PARAMETERS.md](PARAMETERS.md#your-own-parameter-files) for what it brings
along.

## Voice

Any fighter can have a voice, even on a voiceless base (Pac-Man, Villager,
Isabelle, Mega Man, Piranha Plant, Steve).

- Ship `sound/bank/fighter_voice/vc_<your name>_cNN.nus3audio`, `.nus3bank`
  and `.tonelabel`. The file name picks the bank, whatever name is inside.
  The Workbench puts them in each costume's group and shares c00 with
  costumes that lack one.
- Play the labels from your sound scripts (`PLAY_SE`, `PLAY_SEQUENCE`,
  `PLAY_FLY_VOICE`). Moves kept from a voiceless base stay silent until you
  add the calls.
- A fighter plays only its own banks, even with another fighter in the
  match.
- Kirby's copy has its own bank: [Copy sounds](ARTICLES_AND_KIRBY.md#copy-sounds).

## Move list pages

The move list shows extra pages below a fighter's specials; the game stops
at four. Up to 32:

```toml
[move_list]
pages = 6              # extra pages below the specials
image_pages = [1, 4]   # the ones that show a picture, counted from 0
```

```rust
Manifest::new("my_fighter", "captain").move_list(6).move_list_images(&[1, 4])
```

- A page's title and text are `nam_peculiar_NN_<your name>` and
  `nam_peculiar_help_NN_<your name>` in `ui/message/msg_name.xmsbt`, NN from
  `00`.
- A picture page shows
  `ui/replace/skill/skill_1/skill_1_<your name>_extra_page_<N>.bntx`, N from
  `0`. Without `image_pages`, the first three pages take your base's choice
  and the rest are text.
- Players turn pages with up and down. Past the fourth page the four panels
  refill as they turn.
- A vanilla fighter with `ext_skill_page_num` above 4 in `ui_chara_db` pages
  the same way.
- On an older Dionysus, `register` leaves `[move_list]` out and logs it; a
  `fighter.toml` with it needs a release that has it.

## Final Smash backgrounds and movies

Any fighter can ship its own; your Final Smash statuses start them.

| Ship | At | Start it with |
|---|---|---|
| A background scene | `finalsmash/<your name>/shared/`, laid out like a vanilla one (`model/<set>/`, `render/`, `lut/`, `motion/`) | `FighterManager::start_finalbg(manager, kind, 1.0)`, then `exit_finalbg` |
| Movies | `prebuilt:/movie/fighter/<your name>/cNN/<file>.h264`; the pack folder is `prebuilt;` | `FighterManager::prepare_movie(manager, Hash40::new(path))`, then `start_movie` once `is_prepared_movie`, `exit_movie` once `is_end_movie` |

```rust
let manager = dionysus_api::fighter_manager();
let kind = MY_FIGHTER.kind().get().unwrap();
FighterManager::start_finalbg(manager, kind, 1.0);
FighterManager::prepare_movie(manager, Hash40::new("prebuilt:/movie/fighter/my_fighter/c00/final_00.h264"));
```

- `kind` is your fighter's kind or your base's.
- The Workbench declares both in `config.json`.
- Without a background of your own, your base's is used. If one comes up
  black, the `[fsload]` log line says what was found.

## Extra files per costume (`append`)

Files under `fighter/<your name>/append/` load on the loading screen and stay
for the match, on any base: `append/cNN/...` with costume NN,
`append/<file>` with every costume.

- The Workbench declares one `append/cNN` group per costume and shares a
  c00-only file with the others.
- A Joker clone also gets Joker's append group (Mona and the Final Smash
  cut-in).
