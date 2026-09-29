"""Direct production invalidation module: closure, cycles, deletion and fallback."""
import os,pathlib,subprocess,unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
PROBE=pathlib.Path(os.environ.get('UG_INVALIDATION_PROBE',ROOT/'target/release/invalidation-probe'))
class Invalidation(unittest.TestCase):
 def test_reverse_closure_and_retained_cache_membership(self):
  p=subprocess.run([str(PROBE)],capture_output=True,text=True,timeout=15);self.assertEqual(p.returncode,0,p.stderr)
  rows=[r.split('\t')for r in p.stdout.splitlines()if r];dirty={r[1]:set(r[2:])for r in rows if r[0]=='DIRTY'};retained={r[1]:set(r[2:])for r in rows if r[0]=='RETAINED'}
  self.assertEqual(dirty,{'same':{'g','h'},'changed':{'a','b','d','g','h'},'cycle':{'e','f','g','h'},'deleted':{'b','d','g','h'},'fuel-fallback':set('abcdefgh')})
  self.assertEqual(retained,{'same':set('abcdef'),'changed':set('cef'),'cycle':set('abcd'),'deleted':set('cef')})
if __name__=='__main__':unittest.main()
