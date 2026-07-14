"""Strict JSON and normalized value parsing for authority definitions."""

from __future__ import annotations

import json
from pathlib import Path, PurePosixPath


class AuthorityContractError(ValueError):
    pass


def strict_json(content: bytes, path: Path) -> object:
    def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
        result: dict[str, object] = {}
        for key, value in pairs:
            if key in result:
                raise AuthorityContractError(f"duplicate JSON key in {path}: {key}")
            result[key] = value
        return result

    try:
        return json.loads(content, object_pairs_hook=unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise AuthorityContractError(f"invalid JSON in {path}: {error}") from error


def safe_path(value: object, field: str, path: Path) -> str:
    if not isinstance(value, str) or not value or value.strip() != value or "\\" in value:
        raise AuthorityContractError(f"invalid {field} in {path}")
    parsed = PurePosixPath(value)
    if (
        parsed.is_absolute()
        or str(parsed) != value
        or any(part in {"", ".", ".."} for part in parsed.parts)
    ):
        raise AuthorityContractError(f"unsafe {field} in {path}")
    return value


def string_tuple(value: object, field: str, path: Path, paths: bool = False) -> tuple[str, ...]:
    if not isinstance(value, list) or not value:
        raise AuthorityContractError(f"invalid {field} in {path}")
    parsed = tuple(
        safe_path(item, field, path) if paths else item
        for item in value
        if isinstance(item, str) and item
    )
    if len(parsed) != len(value) or parsed != tuple(sorted(set(parsed))):
        raise AuthorityContractError(f"noncanonical {field} in {path}")
    return parsed


def exact(row: object, fields: set[str], path: Path) -> dict[str, object]:
    if not isinstance(row, dict) or set(row) != fields:
        raise AuthorityContractError(f"invalid authority row shape in {path}")
    return row


def required_text(value: object, field: str, path: Path) -> str:
    if (
        not isinstance(value, str)
        or not value
        or value.strip() != value
        or any(ord(character) < 32 for character in value)
    ):
        raise AuthorityContractError(f"invalid {field} in {path}")
    return value


def required_token(value: object, field: str, path: Path) -> str:
    text = required_text(value, field, path)
    if any(not (character.isalnum() or character in ".+_-") for character in text):
        raise AuthorityContractError(f"invalid {field} token in {path}")
    return text
