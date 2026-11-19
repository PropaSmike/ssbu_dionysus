# Spirits

A pack adds spirits and DLC spirit boards with four prcxml files in a
`spirit/` folder beside its `config.json`. Nothing of the game's is replaced,
and no code is needed. Each spirit gets its own number in the Spirit List, its
own battle and its own place in the save; each board gets its own button and
its own prize.

## spirit/ prcxml files

| File | Holds | Shaped like |
|---|---|---|
| `spirit/ui_spirit_db.prcxml` | a row per spirit | `ui/param/database/ui_spirit_db.prc` |
| `spirit/ui_spirits_battle_db.prcxml` | each spirit's battle row and fighter rows | `ui/param/database/ui_spirits_battle_db.prc` |
| `spirit/ui_spirit_layout_db.prcxml` | each spirit's art placement | `ui/param/database/ui_spirit_layout_db.prc` |
| `spirit/spirits_board_special_param.prcxml` | a row per board | `ui/param_patch/spirits_board_special/spirits_board_special_param.prc` |

Each file holds only the pack's own rows, written the way the game's tables
hold them. A pack ships the files it needs.

```xml
<?xml version="1.0" encoding="utf-8"?>
<struct>
  <list hash="db_root">
    <struct index="0">
      <hash40 hash="ui_spirit_id">wawa</hash40>
      <hash40 hash="base">sidestepper</hash40>
      <hash40 hash="rank">hope</hash40>
      <int hash="base_attack">1200</int>
    </struct>
  </list>
</struct>
```

```xml
<?xml version="1.0" encoding="utf-8"?>
<struct>
  <list hash="battle_data_tbl">
    <struct index="0">
      <hash40 hash="battle_id">wawa</hash40>
      <hash40 hash="ui_stage_id">ui_stage_battle_field</hash40>
    </struct>
  </list>
  <list hash="fighter_data_tbl">
    <struct index="0">
      <hash40 hash="battle_id">wawa</hash40>
      <hash40 hash="fighter_kind">ui_chara_mario</hash40>
    </struct>
    <struct index="1">
      <hash40 hash="battle_id">wawa</hash40>
    </struct>
  </list>
</struct>
```

- A spirit row needs `ui_spirit_id`, the spirit's id (lower case letters,
  digits and `_`, up to 48), and `base`, the game's spirit it copies. The id
  also names its text and pictures. When `ui_spirit_id` is only a `0x...`
  hash, `name_id` gives the id.
- The spirit copies its base's row, battle, fighters and layout, then takes
  every value its rows list.
- Battle and fighter rows belong to the spirit their `battle_id` names, layout
  rows to the one their `ui_spirit_layout_id` names. A row for a spirit the
  spirit file does not add is refused.
- Its fighter rows are its fighters, in order, 1 to 16. Without any, the
  spirit keeps its base's.
- The engine sets `save_no`, `directory_id`, `fixed_no` and `spirit_name`;
  the files' values for them are ignored.
- Hash fields take the game's names (`rank` `legend`, `type`
  `spirits_type_support`, `fighter_kind` `ui_chara_mario`) or `0x...`.
- A field the base's row does not have, or a value that does not fit, is left
  out and named in the log.
- The spirit is not a DLC spirit (`is_dlc` false) and has no unlock check,
  whatever its base: set `is_dlc` yourself only with pictures under
  `ui/replace_patch/spirits/`.
- In a battle, fighters with `first_appear` true start the fight; the others
  come in as reinforcements, as in the base's battle.
- A file that cannot be read, or a row it refuses, skips the pack's prcxml
  files; the log names the file and row.

The Clone Pack Workbench writes these files: Add a spirit or Add a board
copies every row of the base under the new id, and its prcxml files tab edits
the values.

## Files

| What | File |
|---|---|
| Pictures | `ui/replace/spirits/spirits_0/spirits_0_<id>.bntx`, `spirits_1`, `spirits_2` (and `spirits_3` when the base has one) |
| Name | `spi_<id>` in `ui/message/msg_spirits.xmsbt` |

