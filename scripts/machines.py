#!/usr/bin/env python3
"""TP machine availability from https://tp.telecom-paris.fr/."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

# Define your blacklisted hosts or prefixes here
BLACKLIST = {
    "tp-1a201",
    "tp-102",
}

@dataclass
class Machine:
    host: str
    free: bool
    sessions: int


class MachineState:
    def __init__(self, blacklist: set[str] | None = None) -> None:
        self.machines: list[Machine] = []
        self.blacklist = blacklist or set()

    def _is_blacklisted(self, host: str) -> bool:
        """Checks if a host or its room prefix is in the blacklist."""
        # Direct match check (e.g., 'tp-101-01')
        if host in self.blacklist:
            return True
        
        # Prefix match check (e.g., if 'tp-102' is blacklisted, matches 'tp-102-05')
        if any(host.startswith(b) for b in self.blacklist):
            return True
            
        return False

    def update(self) -> None:
        req = Request("https://tp.telecom-paris.fr/ajax.php", headers={"User-Agent": "Mozilla/5.0"})
        with urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())
        self.machines = [
            Machine(
                host=row[0],
                free=row[1] is True and sum(v for v in row[2:] if isinstance(v, int)) == 0,
                sessions=sum(v for v in row[2:] if isinstance(v, (int, float))),
            )
            for row in data.get("data", [])
            if isinstance(row, list) and len(row) >= 2 and isinstance(row[0], str)
            if not self._is_blacklisted(row[0])  # <-- Filter added here
        ]

    def available(self) -> list[str]:
        return [m.host for m in self.machines if m.free]
