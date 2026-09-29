"""Chunked select ranking equals one frame, through the real binary.

Every statistic of Bend's ranking stages is a sum of per-chunk rows, so the answer must not
depend on how candidates are cut into frames. Each query runs with the largest chunk budget
(one frame when the candidates fit 16 MiB) and again with small forced budgets
(UG_SELECT_CHUNK_BYTES), cutting files into many partial rows; everything except the
measurements must be equal. Random repositories exercise duplicate excerpts, grammar
changes, secret-shaped rows, second windows and tie-only queries (UG_SELECT_CASES, default
120); the development repositories helix and django (select_differential.DEV) run at 4096
bytes. A class of byte-identical rows larger than the frame bound is named, not refused."""
import hashlib, json, os, pathlib, random, shutil, subprocess, tempfile, unittest
from journey import BIN, row

WORDS = 'cache token parser render undo history editor buffer socket retry config schema migrate index query stream chunk frame lane budget'.split()
LINES = ['def {w}_{v}():', 'class {W}{V}:', 'fn {w}_{v}() {{}}', 'assert {w} == {v}', 'import {w}', 'from {w} import {v}',
         'use {w}::{v};', 'expect({w}).toBe({v})', 'export function {w}{V}() {{}}', 'let {w} = {v};', '# {w} {v} notes',
         'the {w} and {v} of it', '{w}.assert_{v}()', 'return {w}', '// {w} handles {v}', 'x = 1']
EXTENSIONS = ['.py', '.rs', '.ts', '.js', '.md', '.txt', '.json']
SECRET = '-----BEGIN EC PRIVATE KEY-----'
FILLER = '\n' + '#' * 4200 + '\n'
ONE_FRAME = 16 * 1024 * 1024


def line(rng, words=WORDS):
    w, v = rng.choice(words), rng.choice(words)
    return rng.choice(LINES).format(w=w, v=v, W=w.title(), V=v.title())


def snippet(rng):
    if rng.random() < 0.02:
        return SECRET + '\n' + line(rng)
    return '\n'.join(line(rng) for _ in range(rng.randint(1, 3)))


def write_index(root, files):
    """files: {path: (text, spans)}; spans are (start, end) byte offsets."""
    sets = []
    for path in sorted(files):
        text, spans = files[path]
        data = text.encode()
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(data)
        facts = [['line', None, '1', str(a), str(b)] for a, b in spans]
        sets.append({'path': path, 'content_sha256': hashlib.sha256(data).hexdigest(), 'representation': 'source-spans/2', 'facts': facts})
    report = {'root': str(root.resolve()), 'scope': '*', 'manifest': sorted(files), 'fact_sets': sets, 'obligations': []}
    path = root.parent / 'index.json'
    path.write_text(json.dumps(report))
    return path


def repository(rng, root):
    """Themed directories and a skewed snippet pool make ties and aliases; many-window files
    are split across chunks at small budgets."""
    pool = [snippet(rng) for _ in range(rng.randint(150, 900))]
    weights = [1 / (k + 1) ** rng.uniform(0.3, 1.2) for k in range(len(pool))]
    dirs = [f'd{k}_{rng.choice(WORDS)}' for k in range(rng.randint(4, 30))]
    files = {}
    for k in range(rng.randint(600, 1600)):
        first = rng.choices(pool, weights)[0]
        text, spans = first, [(0, len(first.encode()))]
        shape = rng.random()
        if shape < 0.2:  # a second window past the 4 KiB window bound
            second = rng.choices(pool, weights)[0]
            start = len((first + FILLER).encode())
            text, spans = first + FILLER + second, spans + [(start, start + len(second.encode()))]
        elif shape < 0.3:  # many windows in one file
            text, spans = '', []
            for _ in range(rng.randint(2, 40)):
                piece = rng.choices(pool, weights)[0] + FILLER
                start = len(text.encode())
                text += piece
                spans.append((start, start + len(piece.encode()) - len(FILLER)))
        name = rng.choice(WORDS) if rng.random() < 0.3 else f'f{k}'
        files[f'{rng.choice(dirs)}/{name}{k}{rng.choice(EXTENSIONS)}'] = (text, spans)
    return write_index(root, files)


def select(root, report, query, chunk=None, details=True):
    env = {k: v for k, v in os.environ.items() if k != 'UG_SELECT_CHUNK_BYTES'}
    if chunk:
        env['UG_SELECT_CHUNK_BYTES'] = str(chunk)
    p = subprocess.run([str(BIN), 'select', '--root', str(root), '--report', str(report), '--query', query, '--local'] + (['--details'] if details else []),
                       capture_output=True, text=True, env=env, timeout=1800)
    assert p.returncode == 0, (p.returncode, p.stdout[-3000:], p.stderr[-2000:])
    return json.loads(p.stdout)


