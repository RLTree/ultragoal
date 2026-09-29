"""Check exact production effect and dependency ownership using EJ's AST inventory."""
import hashlib
import ast
import json
import re
import shlex
import platform
from collections import defaultdict
from functools import lru_cache
from pathlib import Path
import subprocess
import sys
from source_inventory import admitted_bytes, hashes, rust_parse_limit, stream_digest

EFFECTS = {'filesystem', 'process', 'environment', 'structured_input'}
STRUCTURAL_SOURCE = Path(__file__).resolve().parent.parent / 'ultragoal-legibility'
STRUCTURAL_PROVENANCE_SHA256 = 'e201f38ecfcfbddf78851eeeebb2ae06b621db1a135a0c18033007c8996b9e06'
LIBC_EFFECTS = {'libc::open', 'libc::openat', 'libc::close', 'libc::closedir', 'libc::fdopendir',
                'libc::fstatat', 'libc::readdir', 'libc::kill', 'libc::signal', 'libc::setsockopt',
                'libc::flock', 'libc::host_statistics64', 'libc::mach_host_self',
                'libc::mach_task_self', 'libc::geteuid', 'libc::getpeereid', 'libc::listen',
                'libc::poll', 'libc::recv', 'libc::__error'}
LIBC_DATA = {'libc::O_NOFOLLOW', 'libc::O_CLOEXEC', 'libc::c_int', 'libc::S_IFMT',
             'libc::S_IFREG', 'libc::ESRCH', 'libc::SIGKILL', 'libc::pid_t', 'libc::EIO',
             'libc::stat', 'libc::O_NONBLOCK', 'libc::O_RDONLY', 'libc::vm_size_t',
             'libc::SOL_SOCKET', 'libc::SO_RCVBUF', 'libc::SO_SNDBUF', 'libc::socklen_t',
             'libc::ENOENT', 'libc::S_IFDIR', 'libc::S_IFLNK', 'libc::SIGINT',
             'libc::SIGTERM', 'libc::sighandler_t', 'libc::AT_SYMLINK_NOFOLLOW',
             'libc::DIR', 'libc::O_DIRECTORY', 'libc::O_NOFOLLOW_ANY', 'libc::LOCK_EX',
             'libc::HOST_VM_INFO64', 'libc::HOST_VM_INFO64_COUNT', 'libc::KERN_SUCCESS',
             'libc::host_t', 'libc::kern_return_t', 'libc::mach_port_t',
             'libc::vm_statistics64', 'libc::MSG_DONTWAIT', 'libc::MSG_PEEK',
             'libc::POLLIN', 'libc::pollfd'}
JSON_PARSERS = {'serde_json::from_str', 'serde_json::from_slice', 'serde_json::from_value',
                'serde_json::Deserializer::from_reader'}
JSON_DATA = {'serde_json::Value', 'serde_json::json', 'serde_json::to_vec',
             'serde_json::Value::Null', 'serde_json::Map::new',
             'serde_json::Value::Object', 'serde_json::Value::Array', 'serde_json::Map',
             'serde_json::Value::as_array',
             'serde_json::to_string_pretty', 'serde_json::to_value',
             'serde_json::Value::String', 'serde_json::Value::as_str',
             'serde_json::to_string'}
SERDE_CONTRACT = {'serde::de::Error', 'serde::Deserializer', 'serde::de::MapAccess',
                  'serde::de::SeqAccess', 'serde::de::Visitor', 'serde::Deserialize',
                  'serde::Deserialize::deserialize', 'serde::de::DeserializeSeed',
                  'serde::de::Deserialize', 'serde::de::Deserializer',
                  'serde::de::Error::custom'}
