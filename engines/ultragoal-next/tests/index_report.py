"""The streamed index report: its fact sets carry exactly Bend's facts (line facts as runs),
`select` answers alike from the old and new forms, and a cancelled index still ends as one
JSON object that `select` refuses. No provider access."""
import collections, hashlib, json, pathlib, signal, subprocess, tempfile, time, unittest, urllib.parse
from journey import BIN, core, row

FILES = {
    'src/app.py': 'import os\n\ndef run(cache):\n    return os.getcwd()\n\n\n# π trailing\n',
    'src/lib.rs': 'use std::fs;\nfn main() {\n    let x = 1;\n}\n',
    'docs/guide.md': '# Guide\n\nSee [cache](cache.md).\n\n## Reload\ntext\n',
    'notes/crlf.txt': 'first\r\nsecond\r\n\r\nlast',
    'config.json': '{"request_timeout_seconds": 3}\n',
}


def expand(s):
    """A set's facts in the earlier form: every `line_runs` line back as a `line` fact."""
    facts = [tuple(f) for f in s['facts']]
    for run in s['line_runs']:
        n, start = run[0], run[1]
        for length in run[2:]:
            facts.append(('line', None, str(n), str(start), str(start + length)))
            n, start = n + 1, start + length + 1
    return facts


class IndexReport(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.p = pathlib.Path(self.tmp.name)
        self.root = self.p / 'repo'
        for name, text in FILES.items():
            (self.root / name).parent.mkdir(parents=True, exist_ok=True)
            (self.root / name).write_bytes(text.encode())

    def tearDown(self):
        self.tmp.cleanup()

    def index(self):
        p = subprocess.run([str(BIN), 'index', '--root', str(self.root), '--scope', '*', '--no-cache'], capture_output=True, text=True, timeout=60)
        self.assertEqual(p.returncode, 0, p.stdout[-2000:] + p.stderr)
        return p.stdout

    def select(self, report, query):
        p = subprocess.run([str(BIN), 'select', '--root', str(self.root), '--report', str(report), '--query', query, '--local', '--details'], capture_output=True, text=True, timeout=60)
        self.assertEqual(p.returncode, 0, p.stdout[-2000:] + p.stderr)
        v = json.loads(p.stdout)
        return {k: v[k] for k in ('shortlist', 'candidate_provenance', 'provenance_sha256', 'prepared_evidence_sha256', 'coverage', 'anchors')}

    def test_fact_sets_come_first_one_per_line_and_hold_bends_facts(self):
        text = self.index()
        self.assertTrue(text.startswith('{\n  "fact_sets": [\n    {"'), text[:80])
        head, _, rest = text.partition('\n  ],\n')
        set_lines = head.split('\n')[2:]
        self.assertEqual(len(set_lines), len(FILES))
        self.assertTrue(all(line.startswith('    {') for line in set_lines))
        report = json.loads(text)
        self.assertEqual(list(report)[0], 'fact_sets')
        self.assertEqual(report['state'], 'indexed')
        # Bend's own rows for the same files are the reference.
        frame = 'INDEX\nUG\t1\n' + row('Q', '*') + row('OP', 'now', 'running', 'revision', 'timely')
        for name in sorted(FILES):
            data = FILES[name].encode()
            frame += row('F', name, hashlib.sha256(data).hexdigest(), FILES[name])
        rows = {r[1]: r for r in core(frame) if r[0] in ('P', 'P_SPANS', 'P_MIXED')}
        unescape = urllib.parse.unquote
        runs = 0
        for s in report['fact_sets']:
            r = rows[s['path']]
            compact, mixed = r[0] == 'P_SPANS', r[0] == 'P_MIXED'
            expected = []
            for line in unescape(r[4]).splitlines():
                f = [unescape(x) for x in line.split('\t')]
                null = (compact or (mixed and f[1] == '' and f[3] != f[4])) and f[2] != '0'
                expected.append((f[0], None if null else f[1], f[2], f[3], f[4]))
            self.assertEqual(collections.Counter(expand(s)), collections.Counter(expected), s['path'])
            self.assertFalse(any(f[0] == 'line' and f[1] is None for f in s['facts']), s['path'])
            runs += len(s['line_runs'])
        self.assertGreater(runs, 0)

    def test_select_answers_alike_from_old_and_new_forms(self):
        new = self.p / 'new.json'
        new.write_text(self.index())
        report = json.loads(new.read_text())
        for s in report['fact_sets']:
            s['facts'] = [list(f) for f in expand(s)]
            del s['line_runs']
        old = self.p / 'old.json'
        old.write_text(json.dumps(report, indent=2))
        for query in ['cache reload guide', 'run getcwd', 'second last', 'request timeout']:
            self.assertEqual(self.select(new, query), self.select(old, query), query)

    def test_duplicate_keys_in_a_fact_set_are_refused(self):
        report = self.p / 'index.json'
        text = self.index()
        report.write_text(text.replace('"grammar":', '"path":"x","grammar":', 1))
        p = subprocess.run([str(BIN), 'select', '--root', str(self.root), '--report', str(report), '--query', 'cache', '--local'], capture_output=True, text=True, timeout=60)
        self.assertEqual(p.returncode, 3, p.stdout)
        self.assertIn('duplicate key', json.loads(p.stdout)['error'])

    def test_cancelled_index_still_ends_as_one_json_object(self):
        # Two lanes of several requests: 48 files of 1 MiB of short lines.
        line = 'value = compute(cache, reload) + offset  # note\n'
        body = line * (1024 * 1024 // len(line))
        for i in range(48):
            (self.root / f'bulk/f{i:02}.py').parent.mkdir(exist_ok=True)
            (self.root / f'bulk/f{i:02}.py').write_text(body)
        out = self.p / 'cancelled.json'
        with open(out, 'wb') as sink:
            p = subprocess.Popen([str(BIN), 'index', '--root', str(self.root), '--scope', '*', '--no-cache'], stdout=sink, stderr=subprocess.PIPE)
            deadline = time.monotonic() + 60
            while out.stat().st_size < 2 and p.poll() is None and time.monotonic() < deadline:
                time.sleep(0.01)
            p.send_signal(signal.SIGINT)
            _, err = p.communicate(timeout=60)
        text = out.read_text()
        report = json.loads(text)
        if p.returncode == 0:
            self.skipTest('index finished before the signal')
        self.assertEqual((p.returncode, report['error_code'], report['state']), (130, 'cancelled', 'cancelled'), err)
        self.assertTrue(text.startswith('{\n  "fact_sets": ['))
        self.assertNotIn('manifest', report)
        s = subprocess.run([str(BIN), 'select', '--root', str(self.root), '--report', str(out), '--query', 'cache', '--local'], capture_output=True, text=True, timeout=60)
        self.assertEqual(s.returncode, 3, s.stdout)


if __name__ == '__main__':
    unittest.main()
