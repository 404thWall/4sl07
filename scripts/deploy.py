#!/usr/bin/env python3
"""SCP a file to N free TP machines and run it."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from machines import MachineState

REMOTE_PATH = "~/4sl07/deploy/"

def kill_previous_sessions(user: str) -> None:
    with open("deployed_hosts.txt", "r") as f:
        for line in f:
            host = line.strip()
            try:
                subprocess.run(["ssh", f"{user}@{host}", "tmux kill-session -t 4sl07"], check=True)
            except subprocess.CalledProcessError:
                pass  # Ignore errors (e.g. session not found)

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
    args = parser.parse_args()

    if not args.file.exists():
        print(f"File not found: {args.file}", file=sys.stderr)
        return 1

    state = MachineState()
    state.update()
    hosts = state.available()[: args.count]

    if not hosts:
        print("No free machines available.", file=sys.stderr)
        return 1
    
    print("Killing previous sessions...")
    kill_previous_sessions(args.user)

    print(f"[{hosts[0]}] scp...")
    scp(args.user, hosts[0], args.file)

    for host in hosts:
        print(f"[{host}] starting...")
        ssh_run(args.user, host, args.file, args.cmd)

    with open("deployed_hosts.txt", "w") as f:
        for host in hosts:
            f.write(f"{host}\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
