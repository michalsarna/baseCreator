🌐 **English** · [Polski](Narzedzia-projektowe)

# Design Tools

BaseCreator edits shapes with the same tools as LightCreator, MillCreator and PenCreator. The keys are on [Keyboard Shortcuts](Keyboard-Shortcuts), the menus on [Menus](Menus).

## Drawing

The palette at the left has **Select (V)**, **Node edit (N)**, **Rectangle (R)**, **Ellipse (E)**, **Triangle (Y)**, **Star (S)**, **Polygon (G)**, **Heart (K)**, **Spiral (I)**, **Line (L)**, **Polyline / pen (P)**, **Text (T)**, **Pan (H)**, **Zoom in (Z)**, **Zoom out (X)** and **Ruler (M)**. The tools that insert objects are also in the **Insert** menu, with **Generate shape…** (the generators of plugins) and **Online clipart…**.

## Text

Choose **Text (T)** and click on the page: a text appears and the **Properties** tab opens with the field to type in. There you set the font, the size, bold and italic, the letter and line spacing and the alignment. **Convert to curves** turns the text into shapes whose nodes can be edited. A new text takes the settings of the text changed last.

## Editing

* **Right mouse button** on a shape opens its menu: copying, grouping, locking, flips and quarter turns, the submenus **Arrange** and **Shape operations**, and for a picture **Adjust image…** and **Trace image…**. The entries that do not fit the selection are grey.
* **Esc twice** (within half a second) goes back to **Select (V)**.
* **Undo** keeps up to 100 steps for each drawing.

## Shape operations

**Union**, **Intersection**, **Subtract** and **Exclusive or** combine closed shapes. **Offset shape…** makes a larger or smaller outline, **Round corners…** rounds them, **Convert to curves** puts curves in place of lines, and **Convert to path** makes a path of a rectangle or a star so that its nodes can be edited.

## Arranging and copying

All of these are in the **Arrange** menu:

* **Auto-join paths…** joins open paths whose ends are nearer than a tolerance; **Close path with tolerance…** closes the ones whose two ends are that near. **Dogbone / T-bone corners…** adds a notch to sharp corners, for shapes that will be milled.
* **Grid array…** copies in rows and columns. **Circular array…** puts copies round the middle of the selection. At most 2000 copies are made.
* **Copy along path…**: select the shapes, then the path **last**. **Text on path…**: select the text, then the path last; afterwards they are outlines, not text.
* **Trim by shape…**: select the lines, then a closed shape **last**. **Remove duplicates** deletes shapes with the same outline on the same layer. **Break apart** makes one shape of every outline. **Simplify…** takes away the points within a tolerance of the line.
* **Serial text…**: a text is copied with its `{n}` counted up or, with a CSV file, once for every row. **Barcode / QR code…** puts a code on the layer in use as outlines.
* **Nest shapes…** packs the selected shapes in rows by their boxes, not their outlines. **Measure…** shows the size, position, area and length of the selection and, for two shapes, the distances between them.

## Pictures

**File → Import → Import image…**, or dropping a picture on the window, opens the import dialog. **Adjust image…** changes a picture on the page, and **Trace image…** turns its dark or light areas into outlines. Pictures have no outlines of their own, so export leaves them out until they are traced.

**View → Background picture…** puts a photo or a scan under the drawing, to draw over. It is saved with the project.

## The clipart gallery

The **Clipart** tab has ready-made pictures, in a gallery shared with the other programs; **Insert → Online clipart…** searches free pictures on the internet.

## The right panel

The tabs are **Properties**, **Layers**, **Clipart** and **Description**. See [Panes and Tabs](Panes-and-Tabs#the-right-panel).

## Ruler, selection and the node bar

* **Ruler (M):** the tool after **Zoom out (X)** in the palette. Drag from one point to another, or click two points: a line over the drawing shows the length, ΔX, ΔY and the angle, in the units of the program. A point near a node of a shape takes the node itself (otherwise the grid when snapping is on). A click on a straight segment of a shape (a side of a rectangle, a line) measures that segment. The measurement stays until the next one, **Esc** or another tool; nothing in the drawing is changed.
* **Right click with a tool** (a shape, the pen, the node tool, the ruler…) works as a double **Esc**: back to **Select (V)** with nothing selected; a line being drawn with the pen is finished first. With the select tool the right click opens the context menu as before.
* **Several objects selected:** besides the frame of the whole selection, every object has a thin dashed frame of its own.
* **The node bar** shows the keys after the names: **Corner (C)**, **To line (L)**, **To curve (S)**, **Add node (I)**, **Delete node (D)**, **Round (R)**, **Break (B)**. **Open / close** is gone: **Break (B)** opens a path, and a path is closed by dropping one of its ends onto the other.
* **A layer that is not made** (its output switched off) shows its objects faded towards the work area.

* **Pictures:** TIFF (`.tif`, `.tiff`) is read too, as PNG, JPEG, BMP, GIF and WebP.

## Dimensions on the work area, the ruler and the node menu

* **Dimensions** (the button next to **Toolpaths** on the bar over the work area) shows the drawing as on a technical drawing: the width and height of the whole and of every outline (the outside and the holes of a shape each by itself), and the diameters of the round ones, the same ones named once (**4 × Ø 6**). It changes nothing in the drawing.
* **Ruler (M)** takes a node of a shape (a square marks it), the middle of a segment (a diamond) or the centre of a circle or an ellipse (a circle). A click on a circle shows its radius from the centre to the point clicked and its diameter; on an ellipse both radii and the radius to the point; on a curve its length, and when the curve is an arc of a circle, the radius drawn from the centre to the point clicked and its length.
* **Node edit:** a right click on a node or a segment opens the menu of the node bar; **Smooth** has the key **G**, and the radius of **Round (R)** stands after its button.
