"""Production Bend Choice schema ownership and fail-closed planning."""
import unittest
from journey import core, row

class JevLayout(unittest.TestCase):
 def plan(self, fields):
  return core('JEV_LAYOUTS\n'+row('question',*fields))
 def test_supported_layouts_have_canonical_order(self):
  layouts={
   'CHOICE':['supported','contradicted','insufficient'],
   'RELEVANCE':['relevant','irrelevant','insufficient'],
   'CONFLICT':['explicit-conflict','no-explicit-conflict','uncertain'],
   'SUPPORT':['complete-support','missing-support','uncertain']}
  for tag,names in layouts.items():
   with self.subTest(tag=tag):
    raw=[field for name in reversed(names) for field in [name,'Description']]
    self.assertEqual(self.plan(raw),[['CHOICE_LAYOUT','question',tag,*names]])
 def test_malformed_or_unrecognized_criteria_fail_closed(self):
  bad=[[],['supported','yes'],
   ['supported','a','contradicted','b','insufficient','c','extra','d'],
   ['supported','a','contradicted','b','insufficient',''],
   ['supported','a','contradicted','b','insufficient'],
   ['supported','a','supported','b','insufficient','c'],
   ['other','a','contradicted','b','insufficient','c']]
  for raw in bad:
   with self.subTest(raw=raw):self.assertEqual(self.plan(raw)[0][:2],['ERROR','question'])
 def test_multiple_questions_preserve_identity_and_order(self):
  raw=['supported','a','contradicted','b','insufficient','c']
  result=core('JEV_LAYOUTS\n'+row('first',*raw)+row('second',*raw))
  self.assertEqual([r[1] for r in result],['first','second'])

if __name__=='__main__':unittest.main()
