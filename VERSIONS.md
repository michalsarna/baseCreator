# BaseCreator: versions

The releases of the program [BaseCreator](docs/BaseCreator.md) have the numbers of the tags of this repository. Not every tag is a release:
a tag can be made only for LightCreator, MillCreator and PenCreator. Here is what is new for the people who use BaseCreator.

## v1.71.5

- **Windows of their own look like the main window:** a tab taken out into a window of its own (the preview, the camera of MillCreator and PenCreator…) has no frame of the system any more (so no rounded corners on macOS), but the same flat bar across the top as the main window, with the icon, the title and the minimise, maximise and close buttons, and it is resized by its edges. Close puts the tab back, as before. `window::own_window` and `own_window_builder` are for the programs' own windows too (the camera view of LightCreator); `PaneHost::window_logo` gives the icon.

## v1.71.4

- **One zoom bar for every picture:** zoom in, zoom out and the whole picture, with the zoom (`×1.5`), on a small bar in the lower left corner of the picture, as in the image import window: now also in the trace window (the buttons were in its column of settings) and in the G-code viewer (it had a **Reset view** button in its toolbar). `zoom_bar::overlay` is for the programs' own previews too.

## v1.71.3

- **Paste a picture:** Ctrl/Cmd+V or **Edit → Paste** with a picture in the clipboard of the system (a screenshot, a picture copied in a browser or a graphics program) opens the usual import window with it. Shapes copied in the program are still pasted as shapes, unless a picture was copied after them.
- Ctrl+V over the drawing pastes the copied shapes on Windows and Linux too: the window system hands the key over as a paste, which the drawing did not take.

## v1.71.2

- **Import and adjust an image:** the picture zooms and moves as in the trace window: the wheel zooms about the pointer, a drag moves it, a double click shows it whole, and the buttons in its corner zoom in, zoom out and show the whole picture (`PictureView`, shared by both windows).

## v1.71.1

- **The list of fonts of a text:** with the list open, the up and down arrows go through it and change the font of the text at once, and the list scrolls to the font; the list shows ten fonts at a time.

## v1.71.0

