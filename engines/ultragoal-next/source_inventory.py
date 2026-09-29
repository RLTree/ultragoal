"""Exact root-relative authored and build-generated inputs for build and package."""
from __future__ import annotations
import hashlib
import os
import re
import stat
import subprocess
import sys
from pathlib import Path

ROOT_CACHE_DIRS = frozenset({'target', '.ruff_cache'})
GENERATED = {
    'Identity.bend': 'build.py: rule/parser closure identities',
    'src/native_identity.rs': 'build.py: syn wrapper/toolchain identity',
}

SOURCE_MACROS = frozenset({'include', 'include_str', 'include_bytes'})

def rust_parse_limit() -> int:
    """Admit the expanding token scan only when current host memory can hold it."""
    try:
        physical = os.sysconf('SC_PHYS_PAGES') * os.sysconf('SC_PAGE_SIZE')
        if sys.platform == 'darwin':
            result = subprocess.run(['/usr/bin/vm_stat'], capture_output=True, text=True,
                                    check=True, timeout=2)
            page = int(re.search(r'page size of (\d+) bytes', result.stdout).group(1))
            counts = {name: int(re.search(rf'^Pages {name}:\s*(\d+)\.', result.stdout,
                                          re.MULTILINE).group(1))
                      for name in ('free', 'inactive', 'speculative')}
            available = sum(counts.values()) * page
        elif sys.platform.startswith('linux'):
            available = int(re.search(r'^MemAvailable:\s*(\d+) kB',
                                      Path('/proc/meminfo').read_text(), re.MULTILINE).group(1)) * 1024
        else:
            raise ValueError('unsupported host')
    except (OSError, ValueError, AttributeError, subprocess.SubprocessError) as error:
        raise ValueError(f'source_headroom_unavailable: {error}') from error
    # Token tuples and decoded text can multiply the source bytes manyfold.
    limit = max(0, available - physical // 8) // 64
    if not limit:
        raise ValueError('source_resource_pressure: Rust token headroom unavailable')
    return limit

def signature(info: os.stat_result) -> tuple[int, int, int, int, int]:
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)

def source_fd(path: Path) -> tuple[int, os.stat_result]:
    flags = os.O_RDONLY | os.O_CLOEXEC | os.O_NONBLOCK | os.O_NOFOLLOW
    fd = os.open(path, flags)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or signature(path.lstat()) != signature(info):
            raise ValueError(f'source_changed_or_not_regular: {path}')
        return fd, info
    except BaseException:
        os.close(fd)
        raise

def admitted_bytes(path: Path, limit: int) -> bytes:
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
        raise ValueError(f'source_resource_pressure: {path}: bytes={info.st_size}:headroom={limit}')
    fd, opened = source_fd(path)
    with os.fdopen(fd, 'rb') as stream:
        data = stream.read(limit + 1)
        after = os.fstat(stream.fileno())
    if len(data) > limit:
        raise ValueError(f'source_resource_pressure: {path}: bytes>{limit}')
    if len(data) != opened.st_size or signature(after) != signature(opened) or signature(path.lstat()) != signature(opened):
        raise ValueError(f'source_changed: {path}')
    return data

def rust_text(path: Path, limit: int) -> str:
    try:
        return admitted_bytes(path, limit).decode('utf-8')
    except UnicodeError as error:
        raise ValueError(f'source_not_utf8: {path}') from error

def stream_digest(path: Path) -> str:
    fd, opened = source_fd(path)
    digest = hashlib.sha256()
    count = 0
    with os.fdopen(fd, 'rb') as stream:
        while chunk := stream.read(1024 * 1024):
            count += len(chunk)
            digest.update(chunk)
        after = os.fstat(stream.fileno())
    if count != opened.st_size or signature(after) != signature(opened) or signature(path.lstat()) != signature(opened):
        raise ValueError(f'source_changed: {path}')
    return digest.hexdigest()

def rust_tokens(source: str):
    """Small Rust token reader for source-bearing attributes and macros only."""
    tokens = []
    i = 0
    while i < len(source):
        if source[i].isspace():
            i += 1
        elif source.startswith('//', i):
            end = source.find('\n', i)
            i = len(source) if end < 0 else end
        elif source.startswith('/*', i):
            depth = 1
            i += 2
            while i < len(source) and depth:
                if source.startswith('/*', i): depth += 1; i += 2
                elif source.startswith('*/', i): depth -= 1; i += 2
                else: i += 1
            if depth: raise ValueError('unterminated Rust block comment')
        elif source.startswith(('r"', 'r#', 'br"', 'br#', 'cr"', 'cr#'), i):
            prefix = 2 if source.startswith(('br', 'cr'), i) else 1
            start = i + prefix
            hashes = 0
            while start + hashes < len(source) and source[start + hashes] == '#': hashes += 1
            quote = start + hashes
            if quote >= len(source) or source[quote] != '"':
                tokens.append(('ident', source[i:i + prefix]))
                i += prefix
                continue
            marker = '"' + '#' * hashes
            end = source.find(marker, quote + 1)
            if end < 0: raise ValueError('unterminated Rust raw string')
            tokens.append(('string', source[quote + 1:end]))
            i = end + len(marker)
        elif source.startswith("b'", i) or source[i] == "'":
            quote = i + (source[i] == 'b')
            if quote == i and quote + 1 < len(source) and (source[quote + 1].isascii() and
                    (source[quote + 1].isalpha() or source[quote + 1] == '_')):
                end = quote + 2
                while end < len(source) and source[end].isascii() and (source[end].isalnum() or source[end] == '_'): end += 1
                if end >= len(source) or source[end] != "'":
                    tokens.append(('lifetime', source[quote + 1:end]))
                    i = end
                    continue
            end = quote + 1
            while end < len(source) and source[end] not in ("'", '\n'):
                end += 2 if source[end] == '\\' else 1
            if end >= len(source) or source[end] != "'":
                raise ValueError('unterminated Rust character literal')
            tokens.append(('char', source[quote + 1:end]))
            i = end + 1
        elif source[i] == '"' or source.startswith('b"', i):
            quote = i + (source[i] == 'b')
            i = quote + 1
            chars = []
            while i < len(source) and source[i] != '"':
                if source[i] == '\\':
                    chars.append(source[i:i + 2]); i += 2
                else:
                    chars.append(source[i]); i += 1
            if i >= len(source): raise ValueError('unterminated Rust string')
            tokens.append(('string', ''.join(chars)))
            i += 1
        elif source[i].isascii() and (source[i].isalpha() or source[i] == '_'):
            end = i + 1
            while end < len(source) and source[end].isascii() and (source[end].isalnum() or source[end] == '_'): end += 1
            tokens.append(('ident', source[i:end]))
            i = end
        else:
            tokens.append(('punct', source[i]))
            i += 1
    return tokens