def ranked(result):
    return {k: v for k, v in result.items() if k != 'measurements'}


class SelectChunks(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.base = pathlib.Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_chunked_ranking_equals_one_frame(self):
        cases = int(os.environ.get('UG_SELECT_CASES', '120'))
        rng = random.Random(20260924)
        stats = {'cases': 0, 'differences': 0, 'match_frames': 0, 'passage_frames': 0, 'aliases': 0, 'groups': 0}
        repo = 0
        while stats['cases'] < cases:
            root = self.base / f'r{repo}' / 'repo'
            report = repository(rng, root)
            sizes = [4096, 6144, 8192, 12288, 16384, 24576, 32768, 49152, 65536]
            for _ in range(5):
                query = ' '.join(rng.sample(WORDS, rng.randint(1, 4))) if rng.random() < 0.9 else 'zzz nothing matches'
                single = select(root, report, query, ONE_FRAME)
                self.assertEqual(single['measurements']['ranking']['route'], 'single-frame')
                stats['aliases'] += single['measurements']['duplicate_candidates_represented_as_aliases']
                stats['groups'] += len(single['shortlist'])
                for chunk in rng.sample(sizes, 2):
                    chunked = select(root, report, query, chunk)
                    measured = chunked['measurements']['ranking']
                    self.assertEqual(measured['route'], 'chunked', (repo, query, chunk))
                    stats['cases'] += 1
                    stats['match_frames'] += measured['stages']['match']['frames']
                    stats['passage_frames'] += measured['stages']['passage']['frames']
                    if ranked(chunked) != ranked(single):
                        stats['differences'] += 1
                        print('DIFFERENCE', repo, repr(query), chunk, flush=True)
            shutil.rmtree(root.parent)
            repo += 1
        print('SELECT_CHUNKS', json.dumps(stats, sort_keys=True), flush=True)
        self.assertEqual(stats['differences'], 0, stats)

    def test_development_repositories_at_4096_bytes(self):
        from select_differential import DEV
        repos = [r for r in ('helix', 'django') if (DEV / 'idx' / f'{r}.json').is_file()]
        if not repos:
            self.skipTest(f'development set absent at {DEV}')
        stats = {}
        for repo in repos:
            query = json.loads((DEV / 'questions' / f'{repo}.json').read_text())['questions'][0]['question']
            root, report = DEV / 'repos' / repo, DEV / 'idx' / f'{repo}.json'
            wide, narrow = select(root, report, query, ONE_FRAME, False), select(root, report, query, 4096, False)
            stats[repo] = {'equal': ranked(wide) == ranked(narrow), 'frames': [wide['measurements']['ranking']['stages']['match']['frames'], narrow['measurements']['ranking']['stages']['match']['frames']],
                           'ms': [wide['measurements']['ranking']['ms'], narrow['measurements']['ranking']['ms']], 'groups': len(wide['shortlist'])}
        print('SELECT_CHUNKS_DEV', json.dumps(stats), flush=True)
        self.assertTrue(all(s['equal'] for s in stats.values()), stats)

    def test_alias_class_over_the_frame_bound_is_named(self):
        # 4,600 byte-identical 3.9 KB excerpts (about 18 MiB of rows) that match the query,
        # beside 400 distinct ones: the chosen window's aliases pass the 16 MiB frame bound.
        rng = random.Random(7)
        root = self.base / 'big' / 'repo'
        common = 'def undo_history():\n' + '\n'.join(f'    history.append({k})  # undo editor buffer' for k in range(88))
        assert 3800 < len(common) < 4096
        files = {f'vendor/copy{k:05}/undo.py': (common, [(0, len(common))]) for k in range(4600)}
        for k in range(400):
            text = snippet(rng) + f'\n# unique {k}'
            files[f'src/m{k:04}_{rng.choice(WORDS)}.py'] = (text, [(0, len(text))])
        report = write_index(root, files)
        result = select(root, report, 'undo history editor')
        unassessed = result['coverage']['unassessed']
        self.assertGreater(len(unassessed), 0)
        self.assertTrue(all(u['reason'] == 'select frame bound' and u['path'].startswith('vendor/') for u in unassessed))
        self.assertEqual(result['measurements']['ranking']['unranked'], len(unassessed))
        left = {u['id'] for u in unassessed}
        vendor = [c for c in result['shortlist'] if c['path'].startswith('vendor/')]
        self.assertEqual(len(vendor), 1)
        named = {a['id'] for a in vendor[0]['aliases']}
        self.assertFalse(named & left)
        # Every copy is either an alias of the one group, its representative, or named.
        self.assertEqual(len(named) + 1 + len(left), 4600)
        print('OVERSIZED', json.dumps({'aliases': len(named), 'named': len(left), 'fuse_bytes': result['measurements']['ranking']['stages']['fuse']['bytes']}), flush=True)


if __name__ == '__main__':
    unittest.main()
