# Dionysus

Dionysus is a Smash Ultimate plugin that adds content instead of replacing a
slot. Its Clone Engine adds fighters, items, stages and enemies of their own,
each built on a vanilla one. Dionysus also adds modes and menus, and Mii hats
and outfits.

Game version 13.0.4 or 13.0.5. Dionysus detects the version at boot.
`sd:/ultimate/dionysus/config.toml` can name it instead:

```toml
game_version = "auto"
```

`auto`, `13.0.4` or `13.0.5`. A wrong value is logged and the detected
version is used. The other plugins must match the game: ARCropolis 4.0.8
runs only on 13.0.4, 4.1.x only on 13.0.5, and Smashline needs 1.6.7 or
newer on 13.0.5. Smashline 2.0 runs only on 13.0.5; with it, the engine
registers articles through Smashline's `clone_weapon`, so weapon kinds are
shared with other mods. Plugins built for Smashline 1.x `clone_weapon` crash
a 2.0 boot until they are rebuilt.

## What you need

- Ozone and ARCropolis
- Smashline
- NRO Hook
- The CSK Collection, which adds the character and stage select entries
- ParamConfig, optional but recommended, for changing fighter stats

## Install

Put Dionysus in the global plugin folder of your SD card:

```text
atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_dionysus.nro
```

Smashline (`libsmashline_plugin.nro`) goes in the same folder.

Updating from Clone Engine: delete `libssbu_clone_engine.nro` from that
folder. The two cannot run together: with both installed, Dionysus says so and
closes the game. Its settings move from `sd:/ultimate/clone_engine/` to
`sd:/ultimate/dionysus/` on the first boot; the old folder is left as it was.

The `Dionysus Modes` folder from the release goes under `ultimate/mods/`:
it is the modes page, where packs that add modes of their own list them (X
on the main menu; see docs/wiki/MENUS.md).

Every mod pack goes in its own folder under `ultimate/mods/`. If a pack has
code, that code is a `plugin.nro` inside the pack's own folder, never in the
global plugin folder.

The first boot writes `sd:/ultimate/dionysus/settings.toml`:

```toml
gliding = true
stage_select_ui = "vanilla"
```

`false` turns off the glide that packs get from the engine, for use with
another glide mod.

`stage_select_ui` says which stage select screen you have installed, so
Dionysus can add buttons for stages past the 121 the game has:

| Value | Use it with | What Dionysus does |
|---|---|---|
| `vanilla` | The game's screen, or a reskin that keeps its grid | Adds the buttons and shrinks the grid to fit them |
| `page_turning` | A screen with its own page turning, like Ultimate S | Adds the buttons off screen for its script to place |
| `none` | Anything else | Leaves the layout alone |

Nothing changes until there are more stages than buttons. With `vanilla`, a
screen that is not a plain grid is left alone and the log says so.

## A fighter

The plugin registers the fighter, then keeps its CSK, ParamConfig and
Smashline calls as they were; only the names change:

```rust
use dionysus_api::v2::{fighter, Manifest};

fighter!(MY_FIGHTER, "my_fighter"); // the handle, and the identity

#[skyline::main(name = "my_fighter")]
pub fn main() {
    let manifest = Manifest::new(
        "my_fighter", // name: files under fighter/my_fighter, Smashline agent name
        "mario",      // base: the vanilla fighter it behaves like
    )
    .costumes(8)
    .own_css();       // the CSK call below publishes the select entry
    let Ok(kind) = MY_FIGHTER.register(manifest) else { return };

    add_chara_db_entry_info(CharacterDatabaseEntry {
        ui_chara_id: hash40("ui_chara_my_fighter"),                                 // ui_chara_<name>
        fighter_kind: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),      // fighter_kind_<name>
        fighter_kind_corps: Hash40Type::Overwrite(hash40("fighter_kind_my_fighter")),
        ..
    });
    param_config::update_float(kind, vec![-1], (hash40("param_special_hi"), hash40("y_spd_air")), 1.5); // the clone's kind, every costume
    smashline::Agent::new("my_fighter")                                              // scripts under the clone's own name
        .game_acmd("game_attack11", game_attack11, smashline::Priority::Default)
        .on_line(smashline::Main, fighter_frame)
        .install();
}
```

A pack with no code declares the same thing in a `fighter.toml` next to its
`config.json`, and the engine publishes the select entry:

```toml
name = "my_fighter"        # your files live under fighter/my_fighter
base = "mario"             # the vanilla fighter it behaves like
display_name = "My Fighter"
costumes = 8
series = "mario"
```

Start from [custom_fighters/template_v2](custom_fighters/template_v2/).
Packs written before `fighter.toml`, where the plugin registers the fighter
itself, keep working; that form is documented in
[docs/DEPRECATED.md](docs/DEPRECATED.md).

## Numbers are not yours to pick

The engine assigns every kind and id at boot, and they change with the
player's other mods. Use your name (`my_fighter`) everywhere; never write a
number into a file name or a config.

## Clone Pack Workbench

