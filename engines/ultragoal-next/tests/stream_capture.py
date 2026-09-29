"""Streamed capture and chunk dispatch, streamed index and named bounds; no provider access."""
import ctypes, hashlib, json, pathlib, selectors, subprocess, tempfile, time, unittest
from journey import BIN, obligation

LIBPROC = ctypes.CDLL('/usr/lib/libproc.dylib')
NEEDLES = {0: 'NEEDLE-FIRST-9a1', 20: 'NEEDLE-MID-9a1', 39: 'NEEDLE-LAST-9a1'}

def peak_footprint(pid):
    """Largest physical footprint (current or lifetime maximum) of a live process, in bytes."""
    buf = (ctypes.c_uint64 * 64)()
    if LIBPROC.proc_pid_rusage(pid, 4, buf) != 0:
        return 0
    v = list(buf)[2:]
    return max(v[7], v[28])

def run(args, timeout=240):
    # Index reports can exceed a macOS pipe's 16 KiB capacity. Drain to owned
    # temporary files while sampling RSS, then parse only the completed output.
    with tempfile.TemporaryFile(mode='w+t') as stdout, tempfile.TemporaryFile(mode='w+t') as stderr:
        p = subprocess.Popen([str(BIN), *args], stdout=stdout, stderr=stderr, text=True)
        peak, deadline = 0, time.monotonic() + timeout
        while p.poll() is None and time.monotonic() < deadline:
            peak = max(peak, peak_footprint(p.pid)); time.sleep(0.02)
        if p.poll() is None:
            p.kill(); p.wait()
            raise subprocess.TimeoutExpired([str(BIN), *args], timeout)
        stdout.seek(0);stderr.seek(0)
        return p.returncode, json.loads(stdout.read()), stderr.read(), peak

