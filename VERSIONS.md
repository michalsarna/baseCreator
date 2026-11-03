# BaseCreator: versions

The releases of the program [BaseCreator](docs/BaseCreator.md) have the numbers of the tags of this repository. Not every tag is a release:
a tag can be made only for LightCreator, MillCreator and PenCreator. Here is what is new for the people who use BaseCreator.

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
