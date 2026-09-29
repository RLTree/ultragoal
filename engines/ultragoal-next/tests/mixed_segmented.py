"""Mixed large and small sources must preserve the ordinary Bend result."""
import hashlib, json, os, pathlib, subprocess, tempfile, unittest
from journey import BIN, CORE, obligation, row

def direct(contract, files):
    frame = contract + row('OP', 'reference', 'running', 'contract', 'timely')
    for path, body in sorted(files.items()):
        frame += row('F', path, hashlib.sha256(body.encode()).hexdigest(), body)
    p = subprocess.run([str(CORE)], input=frame, capture_output=True, text=True, timeout=60)
    assert p.returncode == 0, p.stderr
    return {r[1]: r[2] for r in (line.split('\t') for line in p.stdout.splitlines()) if r[0] == 'RESULT'}

class MixedSegmented(unittest.TestCase):
    def check(self, size):
        with tempfile.TemporaryDirectory() as d:
            root = pathlib.Path(d); repo = root / 'repo'; repo.mkdir()
            files = {'large.txt': 'x' * size + '\ntail-needle\n', 'small.txt': 'forbidden\n'}
            for path, body in files.items(): (repo / path).write_text(body)
            contract = 'UG\t1\n' + obligation(id='present', scope='*', argument='tail-needle') + obligation(id='forbid', scope='*', rule='absent-text', argument='forbidden') + obligation(id='member', scope='large.txt', rule='member', projection='membership', argument='')
            path = root / 'contract.tsv'; path.write_text(contract)
            before = {p: hashlib.sha256((repo / p).read_bytes()).hexdigest() for p in files}
            p = subprocess.run([str(BIN), 'check', '--root', str(repo), '--contract', str(path), '--no-cache', '--local', '--details'], capture_output=True, text=True, timeout=120)
            report = json.loads(p.stdout)
            after = {name: hashlib.sha256((repo / name).read_bytes()).hexdigest() for name in files}
            self.assertEqual(before, after)
            self.assertEqual((p.returncode, report['state']), (1, 'failed'), p.stdout[-1000:] + p.stderr)
            self.assertEqual({o['id']: o['state'] for o in report['obligations']}, {'present': 'verified', 'forbid': 'failed', 'member': 'verified'})
            self.assertEqual(report['inputs']['problems'], {})
            self.assertEqual(next(o['counterexample']['path'] for o in report['obligations'] if o['id'] == 'forbid'), 'small.txt')
            self.assertEqual(report['obligations'][0]['chunk_coverage']['partials'], 'aligned')
            return contract, files, report

    def test_six_megabyte_source_matches_one_shot(self):
        contract, files, report = self.check(6 * 1024 * 1024)
        self.assertEqual({o['id']: o['computed_state'] for o in report['obligations']}, direct(contract, files))

    @unittest.skipUnless(os.environ.get('UG_SEGMENT_LARGE') == '1', 'focused former-boundary fixture')
    def test_seventeen_megabyte_source_and_small_counterexample(self):
        _, _, report = self.check(17 * 1024 * 1024)
        self.assertTrue(report['content_chunks']['evaluated_bytes'] > 17 * 1024 * 1024)

    @unittest.skipUnless(os.environ.get('UG_SEGMENT_LARGE') == '1', 'focused former-boundary fixture')
    def test_index_and_select_keep_large_and_small_fact_sets(self):
        with tempfile.TemporaryDirectory() as d:
            root = pathlib.Path(d); repo = root / 'repo'; repo.mkdir()
            (repo / 'large.py').write_text('x' * (17 * 1024 * 1024) + '\ntail-needle\n')
            (repo / 'small.py').write_text('def small(): return 1\n')
            before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in repo.iterdir()}
            p = subprocess.run([str(BIN), 'index', '--root', str(repo), '--scope', '*', '--no-cache'], capture_output=True, text=True, timeout=120)
            self.assertEqual(p.returncode, 0, p.stdout[-1000:] + p.stderr)
            report = json.loads(p.stdout)
            self.assertEqual(report['state'], 'indexed')
            self.assertEqual({set['path'] for set in report['fact_sets']}, {'large.py', 'small.py'})
            self.assertEqual(report['inputs']['problems'], {})
            path = root / 'index.json'; path.write_text(p.stdout)
            q = subprocess.run([str(BIN), 'select', '--root', str(repo), '--report', str(path), '--query', 'tail-needle'], capture_output=True, text=True, timeout=120)
            self.assertEqual(q.returncode, 0, q.stdout[-1000:] + q.stderr)
            selection = json.loads(q.stdout)
            self.assertTrue(any(c['path'] == 'large.py' and 'tail-needle' in c['excerpt'] for c in selection['shortlist']))
            self.assertEqual({p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in repo.iterdir()}, before)

if __name__ == '__main__': unittest.main()