SHA_DIGEST = {'sha2::Sha256::new', 'sha2::Sha256', 'sha2::Sha256::digest'}
SYN_SYNTAX = {'syn::Error', 'syn::File', 'syn::parse_file'}
ADVISORY_EJ = {'raw_authority_unregistered', 'raw_authority_outside_boundary',
               'dependency_direct_bypass', 'dependency_profiles_missing',
               'semantic_coverage_unsupported', 'authored_line_cap',
               'module_partial_factoring', 'module_residual_prefix'}
EFFECT_FIELDS = {'path', 'symbol', 'site_kind', 'kind', 'callee', 'component', 'outcome',
                 'validation_calls', 'validation_policy', 'require_closed',
                 'evidence', 'review_state', 'source_sha256', 'evidence_sha256', 'cfg_policy'}
DEPENDENCY_FIELDS = {'path', 'crate', 'owner', 'symbol', 'classification',
                     'component', 'evidence', 'review_state', 'source_sha256',
                     'evidence_sha256'}
CALLER_FIELDS = {'target_path', 'target_symbol', 'call', 'allowed', 'coverage', 'evidence'}
PRIVILEGED = {('src/core_session.rs', 'Session::start'),
              ('src/process.rs', 'run_bounded'),
              ('src/provider.rs', 'assess_operation'),
              ('src/provider.rs', 'ledger_file'),
              ('src/provider/credential.rs', 'keychain_key'),
              ('src/session_transport.rs', 'serve_request'),
              ('src/provider/dispatch.rs', "Screened<'_>::send"),
              ('src/native_observation.rs', 'Pending::issue')}

def strict_json(text):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result: raise ValueError(f'duplicate JSON key: {key}')
            result[key] = value
        return result
    return json.loads(text, object_pairs_hook=unique)

def structural_audit(root):
    """Run the pinned project-owned structural engine, never a plugin cache."""
    provenance = admitted_bytes(STRUCTURAL_SOURCE / 'UPSTREAM_PROVENANCE.json', rust_parse_limit())
    if hashlib.sha256(provenance).hexdigest() != STRUCTURAL_PROVENANCE_SHA256:
        raise ValueError('structural analyzer provenance differs from UG-pinned identity')
    manifest = strict_json(provenance.decode())
    expected = manifest['source_files']
    actual = hashes(STRUCTURAL_SOURCE, expected_paths=set(expected) | {'UPSTREAM_PROVENANCE.json'})
    actual.pop('UPSTREAM_PROVENANCE.json')
    if not isinstance(expected, dict) or actual != expected:
        raise ValueError('structural analyzer source or lock differs from pinned provenance')
    qualified = manifest['qualified_binary']
    host = subprocess.check_output(['rustc', '-Vv'], text=True)
    if (qualified['host'] not in host or qualified['rustc'] != host.splitlines()[0] or
            platform.system() != 'Darwin' or platform.machine() != 'arm64'):
        raise ValueError('structural analyzer target/toolchain not qualified')
    target = STRUCTURAL_SOURCE / 'target'
    binary = target / 'release/plugin-eval-legibility'
    if not binary.is_file():
        built = subprocess.run(['cargo', 'build', '--manifest-path', str(STRUCTURAL_SOURCE / 'Cargo.toml'),
                                '--target-dir', str(target), '--locked', '--offline', '--release'],
                               capture_output=True, text=True)
        if built.returncode:
            raise ValueError(f'structural analyzer build unavailable ({built.returncode}): {built.stderr[-300:]}')
    if binary.is_symlink() or stream_digest(binary) != qualified['sha256']:
        raise ValueError('structural analyzer binary differs from qualified source/toolchain build')
    observed = subprocess.run([str(binary), '--root', str(root), '--inventory'], capture_output=True, text=True)
    if observed.returncode not in (0, 1):
        raise ValueError(f'structural analyzer execution unavailable ({observed.returncode}): {observed.stderr[:200]}')
    audit = strict_json(observed.stdout)
    if audit.get('schema') != 'engineering-judgment.legibility.v1':
        raise ValueError('structural analyzer schema mismatch')
    return audit

