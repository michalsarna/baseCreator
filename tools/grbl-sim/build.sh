#!/usr/bin/env bash
# Build the GRBL simulator (grbl-sim) from the real GRBL 1.1h firmware sources.
#
#   tools/grbl-sim/build.sh [output-dir]      default: target/grbl-sim
#
# The result is `<output-dir>/grbl/grbl/sim/grbl_sim.exe` (the `.exe` suffix is grbl-sim's own naming, it is a
# normal Linux / macOS binary). It speaks the exact GRBL serial protocol on stdin / stdout, so the applications can be
# tested against the real firmware code without an Arduino; see docs/grbl-sim.md.
set -euo pipefail

GRBL_REF="${GRBL_REF:-v1.1h.20190825}"   # gnea/grbl tag
SIM_REF="${SIM_REF:-master}"             # grbl/grbl-sim branch or commit

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="${1:-$root/target/grbl-sim}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"

if [ ! -d "$out/grbl/.git" ]; then
    # Leftovers of an earlier build or a partly restored cache (no clone, so git would refuse to clone into it).
    rm -rf "$out/grbl"
    git -c advice.detachedHead=false clone --quiet --depth 1 --branch "$GRBL_REF" https://github.com/gnea/grbl.git "$out/grbl"
fi
sim="$out/grbl/grbl/sim"
if [ ! -d "$sim/.git" ]; then
    git clone --quiet https://github.com/grbl/grbl-sim.git "$sim"
    git -C "$sim" -c advice.detachedHead=false checkout --quiet "$SIM_REF"
fi

# Simulator-only fixes, see the comments in the patches and docs/grbl-sim.md:
#  grbl-sim-fixes.patch
#  * input read unbuffered (each reply used to wait for the next byte to arrive),
#  * no terminal mode switching around every input poll (tens of thousands of syscalls per simulated second),
#  * a separate SREG for the interrupt thread (a race could leave interrupts disabled and stall the steppers),
#  * exit when the input hangs up (a simulator whose bridge was killed kept running);
#  grbl-fixes.patch
#  * atomic updates of GRBL's realtime flags: on an AVR cli() makes them atomic, in grbl-sim the interrupts run on
#    another thread, and a reset request could be lost.
apply_patch() { # repo, patch
    if git -C "$1" apply --check "$here/$2" 2>/dev/null; then
        git -C "$1" apply "$here/$2"
    elif ! git -C "$1" apply --reverse --check "$here/$2" 2>/dev/null; then
        echo "$2 does not apply to $1" >&2
        exit 1
    fi
}
apply_patch "$sim" grbl-sim-fixes.patch
apply_patch "$out/grbl" grbl-fixes.patch
[ "$(uname -s)" = Darwin ] && apply_patch "$sim" grbl-sim-macos.patch

extra=""
case "$(uname -s)" in
    # platform_OSX.c uses the Mach clock calls without including their headers; recent clang refuses implicit declarations.
    Darwin) platform=OSX; extra="-include mach/mach.h -include mach/clock.h" ;;
    *) platform=LINUX ;;
esac

# -fcommon: grbl-sim defines globals in headers, which GCC >= 10 / recent clang reject by default
# ("multiple definition of `wdt'").
make -C "$sim" --no-print-directory new PLATFORM="$platform" FLAGS="-g -O2 -fcommon -w $extra" >/dev/null

echo "$sim/grbl_sim.exe"
