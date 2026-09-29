"""Real foreground owner / fresh frontend integration; no provider access."""
import json,os,pathlib,selectors,subprocess,tempfile,unittest,shutil,socket,signal,time
from journey import BIN,obligation

class RetainedSession(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.p=pathlib.Path(self.tmp.name);self.root=self.p/'repo';self.root.mkdir();(self.root/'a.py').write_text('import pathlib\n');self.contract=self.p/'contract.tsv';self.contract.write_text('UG\t1\n'+obligation(rule='fact-import',argument='import pathlib'));self.owner=None
 def tearDown(self):
  if self.owner:
   if self.owner.poll()is None:self.owner.terminate()
   self.owner.communicate(timeout=8)
  self.tmp.cleanup()
 def start(self):
  self.owner=subprocess.Popen([str(BIN),'session','--root',str(self.root),'--directory',str(self.p/'session')],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  sel=selectors.DefaultSelector();sel.register(self.owner.stdout,selectors.EVENT_READ);self.assertTrue(sel.select(8));line=self.owner.stdout.readline();self.assertTrue(line)
  self.ready=json.loads(line);self.assertEqual(self.ready.get('event'),'session-ready',self.ready);self.handle=pathlib.Path(self.ready['handle']);return self.ready
 def check(self,session=True,expected=0):
  args=[str(BIN),'check','--root',str(self.root),'--contract',str(self.contract),'--no-cache','--details']
  if session:args+=['--session',str(self.handle)]
  p=subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True);out,err=p.communicate(timeout=10);self.assertEqual(p.returncode,expected,out+err);return json.loads(out),p.pid
 def test_fresh_frontends_use_same_core_and_revalidate_changes(self):
  self.start();a,pa=self.check();b,pb=self.check();self.assertNotEqual(pa,pb);self.assertEqual(a['computation_session']['core_pid'],b['computation_session']['core_pid']);self.assertEqual(a['computation_session']['owner_pid'],self.owner.pid);self.assertEqual(b['computation_session']['frontend_pid'],pb);self.assertEqual(b['obligations'][0]['computation'],'reused');self.assertEqual(a['work']['parsed'],1);self.assertEqual(b['work']['parsed'],0);self.assertEqual(b['work']['reused_in_core'],1);self.assertEqual(a['work']['graph_invalidated'],2);self.assertEqual(b['work']['graph_invalidated'],0)
  (self.root/'a.py').write_text('# import pathlib\n');changed,_=self.check(expected=1);cold,_=self.check(session=False,expected=1);self.assertEqual(changed['obligations'][0]['state'],cold['obligations'][0]['state']);self.assertEqual(changed['obligations'][0]['computation'],'computed');self.assertEqual(changed['work']['disk_results_admitted'],0)
 def test_client_disconnect_and_extra_bytes_cancel_only_that_request(self):
  self.start();before,_=self.check();handle=json.loads(self.handle.read_text())
  def frame(body):return len(body).to_bytes(8,'big')+body
  for extra in (b'',b'extra'):
   request={'schema':'ultragoal-session-request/1','incarnation':handle['incarnation'],'workspace_identity':handle['workspace_identity'],'core_sha256':handle['core_sha256'],'instance':os.urandom(16).hex(),'sequence':0,'index':False,'frames':1,'output_limit':4096,'remaining_ms':5000}
   with socket.socket(socket.AF_UNIX) as client:
    client.connect(handle['endpoint']);client.sendall(frame(json.dumps(request,separators=(',',':')).encode())+frame(b'EXCLUSIONS\n')+extra)
   time.sleep(.1)
   if self.owner.poll() is not None:
    out,err=self.owner.communicate(timeout=8);self.fail(f'owner exited after client departure {extra!r}: {self.owner.returncode} {out} {err}')
   after,_=self.check();self.assertEqual(after['obligations'][0]['state'],'verified');self.assertIsNone(self.owner.poll());self.assertNotEqual(before['computation_session']['core_pid'],after['computation_session']['core_pid']);before=after
  self.owner.send_signal(signal.SIGTERM);out,err=self.owner.communicate(timeout=8);self.assertEqual(self.owner.returncode,130,(out,err));self.assertEqual(json.loads(out)['state'],'cancelled')
 def test_rotated_cores_keep_reuse_and_expiry_is_session_unavailable(self):
  self.start();reports=[self.check()[0] for _ in range(12)];pids=[r['computation_session']['core_pid'] for r in reports]
  self.assertGreater(len(set(pids)),1,pids);self.assertEqual(len({r['computation_session']['owner_pid'] for r in reports}),1)
  for r in reports[1:]:self.assertEqual((r['obligations'][0]['computation'],r['work']['parsed'],r['work']['reused_in_core']),('reused',0,1))
  self.owner.terminate();self.owner.communicate(timeout=8)
  for pid in set(pids):self.assertRaises(ProcessLookupError,os.kill,pid,0)
  p=subprocess.run([str(BIN),'check','--root',str(self.root),'--contract',str(self.contract),'--session',str(self.handle)],capture_output=True,text=True,timeout=10);self.assertEqual((p.returncode,p.stderr.strip()),(3,'error: session_unavailable'))
 def test_session_lifetime_is_a_bounded_runtime_input(self):
  p=subprocess.run([str(BIN),'session','--root',str(self.root),'--directory',str(self.p/'short'),'--lifetime-seconds','5'],capture_output=True,text=True,timeout=8)
  self.assertEqual((p.returncode,p.stderr.strip()),(3,'error: invalid_arguments'));self.assertFalse((self.p/'short').exists())
  self.owner=subprocess.Popen([str(BIN),'session','--root',str(self.root),'--directory',str(self.p/'session'),'--lifetime-seconds','30','--idle-seconds','5'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  ready=json.loads(self.owner.stdout.readline());self.assertEqual((ready['session']['bounds']['absolute_seconds'],ready['session']['bounds']['idle_seconds']),(30,5))
  out,_=self.owner.communicate(timeout=15);self.assertEqual(json.loads(out)['state'],'closed')
 def test_other_workspace_and_replaced_endpoint_are_refused(self):
  self.start();other=self.p/'other';other.mkdir();p=subprocess.run([str(BIN),'check','--root',str(other),'--contract',str(self.contract),'--session',str(self.handle)],capture_output=True,text=True,timeout=10);self.assertEqual(p.returncode,3);self.assertIn('workspace binding mismatch',p.stdout)
  self.owner.terminate();self.owner.communicate(timeout=8);p=subprocess.run([str(BIN),'check','--root',str(self.root),'--contract',str(self.contract),'--session',str(self.handle)],capture_output=True,text=True,timeout=10);self.assertEqual(p.returncode,3)
 def test_startup_failure_removes_only_new_owned_endpoint(self):
  isolated=self.p/'isolated';isolated.mkdir();binary=isolated/'ultragoal';shutil.copy2(BIN,binary)
  directory=self.p/'failed-start';p=subprocess.run([str(binary),'session','--root',str(self.root),'--directory',str(directory)],capture_output=True,text=True,timeout=8)
  self.assertEqual(p.returncode,3);self.assertFalse(directory.exists(),p.stdout+p.stderr)
  directory.mkdir();marker=directory/'keep';marker.write_text('existing user bytes')
  p=subprocess.run([str(BIN),'session','--root',str(self.root),'--directory',str(directory)],capture_output=True,text=True,timeout=8)
  self.assertEqual(p.returncode,3);self.assertEqual(marker.read_text(),'existing user bytes')
 def test_existing_empty_private_directory_is_usable_and_preserved(self):
  directory=self.p/'session';directory.mkdir(mode=0o700)
  self.assertTrue(directory.is_dir());self.start();self.assertTrue(directory.is_dir());self.check();self.owner.terminate();out,err=self.owner.communicate(timeout=8)
  self.assertTrue(directory.is_dir(),(out,err));self.assertEqual(list(directory.iterdir()),[])
  os.chmod(directory,0o755)
  p=subprocess.run([str(BIN),'session','--root',str(self.root),'--directory',str(directory)],capture_output=True,text=True,timeout=8)
  self.assertEqual(p.returncode,3);self.assertTrue(directory.is_dir())
 def test_cold_default_has_no_owner_or_endpoint(self):
  report,_=self.check(session=False);self.assertNotIn('computation_session',report);self.assertFalse((self.p/'session').exists())

if __name__=='__main__':unittest.main()
