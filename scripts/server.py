#!/usr/bin/env python3
"""Minimal TCP server that returns system load information from uptime."""

from __future__ import annotations

import socketserver
import subprocess

HOST = '0.0.0.0'
PORT = 24813

def get_uptime() -> bytes:
    try:
        result = subprocess.run(
            ["uptime"],
            check=True,
            capture_output=True,
            text=True,
        )
        return (result.stdout.strip() + "\n").encode("utf-8")
    except subprocess.CalledProcessError as exc:
        return f"ERROR: uptime failed with exit code {exc.returncode}\n".encode("utf-8")


class UptimeTCPHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        # Read and ignore incoming data; any request triggers an uptime response.
        _ = self.request.recv(4096)
        self.request.sendall(get_uptime())


def main() -> int:
    with socketserver.ThreadingTCPServer((HOST, PORT), UptimeTCPHandler) as server:
        print(f"Listening on tcp://{HOST}:{PORT}")
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