def source_ref(root, value):
    if not isinstance(value, str) or not value or value.startswith('/') or '*' in value:
        raise ValueError(f'invalid source reference: {value!r}')
    path = Path(value)
    if any(part in ('', '.', '..') for part in path.parts):
        raise ValueError(f'invalid source reference: {value!r}')
    resolved = (root / path).resolve(strict=True)
    if not resolved.is_relative_to(root) or not resolved.is_file() or (root / path).is_symlink():
        raise ValueError(f'outside source reference: {value!r}')
    return resolved

@lru_cache(maxsize=None)
def gate_invocations(gate):
    python_tests, scripts = set(), set()
    rust = False
    for line in gate.splitlines():
        try: words = shlex.split(line, comments=True)
        except ValueError: continue
        if words[:4] == ['cargo', 'test', '--locked', '--offline']:
            rust = True
        if words[:6] == ['python3', '-m', 'unittest', 'discover', '-s', 'tests'] and len(words) >= 8 and words[6] == '-p':
            python_tests.add(words[7])
        if len(words) == 2 and words[0] == 'python3' and words[1].startswith('tests/'):
            scripts.add(words[1])
    return python_tests, scripts, rust

@lru_cache(maxsize=None)
def python_symbols(path):
    tree = ast.parse(path.read_text(), filename=str(path))
    tests = set()
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name.startswith('test_'):
            tests.add(node.name)
        if isinstance(node, ast.ClassDef) and any(isinstance(base, (ast.Name, ast.Attribute)) and
               (base.id if isinstance(base, ast.Name) else base.attr) == 'TestCase' for base in node.bases):
            tests.update(child.name for child in node.body if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)) and child.name.startswith('test_'))
    entry = {node.name for node in tree.body if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))}
    called = set()
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            continue
        called.update(call.func.id for call in ast.walk(node) if isinstance(call, ast.Call) and isinstance(call.func, ast.Name))
    return tests, entry & called

@lru_cache(maxsize=None)
def rust_test_symbols(path):
    source = path.read_text()
    # Recognize source tokens, not test-shaped text inside comments, regular
    # strings or raw strings. Keep line breaks so attribute/function pairs
    # retain their source layout.
    output = list(source)
    i = 0
    while i < len(source):
        start = i
        if source.startswith('//', i):
            i = source.find('\n', i)
            if i < 0: i = len(source)
        elif source.startswith('/*', i):
            depth = 1; i += 2
            while i < len(source) and depth:
                if source.startswith('/*', i): depth += 1; i += 2
                elif source.startswith('*/', i): depth -= 1; i += 2
                else: i += 1
        else:
            raw = re.match(r'(?:b)?r(#+)?"', source[i:])
            if raw:
                marker = '"' + (raw.group(1) or '')
                i = source.find(marker, i + raw.end())
                i = len(source) if i < 0 else i + len(marker)
            elif source[i] == '"':
                i += 1
                while i < len(source):
                    if source[i] == '\\': i += 2
                    elif source[i] == '"': i += 1; break
                    else: i += 1
            else:
                i += 1
                continue
        for j in range(start, min(i, len(source))):
            if source[j] != '\n': output[j] = ' '
    source = ''.join(output)
    pattern = r'(?m)^\s*#\[\s*test\s*\](?:\s*#\[[^\]]+\])*\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)\s*\('
    return set(re.findall(pattern, source))

