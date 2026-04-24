#!/usr/bin/env python3
"""TCP client: reads a machine list, appends .enst.fr, and queries the uptime server."""

from __future__ import annotations

import socket
import sys
from pathlib import Path

PORT = 24813
FILE = "machines.txt"
DOMAIN = ".enst.fr"


def query(host: str) -> str:
    with socket.create_connection((host, PORT), timeout=5) as sock:
        sock.sendall(b"ping\n")
        chunks: list[bytes] = []
        while chunk := sock.recv(4096):
            chunks.append(chunk)
    return b"".join(chunks).decode("utf-8", errors="replace").strip()


def main() -> int:
    for line in Path(FILE).read_text().splitlines():
        if not line.strip():
            continue
        host = line.strip() + DOMAIN
        try:
            print(f"{host}: {query(host)}")
        except OSError as exc:
            print(f"{host}: connection failed – {exc}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
