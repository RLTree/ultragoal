"""Bend JSON grammar against stdlib on bounded generated inputs."""
import json,random,unittest
from journey import core,row
def valid(s):return core('JSON\n'+row(s))==[['JSON','valid']]
class JsonParser(unittest.TestCase):
 def test_standard_values(self):
  for s in ['null','true','false','0','-0','1.25e-20','1E+09','[]','{}','[1,true,null,"x"]','{"a":{"b":[1,2]},"c":"π"}','"\\uD83D\\uDE00"','"\\u0000"']:
   self.assertTrue(valid(s),s)
 def test_malformed_and_duplicate(self):
  for s in ['', 'NaN','Infinity','01','-01','1.','1e','+1','--1','[1,]','{"a":1,}','{"a":1,"a":2}','{"a":1,"\\u0061":2}','{"a" 1}','{a:1}','true false','"unterminated','"\\uD800"','"\\uDC00"','"\\uD800\\u0041"','"\\x20"','"\x01"','['*65+']'*65]:
   self.assertFalse(valid(s),repr(s))
 def test_generated_supported_grammar(self):
  rng=random.Random(73)
  def value(depth):
   if depth==0:return rng.choice([None,True,False,0,-2,1.25,'π 😀 \n \t " \\'])
   choice=rng.randrange(3)
   if choice==0:return [value(depth-1)for _ in range(rng.randrange(5))]
   if choice==1:return {f'key{i}':value(depth-1)for i in range(rng.randrange(5))}
   return value(0)
  for i in range(300):
   source=json.dumps(value(4),ensure_ascii=bool(i%2),separators=(',',':'))
   self.assertTrue(valid(source),source)
if __name__=='__main__':unittest.main()
