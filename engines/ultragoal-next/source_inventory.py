"""Exact root-relative authored and build-generated inputs for build and package."""
from __future__ import annotations
import hashlib
from pathlib import Path

ROOT_CACHE_DIRS = frozenset({'target', '.ruff_cache'})
GENERATED = {
    'Identity.bend': 'build.py: rule/parser closure identities',
    'src/native_identity.rs': 'build.py: syn wrapper/toolchain identity',
}

SOURCE_MACROS = frozenset({'include', 'include_str', 'include_bytes'})

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

def checked_rust_inputs(path: Path, included: set[Path]):
    tokens = rust_tokens(path.read_text())
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

def paths(root: Path) -> list[Path]:
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
    included = set(found)
    for path in found:
        if path.suffix == '.rs': checked_rust_inputs(path, included)
    return found

def hashes(root: Path) -> dict[str, str]:
    root = root.resolve(strict=True)
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in paths(root)}