def evidence_ref(root, raw, source_hashes, gate):
    if not isinstance(raw, str) or raw.count('::') != 1:
        raise ValueError(f'evidence must name file::test: {raw!r}')
    path, test = raw.split('::')
    file = source_ref(root, path)
    python_tests, scripts, rust = gate_invocations(gate)
    if path.endswith('.rs'):
        if test not in rust_test_symbols(file):
            raise ValueError(f'evidence Rust test unavailable: {raw}')
        if not rust:
            raise ValueError(f'Rust evidence not registered in check-next: {raw}')
    elif path.startswith('tests/') and path.endswith('.py'):
        tests, entries = python_symbols(file)
        if file.name in python_tests:
            if test not in tests:
                raise ValueError(f'evidence Python test unavailable: {raw}')
        elif path in scripts:
            if test not in entries:
                raise ValueError(f'evidence script entry unavailable: {raw}')
        else:
            raise ValueError(f'evidence test not registered in check-next: {raw}')
    else:
        raise ValueError(f'evidence must name registered test source: {raw}')
    return path, source_hashes[path]

def dependency_class(crate, symbol):
    if crate == 'libc':
        if symbol in LIBC_EFFECTS: return 'native_effect'
        if symbol in LIBC_DATA: return 'native_type_or_constant'
    if crate == 'serde' and symbol in SERDE_CONTRACT: return 'typed_parse_contract'
    if crate == 'serde_json':
        if symbol in JSON_PARSERS: return 'structured_input_parser'
        if symbol == 'serde_json::to_writer': return 'effectful_emitter'
        if symbol in JSON_DATA: return 'data_or_emitter'
    if crate == 'sha2' and symbol in SHA_DIGEST: return 'source_digest'
    if crate == 'syn' and symbol in SYN_SYNTAX:
        return 'syntax_parser' if symbol == 'syn::parse_file' else 'syntax_type'
    return None

def module_for(path):
    parts = Path(path).with_suffix('').parts
    if not parts or parts[0] != 'src':
        raise ValueError(f'caller source outside Rust crate: {path}')
    parts = parts[1:]
    if parts and parts[-1] in {'main', 'lib', 'mod'}:
        parts = parts[:-1]
    return tuple(parts)

def direct_imports(root, path):
    aliases = {}
    source = (root / path).read_text()
    for expression in re.findall(r'(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);', source):
        if '{' in expression:
            prefix, rest = expression.split('{', 1)
            if '}' not in rest: raise ValueError(f'caller import unresolved: {path}')
            expanded = [prefix.rstrip(':') + '::' + item.strip() for item in rest.split('}', 1)[0].split(',')]
        else:
            expanded = [expression.strip()]
        for item in expanded:
            if not item or '*' in item or '{' in item: continue
            parts = re.split(r'\s+as\s+', item, maxsplit=1)
            qualified = parts[0].strip()
            alias = parts[1].strip() if len(parts) == 2 else qualified.split('::')[-1]
            if not re.fullmatch(r'[A-Za-z_][A-Za-z_0-9]*', alias): continue
            canonical = canonical_direct(path, qualified, {})
            if alias in aliases and aliases[alias] != canonical:
                raise ValueError(f'caller import alias ambiguous: {path}:{alias}')
            aliases[alias] = canonical
    return aliases

def canonical_direct(path, call, aliases):
    parts = call.split('::')
    if not parts or any(not part for part in parts): return None
    module = module_for(path)
    if parts[0] == 'crate': return '::'.join(parts[1:])
    if parts[0] == 'self': return '::'.join((*module, *parts[1:]))
    if parts[0] == 'super': return '::'.join((*module[:-1], *parts[1:]))
    if parts[0] in aliases: return aliases[parts[0]] + ('::' + '::'.join(parts[1:]) if len(parts) > 1 else '')
    return '::'.join((*module, *parts))

