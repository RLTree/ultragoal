"""Regular-file reads and atomic deterministic writes."""

from __future__ import annotations

import os
from enum import IntEnum
from pathlib import Path
import tempfile


class ProjectionFilesystemError(OSError):
    pass


class ProjectionFileMode(IntEnum):
    REGULAR = 0o644
    EXECUTABLE = 0o755


def regular_bytes(path: Path) -> bytes:
    if path.is_symlink() or not path.is_file():
        raise ProjectionFilesystemError(f"canonical regular file missing: {path}")
    return path.read_bytes()


def atomic_write(path: Path, content: bytes, mode: ProjectionFileMode) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, mode.value)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
