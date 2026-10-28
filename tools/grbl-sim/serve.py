#!/usr/bin/env python3
"""Expose grbl-sim on a TCP port, the same way ser2net exposes a real laser.

    python3 tools/grbl-sim/serve.py [--port 3333] [--sim PATH]

Then in an application (LightCreator, MillCreator, PenCreator): device settings -> connection "TCP", host 127.0.0.1, port 3333, controller GRBL.

Every connection starts a fresh simulator in its own temporary directory, so each session begins with GRBL's
default settings. The connection is handed over once the simulator has booted, like ser2net in front of a running
controller. Standard library only (uses a pseudo-terminal), works on Linux and macOS.
"""
import argparse
import os
import pty
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import tty

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
DEFAULT_SIM = os.path.join(ROOT, "target", "grbl-sim", "grbl", "grbl", "sim", "grbl_sim.exe")


def pump_to_sim(conn, fd, proc):
    try:
        while True:
            data = conn.recv(4096)
            if not data:
                break
            os.write(fd, data)
    except OSError:
        pass
    finally:
        proc.kill()


def pump_to_client(conn, fd):
    try:
        while True:
            data = os.read(fd, 4096)
            if not data:
                break
            conn.sendall(data)
    except OSError:
        pass  # EIO once the simulator has exited
    finally:
        try:
            conn.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass


def wait_for_boot(fd, timeout=10.0):
    """Swallow the simulator's start-up output up to its banner, so the client meets an already running GRBL, as it
    would behind ser2net. A fresh simulator has an empty EEPROM and prints `error:7` and its settings first."""
    import select
    import time
    seen = b""
    end = time.monotonic() + timeout
    while b"['$' for help]" not in seen and time.monotonic() < end:
        ready, _, _ = select.select([fd], [], [], 0.1)
        if ready:
            try:
                seen += os.read(fd, 4096)
            except OSError:
                return


def serve_one(conn, addr, sim, verbose):
    workdir = tempfile.mkdtemp(prefix="grbl-sim-")
    # grbl-sim block-buffers its output on a pipe, so it gets a raw pseudo-terminal, like `socat ... pty,raw,echo=0`.
    master, slave = pty.openpty()
    tty.setraw(slave)
    # -n: plain GRBL responses, -b/-s: block and step traces go to files instead of the serial stream.
    args = [sim, "-n", "-b", os.path.join(workdir, "block.out"), "-s", os.path.join(workdir, "step.out")]
    proc = subprocess.Popen(args, cwd=workdir, stdin=slave, stdout=slave, stderr=subprocess.DEVNULL, close_fds=True)
    os.close(slave)
    wait_for_boot(master)
    if verbose:
        print(f"[grbl-sim] {addr[0]}:{addr[1]} connected, traces in {workdir}", flush=True)
    threading.Thread(target=pump_to_sim, args=(conn, master, proc), daemon=True).start()
    pump_to_client(conn, master)
    proc.wait()
    os.close(master)
    conn.close()
    if verbose:
        print(f"[grbl-sim] {addr[0]}:{addr[1]} disconnected", flush=True)
    else:
        shutil.rmtree(workdir, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=3333, help="0 picks a free port")
    ap.add_argument("--sim", default=os.environ.get("GRBL_SIM_EXE") or os.environ.get("LC_GRBL_SIM_EXE") or DEFAULT_SIM, help="path to grbl_sim.exe")
    ap.add_argument("-v", "--verbose", action="store_true", help="log connections and keep the step / block traces")
    a = ap.parse_args()
    a.sim = os.path.abspath(a.sim)  # the simulator runs in a temporary directory
    if not os.access(a.sim, os.X_OK):
        sys.exit(f"grbl-sim not found at {a.sim}; build it with tools/grbl-sim/build.sh")
    srv = socket.create_server((a.host, a.port), reuse_port=False)
    # With --port 0 the OS picks a free port; the line below tells the caller which one.
    print(f"[grbl-sim] listening on {a.host}:{srv.getsockname()[1]}", flush=True)
    try:
        while True:
            conn, addr = srv.accept()
            conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            threading.Thread(target=serve_one, args=(conn, addr, a.sim, a.verbose), daemon=True).start()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
