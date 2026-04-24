#!/usr/bin/env python3
"""TCP client: reads a machine list, appends .enst.fr, and queries the uptime server."""

from __future__ import annotations

import socket
import sys
from pathlib import Path

PORT = 24813
FILE = "deployed_hosts.txt"
DOMAIN = ".enst.fr"

def extract_load(uptime_str: str) -> float | None:
    """Extracts the 1-minute load average from an uptime string."""
    try:
        if "load average:" not in uptime_str:
            return None
        
        load_part = uptime_str.split("load average:")[1].strip()
        
        # 2. Split by ', ' (comma followed by space) to separate the three averages
        # Example: "0,12, 0,06, 0,02" -> ["0,12", "0,06", "0,02"]
        individual_loads = load_part.split(", ")
        
        load_1min_raw = individual_loads[0].replace(",", ".")
        load_5min_raw = individual_loads[1].replace(",", ".")
        load_15min_raw = individual_loads[2].replace(",", ".")
        
        return float(load_1min_raw), float(load_5min_raw), float(load_15min_raw)
    except (ValueError, IndexError):
        return None


def query(host: str) -> str:
    with socket.create_connection((host, PORT), timeout=5) as sock:
        sock.sendall(b"ping\n")
        chunks: list[bytes] = []
        while chunk := sock.recv(4096):
            chunks.append(chunk)
    return b"".join(chunks).decode("utf-8", errors="replace").strip()


def main() -> int:
    all_loads: list[(float, float, float)] = []

    for line in Path(FILE).read_text().splitlines():
        if not line.strip():
            continue
        host = line.strip() + DOMAIN
        try:
            result = query(host)
            print(f"{host}: {result}")
            loads = extract_load(result)
            if loads is not None:
                all_loads.append(loads)
            else:
                print(f"  --> Warning: Could not parse load from output.")
        except OSError as exc:
            print(f"{host}: connection failed - {exc}")

    if all_loads:
        # zip(*all_loads) unpacks [(1,5,15), (1,5,15)] into three separate iterables
        avg_1, avg_5, avg_15 = [sum(x) / len(x) for x in zip(*all_loads)]
        
        print("\n" + "="*45)
        print(f"Summary for {len(all_loads)} servers:")
        print(f"Global Average (1 min) : {avg_1:.2f}")
        print(f"Global Average (5 min) : {avg_5:.2f}")
        print(f"Global Average (15 min): {avg_15:.2f}")
        print("="*45)
    else:
        print("\nNo load data collected.")
    

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
