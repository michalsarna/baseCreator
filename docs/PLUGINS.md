# Plugins

Plugins add things to every program built on baseCreator (LightCreator, MillCreator, PenCreator). A plugin is a **folder**
with a `plugin.json` and the files it names. It lives in `<data>/baseCreator/plugins/<id>/` (macOS
`~/Library/Application Support/baseCreator/plugins`; `BASECREATOR_DIR` moves the whole folder), or is installed from a
`.zip` in the **Plugins** window.

A plugin can bring four kinds of things. They can be mixed in one plugin.

| Kind | Needs code? | What it is |
|---|---|---|
| **Pack** | no | machine presets, clipart, translations, data files (tool libraries, materials) |
| **Importer** | script | reads a file format and returns shapes |
| **Generator** | script | a form with parameters; returns shapes |
| **Postprocessor** | script | rewrites the lines of a G-code program for another controller |

## The official list

Plugins can also be installed from the official list: **Settings → Plugins… → Official plugins**. The list is built from the
repository [baseCreator-plugins](https://github.com/michalsarna/baseCreator-plugins) (with a guide for authors and the way
to send a plugin) and published at `https://michalsarna.github.io/baseCreator-plugins/index.json`. It shows every plugin,
also those for another program; **Install** downloads the zip from that site only, checks its SHA-256 and that it holds the
plugin and version the list names, and **Update** appears when the list has a newer `version`. A plugin whose `min_base` is
newer than the program is not offered. Looking for updates at the start of a program is off until the person switches it
on (the switch is shared by the programs).

## Safety