def enclosed(tokens, start, opening, closing):
    depth = 1
    end = start + 1
    while end < len(tokens):
        if tokens[end][1] == opening: depth += 1
        elif tokens[end][1] == closing:
            depth -= 1
            if depth == 0: return tokens[start + 1:end], end
        end += 1
    raise ValueError('unclosed Rust source-bearing construct')

def checked_rust_inputs(path: Path, included: set[Path], limit: int):
    tokens = rust_tokens(rust_text(path, limit))
    def bind(literal):
        if '\\' in literal or '\0' in literal:
            raise ValueError(f'unresolved Rust source input: {path}: {literal}')
        target = (path.parent / literal).resolve()
        if target not in included:
            raise ValueError(f'unbound Rust source input: {path}: {literal}')
    i = 0
    while i < len(tokens):
        kind, value = tokens[i]
        if value == 'use':
            end = i + 1
            while end < len(tokens) and tokens[end][1] != ';': end += 1
            if any(token == ('ident', macro) for token in tokens[i + 1:end] for macro in SOURCE_MACROS):
                raise ValueError(f'unresolved Rust source macro alias: {path}')
        bracket = i + 2 if i + 2 < len(tokens) and tokens[i + 1][1] == '!' else i + 1
        if value == '#' and bracket < len(tokens) and tokens[bracket][1] == '[':
            body, end = enclosed(tokens, bracket, '[', ']')
            if ('ident', 'path') in body:
                if len(body) != 3 or body[0] != ('ident', 'path') or body[1][1] != '=' or body[2][0] != 'string':
                    raise ValueError(f'unresolved Rust path attribute: {path}')
                bind(body[2][1])
            # Keep walking the attribute: doc attributes can contain an
            # include_str! invocation even when this is not a path attribute.
            i += 1
            continue
        if kind == 'ident' and value in SOURCE_MACROS and i + 1 < len(tokens) and tokens[i + 1][1] == '!':
            if i + 2 >= len(tokens) or tokens[i + 2][1] != '(':
                raise ValueError(f'unresolved Rust source macro: {path}: {value}')
            body, end = enclosed(tokens, i + 2, '(', ')')
            if len(body) != 1 or body[0][0] != 'string':
                raise ValueError(f'unresolved Rust source macro: {path}: {value}')
            bind(body[0][1])
            i = end + 1
            continue
        i += 1

def paths(root: Path, expected_paths: set[str] | None = None) -> list[Path]:
    root = root.resolve(strict=True)
    found: list[Path] = []
    def visit(directory: Path) -> None:
        for path in sorted(directory.iterdir()):
            relative = path.relative_to(root)
            if path.is_symlink():
                raise ValueError(f'source symlink: {relative.as_posix()}')
            if path.is_dir():
                # Cargo and Ruff write at these configured roots. A nested
                # directory named target can be an authored Rust module.
                if not (relative.as_posix() in ROOT_CACHE_DIRS or path.name == '__pycache__'):
                    visit(path)
            elif path.is_file():
                found.append(path)
            else:
                raise ValueError(f'unsupported source entry: {relative.as_posix()}')
    visit(root)
    # Every explicit Rust source-bearing input must be among the enumerated
    # files. This covers all excluded cache classes and outside-root paths;
    # computed forms fail closed until a real generator/input mapping owns them.
    if expected_paths is not None:
        actual = {path.relative_to(root).as_posix() for path in found}
        if actual != expected_paths:
            changed = sorted(actual ^ expected_paths)
            raise ValueError('stale build source membership: ' + ', '.join(changed[:12]))
    included = set(found)
    rust = [path for path in found if path.suffix == '.rs']
    limit = rust_parse_limit() if rust else 0
    for path in rust:
        checked_rust_inputs(path, included, limit)
    return found

def hashes(root: Path, expected_paths: set[str] | None = None) -> dict[str, str]:
    root = root.resolve(strict=True)
    return {p.relative_to(root).as_posix(): stream_digest(p)
            for p in paths(root, expected_paths)}
