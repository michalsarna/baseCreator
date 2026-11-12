# BaseCreator: versions

The releases of the program [BaseCreator](docs/BaseCreator.md) have the numbers of the tags of this repository. Not every tag is a release:
a tag can be made only for LightCreator, MillCreator and PenCreator. Here is what is new for the people who use BaseCreator.

## v1.95.0

- **Project setup for paper** (`project_setup::Kind::paper`, `PAPER`, `papers_that_fit`): a list of paper sizes (A6 … A1, B5, B4, Letter, Legal, Tabloid), only those that fit the work area of the machine, upright or lying; no thickness for a sheet.
- 6 texts in every language (3 moved from MillCreator: the messages of the project setup).

## v1.94.0

- **Project setup for every program that works on a piece of material** (`project_setup`, the window of MillCreator moved here): a block or a cylinder on the rotary, with its picture, and a new **Two-sided job**: the axis the material is turned over round (H or V, a dashed line on the picture) and **Alignment marks**.
- **Two-sided jobs** (`base_core::setup`: `FlipAxis`, `ZeroPoint`, `turn`, the project section `two_sided` with side B; `two_sided`: the **Side A / Side B** switch, **Ghosts of the other side** and the ghosts drawn turned over).
- 35 texts in every language (21 moved from MillCreator).

## v1.93.0

- **Web server and API for every program** (`web_api`, moved from LightCreator): **Settings → Program options…** switches on a page with the project and an API (`/api/status`, `/api/project.svg`, …), behind an access phrase (at first the name of the program). Programs with a machine also show it, its cameras and STOP / Home / jog; BaseCreator shows the project. The default ports go up from LightCreator's: 8765 LightCreator, 8766 MillCreator, 8767 PenCreator, 8768 BaseCreator, so that all of them can run on one computer.
- 14 texts in every language (12 moved from LightCreator).

## v1.92.1