Scripts are written in [Rhai](https://rhai.rs) and run in a sandbox: **no files, no network, no system, no machine**. They
can only compute and return data. Time (10 s) and memory are limited, `eval` and `import` are off. What a script returns
is checked before it is used:

* shapes: numbers must be finite and sane, at most 200 000 shapes and 2 000 000 points;
* G-code from a postprocessor: lines of at most 240 characters, no control characters, and **no controller commands**
  (`$…`, `~`, `!`, `?`), so a postprocessor cannot change the settings of the controller or reset it. The program still
  runs its own check of the program afterwards.

Even so, a plugin is somebody's code that decides what your machine is told to do. Install only plugins you trust, and
look at a new postprocessor's output before you run it on a machine.

## The manifest

```json
{
  "format": 1,
  "id": "my-plugin",
  "name": "My plugin",
  "version": "1.0.0",
  "author": "Me",
  "license": "MIT",
  "description": "What it does.",
  "programs": ["millcreator", "pencreator"],

  "presets": [ { "name": "My router (400 × 300)", "class": "mill", "area": [400, 300, 60], "rapid": 2000, "spindle_max": 12000 } ],
  "clipart": [ { "file": "clipart/star.svg", "name": "Star", "category": "My pack", "license": "CC0", "author": "Me" } ],
  "texts": { "pl": { "My plugin": "Moja wtyczka" } },
  "data": [ { "kind": "mill-tools", "file": "data/tools.json" } ],

  "importers": [ { "id": "xy", "name": "XY points", "extensions": ["xy", "csv"], "script": "import.rhai" } ],
  "generators": [ { "id": "holes", "name": "Holes on a circle", "script": "holes.rhai", "params": [ … ] } ],
  "postprocessors": [ { "id": "numbers", "name": "Number the lines", "script": "numbers.rhai" } ]
}
```

* `id`: 1–64 characters `a-z 0-9 - _`; **the folder must have the same name**. `programs`: leave out for all programs.
* Paths are relative to the plugin folder, with no `..`; scripts end in `.rhai`.
* `format` is 1. A plugin with a newer format is refused with a message to update the program.
* A plugin with a mistake in the manifest or a missing file is listed in the Plugins window with the reason, and not loaded.

### Packs

* `presets`: `class` is `mill`, `pen` or `laser`; `area` is X, Y, Z travel in mm (Z 0 for a plotter or laser); `rapid` in
  mm/min; `spindle_max` in S units. A program lists them after its own presets when a device is added.
* `clipart`: SVG files that join the shared gallery (once; pictures with the same name and category are not added twice).
* `texts`: `language code → English text → translation`. They translate the names, descriptions and parameter labels the
  plugin shows (not the program's own texts).
* `data`: files a program picks up by `kind` (`mill-tools` for tool libraries, `materials`, …). What a kind means is up to
  the program; unknown kinds are ignored.

### Generators: the form

```json
"params": [
  { "id": "count",  "label": "Number of holes", "kind": "number", "default": 6, "min": 1, "max": 200, "unit": "" },
  { "id": "centre", "label": "Centre hole",     "kind": "bool",   "default": true },
  { "id": "label",  "label": "Text",            "kind": "text",   "default": "A" },
  { "id": "style",  "label": "Tooth shape",     "kind": "choice", "default": "square", "options": ["square", "round"] }
]
```

The script gets the values in the map `p` (`p.count`, `p.centre`, …). Numbers are always floats and limited to `min` /
`max`; use `p.count.to_int()` for a loop. `id`s are letters, digits, `-` and `_` (so `p.my_value`; avoid words that are
keywords in Rhai, such as `module`, `import`, `private`).

### Importers: a form, a preview, binary files

An importer entry can have more than the script:

```json
{ "id": "stl", "name": "STL model", "extensions": ["stl"], "script": "stl.rhai",
  "binary": true, "form": true, "preview": true, "description": "…",
  "params": [ { "id": "scale", "label": "Scale", "kind": "number", "default": 1 } ] }
```

* `binary`: the script gets the file as a **blob** of bytes instead of a text. `data.len()`, `data.parse_le_int(pos, len)` read
  integers, `read_f32(data, pos)` and `read_f64(data, pos)` read floats (Rhai's own `parse_le_float` does not read a 4-byte float).
* `params`: a form like the one of a generator. The values reach the script in `ctx.p` (`ctx.p.scale`).
* `form`: the script also has `fn form(data, ctx)` that returns more fields of the form, made from the contents of the file: an
  array of maps `#{ id, label, kind, "default", min, max, options, unit }` (up to 100; write `"default"` in quotes, it is a keyword).
* `preview` (or any of the above): the program opens an **import window** with the form and a preview of the shapes instead of
  importing at once. The preview is made by running `read` in the background after each change of the form (a moment after the
  last one); the button **Import** puts the shapes that are shown into the drawing.

A plugin that needs these (they came with baseCreator 1.6.0) says `"min_base": "1.6.0"`: older programs then refuse to load it
and the official list does not offer it to them.

## The scripts

Each script defines one function. `ctx` is `#{ app, area_w, area_h }`: the program (`"millcreator"`…) and the size of the
work area in mm. (A function that does not need `ctx` may leave the parameter out.)

| Kind | Function | Gets | Returns |
|---|---|---|---|
| importer | `fn read(text, ctx)` | the file as text (a blob of bytes for a `binary` importer) | shapes |
| generator | `fn generate(p, ctx)` | the values of the form | shapes |
| postprocessor | `fn post(lines, ctx)` | the lines of the program (array of strings) | the new lines (array of strings) |

*Shapes* are one shape, or an array of shapes (arrays can be nested). **Coordinates are millimetres, origin at the top
left of the work area, Y down** — the same as the drawing. (An importer for a format with Y up turns it over with
`ctx.area_h - y`.)

| Function | Makes |
|---|---|
| `rect(x, y, w, h)`, `rounded_rect(x, y, w, h, r)` | rectangles (x, y is the top left corner) |
| `circle(cx, cy, r)`, `ellipse(cx, cy, rx, ry)` | circles and ellipses |
| `line(x1, y1, x2, y2)` | a line |
| `polyline(points)`, `polygon(points)` | open and closed shapes from `[[x, y], …]` |
| `path(segments)` | curves: `["M", x, y]`, `["L", x, y]`, `["Q", cx, cy, x, y]`, `["C", c1x, c1y, c2x, c2y, x, y]`, `["Z"]` |
| `text("…", x, y, size)` | live text (set `.family`, `.bold`, `.italic`) |
| `group([shapes])` | shapes that move together |
| `translate(s, dx, dy)`, `rotate(s, degrees, cx, cy)`, `scale(s, sx, sy)` | return the moved shape |
| `warn("…")` | adds a message shown to the user (`print` does the same) |

| `union_polygons(polys)` | merges any number of overlapping closed polygons (`[[x, y], …]`) into the outlines of the area they cover |
| `xf_rings(rings, sx, sy, dx, dy)` | moves and mirrors rings in one pass: `x' = sx * x + dx`, `y' = sy * y + dy` |
| `read_stl(data)` | reads an STL mesh (binary or ASCII) from a blob: a `Mesh` with `tri_count()`, `bounds()` (min x, y, z, max x, y, z), `groups(angle, dist)` (the planes of the model, largest first, as maps `#{ id, nx, ny, nz, d, area, tris, min_z, max_z }`), `project()` and `project(ids)` (the shadow on the floor of the whole model or of the chosen planes, as rings) and `section(z)` (the outlines where the plane at height `z` cuts it) |

**Big data and Rhai.** Rhai checks the size of every array after each change, so a script that pushes thousands of items into an
array gets slower and slower (the time grows with the square of the count). That is why the loops over the triangles of a mesh and
the union of thousands of polygons are done by the functions above, in native code; a script keeps its arrays small (the planes of
a model, the rings of an outline) and builds the shapes from them.

Put a shape on a layer with `s.layer = 3;` (0 is the first layer). Rhai strings are changed in place by `trim()` and
friends: `let t = line; t.trim();`.

Examples with tests are in `examples/plugins/`: a pack, an importer, two generators and two postprocessors.

## In the programs

LightCreator, MillCreator and PenCreator show **Settings → Plugins…** (the window), **File → Import → Import with a plugin…** and **Generate shape…**; they read plugin file types on Open and on drag and drop, and pass the G-code of **Export G-code…** through the chosen postprocessor (not the job sent straight to the machine). MillCreator and PenCreator also list the presets of plugins when a device is added, and MillCreator adds the bits of `mill-tools` data files to its library by name.

Not supported (yet): parameters for postprocessors, plugins that add operations to the toolpaths, and plugins that talk to the machine.

## For programs on baseCreator

* `base_core::plugins::Registry` loads the folder (`Registry::load(plugins_dir(base))`), switches plugins on and off,
  installs a zip, and answers `presets()`, `data()`, `importer_for()`, `generators()`, `postprocessors()`, `translate()`
  and `sync_clipart()`.
* `base_core::plugin_script`: `import`, `generate`, `postprocess` run a script and return `Shapes { scene, warnings }` or
  the lines; `check_output` is the check of the G-code lines.
* `base_core::registry` reads the official list (`parse_index`, `Entry::status`, `install`, `compare_versions`); the fetching is in `plugins_ui`.
* `cargo run -p base-core --example check_plugins -- <folder>` loads a folder of plugins and runs their scripts (the CI of the plugins repository uses it).
* `base_core::presets::with_plugins` adds the presets of plugins to a built-in list.
* `base_ui::plugins_ui::PluginsUi` is the state for the two windows (`window`, `generator_window`) and also
  `import_file(path, area)`, `import_extensions()`, `post_combo(ui)` and `postprocess(lines, area)`. A program shows the
  windows from menu items `MENU_PLUGINS` and `MENU_GENERATE`, adds the returned scene with `Scene::append`, and runs
  `postprocess` on the lines before it writes a G-code file. A program should also run its own checks on the result.
