# Articles and Kirby

## Articles

Declare one in the manifest:

```rust
Manifest::new("my_fighter", "samus")
    .article(
        "beam",        // folder fighter/my_fighter/model/beam, scripts under "my_fighter_beam"
        "samus/cshot", // the vanilla article it is copied from: fighter/weapon
    )
```

or in `fighter.toml`:

```toml
[[article]]
name = "beam"          # files under fighter/my_fighter/model/beam
from = "samus/cshot"   # the vanilla article it is copied from
```

Its scripts go under the name `<fighter>_<article>`:

```rust
smashline::Agent::new("my_fighter_beam")                              // the article's own name
    .game_acmd("game_fly", game_fly, smashline::Priority::Default)     // script name, function, priority
    .status(smashline::Main, *WEAPON_SAMUS_CSHOT_STATUS_KIND_FLY, beam_main) // status line, status kind, function
    .install();
```

Statuses may use numbers past the source weapon's own. A `kirby_article`'s
scripts and statuses go under `kirby_<name>`. `MY_FIGHTER.article("beam").status(...)`
from earlier releases still works; it installs through Smashline the same way.

Each article needs a folder of its own. Read
`MY_FIGHTER.article("beam").index()` right before you use it, never earlier,
and never fall back to 0. `MY_FIGHTER.article("beam").spawn(boma)` does both.

The weapon kind is assigned at boot. With Smashline 2.0, Smashline numbers
it, shared with other mods' `clone_weapon` calls; otherwise the engine does.
Read `MY_FIGHTER.article("beam").weapon_kind()`; never assume `615 + n`.
Under Smashline 2.0 a base fighter of kind 94 or above takes no articles.

Once built, an article shows the game the weapon kind of the article it was
copied from, so the game's code and the source's own scripts treat it as that
article (Example : a copy of Toon Link's arrow turns in flight and sticks in fighters as
the original one does). On an article, `utility::get_kind` answers the source's kind and
`dionysus_api::article_kind(boma)` answers its own. ParamConfig still sees its
own. `article_kind_facade = false` in `sd:/ultimate/dionysus/settings.toml`
turns this off.

## Articles with collision

An article with an `.lvd` file keeps the base's file name inside your folder:

```text
fighter/<your fighter>/model/<your article>/c00/<base article name>.lvd
```

Renamed, it loads with no collision.

A costume folder without its own `.lvd` uses the one from the nearest costume
that has one. Several articles made from the same base article each use their
own file.

### Platforms on any article

Any article made with `from` can carry a platform from an `.lvd` file of your own, whatever its
source. Each vertex you list follows a joint every frame:

```rust
use dionysus_api::v2::ArticleLvd;

Manifest::new("my_fighter", "samus")
    .article("plank", "koopajr/cannonball")
    .article_lvd(
        "plank",
        ArticleLvd::new("plank.lvd")                 // in the article's model folder
            .vertex_at(0, "top", 0.0, 10.0, -8.0)    // vertex 0: 10 up, 8 behind
            .vertex_at(1, "top", 0.0, 10.0, 8.0),    // vertex 1: 10 up, 8 ahead
    )
```

or in `fighter.toml`:

```toml
[[article]]
name = "plank"
from = "koopajr/cannonball"
lvd = "plank.lvd"

[[article.lvd_vertex]]
vertex = 0
joint = "top"
offset = [0, 10, -8]

[[article.lvd_vertex]]
vertex = 1
joint = "top"
offset = [0, 10, 8]
```

The file goes in `fighter/<your fighter>/model/<your article>/cNN/`, under any name, and in your
pack's config like the article's other files. The article uses the costume of the fighter that
owns it, or the nearest costume that ships the file.

| Key | Values | Default |
|---|---|---|
| `vertex` | a vertex index in the file | required |
| `joint` | a joint of the article's model, or `top` | required |
| `offset` | `[x, y, z]` in the joint's own axes (on `top`, y is up and z points the way the article faces) | the joint itself |
| `joint_left` | the joint this vertex follows while the article faces left | the list's joints in reverse order |

Facing left, the vertices take their joints in reverse order, so a floor drawn left to right
stays face up. A vertex you do not list stays where the file puts it, in stage coordinates.
Up to 32 vertices. The platform is on from the moment the article appears; status scripts can
switch it with `GroundModule::set_gr_collision_mode`. The log names the file, the costume and
where each vertex landed.

## Hurtboxes

An article made with `from` can be given hurtboxes, on top of any its source already has.
In the manifest:

```rust
use dionysus_api::v2::Hurtbox;

Manifest::new("my_fighter", "samus")
    .article("rock", "koopajr/cannonball")
    .article_hurtbox(
        "rock",                          // the article
        Hurtbox::new("rot", 4.0)         // joint of the article's model, radius
            .offset(0.0, 2.0, 0.0)       // from the joint
            .offset2(0.0, 6.0, 0.0),     // second end: makes a capsule
    )
```

or in `fighter.toml`, under the article:

