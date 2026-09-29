"""Actual checker rejects new effects, unprofiled uses and skipped validation."""
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from source_inventory import paths
from check_ownership import canonical_direct, rust_test_symbols

ROOT = pathlib.Path(__file__).resolve().parents[1]

class OwnershipNegative(unittest.TestCase):
    def copy(self):
        (ROOT / 'target').mkdir(exist_ok=True)
        temp = tempfile.TemporaryDirectory(prefix='ownership-', dir=ROOT / 'target')
        repo = pathlib.Path(temp.name) / 'repo'
        (repo / 'engines').mkdir(parents=True)
        (repo / 'scripts').mkdir()
        shutil.copy2(ROOT.parent.parent / 'scripts/check-next', repo / 'scripts/check-next')
        copy = repo / 'engines/ultragoal-next'
        copy.mkdir()
        for source in paths(ROOT):
            target = copy / source.relative_to(ROOT)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
        analyzer_source = ROOT.parent / 'ultragoal-legibility'
        analyzer_copy = repo / 'engines/ultragoal-legibility'
        shutil.copytree(analyzer_source, analyzer_copy,
                        ignore=shutil.ignore_patterns('target', '__pycache__'))
        compiled = analyzer_source / 'target/release/plugin-eval-legibility'
        if compiled.is_file():
            target = analyzer_copy / 'target/release/plugin-eval-legibility'
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(compiled, target)
        return temp, copy

    def check(self, copy):
        run = subprocess.run([sys.executable, str(copy / 'check_ownership.py')],
                             capture_output=True, text=True, timeout=30)
        return run.returncode, json.loads(run.stdout)['failures']

    def test_unmodified_qualified_copy_is_green(self):
        temp, copy = self.copy()
        try:
            code, findings = self.check(copy)
            self.assertEqual((code, findings), (0, []))
        finally:
            temp.cleanup()

    def test_missing_or_tampered_structural_engine_fails_closed(self):
        temp, copy = self.copy()
        try:
            analyzer = copy.parent / 'ultragoal-legibility'
            provenance = analyzer / 'UPSTREAM_PROVENANCE.json'
            provenance.rename(analyzer / 'provenance-missing')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('UPSTREAM_PROVENANCE.json' in f for f in findings), findings)
            (analyzer / 'provenance-missing').rename(provenance)
            source = analyzer / 'src/lib.rs'
            source.write_text(source.read_text() + '\n// unreviewed analyzer edit\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('structural analyzer source or lock differs' in f for f in findings), findings)
            source.write_text((analyzer / 'src/lib.rs').read_text().removesuffix('\n// unreviewed analyzer edit\n'))
            provenance.write_text(provenance.read_text() + ' ')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('structural analyzer provenance differs' in f for f in findings), findings)
            provenance.write_text((ROOT.parent / 'ultragoal-legibility/UPSTREAM_PROVENANCE.json').read_text())
            binary = analyzer / 'target/release/plugin-eval-legibility'
            binary.write_bytes(binary.read_bytes() + b'tampered')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('structural analyzer binary differs' in f for f in findings), findings)
        finally:
            temp.cleanup()

    def test_analyzer_source_symlink_and_nested_authored_source_fail_closed(self):
        temp, copy = self.copy()
        try:
            analyzer = copy.parent / 'ultragoal-legibility'
            source = analyzer / 'src/lib.rs'
            duplicate = analyzer / 'source-copy.rs'
            duplicate.write_bytes(source.read_bytes())
            source.unlink()
            source.symlink_to(duplicate)
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('source symlink' in f for f in findings), findings)
            source.unlink()
            source.write_bytes(duplicate.read_bytes())
            duplicate.unlink()
            nested = analyzer / 'src/target/extra.rs'
            nested.parent.mkdir()
            nested.write_text('pub fn hidden() {}\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('stale build source membership: src/target/extra.rs' in f for f in findings), findings)
        finally:
            temp.cleanup()

    def test_new_direct_process_and_dependency_are_not_grandfathered(self):
        temp, copy = self.copy()
        try:
            source = copy / 'src/resources.rs'
            source.write_text(source.read_text() +
                              '\nfn forbidden() { unsafe { libc::fork(); } }\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any("effect: unowned new site ('src/resources.rs', 'forbidden'" in f for f in findings))
            self.assertTrue(any("dependency: unowned new site ('src/resources.rs', 'libc', 'forbidden', 'libc::fork')" in f for f in findings))
        finally:
            temp.cleanup()

    def test_keychain_validator_call_is_enforced(self):
        temp, copy = self.copy()
        try:
            source = copy / 'src/provider/credential.rs'
            text = source.read_text()
            old = '    accept_lookup(observed)\n}'
            self.assertEqual(text.count(old), 1)
            source.write_text(text.replace(old, '    Ok(ProviderCredential { secret: String::new() })\n}', 1))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertIn('EJ required: boundary_validation_call_missing:src/provider/credential.rs:keychain_key:accept_lookup', findings)
        finally:
            temp.cleanup()

    def test_same_kind_effect_inside_reviewed_owner_invalidates_review(self):
        temp, copy = self.copy()
        try:
            source = copy / 'src/provider/credential.rs'
            text = source.read_text()
            marker = '    let mut command = Command::new("/usr/bin/security");'
            self.assertEqual(text.count(marker), 1)
            source.write_text(text.replace(marker,
                '    let _ = Command::new("/usr/bin/true").status();\n' + marker, 1))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('contract: Rust source review stale src/provider/credential.rs' in f for f in findings))
        finally:
            temp.cleanup()

    def test_fake_evidence_and_nonexistent_caller_are_rejected(self):
        temp, copy = self.copy()
        try:
            path = copy / 'docs/legibility/material-ownership.json'
            contract = json.loads(path.read_text())
            key = next(item for item in contract['effects'] if item['path'] == 'src/provider/credential.rs' and item['symbol'] == 'key')
            key['evidence'] = '../outside::invented_test'
            contract['callers'][0]['call'] = 'does_not_exist'
            contract['callers'][0]['allowed'] = []
            path.write_text(json.dumps(contract))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('invalid source reference' in f for f in findings))
            self.assertTrue(any('caller: empty or duplicate allowed routes' in f for f in findings))
        finally:
            temp.cleanup()

    def test_contained_non_test_and_renamed_tests_are_not_evidence(self):
        temp, copy = self.copy()
        try:
            path = copy / 'docs/legibility/material-ownership.json'
            contract = json.loads(path.read_text())
            dispatch = next(item for item in contract['effects'] if item['path'] == 'src/provider/dispatch.rs' and item['kind'] == 'process')
            dispatch['evidence'] = 'src/provider/dispatch.rs::pub(super)'
            path.write_text(json.dumps(contract))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('evidence Rust test unavailable' in f for f in findings), findings)
            path.write_text((ROOT / 'docs/legibility/material-ownership.json').read_text())
            rust = copy / 'src/provider/dispatch.rs'
            rust.write_text(rust.read_text().replace('fn secret_shaped_body_never_yields_a_dispatch_handle(', 'fn renamed_dispatch_test('))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('evidence Rust test unavailable' in f for f in findings), findings)
            rust.write_text((ROOT / 'src/provider/dispatch.rs').read_text())
            python = copy / 'tests/journey.py'
            python.write_text(python.read_text().replace('def test_pass_fail_and_unknown(', 'def renamed_journey_test('))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('evidence Python test unavailable' in f for f in findings), findings)
        finally:
            temp.cleanup()

    def test_registered_gate_call_is_required_for_python_evidence(self):
        temp, copy = self.copy()
        try:
            gate = copy.parent.parent / 'scripts/check-next'
            gate.write_text(gate.read_text().replace('python3 -m unittest discover -s tests -p journey.py', ': # removed journey invocation'))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('evidence test not registered in check-next' in f for f in findings), findings)
        finally:
            temp.cleanup()

    def test_test_shaped_text_in_rust_comment_or_string_is_not_a_symbol(self):
        temp, copy = self.copy()
        try:
            path = copy / 'src/commented_tests.rs'
            path.write_text('/* #[test]\nfn block_fake() {} */\n// #[test] fn line_fake() {}\nconst TEXT: &str = r#"\n#[test]\nfn string_fake() {}\n"#;\n#[test]\nfn real_test() {}\n')
            self.assertEqual(rust_test_symbols(path), {'real_test'})
        finally:
            temp.cleanup()

    def test_local_qualified_and_imported_caller_identities(self):
        self.assertEqual(canonical_direct('src/process.rs', 'run_bounded', {}), 'process::run_bounded')
        self.assertEqual(canonical_direct('src/process.rs', 'crate::process::run_bounded', {}), 'process::run_bounded')
        self.assertEqual(canonical_direct('src/context.rs', 'run_bounded', {}), 'context::run_bounded')
        temp, copy = self.copy()
        try:
            source = copy / 'src/process.rs'
            source.write_text(source.read_text().replace('    run_bounded(command, input, timeout, MAX)',
                                                        '    crate::process::run_bounded(command, input, timeout, MAX)'))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)  # reviewed source hash changed
            self.assertFalse(any('caller: expected route absent process::run_bounded' in f for f in findings), findings)
            source.write_text((ROOT / 'src/process.rs').read_text())
            context = copy / 'src/context.rs'
            context.write_text(context.read_text() + '\nuse crate::process::run_bounded as illicit;\nfn illicit_call() { let _ = illicit(std::process::Command::new("/usr/bin/true"), vec![], std::time::Duration::from_secs(1), 10); }\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any("caller: unowned process::run_bounded from ('src/context.rs', 'illicit_call')" in f for f in findings), findings)
            context.write_text((ROOT / 'src/context.rs').read_text() + '\nfn run_bounded() {}\nfn harmless_same_name() { run_bounded(); }\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)  # reviewed source hash changed
            self.assertFalse(any('caller: unowned process::run_bounded' in f for f in findings), findings)
        finally:
            temp.cleanup()

    def test_duplicate_contract_key_and_changed_cargo_version_are_rejected(self):
        temp, copy = self.copy()
        try:
            path = copy / 'docs/legibility/material-ownership.json'
            path.write_text(path.read_text().replace('"schema": 1,', '"schema": 1, "schema": 1,', 1))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('duplicate JSON key' in f for f in findings))
            path.write_text((ROOT / 'docs/legibility/material-ownership.json').read_text())
            cargo = copy / 'Cargo.toml'
            cargo.write_text(cargo.read_text().replace('=0.2.186', '=0.2.185'))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('EJ required:' in f or 'EJ execution unavailable' in f for f in findings))
        finally:
            temp.cleanup()

    def test_unparsed_rust_source_cannot_disappear(self):
        temp, copy = self.copy()
        try:
            (copy / 'src/unparsed.rs').write_text('fn broken( {\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('Rust source not parsed' in f or 'EJ required:' in f for f in findings))
        finally:
            temp.cleanup()

    def test_new_network_route_and_changed_writer_sink_are_rejected(self):
        temp, copy = self.copy()
        try:
            provider = copy / 'src/provider.rs'
            provider.write_text(provider.read_text() +
                '\nfn illicit_network() { let _ = std::os::unix::net::UnixStream::connect("missing.sock"); }\n')
            context = copy / 'src/context.rs'
            text = context.read_text()
            old = 'serde_json::to_writer(&mut digest, &item)'
            self.assertEqual(text.count(old), 1)
            context.write_text(text.replace(old,
                'serde_json::to_writer(&mut std::io::stdout(), &item)', 1))
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any("effect: unowned new site ('src/provider.rs', 'illicit_network', 'direct_call', 'network'" in f for f in findings))
            self.assertTrue(any('contract: Rust source review stale src/context.rs' in f for f in findings))
        finally:
            temp.cleanup()

    def test_cfg_ambiguity_and_new_indirect_route_are_rejected(self):
        temp, copy = self.copy()
        try:
            resource = copy / 'src/resources.rs'
            resource.write_text(resource.read_text() +
                '\nfn measure() -> Result<Headroom, HeadroomError> { Err(HeadroomError::Unavailable) }\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('boundary_registry_symbol_ambiguous' in f or 'cfg applicability ambiguous' in f for f in findings))
            resource.write_text((ROOT / 'src/resources.rs').read_text())
            context = copy / 'src/context.rs'
            context.write_text(context.read_text() +
                '\nfn illicit_indirect() { let call = std::process::Command::new; let _ = call("/usr/bin/true"); }\n')
            code, findings = self.check(copy)
            self.assertEqual(code, 1)
            self.assertTrue(any('new unresolved indirect-call site' in f or 'unowned new site' in f for f in findings))
        finally:
            temp.cleanup()

if __name__ == '__main__':
    unittest.main()
