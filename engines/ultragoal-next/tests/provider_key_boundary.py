"""An invalid explicit key must stop before any TypeSafe dispatch or billing attempt."""
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from journey import BIN, row

class ProviderKeyBoundary(unittest.TestCase):
    def test_invalid_present_key_never_dispatches(self):
        with tempfile.TemporaryDirectory() as directory:
            base = pathlib.Path(directory)
            root = base / 'repo'
            root.mkdir()
            (root / 'recovery.log').write_text('recovery aborted; state not restored\n')
            contract = base / 'contract.tsv'
            contract.write_text('UG\t1\n' + row('O', 'recovered', '1',
                'Recovery restored the original state', 'synthetic test',
                'recovery.log', 'content', 'semantic', 'recovery-state', 'semantic', 'mandatory'))
            env = {**os.environ, 'TYPESAFE_API_KEY': 'invalid key with spaces'}
            run = subprocess.run([str(BIN), 'check', '--root', str(root), '--contract',
                                  str(contract), '--no-cache', '--disclosure', 'synthetic'],
                                 capture_output=True, text=True, env=env, timeout=30)
            report = json.loads(run.stdout)
            self.assertEqual(run.returncode, 2, run.stderr)
            self.assertEqual(report['obligations'][0]['state'], 'unknown')
            self.assertEqual(report['coverage']['semantic_requests_prepared'], 1)
            self.assertNotIn('dispatched', run.stdout)
            self.assertNotIn('responded', run.stdout)
            self.assertNotIn('invalid key with spaces', run.stdout + run.stderr)

if __name__ == '__main__':
    unittest.main()
