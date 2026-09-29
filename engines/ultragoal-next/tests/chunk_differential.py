"""Chunked content evaluation against the unchunked evaluator on random small repos.

Each case builds one evaluation frame with content inline and the chunked form of
the same frame: an FC marker where the F rows stood, followed only by the rows a
semantic obligation keeps, and after the U rows the CH/CF/PC rows the core's CHUNK
command returns for a partition of the F rows. Partitions are random or
streaming-shaped (single-row chunks, chunks of only empty/unread/unavailable rows,
kept rows at chunk edges); chunk frames travel in INDEX requests of 1-8 frames
across 2-3 fresh session cores. Both frames go through the one-shot route and the
retained graph route of one session core; every RESULT, WITNESS and semantic row must agree,
and a failed absent-text names the file and line a Python reference finds.
UG_BINARY selects the core: its sibling `ug-core`.
usage: chunk_differential.py [CASES] [SEED]"""
import hashlib, json, pathlib, random, re, subprocess, sys, tempfile, unittest
from journey import BIN, CORE, obligation, row

RULES = ['contains', 'absent-text', 'fact-heading', 'fact-import', 'fact-import-member', 'fact-definition', 'fact-json-key', 'fact-json-valid', 'json-syntax', 'member', 'path-member']
LINES = ['import os', 'from x import y', 'use std::fmt;', 'fn needle() {}', 'def needle():', 'class Box:', '# Needle heading', '## other',
         'plain text needle', 'tab\there', 'percent 100%', 'π unicode', '', 'let x = 1;', 'export function f() {}', 'require("m")']
EXT = ['.py', '.rs', '.ts', '.md', '.json', '.txt']
ARGS = ['needle', 'import os', '# Needle heading', 'fn needle() {}', 'def needle():', 'use std::fmt;', 'absent', '100%', 'tab\there']
UNCOVERED = ('member', 'path-member', 'semantic')

def dec(s):
    return re.sub('%(0A|0D|09|25)', lambda m: {'0A': '\n', '0D': '\r', '09': '\t', '25': '%'}[m.group(1)], s)

def fields(line):
    return [dec(x) for x in line.split('\t')]

def selected(path, selector):
    if selector == '*':
        return True
    if selector.startswith('prefix:'):
        return path.startswith(selector[7:])
    return path == (selector[5:] if selector.startswith('file:') else selector)

def affected(selector, path):
    scope = selector[7:] if selector.startswith('prefix:') else selector[5:] if selector.startswith('file:') else selector
    return path == '' or selected(path, selector) or scope.startswith(path + '/')

def body(rng, ext):
    if ext == '.json':
        return rng.choice(['{"needle": 1, "a": [1,2]}', '{"other": true}', '{"needle": 1, "needle": 2}', '{broken', '[1, 2, 3]', '{}'])
    return '\n'.join(rng.choice(LINES) for _ in range(rng.randint(0, 8))) + rng.choice(['', '\n'])

def partition(rng, paths, empty, kept):
    n = len(paths)
    mode = rng.choice(['random', 'single', 'empty-runs', 'kept-edges'])
    if n < 2:
        cuts = []
    elif mode == 'random':
        cuts = sorted(rng.sample(range(1, n), rng.randint(0, n - 1)))
    elif mode == 'single':
        cuts = list(range(1, n))
    elif mode == 'empty-runs':
        cuts = [i for i in range(1, n) if (paths[i] in empty) != (paths[i - 1] in empty)]
    else:
        cuts = [i for i in range(1, n) if paths[i] in kept or paths[i - 1] in kept]
    return mode, [paths[a:b] for a, b in zip([0] + cuts, cuts + [n])]

