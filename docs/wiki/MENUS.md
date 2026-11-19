# Menus

Put a `menu.toml` beside the pack's `config.json`. It lets a main menu
button start a mode of the game's under a name of your own, adds layouts to
the main menu, loads extra files, and changes the game's menu scripts without
shipping them. No code is needed for any of it.

## menu.toml

```toml
[[sequence]]
id = "my_training"           # lower case letters, digits and _
runs_as = "other_training"   # the game's mode it starts, here Training

[[layout]]
scene = "main_menu"
file = "ui/layout/menu/my_menu/my_menu/layout.arc"

[[preload]]
after = "ui/script/menu/main/main_menu_layer_other.lc"
files = ["ui/script/menu/my_menu/my_page.lc"]

[[patch]]
module = "menu/main/main_menu_layer_other"
script = "menu/my_menu/my_other"
```

| Table | What it does |
|---|---|
| `[[sequence]]` | A main menu button pointed at it starts the mode `runs_as` (see Buttons) |
| `scene` in `[[sequence]]` | Go straight to this scene instead of the mode's first screen |
| `skip`, `after_battle` in `[[sequence]]` | Leave Smash's rules page or stage select out, end after a battle (see Choosing the screens of a mode) |
| `[[layout]]` | The main menu builds this layout too; Lua finds it in `LayoutRootList` |
| `[[preload]]` | Whenever the game requests `after`, it requests `files` as well |
| `[[patch]]` | Whenever the game loads `module`, it runs `script` instead (see Patching a menu script) |
| `[[mode]]` | A row on the modes page that starts your mode (see Modes) |
| `[[gallery]]`, `[[gallery_item]]` | A row on the modes page that opens a page of 3D models (see Galleries) |

## Buttons

A main menu button's Lua table carries `sequence`, the code of what it
starts. The engine gives each `[[sequence]]` a code; ask for it by id and
point a button at it from a patch (see Patching a menu script):

```lua
self.button_table[button.OTHER_VR].sequence = prg_func.engine_call("engine.sequence", "my_training")
```

- `engine.sequence` answers a `[[mode]]`'s id too, and 0 for an id no pack
  declared.
- Returning from the mode lands on the same button.
- The first pack to declare an id keeps it; another pack's `[[sequence]]`
  with that id is refused.
- A pack that needs a fixed code gives `code` instead of, or with, `id`: a
  number whose first byte is 0x08 to 0x7d.

## The game's modes

`runs_as` takes the names the game's menu scripts use for their buttons
(`SEQUENCE_OTHER_TRAINING` there), in any case, with or without `sequence_`:

| `runs_as` | Mode |
|---|---|
| `melee_normal` | Smash |
| `melee_team_compe` | Squad Strike |
| `melee_tournament` | Tournament |
| `melee_special_various`, `melee_special_suddendeath`, `melee_special_all_battle` | Custom Smash, Super Sudden Death, Smashdown |
| `spirits_challenge` | Spirit Board |
| `spirits_canpaign` | World of Light (the game's spelling) |
| `other_standard` | Classic Mode |
| `other_kumite_100`, `other_kumite_allstar`, `other_kumite_cruel` | Century, All-Star and Cruel Smash |
| `other_homerun_single`, `other_homerun_coop`, `other_homerun_everyone` | Home-Run Contest |
| `other_training` | Training |
| `other_chara_make` | Mii Fighters |
| `other_stage_make` | Stage Builder |
| `other_amiibo` | amiibo |
| `other_challenger` | Challenger's Approach |
| `other_vr` | VR Mode |

- The rest of the game's names (Vault, options, online) work the same way.
- Smash and Training are the tested ones; the others start what the game's
  own button starts.
- A number works too, as the game codes it (`0x04040000` is Training).

## Starting a battle straight away

```toml
[[sequence]]
id = "quick_battle"
runs_as = "other_training"
scene = "IngameCallSequenceScene"
```

`IngameCallSequenceScene` is the battle itself, with no select screens. It
uses the current player and stage selections, so set them before the button
is pressed, from the button's `func_decide`:

```lua
func_decide = function(layer)
  local pad = layer.virtual_input:get_last_input_device_id()
  prg_func.engine_call("engine.battle_player", 0, "human", "ui_chara_mario", pad)
  prg_func.engine_call("engine.battle_player", 1, "cpu", "ui_chara_luigi", -1)
  prg_func.engine_call("engine.battle_stage", "ui_stage_battle_field")
end
```

Fighters and stages are their `ui_chara_` and `ui_stage_` names, as in the
game's `ui_chara_db` and `ui_stage_db`. The engine hashes any text it is
given, so `0x0edaf3c863` (`hash40("ui_chara_mario")`) works as well.

| Call | Arguments |
|---|---|
| `engine.battle_player` | slot (0 = P1), `"human"`, `"cpu"`, `"amiibo"` or `"none"`, fighter `"ui_chara_..."`, controller |
| `engine.battle_stage` | `"ui_stage_..."` |
| `engine.battle_damage` | slot, starting damage in percent (0 to 999) |
| `engine.battle_color` | slot, costume number (0 = c00, up to 255) |
| `engine.battle_rules` | `"time"`, `"stock"` or `"stamina"`, stocks, time limit in seconds (0 = none), item frequency (0 = no items, up to 5) |
| `engine.battle_entrance` | 1 plays the entrance and the 3-2-1 countdown (default), 0 skips both for the next battle |
| `engine.battle_ready_go` | 1 shows and announces "Ready" and "GO!" (default), 0 starts the next battle without them |
| `engine.battle_pause` | 1 lets the players pause (default), 0 turns pausing off; written with the rules |
| `engine.battle_boss` | the boss: `"masterhand"`, `"crazyhand"`, `"galleom"`, `"ganonboss"`, `"dracula"`, `"dracula2"`, `"marx"`, `"lioleusboss"`, `"kiila"`, `"darz"`, or a boss family clone's name; -1 for none; the next battle only |
| `engine.sequence` | the id of a `[[sequence]]` or `[[mode]]`: its code, 0 for an id no pack declared |

Each returns 0, or a negative code when refused. `engine.battle_player`
also sets the player's starting damage and costume to 0; without it a player starts
with whatever the last battle that used the slot left there (the title
demo's battles included). Like the players, the rules are what the last battle left (the title
demo's included): set them. Setting them also turns pausing on, which a
battle started before the rules page was ever opened would otherwise lack;
`engine.battle_pause` 0 turns it off. A stock battle with a single player ends at once,
the game declares that player the winner; one player needs a time battle,
and with no time limit it lasts until it is ended. A human player with no
fighter, or a `random` stage, crashes the game. `runs_as = "melee_normal"` (Smash)
allows a single player; Training needs a CPU. Quit in the pause menu (after
"End the game?" Yes) ends the battle and returns to the main menu; vanilla
Smash would stop on "NO CONTEST". Quitting Training returns to the main menu.

`engine.battle_boss` makes the next battle a boss battle on its stage: the
game creates the boss, plays its intro, shows its HP bar and ends the battle
on its defeat (victory screen, the boss on the second card). A single player
is fine in a stock battle with a boss. A boss family clone runs on its base
boss's flow, so its intro camera, HP bar name and portrait are the base's.

`engine.battle_entrance` with 0 applies to the next battle only: the fighters
start in place, standing, and "GO!" starts the battle. While the battle is
still loading, a "Ready" gauge shows before "GO!". No fighter goes through
the entry status in such a battle.

`engine.battle_ready_go` with 0 also applies to the next battle only: no
countdown, "Ready" or "GO!" shows and the announcer stays silent. With the
entrance skipped too, the battle simply starts once it has loaded.

## Modes

A `[[mode]]` puts a mode of yours on the modes page. The engine's modes
button opens that page from any main menu page: X, unless `modes_button` in
`sd:/ultimate/dionysus/config.toml` names another (`x`, `zl`, `zr`,
`plus`, `minus`, `lstick`, `rstick`, or `none` for no button). The page itself
is the `Dionysus Modes` pack that comes with Dionysus; it draws on the
game's Mob Smash popup. Its three rows scroll through every installed mode
(L and R jump three), the selected mode's picture shows on the left and its
description in the footer. Every pack's modes get a row of their own and
share no button, code or menu state, so any number of packs can offer modes
side by side. The same pack adds pages of spirit boards to the DLC Spirits
popup (see [Spirits](SPIRITS.md#boards)).

```toml
[[mode]]
id = "my_mode"
name = "My Mode"
description = "One line about it."
script = "menu/my_mode/mode"
button = "ui/layout/menu/my_mode/my_mode_button/layout.arc"
runs_as = "melee_normal"
scene = "IngameCallSequenceScene"
order = 10
```

| Key | What it does |
|---|---|
| `id` | Lower case letters, digits and `_`, up to 32. The first pack to declare an id keeps it; the next one is refused |
| `name` | The row's name, up to 64 bytes |
| `description` | Shown in the footer while the mode is selected, up to 256 bytes |
| `script` | A Lua module returning a table with any of `decide`, `enter`, `update` and `leave` |
| `button` | A layout of your pack drawn where the popup shows its picture, 512 x 796 around its centre. The engine adds it to the main menu; name its main `.bflyt` after its folder. Without one the area shows the mode's name |
| `runs_as`, `scene` | As in `[[sequence]]`. The engine gives the mode a code of its own |
| `skip`, `after_battle` | As in `[[sequence]]` (see Choosing the screens of a mode) |
| `order` | Rows sort by it, then by pack folder name; default 0 |

A mode needs `script` or `runs_as`. Pages 0x7e and 0x7f hold the codes the
engine gives: a `[[sequence]]` `code` may not use them.

Choosing a row runs the mode one of two ways:

- When the script has `enter`, the mode runs as a screen inside the page:
  `enter(host, resumed)` once, `update(host, keys)` every frame, and
  `leave(host)` when it closes. Show your own `[[layout]]`s (with
  `over_side_panel = true`) in `enter` and hide them in `leave`; read input
  from `keys`, not from your layouts.
- Otherwise, with `runs_as`, `decide(host)` runs first if the script has
  it, to set up the battle (return `false` to stay on the page), and the
  mode starts.

When a mode started from the page ends, the main menu opens the page again:
on the mode's screen with `resumed` true, or on its row.

```lua
local mode = {}

function mode.enter(host, resumed)
  LayoutRootList.my_mode_page:get_root_view():get_pane("page"):set_visible(true)
end

function mode.update(host, keys)
  if keys.cancel then
    host:close()
  elseif keys.ok then
    host:leave_for("my_training")
  end
end

function mode.leave(host)
  LayoutRootList.my_mode_page:get_root_view():get_pane("page"):set_visible(false)
end

return mode
```

| Host | |
|---|---|
| `host.index`, `host.id`, `host.name` | The mode |
| `host.resumed` | True on the screen the page reopens after the mode's battle |
| `host:keys()` | This frame's keys: `up`, `down`, `left`, `right`, `ok`, `cancel`, `help`, `prev`, `next` |
| `host:pad()` | The controller that pressed last |
| `host:close()` | Back to the rows; `leave` runs first |
| `host:start()` | Starts the mode's `runs_as`; false when it has none |
| `host:leave_for(id)` | Starts a `[[sequence]]` or `[[mode]]` by id (or code) and comes back to the mode afterwards; false for an unknown id |
| `host:value()`, `host:set_value(n)` | A number the engine keeps for the mode until the game closes |

`modes/template` in the engine's source holds the Lua of `Example Modes`, a
pack with a battle mode and a screen mode; `modes/hub` holds the modes page's.
Both packs come built with the release.

The page uses main menu states 94 to 97 and asks the engine through these
calls:

| Call | Returns |
|---|---|
| `engine.mode_count` | How many modes the packs declare |
| `engine.mode_sequence` | Mode index: its code, or 0 |
| `engine.mode_pressed` | 1 once for each press of the modes button |
| `engine.mode_take_resume` | The index of the mode the menu comes back from, once; -1 |
| `engine.mode_return_to` | Mode index: the menu comes back to it after the next mode |
| `engine.mode_value`, `engine.mode_set_value` | Mode index (and the number): the mode's kept number |
| `engine.buttons` | The buttons held on any controller, as nn::hid `NpadButton` bits |

`prg_func.engine_text(name, index)` returns text: `engine.mode_id`,
`engine.mode_name`, `engine.mode_description`, `engine.mode_script` and
`engine.mode_button` (the button layout's root name), or nil.

## Choosing the screens of a mode

A `[[sequence]]` or `[[mode]]` that runs as Smash (`runs_as = "melee_normal"`,
no `scene`) can leave screens out and end after one battle:

```toml
[[mode]]
id = "my_pick"
name = "My Pick"
runs_as = "melee_normal"
skip = ["rules", "stage_select"]
after_battle = "menu"
script = "menu/my_mode/pick"
```

| Key | What it does |
|---|---|
| `skip` | Screens the mode goes without: `rules` (the rules page), `stage_select` |
| `after_battle` | `menu` ends the mode after each battle's results; the main menu comes back to the mode (its row, or the code's button). `vanilla` (the default) goes back to the stage select as Smash does |

Backing out of the first screen left ends the mode. A mode that skips the
rules page sets them with `engine.battle_rules`, and one that skips the stage
select sets the stage with `engine.battle_stage`, both in `decide` (or the
button's `func_decide`): the engine writes them again just before the battle,
after the fighter select. The fighter select still picks the players.

```lua
local pick = {}

function pick.decide(host)
  prg_func.engine_call("engine.battle_stage", "ui_stage_battle_field")
  prg_func.engine_call("engine.battle_rules", "stock", 2, 120, 0)
  return true
end

return pick
```

Only Smash takes `skip` and `after_battle`: with another `runs_as`, or with a
`scene`, the whole `menu.toml` is refused (`NoFlow` in the `[menuseq]` log).
The log tag is `[menuflow]`.

## Patching a menu script

A `[[patch]]` script runs in place of the module it names and returns what
that module should return. Inside it, requiring the same module gives the
module as it was before the patch: the game's file, another mod's
replacement, or the next pack's patch. Patches chain in pack folder order.

```lua
local other = UiScriptPlayer.require("menu/main/main_menu_layer_other")
local initialize = other.initialize

function other:initialize()
  initialize(self)
  self.button_table[button.OTHER_VR].sequence = prg_func.engine_call("engine.sequence", "my_training")
end

return other
```

Module names are the paths under `ui/script/` without `.lc`. A patch that
is missing or raises an error is skipped and the module loads as usual. The
log tag is `[menuscript]`.

Some modules live under `ui/script_patch/` instead (the stage select's, the
stage builder's) and load with `UiScriptPlayer.require2`. Name the root:

```toml
[[patch]]
module = "common/stage_select_actor3"
script = "menu/my_menu/my_stage_select"
root = "script_patch"
```

`root` is `script` (the default) or `script_patch`. The patch itself always
lives under your pack's `ui/script/`.

A patch can also replace a screen's own script, the one the screen starts
from rather than requires: the stage select (`common/stage_select_actor3`,
`script_patch`), the rule pages (`menu/melee/rule_itemswitch_actor`,
`menu/melee/rule_stageswitch_actor`), Special Smash
(`menu/melee/special_various`), Classic's screens (`standard/standard_*`),
options and more. Such a script defines globals the screen calls, most of
all `main`, which runs as a coroutine for the screen's whole life. Load the
game's script, then wrap what you need:

```lua
UiScriptPlayer.require2("common/stage_select_actor3")
local game_main = main

main = function(...)
  AppPopupManager.open_database("ID_POPUP_MY_PACK_HELLO")
  coroutine.yield()
  while AppPopupManager.is_busy() == true do
    coroutine.yield()
  end
  return game_main(...)
end
```

Use `require2` for a `script_patch` module and `require` for a `script` one.

## Lua

Main menu scripts are Lua 5.3 bytecode with 32-bit float numbers. Build
`luac` with `LUA_FLOAT_TYPE` set to `LUA_FLOAT_FLOAT` and compile with
`luac -s`.

- `UiScriptPlayer.require("menu/my_menu/my_page")` loads
  `ui/script/menu/my_menu/my_page.lc`, a new file included.
- `require` runs the file again for every script that requires it: each
  caller gets its own copy. Keep one shared copy in a global.
- An integer 0 counts as `nil`: `x ~= nil` is false and `if x then` fails
  when `x` is 0 (`0.0` and other numbers behave). Keep state in `true` /
  `false` flags, never in a number compared with `nil`.
- A `[[layout]]` root is named after its main `.bflyt`; a localized layout
  adds the region, as in `title+us_en`.
- A pack layout draws above the menu's popups. Park it at the end of its `out`
  animation when your page starts, as the vanilla popups do, or it shows
  from the first frame.
- `over_side_panel = true` in a `[[layout]]` draws it above the ZR side panel
  and the footer too, for a page that fills the screen.
- `under_movies = true` draws it above the ZR side panel but below movies, for
  a backdrop that covers the menu between pages and movies.
- A layout's `timg/__Combined.bntx` must start on a 4096-byte boundary inside
  `layout.arc`, or the main menu stops at load.
- `layout_view:play_animation(tag, 1.0)` replaces the animation running on
  the same group; animations on other groups keep playing. A missing tag
  does nothing.
- `prg_func.engine_call(name, ...)` takes up to 8 integer or boolean
  arguments and returns an integer, or `nil` when nothing answers `name`.
  `prg_func` exists only in the main menu; every menu script, the main
  menu's included, also has `UiScriptPlayer.engine_call` and
  `UiScriptPlayer.engine_text`, which do the same.

| Name | Returns |
|---|---|
| `engine.active_sequence` | The custom code that started the last mode, 0 after a vanilla one |
| `engine.log` | Nothing; writes its arguments to the log as `[menulog]` |

Read `engine.active_sequence` when your page starts to tell whether the
player is coming back from your own mode.

## Popups

`AppPopupManager.open_database("ID_POPUP_...")` opens one of the game's
popups by id, from any menu script; `AppPopupManager.is_busy()` is true while
it shows and `AppPopupManager.get_result()` tells which button closed it. Wait
for it from a coroutine, as the vanilla scripts do (see the example above).

A pack adds its own popup as a row of `ui/param/database/ui_popup_db.prc`
(`ui/param/database/ui_popup_db.prcxml`) and its text in
`ui/message/msg_popup.xmsbt`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<struct>
  <list hash="db_root">
    <struct index="425">
      <hash40 hash="style">button_1_text</hash40>
      <hash40 hash="se_kind">default</hash40>
      <bool hash="use_blur">true</bool>
      <bool hash="is_patch">false</bool>
      <string hash="btn1_text">pop_btn_ok</string>
      <string hash="btn3_text"></string>
      <string hash="body_text">pop_my_pack_hello</string>
      <string hash="btn2_text"></string>
      <hash40 hash="image_style">none</hash40>
      <hash40 hash="ui_popup_id">id_popup_my_pack_hello</hash40>
      <hash40 hash="replace_kind"></hash40>
      <hash40 hash="post_process">none</hash40>
      <string hash="replace_name"></string>
      <hash40 hash="replace_ui_id"></hash40>
      <int hash="mini_offset_y">0</int>
      <hash40 hash="mini_bg_color">default</hash40>
      <int hash="time_until_end">0</int>
      <hash40 hash="underlay_color">default</hash40>
      <hash40 hash="default_button">none</hash40>
      <hash40 hash="cancel_behavior">button_1</hash40>
      <int hash="input_wait_frame">0</int>
    </struct>
  </list>
</struct>
```

The id you pass is `ui_popup_id` in capitals. `style` is `button_1_text`,
`button_2_text`, `text_only`, `text_only_mini`, `button_1_mini` or
`button_2_mini`; `btn1_text` and `btn2_text` are `msg_popup` labels
(`pop_btn_ok`, `pop_btn_yes`, `pop_btn_no`, `pop_btn_close` are the game's).
Booleans are `true` and `false` in lower case: anything else makes ARCropolis
reject the whole file (`failed to parse prc patch` in its log).

The game has 425 rows (0 to 424). `index` is a position in that list, so two
packs that add a row at the same index replace each other: pick an index no
other pack you ship with uses.

## Movies

`engine.movie_play(path, track)` plays a WebM full screen above every pack
layout, with its audio track `track` (default 0), the way Help plays How to
Play. `path` is a `stream:/movie/...` path of up to 62 bytes; a pack adds
movies as `stream;/movie/<folder>/<name>.webm` (VP9 and Vorbis, as
Ultimate's own).

| Name | Returns |
|---|---|
| `engine.movie_play` | 0, or a negative code: -1 movies are off, -2 bad path, -3 a movie is playing, -4 no memory |
| `engine.movie_state` | 1 while the movie loads or plays, 0 once it has ended |
| `engine.movie_stop` | Ends the movie early; then poll `engine.movie_state` until 0 |
| `engine.movie_keep` | Holds the movie's frame on screen until the next battle has run two frames |
| `engine.movie_hold` | The movie stops on its last frame instead of ending; `engine.movie_stop` ends it |
| `engine.movie_status` | 0 no movie, 1 loading, 2 on screen, 3 ended and holding its last frame, 4 paused |
| `engine.movie_pause` | Stops the picture and the sound where they are; `engine.movie_resume` goes on from there |
| `engine.loading_icon_only` | The next loading screen shows only its corner icon, no tips; 0, or 1 when unavailable |

Call `engine.movie_state` every frame until it returns 0; the movie's screen
is freed then. `engine.movie_play` over a held or kept movie, or one being
stopped, ends that one and loads the new one after it; over a movie still
playing it returns -3. Ultimate runs one movie at a time: a second movie
screen never decodes while the first exists, even stopped. The menu music keeps playing under a movie: stop it with
`UiSoundManager:stop_bgm()` and bring it back with
`UiSoundManager:play_menu_bgm()`.

## Galleries

A gallery is a page of 3D models, each with a name and a text. It gets a row
on the modes page and needs no script.

```toml
[[gallery]]
id = "my_gallery"
name = "My Gallery"
description = "One line for the modes page."

[[gallery_item]]
gallery = "my_gallery"
fighter = "my_fighter"
costume = 0
name = "First Model"
text = "Shown under the model."
```

| Key | What it does |
|---|---|
| `id`, `name`, `description`, `order` | As in `[[mode]]` |
| `music` | Optional. A song id (`ui_bgm_...`, lower case) played while the page is open, from a music database (CSK `database/*.json`) |
| `gallery` | The gallery's `id`; items may come from another pack |
| `fighter` | A fighter or clone by name, with or without `ui_chara_` |
| `costume` | 0 to 255, default 0 |
| `name` | Up to 64 bytes |
| `text` | Up to 1024 bytes; lines wrap, `\n` starts a new one |
| `group` | Optional, up to 64 bytes. Consecutive items with the same group form a group in a grid skin |
| `sources` | Optional, up to 256 bytes, one line per source. A line may end in a vertical bar and a tag, which picks an icon in a grid skin |

Items are listed in the order declared. Each is a fighter's costume, so a clone
holds up to 256 models; for a model that stays still, bind every vertex to
`Trans`. The Workbench's Identity tab edits both tables in a menu part.

### Backdrop

The gallery runs on the Mii Fighters stage. While one is open the engine draws
no floor, turns off the sun shaft and serves the sky from the pack; every
other screen gets the game's own files. A pack's `gallery_backdrop/stc_skydome_set/`
and `gallery_backdrop/stc_ring_set/` (the floor) hold files named as the
stage's own (`model.numshb`, `.numatb`, `.numdlb`, `.numshexb`, `.nusrcmdlb`,
`.nusktb`, and the sky's `settingstage_white_con.nutexb`) and replace them
meanwhile. A set must be a complete model: a model list with no entries
crashes the loader. A sky can be a far quad with an unlit `far` material and
one picture (the camera looks down -Z from about 190 to 550 units, 0.17
radians tall); keep the texture name.

### Skins

A second `[[layout]] scene = "gallery"` with the panes `skin_page` and
`skin_view` is a grid skin: the page hides its own panels and only sets text
and visibility on the skin's panes. The layout loader answers `get_pane` for
any name, so a skin must have every pane below, at fixed sizes: 12 cells in 3
columns, 9 description lines, 2 source rows. Keep the names as listed: one
that begins with another's name gets the wrong pane.

| Pane | Use |
|---|---|
| `skin_grid`, `skin_view` | The grid, and the view of one item |
| `skin_cell_cursor_<k>` | k 0 to 11; shown on the selected cell |
| `skin_cell_{label,small,tiny}_<k>` | The cell's item name: up to 14 bytes in label, 19 in small, more in tiny; the others are emptied |
| `skin_thumb_<i>` | One picture per item, i from 1, at the cell its item takes (position in its group, modulo 12) |
| `skin_grid_group`, `skin_grid_foot`, `skin_grid_page`, `skin_grid_total` | The group's name twice, `(page/pages)` of the group, `(items)` |
| `skin_info`, `skin_angle`, `skin_help` | In `skin_view`: shown outside angle mode, in angle mode, in angle mode with help on |
| `skin_help_text_<k>` | k 0 to 6: Rotate, Slide, Zoom in, Zoom out, Switch view, Back, Help: ON/OFF |
| `skin_name`, `skin_mid_name`, `skin_tiny_name` | The item's name: up to 11 bytes in the first, 18 in the second, more in the third |
| `skin_desc`, `skin_desc_page` | One page of `text`, and `(page/pages)` |
| `skin_source_<k>`, `skin_source_text_<k>`, `skin_source_icon_<k>_<tag>` | k 0 and 1: shown when the item has that source line; the line without its tag; shown when its tag is `<tag>` |

Items are grouped by `group` in the order declared; a group fills pages of 12.

## Model viewer

A mode can show a fighter in 3D inside a menu, with no battle behind it; the
gallery is built on this. Give the `[[mode]]` `runs_as = "other_chara_make"`
and patch `menu/chara_make/make_mii_select_actor` with a script like this (see
Patching a menu script). The patch turns the Mii Fighters screen into the
viewer only when the mode opened it; the real Mii Fighters menu is unchanged.

```lua
UiScriptPlayer.require("menu/chara_make/make_mii_select_actor")

local call = UiScriptPlayer.engine_call
local code = call("engine.sequence", "my_viewer")

if code ~= 0 and call("engine.active_sequence") == code then
  is_finished_in = function() return 1 end
  get_exit_value = function() return -1 end
  main = function()
    ComFrameActor.hide_frame(true)
    call("engine.viewer_begin", 0)
    UiFighterManager.reload_fighter(FIGHTER_ENTRY_ID_SELF, "ui_chara_mario", 0)
    repeat
      coroutine.yield()
    until call("engine.buttons") & 2 ~= 0
    call("engine.viewer_end")
    UiFighterManager.deactivate(FIGHTER_ENTRY_ID_SELF)
    ComFrameActor.hide_frame(false)
  end
end
```

| Call | What it does |
|---|---|
| `engine.viewer_begin` | Readies slot 0 or 1 and hides the side panel. Returns 0, or -1 on a screen with no viewer |
| `engine.viewer_state` | 0 loading, 1 built, 2 shown, -1 no viewer |
| `engine.viewer_end` | Brings the side panel back |
| `UiFighterManager.reload_fighter(entry, "ui_chara_name", costume)` | Loads a fighter into the slot |
| `UiFighterManager.activate(entry)` | Shows it |
| `UiFighterManager.set_fighter_rotation_by_degree(entry, x, y, z)` | Turns it |
| `UiFighterManager.set_camera_pos(x, y, z, look_x, look_y, look_z, fov)` | Places the camera. A fighter fills the screen at (0, 8, 136.7) looking at (0, 8, 0), fov 0.17 |

- The first fighter a slot loads must be a vanilla one: a clone crashes the
  game there. Load a vanilla fighter, wait for state 1, leave it hidden, then
  load the one to show.
- Call `activate` once per load, only at state 1: during a load it crashes the
  game.

## A page of your own

A page is three main menu states (in, main, out) set up from a patch with
`state:setup(id, page, page.update_state_in, "NAME")`, opened by a button
whose table entry has `state = <in id>`, and closed with `state:pop()`.

- Use state ids between 40 and 93; 94 to 98 belong to the modes page. Above
  `STATE_TERM` (99) the main menu ends. A mode on the modes page needs no state
  of its own.
- Each state change turns input off for every layout the menu does not know.
  Call `layout_root:set_enable_input(true)` on the first frame of your main
  state, not in the in state.
- The main menu's virtual input ids are fixed numbers, not `INPUT_*` names:
  0 up, 1 down, 2 left, 3 right, 4 A, 5 B, 6 Y (help), 7 and 8 the
  previous and next keys (`INPUT_PREV`, `INPUT_NEXT`).
- Read each id once per frame, presses before repeats: asking for one
  direction's repeat clears a press of the other.
- `get_pane(name):set_text_string(text)` sets any text box. Text from the
  layout file itself does not show; set it from the script.
- The footer is the global `footer`: `footer.in_pane:set_text_string(text)`,
  then `footer.prev_label = nil` and `footer:change()`.
- `prg_func.set_resume_button_id(id)` with your button's id in the low bytes
  and your own row above them brings the player back to that row after a
  mode, through the same resume path the game uses.

## From a plugin

The menu template (`custom_menus/template`) is a complete pack with a plugin:
every call below is in it, except the viewer and gallery ones.

```rust
use dionysus_api::menu;

unsafe extern "C" fn level_count(_args: *const i64, _count: usize) -> i64 {
    40
}

menu::register_sequence(0x0f01_0000, 0x0404_0000)?;
menu::register_call("my_mod.level_count", level_count)?;
let last = menu::active_sequence();

menu::battle_player(0, menu::PlayerType::Human, "ui_chara_mario", 0)?;
menu::battle_stage("ui_stage_battle_field")?;
menu::battle_damage(0, 42)?;
menu::battle_color(0, 1)?;
menu::battle_rules(menu::Rules { kind: menu::RuleKind::Time, stock: 0, seconds: 0, items: 0 })?;
menu::battle_entrance(false)?;
menu::battle_ready_go(false)?;
menu::battle_pause(true)?;
let stocks = menu::battle_stock(0)?;
let serial = menu::battle_serial()?;
menu::pause_exit(Some(menu::PauseExit { row: "mel_pause_my_exit", confirm: Some("pop_pause_my_exit") }))?;
let how = menu::pause_choice()?;
menu::keep_music_into_battle(true)?;
menu::play_bgm("ui_bgm_my_song")?;
menu::stop_bgm()?;
menu::next_battle()?;
menu::end_battle()?;

menu::play_movie("stream:/movie/my_mod/intro.webm", 0)?;
let playing = menu::movie_playing()?;
menu::stop_movie()?;

menu::viewer_begin(0)?;
let shown = menu::viewer_state(0)? == menu::ViewerState::Shown;
menu::viewer_end()?;

let items = menu::gallery_item_count()?;
let costume = menu::gallery_item_costume(0)?;
```

During a battle, `next_battle` ends it and starts a new one from the current
selections, after a loading screen; set the next stage first, and the
starting damage with `battle_damage` to carry damage over. `end_battle`
ends it and returns to the main menu. Both need a battle started by a
`[[sequence]]` with `scene = "IngameCallSequenceScene"`.

`pause_exit` puts a row of your own in place of Connect Controller in the
pause menu of these battles, with the Quit icon: `row` is a label your pack
adds to `ui/message/msg_melee.xmsbt`, `confirm` one in `msg_popup.xmsbt`
asked instead of "End the game?". Choosing it and answering Yes ends the
battle like Quit. `pause_choice` tells which ended the last battle (`Quit`,
`Exit` or `None`) until the next battle builds its pause menu; read it when
the menu comes back. `pause_exit(None)` restores the vanilla rows.

A battle start stops the song the menu is playing. `keep_music_into_battle(true)`
leaves it on for the next battle start only, so a menu song carries into the
battle; give the stage a silent playlist so its own music does not play over
it. `play_bgm` and `stop_bgm` drive that same player from a plugin, also during
a battle.

`battle_serial` changes each time a battle starts; a plugin keys
per-battle setup on it rather than on `is_ready_go`, which a battle without
"Ready" and "GO!" may never show false.

`viewer_begin`, `viewer_state` and `viewer_end` are the `engine.viewer_*` calls
of the Model viewer: slot 0 or 1, and a state of `Loading`, `Built` or `Shown`.
`gallery_item_count` and `gallery_item_costume` read the open gallery's items.
Each returns `Engine(-1)` on a screen with no viewer, with no gallery open or
for an index past the last item. Call them from a menu call you registered, as
the Lua ones run.

Call names use letters, digits, `_` and `.`; `engine.` is reserved. The log
tags are `[menuseq]`, `[menucall]`, `[menubattle]`, `[battlepause]`,
`[battlemusic]`, `[menumovie]` and `[menuviewer]`.