def check(root: Path):
    audit = structural_audit(root)
    required = [finding for finding in audit['failures']
                if finding.split(':', 1)[0] not in ADVISORY_EJ]
    if required:
        # In particular, resource-pressure and unavailable-source findings
        # precede every Python source hash or token read.
        return {'schema': 'ultragoal-material-ownership/1', 'passed': False,
                'effects': 0, 'dependencies': 0, 'strict_ej_passed': audit['passed'],
                'strict_ej_findings': len(audit['failures']),
                'failures': [f'EJ required: {finding}' for finding in required]}
    inventory = audit['inventory']
    if not isinstance(inventory.get('rust'), list) or not isinstance(inventory.get('files'), list):
        raise ValueError('EJ Rust/source inventory incomplete')
    tests = set(inventory['test_only_files'])
    reports = [report for report in inventory['rust'] if report['path'] not in tests]
    functions = defaultdict(list)
    for report in reports:
        for function in report['functions']:
            functions[(report['path'], function['symbol'])].append(function)
    effects = [{'path': report['path'], 'symbol': a.get('symbol'), 'site_kind': a['site_kind'],
                'kind': a['kind'], 'callee': None}
               for report in reports for a in report['authorities'] if a['kind'] in EFFECTS]
    direct = {d['crate'] for d in inventory['direct_dependencies']}
    uses = [{'path': report['path'], 'crate': u['crate'], 'owner': u.get('owner'), 'symbol': u['symbol']}
            for report in reports for u in report['dependency_uses'] if u['crate'] in direct]
    effects.extend({'path': use['path'], 'symbol': use['owner'], 'site_kind': 'dependency_call',
                    'kind': 'native_ffi', 'callee': use['symbol']}
                   for use in uses if use['crate'] == 'libc' and use['symbol'] in LIBC_EFFECTS)
    effects.extend({'path': use['path'], 'symbol': use['owner'], 'site_kind': 'dependency_call',
                    'kind': 'writer_sink', 'callee': use['symbol']}
                   for use in uses if use['crate'] == 'serde_json' and use['symbol'] == 'serde_json::to_writer')
    effects.extend({'path': path, 'symbol': symbol, 'site_kind': 'direct_call',
                    'kind': 'network', 'callee': call}
                   for (path, symbol), variants in functions.items() for variant in variants
                   for call in variant['direct_calls']
                   if call.startswith(('std::net::', 'std::os::unix::net::')))
    contract = strict_json((root / 'docs/legibility/material-ownership.json').read_text())
    failures = []
    if set(contract) != {'schema', 'effects', 'dependencies', 'callers', 'rust_sources', 'indirect_calls'} or contract.get('schema') != 1:
        failures.append('contract: schema or fields invalid')
    source_hashes = hashes(root)
    rust_files = {path for path in source_hashes if path.endswith('.rs')}
    parsed = {report['path'] for report in inventory['rust']}
    for path in sorted(rust_files - parsed): failures.append(f'inventory: Rust source not parsed {path}')
    for finding in audit['failures']:
        family = finding.split(':', 1)[0]
        if family not in ADVISORY_EJ:
            failures.append(f'EJ required: {finding}')
    gate = source_ref(root.parent.parent, 'scripts/check-next').read_text()
    if not isinstance(contract.get('rust_sources'), dict) or set(contract['rust_sources']) != rust_files:
        failures.append('contract: incomplete Rust source review set')
    else:
        for path, reviewed in contract['rust_sources'].items():
            if reviewed != source_hashes.get(path):
                failures.append(f'contract: Rust source review stale {path}')
    indirect = {(path, symbol) for (path, symbol), variants in functions.items()
                if any('<indirect-call>' in f['direct_calls'] for f in variants)}
    declared_indirect = {(item.get('path'), item.get('symbol')) for item in contract.get('indirect_calls', [])}
    if len(declared_indirect) != len(contract.get('indirect_calls', [])):
        failures.append('contract: duplicate indirect-call record')
    for path in sorted(indirect - declared_indirect):
        failures.append(f'contract: new unresolved indirect-call site {path}')
    for path in sorted(declared_indirect - indirect):
        failures.append(f'contract: stale indirect-call record {path}')
    for item in contract.get('indirect_calls', []):
        name = (item.get('path'), item.get('symbol'))
        if set(item) != {'path', 'symbol', 'owner', 'evidence', 'review_state', 'source_sha256'} or item.get('review_state') != 'reviewed':
            failures.append(f'contract: indirect call unreviewed {name}')
            continue
        source_ref(root, item['owner'])
        if item['source_sha256'] != source_hashes.get(item['path']):
            failures.append(f'contract: indirect source stale {name}')
        try: evidence_ref(root, item['evidence'], source_hashes, gate)
        except (OSError, ValueError) as error: failures.append(f'contract: {error}')
    for label, actual, declared, fields in (
        ('effect', effects, contract['effects'], ('path', 'symbol', 'site_kind', 'kind', 'callee')),
        ('dependency', uses, contract['dependencies'], ('path', 'crate', 'owner', 'symbol')),
    ):
        key = lambda item: tuple(item.get(field) for field in fields)
        a, d = {key(item) for item in actual}, {key(item) for item in declared}
        if len(d) != len(declared): failures.append(f'{label}: duplicate declaration')
        for new in sorted(a - d, key=str): failures.append(f'{label}: unowned new site {new}')
        for stale in sorted(d - a, key=str): failures.append(f'{label}: stale declaration {stale}')
    for item in contract['effects']:
        name = (item['path'], item['symbol'])
        if set(item) != EFFECT_FIELDS or not isinstance(item.get('require_closed'), bool) or not isinstance(item.get('validation_calls'), list):
            failures.append(f'effect: schema invalid {name}')
            continue
        source_ref(root, item['path'])
        source_ref(root, item['component'])
        if item.get('review_state') != 'reviewed' or not item.get('component') or not item.get('outcome') or not item.get('evidence'):
            failures.append(f'effect: incomplete owner/outcome/evidence {name}')
        policy = item.get('validation_policy')
        if policy not in {'direct', 'typed_result', 'receiving_test', 'declaration'}:
            failures.append(f'effect: validation policy absent {name}')
        if policy == 'direct' and not item.get('validation_calls'):
            failures.append(f'effect: direct validation absent {name}')
        if policy == 'declaration' and item['site_kind'] != 'declaration_or_module':
            failures.append(f'effect: false declaration {name}')
        if item['source_sha256'] != source_hashes[item['path']]:
            failures.append(f'effect: source review stale {name}')
        if item.get('evidence'):
            try:
                test_path, digest = evidence_ref(root, item['evidence'], source_hashes, gate)
                if digest != item['evidence_sha256']:
                    failures.append(f'effect: evidence review stale {name}:{test_path}')
            except (OSError, ValueError) as error:
                failures.append(f'effect: {error}')
        variants = functions.get(name, [])
        if variants:
            if item['cfg_policy'] not in {'single', 'exclusive'} or (len(variants) > 1) != (item['cfg_policy'] == 'exclusive'):
                failures.append(f'effect: cfg applicability ambiguous {name}')
            for function in variants:
                if function['return_type'] != item['outcome']: failures.append(f'effect: outcome drift {name}')
                for validation in item['validation_calls']:
                    if validation not in function['direct_calls']:
                        failures.append(f'effect: validation bypass {name}:{validation}')
                if (item['require_closed'] or policy == 'typed_result') and not function['closed_result']:
                    failures.append(f'effect: open consequential outcome {name}')
        elif item['site_kind'] == 'function': failures.append(f'effect: function unavailable {name}')
    for item in contract['dependencies']:
        name = (item.get('path'), item.get('owner'), item.get('symbol'))
        if set(item) != DEPENDENCY_FIELDS:
            failures.append(f'dependency: schema invalid {name}')
            continue
        source_ref(root, item['path'])
        source_ref(root, item['component'])
        if item.get('review_state') != 'reviewed' or not item.get('classification') or not item.get('component') or not item.get('evidence'):
            failures.append(f'dependency: incomplete use {item["path"]}:{item["symbol"]}')
        if item.get('classification') != dependency_class(item['crate'], item['symbol']):
            failures.append(f'dependency: wrong class {item["path"]}:{item["symbol"]}')
        if item['source_sha256'] != source_hashes[item['path']]:
            failures.append(f'dependency: source review stale {name}')
        try:
            test_path, digest = evidence_ref(root, item['evidence'], source_hashes, gate)
            if digest != item['evidence_sha256']:
                failures.append(f'dependency: evidence review stale {name}:{test_path}')
        except (OSError, ValueError) as error:
            failures.append(f'dependency: {error}')
        if item['classification'] in {'native_effect', 'effectful_emitter'}:
            if not any(e['path'] == item['path'] and e['symbol'] == item['owner'] and
                       e['callee'] == item['symbol'] for e in effects):
                failures.append(f'dependency: effect owner absent {name}')
    targets = {(rule.get('target_path'), rule.get('target_symbol')) for rule in contract['callers']}
    if len(targets) != len(contract['callers']):
        failures.append('caller: duplicate target rule')
    for missing in sorted(PRIVILEGED - targets):
        failures.append(f'caller: required privileged target absent {missing}')
    for rule in contract['callers']:
        target = (rule.get('target_path'), rule.get('target_symbol'))
        if set(rule) != CALLER_FIELDS or target not in functions or rule.get('coverage') not in {'direct_and_private', 'direct_and_tested', 'private_handle_and_tested'}:
            failures.append(f'caller: invalid target or schema {target}')
            continue
        if not isinstance(rule.get('allowed'), list) or not rule['allowed'] or len({tuple(pair) for pair in rule['allowed']}) != len(rule['allowed']):
            failures.append(f'caller: empty or duplicate allowed routes {target}')
            continue
        if not isinstance(rule.get('call'), str) or not rule['call'].endswith(rule['target_symbol']):
            failures.append(f'caller: unresolved call spelling {target}')
            continue
        try: evidence_ref(root, rule['evidence'], source_hashes, gate)
        except (OSError, ValueError) as error: failures.append(f'caller: {error}')
        allowed = {tuple(pair) for pair in rule['allowed']}
        for caller in allowed:
            if caller not in functions: failures.append(f'caller: unknown allowed symbol {caller}')
        if rule['coverage'] == 'private_handle_and_tested':
            # EJ does not resolve methods. This one exact private handle is
            # source-bound and has an actual invalid-before-send adapter test.
            if target != ('src/provider/dispatch.rs', "Screened<'_>::send") or allowed != {('src/provider.rs', 'assess_operation')}:
                failures.append(f'caller: unqualified private-handle route {target}')
            if 'struct Screened<' not in (root / target[0]).read_text() or 'screened.send(remaining)' not in (root / 'src/provider.rs').read_text():
                failures.append(f'caller: private handle or consuming call absent {target}')
            continue
        canonical_target = '::'.join((*module_for(target[0]), target[1]))
        found = set()
        for (path, symbol), variants in functions.items():
            aliases = direct_imports(root, path)
            if any(canonical_direct(path, call, aliases) == canonical_target
                   for function in variants for call in function['direct_calls']):
                found.add((path, symbol))
        if not found: failures.append(f'caller: target has no observed direct route {target}')
        for new in sorted(found - allowed): failures.append(f'caller: unowned {canonical_target} from {new}')
        for stale in sorted(allowed - found): failures.append(f'caller: expected route absent {canonical_target} from {stale}')
    return {'schema': 'ultragoal-material-ownership/1', 'passed': not failures,
            'effects': len(effects), 'dependencies': len(uses),
            'strict_ej_passed': audit['passed'], 'strict_ej_findings': len(audit['failures']), 'failures': failures}

if __name__ == '__main__':
    try: result = check(Path(__file__).resolve().parent)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        result = {'schema': 'ultragoal-material-ownership/1', 'passed': False, 'failures': [str(error)]}
    print(json.dumps(result, sort_keys=True))
    sys.exit(0 if result['passed'] else 1)
