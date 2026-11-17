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
`sound.prcxml` work the same way.

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
| the computer player's row (`fighter/common/ai/param/fighter_param.prc`) | prcxml, `cpu_param` |
| lists and arrays in `vl.prc` (`hit_data`, ...) | your own `vl.prc` |
| lists in the common files, `battle_object.prc`, `spirits.prc` | cannot be changed per clone |

The game reads the `fighter_param` and common values past ParamConfig's hooks
(through `FighterParamAccessor2` or straight out of the row), so a ParamConfig
call on them only half applies. Dionysus sets them itself and also passes
them to ParamConfig.

## In code

A plugin can do the same. ParamConfig's calls work unchanged with the clone's
kind; `vec![-1]` is every costume.

```rust
let kind = MY_FIGHTER.kind().get().unwrap();

param_config::update_float(kind, vec![-1], (hash40("param_special_hi"), hash40("y_spd_air")), 1.5);
param_config::update_attribute_mul(kind, vec![3], (hash40("param_special_n"), hash40("fireball_speed_mul")), 1.15); // costume 3 only
param_config::disable_kirby_copy(kind, vec![-1]);

MY_FIGHTER.param("walk_speed_max").set(1.2);
MY_FIGHTER.param("param_motion").sub("flip").int(0);
MY_FIGHTER.param("common").sub("shield_max").set(20.0);
MY_FIGHTER.param("param_thrown").sub("held_offset").mul(1.2);
MY_FIGHTER.cpu_param("width").set(12.0);
```
