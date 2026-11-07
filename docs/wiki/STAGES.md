# Stages

Put a `stage.toml` beside the pack's `config.json`. No code is needed. Start
from [custom_stages/template](../../custom_stages/template/).

## stage.toml

```toml
place = "template_stage"          # lowercase folder name under stage/
display_name = "Template Stage"   # name on the stage select screen
id_name = "Template_Stage"        # the message and UI id; lowercased it must equal place
forms = ["normal", "omega", "battlefield"] # which forms exist
ships_battle_tree = false         # true only once stage/<place>/battle exists
series = "mario"                  # series icon
disp_order = 121                  # position on the stage select screen
donor = "battlefield"             # the vanilla stage whose code and gimmicks run
bgm = "smashbtl"                  # My Music playlist
bgm_setting_no = 0                # its column, 0 to 15
```

| Key | What it does |
|---|---|
| `donor` | Your stage runs this vanilla stage's code (moving parts, hazards) |
| `resource_place` | Your stage uses this vanilla stage's whole file tree |

`content_donor`, `content_donor_tree` and `carry_donor_scenery` are Clone
Pack Workbench keys, ignored by the engine. Always set `content_donor_tree`
with `content_donor`.

## Past 121 stages

The game has 121 stage buttons. Dionysus adds more, up to 255, for any stage
on the screen, whichever mod added it. Players pick how with `stage_select_ui`
in `settings.toml` (see the README).

`disp_order` sets where a stage sits; stages sharing a value sit side by side.
In `stage.toml` it goes up to 255. In the CSK Collection's database JSON it stops at 127:
above that, the whole file is refused and every row in it is lost.
To place a row higher, keep the JSON at 127 or lower
and put a `stage_order.toml` in the pack's folder:

```toml
ui_stage_my_stage = 130   # its ui_stage_id = its position, 0 to 255
```

A plugin can do the same with `stage::set_disp_order("ui_stage_my_stage", 130)`.

## Several stages in one pack

```toml
[[stage]]
place = "sector_z"
id_name = "Sector_Z"
donor = "fox_corneria"

[[stage]]
place = "badtime"
id_name = "Badtime"
donor = "pictochat2"
```

Each stage keeps its own `stage/<place>` folder and sound files, and the
pack's one `config.json` lists all of them. Engines up to `0.2.1-beta.1` read
only the single-stage form and load nothing from a file with blocks.

## Music

```toml
bgm = "demon"           # a playlist, by series name or full label ("bgmdemon" is the same)
bgm_setting_no = 0      # which column of the playlist starts, 0 to 15
bgm_selector = false    # true adds the album selector
```

Most playlists only fill column 0; the boot log warns when the column is
empty. Left out, the stage uses its `donor`'s playlist; with no `donor`
either, My Music is empty.

For a track list of your own, publish a playlist with `playlist_entries` in
the CSK Collection's JSON under `sd:/ultimate/mods/<pack>/database/` and point
`bgm` at it. Do not put `stage_database_entries` in that JSON; the engine
writes the stage row itself.

## Forms

With `ships_battle_tree = false`, the normal folder supplies every form. Set
it true only once `stage/<place>/battle` exists. A missing folder crashes the
load; it does not fall back.

Each form can have its own collision, settings, models, camera, effects and
sounds, so test every form you enable.

## Moving platforms

A moving platform needs its collision in the `.lvd` file plus a matching
dynamic collision entry. The engine applies that entry only while your stage
is running, so other stages with the same `donor` are unaffected. Use the
exact collision and bone names from your own stage files.

Without `config_stage.toml`, list the model sets in `stage.toml`:

```toml
dynamic_collisions = ["platform_set"]
```

For each collision that follows a bone:

- subname: the model set's name, exactly
- bone name: the bone it follows
- points: in that bone's space

The set needs a motion that moves the bone,
`stage/<place>/normal/motion/<set>/<set>_00.nuanmb`; it plays by itself.
Bind a set here or in `config_stage.toml`, not both.

## Model set motions

A model set's motion (`motion/<set>/<set>_00.nuanmb`) plays from the start
of the battle and loops. List the sets that should wait instead:

```toml
paused_motions = ["gdor_01_set", "gmov_00_set"]
```

They hold their first frame until a plugin starts them:

```rust
use dionysus_api::stage;

stage::set_motion_frame("gdor_01_set", 0.0)?;
stage::set_motion_rate("gdor_01_set", 1.0)?;   // 0 holds it, 0.5 half speed
let now = stage::motion("gdor_01_set")?;        // frame and rate
```

These act on the stage being played. A collision bound to the set (see
Moving platforms) follows its motion. A set without a skeletal motion has
nothing to control (`Engine(-2)`).

## Ladders

```toml
[[ladder]]        # [[stage.ladder]] inside a [[stage]] block
x = 625           # the foot of the ladder
y = -50
height = 80
width = 10        # optional, 10 by default
```

Each table builds Ultimate's own ladder (the one 75m uses) on the stage's
normal form: fighters catch it, climb up and down, and step off the top with
their ladder animations. It draws nothing, so the ladder belongs in your stage
model. Ladder tables go after the stage's own keys; up to 64 per stage.

## Barrel cannons

```toml
[[barrel]]               # [[stage.barrel]] inside a [[stage]] block
x = 1165                 # centre
y = 190
model = "gbc_00_set"     # optional: a model set of this stage, drawn and turned with the barrel
rotation = 0             # degrees, counter-clockwise from facing right
rotation_max = 90        # the other end of the sweep (default: rotation)
rotation_speed = 1.08    # degrees per frame; 0 keeps it still
always_rotate = true     # turn even when empty (otherwise only while loaded)
full_rotate = false      # spin all the way round instead of sweeping
auto_fire = false        # fire by itself at fire_rotation (otherwise on the attack button)
fire_rotation = 0
fire_delay = 0           # frames between the fire and the shot
speed = 5                # shot: speed, decrease per frame, gravity, speed at which it ends
decrease = 0.075
gravity = 0.025
fall_speed = 0.25
width = 20               # entry area
height = 15
path = [990, 55, 990.75, 56.4]   # optional: x, y per frame; the shot follows it
path_speed = 2.5                 # path frames per game frame
```

Each table builds Ultimate's own barrel cannon (the one Kongo Jungle uses) on
the stage's normal form, with the turning and firing rules of Brawl's barrels.
Fighters drop in, wait, and are shot along the barrel's direction, or along
`path` when it is given. A barrel draws nothing unless `model` names one of
the stage's model sets (one set per barrel, since a set is drawn once); the
model faces up at rest. Up to 64 per stage.

## Springs

```toml
[[spring]]               # [[stage.spring]] inside a [[stage]] block
x = 155                  # where the spring stands
y = -30
model = "gspr_00_set"    # optional: a model set of this stage, drawn and tilted with the spring
rotation = 0             # degrees; negative leans right, positive leans left
speed = 4                # launch speed (default 4)
width = 10               # contact area, centred on x + offset_x, y + offset_y
height = 8
offset_x = 0
offset_y = -2
launch_frame = 3         # with model: the motion frame the launch comes at
joint = "SpringTop"      # with model: the joint the fighter is held on until then
```

Each table builds Ultimate's own spring (the one Windy Hill uses) on the
stage's normal form. A fighter that lands on it is launched at `speed` along
the spring's lean, and bounces again each time it lands back on it. A speed
of 4 lifts Mario about 95 units. A spring draws nothing unless `model` names
one of the stage's model sets (one set per spring). With `launch_frame`, a
landing plays the set's motion (`motion/<set>/<set>_00.nuanmb`), holds the
fighter on `joint` and launches at that frame; without it the launch is
immediate. Up to 64 per stage.

## Doors

```toml
[[door]]                 # [[stage.door]] inside a [[stage]] block
x = 1770                 # where the fighter walks to
y = 75
width = 15               # entry area, centred on x + offset_x, y + offset_y
height = 20
offset_x = 0
offset_y = 10
air = false              # true: an air door, entered while airborne
player = 1               # optional: only this player (1 to 8) may enter
```

A fighter standing (or, for an air door, flying) in the area with the stick
held up enters `FIGHTER_STATUS_KIND_GIMMICK_DOOR`, the status Brawl's
Subspace doors used: it walks to `x`, turns and walks into the door (an air
door draws it to `x`, `y` and fades it). Only statuses that allow it (wait,
walk, run and the like) can enter, and a Nana-style partner cannot.

A door ends nothing by itself: the fighter stays in the door status until a
plugin acts, for example by starting the next battle. A door draws nothing;
it belongs in the stage model. Up to 64 per stage.
