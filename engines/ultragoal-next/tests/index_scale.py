"""Real large-index boundary regression; no provider access."""
import hashlib,json,pathlib,subprocess,tempfile,unittest
from journey import BIN
class IndexScale(unittest.TestCase):
 def test_multiframe_index_and_selection_preserve_all_source_identities(self):
  with tempfile.TemporaryDirectory()as d:
   p=pathlib.Path(d);root=p/'repo';root.mkdir();digests={}
   for i in range(25):
    content=f'def function_{i}(): return {i}\n# '+('x'*700000)+'\n';name=f'file{i:03}.py';(root/name).write_text(content);digests[name]=hashlib.sha256(content.encode()).hexdigest()
   proc=subprocess.run([str(BIN),'index','--root',str(root),'--scope','*','--no-cache'],capture_output=True,text=True,timeout=45);self.assertEqual(proc.returncode,0,proc.stdout[-2000:]+proc.stderr);result=json.loads(proc.stdout)
   self.assertEqual(result['state'],'indexed');self.assertGreater(result['inputs']['bytes'],16*1024*1024);self.assertEqual({s['path']:s['content_sha256']for s in result['fact_sets']},digests)
   report=p/'index.json';report.write_text(proc.stdout);self.assertLess(report.stat().st_size,100000);self.assertTrue(all(s['representation']=='source-spans/2'for s in result['fact_sets']))
   selected=subprocess.run([str(BIN),'select','--root',str(root),'--report',str(report),'--query','function_0'],capture_output=True,text=True,timeout=45);self.assertEqual(selected.returncode,0,selected.stdout[-2000:]+selected.stderr);selection=json.loads(selected.stdout);self.assertTrue(selection['shortlist']);self.assertEqual(selection['coverage']['changed_paths'],[]);self.assertEqual(selection['recovery']['index_sha256'],hashlib.sha256(report.read_bytes()).hexdigest())
if __name__=='__main__':unittest.main()
