import sys,pathlib,unittest
sys.path.insert(0,str(pathlib.Path(__file__).resolve().parents[1]))
from evaluate import metrics
class Metrics(unittest.TestCase):
 def test_no_automatic_is_undefined(self):
  self.assertIsNone(metrics([{'decision':'review','automatic':False}])['selective_error'])
 def test_attempts_and_missing_cost_preserved(self):
  m=metrics([{'decision':'supported','automatic':True,'correct':False,'critical_positive':True,'cost_usd':.01,'accepted':False},{'decision':'unavailable','accepted':True}]);self.assertEqual(m['attempts'],2);self.assertEqual(m['critical_misses'],1);self.assertEqual(m['selective_error'],1);self.assertIsNone(m['all_attempt_cost_per_accepted']);self.assertEqual(m['unknown_cost_attempts'],1)
if __name__=='__main__':unittest.main()
