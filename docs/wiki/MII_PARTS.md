# Mii hats and outfits

Put a `mii.toml` beside the pack's `config.json` to add hats and outfits to
the Mii Fighter menus. Nothing of the game's is replaced, and no code is
needed.

## mii.toml

```toml
[[mii_hat]]
id = "miku_hair"
base = "h_sans"
model_type = "term"

[[mii_outfit]]
id = "b_miku_f"
base = "b_chinese_m"
fighter = "miiswordsman"
gender = "female"
```

Each part copies every value of its `base` row, then changes these:

### [[mii_hat]]

| Key | Meaning |
|---|---|
| `id` | Folder name and text id. Lower case letters, digits and `_`, up to 48 |
| `base` | The game's hat it copies: camera, placement per hair type, the rest |
| `model_type` | What shows of the Mii's head (see Head) |
| `hair_model_types` | Per hair style overrides, `["12=face_only", "40=hat"]` |
| `disp_order` | Place in the lists; default after the game's |
| `swing` | Swing bones on or off; default the base's |
| `is_patch` | The post-launch hats' look; default the base's |
| `owned` | Default `true`; `false` keeps it out of the menus |

### [[mii_outfit]]

| Key | Meaning |
|---|---|
| `id` | Starts with `b_` and ends in `_m` or `_f`, like the game's |
| `fighter` | `miifighter`, `miiswordsman` or `miigunner` |
| `base` | The game's outfit it copies |
| `gender` | `male`, `female` or `both`; default the base's |
| `name` | Icon name; default the id without `_m` or `_f` |
| `disp_order`, `swing`, `is_patch`, `owned` | As for hats |

## Files

| Part | Files |
|---|---|
| Hat | `miihat/<id>/model/body/c00` to `c11` |
| Hat swing bones | `miihat/<id>/motion/body/c00` |
| Outfit | `fighter/<fighter>/model/<id>/c00` to `c11` |
| Outfit motion | `fighter/<fighter>/motion/<id>/c00` |
| Hat icon | `ui/replace/mii_hat/mii_hat_0/mii_hat_0_<id>.bntx` |
| Outfit icon | `ui/replace/mii_body/mii_body_0/mii_body_0_<name>.bntx` |
| Names | `nam_mii_hat_<id>`, `nam_mii_body_<id>` in `ui/message/msg_name.xmsbt` |

- `c00` to `c11` are the twelve favourite colours. Ship all twelve.
- Each colour folder holds every file the base's colour folder has
  (`model.xmb`, `dark_model.numatb`, `light_model.numatb`,
  `metamon_model.numatb`, `model.nusrcmdlb`, ...), not only the ones a
  replacement mod changes. A new part has no game files to fall back on, and
  loading it can hang.
- An outfit's motion folder holds the base's `update.prc`, `ik.prc`,
  `flip.prc` and `scharge.prc`, and `motion_list.bin` and `swing.prc` when
  the base has them.

### config.json

The game loads a part as a folder. List it and its colour folders, each with
its own files and the motion files:

```json
{
  "new-dir-infos": ["miihat/miku_hair", "miihat/miku_hair/c00", "miihat/miku_hair/c01"],
  "new-dir-files": {
    "miihat/miku_hair/c00": ["miihat/miku_hair/model/body/c00/model.numdlb"],
    "miihat/miku_hair/c01": ["miihat/miku_hair/model/body/c01/model.numdlb"]
  }
}
```

- An outfit's folders are `miibody/<fighter>/<id>` and `miibody/<fighter>/<id>/cNN`.
- The Pack Workbench writes these lists from the files on disk.

## Head

| `model_type` | The Mii's head |
|---|---|
| `normal` | Full hair |
| `hat` | Hair trimmed to fit under a hat |
| `face_only` | Face only, no hair |
| `term` | None: the hat is the whole head, like the game's Sans or Cuphead |

`hair_model_types` picks another of these for some hair styles, by the hair
style number Mii data stores (0 to 131). The Mii maker shows no numbers;
Mii editing tools do. Leave it out to keep the base's.

## Save numbers

- A saved Mii keeps its hat and outfit by number. Each part gets one past
  the game's: hats 133 to 415, outfits 154 to 1023.
- The engine writes them to `sd:/ultimate/dionysus/mii_parts.txt` and
  keeps each line forever, installed or not.
- A Mii wearing a part whose pack is gone shows the part's base instead.
  Put the pack back and it wears the part again.

## With other mods

- The engine adds its rows to the tables as other mods and their `.prcxml`
  patches leave them.
- Two packs with the same id: the first pack folder by name keeps it.

## Checking the engine

Check `runtime_capabilities() & CAP_MII_PARTS`. An older engine ignores
`mii.toml`.

## Log lines

```text
[miiparts] 1 hat(s) and 1 outfit(s) from mii.toml packs; rows are added when the game loads its tables
[miiparts] hat miku_hair (My Pack) added as save number 133 (new)
[miiparts] hat miku_hair (save number 133) is not installed; a Mii wearing it shows h_sans instead
```
