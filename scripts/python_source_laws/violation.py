"""Stable Python source-law diagnostics."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, order=True)
class SourceViolation:
    path: str
    line: int
    code: str

    def diagnostic(self) -> str:
        return f"python_source_law:{self.code}:{self.path}:{self.line}"
