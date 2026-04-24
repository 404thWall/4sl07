#!/usr/bin/env python3
"""TP machine availability from https://tp.telecom-paris.fr/."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

@dataclass
class Machine:
    host: str
    free: bool
    sessions: int


class MachineState:
    def __init__(self) -> None:
        self.machines: list[Machine] = []

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
        ]

    def available(self) -> list[str]:
        return [m.host for m in self.machines if m.free]
