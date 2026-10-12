# baseCreator

Shared code of [LightCreator](https://github.com/michalsarna/LightCreator) (laser) and
[MillCreator](https://github.com/michalsarna/MillCreator) (CNC mill). It started as the part of LightCreator that does
not need a laser, and it grows by moving more of that part here, one piece at a time.

| Crate | What it holds |
|---|---|
| `base-core` | No GUI. Geometry, Bézier paths, arcs and fillets, snapping, boolean operations, text outlines, bitmaps and tracing, camera calibration, the **scene** (`Scene`, `Shape`, `Kind`: shapes with transforms, groups and ids), SVG and PDF reading, the clipart library, units, GRBL settings reports, ser2net helpers |
| `base-ui` | egui parts: colour schemes (`theme`), fonts, the translation layer (`i18n`), a Markdown viewer, unit-aware number fields, camera helpers (feature `camera`) |

What is **not** here yet: the work-area canvas, the node editor, the menus and the dialogs. They are written against
LightCreator's own state (layers, device, action list) and need a host interface first.

## Using it

An application depends on a **tag** of this repository:

```toml
base-core = { git = "https://github.com/michalsarna/baseCreator", tag = "v0.1.0" }
base-ui   = { git = "https://github.com/michalsarna/baseCreator", tag = "v0.1.0" }
```

### Working on the base and an application at the same time

Point Cargo at your checkout of this repository. Put this in `.cargo/config.toml` of the application (add the file to
`.gitignore`) or in `~/.cargo/config.toml`, and do not commit it:

```toml
[patch."https://github.com/michalsarna/baseCreator"]
base-core = { path = "/path/to/baseCreator/crates/base-core" }
base-ui   = { path = "/path/to/baseCreator/crates/base-ui" }
```

### Translations

`base_ui::i18n` keeps the API `tr("English text")`, `trf`, `set_lang`. The texts live in `Texts` sets: an application builds
one (`Texts::new(table, keys, files)`) and registers it with `i18n::register`; `tr` asks the registered sets in order
and then the set of the base, which is empty for now.

## Versions

Semantic versioning with tags `vX.Y.Z` (while the major number is 0, a new minor number may break things).
A change here means a new tag here first. Each application then moves its tag in a pull request of its own. A tag
here never triggers a release of an application.

Tags `-alpha.N` were used while LightCreator was being moved over; use the plain tags.

Licence: MIT OR Apache-2.0, as LightCreator. See `CREDITS.md`.
