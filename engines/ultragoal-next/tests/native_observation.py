"""Conditional Bend bindings plus actual fixed native-call/import behavior."""
import json,tempfile,pathlib,subprocess,unittest
from journey import core,row,BIN

class NativeBindings(unittest.TestCase):
 def request(self):
  r=core('VERIFIER_PLAN\n'+row('op','request','python-ast','a'*64,'b'*64,'c'*64,'4'))
  self.assertEqual(len(r),1);self.assertEqual(r[0][0],'VERIFICATION_REQUEST');return r[0]
 def observe(self,r,o):return core('VERIFIER_OBSERVE\n'+row(row(*r),row(*o)))[0]
 def test_bound_terminal_facts_and_reported_import(self):
  r=self.request();o=['LOCAL_OBSERVATION','op','request','python-ast','a'*64,'b'*64,'c'*64,'0','complete','timely','current','current-pending-call']
  self.assertEqual(self.observe(r,o),['STATE','verified','0'])
  for index,value in [(1,'other-op'),(2,'replayed-request'),(4,'d'*64),(5,'d'*64),(6,'d'*64),(7,'unavailable'),(8,'incomplete'),(9,'late'),(10,'stale'),(11,'reported-import')]:
   changed=o.copy();changed[index]=value
   with self.subTest(index=index):self.assertEqual(self.observe(r,changed),['STATE','unknown','2'])
  o[7]='1';self.assertEqual(self.observe(r,o),['STATE','failed','1'])
  changed=r.copy();changed[9]='full project runtime';self.assertEqual(self.observe(changed,o),['STATE','unknown','2'])
  changed=r.copy();changed[10]='999999';self.assertEqual(self.observe(changed,o),['STATE','unknown','2'])
 def test_plan_requires_closed_kind_identity_and_well_formed_size(self):
  for kind,size,digest in [('arbitrary-shell','4','a'*64),('python-ast','bogus','a'*64),('python-ast','-1','a'*64),('python-ast','4','claimed-hash')]:
   r=core('VERIFIER_PLAN\n'+row('op','req',kind,digest,'b'*64,'c'*64,size));self.assertEqual(r[0][0],'REFUSED')
  large=core('VERIFIER_PLAN\n'+row('op','req','python-ast','a'*64,'b'*64,'c'*64,'16777217'))
  self.assertEqual((large[0][0],large[0][8]),('VERIFICATION_REQUEST','16777217'))

class NativeCurrentCall(unittest.TestCase):
 def test_real_captured_python_then_import_is_reported_only(self):
  with tempfile.TemporaryDirectory()as tmp:
   root=pathlib.Path(tmp);source=root/'source.py';source.write_text('def value(): return 1\n')
   p=subprocess.run([str(BIN),'verify','--python',str(source)],capture_output=True,text=True,timeout=20)
   self.assertEqual(p.returncode,0,p.stdout+p.stderr);v=json.loads(p.stdout)
   self.assertEqual(v['state'],'verified');o=v['native_observation'];r=v['verification_request']
   self.assertEqual(o['origin'],'current-pending-call');self.assertEqual(o['request_id'],r['request_id']);self.assertIsNone(o['host_execution_id'])
   self.assertTrue(o['stdin_complete']);self.assertEqual(o['input_snapshot_sha256'],r['input']['sha256']);self.assertEqual(o['current_input']['state'],'current')
   self.assertGreater(o['local_child_pid'],0);self.assertIn('not protected host authentication',v['host_attestation'])
   report=root/'report.json';report.write_text(json.dumps(v));p=subprocess.run([str(BIN),'explain','--report',str(report)],capture_output=True,text=True,timeout=10)
   self.assertEqual(p.returncode,0,p.stdout+p.stderr);imported=json.loads(p.stdout);self.assertEqual(imported['state'],'reported');self.assertFalse(imported['admitted_as_current']);self.assertEqual(imported['reported_state'],'verified')
   source.write_text('def value(\n');p=subprocess.run([str(BIN),'verify','--python',str(source)],capture_output=True,text=True,timeout=20);self.assertEqual(p.returncode,1,p.stdout+p.stderr);self.assertEqual(json.loads(p.stdout)['state'],'failed')

if __name__=='__main__':unittest.main()
