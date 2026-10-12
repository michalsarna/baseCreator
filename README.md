# baseCreator

Shared code of [LightCreator](https://github.com/michalsarna/LightCreator) (laser) and
[MillCreator](https://github.com/michalsarna/MillCreator) (CNC mill). Early stage: the crates are being split out of
LightCreator step by step, and nothing here is meant to be used on its own yet.

| Crate | What it holds |
|---|---|
| `base-core` | No GUI: geometry, Bezier paths, arcs and fillets, snapping, the scene (shapes, groups, ids), SVG and PDF reading, the clipart library, boolean operations, text outlines, bitmaps, tracing, camera calibration, GRBL settings reports, ser2net helpers |
| `base-ui` | egui parts shared by the applications: colour schemes, fonts, the translation layer (`i18n`, with `register` for an application's own texts), a Markdown viewer, unit-aware number fields, camera helpers (feature `camera`) |

The applications depend on a tag of this repository (`tag = "v0.1.0"`); a change here means a new tag here first, then a
separate pull request in each application that moves its tag.

Licence: MIT OR Apache-2.0, as LightCreator.
