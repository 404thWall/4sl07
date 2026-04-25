#!/usr/bin/env python3
"""SCP a file to N free TP machines and run it."""

from __future__ import annotations

import argparse
import subprocess
import sys
import time
import json
from pathlib import Path
from machines import MachineState
from multiprocessing import Process

REMOTE_PATH = "~/4sl07/deploy/"
CMD_TIMEOUT = 15  # seconds
CONFIG_FILE = Path("./deploy_command.json")

def load_config():
    if CONFIG_FILE.exists():
        try:
            with open(CONFIG_FILE, "r") as f:
                return json.load(f)
        except (json.JSONDecodeError, IOError):
            return {}
    return {}

def save_config(args_dict):
    to_save = {
        k: (str(v) if isinstance(v, Path) else v) 
        for k, v in args_dict.items() 
        if k not in ['save', 'kill'] and v is not None
    }
    with open(CONFIG_FILE, "w") as f:
        json.dump(to_save, f, indent=4)

def run_process(cmd: list[str]):
    try:
        subprocess.run(cmd, check=True)
    except subprocess.CalledProcessError as e:
        print(f"Command failed (exit {e.returncode})", file=sys.stderr)

def run_command_batch(cmd: list[str], user: str, hosts: list[str]):
    '''
    Run a command on multiple hosts in parallel using multiprocessing.
    The executed command is `cmd user@host cmd_args...`.
    '''
    processes: list[tuple[str, Process]] = []
    for host in hosts:
        command = [c.format(host=host, user=user) for c in cmd]
        process = Process(target=run_process, args=(command,))
        process.start()
        processes.append((host, process))
    
    start_time = time.time()
    for host, process in processes:
        process.join(timeout=max(0, CMD_TIMEOUT - (time.time() - start_time)))
        if process.is_alive():
            print(f"[{host}] Command is still running after {CMD_TIMEOUT} seconds, killing it to avoid a blocking situation...")
            process.terminate()
            process.join()

def run_command(cmd: list[str]):
    process = Process(target=subprocess.run, args=(cmd,), kwargs={"check": True})
    process.start()
    process.join(timeout=10)
    if process.is_alive():
        print(f"Command is still running after 10 seconds, killing it to avoid overloading the machines...")
        process.terminate()
        process.join()
    

def kill_previous_sessions(user: str) -> None:
    with open("deployed_hosts.txt", "a+") as f:
        f.seek(0)
        hosts = [line.strip() for line in f if line.strip()]
        batch_size = 5
        for i in range(0, len(hosts), batch_size):
            batch_hosts = hosts[i:min(i+batch_size, len(hosts))]
            print(f"Killing sessions on hosts: {', '.join(batch_hosts)} ({i+1} / {len(hosts)})...")
            run_command_batch(["ssh", "{user}@{host}", "tmux kill-session -t 4sl07"], user, batch_hosts)
            time.sleep(1)

def scp(user: str, host: str, file: Path) -> None:
    try:
        run_command(["ssh", f"{user}@{host}", f"mkdir -p {REMOTE_PATH}"])
        run_command(["scp", str(file), f"{user}@{host}:{REMOTE_PATH}"])
    except subprocess.CalledProcessError as e:
        print(f"[{host}] scp failed (exit {e.returncode})", file=sys.stderr)
        raise


def ssh_run(user: str, hosts: list[str], file: Path, cmd: str | None = None) -> None:
    command = cmd if cmd else f"{REMOTE_PATH}{file.name}"
    run_command_batch(["ssh", "{user}@{host}", f"chmod +x {REMOTE_PATH}{file.name} & tmux new -A -s 4sl07 -d {command}"], user, hosts)


def main() -> int:
    saved_prefs = load_config()
    parser = argparse.ArgumentParser(description="Deploy a file to free TP machines")

    parser.add_argument("file", type=Path, nargs="?", 
                        default=saved_prefs.get("file"), 
                        help="File to deploy")

    parser.add_argument("--user", type=str,
                        default=saved_prefs.get("user"), 
                        help="SSH username")

    parser.add_argument("--count", type=int, 
                        default=saved_prefs.get("count", 4), 
                        help="Number of machines")

    parser.add_argument("--cmd", type=str, 
                        default=saved_prefs.get("cmd"), 
                        help="Command to run instead of simply running the file")
                        
    parser.add_argument("--kill", action="store_true", 
                        help="Only kill previous sessions, do not deploy or run anything")

    parser.add_argument("--save", action="store_true", default=True, 
                        help="Save settings for next time (default: True)")
    parser.add_argument("--no-save", action="store_false", dest="save")

    args = parser.parse_args()

    if not args.user:
        parser.error("--user is required to do anything (none found in CLI or memory)")

    print("Killing previous sessions...")
    kill_previous_sessions(args.user)

    if args.kill:
        return 0
    
    missing = []
    if not args.file: missing.append("file (positional)")
    if not args.user: missing.append("--user")
    if missing:
        parser.error(f"Missing required parameters: {', '.join(missing)}")

    if not args.file.exists():
        print(f"File not found: {args.file}", file=sys.stderr)
        return 1

    if args.save:
        save_config(vars(args))
        print(f"Config updated in {CONFIG_FILE}")

    print(f"Deploying using {args}")

    state = MachineState()
    state.update()
    hosts = state.available()[: args.count]

    if not hosts:
        print("No free machines available.", file=sys.stderr)
        return 1

    print(f"[{hosts[0]}] scp...")
    scp(args.user, hosts[0], args.file)

    with open("deployed_hosts.txt", "w+") as f:
        batch_size = 5
        for i in range(0, len(hosts), batch_size):
            batch_hosts = hosts[i:min(i+batch_size, len(hosts))]
            for host in batch_hosts:
                f.write(f"{host}\n")
            f.flush()

            print(f"Running on hosts: {', '.join(batch_hosts)} ({i+1} / {len(hosts)})...")
            ssh_run(args.user, batch_hosts, args.file, args.cmd)
            time.sleep(1)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
