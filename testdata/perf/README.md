# The performance test project

`perf-block.cre` is the project on which the speed of LightCreator, MillCreator and PenCreator is measured before every release:

- a **block of 300 × 400 × 500 mm** (the `faces` section): on each of its six faces **10 texts** and **100 objects** (squares, circles,
  ellipses, hearts, stars, polylines and a few dwarfs of a real project) on ten layers;
- **5000 objects beside the work areas** (x from 5 to 9 m), among them big dwarfs: no preview may show them, and they must not make it
  slow;
- the layers of every program: LightCreator lines, fills, an offset fill and fill with line (`laser`), MillCreator its default
  operations (cut out, holes, pocket, engrave, drill: `mill`), PenCreator outlines, fills and both (`pen`). PenCreator has no
  blocks and works on face A.

A copy of it is kept in `~/Documents/test/perf-block.cre`; `scripts/perf.sh` checks that both are the same.

## How it is made

Never by hand:

1. `cargo run --release -p base-core --example perf_project -- testdata/perf/perf-block.cre <a LightCreator project with dwarfs>`
   (the drawings of the faces);
2. in LightCreator, MillCreator and PenCreator (in this order):
   `cargo test --release -p <program> perf_prepare -- --ignored` (the layers of each program);
3. `cp testdata/perf/perf-block.cre ~/Documents/test/`.

## How it is measured

`scripts/perf.sh` (the repositories of the programs beside this one) runs in each program
`cargo test --release -p <program> perf_open -- --ignored --nocapture --test-threads=1` and adds the results to
[RESULTS.md](RESULTS.md):

- **open**: the project is read into the program;
- **gcode**: the G-code of every layer, of every face (with the postprocessors), as it is saved;
- **preview**: the preview of every face, the job and one frame of its tab.

The times are milliseconds on the machine named in the row; compare rows of the same machine and of the same file (its SHA-256).
