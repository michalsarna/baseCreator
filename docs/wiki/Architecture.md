🌐 **English** · [Polski](Architektura)

# Architecture

BaseCreator is one binary crate, `crates/basecreator`, in the repository of its libraries: `base-core` (geometry, the scene, the SVG / PDF / DXF / LightBurn / UGS readers, the project container) and `base-ui` (the window, the menus, the editor, the panes and tabs, clipart, plugins). LightCreator, MillCreator and PenCreator use the same libraries, pinned by a git tag.

| File | What it holds |
|---|---|
| `main.rs` | The application: open, save, import, the start screen, the bars, the side panel, the keys, the windows. |
| `menu.rs` | The entries of the menus (the layout is `standard_menus` of base-ui). |
| `native_menu.rs` | The macOS menu bar. |
| `workspaces.rs` | The open drawings as tabs. |
| `panes_glue.rs` | The panes of the work area. |
| `section.rs` | The `draw` section of the project file: the page and the layers. |
| `page_ui.rs` | **Page setup…** |
| `layers_ui.rs` | The Layers tab. |
| `plugins_glue.rs`, `autosave_glue.rs`, `backup_ui.rs`, `update_glue.rs` | Plugins, automatic saving, backup and the update check, joined to the program. |
| `texts.rs` | BaseCreator's own translations. |
| `screenshots.rs` | The renderer of the pictures. |
| `tests.rs` | The tests of the program. |
