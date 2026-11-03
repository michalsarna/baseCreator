🌐 **English** · [Polski](Import-i-eksport)

# Import and Export

The lists are the same in LightCreator, MillCreator, PenCreator and BaseCreator. What needs a machine is not here: there is no G-code, and the 3D model and the relief STL stay in MillCreator.

## Import

Everything in **File → Import** is added to the open drawing, is selected, and goes away with one undo. On the start screen a new drawing is made for it.

| Entry | Files | What arrives |
|---|---|---|
| **Import image…** | PNG, JPG, BMP, GIF, WebP | A picture, through the import dialog. |
| **Import SVG…** (Ctrl+I) | `.svg` | The paths, each on the layer nearest to its colour. |
| **Import Adobe Illustrator / PDF…** | `.pdf`, PDF-compatible `.ai` | The same. A file with Illustrator layers asks whether to go by colour or by those layers. |
| **Import DXF…** | ASCII `.dxf` | Lines, arcs and curves, in millimetres. The layers of the DXF go to the layers that have no shapes yet. |
| **Import LightBurn project…** | `.lbrn2`, `.lbrn` | The shapes. |
| **Import UGS design…** | `.ugsd` | The shapes on their layers. Feeds and depths are ignored. |
| **Import with a plugin…** | what an installed plugin reads | See [Plugins](Plugins). |

**File → Open…**, and dropping a file on the window, makes a **new tab** of a `.cre`, an SVG, a PDF or an Illustrator file. A DXF, LightBurn or UGS file is imported into the open drawing.

## Export

| Entry | What it writes |
|---|---|
| **Export SVG…** | All the shapes as SVG, the size of the page. Curves are kept. |
| **Export PLT / DXF…** | The outlines of all the shapes as HPGL (`.plt`, one pen per layer) or DXF R12, by the extension you choose. |
| **Export selection as SVG…** / **Export selection as DXF…** | The selected shapes (all of them when nothing is selected). |

Exporting the whole drawing also writes the shapes on hidden layers. **Pictures have no outlines and are left out**, and the program says how many; trace them first with **Trace image…**.

The PLT file has not been tried on a real plotter yet. See [Limitations](Limitations).
