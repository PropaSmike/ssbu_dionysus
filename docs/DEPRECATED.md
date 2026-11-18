# Deprecated: the long form

Registration in code, from before `fighter.toml`, `item.toml`, `stage.toml`
and `v2`. Still works; not for new packs. Each section names the replacement
in [API.md](API.md).

The long-form fighter and item templates are retired; the
[fighter](../custom_fighters/template_v2/) and
[item](../custom_items/template_v2/) templates show the same features the new
way. The [stage](../custom_stages/template/) template still uses these calls.

## Fighters

Replacement: `Manifest` + `NAME.register`, or `fighter.toml`.

```rust
use dionysus_api::{CloneKind, CloneRegistration, KIND_AUTO};

static KIND: CloneKind = CloneKind::new("fighter_kind_wawa"); // holds the kind against the permanent name

let mut fighter = CloneRegistration::new(
    KIND_AUTO,            // let the engine pick the kind
    3,                    // base kind: the vanilla fighter (3 = Samus)
    "ui_chara_wawa",      // CSK identity: ui_chara_<name>
    "fighter_kind_wawa",  // permanent identity: fighter_kind_<name>
    "wawa",               // resource name: files under fighter/wawa
    "samus",              // base resource name: files you do not ship
);
fighter.color_count = 8;  // costumes; color_start defaults to 0

KIND.store(dionysus_api::allocate(&fighter)?); // register, keep the kind
```

| Field | Meaning |
|---|---|
| `custom_kind` | `KIND_AUTO`. |
| `base_kind` | Vanilla fighter kind. |
| `ui_chara` | `ui_chara_<name>`. |
| `fighter_kind_name` | `fighter_kind_<name>`, the permanent identity. |
| `resource_name` | File root under `fighter/`. |
| `base_resource_name` | Vanilla root for files you do not ship. |
| `color_start`, `color_count` | Costume range. |
| `effect_namespace`, `article_namespace` | Leave 0. |
| `articles` | Base weapon kinds that need file names of their own. |
| `copy_status_first`, `copy_status_count` | Kirby copy statuses. |
| `flags` | `FLAG_OWNS_PARAM_RESOURCES`, `FLAG_KIRBY_COPY_FULL_MODEL`. |

| Function | Meaning |
|---|---|
| `allocate(&CloneRegistration)` | Register, get the kind. |
| `register(&CloneRegistration)` | Register with a kind you chose. |
| `CloneKind::new(name)`, `.store(kind)`, `.get()` | Holds the kind. |
| `capacity_committed()` | Whether custom kinds 121 to 255 can be allocated; true in every current build. |

Order: engine and Smashline checks, `allocate`, Smashline scripts,
ParamConfig and articles, Kirby and shared hooks, mount callback, CSK entry
last.

## Knowing which fighter you are

Replacement: `NAME.is(x)`, `NAME.owns(x)`.

| Function | Use |
|---|---|
| `is_kind(boma, kind)` | Fighter check. |
| `is_owned_by_kind(boma, kind)` | Weapon and article check. |
| `true_kind(boma)` | Kind behind a fighter BOMA. |
| `owner_true_kind(boma)` | Kind of a fighter, or of a weapon's owner. |
| `base_kind(kind)` | Vanilla base of a clone kind. |
| `entry_kind(entry_id)` | Kind of a match slot. |
| `article_owner_kind(boma_as_u64)` | `owner_true_kind` without the smash types. |
| `pocket_holder_kind(boma_as_u64)` | Original owner of a pocketed weapon. |

## Parameters

Replacement: `[params]`, `NAME.param("x")`.

| Function | Scope |
|---|---|
| `param_override(kind, param, op, value)` | Float, every costume. |
| `param_override_slot(kind, slot, param, op, value)` | Float, one costume. |
| `param_override_full(kind, slot, param, subparam, op, value)` | Float, nested. |
| `param_int_override(kind, param, value)` | Integer, every costume. |
| `param_int_override_slot(kind, slot, param, value)` | Integer, one costume. |
| `param_int_override_full(kind, slot, param, subparam, value)` | Integer, nested. |