def case(rng):
    files = {}
    for _ in range(rng.randint(1, 14)):
        ext = rng.choice(EXT)
        files[f'{rng.choice(["src/", "a/", "src/b/", ""])}f{rng.randint(0, 40)}{ext}'] = body(rng, ext)
    paths = sorted(files)
    scopes = ['*', 'prefix:src/', 'prefix:a/', rng.choice(paths), 'file:' + rng.choice(paths), 'missing.py']
    obs, semantic, specs = [], [], []
    for i in range(rng.randint(1, 5)):
        scope = rng.choice(scopes)
        if rng.random() < 0.2:
            semantic.append(scope)
            obs.append(obligation(id=f'o{i}', rule='semantic', scope=scope, argument='alignment', assurance='semantic'))
            continue
        rule = rng.choice(RULES)
        arg = rng.choice(ARGS)
        if rule in ('member', 'path-member', 'fact-json-valid', 'json-syntax'):
            arg = ''
        elif rule == 'fact-json-key':
            arg = 'needle'
        elif rule == 'fact-import-member':
            arg = 'x|y|y'
        projection = 'membership' if rule in ('member', 'path-member') else 'content'
        obs.append(obligation(id=f'o{i}', rule=rule, scope=scope, projection=projection, argument=arg)); specs.append((f'o{i}', rule, scope, arg))
    unavailable = set(p for p in paths if rng.random() < 0.08)
    unread = set(p for p in paths if rng.random() < 0.08) - unavailable
    digest = {p: 'unavailable' if p in unavailable else 'unread' if p in unread else hashlib.sha256(files[p].encode()).hexdigest() for p in paths}
    def frow(p, content):
        return row('F', p, digest[p], '' if p in unavailable or p in unread else content)
    head = 'UG\t1\n' + ''.join(obs)
    op = row('OP', 'op', 'running', 'contract', 'timely')
    tail = ''.join(row('U', p, 'file', 'unreadable') for p in sorted(unavailable))
    kept = [p for p in paths if any(selected(p, s) for s in semantic)]
    inline = head + op + ''.join(frow(p, files[p]) for p in paths) + tail
    marked = head + op + row('FC') + ''.join(frow(p, files[p]) for p in kept) + tail
    empty = set(p for p in paths if p in unavailable or p in unread or files[p] == '')
    mode, parts = partition(rng, paths, empty, set(kept))
    chunks = ['CHUNK\n' + head + row('CI', str(i)) + ''.join(frow(p, files[p]) for p in part) for i, part in enumerate(parts)]
    meta = {'mode': mode, 'parts': parts, 'digest': digest, 'kept': kept, 'empty': empty, 'obs': specs, 'files': files,
            'missing': lambda scope: sum(affected(scope, p) for p in unavailable) + sum(selected(p, scope) for p in unread)}
    return inline, marked, chunks, meta

