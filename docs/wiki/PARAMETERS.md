# Stats and parameters

A clone is its own fighter: its values reach all of its costumes and never
the base. None of this needs code.

## fighter.toml

```toml
[params]
walk_speed_max = 1.2
param_motion.flip = 0              # a whole number sets an integer
common.shield_max = 20.0
param_special_hi.y_spd_air = 1.5   # a vl.prc value

[params.mul]
weight = 1.05

[params.c03]
param_special_hi.y_spd_air = 2.0   # costume 3 only
```

The keys are in the [API reference](../API.md#parameters). ParamConfig must
be installed. The Clone Pack Workbench's Parameters tab writes these.

## prcxml files

A prcxml made for one of the shared files works on a clone from the clone's
own `param` folder:

| Shared file | Your file |
|---|---|
| `fighter/common/param/fighter_param.prc` | `fighter/<name>/param/fighter_param.prcxml` |
| `fighter/common/param/fighter_param_motion.prc` | `fighter/<name>/param/fighter_param_motion.prcxml` |
| `fighter/common/param/common.prc` | `fighter/<name>/param/common.prcxml` |

`item.prcxml`, `etc.prcxml`, `power_up.prcxml`, `effect.prcxml` and
`sound.prcxml` work the same way. The Clone Pack Workbench's Param files tab
starts each of these files, and the item, enemy and stage ones below, as a copy
of the game's own values.

Write it as you would for the shared file. The row index does not matter:
the row is always your clone's, so a file made for the base's row works as it
is. Its values reach every costume, `[params]` wins over them, and the boot
log names anything left out (a label such as `size`, a list). ParamConfig
must be installed.

Do not ship a prcxml for the shared file itself (`fighter/common/param/`):
it changes the base for every player.

Items take `common_param.prcxml` ([ITEMS.md](ITEMS.md)) and stages
`item_switch_param.prcxml`, `stage_sound_settings.prcxml` and
`stage_param.prcxml` ([STAGES.md](STAGES.md)).

## Params of your own

Any key works, not only the game's: a new field in a group or a whole new
group.

```toml
[params]
param_special_n.charge_frames = 30   # a new field in a group the game has
my_group.speed = 2.25                # a group of your own
param_fireball.speed = 0.4           # your fireballs: the article's block
```

Your moveset reads them like any other value:

```rust
WorkModule::get_param_int(boma, hash40("param_special_n"), hash40("charge_frames"));
WorkModule::get_param_float(boma, hash40("my_group"), hash40("speed"));
```

The base and every other fighter read 0 for a key nobody set. ParamConfig
must be installed.

`fighter/<name>/param/vl.prcxml` does the same with no code and no copy of
`vl.prc`: every value under a top-level group, or under the first row of a
top-level list, is set for your clone. Lists inside a group, hash40 values and
text are left out, and `[params]` wins over the file. Ship your own `vl.prc`
too and ARCropolis patches it with the file, every row included; the Clone
Pack Workbench's Param files tab writes that copy when you change such a
value.

Your articles read their own block: values under `param_<article>` (the
base's article, `param_fireball`) or `param_<fighter>_<article>` (an article
of yours) reach the articles your fighter makes, new fields included, and the
base's keep theirs. An article ignores the group it is asked for, so its code
can read `get_param_float(boma, hash40("param_fireball"), hash40("speed"))`.
Only values for every costume reach articles.

Items, enemies and stages take keys of their own too, from a `param.prcxml`
or from code, with no ParamConfig. A value at the top of the file is in the
group `param`; a struct is a group of its own.

| Clone | File | Read with |
|---|---|---|
| Item | `item/<name>/param/param.prcxml` | `app::item::self_param_float(kind, hash40("key"))`, `self_struct_param_float(kind, hash40("group"), hash40("key"))`, or `WorkModule::get_param_float(boma, hash40("param"), hash40("key"))` |
| Enemy | `enemy/<name>/param/param.prcxml` | `WorkModule::get_param_float(boma, hash40("param"), hash40("key"))` |
| Stage | `stage/<place>/normal/param/param.prcxml` | `dionysus_api::stage::param_float("key")`, for the stage being played |

```rust
WAWA.param("speed").set(2.0);                                // or v2::item_param(kind, "speed")
dionysus_api::enemy::param(kind, "my_group.range").mul(1.5); // "group.key" for a group
dionysus_api::stage::param("my_stage", "wind").set(0.5);
```

Code sets over the file. A key the game has (an item's `have_scale`) answers
your value too, and `.mul` scales the game's value (not for stages). A key
nobody set reads 0, or `None` on a stage.

## Your own parameter files

Ship `fighter/<your name>/param/vl.prc` and it is used instead of the
base's, also with the base fighter in the same match: each reads its own.
The camera and CPU parameter trees are all or nothing: set
`owns_param_resources` only once the whole tree is in your folder; a partial
tree crashes or loads forever.

A computer player plays your fighter with its base's AI, whether or not the
base is in the match. Its row of `fighter/common/ai/param/fighter_param.prc`
(recovery reach, attack ranges, Final Smash range) is its base's unless you
ship `fighter/<name>/ai/param/fighter_param.prcxml` (its first `base` row,
whatever its index or `name_hash`; no ParamConfig needed) or set it with
`cpu_param` ([API](../API.md#computer-players)).

## What reaches what

| Values | Reached by |
|---|---|
| `vl.prc` (`param_special_*`, `param_private`, `fly_data`, ...) | `[params]` (through ParamConfig), your own `vl.prc` |
| `fighter_param.prc` attributes (`walk_speed_max`, `jump_y`, `scale`, `weight`, ...) | `[params]`, prcxml |
| `fighter_param_motion.prc` (`param_motion`), the `fighter/common/param/` files (`common`) | `[params]`, prcxml |
| throw positions (`param_thrown`) | `[params]` |
| keys of your own (new fields, new groups) | `[params]`, `vl.prcxml` |
| keys of your own on items, enemies and stages | `param.prcxml`, `param` in code |
| your articles' blocks (`param_<article>`) | `[params]`, `vl.prcxml`, your own `vl.prc` |
| the computer player's row (`fighter/common/ai/param/fighter_param.prc`) | prcxml, `cpu_param` |
| lists and arrays in `vl.prc` (`hit_data`, ...) | your own `vl.prc` |
| lists in the common files, `battle_object.prc`, `spirits.prc` | cannot be changed per clone |

The game reads the `fighter_param` and common values past ParamConfig's hooks
(through `FighterParamAccessor2` or straight out of the row), so a ParamConfig
call on them only half applies. Dionysus sets them itself and also passes
them to ParamConfig.

## In code

A plugin can do the same. Every param call has one shape: name the value,
then `.set`, `.mul` or `.int` ([API](../API.md#parameters)).

```rust
MY_FIGHTER.param("param_special_hi.y_spd_air").set(1.5);                 // a vl.prc value
MY_FIGHTER.param("param_special_n.fireball_speed_mul").slot(3).mul(1.15); // costume 3 only
MY_FIGHTER.param("walk_speed_max").set(1.2);
MY_FIGHTER.param("param_motion.flip").int(0);
MY_FIGHTER.param("common.shield_max").set(20.0);
MY_FIGHTER.param("param_thrown.held_offset").mul(1.2);
MY_FIGHTER.cpu_param("width").set(12.0);
```

ParamConfig's own calls also work with the clone's kind (`vec![-1]` is every
costume), for what has no param call, such as
`param_config::disable_kirby_copy(kind, vec![-1])`.
