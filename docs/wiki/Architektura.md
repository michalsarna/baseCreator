🌐 [English](Architecture) · **Polski**

# Architektura

BaseCreator to jedna skrzynia wykonywalna, `crates/basecreator`, w repozytorium swoich bibliotek: `base-core` (geometria, scena, czytniki SVG / PDF / DXF / LightBurn / UGS, kontener projektu) i `base-ui` (okno, menu, edytor, panele i karty, cliparty, wtyczki). LightCreator, MillCreator i PenCreator używają tych samych bibliotek, przypiętych tagiem git.

| Plik | Co zawiera |
|---|---|
| `main.rs` | Program: otwieranie, zapis, import, ekran startowy, paski, boczny panel, klawisze, okna. |
| `menu.rs` | Pozycje menu (układ to `standard_menus` z base-ui). |
| `native_menu.rs` | Pasek menu macOS. |
| `workspaces.rs` | Otwarte rysunki jako karty. |
| `panes_glue.rs` | Panele obszaru roboczego. |
| `section.rs` | Sekcja `draw` pliku projektu: strona i warstwy. |
| `page_ui.rs` | **Ustawienia strony…** |
| `layers_ui.rs` | Karta Warstwy. |
| `plugins_glue.rs`, `autosave_glue.rs`, `backup_ui.rs`, `update_glue.rs` | Wtyczki, automatyczny zapis, kopia zapasowa i sprawdzanie aktualizacji, połączone z programem. |
| `texts.rs` | Własne tłumaczenia BaseCreatora. |
| `screenshots.rs` | Renderowanie obrazów. |
| `tests.rs` | Testy programu. |
