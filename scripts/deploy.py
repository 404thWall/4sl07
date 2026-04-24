#!/usr/bin/env python3
"""SCP a file to N free TP machines and run it."""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path
from machines import MachineState

REMOTE_PATH = "~/4sl07/deploy/"

def kill_previous_sessions(user: str) -> None:
    with open("deployed_hosts.txt", "a+") as f:
        f.seek(0)
        for line in f:
            host = line.strip()
            try:
                subprocess.run(["ssh", f"{user}@{host}", "tmux kill-session -t 4sl07"], check=True)
            except subprocess.CalledProcessError as e:
                print(f"[{host}] failed to kill session (exit {e.returncode}), maybe it was already killed?", file=sys.stderr)
                pass  # Ignore errors (e.g. session not found)
            time.sleep(1)

def scp(user: str, host: str, file: Path) -> None:
    try:
        subprocess.run(["ssh", f"{user}@{host}", f"mkdir -p {REMOTE_PATH}"], check=True)
        subprocess.run(["scp", str(file), f"{user}@{host}:{REMOTE_PATH}"], check=True)
    except subprocess.CalledProcessError as e:
        print(f"[{host}] scp failed (exit {e.returncode})", file=sys.stderr)
        raise


def ssh_run(user: str, host: str, file: Path, cmd: str | None = None) -> None:
    command = cmd if cmd else f"{REMOTE_PATH}{file.name}"
    try:
        subprocess.run(["ssh", f"{user}@{host}", f"chmod +x {REMOTE_PATH}{file.name} & tmux new -A -s 4sl07 -d {command}"], check=True)
    except subprocess.CalledProcessError as e:
        print(f"[{host}] ssh failed (exit {e.returncode})", file=sys.stderr)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description="Deploy a file to free TP machines")
    parser.add_argument("file", type=Path, help="File to deploy")
    parser.add_argument("--user", required=True, help="SSH username")
    parser.add_argument("--count", type=int, default=4, help="Number of machines")
    parser.add_argument("--cmd", type=str, help="Command to run instead of the file")
    parser.add_argument("--kill", help="Only kill previous sessions, do not deploy or run anything", action="store_true")
    args = parser.parse_args()

    print("Killing previous sessions...")
    kill_previous_sessions(args.user)

    if args.kill:
        return 0

    if not args.file.exists():
        print(f"File not found: {args.file}", file=sys.stderr)
        return 1

    state = MachineState()
    state.update()
    hosts = state.available()[: args.count]

    if not hosts:
        print("No free machines available.", file=sys.stderr)
        return 1

    print(f"[{hosts[0]}] scp...")
    scp(args.user, hosts[0], args.file)

    with open("deployed_hosts.txt", "w+") as f:
        i = 0
        for host in hosts:
            i += 1
            print(f"[{host}] starting ({i}/{len(hosts)})...")
            f.write(f"{host}\n")
            f.flush()
            ssh_run(args.user, host, args.file, args.cmd)
            time.sleep(1)
            if i % 30 == 0:
                print("Sleeping for 15 seconds to avoid overloading the machines...")
                time.sleep(15)
                print("Resuming...")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