```toml
[[article]]
name = "rock"
from = "koopajr/cannonball"

[[article.hurtbox]]
node = "rot"            # joint of the article's model
size = 4.0              # radius
offset = [0, 2, 0]      # from the joint
offset2 = [0, 6, 0]     # optional second end; with it the shape is a capsule
```

| Key | Values | Default |
|---|---|---|
| `node` | a joint of the article's model | required |
| `size` | radius | required |
| `offset`, `offset2` | `[x, y, z]` | `[0, 0, 0]`, none |
| `part` | `body`, `etc`, `legs`, `body_legs`, `head` | `etc` |
| `height` | `low`, `center`, `high` | `center` |
| `status` | `normal`, `invincible`, `xlu`, `off` | `normal` |
| `shape` | `sphere`, `aabb`, `capsule` | `capsule` with `offset2`, else `sphere` |

The names also accept the game's constant names (`COLLISION_PART_HEAD`) and plain numbers.
Up to 16 per article. What a hit does is up to the article's status scripts.
`custom_fighters/template_v2` puts one on its fireball.

## Kirby copies

Reserve the status numbers the copied move needs:

```rust
Manifest::new("my_fighter", "mario")
    .kirby(1)          // how many status numbers the copied move needs
```

```toml
[kirby]
statuses = 1
```

Read them back with `kirby_status(n)`, register the scripts on Kirby, then
arm:

```rust
smashline::Agent::new("kirby")                                  // Kirby's copy scripts go on Kirby
    .status(Main, MY_FIGHTER.kirby_status(0), kirby_special_n)  // status line, the first reserved number, function
    .install();
MY_FIGHTER.arm_kirby();                                         // after the statuses are installed
```

`arm_kirby` must come after the statuses are installed.

### Hooks that run on Kirby

Your fighter's `slot` and `common_slot` hooks never run for the copy: the
object playing it is Kirby's, a vanilla kind. `copy_slot` registers the entry
Kirby runs while he holds your ability.

```rust
#[hook(copy_slot = slot::common::SYS_LINE_STATUS_END_CONTROL)]
unsafe fn kirby_status_end(agent: u64) -> L2CValue {  // agent is KIRBY's agent, not yours
    call_original!(agent)                             // Kirby's own entry, also when no copy is held
}
```

Slots 10 to 14 only; 10 and 12 return an `L2CValue`, 11, 13 and 14 a `u64`.
Two Kirbys holding two different copies each get their own. Slot 10 runs once as Kirby's agent starts,
before he holds any copy, so a copy hook there never fires; slot 12 runs at the
end of every status. See [Common vtables](../API.md#common-vtables).

The copy model, hat or full body, goes in
`fighter/kirby/model/copy_<your name>_fitkirby/cNN/`, and the pack's
`config.json` declares the copy files under `fighter/<your name>/kirbycopy/cNN`.
Do not create empty `kirbycopy/cNN` groups and do not fill them with the base
fighter's Kirby files; both make the game's file cache fail.

`.kirby_full_model()` (`full_model = true`) gives Kirby your whole body
instead of a hat. It needs a complete model and animation set for every Kirby
colour.

### More than one model

`.kirby_model("my_fighter_kirby_extra")` (`model = "..."` in `[kirby]`)
declares one extra model folder under `fighter/kirby/model/`. Anything in a
model you did not declare is never loaded.

### Your own copy animations

Only the animations you change; the rest come from the base fighter:

```rust
use dionysus_api::v2::Motion;

Manifest::new("my_fighter", "mario")
    .kirby_motion(
        Motion::new(
            "my_fighter_special_n",         // motion name; must start with your name
            "myfighterd00specialn.nuanmb",  // animation file
        )
        .template("mario_special_n")        // borrow flags and scripts from this copy motion
        .scripts("myfighterspecialn"),      // game_, sound_, effect_, expression_ scripts, on Agent::new("kirby")
    )
```

```toml
[[kirby.motion]]
name = "my_fighter_special_n"
animation = "myfighterd00specialn.nuanmb"
template = "mario_special_n"
scripts = "myfighterspecialn"
```

The animation file goes in `fighter/kirby/motion/<your name>body/c00/` and is
listed in `new-dir-files` under your `kirbycopy/cNN/bodymotion` group. A
motion with a template and no scripts keeps the template's hitboxes.

### Copy sounds

While Kirby holds your copy, `PLAY_SE` and `PLAY_SEQUENCE` play any label of
two banks; labels from other banks make no sound.

- Your `se_<your name>_cNN`, from the status after the swallow on.
- Your copy voice bank, `sound/bank/fighter_voice/vc_kirby_copy_<your name>`
  (`.nus3audio`, `.nus3bank`, `.tonelabel`), listed under every
  `kirbycopy/cNN` group. Without it Kirby uses the base's copy voices.

### Hiding a hat piece from the start

```rust
Manifest::new("my_fighter", "mario")
    .kirby_mesh("wing", false)   // mesh name, visible at the start
```

```toml
[[kirby.mesh]]
name = "wing"
visible = false
```

Your animations still control the mesh once they play.
