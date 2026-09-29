"""A row Policy.disclosable refuses adds nothing to any count and never becomes a shortlist
group or alias: every ranking stage answers the same with and without refused rows, whether
they are refused by content or by path, alone in a file or between admitted windows."""
import json, pathlib, subprocess, tempfile, unittest, urllib.parse
from journey import BIN, CORE, row

KEY = '-----BEGIN EC PRIVATE KEY-----'
QUERY = 'How does the cache reload parser handle soft wrap tokens?'


def core(frame):
    p = subprocess.run([str(CORE)], input=frame, text=True, capture_output=True, timeout=60)
    assert p.returncode == 0, p.stderr
    return [[urllib.parse.unquote(x) for x in line.split('\t')] for line in p.stdout.splitlines()]


def c(i, path, text):
    return row('C', f'c{i}', path, 'd' * 64, '0', str(len(text.encode())), text)


ADMITTED = [('src/cache.rs', 'fn reload_cache(parser: &Parser) {\n    soft_wrap(tokens)\n}'),
            ('src/cache.rs', 'pub struct CacheParser { wrap: bool }'),
            ('docs/wrap.md', 'Soft wrap splits long lines; the cache keeps parsed tokens.'),
            ('src/parse.py', 'def parse_tokens(text):\n    return reload(text)')]
REFUSED = [('src/cache.rs', KEY + '\nfn reload_cache soft wrap parser tokens cache'),
           ('keys/.env', 'CACHE_RELOAD=soft wrap parser tokens'),
           ('secrets/credentials.json', '{"cache": "reload parser soft wrap tokens"}'),
           ('docs/wrap.md', 'token = "ghp_' + 'a' * 36 + '" soft wrap cache reload')]


def frames(with_refused):
    """Admitted rows c0..c3 in order; each refused row (c10..c13) follows one of them."""
    out = []
    for k, (path, text) in enumerate(ADMITTED):
        out.append(c(k, path, text))
        if with_refused:
            out.append(c(10 + k, *REFUSED[k]))
    return ''.join(out)


class SelectDisclosure(unittest.TestCase):
    def test_refused_rows_add_nothing_to_any_stage(self):
        for path, text in REFUSED:
            self.assertEqual(core('DISCLOSURE_CHECK\n' + row(path, text))[0][1], 'refused', path)
        for path, text in ADMITTED:
            self.assertEqual(core('DISCLOSURE_CHECK\n' + row(path, text))[0][1], 'eligible', path)
        head = row('QUERY', QUERY)
        match = {w: core('SELECT_MATCH\n' + head + frames(w)) for w in (False, True)}
        self.assertEqual(match[True], match[False])
        self.assertTrue(any(r[0] == 'FILE' and len(r) > 3 for r in match[False]))
        ranked = core('FILE_RANK\n' + head + row('SHORTLIST', '8') + ''.join(row(*r) for r in match[False]))
        self.assertEqual(ranked[0][:3], ['RANKED', '3', '3'])
        passage = 'SELECT_PASSAGE\n' + head + ''.join(row(*r) for r in ranked if r[0] == 'NORM') + ''.join(row(*r) for r in ranked if r[0] == 'IDF')
        best = {w: core(passage + frames(w)) for w in (False, True)}
        self.assertEqual(best[True], best[False])
        pool = ''.join(row(*r) for r in ranked if r[0] == 'POOL') + ''.join(row(*r) for r in best[False])
        # A refused row byte-identical to a chosen window (refused by its path) is no alias.
        twin = row('C', 'c99', 'keys/.env', 'e' * 64, '0', str(len(ADMITTED[2][1].encode())), ADMITTED[2][1])
        fused = {w: core('SELECT_FUSE\n' + row('SHORTLIST', '8') + pool + frames(False) + (twin if w else '')) for w in (False, True)}
        self.assertEqual(fused[True], fused[False])
        self.assertEqual(sum(r[0] == 'CANDIDATE' for r in fused[False]), 3)
        self.assertFalse(any('c99' in r or 'keys/.env' in r for r in fused[True]))

    def test_refused_file_changes_no_ranking_count(self):
        with tempfile.TemporaryDirectory() as d:
            results = []
            for extra in (False, True):
                root = pathlib.Path(d) / f'r{extra}' / 'repo'
                root.mkdir(parents=True)
                for k, (path, text) in enumerate(ADMITTED):
                    (root / f'f{k}_{pathlib.Path(path).name}').write_text(text + '\n')
                if extra:
                    (root / 'leak.rs').write_text(KEY + '\nfn reload_cache soft wrap parser tokens cache\n')
                v = subprocess.run([str(BIN), 'index', '--root', str(root), '--scope', '*', '--no-cache'], capture_output=True, text=True, timeout=60)
                self.assertEqual(v.returncode, 0, v.stderr)
                report = root.parent / 'index.json'
                report.write_text(v.stdout)
                s = subprocess.run([str(BIN), 'select', '--root', str(root), '--report', str(report), '--query', QUERY, '--local'], capture_output=True, text=True, timeout=60)
                self.assertEqual(s.returncode, 0, s.stderr)
                results.append(json.loads(s.stdout))
            self.assertEqual(results[0]['coverage']['ranking'], results[1]['coverage']['ranking'])
            self.assertEqual([(c['path'], c['start'], c['end']) for c in results[0]['shortlist']], [(c['path'], c['start'], c['end']) for c in results[1]['shortlist']])
            self.assertNotIn('leak.rs', json.dumps(results[1]['shortlist']))


if __name__ == '__main__':
    unittest.main()