`ParamOp::Set` or `ParamOp::Mul`; `ANY_SLOT` for every costume. The
`param_motion`, `common` and `param_thrown` keys are as in
[API.md](API.md#parameters).

```rust
// (kind, costume or ANY_SLOT, param, sub field, Set or Mul, value)
param_override_full(kind, ANY_SLOT, "param_motion", "escape_air_slide_speed", ParamOp::Mul, 2.0);
param_override_full(kind, ANY_SLOT, "common", "shield_max", ParamOp::Set, 20.0);
```

## Manifest params

They still work: each writes a line of the `fighter.toml` or `item.toml` text
the manifest builds. Replacement: the same value as a param call after
`register` ([API](API.md#parameters)).

| Manifest method | Param call after `register` |
|---|---|
| `param(name, v)`, `param_int(name, v)`, `param_mul(name, v)` | `WAWA.param(name).set(v)`, `.int(v)`, `.mul(v)` |
| `params(&[..])`, `params_int(&[..])`, `params_mul(&[..])` | One call per value. |
| `param_at(costume, name, v)`, `param_int_at`, `param_mul_at` | `WAWA.param(name).slot(costume).set(v)`, `.int(v)`, `.mul(v)` |
| `common(field, v)`, `common_int(field, v)` | `ITEM.common(field).set(v)`, `.int(v)` |
| `owner_param(fighter, field, v)`, `owner_param_int` | `ITEM.owner_param(fighter, field).set(v)`, `.int(v)` |

## Articles

Replacement: `Manifest::article`, `NAME.article("x")`.

```rust
let barrel = dionysus_api::clone_article_handle(
    "koopajr",                        // source owner: the vanilla fighter the weapon belongs to
    WEAPON_KIND_KOOPAJR_CANNONBALL,   // source weapon kind
    "wawa",                           // destination owner: your resource name
    "wawa_barrel",                    // article folder under fighter/wawa/model/ and script name
    base_kind,                        // your fighter's base kind
)?;
if let Some(index) = barrel.index() {              // the table index, read right before use
    ArticleModule::generate_article(boma, index, false, 0);
}
```

| Function | Meaning |
|---|---|
| `clone_article_handle(source_owner, source_weapon, destination_owner, name, base_kind)` | Make an article, get a handle. |
| `clone_article(source_owner, source_weapon, destination_owner, name)` | Make one, get the weapon kind. |
| `clone_article_for(...)` | Same, table owner and file owner differ. |
| `article_index(fighter_kind, weapon_kind)` | Current table index. Read right before use; never fall back to 0. |
| `article_status(weapon_kind, line, status, function)` | Status code on an article. |
| `clone_copy_article_handle(...)`, `clone_copy_article(target_kind, source_owner, source_weapon, resource_owner, name)` | Article for Kirby's copy. |
| `copy_article_index(target_kind, weapon_kind)` | Current index in the Kirby copy table. |

## Kirby copies

Replacement: `Manifest::kirby*`, `NAME.arm_kirby()`.

| Function | Meaning |
|---|---|
| `arm_kirby_copy_status_family(kind, first, count)` | Publish the copy statuses, after they are installed on `Agent::new("kirby")`. |
| `clone_copy_model(kind, "directory")` | Extra copy model, up to three. |
| `clone_copy_motion(kind, &CopyMotion)` | Copy animation. `CopyMotion::new(name, file)` with `.template`, `.game_script`, `.scripts(sound, effect, expression)`, `.flags`, `.blend_frames`, `.xlu`, `.cancel_frame`, `.no_stop_intp`, `.animation_unk`, `.without_extra()`. |
| `clone_copy_mesh_default(kind, "mesh", visible)` | Starting mesh visibility, up to 16. |

## Shared hooks

Replacement: `#[hook(offset = 0x..., me = ...)]`.

| Function | Meaning |
|---|---|
| `shared_hook_checked(&SharedHookRegistrationV1)` | Register a callback. |
| `shared_hook(offset, callback)` | Register on an allowlisted address, no fingerprint. |
| `shared_hook_status()` | Readiness and failure flags. |
| `shared_hook_original(offset, &[u64; 6])` | Run the original once. |
| `shared_hook_words(offset)` | The four instruction words recorded at `offset` when its hook went in. |

Callbacks: `x0..x5` in, `x0` out; return `HOOK_DECLINED` or `HOOK_HANDLED`.

## Items

Replacement: `ItemManifest` + `ITEM.register`, or `item.toml`.

```rust
use dionysus_api::{CloneItemKind, ItemCloneRegistration, ItemStatusLine, KIND_AUTO};

static MY_ITEM: CloneItemKind = CloneItemKind::new("my_item"); // holds the kind against the resource name

let kind = dionysus_api::allocate_item(&ItemCloneRegistration::new(
    KIND_AUTO, // let the engine pick the kind
    0x32,      // base kind: the vanilla item
    "my_item", // resource name: files under item/my_item
    "my_item", // script name
))?;
MY_ITEM.store(kind);
// (kind, status line, status name of the base item, function)
dionysus_api::item_status_named(kind, ItemStatusLine::Update, "WAIT", my_wait_update as *const ())?;
// (fighter, the kind wrapped in the game's type, the game's own arguments)
ItemModule::have_item(boma, smash::app::ItemKind(MY_ITEM.raw()), 0, 0, false, false);
```

| Function | Meaning |
|---|---|
| `allocate_item(&ItemCloneRegistration)` | Register with `KIND_AUTO`, get the kind. `ItemCloneRegistration::new(kind, base_kind, resource_name, agent_name)`. |
| `register_item(&ItemCloneRegistration)` | Register with a kind you chose. |
| `register_item_ui(&ItemUiRegistration)` | Training menu cell. |
| `CloneItemKind::new(resource_name)`, `.store`, `.get`, `.raw`, `.is` | Holds the kind. |
| `item_status_named(kind, line, name, function)` | Status callback by name, at startup. |
| `item_status(kind, line, status, function)` | Same with a number; 0 refused. |
| `item_status_kind(name)` | Resolve a status name. Fails until the item module's table exists. |
| `item_common_set(kind, hash, value)`, `item_common_set_i32(kind, hash, value)` | A common-file float or int, for this item only. Now `v2::item_common(kind, "field").set(v)` or `.int(v)`. |
| `item_common_set_label(kind, hash, label)`, `item_common_set_hash(kind, hash, name)` | A kind field by label, a bone or motion name field. Now `.label("...")` and `.hash("...")` on `v2::item_common`. |
| `item_owner_param_set_f32(kind, owner_kind, offset, value)`, `item_owner_param_set_i32` | A word of the owner fighter's `vl.prc` by byte offset (multiple of 4, below `0x4000`), for this item only. Now `v2::item_owner_param(kind, "fighter", "field").set(v)`. |
| `item_generate_add(kind, generator, per, min, max, variation)` | Natural or container drops. `generator`: `hash40("item_genid_random")` or `hash40("item_kind_<container>")`; `variation`: `ITEM_VARIATION_AUTO`. |

```rust
// (item kind, owner fighter kind (0x58 = Steve), byte offset of the field in its vl.prc (0x518 = life), value)
dionysus_api::item_owner_param_set_i32(MY_ITEM.raw(), 0x58, 0x518, 600)?;
// (item kind, generator, weight, min at once, max at once, variation)
dionysus_api::item_generate_add(MY_ITEM.raw(), hash40("item_genid_random"), 30, 1, 1, ITEM_VARIATION_AUTO)?;
```

## Stages

Replacement: `stage.toml`.

| Function | Meaning |
|---|---|
| `stage_capacity()` | Remaining places and StageIDs. |
| `allocate_stage(&StageAllocation)` | Create a place and its forms. |
| `set_stage_behaviour(place, donor)` | Vanilla stage class and gimmicks. Before `register_stage`. |
| `set_stage_music(place, &StageMusic)` | Playlist, column, album selector. Before `register_stage`. |
| `stage_id_for(place, form)` | StageID of one form. |
| `register_stage(&StageRegistration)` | Name, series, order, stage select entry. |

Needs `CAP_STAGE_MINT`, `CAP_STAGE_CONFIG`, `CAP_STAGE_SELECT_EXTENDED`,
`CAP_STAGE_CSK`.