[Clone Pack Workbench](https://github.com/PropaSmike/clone_pack_workbench)
builds the pack itself: it writes `config.json`, `fighter.toml`, `item.toml`
and `stage.toml`, and checks a pack before you install it. Start there for a
pack without code.

## Templates

| Template | What it shows |
|---|---|
| [Fighter](custom_fighters/template_v2/) | Registration by code, a capability check, the CSK select entry, ParamConfig and engine stats, a Smashline moveset, an article with its own scripts and ParamConfig behaviours, a Kirby copy with its own motions, article and scripts, and slot, common, copy and offset hooks |
| [Item](custom_items/template_v2/) | Registration by code, a capability check, a pack skeleton, a status, common and owner parameters, natural drops, item and common vtable hooks, and a boss family with its members (`KIND_AUTO`) |
| [Stage](custom_stages/template/) | A stage with normal, Omega and Battlefield forms and a stage select entry, moving platforms, paused model motions, ladders, barrel cannons, springs and a door the plugin opens |
| [Enemy](custom_enemies/template/) | A clone of Nabbit with its own logic, brought into every battle on any stage |
| [Menu](custom_menus/template/) | Modes on the modes page: a gauntlet of battles with team swaps, a custom pause row, music, HUD layouts and a boss on the boss bar; the game's boss battle; a mode without select screens; a movie screen; main menu codes and patches |

The long-form API is in [DEPRECATED.md](docs/DEPRECATED.md). Its templates
are retired; the ones above show everything they did.

## What Dionysus handles

- fighters: identity, files, character select entry, colours, effects,
  sounds, camera, CPU behaviour, stats, Final Smash backgrounds
- articles (a fighter's projectiles and objects), with Kirby copy support
- items: files, settings, scripts, statuses, Training menu cell, drops
- item families: Assist Trophies, Pokemon and bosses
- enemies: Ultimate's stage enemies on any stage, and new enemy kinds made
  from them
- stages: identity, forms, stage select entry and buttons, collision,
  settings, music
- menus: main menu buttons, modes, battles started from a menu
- several packs changing the same game function at once

## Documentation

- [Getting started](docs/wiki/GETTING_STARTED.md)
- [Fighters](docs/wiki/FIGHTERS.md)
- [From a one-slot moveset](docs/wiki/FROM_A_ONE_SLOT_MOVESET.md)
- [Stats and parameters](docs/wiki/PARAMETERS.md)
- [Articles and Kirby](docs/wiki/ARTICLES_AND_KIRBY.md)
- [Items](docs/wiki/ITEMS.md)
- [Stages](docs/wiki/STAGES.md)
- [Menus](docs/wiki/MENUS.md)
- [Mii hats and outfits](docs/wiki/MII_PARTS.md)
- [API reference](docs/API.md), for packs with code
- [Deprecated: the long form](docs/DEPRECATED.md), for packs written before `fighter.toml`

## Using the API in your plugin

```toml
[dependencies]
dionysus_api = { git = "https://github.com/PropaSmike/ssbu_dionysus", tag = "<release>" }
```

Use the tag of the Dionysus release you installed. Releases from before the
rename name the crate `clone_engine_api`.

## Building Dionysus

```sh
cargo skyline build --release
```

Features named `research_*`, `diag_*` or `selftest_*` are left out of release
builds.

## Credits

Dionysus exists thanks to the work and research of the Smash Ultimate
modding community. People are not always kind to those who put in the hardest
work, so please show them the respect they deserve.

**Runtime dependencies**
- [Ozone](https://github.com/WuBoytH/ozone): WuBoytH, Shadów, blujay, Raytwo
- [Skyline](https://github.com/skyline-dev/skyline), which came before it: shadowninja108, jam1garner, 3096, Raytwo, Genwald, blu-dev, jugeeya, Sammi-Husky
- [ARCropolis](https://github.com/Raytwo/arcropolis): Raytwo, blu-dev, Coolsonickirby, jam1garner, jozz024, WuBoytH, itsmeft24, Genwald
- [Smashline](https://github.com/HDR-Development/smashline): WuBoytH, blu-dev, FatherOfEgg, moklmaru, plyrthn, Moydow
- [NRO Hook](https://github.com/ultimate-research/nro-hook-plugin): jam1garner, jugeeya, blu-dev
- [The CSK Collection](https://github.com/Coolsonickirby/the_csk_collection_api): Coolsonickirby, zrksyd
- [ParamConfig](https://github.com/CSharpM7/lib_paramconfig): CSharpM7, Coolsonickirby, theincredibleplayer

**Build dependencies**
- [skyline-rs](https://github.com/ultimate-research/skyline-rs): jam1garner, Raytwo, jugeeya, blu-dev, WuBoytH, tech-ticks, Genwald, TheGreenPlanet
- [skyline-smash](https://github.com/ultimate-research/skyline-smash): WuBoytH, blu-dev, jobrien97, jam1garner, jugeeya, Ayerbe-Dev, theincredibleplayer, FaultyPine
- [smash-script](https://github.com/WuBoytH/smash-script): blu-dev, Claude-1308, Ayerbe-Dev, FaultyPine, WuBoytH
- [ninput](https://github.com/blu-dev/ninput): blu-dev

**Research**
- [smash-vtables](https://github.com/theincredibleplayer/smash-vtables): theincredibleplayer

## License

Dionysus is free software under the GNU General Public License version 3.
It comes with no warranty. See [LICENSE](LICENSE).