class Session:
    def __init__(self):
        self.p = subprocess.Popen([str(CORE), '--threads', '4', '--gpu', 'off', '--', '--session'], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        self.seq = 0
    def response(self):
        h = self.p.stdout.readline().decode().split('\t')
        assert len(h) == 4 and h[0] == 'S1' and h[1] == str(self.seq), h
        return h[2], self.p.stdout.read(int(h[3])).decode()
    def ask(self, text):
        b = f'S1\t{self.seq}\tONE\n{text}'.encode()
        self.p.stdin.write(f'{len(b)}\n'.encode() + b); self.p.stdin.flush()
        state, out = self.response()
        assert state == 'done', state
        self.seq += 1
        return out
    def index(self, frames):
        """One INDEX request of several frames, as the frontend sends a chunk group."""
        control = f'S1\t{self.seq}\tINDEX\t{len(frames)}\n'.encode()
        self.p.stdin.write(b''.join(f'{len(b)}\n'.encode() + b for b in [control] + [f.encode() for f in frames])); self.p.stdin.flush()
        out = []
        while True:
            state, text = self.response()
            out.append(text)
            if state == 'done':
                break
        self.seq += 1
        return ''.join(out)
    def close(self):
        self.p.stdin.write(b'0\n'); self.p.stdin.close(); self.p.stdout.close(); assert self.p.wait(timeout=60) == 0

def dispatch(rng, frames):
    """Chunk frames in requests of 1-8 frames, round-robin over 2-3 fresh cores."""
    cores = [Session() for _ in range(rng.randint(2, 3))]
    out, i, requests = [], 0, 0
    try:
        while i < len(frames):
            n = rng.randint(1, 8)
            out.append(cores[requests % len(cores)].index(frames[i:i + n]))
            i += n; requests += 1
    finally:
        for c in cores:
            c.close()
    return ''.join(out), requests, len(cores)

def results(text):
    return [r.split('\t') for r in text.splitlines() if r.startswith('RESULT\t')]

def semantic_rows(text):
    return [r for r in text.splitlines() if r.startswith(('REQUEST\t', 'SEMANTIC_UNAVAILABLE\t'))]

def normal(rs):
    # The computed/reused/miss work label is route-specific; everything else must agree.
    return [r[:7] + r[8:] for r in rs]

def witnesses(text):
    return {f[1]: f[2:] for f in map(fields, text.splitlines()) if f[0] == 'WITNESS'}

def expected_witnesses(meta):
    """Reference: a failed absent-text names the smallest selected path holding the text and the line where it first starts."""
    out = {}
    for oid, rule, scope, arg in meta['obs']:
        hits = sorted(p for p, c in meta['files'].items() if rule == 'absent-text' and selected(p, scope) and meta['digest'][p] not in ('unavailable', 'unread') and arg in c)
        if hits:
            out[oid] = [hits[0], str(meta['files'][hits[0]][:meta['files'][hits[0]].index(arg)].count('\n') + 1)]
    return out

def omitted(field):
    return [dec(x) for x in field[:-1].split('\t')] if field else []

def coverage(text):
    """Obligation id -> [chunks, evaluated, omitted paths, unavailable, alignment]."""
    out = {}
    for line in text.splitlines():
        if line.startswith('CHUNK_COVERAGE\t'):
            f = fields(line)
            assert len(f) == 7, f
            out[f[1]] = [f[2], f[3], omitted(f[4]), f[5], f[6]]
    return out

def listing(text):
    """(CH fields, [(path, digest)]) per chunk; CF rows must follow their CH row directly."""
    tags = ''.join({'CH': 'H', 'CF': 'F', 'PC': 'P', 'PW': 'W'}[l.split('\t')[0]] for l in text.splitlines())
    assert re.fullmatch('(HF*(PW?)*)*', tags), tags
    blocks = []
    for line in text.splitlines():
        f = fields(line)
        if f[0] == 'CH':
            blocks.append((f, []))
        elif f[0] == 'CF':
            assert len(f) == 3, f
            blocks[-1][1].append((f[1], f[2]))
    return blocks

def run(cases, seed):
    rng = random.Random(seed); s = Session()
    counts = {'cases': 0, 'results': 0, 'chunks': 0, 'requests': 0, 'cores': 0, 'kept_rows': 0, 'empty_only_chunks': 0,
              'kept_edge_chunks': 0, 'modes': {}, 'states': {}, 'unavailable_nonzero': 0, 'witnesses': 0, 'mismatches': 0}
    try:
        for n in range(cases):
            if n and n % 300 == 0:  # a session core serves at most 4096 requests
                s.close(); s = Session()
            inline, marked, chunks, meta = case(rng)
            partials, requests, cores = dispatch(rng, chunks)
            blocks = listing(partials)
            assert len(blocks) == len(meta['parts']), (len(blocks), meta['parts'])
            for (ch, cf), part in zip(blocks, meta['parts']):
                assert ch[5] == str(len(part)) and cf == [(p, meta['digest'][p]) for p in part], (ch, cf, part)
            chunked = marked + partials
            want_run = s.ask('\n' + inline); got_run = s.ask('\n' + chunked)
            want_graph = s.ask(inline); got_graph = s.ask(chunked)
            for label, want, got in [('run', want_run, got_run), ('graph', want_graph, got_graph), ('graph-vs-run', want_run, got_graph)]:
                if (want.startswith('ERROR') or got.startswith('ERROR') or normal(results(want)) != normal(results(got))
                        or semantic_rows(want) != semantic_rows(got) or witnesses(want) != witnesses(got)):
                    counts['mismatches'] += 1
                    print(json.dumps({'case': n, 'route': label, 'want': want[:600], 'got': got[:600], 'frame': chunked[:600]}))
            want_witness = expected_witnesses(meta)
            for label, text in [('run', got_run), ('graph', got_graph)]:
                got_witness = {k: v for k, v in witnesses(text).items() if k in {o[0] for o in meta['obs'] if o[1] == 'absent-text'}}
                if got_witness != want_witness:
                    counts['mismatches'] += 1
                    print(json.dumps({'case': n, 'route': label, 'witness': got_witness, 'reference': want_witness}))
                cov = coverage(text)
                for r in results(text):
                    content = r[11] not in UNCOVERED
                    assert (r[1] in cov) == content, (label, r, cov)
                    if content:
                        missing = str(meta['missing'](dec(r[8])))
                        assert cov[r[1]] == [str(len(chunks)), str(len(chunks)), [], missing, 'aligned'], (label, cov[r[1]], missing)
                        counts['unavailable_nonzero'] += missing != '0'
            for r in results(got_graph):
                counts['states'][r[2]] = counts['states'].get(r[2], 0) + 1
            counts['modes'][meta['mode']] = counts['modes'].get(meta['mode'], 0) + 1
            counts['kept_rows'] += len(meta['kept'])
            counts['empty_only_chunks'] += sum(all(p in meta['empty'] for p in part) for part in meta['parts'])
            counts['kept_edge_chunks'] += sum(part[0] in meta['kept'] or part[-1] in meta['kept'] for part in meta['parts'])
            counts['cases'] += 1; counts['results'] += len(results(got_run)); counts['witnesses'] += len(want_witness); counts['chunks'] += len(chunks)
            counts['requests'] += requests; counts['cores'] += cores
    finally:
        s.close()
    return counts

def targeted():
    """Omitted chunks named by path, unavailable counts, missing partials and malformed frames."""
    s = Session(); out = {}
    big = 'x' * (8 * 1024 * 1024 + 1)
    obs = (obligation(id='has', rule='contains', scope='*', argument='needle') + obligation(id='none', rule='absent-text', scope='*', argument='needle') +
           obligation(id='gone', rule='absent-text', scope='*', argument='zzz') + obligation(id='head', rule='fact-heading', scope='c.md', argument='# Needle heading') +
           obligation(id='sem', rule='semantic', scope='prefix:c', argument='alignment', assurance='semantic'))
    head = 'UG\t1\n' + obs; op = row('OP', 'op', 'running', 'contract', 'timely')
    def split(files, first, gone=(), unread=(), extra=''):
        digest = lambda p: 'unavailable' if p in gone else 'unread' if p in unread else hashlib.sha256(files[p].encode()).hexdigest()
        f = lambda p, c: row('F', p, digest(p), '' if p in gone or p in unread else c)
        paths = sorted(files); cut = paths.index(first)
        parts = (s.ask('CHUNK\n' + head + row('CI', '0') + ''.join(f(p, files[p]) for p in paths[:cut])) +
                 s.ask('CHUNK\n' + head + row('CI', '1') + ''.join(f(p, files[p]) for p in paths[cut:])))
        tail = ''.join(row('U', p, 'file', 'unreadable') for p in sorted(gone)) + extra
        marked = head + op + row('FC') + ''.join(f(p, files[p]) for p in paths if selected(p, 'prefix:c')) + tail
        return marked, parts, f, head + op + ''.join(f(p, files[p]) for p in paths) + tail
    state = lambda text: {r[1]: r[2] for r in results(text) if r[1] != 'sem'}
    chunk_states = lambda parts: [l.split('\t')[:4] for l in parts.splitlines() if l.startswith('CH\t')]
    both = lambda frame: [s.ask(frame)[:60], s.ask('\n' + frame)[:60]]
    try:
        files = {'a.py': 'needle here', 'b.py': 'nothing', 'c.md': '# Needle heading'}
        marked, parts, f, inline = split(files, 'b.py')
        text = s.ask(marked + parts)
        out['complete'] = state(text); out['complete_run'] = state(s.ask('\n' + marked + parts)); out['complete_coverage'] = coverage(text); out['complete_witness'] = witnesses(text)
        out['kept_semantic'] = [semantic_rows(text), semantic_rows(s.ask(inline))]
        m1, p1, _, _ = split(files | {'z.txt': big}, 'b.py')
        out['chunk_1_state'] = chunk_states(p1)
        text = s.ask(m1 + p1); out['chunk_1_omitted'] = state(text); out['chunk_1_omitted_coverage'] = coverage(text); out['chunk_1_witness'] = witnesses(text)
        m0, p0, _, _ = split(files | {'a2.txt': big}, 'b.py')
        out['chunk_0_state'] = chunk_states(p0)
        text = s.ask(m0 + p0); out['chunk_0_omitted'] = state(text); out['chunk_0_omitted_coverage'] = coverage(text); out['chunk_0_witness'] = witnesses(text)
        mu, pu, _, _ = split(files | {'u.py': 'needle', 'r.py': 'x'}, 'b.py', gone={'u.py'}, unread={'r.py'},
                             extra=row('U', 'link', 'symlink', 'unsupported source kind') + row('I', 'docs', 'Limit'))
        out['unavailable_coverage'] = coverage(s.ask(mu + pu))
        dropped = '\n'.join(l for l in parts.splitlines() if not (l.startswith('PC\t1\tabsent-text') and '\tzzz\t' in l)) + '\n'
        text = s.ask(marked + dropped); out['partial_missing'] = state(text); out['partial_missing_coverage'] = coverage(text)
        lines = parts.splitlines(keepends=True)
        cf = [i for i, l in enumerate(lines) if l.startswith('CF\t')]
        ch1 = [i for i, l in enumerate(lines) if l.startswith('CH\t')][1]
        assert fields(lines[ch1].rstrip('\n'))[5] == '2' and [lines[i][:8] for i in cf] == ['CF\ta.py\t', 'CF\tb.py\t', 'CF\tc.md\t'], parts
        unchunked = head + op + ''.join(f(p, '') for p in sorted(files))
        malformed = {
            'coverage_mismatch': marked + parts.replace('b.py', 'x.py', 1),
            'content_in_chunked_frame': head + op + row('FC') + ''.join(f(p, files[p]) for p in sorted(files)) + parts,
            'partial_without_chunk': unchunked + ''.join(l for l in lines if l.startswith('PC\t')),
            'listing_without_chunk': unchunked + ''.join(l for l in lines if l.startswith('CF\t')),
            'marker_without_chunk': head + op + row('FC') + f('c.md', files['c.md']),
            'chunk_row_in_evaluation': marked + row('CI', '9') + parts,
            # b.py is not kept, so only the CH count can catch its missing CF row.
            'cf_rows_one_short': marked + ''.join(l for i, l in enumerate(lines) if i != cf[1]),
            'cf_count_one_short': marked + ''.join(row(*fields(l.rstrip('\n'))[:5], '1', fields(l.rstrip('\n'))[6]) if i == ch1 else l for i, l in enumerate(lines)),
            'cf_without_ch': marked + parts + row('CF', 'z.py', hashlib.sha256(b'').hexdigest()),
            'kept_digest_differs': head + op + row('FC') + row('F', 'c.md', hashlib.sha256(b'other').hexdigest(), files['c.md']) + parts,
            'kept_missing_from_chunks': marked + row('F', 'c0.md', hashlib.sha256(b'c0').hexdigest(), 'c0') + parts,
            'non_kept_f_row': head + op + row('FC') + f('b.py', '') + f('c.md', files['c.md']) + parts,
            'no_fc': head + op + f('c.md', files['c.md']) + parts,
            'two_fc': head + op + row('FC') + row('FC') + f('c.md', files['c.md']) + parts,
            'unsorted_cf': marked + ''.join(lines[cf[-1]] if i == cf[-2] else lines[cf[-2]] if i == cf[-1] else l for i, l in enumerate(lines)),
        }
        out['malformed'] = {k: both(v) for k, v in malformed.items()}
    finally:
        s.close()
    return out

class ChunkDifferential(unittest.TestCase):
    def test_random_splits_match_unchunked(self):
        counts = run(300, 20260924)
        self.assertEqual(counts['mismatches'], 0, counts)
        # The streaming shapes were exercised, not merely possible.
        self.assertEqual(set(counts['modes']), {'random', 'single', 'empty-runs', 'kept-edges'}, counts)
        for key in ['kept_rows', 'empty_only_chunks', 'kept_edge_chunks', 'unavailable_nonzero', 'witnesses']:
            self.assertGreater(counts[key], 0, (key, counts))
        self.assertGreater(counts['requests'], counts['cases'], counts)
    def test_omission_and_malformed_frames(self):
        out = targeted()
        self.assertEqual(out['complete'], {'has': 'verified', 'none': 'failed', 'gone': 'verified', 'head': 'verified'})
        self.assertEqual(out['complete_run'], out['complete'])
        # The failed absent-text names where the text is; an omitted chunk holding it names nothing.
        self.assertEqual(out['complete_witness'], {'none': ['a.py', '1']})
        self.assertEqual(out['chunk_1_witness'], {'none': ['a.py', '1']})
        self.assertEqual(out['chunk_0_witness'], {})
        self.assertEqual(out['complete_coverage']['gone'], ['2', '2', [], '0', 'aligned'])
        semantic, inline = out['kept_semantic']
        self.assertEqual(semantic, inline)
        self.assertTrue(semantic and semantic[0].startswith('REQUEST\tsem\t') and '# Needle heading' in semantic[0], semantic)
        self.assertEqual(out['chunk_1_state'], [['CH', '0', 'complete', ''], ['CH', '1', 'omitted', 'chunk content byte bound']])
        self.assertEqual(out['chunk_1_omitted'], {'has': 'verified', 'none': 'failed', 'gone': 'unknown', 'head': 'unknown'})
        # The file over 8 MiB is named by path where it is in scope and not where it is out of scope.
        self.assertEqual(out['chunk_1_omitted_coverage']['gone'], ['2', '1', ['b.py', 'c.md', 'z.txt'], '0', 'aligned'])
        self.assertEqual(out['chunk_1_omitted_coverage']['head'], ['2', '1', ['c.md'], '0', 'aligned'])
        self.assertEqual(out['chunk_0_state'], [['CH', '0', 'omitted', 'chunk content byte bound'], ['CH', '1', 'complete', '']])
        self.assertEqual(out['chunk_0_omitted'], {'has': 'unknown', 'none': 'unknown', 'gone': 'unknown', 'head': 'verified'})
        self.assertEqual(out['chunk_0_omitted_coverage']['head'], ['2', '2', [], '0', 'aligned'])
        self.assertEqual(out['chunk_0_omitted_coverage']['gone'], ['2', '1', ['a.py', 'a2.txt'], '0', 'aligned'])
        # In scope of `*`: the U row of u.py, unread r.py, the symlink's U row and the docs issue; none for c.md.
        self.assertEqual(out['unavailable_coverage']['gone'], ['2', '2', [], '4', 'aligned'])
        self.assertEqual(out['unavailable_coverage']['head'], ['2', '2', [], '0', 'aligned'])
        self.assertEqual(out['partial_missing']['gone'], 'unknown')
        self.assertEqual(out['partial_missing_coverage']['gone'][4], 'partials-missing')
        for key, texts in out['malformed'].items():
            for route, text in zip(['graph', 'run'], texts):
                self.assertTrue(text.startswith('ERROR\t'), (key, route, text))
    @unittest.skipUnless(BIN.exists(), 'needs the ultragoal frontend beside the core')
    def test_semantic_files_keep_content_beside_chunked_rules(self):
        with tempfile.TemporaryDirectory() as d:
            root = pathlib.Path(d, 'repo'); root.mkdir()
            for i in range(3):
                (root / f'big{i}.txt').write_text('filler line\n' * 90000)
            (root / 'spec.md').write_text('Cancellation keeps the original bytes.\n')
            contract = pathlib.Path(d, 'contract.tsv')
            def check(semantic_scope):
                contract.write_text('UG\t1\n' + obligation('absent', 'absent-text', '*', argument='TODO')
                                    + obligation('sem', 'semantic', semantic_scope, argument='alignment', assurance='semantic'))
                p = subprocess.run([str(BIN), 'check', '--root', str(root), '--contract', str(contract), '--no-cache', '--local'],
                                   capture_output=True, text=True, timeout=120)
                report = json.loads(p.stdout)
                return report, {o['id']: o['state'] for o in report['obligations']}, [s for s in report['semantic'] if s.get('obligation') == 'sem']
            report, states, unavailable = check('spec.md')
            self.assertIn('content_chunks', report)
            self.assertEqual((states['absent'], unavailable), ('verified', []))
            self.assertEqual(report['coverage']['semantic_requests_prepared'], 1)
            report, states, unavailable = check('*')
            self.assertIn('content_chunks', report)
            self.assertEqual((states['absent'], states['sem'], unavailable), ('verified', 'unknown', []))
            self.assertEqual(report['coverage']['semantic_requests_prepared'], 1)
            self.assertEqual(report['inputs']['problems'], {})

if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1].isdigit():
        print(json.dumps(run(int(sys.argv[1]), int(sys.argv[2]) if len(sys.argv) > 2 else 1)))
        print(json.dumps(targeted()))
    else:
        unittest.main()
