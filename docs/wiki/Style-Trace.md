🌐 **English** · [Polski](Obrys-w-stylu)

# Style trace

**Arrange → Style trace…** draws a selected picture again as lines in one of 19 styles, ready to export, print or open in the other programs: squiggles, a spiral, waves, halftone circles, hatching, stippling, one continuous line, flow lines, a sketch and more. It is also in the right-click menu of a picture and in the **Image** section of **Properties**. It works in LightCreator, MillCreator, PenCreator and BaseCreator alike.

![The Style trace window in flow lines](https://raw.githubusercontent.com/michalsarna/LightCreator/main/docs/screenshots/14b-style-trace.png)

## Step by step

1. Select a picture and choose **Style trace…**.
2. Pick the **Style**; a line under it says what it draws. Some styles have a **Kind** (for example dots, dashes or triangles for stippling).
3. Move the settings of the style. The preview shows the lines exactly as they will be put on the drawing; it is worked out in the background, so the window never stops (old lines stay grey until the new ones are ready). The line under the buttons says how many lines there are and how long they are together.
4. **Tone:** **Black point** and **White point** cut off the darkest and the lightest greys, **Gamma** brightens (above 1) or darkens the midtones, **Trace the light areas** swaps dark and light.
5. **Put on the layer:** the program picks the first layer without shapes (not the picture's own). You can choose another layer from the list.
6. **Delete the image afterwards** removes the picture when the lines are made.
7. **Trace** puts the lines on the drawing as one shape, in the place of the picture (moved, turned and sized with it). **Edit → Undo** takes them back.

Styles that use chance (stippling, flow lines, sketch, scribble, squares, needles, Truchet tiles) have a **Random pattern** number: the same number gives the same lines every time, **Another** tries another one.

## The styles

| Style | What it draws | Settings |
|---|---|---|
| **Squiggle** | Rows of lines that wave faster and higher where the picture is darker. | **Spacing of the lines**, **Wave height**, **Waves per mm**, **Angle**; Kind: rows from edge to edge or **Lift the pen over white**. |
| **Sawtooth** | The same rows as zigzags. | As Squiggle. |
| **Springs** | Rows of loops like a stretched spring, bigger where it is darker. | As Squiggle. |
| **Spiral** | One spiral from the middle that wobbles more in the shadows (the *Frequency lines* of other programs). | **Spacing of the turns**, **Wave height**, **Waves per mm**; Kind: waves or zigzag. |
| **Waves** | Rows lifted by the darkness, like a landscape seen from the side; the rows in front hide what is behind them. | **Spacing of the lines**, **Height of the waves**; Kind: hide what is behind or draw every row whole. |
| **Halftone circles** | A grid of circles whose size follows the tone, as in a printed photo. | **Cell size**, **Largest circle**, **Spacing of the rings**, **Angle**; Kind: circles, rings (filled with rings) or spirals. |
| **Etching** | Hatching in layers, as in an old engraving: every darker tone adds lines in another direction. | **Spacing of the lines**, **Tone levels** (1 to 6), **Angle**; Kind: straight or wavy lines. |
| **Squares** | A grid of squares one inside the other, more of them where it is darker. | **Cell size**, **Most squares in a cell**, **Shake** (a random turn), **Angle**. |
| **Needles** | Short strokes on a grid, longer where it is darker. | **Cell size**, **Stroke length**, **Angle**; Kind: along the shapes, at the angle or at random. |
| **Truchet tiles** | Tiles of quarter circles turned at random, more arcs where it is darker. | **Tile size**, **Most arcs in a tile**. |
| **Contour lines** | Lines of equal brightness, like the contour lines of a map. | **Smoothing**, **Tone levels** (2 to 40). |
| **Hilbert curve** | One space-filling curve, with finer turns where it is darker. | **Smallest step**, **Tone levels**; Kind: one line or **Lift the pen over white**. |
| **Stippling** | Dots spread by the tone (weighted Voronoi stippling): close in the shadows, sparse in the lights. | **Number of dots**, **Dot size**; Kind: dots, circles by tone, dashes or triangles. |
| **One line (TSP)** | The dots of stippling joined into one line that does not cross itself, drawn without lifting the pen. | **Number of dots** (up to 15 000). |
| **Triangles and cells** | A mesh of triangles (Delaunay), or of the cells round the dots (Voronoi), over dots spread by the tone. | **Number of dots**; Kind: triangles or cells. |
| **Line art** | Only the edges of the picture, as lines. | **Smallest detail**, **Edge sensitivity**, **Shortest line**; Kind: edges, or edges and the dark areas. |
| **Flow lines** | Long lines that flow along the shapes, closer together where it is darker. | **Smallest gap** (in the darkest parts), **Largest gap** (in the lightest), **Size of the curves**, **Angle**; Kind: along the shapes or a random flow field. |
| **Sketch** | Many short straight strokes laid where the picture is darkest, as in a pencil sketch. | **Stroke length**, **Number of strokes**, **Ink of one stroke** (how much of the darkness one stroke takes); Kind: separate strokes or strokes in chains. |
| **Scribble** | One wandering line of loops that scribbles the dark parts in. | **Loop size**, **Length of the line** (metres), **Ink of one pass**. |

## Good to know

* The settings are in millimetres of the picture, so a bigger picture gets more lines, not thicker ones. Keep the spacing of the lines larger than the width of the pen, the laser dot or the bit.
* A dot is a small closed circle, so it is drawn, burnt or engraved like any other line.
* **One line (TSP)**, **Spiral** and **Hilbert curve** are drawn in one long move; the other styles are put in an order that keeps the moves between the lines short.
* Very fine settings make a great many points: the line under the buttons warns when the drawing will get slow, and the result stops at two million points.
* Adjust the picture first (**Arrange → Adjust image…**: brightness, contrast, levels) when the style needs more contrast.

See also: [Design Tools](Design-Tools), [Menus](Menus).
