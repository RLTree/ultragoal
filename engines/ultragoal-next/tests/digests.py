"""Bend-owned canonical identities checked against an independent byte oracle."""
import hashlib,pathlib,re,unittest
from journey import core,row,obligation
ROOT=pathlib.Path(__file__).resolve().parents[1]
class Digests(unittest.TestCase):
 def test_text_hash_vectors_and_unicode_bytes(self):
  for text in ['', 'abc','é','e\u0301','π\t100%\n','🙂'*20,'\0']:
   with self.subTest(text=text):self.assertEqual(core('HASH\n'+row(text)),[['DIGEST','sha256',hashlib.sha256(text.encode()).hexdigest()]])
 def test_computation_and_fact_identity_are_not_rehashed_by_bridge(self):
  source=(ROOT/'Identity.bend').read_text();rules=re.search(r'def rules\(\).*?"([0-9a-f]{64})"',source,re.S).group(1);parsers=re.search(r'def parsers\(\).*?"([0-9a-f]{64})"',source,re.S).group(1)
  text='hello\n';digest=hashlib.sha256(text.encode()).hexdigest();frame='UG\t1\n'+obligation()+row('OP','now','running','revision','timely')+row('F','a.py',digest,text)
  result=next(r for r in core(frame)if r[0]=='RESULT');preimage=row(rules,'1','a.py','content','contains','hello')+row('a.py',digest)
  self.assertEqual(result[6],hashlib.sha256(preimage.encode()).hexdigest())
  frame='INDEX\nUG\t1\n'+row('Q','*')+row('OP','now','running','revision','timely')+row('F','a.py',digest,text)
  fact=next(r for r in core(frame)if r[0]=='P_SPANS');preimage=row(parsers,fact[3],digest,'source-spans/2');self.assertEqual(fact[6],hashlib.sha256(preimage.encode()).hexdigest())
if __name__=='__main__':unittest.main()