Copy the base's pictures to see the sizes each one takes.

## Making spirits with Spirit Board

[Spirit Board](https://gamebanana.com/tools/9621), made by Tougon and redistributed by Hinata, edits every
field of a spirit, with a preview of its art. Its export replaces the game's
whole tables, so two exports do not combine. The Clone Pack Workbench's
**Import from Spirit Board** turns the export's new spirits into rows of the
pack's prcxml files, with their names and pictures, so they sit beside every
other spirit pack.

## Save numbers

- The save keeps a spirit's progress by number. The game's spirits and the
  online event spirits use 0 to 1512; added spirits get 1513 to 1999, the last
  number the save has room for. That is 487 spirits for every pack together.
- The engine writes them to `sd:/ultimate/dionysus/spirits.txt` and keeps each
  line forever, installed or not, so a removed pack's number is never handed
  to another spirit.
- A spirit's number in the Spirit List is its save number plus one.
- A save edited to collect every spirit marks all 2000 numbers collected, so
  added spirits show as collected there and their boards as complete.

## Boards

```xml
<?xml version="1.0" encoding="utf-8"?>
<struct>
  <list hash="special_board_data_list">
    <struct index="0">
      <hash40 hash="label">ui_chara_wawa_board</hash40>
      <hash40 hash="base">trail</hash40>
      <string hash="name">Wawa's Board</string>
      <hash40 hash="background">jack</hash40>
      <string hash="art">ui/layout/menu/my_boards/spb_wawa_board/layout.arc</string>
      <uint hash="value">5000</uint>
      <list hash="spirit_list">
        <struct index="0"><hash40 hash="spirit_id">wawa</hash40></struct>
        <struct index="1"><hash40 hash="spirit_id">geno</hash40></struct>
      </list>
    </struct>
  </list>
</struct>
```

A board copies its `base`: its row of
`ui/param_patch/spirits_board_special/spirits_board_special_param.prc`, with
the base's prize, music and background. Then it takes every value its row
lists. It plays as one of the game's DLC spirit boards.

| Field | Meaning |
|---|---|
| `label` | `ui_chara_<id>`, the board's id: lower case letters, digits and `_`, up to 48 |
| `base` | The game's board it copies: `jack`, `brave`, `buddy`, `dolly`, `master`, `tantan`, `pickel`, `edge`, `flame_first`, `demon`, `trail`, or `ui_chara_...` / `0x...` |
| `name` | The text on its button and card; default the id. One line, up to 64 bytes |
| `background` | The game's board whose background it takes; default the base's |
| `spirit_list` | Its spirits: the game's ids and any pack's, up to 64 |
| `art` | Its button banner and card on the DLC Spirits popup: a layout path, see below |
| `value` | Its prize value; the game's boards have 10000, a prize of 10,000 gold |
| `bgm_id` | Its music, a `ui_bgm_...` id |
| `prize_id` | Its prize; the game's boards give `ui_prize_gold_direct` |
| `layout_no` | Its background's number: 0 to 10 are the game's, 11 and up a pack's own (below) |
| `shop_item_tag` | Its shop tag; the engine clears the base's |

- Every field takes the type the base's row has; a field the row does not
  have, or a value that does not fit, is left out and named in the log.
- The engine sets `save_no`; the file's value is ignored.
- `layout_no` wins over `background`.

- The `Dionysus Modes` pack shows the boards: R on the DLC Spirits popup turns
  to pages of pack boards, 11 a page, and L turns back. Without it the boards
  are in the table but have no button.
- Choosing a board opens it straight away, with no eShop check.
- Spirits that are neither the game's nor a pack's are left out and named in
  the log; a board with none left is skipped.
- A board is complete once every spirit on it is collected; it pays its prize
  once.

The page asks the engine through `engine.spirit_board_count` and
`prg_func.engine_text` with `engine.spirit_board_id`, `engine.spirit_board_name`
and `engine.spirit_board_art` (board index from 0).

## Board art

`art` names a layout the pack ships, such as
`ui/layout/menu/my_boards/spb_wawa_board/layout.arc`. The engine adds it to
the main menu, over the side panel, and the page shows its panes on the
board's place:

| Pane | Size | Shown |
|---|---|---|
| `banner` | 462 x 92 | on the board's button, in place of its name |
| `card` | 460 x 796 | in place of the card while the board is picked |

- The layout is named after its folder (`spb_wawa_board`), and both panes
  start hidden: every main menu screen loads it.
- A pane the layout lacks keeps the name in its place.
- The name is not drawn over the art: put the board's title in the pictures.
- The Clone Pack Workbench makes the layout from two pictures.

## Own background

A board's background is the layout
`ui/layout/patch/spirits_board_special_bg_NN/spirits_board_special_bg_NN/layout.arc`,
`NN` its `layout_no` (at least two digits). The game ships 0 to 10. A pack
gives its board its own by shipping that layout at 11 or above and setting
`layout_no` to the number.

- The layout is named `spirits_board_special_bg_NN`; one picture over the
  screen is enough.
- Two packs with the same number show one background for both: pick a
  number no other installed pack ships.
- The Clone Pack Workbench makes the layout from a picture, at a free number.

## Board numbers

- The save keeps a board's prize by number in a 32-bit field. The game's
  boards use 0 to 10; added boards get 11 to 31. That is 21 boards for every
  pack together.
- The engine writes them to `sd:/ultimate/dionysus/spirit_boards.txt` and
  keeps each line forever, as with spirits.

## spirit.toml

The engine also reads a `spirit.toml` beside `config.json`, the older short
form: each `[[spirit]]` or `[[board]]` names its `id` and `base` and only the
fields it changes.

```toml
[[spirit]]
id = "wawa"
base = "sidestepper"
rank = "hope"
fighters = 2
fighter.0.fighter_kind = "ui_chara_mario"
battle.ui_stage_id = "ui_stage_battle_field"

[[board]]
id = "wawa_board"
base = "trail"
name = "Wawa's Board"
spirits = ["wawa", "geno"]
value = 5000
```

| Key | Meaning |
|---|---|
| `fighters` | How many fighters its battle has, 1 to 16; default the base's. Past the base's count the last one repeats |
| `<field>`, `battle.<field>`, `fighter.N.<field>`, `layout.<field>` | A field of its row, battle row, fighter `N` (from 0) and layout row |
| `spirits` | A board's spirits, on one line |

- Listing a field the engine sets is an error here.
- The Clone Pack Workbench moves a pack's `spirit.toml` rows into its prcxml
  files the first time it saves the pack's spirits.

## With other mods

- The engine adds its rows to the tables as other mods and their `.prcxml`
  patches leave them.
- Two packs with the same spirit or board id: the first pack folder by name
  keeps it.

## Checking the engine

Check `runtime_capabilities() & CAP_SPIRITS`. An older engine ignores the
`spirit/` files.

## Log lines

```text
[spirits] My Pack: 2 spirit(s) and 1 board(s) from its spirit/ prcxml files
[spirits] 2 spirit(s) and 1 board(s) from spirit packs; rows are added when the game loads its tables
[spirits] wawa (My Pack) added as save number 1513, spirit no. 1514 (new)
[spirits] served the spirit table: 335772 bytes from the game and mods, 336100 with the packs' rows
[spirits] board wawa_board (My Pack): art ui/layout/menu/my_boards/spb_wawa_board/layout.arc joins the main menu
[spirits] board wawa_board (My Pack) added as board number 11 with 3 spirit(s) (new)
[spirits] My Pack: its spirit/ prcxml files are not readable: Prcxml { file: "spirit/ui_spirits_battle_db.prcxml", row: 2, message: "the row is for wawb, which spirit/ui_spirit_db.prcxml does not add" }; skipped
```