- **rfd 0.18** (the file dialogs; Dependabot PenCreator #185): nothing changes for the person using the program.

## v1.92.0

- **Arrange → Add as no-go zone** (`StdAct::AddAsZones`, `MenuParts::zones`) in the programs that keep zones, besides the right-click menu.
- **Right-click menu:** *Shape operations* also has Auto-join paths…, Close path with tolerance… and Dogbone / T-bone corners…; a new submenu *Arrays and nesting* has Grid array…, Circular array…, Copy along path… and Nest shapes….
- 6 new texts in every language (also for the menus of the programs: No-go zones…, Layer presets…, Preview G-code, Show no-go zones, Saved positions…).

## v1.91.0

- **Layer in the Properties tab** (`editor::layer_section`, `set_layer`): the colour and the list of the layers, as in LightCreator; a choice moves the selected unlocked shapes and makes the layer the active one.
- **Closing the last drawing** asks whether to close the program (`doc_tabs::ask_quit`): **Close the program** or **Back to the start screen** (also Escape).
- **Chinese New Year:** a festive icon and two festive colour schemes (dark, light) from five days before to its seventh day (`seasonal::chinese_new_year`, a table of the dates 2020 – 2060).
- 7 texts in every language.

## v1.90.0

- **Add as no-go zone** in the right-click menu of the work area (for the programs that keep zones, `EditorHost::can_add_zones` / `add_zones`): every unlocked selected shape becomes a no-go zone of the device, a circle when it is round and otherwise the rectangle round it (`zones::shape_around`), and leaves the drawing (undo brings it back).
- **No-go zones panel** (`zones_ui::panel`) for a tab of the side panel shown while the device has zones.
- **Rotary for three-axis GRBL machines** (`rotary_ui`): the page of the device settings and the lines of the way round on the work area, now shared by MillCreator and PenCreator.
- 18 texts in every language (5 new, 13 moved from MillCreator); a new icon (`ban`).

## v1.89.1

- **Quick guide** (F1): two new lines, in English and Polish: nesting by the real outlines and onto new sheets, and the no-go zones and the accelerations of the machine (the time of a job).

## v1.89.0

- **Nesting by the real outlines** (`nest::nest`, Arrange → Nest shapes…, **By the real outlines**, on by default): the parts are packed by their true shapes on a grid of the sheet, the biggest first, at the first place from the top left where they fit, turned in the steps asked for (**Turning**: none, half turns, quarter turns, eighth turns). A small part can go into the hole of a big one, an L into the corner of another; a group is one part. What does not fit stays where it was and is said. The packing by boxes is still there (switch the option off).
- **More sheets:** with **What does not fit goes on new sheets** (for the programs that have tabs) what is left is nested on new sheets, each one a drawing in a new tab with the same layers and material (`EditorHost::can_add_sheets`, `add_sheets`); nothing is lost.
- 10 new texts in every language.

## v1.88.0

- **No-go zones in every program** (`zones`, moved here from LightCreator): rectangles and circles of the work area a tool must never enter, each with a name, a switch and the heights it stands between (`Zone::at_heights`). **`zones::route`** finds the shortest way round the zones (grown by a margin) for a move through the air: a graph of the corners of the zones; `None` when the move starts or ends in a zone or the zones wall it in. `segment_hits_with` checks with a margin (the radius of a bit), `ZoneShape::polygon`, `check_all`.
- `zones_ui::page` (the page of the device settings, the same in all programs) and `zones_ui::paint` (the red areas on the work area).
- 10 new texts in every language (9 of them from LightCreator).

## v1.87.0

- **Leads** (`lead`): a closed cut comes onto its line from the waste side and leaves it the same way, so the mark of the start is not on the part. A lead is a **Line** at an angle or an **Arc** that touches the line; an **Overlap** goes on past the start before the lead out. `lead::plan` gives the points; a lead that would cross the line or reach into another ring is made half or a quarter as big, or left out (`Leads::shrunk`, `dropped`). `lead::good_start` starts a ring that begins at a corner in the middle of its longest piece. `lead::ring_with_overlap`, `lead::walk`.
- `lead_ui::rows`: the rows of a layer's settings for the leads (used by MillCreator for its profiles and LightCreator for its line layers).
- 11 new texts in every language.

## v1.86.0

- **The time of a job with the acceleration of the machine** (`motion`): every move speeds up and slows down at the acceleration of the axes it uses (a slow Z makes plunges longer), a corner is taken at the speed the junction deviation allows, the speeds are planned back from where the machine must stop and forward from where it can get to, and only the next 15 moves are known at once (as GRBL plans them), so a long chain of tiny fast moves is held back. Without the accelerations the time is the length at the feed, as before. A zigzag of short moves takes in grbl-sim about what is planned (16.7 s against 13.9 s planned; length / feed would say 4.3 s).
- **The G-code viewer** times a file for the machine of the program (`gcode_view::parse_with`, `gcode_view_ui::set_machine`): its accelerations and its speed of rapid moves.
- **Read the machine** also shows the accelerations of X, Y, Z (`$120` – `$122`) and the junction deviation (`$11`) (`Suggestion::accel`, `junction`; `Motion::take`), and `device_pages::motion_rows` sets them on a device page.
- 3 new texts in every language.

## v1.85.0

- **The Recording tab** (`recording_ui::tab`): a panel for each camera of the device, named after it, with everything it records (a photo, a video, the time-lapse, the video format, the way of the time-lapse, the folder) and the way its view opens; under them the time-lapse photos with the head moved aside and the long lines sent in parts (they were at the bottom of the Control tab). The camera view keeps only the buttons: a photo, a video, the time-lapse started and stopped.
- **Each camera records on its own** (`camera_rec::Rigs`): one recorder per camera, with its own settings kept with the camera (`CameraSource::rec`; the settings last used are the start of a camera without its own). The camera the view shows gets its pictures from the view; another camera that records reads its own stream. With more than one camera the names of the files have the camera in them (`box-Side_camera-photo-0001.jpg`), and all cameras use the same folder of the run.
- The time-lapse photos aside with several cameras (`Trips::frame_all`): a picture due on any camera stops the job; at the place, every camera whose time-lapse runs takes a new picture.
- `RecSettings`, `LapseMode`, `VideoFormat` are in `base_core::camimg` (still reachable from `camera_rec`); `camera_rec::load_settings` / `store_settings` instead of `RecSettings::load` / `store`; `Recorder::controls` returns whether the settings changed, `Recorder::quick` is the row of buttons.
- 2 new texts in every language; two hints now name the Recording tab.

## v1.84.0

- **Time-lapse photos aside** (`photo_park`, `photo_trip`, the Control tab): during a job each picture of the time-lapse waits until the job can stop, the head goes to a place of the machine (machine coordinates, with "Use the current position"), the machine stands still for a moment (the wait before the photo), the camera takes a picture that came after that, and the head comes back to where the job stopped (the units, the modes, the spindle with its time to come to speed, down to the depth at the feed) and the job goes on. A laser stops at any line (the beam off for the trip); a mill and a plotter only before the first rapid move after a lift, so a picture due on time or after a layer waits for the next lift. A mill may keep its spindle running. The link stops a job for a photo (`Cmd::PhotoStops`, `PhotoArm`, `PhotoWait`, `PhotoContinue`, `Evt::PhotoStop`, `PhotoReady`) only once the controller has done everything and reported standing still, and never twice before the same line.
- **After every lift** (`LapseMode::EveryLift`): a picture each time the tool goes up, with the head moved aside, for the programs whose machines lift a tool.
- **HPGL: the way aside for photos** (`HpglCommands::park`, the HPGL page of the device): the commands that take the pen aside after it is up; it comes back with an absolute move. Empty: the place of the Control tab.
- **Split long lines** (`gcode_split`, the Control tab): long straight cuts are sent as 2 to 256 shorter lines, each at least a given length, so a job can stop sooner and its progress moves evenly. Rapid moves, arcs and unusual lines stay whole; inches and relative moves add up to the same path.
- `Controller::stops_for_photos`: only a controller that says when it stands still (GRBL, Smoothieware) or a plotter the link drives itself (EBB, HPGL) stops for photos; Marlin does not.
- `resume::preamble` and `can_resume` (the way back into a job at a line, used by Start from a line and by the photos), `Modal::step`.
- 18 new texts in every language.

## v1.83.0

- **Style trace** (`style_trace`, `base_ui::editor::EditorStyleTrace`, **Arrange → Style trace…**, also in the right-click menu and in Properties): a picture drawn again as lines in one of 19 styles: squiggle, sawtooth, springs, spiral, waves (with hidden lines), halftone circles / rings / spirals, etching (hatching by tone levels), squares, needles, Truchet tiles, contour lines (marching squares), an adaptive Hilbert curve, weighted Voronoi stippling (dots, circles by tone, dashes, triangles), one line (stippling joined by nearest neighbour and 2-opt), Delaunay triangles or Voronoi cells, line art (Canny edges), evenly spaced flow lines (Jobard–Lefer, along the shapes or a flow field), sketch and scribble. Tone settings (black and white point, gamma, light for dark), a seed for the random styles, a live preview worked out in a thread of its own, the result put on the drawing as one path in the place of the picture. Dots are small closed circles, so every program draws, burns or mills them; the lines are ordered to keep the moves short.
- `EditorHost::line_layer` / `make_line_layer` (the layer a style trace goes on and how it is made to draw lines), `layer_count`, `layer_name`.

## v1.82.1

- The window of the knife library keeps its size: it grew a little on every frame, up to the height of the screen.

## v1.82.0

- **Knife library** (`knife_lib`, `base_ui::knives_ui`): ready-made drag knives (the 30°, 45° and 60° blades and a drag knife in a holder of a CNC machine) and your own ones, each with the offset of its tip, the corners it turns round, the overcut and notes. The window of the library copies a ready-made knife into one of your own, changes and deletes it; `library_row` puts a knife of the library on a device or a layer, `knife_rows` are its values. PenCreator and MillCreator keep the library with their settings and in the backup.
- **HPGL commands of a device** (`plotter::HpglCommands`, `plotter_ui::hpgl_page`): what an HPGL plotter or cutter gets at the start and at the end of the program (`M2`, `M30`), for the pen or the knife down and up, the speed (`VS{speed};`, at most the top speed of the device; empty: never) and the force (`FS{force};`, sent once at the start; 0: never). An old device gets the usual commands.

## v1.81.0

- **Drag knife** (`drag_knife`): `compensate` turns an outline into the path of the axis of a drag knife (the axis ahead of the tip by the offset; a swivel round every corner sharper than `swivel_deg`; the blade turned from the way it pointed after the last cut; a closed outline cut on past its start by `overcut`), `simulate` drags the tip behind a path (the previews and the tests), `nesting` / `inside_first` order the cuts holes first. PenCreator has it as a tool of the device, MillCreator as an operation of a layer.

## v1.80.0

- **Units:** `Units` has centimetres, metres and feet too (`ALL`, five, for showing lengths; `DEVICE`, millimetres and inches, for a machine), with `is_metric`, `symbol`, `mm_per_unit`. `GridPrefs::units` (Settings → Program options → **Units**, `units_ui::display_units_row`; `None` = as the device) is what `EditorHost::units` gives in the programs; `EditorHost::machine_units` is the device's.
- **G-code in the units of the machine** (`gcode_units`): a program written in millimetres turned into one for a machine in inches: G21 → G20, lengths (X Y Z U V W I J K R Q) and feeds divided by 25.4; angles, S, P, T, N and comments kept; GRBL settings untouched, jogs converted.
- **Both units** when the program and the machine are of different systems: `DimStyle::in_units` ("25.4 mm [1.000"]"), `units_ui::fmt_len_both` (the ruler).
- `DimStyle::upright`: the numbers stand upright (ASME) instead of along their lines (ISO, JIS).
- 8 new texts in every language.

## v1.79.0

- **TIFF pictures** (`.tif`, `.tiff`) are read like the other pictures (import, background, drop); `tif` and `tiff` are reserved for plugins.
- **`dims`**: dimensions as on a technical drawing (extension lines, arrows, the number along the line): `dim`, `plan` (the whole, the parts and the distance from the zero when it is in the picture), `box3d` / `box3d_labelled` (the edges of a block in 3D). The previews of the programs show them with their **Dimensions** button; MillCreator also draws its technical drawing and the block of its project setup with them.
- **`widgets::toggle`**: an option as a button, light blue while it is on (the options of the bars of the previews).
- **Dimensions over the drawing** (`editor::dims_button`, next to the toolpaths in every program; `EditorState::show_dims`): `dims::Features` (every outline by itself, round ones as circles), `dims::drawing`, `dims::diameters` ("4 × Ø 6"), `dims::features3d` (on a plane seen in 3D).
- **Ruler:** snaps to nodes, middles of segments and centres of circles and ellipses (marked); a click on a circle or an ellipse shows its radii, on a curve its length and, for an arc, the radius from its centre to the point (`editor::ruler`).
- **Node edit:** a right click on a node or a segment opens the menu of the node bar; **Smooth (G)**; the radius after **Round (R)**.
- **Shift while drawing** keeps the proportions in the picture of the shape being drawn too (a circle with the ellipse tool, a square with the rectangle), not only in the shape made at the end (`editor::keep_square`).
- **`picture_out`**: what egui draws as a picture without a window (a small rasteriser with the fonts of the program), saved as PNG, JPEG or a PDF page (MillCreator's technical drawing).
- **`work_panes::Content::Extra`**: a tab of a program's own.
- 3 new texts in every language.

## v1.78.0

- **Ruler (M):** a new tool (`Tool::Measure`, after the zoom tools): drag or click two points and a line over the drawing shows the length, ΔX, ΔY and the angle; a point near a node takes the node; a click on a straight segment measures it. `EditorState::measure`, `measure_text`. Esc, another tool or the next measurement takes it away.
- **Right click with a tool** is a double Escape: back to the select tool (a line of the pen is finished first); only the select tool has the context menu.
- **Several selected objects** each have a thin dashed frame besides the frame of the selection.
- **Node bar:** the keys are in the names ("Break (B)"); **Open / close** is gone (Break opens, dropping an end on the other closes).
- **A drag whose release was missed** (the button let go outside the window or over another window, or the focus lost while it was held) no longer goes on with the object, slider or colour following the pointer: `window::release_stuck_drag` (through `window_edges`, every frame) ends it, and the editor ends its own drag when the work area is no longer dragged.
- `editor::faded_colour`: the shapes of a layer that is not made, faded towards the work area.
- 6 new texts in every language (also for the menus of the programs: No-go zones…, Layer presets…, Preview G-code, Show no-go zones, Saved positions…).(the ruler).

## v1.77.0

- **Files of LightBurn and MillMage, both ways:** checked against the same drawings saved by LightBurn 2.2 and MillMage 0.9 (`tests/fixtures/interchange`, `tests/interchange.rs`).
  - **Import:** MillMage projects (`.mage`) with their layers; the old PostScript Illustrator files LightBurn writes (`ai`; before, only PDF-compatible ones were read); the pictures of an SVG (`svg::images`, LightBurn puts them there). DXF of both kinds and LightBurn projects were already read.
  - **Export** (*File → Export*, `StdAct::ExportAs`, `editor::Exchange`): *Export DXF…* (AutoCAD 2007, `AC1021`, the tables a strict reader wants, `export::dxf_modern`), *Export DXF R12 (legacy)…* (`export::dxf`), *Export Adobe Illustrator…* (`ai::write`), *Export LightBurn project…* (`.lbrn2`, or `.lbrn` when the file is named so) and *Export MillMage project…* (`lightburn::write`; paths keep their curves, pictures are kept). *Export SVG…* keeps the pictures now; *Export selection as DXF…* writes the current kind.
  - *File → Open…* of an SVG, PDF or Illustrator file goes through `svg::read_scene` (by content, with the pictures of an SVG), so an `.ai` of LightBurn opens too.
  - The settings of the layers go into a LightBurn project when the program knows them (`EditorHost::lightburn_cut`; LightCreator gives the mode, power, speed, passes and interval).
  - **Fixed:** a LightBurn path closed by a curve into its first point lost the handle of that curve (an ellipse came back slightly flattened at its start).
  - `lightburn::EXTENSIONS` (`lbrn2`, `lbrn`, `mage`) for the file windows of the programs; `mage` is reserved for plugins; `EditorHost::export_stem` names the exported file.
- 7 new texts in every language.

## v1.76.0

- **Auto levels work on a real photo:** they stretched the range from the darkest to the lightest pixel, and nearly every photo has a pixel of pure black and one of pure white, so nothing changed. Now the outer 0.5 % of the pixels on each side are left out (`image::auto_range`). The checkbox moved from Tone to **Levels**: while it is on, the two sliders of the range are locked and show the range found in the picture, and the range chosen by hand is not used.
- 6 new texts in every language (also for the menus of the programs: No-go zones…, Layer presets…, Preview G-code, Show no-go zones, Saved positions…).

## v1.75.0

- **The colour of the work area in the previews of a job:** `widgets::area_colour` (the label, a colour button and **Automatic** while a colour of one's own is set) and `GridPrefs::preview_color` (kept with the view settings; `None` is the colour of the drawing, `preview_fill`). LightCreator shows it as *Work area colour*, MillCreator as *Material colour* (the block in the 3D view) and PenCreator as *Paper colour*. BaseCreator has no preview of a job: the colour of its page is in the view options, as before.
- 4 new texts in every language.

## v1.74.0

- **One window for adding a device** (`add_device_ui`), as LightCreator had it: the name, the controller, the connection (its type, the port with Refresh or the host and its TCP port), **Read from device** (the button says how the read went), the rows of the machine (the program's own), an optional camera with a test, **Worth checking**, and **Add device**, **More settings…**, **I have no device** and **Cancel**. MillCreator and PenCreator use it too (they opened the whole device window).
- 18 texts moved here from LightCreator with its translations; two reminders of the first test for a milling machine and a plotter.

## v1.73.0

- **Keys of the node tool:** D deletes the selected nodes, I inserts a node, L makes the segment a line, S a curve, C makes the nodes corners, R rounds them (with the radius of the bar), B breaks the path at the node (a closed path opens there, an open one is cut in two; also a new **Break** button). In the node tool these letters come before the tools that have them (I, L, S, R). The tooltips of the buttons show the keys.
- **A node dropped onto another joins them:** the end of a path on its other end closes the path, a node on its neighbour merges with it, an end on the end of another path makes one path of the two.
- **Width and height** in the Properties have arrows of their direction before W and H.
- **Fit all objects on the zoom bar:** `zoom_bar::overlay_with` adds a fourth button for the previews of a job (they had their own fit buttons in their title line).

## v1.72.1

- **The secondary grid only close up:** when it is switched on, it is drawn only at a zoom of more than 200 % (further away its lines filled the sheet). The tooltip of its button and the view options say so.

## v1.72.0

- **One camera view for every program** (`camera_view`): the three ways of showing it (a window of its own with the bar of the main window, a tab of the side panel, a tab of the work area) with their buttons and their numbers as LightCreator kept them, the picture that zooms (the wheel and the zoom bar, up to 8×), moves and shows itself whole on a double click, the window that keeps its place and is cut to its picture when let go, and keeping the way with the device and each camera. LightCreator's view moved here; MillCreator and PenCreator get the same view (they showed only the picture in a tab).
- Texts: **Separate window**, **Side tab** and **Camera view** in 30 languages (LightCreator's translations).

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
