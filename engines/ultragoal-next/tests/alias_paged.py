"""Paged Bend alias membership equals the original fusion over all C rows."""
import os
import pathlib
import subprocess
import unittest
from segment_wire import row

ROOT = pathlib.Path(__file__).resolve().parents[1]
CORE = pathlib.Path(os.environ.get('UG_SEGMENT_CORE', ROOT / 'target/release/ug-core'))

def core(body):
    result = subprocess.run([str(CORE)], input=body, text=True, capture_output=True, check=True)
    return result.stdout.splitlines()

class AliasPages(unittest.TestCase):
    def test_cross_page_aliases_match_whole_fuse_with_grammar(self):
        best = row('C','p','a.md','d',0,6,'needle')
        aliases = [row('C','q','b.md','d',0,6,'needle'), row('C','r','c.py','d',0,6,'needle'), row('C','s','d.md','d',0,6,'needle')]
        whole = core('SELECT_FUSE\n'+row('SHORTLIST',1)+row('POOL','a.md',1,50)+row('BEST','a.md','p',42)+best+''.join(aliases))
        expected = [line for line in whole if line.startswith('ALIAS\t')]
        primary = row('PRIMARY','p','a.md','d',0,6,'needle')
        paged = [line for part in aliases for line in core('SELECT_ALIASES\n'+primary+part)]
        self.assertEqual(paged, expected)
        self.assertEqual([line.split('\t')[2] for line in paged], ['q','s'])

    def test_malformed_candidate_refuses(self):
        self.assertEqual(core('SELECT_ALIASES\n'+row('PRIMARY','p','a.md','d',0,6,'needle')+'C\tbroken\n'), ['ERROR\tmalformed alias page'])

if __name__ == '__main__': unittest.main()
