"""Former 16 MiB standalone source boundary, including a forbidden C tail."""
import hashlib
import json
import os
import pathlib
import subprocess
import tempfile
import unittest

from journey import BIN

SIZE = 17 * 1024 * 1024


@unittest.skipUnless(os.environ.get('UG_VERIFY_LARGE') == '1', 'focused former source bound')
class LargeStandaloneVerifier(unittest.TestCase):
    def test_every_native_source_surface_has_no_fixed_file_ceiling(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            examples = [
                ('rust', 'fn first() {}\n' + ' ' * SIZE + '\nfn tail() {}\n'),
                ('python', 'first = 1\n' + ' ' * SIZE + '\ntail = 2\n'),
                ('typescript', 'let first = 1;\n' + ' ' * SIZE + '\nlet tail = 2;\n'),
                ('stdin-c', 'int main(void) { return 0; }\n' + ' ' * SIZE + '\n'),
            ]
            for kind, source in examples:
                with self.subTest(kind=kind):
                    path = root / f'source-{kind}.txt'
                    path.write_text(source)
                    before = hashlib.sha256(path.read_bytes()).hexdigest()
                    process = subprocess.run([str(BIN), 'verify', f'--{kind}', str(path)],
                                             capture_output=True, text=True, timeout=120)
                    report = json.loads(process.stdout)
                    self.assertEqual((process.returncode, report.get('state')), (0, 'verified'),
                                     process.stdout[-900:] + process.stderr[-400:])
                    self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), before)
                    path.unlink()

    def test_late_c_preprocessor_directive_cannot_hide_across_pages(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / 'late.c'
            path.write_text('int main(void) { return 0; }\n' + ' ' * SIZE + '\n#include <stdio.h>\n')
            process = subprocess.run([str(BIN), 'verify', '--stdin-c', str(path)],
                                     capture_output=True, text=True, timeout=120)
            report = json.loads(process.stdout)
            self.assertEqual((process.returncode, report.get('state')), (2, 'unknown'),
                             process.stdout[-900:] + process.stderr[-400:])
            self.assertEqual(report.get('admission', [[]])[0][0], 'REFUSED')


if __name__ == '__main__':
    unittest.main()
