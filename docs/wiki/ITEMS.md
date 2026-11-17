# Items

A custom item gets its own model, animations, `param.prc`, scripts,
statuses, shared-file settings and Training menu cell. Effects and sounds
come from the base item's banks. Start from
[custom_items/template_v2](../../custom_items/template_v2/).

## Declaring the item

In the plugin, before anything else:

```rust
use dionysus_api::v2::{item, ItemManifest};
use dionysus_api::ItemStatusLine;

item!(WAWA, "wawa");

#[skyline::main(name = "wawa")]
pub fn main() {
    let manifest = ItemManifest::new(
        "wawa",      // resource name: files under item/wawa, script name
        "killsword", // the vanilla item it is built on, here the Killing Edge
    )
    .spawn_per(30)                    // natural drop weight; most vanilla items are 20 to 50
    .spawn_from(&["box", "barrel"])   // containers that can hold it
    .common("throw_speed_mul", 0.75); // a field of item/common/param/param.prc, for this item only
    let Ok(kind) = WAWA.register(manifest) else { return };

    WAWA.status(ItemStatusLine::Init, "THROW", throw_init); // status line, a status name of the base item, function
}
```

Or as an `item.toml` beside the pack's `config.json`, in which case the
plugin (if any) skips `register`:

```toml
base_item     = "killsword"         # the vanilla item you build on, here the Killing Edge
resource_name = "wawa"              # your files live under item/wawa
spawn_per  = 30                     # optional: natural drop weight
spawn_max  = 2                      # optional, with spawn_min; both default to 1
spawn_from = "box, barrel, capsule" # optional: containers that can hold it

[common]
throw_speed_mul = 0.75
life = 300                          # a whole number is an integer field
```

Optional keys: `agent_name` (script name, defaults to `resource_name`),
`ui_id` (defaults to `ui_item_<resource_name>`), `training_order`. Drops
follow the item switch of your base item: when the player turns the base off,
your item is off too. Several items in one pack are `[[item]]` blocks with
the same keys (engines after `0.2.1-beta.1`).

A prcxml made for `item/common/param/param.prc` sets the same row from
`item/wawa/param/common_param.prcxml` (`assist/` or `pokemon/` for those
families; each member reads its own folder). The row index does not matter
and `name` is ignored: the row is your item's. `[common]` and the plugin's
`.common` win over it, and the boot log names anything left out.

Statuses are registered at startup, by name; they are the base item's and the
lines are `Setting`, `JointSrt`, `Init`, `Update`, `Coroutine`, `Exit`.

To spawn or give the item from code:

```rust
ItemModule::have_item(
    boma,                                             // the fighter that gets the item
    smash::app::ItemKind(WAWA.kind().get().unwrap()), // the clone's kind, wrapped in the game's type
    0, 0, false, false,                               // the game's own arguments, as for a vanilla item
);
```

Packs written before `item.toml` register the item in code with
`allocate_item`; that still works and is in
[Deprecated: the long form](../DEPRECATED.md#items).

To create an item of any kind at a point, with no fighter involved (Assist
Trophy summons included), use the game's own creation entry:

```rust
let id = dionysus_api::item::spawn(kind, x, y, 0.0, lr, 0)?; // battle object id
let item = smash::app::sv_battle_object::module_accessor(id);
TeamModule::set_team(item, 7, true); // a new item has no team; an assist on its own team fights everyone
```

Call it during a battle frame. A boss created this way waits, invisible, in
its first status until it is sent to its `wait` status:

```rust
if let Some(status) = dionysus_api::item::alias_status(item as usize, "wait") {
    StatusModule::change_status_request(item, status, false); // the boss starts its fight
}
```

## Assist Trophies, Pokemon and bosses

These come as families: the Assist, Pokemon or boss first, then the items
it creates. Clone the whole family by naming every member:

```toml
base_item     = "galleom"
resource_name = "my_boss"
members       = ["my_boss", "my_boss_missile", "my_boss_shockwave"]
```

- `members` follows the base family's order, this item first. Galleom's is
  `galleom`, `galleommissile`, `galleomshockwave`.
- Each member takes a free kind and loads its own files from the top folder
  its base lives in: `assist/<member>/`, `pokemon/<member>/`, `boss/<member>/`,
  or `item/<member>/` for an item that fires shots (see API.md, Items).
- A wrong count is refused, and the log says how many members the base
  family has.
- In code: `ItemManifest::new("my_boss", "galleom").members(&[...])`.
- The Workbench fills `members` in from the base it is given.

## An item whose settings live on a fighter

Some items (Steve's blocks, `pickelobject`) read their settings from a
fighter's `vl.prc`. `owner_param` gives your item its own values, by fighter
and field name, without touching the fighter:

```rust
ItemManifest::new("wawa_block", "pickelobject")
    .owner_param("pickel", "life", 600.0)       // owner fighter (Steve), a field of its vl.prc, value
    .owner_param("pickel", "auto_damage", 0.0)
```

```toml
[owner_params]
pickel.life = 600
pickel.auto_damage = 0
```

Set every field that decides the same outcome: a block ends on `life` or
`auto_damage`, whichever comes first.

[custom_items/template_v2](../../custom_items/template_v2/) does exactly
this in code with `owner_param("pickel", "life").set(600.0)`.

## File layout

```text
item/wawa/model/body/c00/...
item/wawa/motion/body/c00/motion_list.bin
item/wawa/param/param.prc
item/wawa/script/animcmd/body/game.lc
item/wawa/script/animcmd/body/effect.lc
item/wawa/script/animcmd/body/sound.lc
```

In `config.json`, key every `new-dir-files` group by the folder that directly
holds the files (`item/wawa/model/body/c00`). Otherwise the files never load
and the log says nothing. Leave `new-dir-infos-base` empty: a base link on a
folder whose children you also add corrupts the base item's files.

`motion_list.bin` names its own folder: the u64 at offset 8 is
`hash40("item/wawa/motion/body/c00")`. An item loads every animation from the
folder that word names, so a list copied from the base keeps the base's
animations (its own files and new motions then play nothing).

## Boot log

```text
[itempack] 2 item.toml pack(s) under sd:/ultimate/mods
[itempack] Wawa Item: public=0x36b base=63 (killsword) resource=wawa ui=ui_item_wawa ui_result=0
```

No line at all means no `item.toml` was found. A `REFUSED` line names the
reason. The `public=` number is the kind this boot; it changes when other packs
are installed, and nothing you ship should depend on it.
