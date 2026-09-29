"""Bend's four ranking stages against the Python reference (select_reference.py, the same
fixed-point arithmetic) on the development questions of the select-design study. For every
question the real `ultragoal select` shortlist and coverage, and the core's own pool and best
windows (its stage commands driven as src/context.rs frames them), must equal the reference.

UG_SELECT_DEV (default .codex-worktree/opus-work/select-design) holds questions/, repos/, idx/
and cache/<repo>.refused.json (the rows Policy.disclosable refuses); UG_SELECT_REPOS narrows
the repositories. The test is skipped when the development set is absent."""
import json, os, pathlib, subprocess, sys, unittest, urllib.parse
from journey import BIN, CORE, row
import select_reference as R

HERE = pathlib.Path(__file__).resolve().parent
DEV = pathlib.Path(os.environ.get('UG_SELECT_DEV', HERE.parents[2] / '.codex-worktree/opus-work/select-design'))
REPOS = os.environ.get('UG_SELECT_REPOS', 'helix nushell django sphinx vite eslint hugo').split()
BUDGET = 2 * 1024 * 1024


def grammar(path):
    for ends, g in [(('.json',), 'json/1'), (('.rs',), 'rust-structure/1'), (('.py',), 'python-structure/1'),
                    (('.ts', '.tsx', '.js', '.jsx'), 'ecmascript-structure/1'), (('.md',), 'markdown-lines/1')]:
        if path.endswith(ends):
            return g
    return 'text-lines/1'


def select(repo, query, g=128, details=False):
    args = [str(BIN), 'select', '--root', str(DEV / 'repos' / repo), '--report', str(DEV / 'idx' / f'{repo}.json'),
            '--query', query, '--local', '--shortlist', str(g)] + (['--details'] if details else [])
    p = subprocess.run(args, capture_output=True, text=True, timeout=1800)
    assert p.returncode == 0, (p.returncode, p.stdout[-2000:], p.stderr[-2000:])
    return json.loads(p.stdout)


def candidates(repo, provenance):
    """The frontend's candidate windows, from --details provenance and the current bytes."""
    root, data, out = DEV / 'repos' / repo, {}, []
    for p in provenance:
        if p['path'] not in data:
            data[p['path']] = (root / p['path']).read_bytes()
        excerpt = data[p['path']][p['start']:p['end']].decode()
        out.append({'id': p['id'], 'path': p['path'], 'digest': p['input_sha256'], 'start': p['start'], 'end': p['end'],
                    'excerpt': excerpt, 'grammar': grammar(p['path'])})
    return out


def crow(c):
    return row('C', c['id'], c['path'], c['digest'], str(c['start']), str(c['end']), c['excerpt'])


def core(frame):
    p = subprocess.run([str(CORE)], input=frame.encode(), capture_output=True, timeout=1800)
    assert p.returncode == 0, p.stderr[:500]
    rows = [[urllib.parse.unquote(x) for x in line.split('\t')] for line in p.stdout.decode().splitlines()]
    assert not any(r[0] == 'ERROR' for r in rows), rows[:3]
    return rows


def chunks(cands):
    """Whole files within the budget where they fit, as src/context.rs cuts them."""
    out, cur, size, k = [], [], 0, 0
    while k < len(cands):
        end = k
        while end < len(cands) and cands[end]['path'] == cands[k]['path']:
            end += 1
        if cur and size + sum(len(crow(c).encode()) for c in cands[k:end]) > BUDGET:
            out.append(cur); cur, size = [], 0
        for c in cands[k:end]:
            if cur and size + len(crow(c).encode()) > BUDGET:
                out.append(cur); cur, size = [], 0
            cur.append(c); size += len(crow(c).encode())
        k = end
    return out + ([cur] if cur else [])


def core_stages(query, cands, g):
    """The core's pool and best windows."""
    head = row('QUERY', query)
    partial = [r for ch in chunks(cands) for r in core('SELECT_MATCH\n' + head + ''.join(map(crow, ch)))]
    ranked = core('FILE_RANK\n' + head + row('SHORTLIST', str(g)) + ''.join(row(*r) for r in partial))
    pool = [r for r in ranked if r[0] == 'POOL']
    pooled = {r[1] for r in pool}
    passage = 'SELECT_PASSAGE\n' + head + ''.join(row(*r) for r in ranked if r[0] == 'NORM') + ''.join(row(*r) for r in ranked if r[0] == 'IDF')
    bests = [r for ch in chunks([c for c in cands if c['path'] in pooled]) for r in core(passage + ''.join(map(crow, ch)))]
    return [(r[1], int(r[3])) for r in pool], {r[1]: (r[2], int(r[3])) for r in bests}


class SelectDifferential(unittest.TestCase):
    def test_stages_equal_the_reference_on_development_questions(self):
        repos = [r for r in REPOS if (DEV / 'questions' / f'{r}.json').is_file() and (DEV / 'idx' / f'{r}.json').is_file()]
        if not repos:
            self.skipTest(f'development set absent at {DEV}')
        stats = {'questions': 0, 'differences': 0, 'shortlist_groups': 0, 'pooled_files': 0}
        for repo in repos:
            questions = json.loads((DEV / 'questions' / f'{repo}.json').read_text())['questions']
            refused = set(json.loads((DEV / 'cache' / f'{repo}.refused.json').read_text()))
            cands = None
            for q in questions:
                result = select(repo, q['question'], details=cands is None)
                if cands is None:
                    cands = candidates(repo, result['candidate_provenance'])
                admitted = [c for c in cands if c['id'] not in refused]
                want = R.rank(q['question'], admitted, 128)
                pool, best = core_stages(q['question'], cands, 128)
                got = {'shortlist': [(c['id'], c['lexical_score'], c['path']) for c in result['shortlist']],
                       'files': result['coverage']['ranking']['files'], 'matched': result['coverage']['ranking']['files_matched'],
                       'stop': result['coverage']['ranking']['stopping_score'], 'pool': pool, 'best': best}
                same = {k: got[k] == want[k] for k in got}
                stats['questions'] += 1
                stats['differences'] += not all(same.values())
                stats['shortlist_groups'] += len(got['shortlist'])
                stats['pooled_files'] += len(pool)
                print(q['id'], 'SAME' if all(same.values()) else 'DIFF ' + ' '.join(k for k, v in same.items() if not v), flush=True)
        print('SELECT_DIFFERENTIAL', json.dumps(stats), flush=True)
        self.assertEqual(stats['differences'], 0, stats)


if __name__ == '__main__':
    unittest.main()
