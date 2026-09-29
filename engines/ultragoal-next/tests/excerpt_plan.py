"""Bend-owned excerpt windows preserve the established source boundary behavior."""
import unittest,random
from journey import core,row

class ExcerptPlan(unittest.TestCase):
 def plan(self,spans):
  return core('WINDOW_PLAN\n'+row('q',*[str(v)for pair in spans for v in pair]))
 def test_nested_spans_do_not_consume_slots_or_shrink_windows(self):
  self.assertEqual(self.plan([(0,20),(1,2),(2,3),(20,30)]),[['WINDOW','q','0','30']])
 def test_span_count_byte_limit_and_oversize_boundaries(self):
  self.assertEqual(self.plan([(i,i+1)for i in range(9)]),[['WINDOW','q','0','8'],['WINDOW','q','8','9']])
  self.assertEqual(self.plan([(0,4096),(4096,4097)]),[['WINDOW','q','0','4096'],['WINDOW','q','4096','4097']])
  self.assertEqual(self.plan([(0,1),(1,4098),(4098,4100)]),[['WINDOW','q','0','1'],['OVERSIZE','q','1','4098'],['WINDOW','q','4098','4100']])
  self.assertEqual(self.plan([]),[])
 def test_invalid_numbers_order_and_shapes_are_refused(self):
  for fields in [[2,1],[1,2,0,1],['x',1],[0],[0,4294967296]]:
   with self.subTest(fields=fields):self.assertTrue(any(r[0]=='ERROR'for r in core('WINDOW_PLAN\n'+row('q',*map(str,fields)))))
 def test_seeded_spans_remain_covered_or_explicitly_oversized(self):
  rng=random.Random(4096)
  for _ in range(100):
   spans=sorted(set((a:=rng.randrange(20000),a+rng.randrange(5000))for _ in range(rng.randrange(1,40))))
   out=self.plan(spans);windows=[(int(r[2]),int(r[3]))for r in out if r[0]=='WINDOW'];oversize=[(int(r[2]),int(r[3]))for r in out if r[0]=='OVERSIZE']
   self.assertTrue(all(b-a<=4096 for a,b in windows))
   self.assertEqual(oversize,[p for p in spans if p[1]-p[0]>4096])
   self.assertTrue(all((a,b)in oversize or any(x<=a and b<=y for x,y in windows)for a,b in spans))
   self.assertTrue(all(any(a==x for a,_ in spans)and any(b==y for _,b in spans)for x,y in windows))

if __name__=='__main__':unittest.main()