- **grblHAL and FluidNC:** the start-up line of grblHAL (`GrblHAL 1.1f …`) is known, so a job no longer waits 2.5 s after the reset of the connection (FluidNC's `Grbl 3.7 [FluidNC …]` was known already). `$I` of grblHAL is read whole: `[NEWOPT:…]` (what it can do besides GRBL 1.1, `FirmwareInfo::extensions`), `[FIRMWARE:grblHAL]`, `[AXS:…]`, `[BOARD:…]`, `[DRIVER:…]`; grblHAL is recognised by them even when the version line does not name it.
- Once grblHAL is recognised it is asked once for its complete status report (`0x87`), so the work offset and the overrides are known at once instead of after a few reports.
- The extra fields of grblHAL's report: homed since it started (`H:`) and the tool in the spindle (`T:`); the Control tab shows **Not homed** and **Tool n**. Its `Tool` state counts as working, so a tool change is not reported as a controller that stopped answering.
- `FirmwareInfo` and `grbl_status::Report` have new fields: struct literals in the programs need `..Default::default()`.


- **The image import window keeps its size:** in Polish (and other languages with longer words) the row of the rotate and flip buttons was wider than its column, and the window grew a little every frame up to the width of the screen. The columns of the import and trace windows now have fixed places, and the row breaks into two lines when it has to.

## v1.69.6

- **The system clipboard:** BaseCreator was built without it, so nothing could be pasted from another program and nothing copied into one. The programs on this base need the `clipboard` feature of eframe too.
- The address of a camera: what is pasted loses the line breaks and spaces around it, and an empty one asks for an address (`camera_feed::address_field`, `lacks_address`).

## v1.69.5

- A camera of the computer opens on macOS: the library throws an exception out of some formats, not always the same ones, so each try is guarded and smaller sizes follow.

## v1.69.4

- **Edit → Copy and Paste** of the menu work in a text field; they copied and pasted shapes of the drawing.
- The status line can be copied.

## v1.69.3

- The window of the control devices has two lists, what is available in the system and what is configured, with arrows between them, and a choice of the default device with *No default device*. A device taken off the list keeps its keys in case it comes back.
- **One page of the cameras** of a device and one test window (`camera_feed::camera_page`, `CamTest`), which LightCreator had to itself.
- **Messages can be copied:** `widgets::message`, selectable and with Copy on the right button.
- **macOS:** a connection or a camera on the local network that the system refuses now says where to allow the program.

## v1.69.2

- The window of the control devices fits on the screen.

## v1.69.1

- The window of the control devices: the keys of a device stand on one line.

## v1.69.0

- For the USB keyboards and pads of LightCreator, MillCreator and PenCreator: the definitions, the keys and the logic of a press (`base_core::control_devices`), reading the pads with `gilrs`, moves that last while a key is held, the windows of the settings (`base_ui::control_devices_ui`) and a submenu of the Machine menu. BaseCreator has no machine: it changes nothing there, but building it on Linux now needs `libudev-dev`.

## v1.68.0

- **New colour schemes:** Superman, Batman, and Joker in a dark and a light version, in the menu and on the start screen, 18 in all.

## v1.67.2

- **Start** is the word *Start* in every language, on the Start screen and in the text on a path.

## v1.67.1

- **Polish names:** the Start screen says *Start* (it said *Początek*), and *Export* in the File menu is *Eksportuj*, like *Importuj*. `tool_check::check_polish_names` keeps Start, Export, Import and Opacity the same in every program.
- For layer presets in the backups of LightCreator, MillCreator and PenCreator: `layer_presets_ui::backup_pair` and `Library::replace_text`. BaseCreator has no layer presets.

## v1.67.0

- **One quick guide:** the sections on the window, drawing, import, the gallery, the machine and the keys are in `base_ui::guide`, in English and Polish, and every program adds its own sections around them. BaseCreator's guide now explains node editing, rounding corners, locked objects, text and the clipart gallery.

## v1.66.0

- The window of the picture under the drawing closes its corner handles with it.
- For LightCreator's move to the shared overlay: hooks for corners kept with the device (`overlay_calibration`, `overlay_changed`), for the choice of a camera in the window (`overlay_camera_ui`, `overlay_is_camera`, `overlay_can_start_camera`), and `Background::data`.

## v1.65.3

- **Fix:** symbols that no font of the programs has showed as empty boxes: the delete, rename and confirm buttons of layer presets, the delete button of saved positions and of the rows of a CSV file, and the mark in front of the explanation of an alarm or error in the console. egui 0.37 ships its symbols only on request. They now use characters of the fonts and the icons of the programs.
- `tool_check::check_symbols` looks at every character in the string literals of a program against its fonts.

## v1.65.2

- **Negative image** is in Properties of a selected picture in every program (it was there only in LightCreator; the import and adjust window has had it everywhere). It can be undone and it does not change a locked picture. The tip no longer speaks of engraving, because a plotter draws and BaseCreator only keeps the picture.
- The text of the *Load preset* button is in the shared table, for LightCreator's new layer presets.

## v1.65.1

- **F1** opens the quick guide in every program: a function key now works alone in `menu::shortcut_act` (the menu showed F1, but only LightCreator had the key).
- The texts of the check of the program are in the shared table, so LightCreator shows them translated too.
- `tool_check::check_windows` presses F1 and Ctrl+P, and `check_guide_paths` takes the language of the guide, so the Polish guides are checked against the Polish menus.
- **Fix:** a job sent right after the check mode ends is no longer lost. Leaving check mode restarts GRBL, and the link now waits for the banner as it does after a stop. With *Check the program before every run* the job could hang at 0 lines. A simulator test covers it; it failed on CI before this fix.

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
## v1.70.0

- **Plotters that do not speak G-code:** the AxiDraw and the EggBot (EiBotBoard, EBB) and HPGL plotters and cutters on a serial port. The programs make G-code as for a GRBL; the link turns every line into the plotter's own commands (`plotter::Translator`), so the buttons, the progress, the pauses at a pen change, the hold and the stop work the same way.
  - EBB: one command at a time, each answered with `OK`; the motors of the AxiDraw are mixed (X + Y, X − Y), 80 steps per mm, the pen heights in percent (`SC,4`, `SC,5`), at most 24 steps a millisecond. The EggBot's motor 1 is the pen arm (Y) and motor 2 the egg (X), as evil-mad's plotink sends them. The board's `OK` means a move is queued, so before a pause the link asks it (`QM`) until its motors stand. A stop drops the board's queue (`ES`), lifts the pen and reads where the motors are (`QS`).
  - HPGL: absolute plotter units (40 per mm), `PU`/`PD`/`PA`, the speed with `VS`; the port's flow control (RTS/CTS or XON/XOFF) keeps the plotter's buffer from overflowing; a plotter that takes nothing for 20 s is reported once.
  - Offered as `GrblVersion::PLOTTERS` (after `ALL`, which stays the same for the laser and the milling machine); the settings are `Connection::plotter` and `plotter_ui::settings_rows`.
  - A plotter is never asked for `$I`, `$$` or `$G`, a check run is skipped (it would draw), jogs are plain moves.
  - Not tried on a real AxiDraw or HPGL plotter yet: only against stand-ins in the tests.

## v1.69.7

- **The image import window keeps its size:** in Polish (and other languages with longer words) the row of the rotate and flip buttons was wider than its column, and the window grew a little every frame up to the width of the screen. The columns of the import and trace windows now have fixed places, and the row breaks into two lines when it has to.

## v1.69.6

- **The system clipboard:** BaseCreator was built without it, so nothing could be pasted from another program and nothing copied into one. The programs on this base need the `clipboard` feature of eframe too.
- The address of a camera: what is pasted loses the line breaks and spaces around it, and an empty one asks for an address (`camera_feed::address_field`, `lacks_address`).

## v1.69.5

- A camera of the computer opens on macOS: the library throws an exception out of some formats, not always the same ones, so each try is guarded and smaller sizes follow.

## v1.69.4

- **Edit → Copy and Paste** of the menu work in a text field; they copied and pasted shapes of the drawing.
- The status line can be copied.

## v1.69.3

- The window of the control devices has two lists, what is available in the system and what is configured, with arrows between them, and a choice of the default device with *No default device*. A device taken off the list keeps its keys in case it comes back.
- **One page of the cameras** of a device and one test window (`camera_feed::camera_page`, `CamTest`), which LightCreator had to itself.
- **Messages can be copied:** `widgets::message`, selectable and with Copy on the right button.
- **macOS:** a connection or a camera on the local network that the system refuses now says where to allow the program.

## v1.69.2

- The window of the control devices fits on the screen.

## v1.69.1

- The window of the control devices: the keys of a device stand on one line.

## v1.69.0

- For the USB keyboards and pads of LightCreator, MillCreator and PenCreator: the definitions, the keys and the logic of a press (`base_core::control_devices`), reading the pads with `gilrs`, moves that last while a key is held, the windows of the settings (`base_ui::control_devices_ui`) and a submenu of the Machine menu. BaseCreator has no machine: it changes nothing there, but building it on Linux now needs `libudev-dev`.

## v1.68.0

- **New colour schemes:** Superman, Batman, and Joker in a dark and a light version, in the menu and on the start screen, 18 in all.

## v1.67.2

- **Start** is the word *Start* in every language, on the Start screen and in the text on a path.

## v1.67.1

- **Polish names:** the Start screen says *Start* (it said *Początek*), and *Export* in the File menu is *Eksportuj*, like *Importuj*. `tool_check::check_polish_names` keeps Start, Export, Import and Opacity the same in every program.
- For layer presets in the backups of LightCreator, MillCreator and PenCreator: `layer_presets_ui::backup_pair` and `Library::replace_text`. BaseCreator has no layer presets.

## v1.67.0

- **One quick guide:** the sections on the window, drawing, import, the gallery, the machine and the keys are in `base_ui::guide`, in English and Polish, and every program adds its own sections around them. BaseCreator's guide now explains node editing, rounding corners, locked objects, text and the clipart gallery.

## v1.66.0

- The window of the picture under the drawing closes its corner handles with it.
- For LightCreator's move to the shared overlay: hooks for corners kept with the device (`overlay_calibration`, `overlay_changed`), for the choice of a camera in the window (`overlay_camera_ui`, `overlay_is_camera`, `overlay_can_start_camera`), and `Background::data`.

## v1.65.3

- **Fix:** symbols that no font of the programs has showed as empty boxes: the delete, rename and confirm buttons of layer presets, the delete button of saved positions and of the rows of a CSV file, and the mark in front of the explanation of an alarm or error in the console. egui 0.37 ships its symbols only on request. They now use characters of the fonts and the icons of the programs.
- `tool_check::check_symbols` looks at every character in the string literals of a program against its fonts.

## v1.65.2

- **Negative image** is in Properties of a selected picture in every program (it was there only in LightCreator; the import and adjust window has had it everywhere). It can be undone and it does not change a locked picture. The tip no longer speaks of engraving, because a plotter draws and BaseCreator only keeps the picture.
- The text of the *Load preset* button is in the shared table, for LightCreator's new layer presets.

## v1.65.1

- **F1** opens the quick guide in every program: a function key now works alone in `menu::shortcut_act` (the menu showed F1, but only LightCreator had the key).
- The texts of the check of the program are in the shared table, so LightCreator shows them translated too.
- `tool_check::check_windows` presses F1 and Ctrl+P, and `check_guide_paths` takes the language of the guide, so the Polish guides are checked against the Polish menus.
- **Fix:** a job sent right after the check mode ends is no longer lost. Leaving check mode restarts GRBL, and the link now waits for the banner as it does after a stop. With *Check the program before every run* the job could hang at 0 lines. A simulator test covers it; it failed on CI before this fix.

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
