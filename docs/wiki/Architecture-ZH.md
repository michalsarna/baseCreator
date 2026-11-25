🌐 [English](Architecture) · [Polski](Architektura) · **简体中文** · [한국어](Architecture-KO)

# 架构

BaseCreator 是一个二进制 crate，`crates/basecreator`，位于其库所在的仓库中：`base-core`（几何、场景、SVG / PDF / DXF / LightBurn / UGS 读取器、项目容器）和 `base-ui`（窗口、菜单、编辑器、窗格和标签页、剪贴画、插件）。LightCreator、MillCreator 和 PenCreator 使用同样的库，并通过 git 标签固定版本。

| 文件 | 内容 |
|---|---|
| `main.rs` | 应用程序：打开、保存、导入、启动界面、各工具条、侧边面板、快捷键、各窗口。 |
| `menu.rs` | 菜单条目（布局来自 base-ui 的 `standard_menus`）。 |
| `native_menu.rs` | macOS 菜单栏。 |
| `workspaces.rs` | 以标签页形式打开的图纸。 |
| `panes_glue.rs` | 工作区的窗格。 |
| `section.rs` | 项目文件的 `draw` 部分：页面和图层。 |
| `page_ui.rs` | **页面设置…**（**Page setup…**） |
| `layers_ui.rs` | 图层标签页。 |
| `plugins_glue.rs`、`autosave_glue.rs`、`backup_ui.rs`、`update_glue.rs` | 插件、自动保存、备份和更新检查与程序的衔接。 |
| `texts.rs` | BaseCreator 自己的翻译。 |
| `screenshots.rs` | 截图的渲染器。 |
| `tests.rs` | 程序的测试。 |
