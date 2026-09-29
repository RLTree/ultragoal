"""The rollback rehearsal must still check extracted bytes under python -O."""
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

SOURCE = pathlib.Path(__file__).resolve().parents[1] / 'migration.py'


class MigrationGuard(unittest.TestCase):
    def test_rehearsal_rejects_tampered_extraction_even_with_optimized_python(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            legacy = root / 'legacy'
            legacy.mkdir()
            payload = legacy / 'payload.txt'
            payload.write_bytes(b'original release bytes')
            digest = hashlib.sha256(payload.read_bytes()).hexdigest()
            (legacy / 'RELEASE_FILES.json').write_text(json.dumps({'files': {'payload.txt': {'sha256': digest}}}))
            good = subprocess.run([sys.executable, '-O', str(SOURCE), '--legacy-root', str(legacy), '--out', str(root / 'good')], capture_output=True, text=True)
            self.assertEqual(good.returncode, 0, good.stdout + good.stderr)
            self.assertTrue((root / 'good/plan.json').is_file())
            injected = '''
import importlib.util, pathlib, sys, tarfile
source, legacy, output = sys.argv[1:]
original = tarfile.TarFile.extractall
def corrupt(self, destination, *args, **kwargs):
    original(self, destination, *args, **kwargs)
    pathlib.Path(destination, 'payload.txt').write_bytes(b'corrupt extracted bytes')
tarfile.TarFile.extractall = corrupt
spec = importlib.util.spec_from_file_location('migration_under_test', source)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
sys.argv = [source, '--legacy-root', legacy, '--out', output]
module.main()
'''
            bad = subprocess.run([sys.executable, '-O', '-c', injected, str(SOURCE), str(legacy), str(root / 'bad')], capture_output=True, text=True)
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn('restoration mismatch', bad.stdout + bad.stderr)
            self.assertFalse((root / 'bad/plan.json').exists())
            self.assertEqual(payload.read_bytes(), b'original release bytes')


if __name__ == '__main__':
    unittest.main()
