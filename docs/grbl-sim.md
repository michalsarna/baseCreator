# Testing with the GRBL simulator

[grbl-sim](https://github.com/grbl/grbl-sim) compiles the real GRBL 1.1h firmware sources into a program that runs
on the PC and talks the GRBL serial protocol on its standard input and output. It answers `ok` / `error:N`, sends
status reports, runs the planner and moves its simulated axes in roughly real time, so the streaming, status, feed
hold and reset handling of the programs (LightCreator, MillCreator, PenCreator) can be checked without a machine.

What it does **not** cover: the USB-serial link itself, laser power output, limit switches and homing, and the
exact timing of a real ATmega328P. A successful run against the simulator is a good sign, not a replacement for a
low-power test on the machine.

## Build the simulator

Linux or macOS, needs `git`, `make` and a C compiler:

```sh
tools/grbl-sim/build.sh
```

This fetches GRBL `v1.1h.20190825` and grbl-sim into `target/grbl-sim/` and prints the path of the binary
(`target/grbl-sim/grbl/grbl/sim/grbl_sim.exe`; the `.exe` is grbl-sim's own naming). `GRBL_REF` and `SIM_REF`
select other versions.

The script applies patches with fixes found while testing the programs, all of them about running the AVR code
on a PC; none changes what GRBL does on a real controller:

`grbl-sim-fixes.patch`:

* **Replies held back.** grbl-sim polls its input with `select()` but reads it with buffered `getchar()`, so the rest
  of a line waited in stdio's buffer until more input arrived and every `ok` came one line late. A
  character-counting sender then crawls along at the pace of its status polls.
* **Syscalls on every poll.** It switched the terminal mode around every input poll, tens of thousands of syscalls
  per simulated second. The patched simulator expects a raw terminal, which `serve.py` and the `socat … raw` line
  below provide; typed by hand in a normal terminal, input arrives line by line.
* **Steppers stalling.** Interrupts run on their own thread, and both threads saved, cleared and restored the one
  shared interrupt flag (`sreg = SREG; cli(); … SREG = sreg;`). Interleaved, they could leave interrupts disabled:
  the machine then reports `Run` at full feed while hardly moving. This showed up with a full planner and status
  polls running, i.e. during any real job. The interrupt thread now has its own SREG.
* **Orphaned simulators.** It ignored a hung-up terminal, so a simulator whose bridge was killed kept running and
  kept two CPU cores busy. It now exits when its input hangs up.
* **A byte let in that GRBL then dropped.** The simulator reads a byte from its input only when GRBL's receive ring has
  room, but it tested the ring as one slot too small (`RX_BUFFER_SIZE` instead of `RX_BUFFER_SIZE + 1`): with the ring
  full at the end of the array a byte got through and was thrown away, and two lines of a job ran together
  (`error:24`) or one was garbled (`error:1`, `error:3`).

`grbl-sim-macos.patch` (applied on macOS only):

* **A hundred times too slow.** The Mach clock service costs a message to the kernel for every call and the simulator
  asks for the time once per simulated clock tick. The clock of `clock_gettime_nsec_np` is as accurate and cheap.
  `build.sh` also adds the Mach headers that `platform_OSX.c` forgets to include, which recent clang refuses.

`grbl-fixes.patch`, the only changes to GRBL's own sources:

* **Reset requests lost.** GRBL sets and clears its
  realtime flags with a read-modify-write inside `cli()`, atomic on an AVR. In the simulator the serial interrupt
  runs on another thread, so a reset (0x18) could be overwritten by the main loop clearing the status-report flag
  at the same moment; GRBL then stayed in `Alarm` and never restarted. These four updates are made atomic.
* **Serial buffers shared by two threads.** GRBL's receive and transmit rings are written by the interrupt thread and read
  by the main thread (or the other way round) with nothing but `volatile`, which on a PC allows the data to be seen
  after the new head, and `UCSR0B |= …` / `&= ~…` of the data-register-empty interrupt is a read-modify-write that two
  threads can overwrite (the reply then waits for the next write). The heads and tails are stored with release and read
  with acquire, the register is changed atomically and the interrupt handler looks again after clearing the flag.

## Use it from the app

```sh
python3 tools/grbl-sim/serve.py            # listens on 127.0.0.1:3333
```

In a program open its device settings, choose controller **GRBL**, connection **TCP**, host `127.0.0.1`,
port `3333`, and connect. The simulator behaves like a laser shared with ser2net. Every connection starts a fresh
simulator with GRBL's default settings; the connection is accepted once it has booted, so the console shows the
`Grbl 1.1h` banner that answers the soft reset the program sends on connect.

The defaults are slow (`$110`/`$111` = 500 mm/min, 10 mm/s² acceleration). For quicker runs send, in the console:

```
$110=30000
$111=30000
$120=5000
$121=5000
$32=1
```

`-v` keeps each session's step and block traces (`step.out`, `block.out`) and prints where they are, which is
useful for checking what the planner actually executed.

Prefer a serial port? On Linux `socat` gives the simulator a virtual one:

```sh
socat PTY,raw,link=/tmp/ttyGRBL,echo=0 "EXEC:'target/grbl-sim/grbl/grbl/sim/grbl_sim.exe -n -s /dev/null -b /dev/null',pty,raw,echo=0"
```

and enter `/tmp/ttyGRBL` as the port (it does not appear in the port list).

## Automated tests

The harness is `base_ui::sim_test` (`crates/base-ui/src/sim_test.rs`, behind the feature `sim-tests`): `Sim` starts
`serve.py` on a free port, `Session` connects the real serial worker (`link.rs`) to it and collects what it reports,
with waits that give up only when nothing happens any more (a slow runner is not a failure) and a description of the
line each `error:` answers. The tests that do not depend on a program live in the same file:

| Test | Checks |
|------|--------|
| `grbl_sim_keeps_up_with_real_time` | the simulator itself runs at real speed (the other tests rely on it) |
| `grbl_sim_long_job_keeps_character_counting_in_sync` | hundreds of short lines with the 120-byte window always full |
| `grbl_sim_rejected_line_does_not_desync_the_stream` | an unsupported command gets `error:20` and the rest of the job still runs |
| `grbl_sim_feed_hold_and_resume` | `!` stops the machine (`Hold:0`), `~` resumes and the job completes |
| `grbl_sim_abort_then_new_job_runs` | a soft reset mid-job, then a new job streams normally |
| `grbl_sim_job_sent_right_after_connect_is_not_lost` | a job started while GRBL is still restarting after connect completes |
| `grbl_sim_job_stops_at_a_pause_and_goes_on_when_told` | `M0` (change the bit / the pen): the job waits, nothing moves, `Continue` goes on |
| `grbl_sim_settings_can_be_read_changed_backed_up_and_restored` | `$$` report, a change, the backup text and its restore |
| `grbl_sim_spindle_and_arcs_run_and_end_where_expected` | `M3 S…`, `G4`, a plunge in Z, an arc, back to zero |

They are ignored in a normal `cargo test`. Run them with:

```sh
tools/grbl-sim/build.sh
cargo test -p base-ui --features sim-tests sim_test -- --ignored --test-threads=1
```

### In a program

A program tests what is its own (the G-code it generates) against the same simulator:

```toml
[dev-dependencies]
base-ui = { git = "https://github.com/michalsarna/baseCreator", tag = "v…", features = ["sim-tests"] }
```

```rust
use base_ui::sim_test::Session;

#[test]
#[ignore = "needs grbl-sim"]
fn my_job_runs() {
    let mut s = Session::connect();
    s.wait_banner();
    s.fast_machine(&[]);          // quick axes; extra settings of the program as the argument (`$32=1`)
    let job = my_program::generate_job();
    let n = job.len();
    s.run_job(job, 60.0);
    s.wait_idle_at(0.0, 0.0, 30.0);
    assert!(s.errors().is_empty(), "{:?}", s.errors_described());
}
```

The simulator is looked for in `target/grbl-sim` of the workspace being tested, or at `GRBL_SIM_EXE`. In CI build it with
the action of this repository, at the tag the program is pinned to (it caches the build):

```yaml
- uses: michalsarna/baseCreator/.github/actions/grbl-sim@v…
- run: cargo test -p myprogram sim -- --ignored --test-threads=1
```

## grblHAL

For machines running [grblHAL](https://github.com/grblHAL) there is a matching
[grblHAL Simulator](https://github.com/grblHAL/Simulator). It is built with CMake and started the same way
(`grblHAL_sim -n -s step.out -b block.out`), so `serve.py --sim path/to/grblHAL_sim` should work with it too;
this has not been tried yet.

## Known limits

grbl-sim simulates the 16 MHz AVR one clock tick at a time and keeps about two CPU cores busy while it runs.
`grbl_sim_keeps_up_with_real_time` reports when a machine is too busy for it to keep real time.
