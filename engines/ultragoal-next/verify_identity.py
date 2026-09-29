"""Refuse stale build inputs, release binaries and development proof probes."""
import argparse
import json
from pathlib import Path
from source_inventory import admitted_bytes, hashes, rust_parse_limit, stream_digest

ROOT = Path(__file__).resolve().parent
RELEASE = ROOT / 'target/release'

def digest(path):
    return stream_digest(path)

def verify():
    identity = json.loads(admitted_bytes(RELEASE / 'identity.json', rust_parse_limit()))
    # Refuse a new or missing source by name before opening it for Rust token
    # validation or hashing. A sparse unlisted file must be stale, not work.
    actual = hashes(ROOT, expected_paths=set(identity['source']))
    if actual != identity['source']:
        changed = sorted(set(actual) ^ set(identity['source']) |
                         {path for path in actual.keys() & identity['source'].keys()
                          if actual[path] != identity['source'][path]})
        raise ValueError('stale build source: ' + ', '.join(changed[:12]))
    for section in ('binaries', 'probes'):
        members = identity.get(section)
        if not isinstance(members, dict) or not members:
            raise ValueError(f'identity lacks {section}')
        for name, expected in members.items():
            path = RELEASE / name
            if path.is_symlink() or not path.is_file() or digest(path) != expected:
                raise ValueError(f'stale {section}: {name}')
    if identity.get('version') != 'bend 2.0.27':
        raise ValueError('unqualified Bend build version')
    return {'source_files': len(actual), 'binaries': sorted(identity['binaries']),
            'probes': sorted(identity['probes']), 'identity_sha256': digest(RELEASE / 'identity.json')}

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.parse_args()
    try:
        print(json.dumps({'state': 'current', **verify()}, sort_keys=True))
    except (OSError, KeyError, ValueError, json.JSONDecodeError) as error:
        print(json.dumps({'state': 'stale', 'reason': str(error)}))
        raise SystemExit(1)
