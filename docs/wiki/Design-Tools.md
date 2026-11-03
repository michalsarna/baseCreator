🌐 **English** · [Polski](Narzedzia-projektowe)

# Design Tools

BaseCreator edits shapes with the same tools as LightCreator, MillCreator and PenCreator. The keys are on [Keyboard Shortcuts](Keyboard-Shortcuts), the menus on [Menus](Menus).

## Drawing

The palette at the left has **Select (V)**, **Node edit (N)**, **Rectangle (R)**, **Ellipse (E)**, **Triangle (Y)**, **Star (S)**, **Polygon (G)**, **Heart (K)**, **Spiral (I)**, **Line (L)**, **Polyline / pen (P)**, **Text (T)**, **Pan (H)**, **Zoom in (Z)** and **Zoom out (X)**. The tools that insert objects are also in the **Insert** menu, with **Generate shape…** (the generators of plugins) and **Online clipart…**.

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

**View → Background picture…** puts a photo or a scan under the drawing, to draw over.

## The clipart gallery

The **Clipart** tab has ready-made pictures, in a gallery shared with the other programs; **Insert → Online clipart…** searches free pictures on the internet.

## The right panel

The tabs are **Properties**, **Layers**, **Clipart** and **Description**. See [Panes and Tabs](Panes-and-Tabs#the-right-panel).
