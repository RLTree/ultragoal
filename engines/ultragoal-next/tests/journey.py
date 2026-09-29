"""Real binary regressions. Temporary inputs only; no provider access."""
import hashlib,json,pathlib,random,subprocess,tempfile,time,unittest,os
HERE=pathlib.Path(__file__).resolve().parents[1]
BIN=pathlib.Path(os.environ.get('UG_BINARY',HERE/'target/release/ultragoal'))
CORE=BIN.with_name('ug-core')
def enc(s):return s.replace('%','%25').replace('\n','%0A').replace('\r','%0D').replace('\t','%09')
def row(*xs):return '\t'.join(map(enc,xs))+'\n'
def obligation(id='text',rule='contains',scope='a.py',projection='content',argument='hello',assurance='exact',revision='1'):
 return row('O',id,revision,'Keep required behavior','synthetic acceptance',scope,projection,rule,argument,assurance,'mandatory')
def core(frame):
 p=subprocess.run([str(CORE)],input=frame,text=True,capture_output=True,timeout=20)
 assert p.returncode==0,(p.returncode,p.stderr)
 return [r.split('\t') for r in p.stdout.splitlines()]
class Journey(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.p=pathlib.Path(self.tmp.name);self.root=self.p/'repo';self.root.mkdir();self.contract=self.p/'contract.tsv';self.contract.write_text('UG\t1\n'+obligation());(self.root/'a.py').write_text('hello\n');self.cache=self.p/'cache'
 def tearDown(self):self.tmp.cleanup()
 def cli(self,*extra,expected=0):
  p=subprocess.run([str(BIN),'check','--root',str(self.root),'--contract',str(self.contract),*extra],capture_output=True,text=True,timeout=30);self.assertEqual(p.returncode,expected,p.stdout+p.stderr);return json.loads(p.stdout)
 def test_pass_fail_and_unknown(self):
  self.cli();(self.root/'a.py').write_text('missing');self.cli(expected=1)
  self.contract.write_text('UG\t1\n'+obligation(rule='native',assurance='runtime'));self.cli('--local',expected=2)
 def test_requirement_revision_and_predicate_surface_are_explicit(self):
  requirement='Preserve original bytes on cancellation'
  self.contract.write_text('UG\t1\n'+row('O','trace','7',requirement,'user acceptance clause A','a.py','content','contains','hello','exact','mandatory'))
  report=self.cli();o=report['obligations'][0]
  self.assertEqual(o['requirement'],requirement);self.assertEqual(o['origin'],'user acceptance clause A');self.assertEqual(o['requirement_revision'],'7')
  self.assertEqual(o['check']['predicate_surface'],'captured text search predicate');self.assertIn('original-outcome and adoption alignment is not certified',o['assurance'])
  self.contract.write_text('UG\t1\n'+obligation(rule='native',argument='cargo test',assurance='runtime',revision='8'))
  report=self.cli('--local',expected=2);o=report['obligations'][0]
  self.assertEqual(o['requirement_revision'],'8');self.assertEqual(o['state'],'unknown');self.assertIn('currently unavailable',o['check']['predicate_surface'])
 def test_brief_report_keeps_states_and_next_steps_only(self):
  self.contract.write_text('UG\t1\n'+obligation()+obligation(id='gone',rule='absent-text',argument='hello'))
  full=self.cli(expected=1);brief=self.cli('--brief',expected=1)
  self.assertEqual([(o['id'],o['state'])for o in brief['obligations']],[(o['id'],o['state'])for o in full['obligations']])
  ok,bad=brief['obligations']
  self.assertEqual(ok,{'id':'text','state':'verified'})
  self.assertEqual(bad['counterexample'],{'path':'a.py','line':1});self.assertIn('a.py:1',bad['next_action'])
  self.assertNotIn('fact_sets',brief);self.assertLess(len(json.dumps(brief)),len(json.dumps(full))/3)
 def test_wire_escape_and_unicode(self):
  (self.root/'a.py').write_text('π\t100%\nhello');self.cli()
 def test_contract_rejects_injected_operation(self):
  with self.contract.open('a')as f:f.write('OP\tforged\trunning\tx\ttimely\n')
  self.cli(expected=3)
 def test_duplicate_missing_and_projection(self):
  self.contract.write_text('UG\t1\n'+obligation()+obligation());self.cli(expected=3)
  self.contract.write_text('UG\t1\n');self.cli(expected=3)
  self.contract.write_text('UG\t1\n'+obligation(projection='membership'));self.cli(expected=3)
 def test_corrupt_cache_recomputes(self):
  self.cli('--cache',str(self.cache));next(self.cache.glob('*.json')).write_text('{broken');self.cli('--cache',str(self.cache))
 def test_cache_identity_and_admission(self):
  self.cli('--cache',str(self.cache));b=self.cli('--cache',str(self.cache));self.assertEqual(b['obligations'][0]['cache'],'advisory-match')
  (self.root/'README.md').write_text('unrelated');c=self.cli('--cache',str(self.cache));self.assertEqual(c['obligations'][0]['cache'],'advisory-match')
  (self.root/'a.py').write_text('changed');d=self.cli('--cache',str(self.cache),expected=1);self.assertEqual(d['obligations'][0]['cache'],'miss')
 def test_unchanged_advisory_cache_does_not_publish_again(self):
  self.cli('--cache',str(self.cache));p=next(self.cache.glob('*.json'));before=p.stat();raw=p.read_bytes()
  self.cli('--cache',str(self.cache));after=p.stat();self.assertEqual(raw,p.read_bytes());self.assertEqual(before.st_ino,after.st_ino);self.assertEqual(before.st_mtime_ns,after.st_mtime_ns)
  (self.root/'a.py').write_text('changed');self.cli('--cache',str(self.cache),expected=1);self.assertNotEqual(raw,p.read_bytes())
 def test_membership_only_content_edit_reuses_add_invalidates(self):
  self.contract.write_text('UG\t1\n'+obligation(rule='member',scope='prefix:a',projection='membership'))
  self.cli('--cache',str(self.cache));(self.root/'a.py').write_text('change');self.assertEqual(self.cli('--cache',str(self.cache))['obligations'][0]['cache'],'advisory-match')
  (self.root/'another.py').write_text('new');self.assertEqual(self.cli('--cache',str(self.cache))['obligations'][0]['cache'],'miss')
 def test_unknown_dependency_and_assurance(self):
  self.contract.write_text('UG\t1\n'+obligation(projection='unknown'));self.cli(expected=2)
  self.contract.write_text('UG\t1\n'+obligation(assurance='runtime'));self.cli(expected=2)
 def test_invalid_utf8_and_symlink(self):
  (self.root/'a.py').write_bytes(b'\xff');self.cli(expected=2)
  (self.root/'a.py').unlink();(self.root/'a.py').symlink_to(self.contract);self.cli(expected=2)
 def test_selected_file_beyond_former_depth_bound(self):
  folder=self.root
  for _ in range(70):
   folder=folder/'d';folder.mkdir()
  relative=str((folder/'needed.py').relative_to(self.root))
  (folder/'needed.py').write_text('tail evidence\n')
  self.contract.write_text('UG\t1\n'+obligation(scope=relative,argument='tail evidence'))
  result=self.cli()
  self.assertTrue(result['inputs']['listing_complete'])
  self.assertEqual(result['obligations'][0]['state'],'verified')
 def test_cancel_supersede_late(self):
  for life,timeflag in [('cancelled','timely'),('superseded','timely'),('running','late')]:
   out=core('UG\t1\n'+obligation()+row('OP','op',life,'contract',timeflag)+row('F','a.py','digest','hello'))
   self.assertEqual(out[1][2],'unknown')
 def test_session_core_reads_a_frame_that_arrives_in_small_pieces(self):
  # 1.5 MiB in 200-byte writes needs more than 4,096 reads; a fixed read budget ended the core.
  content='x'*1500000+'tail-needle'
  text='UG\t1\n'+obligation(argument='tail-needle')+row('OP','op','running','contract','timely')+row('F','a.py',hashlib.sha256(content.encode()).hexdigest(),content)
  body=f'S1\t0\tONE\n{text}'.encode();data=f'{len(body)}\n'.encode()+body
  p=subprocess.Popen([str(CORE),'--threads','4','--gpu','off','--','--session'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  for i in range(0,len(data),200):
   p.stdin.write(data[i:i+200]);p.stdin.flush();time.sleep(0.0002)
  header=p.stdout.readline().decode().split('\t')
  self.assertEqual(header[:3],['S1','0','done'],p.stderr.read1(4096) if header==[''] else header)
  self.assertIn('tail-needle',p.stdout.read(int(header[3])).decode())
  p.stdin.write(b'0\n');p.stdin.close();self.assertEqual(p.wait(timeout=60),0)
 def test_provider_numerics(self):
  self.assertEqual(core('JEV\nCHOICE\tx\tsupported\t0.9\t0.9\t0.05\t0.05\n')[0][2],'advisory-supported')
  for frame in ['NOUL\tx\tNaN','NOUL\tx\t1.1','CHOICE\tx\tsupported\t1\t1\t1\t1','SCORE\tx\t2\t1\t1\t0\t0']:
   self.assertEqual(core('JEV\n'+frame+'\n')[0][2],'malformed')
  self.assertEqual(core('JEV\nSCORE\tx\t1.5\t0.8\t0\t0.5\t0.5\n')[0][2],'validated-advisory')
 def test_winner_must_hold_the_top_probability_and_ties_never_qualify(self):
  answer=lambda frame:core('JEV\n'+frame+'\n')[0][2]
  self.assertEqual(answer('RELEVANCE\tx\trelevant\t0.9\t0.5\t0.4\t0.1'),'advisory-relevant')
  self.assertEqual(answer('RELEVANCE\tx\trelevant\t0.9\t0.45\t0.45\t0.1'),'advisory-tied')
  # No near-winner allowance: 0.44999 below 0.45 is not the top answer.
  self.assertEqual(answer('RELEVANCE\tx\trelevant\t0.9\t0.44999\t0.45\t0.10001'),'malformed')
  self.assertEqual(answer('CONFLICT\tx\texplicit-conflict\t0.9\t0.4\t0.4\t0.2'),'advisory-tied')
  self.assertEqual(answer('CHOICE\tx\tsupported\t0.9\t0.4\t0.4\t0.2'),'unknown')
  self.assertEqual(answer('CHOICE\tx\tsupported\t0.9\t0.5\t0.4\t0.1'),'advisory-supported')
  decide=lambda c,s:core('SEMANTIC_CALIBRATED\n'+row('3',c,s,'','','','','','','alignment'))[0][1]
  self.assertEqual(decide('advisory-tied','advisory-complete-support'),'insufficient')
  self.assertEqual(decide('advisory-explicit-conflict','advisory-tied'),'contradicted')
 def test_choice_sums_allow_two_decimal_rounding_per_option(self):
  answer=lambda frame:core('JEV\n'+frame+'\n')[0][2]
  self.assertEqual(answer('RELEVANCE\tx\tirrelevant\t0.9\t0.01\t0.93\t0.05'),'advisory-irrelevant')
  self.assertEqual(answer('RELEVANCE\tx\tirrelevant\t0.9\t0.01\t0.92\t0.05'),'malformed')
  # Score normalization stays strict.
  self.assertEqual(core('JEV\nSCORE\tx\t1.0\t0.9\t0\t0.99\t0\t0\t0\n')[0][2],'malformed')
 def test_not_sent_attempts_retry_while_attempts_and_time_remain(self):
  retry=lambda *r:core('RETRY\n'+row(*r))[0][0]
  self.assertEqual(retry('not-sent','1','2000','running'),'RETRY')
  self.assertEqual(retry('not-sent','0','2000','running'),'STOP')
  self.assertEqual(retry('000','1','2000','running'),'STOP')
  self.assertEqual(retry('503','1','2000','running'),'RETRY')
  self.assertEqual(retry('503','1','49','running'),'STOP')
  self.assertEqual(retry('503','1','50','running'),'RETRY')
  for time_left in ('','unknown','4294967296'):
   self.assertEqual(retry('not-sent','1',time_left,'running'),'STOP',time_left)
  for ambiguous in ('timeout','connection-closed','sent-unknown'):
   self.assertEqual(retry(ambiguous,'1','2000','running'),'STOP',ambiguous)
 def test_score_accepts_two_decimal_wire_rounding_only(self):
  # Observed jev-1.13 answers whose score differs from the rounded probabilities' mean.
  for frame in ['SCORE\tx\t0.03\t0.98\t0.98\t0.02\t0\t0\t0','SCORE\tx\t0.88\t0.46\t0.38\t0.42\t0.15\t0.03\t0.02','SCORE\tx\t1.44\t0.62\t0.01\t0.56\t0.43\t0\t0','SCORE\tx\t0.05\t0.9\t1\t0\t0\t0\t0']:
   self.assertEqual(core('JEV\n'+frame+'\n')[0][2],'validated-advisory',frame)
  # Five levels allow 0.055 and three levels 0.02; normalization stays strict.
  for frame in ['SCORE\tx\t0.06\t0.9\t1\t0\t0\t0\t0','SCORE\tx\t0.03\t0.9\t1\t0\t0','SCORE\tx\t0.52\t0.9\t0.5\t0.5\t0.01\t0\t0']:
   self.assertEqual(core('JEV\n'+frame+'\n')[0][2],'malformed',frame)
 def test_read_only_preserves_dirty(self):
  before={p.name:p.read_bytes()for p in self.root.iterdir()};self.cli('--no-cache');self.assertEqual(before,{p.name:p.read_bytes()for p in self.root.iterdir()})
 def test_randomized_incremental_matches_fresh(self):
  rng=random.Random(1937)
  for i in range(100):
   target=self.root/f'f{rng.randrange(8)}.ts'
   if rng.random()<.2:target.unlink(missing_ok=True)
   else:target.write_text(rng.choice(['hello','bad','']))
   (self.root/'a.py').write_text(rng.choice(['hello','bad']))
   cmd=[str(BIN),'check','--root',str(self.root),'--contract',str(self.contract)]
   fresh=json.loads(subprocess.run(cmd+['--no-cache'],capture_output=True,text=True,timeout=30).stdout)
   cached=json.loads(subprocess.run(cmd+['--cache',str(self.cache)],capture_output=True,text=True,timeout=30).stdout)
   self.assertIn('obligations',fresh,fresh);self.assertIn('obligations',cached,cached)
   self.assertEqual(fresh['state'],cached['state'])
   self.assertEqual([{k:v for k,v in x.items()if k not in ['cache','changed_concern']}for x in fresh['obligations']],[{k:v for k,v in x.items()if k not in ['cache','changed_concern']}for x in cached['obligations']])

 def test_reviewer_exact_reproduction(self):
  (self.root/'a.py').write_text('bad');self.cli('--cache',str(self.cache),expected=1)
  (self.root/'a.py').write_text('hello');(self.root/'f2.ts').write_text('bad');(self.root/'f7.ts').write_text('')
  self.assertEqual(self.cli('--no-cache')['state'],self.cli('--cache',str(self.cache))['state'])
 def test_forged_disk_cache_cannot_verify(self):
  (self.root/'a.py').write_text('bad');self.cli('--cache',str(self.cache),expected=1)
  p=next(self.cache.glob('*.json'));v=json.loads(p.read_text());v['entries']={k:'verified'for k in v['entries']};payload=json.dumps(v['entries'],sort_keys=True,separators=(',',':')).encode();v['checksum']=hashlib.sha256(payload).hexdigest();p.write_text(json.dumps(v));self.cli('--cache',str(self.cache),expected=1)
 def test_fragmented_pipe_and_utf8(self):
  frame=('UG\t1\n'+obligation()+row('OP','op','running','contract','timely')+row('F','a.py','digest','π'*10000+'hello')).encode()
  p=subprocess.Popen([str(CORE)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
  for i in range(0,len(frame),137):p.stdin.write(frame[i:i+137]);p.stdin.flush()
  p.stdin.close();out=p.stdout.read();err=p.stderr.read();p.stdout.close();p.stderr.close();self.assertEqual(p.wait(timeout=30),0,err);self.assertIn(b'\tverified\t',out)
  for invalid in [b'\xff',b'\xc0\x80',b'\xed\xa0\x80',b'\xf4\x90\x80\x80']:
   p=subprocess.run([str(CORE)],input=invalid,capture_output=True,timeout=10);self.assertNotEqual(p.returncode,0)
 def test_large_frame_complete(self):
  (self.root/'a.py').write_text('x'*200000+'hello');self.cli()
 def test_real_fact_query(self):
  self.contract.write_text('UG\t1\n'+obligation(rule='fact-import',scope='a.py',argument='import pathlib'))
  (self.root/'a.py').write_text('import pathlib\n');self.cli();(self.root/'a.py').write_text('# import pathlib\n');self.cli(expected=1)
 def test_trusted_content_fact_reuse(self):
  self.contract.write_text('UG\t1\n'+obligation(rule='fact-import',scope='*',argument='import pathlib'))
  (self.root/'a.py').write_text('import pathlib\n');(self.root/'b.py').write_text('import pathlib\n')
  report=self.cli('--details');self.assertEqual(report['work']['parsed'],1);self.assertEqual(report['work']['reused_in_invocation'],1)
  self.assertEqual(report['fact_sets'][0]['facts'][0],['import-candidate','import pathlib','1','0','14'])
 def test_unrelated_binary_not_decoded(self):
  (self.root/'unrelated.bin').write_bytes(b'\xff'*100);self.cli()
 def test_cache_symlink_cannot_write_source(self):
  self.cache.symlink_to(self.root,target_is_directory=True);self.cli('--cache',str(self.cache),expected=3)
  self.assertEqual(sorted(p.name for p in self.root.iterdir()),['a.py'])
 def test_signal_cancellation(self):
  import signal
  source=self.p/'many.rs';source.write_text(''.join(f'fn f{i}(){{}}\n'for i in range(200000)))
  p=subprocess.Popen([str(BIN),'verify','--rust',str(source),'--events'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  self.assertEqual(json.loads(p.stderr.readline())['event'],'native-input-captured');p.send_signal(signal.SIGINT);out,err=p.communicate(timeout=20)
  self.assertEqual(p.returncode,130,out+err);self.assertEqual(json.loads(out)['state'],'cancelled')
 def test_change_and_restore_during_operation_is_not_current(self):
  content='hello'*100000;(self.root/'a.py').write_text(content)
  p=subprocess.Popen([str(BIN),'check','--root',str(self.root),'--contract',str(self.contract),'--no-cache','--events'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
  event=json.loads(p.stderr.readline());self.assertEqual(event['event'],'captured')
  (self.root/'a.py').write_text('changed');(self.root/'a.py').write_text(content)
  out,err=p.communicate(timeout=30);self.assertEqual(p.returncode,2,out+err);report=json.loads(out);self.assertFalse(report['current']);self.assertEqual(report['obligations'][0]['state'],'unknown');self.assertEqual(report['obligations'][0]['computed_state'],'verified')
if __name__=='__main__':unittest.main()