class StreamCapture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.p = pathlib.Path(self.tmp.name); self.root = self.p / 'repo'; self.root.mkdir(); self.owner = None
    def tearDown(self):
        if self.owner:
            if self.owner.poll() is None: self.owner.terminate()
            self.owner.communicate(timeout=8)
        self.tmp.cleanup()
    def files(self, lines):
        """Forty 1 MiB files: many short lines with needles, or one long line each."""
        filler = 'filler line of streamed content\n'
        for i in range(40):
            body = (filler * (2**20 // len(filler) + 1))[:2**20] if lines else chr(97 + i % 26) * (2**20 - 1) + '\n'
            if lines and i in NEEDLES:
                body = NEEDLES[i] + body[len(NEEDLES[i]):]
            (self.root / f'f{i:02}.txt').write_text(body)
    def contract(self, text):
        c = self.p / 'contract.tsv'; c.write_text('UG\t1\n' + text); return str(c)

    def test_needles_in_first_middle_and_last_files_across_streamed_requests(self):
        self.files(lines=True)
        text = ''.join(obligation(id=f'has{i}', rule='contains', scope='*', argument=n) + obligation(id=f'not{i}', rule='absent-text', scope='*', argument=n) for i, n in NEEDLES.items())
        code, r, err, peak = run(['check', '--root', str(self.root), '--contract', self.contract(text), '--no-cache', '--local'])
        self.assertEqual(code, 1, err)
        states = {o['id']: o['state'] for o in r['obligations']}
        self.assertEqual(states, {**{f'has{i}': 'verified' for i in NEEDLES}, **{f'not{i}': 'failed' for i in NEEDLES}})
        self.assertGreaterEqual(r['content_chunks']['core_requests'], 5)
        self.assertEqual(r['inputs']['problems'], {}); self.assertEqual(r['inputs']['bytes'], 40 * 2**20)
        for o in r['obligations']:
            self.assertEqual(o['chunk_coverage']['evaluated'], o['chunk_coverage']['chunks'], o)
            self.assertEqual(o['chunk_coverage']['omitted'], [])
        self.assertLess(peak, 150 * 2**20, peak)

    def test_native_syntax_failure_keeps_its_row_inside_a_chunked_check(self):
        self.files(lines=True)
        (self.root / 'bad.rs').write_text('fn main(\n')
        text = obligation(id='has', rule='contains', scope='*', argument=NEEDLES[0]) + obligation(id='syntax', rule='native', scope='bad.rs', argument='rust-syntax')
        code, r, err, _ = run(['check', '--root', str(self.root), '--contract', self.contract(text), '--no-cache', '--local'])
        self.assertEqual(code, 1, err); self.assertIn('content_chunks', r)
        self.assertEqual({o['id']: o['state'] for o in r['obligations']}, {'has': 'verified', 'syntax': 'failed'})

    def test_index_segments_a_row_over_the_frame_bound_and_indexes_every_file(self):
        self.files(lines=False)
        source = '%' * (6 * 2**20)
        (self.root / 'percent.txt').write_text(source)
        code, r, err, _ = run(['index', '--root', str(self.root), '--scope', '*', '--no-cache'])
        self.assertEqual((code, r['state']), (0, 'indexed'), err)
        self.assertEqual(r['inputs']['problems'], {})
        self.assertEqual(sorted(s['path'] for s in r['fact_sets']), sorted([*(f'f{i:02}.txt' for i in range(40)), 'percent.txt']))
        self.assertEqual(next(s['content_sha256'] for s in r['fact_sets'] if s['path']=='percent.txt'), hashlib.sha256(source.encode()).hexdigest())

    def test_multi_request_index_leaves_the_owner_core_alone(self):
        self.files(lines=False)
        self.owner = subprocess.Popen([str(BIN), 'session', '--root', str(self.root), '--directory', str(self.p / 'session'), '--idle-seconds', '300'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        sel = selectors.DefaultSelector(); sel.register(self.owner.stdout, selectors.EVENT_READ); self.assertTrue(sel.select(8))
        handle = json.loads(self.owner.stdout.readline())['handle']
        small = self.contract(obligation(id='one', rule='contains', scope='f00.txt', argument='aaaa'))
        _, before, err, _ = run(['check', '--root', str(self.root), '--contract', small, '--no-cache', '--session', handle])
        self.assertIn('computation_session', before, err)
        code, index, err, _ = run(['index', '--root', str(self.root), '--scope', '*', '--no-cache', '--session', handle])
        self.assertEqual((code, index['state'], len(index['fact_sets'])), (0, 'indexed', 40), err)
        self.assertLess(index['computation_session']['input_bytes'], 2**20, index['computation_session'])
        _, after, err, _ = run(['check', '--root', str(self.root), '--contract', small, '--no-cache', '--session', handle])
        self.assertIn('computation_session', after, err)
        self.assertEqual(after['computation_session']['core_pid'], before['computation_session']['core_pid'])
        self.assertEqual(after['obligations'][0]['computation'], 'reused')

    def test_session_directory_past_the_socket_path_limit_serves_warm_checks(self):
        (self.root / 'a.py').write_text('import pathlib\n')
        parent = self.p / ('p' * 100); parent.mkdir()
        directory = parent / ('session-' + 's' * 60)
        self.assertGreaterEqual(len(str(directory)), 150)
        self.owner = subprocess.Popen([str(BIN), 'session', '--root', str(self.root), '--directory', str(directory), '--idle-seconds', '300'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        sel = selectors.DefaultSelector(); sel.register(self.owner.stdout, selectors.EVENT_READ); self.assertTrue(sel.select(8))
        ready = json.loads(self.owner.stdout.readline()); endpoint = pathlib.Path(ready['session']['endpoint'])
        self.assertLessEqual(len(str(endpoint).encode()), 103); self.assertTrue(endpoint.is_socket())
        contract = self.contract(obligation(rule='fact-import', scope='a.py', argument='import pathlib'))
        cold = run(['check', '--root', str(self.root), '--contract', contract, '--no-cache', '--local'])
        warm = run(['check', '--root', str(self.root), '--contract', contract, '--no-cache', '--session', ready['handle']])
        self.assertEqual((warm[0], cold[0]), (0, 0), warm[2])
        self.assertIn('computation_session', warm[1])
        key = lambda r: [(o['id'], o['state'], o['computation_key']) for o in r['obligations']]
        self.assertEqual(key(warm[1]), key(cold[1]))
        self.owner.terminate(); self.owner.communicate(timeout=8)
        self.assertFalse(endpoint.exists()); self.assertFalse(directory.exists())

if __name__ == '__main__':
    unittest.main()
