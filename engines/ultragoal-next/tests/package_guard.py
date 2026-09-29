"""Package custody failures stay local and do not replace existing artifacts."""
import hashlib,json,pathlib,shutil,subprocess,sys,tempfile,unittest
SOURCE=pathlib.Path(__file__).resolve().parents[1]/'package.py'
INVENTORY=SOURCE.with_name('source_inventory.py')
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
class PackageGuard(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.p=pathlib.Path(self.tmp.name)/'candidate';self.p.mkdir();shutil.copy2(SOURCE,self.p/'package.py');shutil.copy2(INVENTORY,self.p/'source_inventory.py');skill=self.p/'plugin/skills/harness-ultragoal';skill.mkdir(parents=True);(skill/'SKILL.md').write_text('A bounded example skill.\n');(self.p/'plugin/.codex-plugin').mkdir();(self.p/'plugin/.codex-plugin/plugin.json').write_text('{"name":"fixture","version":"0.0.1"}');release=self.p/'target/release';release.mkdir(parents=True)
  (self.p/'SEMANTIC_CONTRACT.md').write_text('Schema fixture; not a runtime claim.\n')
  for name in ['ultragoal','ug-core']:(release/name).write_bytes(name.encode())
  identity={'source':{str(p.relative_to(self.p)):digest(p)for p in self.p.rglob('*')if p.is_file()and'target'not in p.relative_to(self.p).parts},'binaries':{n:digest(release/n)for n in ['ultragoal','ug-core']}};(release/'identity.json').write_text(json.dumps(identity))
 def tearDown(self):self.tmp.cleanup()
 def run_package(self):return subprocess.run([sys.executable,str(self.p/'package.py')],capture_output=True,text=True,timeout=10)
 def test_exact_package_then_mixed_binary_rejected(self):
  good=self.run_package();self.assertEqual(good.returncode,0,good.stderr);package=pathlib.Path(good.stdout.strip());manifest=package/'MANIFEST.json';before=digest(manifest);data=json.loads(manifest.read_text());self.assertTrue(all(digest(package/p)==h for p,h in data['files'].items()))
  self.assertEqual(json.loads((package/'.codex-plugin/plugin.json').read_text())['version'],'0.0.1+'+package.parent.name)
  (self.p/'target/release/ug-core').write_bytes(b'changed');bad=self.run_package();self.assertNotEqual(bad.returncode,0);self.assertIn('binary identity mismatch',bad.stderr);self.assertEqual(digest(manifest),before)
 def test_added_source_rejected_before_output(self):
  (self.p/'new-source.txt').write_text('new');bad=self.run_package();self.assertNotEqual(bad.returncode,0);self.assertIn('source identity mismatch',bad.stderr);self.assertFalse((self.p/'target/packages').exists())
 def test_source_symlink_rejected(self):
  outside=self.p.parent/'outside';outside.write_text('synthetic');(self.p/'plugin/alias').symlink_to(outside);bad=self.run_package();self.assertNotEqual(bad.returncode,0);self.assertIn('source symlink',bad.stderr);self.assertFalse((self.p/'target/packages').exists())
if __name__=='__main__':unittest.main()
