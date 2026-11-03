# BaseCreator: versions

The releases of the program [BaseCreator](docs/BaseCreator.md) have the numbers of the tags of this repository. Not every tag is a release:
a tag can be made only for LightCreator, MillCreator and PenCreator. Here is what is new for the people who use BaseCreator.

## v1.65.1

- **F1** opens the quick guide in every program: a function key now works alone in `menu::shortcut_act` (the menu showed F1, but only LightCreator had the key).
- The texts of the check of the program are in the shared table, so LightCreator shows them translated too.
- `tool_check::check_windows` presses F1 and Ctrl+P, and `check_guide_paths` takes the language of the guide, so the Polish guides are checked against the Polish menus.
- A simulator test: a job sent right after the check mode ends is not lost.

## v1.65.0

- Printing is shared: `base_core::print` (the PDF, from shapes and the look of their layers) and `base_ui::print_ui` (the window). BaseCreator has **File → Print…** now.
- The quick guide in BaseCreator (Help, F1). `tool_check::check_guide_paths`: every `Menu > Entry` a guide names must exist in the menus of the program.
- `menu::machine_menu`: the same Machine menu, in the same order, in every program with a machine. `ser2net_ui` is shared too.
- `tools_ui::diagnostics_view` works from plain data, so a program that does not use `Machine` can have the window; `export_diagnostics` can be called on its own.
- The picture under the drawing is a camera overlay where the program has a camera: the live picture, reference points kept with the device, flip and rotate. It is saved with the project, in the `overlay` section that LightCreator already wrote (`base_core::overlay_data`), so a project brings its picture along to any program.
- **Fix:** a stop during a check run no longer turns the check mode of GRBL on again. The stop itself ends it (checked in the simulator), and the `$C` sent after it would have kept the next job from moving.
- `tool_check::check_windows`: the windows a program has of its own, opened through its menus.

## v1.64.1

- Esc closes the question when a tab is closed and the G-code view again, and cancels picking a place on the work area (it was taken by the keys of the editor in v1.64.0).

## v1.64.0

The audit of the tools of all the programs: the same things now work the same way in BaseCreator, MillCreator, PenCreator and LightCreator.

- **Polygon and spiral:** the small window with the number of sides or turns was in LightCreator only, so these shapes could not be changed elsewhere. It is in the editor now, and a change also applies to the shape drawn last.
- **Keys:** Space holds the hand, the arrows move the selection (Shift: 10 mm), a tool's letter also ends a line being drawn and opens the window of the polygon or spiral, and Esc clears the chosen nodes first in node editing.
- **Union, Intersection, Subtract and Exclusive or** are buttons of the tool palette in every program.
- The lock on a locked shape is the same icon everywhere. Menu and window texts that were English in some programs (*Arrange*, *Save G-code*) are translated.
- New: `editor::{hold_space, plain_keys}`, `tool_check` (feature `tool-checks`: a check of every tool through the real window, run by every program); `tool_palette` has no `between` any more.

## v1.63.0

- **The text tool works in BaseCreator, MillCreator and PenCreator:** a click with **Text (T)** did nothing there. It puts a text on the page and the Properties tab opens with the field to type in, the font, size, style, spacing and alignment, and **Convert to curves**. It is one shared code now, also for LightCreator.
- New: `editor::text_ui`, `EditorHost::show_properties_tab`; `EditorHost::place_text` works by default.

## v1.62.0

For the programs with a machine; in BaseCreator only *View options* changes.

- **The console remembers your commands:** the Up and Down keys in its field go through what you sent from the console, and only that; what you were typing comes back at the end.
- **Your commands have a colour of their own**, so they stand out from what the program sends.
- *Settings → View options…* has a new **G-code console** part: the font size and the colours. The default look of the other lines is as before.
- BaseCreator's *View options* no longer shows the switches about G-code.
- `ConsoleState::show` takes a `&ConsoleLook` (from `GridPrefs::console`); new: `Dir::User`, `EditorHost::has_machine`.

## v1.61.0

This release also brings what is below under v1.60.1 and v1.60.0: the same file formats in every program, a wiki and package managers.

- **Festive icons:** around Christmas, the New Year, Halloween and Easter the program wears an icon of the holiday, as the other programs do. The festive colour schemes were there already.
- **A plugin that changes G-code cannot be installed here**, as there is no G-code: the official list says *Not for this program* and a `.zip` is refused. One that another program installed in the shared folder does nothing here and is not offered for updates. New: `Registry::refuse_gcode`, `plugins::WITHOUT_GCODE`, `Entry::refused_by`, `Index::updates_for`.

## v1.60.1

**A wiki and package managers.**

- The program has its own [wiki](https://github.com/michalsarna/baseCreator/wiki), in English and Polish.
- Install it with Homebrew (`brew install --cask michalsarna/tap/basecreator`) or Scoop (`scoop install basecreator`, after `scoop bucket add creator https://github.com/michalsarna/scoop-bucket`). It has been submitted to winget, and an AUR package is prepared.

## v1.60.0

**The same file formats in every program.** LightCreator, MillCreator, PenCreator and BaseCreator now have one list in *File → Import* and *File → Export*; it comes from `standard_menus`, so a program cannot miss one.

- *Import:* image, SVG (`Ctrl+I`), Adobe Illustrator / PDF, DXF, LightBurn project, UGS design. New in BaseCreator: SVG, Illustrator / PDF and UGS. Importing an SVG or a PDF adds its paths to the drawing that is open, on the layer of their colour, and can be undone; *Open…* still makes a new tab. A file with Illustrator layers asks whether to go by colour or by those layers.
- *Export:* SVG and PLT / DXF of the whole drawing, and the selection as SVG or DXF. New: HPGL (`.plt`), one pen per layer.
- What belongs to one program stays there: the 3D model and the relief STL in MillCreator, G-code in the programs with a machine.
- `MenuParts`: `import_svg`, `import_ai`, `import_dxf`, `import_lightburn`, `import_ugsd`, `export_svg`, `export_cam`, `import_extra`, `export_first`, `export_extra` in place of `import` and `export`. New: `base_core::svg::{into_scene, read_drawing}`, `base_core::export::{hpgl, pictures_in}`, `EditorImages::import_drawing_path`, `EditorShapeOps::{export_drawing_svg, export_drawing_cam}`.

## v1.59.0

The first release of BaseCreator: a drawing program with nothing of a machine, for macOS, Windows and Linux.

- Draw and edit: shapes, nodes, Bézier lines, text, grouping, aligning, boolean operations, offset, rounded corners, pictures and tracing, the clipart gallery.
- **Project files `.cre` shared with LightCreator, MillCreator and PenCreator:** a project made in one of them opens here, and what belongs to that program stays in the file after saving here.
- **A page in place of a work area:** *File → Page setup…* has the usual sheets, a size of your own, portrait and landscape, mm or inches. Every drawing keeps its page.
- Layers are groups with a name and a colour, and can be hidden.
- Import SVG, PDF, Illustrator, DXF, LightBurn, PNG and JPEG; export the drawing or the selection as SVG or DXF. Pictures are not exported.
- Plugins from the official list: generators and importers work; plugins with G-code postprocessors can be installed, but do nothing here, as there is no G-code.
- *Help → Check for updates…* and a check at the start, which can be switched off in *Settings → Program options…*.
- 30 languages, colour schemes, tabs and panes, automatic saving and backups, as in the other programs.
