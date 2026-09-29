"""Read-only fit proposals applied/recovered through ordinary native git tools."""
import pathlib,tempfile,subprocess,json,unittest,os
from journey import BIN,row,obligation

class FitJourney(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.p=pathlib.Path(self.tmp.name);self.root=self.p/'repo';self.root.mkdir()
  subprocess.run(['git','init','-q',str(self.root)],check=True)
  (self.root/'tracked.txt').write_text('original\n');subprocess.run(['git','-C',str(self.root),'add','tracked.txt'],check=True);(self.root/'tracked.txt').write_text('user dirty edit\n')
  (self.root/'untracked.txt').write_text('unrelated user note\n');(self.root/'config.json').write_text('{"mode":"safe"}\n')
  self.contract=self.p/'requirements.tsv';self.text='UG\t1\n'+row('O','json','1','Configuration parses as JSON','synthetic original clause1','config.json','content','json-syntax','','exact','mandatory')+row('O','key','1','Parsed configuration includes a key named mode','synthetic original clause2','config.json','content','fact-json-key','mode','exact','mandatory');self.contract.write_text(self.text)
 def tearDown(self):self.tmp.cleanup()
 def cli(self,*args,code=0):
  p=subprocess.run([str(BIN),*args],capture_output=True,text=True,timeout=30);self.assertEqual(p.returncode,code,p.stdout+p.stderr);return json.loads(p.stdout)
 def fit(self):return self.cli('fit','--root',str(self.root),'--contract',str(self.contract))
 def apply(self,patch,valid=True):
  file=self.p/'patch.diff';file.write_text(patch);check=subprocess.run(['git','-C',str(self.root),'apply','--check',str(file)],capture_output=True,text=True)
  if not valid:self.assertNotEqual(check.returncode,0,check.stdout+check.stderr);return
  self.assertEqual(check.returncode,0,check.stderr);subprocess.run(['git','-C',str(self.root),'apply',str(file)],check=True)
 def preserved(self):
  self.assertEqual((self.root/'tracked.txt').read_text(),'user dirty edit\n');self.assertEqual((self.root/'untracked.txt').read_text(),'unrelated user note\n')
 def test_propose_host_apply_failure_recovery_and_inverse(self):
  plan=self.fit();self.assertEqual(plan['state'],'proposal');self.assertFalse((self.root/'.ultragoal.tsv').exists());self.preserved()
  self.apply(plan['patch']);self.assertEqual((self.root/'.ultragoal.tsv').read_text(),self.text)
  report=self.cli('check','--root',str(self.root),'--contract',str(self.root/'.ultragoal.tsv'));self.assertEqual(report['state'],'verified')
  (self.root/'config.json').write_text('{"other":1}\n');bad=self.cli('check','--root',str(self.root),'--contract',str(self.root/'.ultragoal.tsv'),code=1)
  path=self.p/'failure.json';path.write_text(json.dumps(bad));explain=self.cli('explain','--report',str(path));self.assertFalse(explain['admitted_as_current']);self.assertTrue(any(o['state']=='failed'and "required key 'mode'"in o['next_action']for o in explain['obligations']))
  (self.root/'config.json').write_text('{"mode":"safe"}\n');self.cli('check','--root',str(self.root),'--contract',str(self.root/'.ultragoal.tsv'));self.preserved()
  self.apply(plan['rollback_patch']);self.assertFalse((self.root/'.ultragoal.tsv').exists());self.preserved();self.assertIn('unverified',plan['adoption'])
 def test_inverse_patch_cannot_erase_subsequent_user_edit(self):
  plan=self.fit();self.apply(plan['patch']);target=self.root/'.ultragoal.tsv';target.write_text(target.read_text()+'USER-ADDED-DATA\n');before=target.read_bytes();self.apply(plan['rollback_patch'],False);self.assertEqual(target.read_bytes(),before);self.preserved()
 def test_retrofit_preserves_requirements_and_refuses_stale_patch(self):
  target=self.root/'.ultragoal.tsv';target.write_text(self.text);self.contract.write_text(self.text+obligation('new','contains','config.json','content','safe',revision='1'))
  plan=self.fit();self.assertEqual(plan['state'],'proposal');target.write_text(self.text+'USER-CHANGED-BASELINE\n');before=target.read_bytes();self.apply(plan['patch'],False);self.assertEqual(target.read_bytes(),before)
 def test_removal_or_revision_change_is_not_silently_adopted(self):
  (self.root/'.ultragoal.tsv').write_text(self.text);self.contract.write_text('UG\t1\n'+obligation('json','json-syntax','config.json','content','',revision='2'))
  plan=self.fit();self.assertEqual(plan['state'],'requirement-change');self.assertIsNone(plan['patch']);self.assertEqual((self.root/'.ultragoal.tsv').read_text(),self.text)
 def test_symlink_and_dangling_target_are_never_followed(self):
  external=self.p/'outside';external.write_text('private external content\n');target=self.root/'.ultragoal.tsv';target.symlink_to(external)
  plan=self.fit();self.assertEqual(plan['state'],'unavailable-target');self.assertIsNone(plan['patch']);self.assertEqual(external.read_text(),'private external content\n')
  target.unlink();target.symlink_to(self.p/'absent');plan=self.fit();self.assertEqual(plan['state'],'unavailable-target');self.assertIsNone(plan['patch']);self.preserved()
 def test_supplied_missing_final_newline_is_preserved(self):
  text=self.text.rstrip('\n');self.contract.write_text(text);plan=self.fit();self.apply(plan['patch']);self.assertEqual((self.root/'.ultragoal.tsv').read_bytes(),text.encode());self.apply(plan['rollback_patch']);self.assertFalse((self.root/'.ultragoal.tsv').exists())
 def test_large_manifest_does_not_hit_a_fixed_fit_file_ceiling(self):
  manifest=self.root/'Cargo.toml';manifest.write_text('[package]\nname = "example"\nversion = "0.1.0"\n' + '# measured fixture\n'*70000)
  self.assertGreater(manifest.stat().st_size,1024*1024)
  before=manifest.read_bytes();plan=self.cli('fit','--root',str(self.root))
  self.assertEqual(plan['state'],'proposal');self.assertIn('Cargo.toml',plan['proposed_contract'])
  self.assertEqual(next(o['state']for o in plan['observations']if o['path']=='Cargo.toml'),'captured')
  self.assertEqual(manifest.read_bytes(),before);self.preserved()

if __name__=='__main__':unittest.main()
