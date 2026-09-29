import json,os,pathlib,subprocess,tempfile,unittest
from journey import BIN,core,row,obligation
def stages(query,candidates,shortlist='128'):
 """The four select stages (Retrieval.bend) over (id, path, text) rows, one frame each, as src/context.rs runs them."""
 rows=''.join(row('C',i,p,'d','0',str(len(t.encode())),t)for i,p,t in candidates);head=row('QUERY',query)
 ranked=core('FILE_RANK\n'+head+row('SHORTLIST',shortlist)+''.join(row(*r)for r in core('SELECT_MATCH\n'+head+rows)))
 best=core('SELECT_PASSAGE\n'+head+''.join(row(*r)for tag in('NORM','IDF')for r in ranked if r[0]==tag)+rows)
 return core('SELECT_FUSE\n'+row('SHORTLIST',shortlist)+''.join(row(*r)for r in ranked if r[0]=='POOL')+''.join(row(*r)for r in best)+rows)
class ContextJourney(unittest.TestCase):
 def test_forced_rank_pages_match_single_frame_selection(self):
  for i in range(12):(self.root/f'case{i:02}.py').write_text(f'def alpha_{i}(): return alpha\n')
  path=self.index();args=[str(BIN),'select','--root',str(self.root),'--report',str(path),'--query','alpha','--details']
  def run(env):
   p=subprocess.run(args,env=env,text=True,capture_output=True,timeout=60);self.assertEqual(p.returncode,0,p.stdout+p.stderr);return json.loads(p.stdout)
  old=run(os.environ.copy());paged=run({**os.environ,'UG_SELECT_RANK_PAGE_BYTES':'1024'})
  self.assertEqual(paged['shortlist'],old['shortlist'])
  self.assertEqual(paged['coverage'],old['coverage'])
  self.assertEqual(paged['candidate_provenance'],old['candidate_provenance'])
 def test_forced_plan_pages_preserve_broad_scope_reads(self):
  for i in range(80):(self.root/f'case{i:02}.py').write_text('alpha marker\n')
  contract=self.p/'broad.tsv';contract.write_text('UG\t1\n'+obligation(scope='prefix:case',argument='alpha marker'))
  args=[str(BIN),'check','--root',str(self.root),'--contract',str(contract)]
  def run(env):
   p=subprocess.run(args,env=env,text=True,capture_output=True,timeout=60);self.assertEqual(p.returncode,0,p.stdout+p.stderr);return json.loads(p.stdout)
  normal=run(os.environ.copy());paged=run({**os.environ,'UG_PLAN_PAGE_BYTES':'1024','UG_CHECK_PAGE_BYTES':'1024'})
  self.assertEqual([(o['id'],o['state'])for o in paged['obligations']],[(o['id'],o['state'])for o in normal['obligations']])
  self.assertEqual(paged['inputs']['problems'],normal['inputs']['problems'])
 def test_nested_source_spans_cannot_shrink_the_excerpt(self):
  text='def selected_target(): return final_token\n';(self.root/'writer.py').write_text(text);path=self.index();report=json.loads(path.read_text());item=next(s for s in report['fact_sets']if s['path']=='writer.py');item['facts'].append(['structure-function-name',None,'1','4','8']);path.write_text(json.dumps(report));result=self.run_cli('select','--root',str(self.root),'--report',str(path),'--query','final_token','--details');self.assertTrue(any('final_token' in c.get('excerpt','')for c in result['shortlist']),result)
 def test_advice_plan_and_disclosure_require_explicit_known_scope(self):
  rows=core('ADVICE_PLAN\n'+row('test-choice','10'));self.assertEqual(len(rows),10);self.assertTrue(all(r[0]=='ADVICE_QUESTION'and len(r)==4 and r[2]=='noul' for r in rows));self.assertEqual(rows[-1][1],'item9')
  self.assertEqual(core('ADVICE_PLAN\n'+row('execute-arbitrary-command','1'))[0][0],'ERROR')
  self.assertEqual(len(core('ADVICE_PLAN\n'+row('test-choice','256'))),256)
  self.assertEqual(core('ADVICE_PLAN\n'+row('test-choice','257'))[0][0],'ERROR')
  offset=core('ADVICE_PLAN\n'+row('test-choice','2','256'));self.assertEqual([r[1]for r in offset],['item256','item257'])
  for disclosure in ['synthetic','public','project']:
   self.assertEqual(core('ADMIT_REQUEST\n'+row('jev-1.13.0',disclosure,'1000','900','8','running','3000'))[0][0],'ADMITTED')
  self.assertEqual(core('ADMIT_REQUEST\n'+row('jev-1.13.0','disabled','1000','900','8','running','3000'))[0][0],'REFUSED')
  self.assertEqual(core('ADMIT_REQUEST\n'+row('jev-1.13.0','synthetic','64000','32000','8','running','3000'))[0][0],'ADMITTED')
  self.assertEqual(core('ADMIT_REQUEST\n'+row('jev-1.13.0','synthetic','64001','900','8','running','3000'))[0][0],'REFUSED')
  self.assertEqual(core('ADMIT_REQUEST\n'+row('jev-1.13.0','synthetic','1000','32001','8','running','3000'))[0][0],'REFUSED')
  ranked=core('ADVICE_RANK\n'+row('0','null')+row('1','0.8')+row('2','0.8')+row('3','0.1'));self.assertEqual(ranked,[['RANK','1'],['RANK','2'],['RANK','3'],['RANK','0']])
 def test_credential_names_and_content_are_screened_before_disclosure(self):
  check=lambda path,text:core('DISCLOSURE_CHECK\n'+row(path,text))[0][1]
  for path,text in [('a/.npmrc','registry=x'),('kubeconfig','x'),('prod.tfstate','{}'),('keys/service-account-1.json','{}'),('.env','X=1'),('certs/server.pem','x'),('.codex/notes.md','x'),
                    ('src/app.py','token = "ghp_'+'a'*36+'"'),('src/app.py','-----BEGIN EC PRIVATE KEY-----'),('sa.json','{"private_key": "k", "client_email": "e"}'),('.env.example','API_KEY=realvalue123')]:
   self.assertEqual(check(path,text),'refused',path)
  for path,text in [('src/app.py','def f(): return 1'),('.env.example','API_KEY=changeme'),('docs/auth.md','Set the password in the vault.')]:
   self.assertEqual(check(path,text),'eligible',path)
 def test_whole_request_body_is_screened_for_credential_shapes(self):
  screen=lambda body:core('SECRET_SCREEN\n'+row(body))[0]
  self.assertEqual(screen('{"state":{"excerpt":"def f(): return 1"},"questions":{}}'),['SCREEN','clear'])
  for secret in ['AKIA'+'B'*16,'xoxb-'+'1'*12,'sk_live_'+'a'*20,'Bearer eyJ'+'a'*9+'.'+'b'*9+'.'+'c'*9,'password: '+'x'*14]:
   self.assertEqual(screen('{"state":{"excerpt":"'+secret+'"},"questions":{}}'),['SCREEN','refused'],secret)
 def test_window_judges_each_item_once_within_both_limits(self):
  items=''.join(row('ITEM',f'i{k}','1000','500')for k in range(20))+row('ITEM','big','40000','10')
  plan=core('JEV_WINDOW\n'+row('FIXED','2000','1000')+items)
  self.assertEqual(plan[0],['WINDOW','8','64000','32000'])
  requests=[r for r in plan if r[0]=='REQUEST']
  self.assertEqual([r[:3]for r in requests],[['REQUEST','0','head'],['REQUEST','1','slice'],['REQUEST','2','slice']])
  self.assertTrue(all(len(r)-3<=8 for r in requests))
  self.assertEqual(sorted(i for r in requests for i in r[3:]),sorted(f'i{k}'for k in range(20)))
  self.assertEqual([r for r in plan if r[0]=='OMITTED'],[['OMITTED','big']])
  # The head stays within half of the state-plus-question limit: 1000 + 3*5000 fits 16000, a fourth does not.
  head=core('JEV_WINDOW\n'+row('FIXED','2000','1000')+''.join(row('ITEM',f'w{k}','4990','10')for k in range(5)))
  self.assertEqual(head[1],['REQUEST','0','head','w0','w1','w2'])
 def test_window_merge_keeps_first_judgment_and_names_uninspected(self):
  items=''.join(row('ITEM',f'i{k}','1000','500')for k in range(20))+row('ITEM','big','40000','10')
  merged=core('JEV_WINDOW_MERGE\n'+row('FIXED','2000','1000')+items+row('RESULT','0','i0','advisory-relevant')+row('RESULT','0','i0','advisory-irrelevant')+row('RESULT','0','i9','advisory-relevant')+row('RESULT','1','i9','advisory-irrelevant'))
  by={r[1]:r for r in merged}
  self.assertEqual(by['i0'],['JUDGMENT','i0','advisory-relevant','0'])
  self.assertEqual(by['i9'],['JUDGMENT','i9','advisory-irrelevant','1'])
  self.assertEqual(by['i1'],['UNINSPECTED','i1','no-accepted-result','0'])
  self.assertEqual(by['big'],['UNINSPECTED','big','omitted-by-window'])
  self.assertEqual(len(merged),21)
 def test_advice_order_ranks_by_shared_words_and_keeps_ties_in_order(self):
  order=core('ADVICE_ORDER\n'+row('cache reload after rename')+row('0','pip freeze')+row('1','send reload signal')+row('2','watch rename events for reload')+row('3','check disk'))
  self.assertEqual(order,[['ORDER','2','2'],['ORDER','1','1'],['ORDER','0','0'],['ORDER','3','0']])
 def test_advice_unavailable_preserves_all_candidates_without_actions(self):
  request=self.p/'advice.json';items=[{'id':f't{i}','description':'A proposed test'}for i in range(10)];request.write_text(json.dumps({'purpose':'test-choice','question':'Which test distinguishes these hypotheses?','observations':['No run has been performed.'],'candidates':items}))
  result=self.run_cli('advise','--request',str(request),code=2);self.assertEqual([r['candidate']for r in result['ranked_candidates']],items);self.assertTrue(all(r['utility_score']is None for r in result['ranked_candidates']));self.assertEqual(sum(len(a['attempts'])for a in result['assessments']),0)
 def test_advice_over_one_plan_batch_keeps_every_candidate(self):
  request=self.p/'advice-large.json';items=[{'id':f't{i}','description':'A proposed test'}for i in range(257)]
  request.write_text(json.dumps({'purpose':'test-choice','question':'Which test distinguishes these hypotheses?','observations':['No run has been performed.'],'candidates':items}))
  result=self.run_cli('advise','--request',str(request),code=2)
  self.assertEqual({r['candidate']['id']for r in result['ranked_candidates']},{c['id']for c in items})
 def test_forced_advice_pages_preserve_order_and_partial_coverage(self):
  request=self.p/'advice-pages.json';items=[{'id':f't{i}','description':('reload rename 'if i%2 else 'disk probe ')+str(i)}for i in range(35)]
  request.write_text(json.dumps({'purpose':'test-choice','question':'Which test distinguishes a reload failure?','observations':['Rename happened.'],'candidates':items}))
  def run(env):
   p=subprocess.run([str(BIN),'advise','--request',str(request)],env=env,text=True,capture_output=True,timeout=60)
   self.assertEqual(p.returncode,2,p.stdout+p.stderr)
   return json.loads(p.stdout)
  normal=run(os.environ.copy());paged=run({**os.environ,'UG_ADVICE_PAGE_BYTES':'1024'})
  self.assertEqual([(x['index'],x['candidate'],x['status'])for x in paged['ranked_candidates']],[(x['index'],x['candidate'],x['status'])for x in normal['ranked_candidates']])
  self.assertEqual(len(paged['ranked_candidates']),len(items))
 @unittest.skipUnless(os.environ.get('UG_ADVICE_LARGE')=='1','focused former request-file boundary')
 def test_advice_request_past_former_file_bound_keeps_every_candidate(self):
  request=self.p/'large-advice.json';items=[{'id':f't{i}','description':'x'*70000}for i in range(257)]
  request.write_text(json.dumps({'purpose':'test-choice','question':'Which probe helps?','observations':['No run yet.'],'candidates':items}))
  self.assertGreater(request.stat().st_size,16*1024*1024)
  p=subprocess.run([str(BIN),'advise','--request',str(request)],capture_output=True,text=True,timeout=300)
  self.assertEqual(p.returncode,2,p.stdout[-1000:]+p.stderr)
  report=json.loads(p.stdout)
  self.assertEqual({r['candidate']['id']for r in report['ranked_candidates']},{c['id']for c in items})
  self.assertTrue(all(r['status']=='uninspected'for r in report['ranked_candidates']))
 def test_relative_advice_path_cannot_hide_private_directory(self):
  private=self.p/'.codex';private.mkdir();request=private/'request.json'
  request.write_text(json.dumps({'purpose':'test-choice','question':'Which test?','observations':[],'candidates':[{'id':'a','description':'Run a test'}]}))
  p=subprocess.run([str(BIN),'advise','--request','.codex/request.json'],cwd=self.p,capture_output=True,text=True,timeout=30)
  self.assertEqual(p.returncode,3,p.stdout+p.stderr)
  self.assertEqual(json.loads(p.stdout)['error_code'],'disclosure_refused')
 def test_conflicting_parse_identity_cannot_reuse_different_bytes(self):
  frame='INDEX\nUG\t1\n'+row('Q','*')+row('OP','now','running','revision','timely')+row('F','a.py','same','def first(): pass\n')+row('F','b.py','same','def second(): pass\n')
  self.assertEqual(core(frame),[['ERROR','conflicting bytes for one declared parse identity']])
 def test_exact_read_plan_and_prefix_fallback_are_bend_owned(self):
  exact=core('CONTRACT_PLAN\nUG\t1\n'+obligation(scope='file:writer.py'));self.assertEqual(exact,[['VALID'],['READ_IF_FILE','writer.py']])
  prefix=core('CONTRACT_PLAN\nUG\t1\n'+obligation(scope='prefix:src/'));self.assertEqual(prefix,[['VALID'],['PLAN_NEEDS_MEMBERSHIP']])
  membership=core('CONTRACT_PLAN\nUG\t1\n'+obligation(rule='member',scope='*',projection='membership'));self.assertEqual(membership,[['VALID']])
  for rule in ['native','semantic']:
   self.assertEqual(core('CONTRACT_PLAN\nUG\t1\n'+obligation(rule=rule,projection='membership'))[0][0],'ERROR')
 def test_conditional_exact_read_does_not_poison_directory_membership(self):
  (self.root/'folder').mkdir();c=self.p/'contract';c.write_text('UG\t1\n'+obligation(id='directory',rule='path-member',scope='folder',projection='membership')+obligation(id='content',scope='folder'))
  result=self.run_cli('check','--root',str(self.root),'--contract',str(c),code=1);self.assertEqual(result['obligations'][0]['state'],'verified');self.assertEqual(result['obligations'][1]['state'],'failed')
 def test_compact_span_rejects_non_boundary_unicode_offset(self):
  (self.root/'writer.py').write_text('π = 1\n');path=self.index();index=json.loads(path.read_text());item=next(s for s in index['fact_sets']if s['path']=='writer.py');self.assertEqual(item['line_runs'],[[1,0,6,0]]);item['line_runs'][0]=[1,1,5,0];path.write_text(json.dumps(index));result=self.run_cli('select','--root',str(self.root),'--report',str(path),'--query','π');self.assertTrue(any(x['path']=='writer.py'for x in result['coverage']['unsupported']))
 def test_oversized_line_preserves_adjacent_source_evidence(self):
  (self.root/'writer.py').write_text('def save(): return 42\n# '+('x'*10000)+'\n')
  idx=self.index();result=self.run_cli('select','--root',str(self.root),'--report',str(idx),'--query','save')
  self.assertTrue(any(c['path']=='writer.py'and 'def save'in c['excerpt']for c in result['shortlist']));self.assertTrue(any(c['path']=='writer.py'for c in result['coverage']['unsupported']))
 def test_offline_response_validation_needs_no_credentials(self):
  request=self.p/'request.json';response=self.p/'response.json'
  request.write_text(json.dumps({'model':'jev-1.13.0','questions':{'conflict':{'type':'choice','instructions':'probe','criteria':{'explicit-conflict':'E','no-explicit-conflict':'N','uncertain':'U'}}},'state':{}}))
  response.write_text(json.dumps({'model':'jev-1.13.0','answers':{'conflict':{'type':'choice','choice':'no-explicit-conflict','confidence':1,'probabilities':{'explicit-conflict':0,'no-explicit-conflict':1,'uncertain':0}}},'usage':None}))
  result=self.run_cli('semantic','--request',str(request),'--response',str(response),code=2);self.assertEqual(result['state'],'reported-advisory');self.assertEqual(result['attempts'],[]);self.assertEqual(result['validation'],[['ANSWER','conflict','advisory-no-explicit-conflict']])
 def test_probe_distribution_validation_rejects_mismatch(self):
  good=core('JEV\n'+row('CONFLICT','q','explicit-conflict','1','1','0','0'));self.assertEqual(good,[['ANSWER','q','advisory-explicit-conflict']])
  bad=core('JEV\n'+row('CONFLICT','q','explicit-conflict','1','0','1','0'));self.assertEqual(bad,[['ANSWER','q','malformed']])
  bad=core('JEV\n'+row('SUPPORT','q','explicit-conflict','1','1','0','0'));self.assertEqual(bad,[['ANSWER','q','malformed']])
 def test_conflict_combination_preserves_ambiguity_and_missing_probes(self):
  def decide(a,b):return core('SEMANTIC_CALIBRATED\n'+row('4',a,b,'','','','','','','alignment'))[0][1]
  self.assertEqual(decide('advisory-explicit-conflict','advisory-missing-support'),'contradicted')
  self.assertEqual(decide('advisory-uncertain','advisory-complete-support'),'insufficient')
  self.assertEqual(decide('advisory-no-explicit-conflict','advisory-complete-support'),'supported')
  self.assertEqual(decide('advisory-explicit-conflict','malformed'),'unavailable')
  self.assertEqual(decide('malformed','advisory-complete-support'),'unavailable')
 def test_every_semantic_obligation_gets_a_request_unless_a_budget_is_declared(self):
  frame='UG\t1\n'+''.join(obligation(id=str(i),rule='semantic',scope='case.txt',argument='alignment',assurance='semantic')for i in range(6))+row('OP','now','running','revision','timely')+row('F','case.txt','digest','content')
  records=core(frame);self.assertEqual(sum(r[0]=='REQUEST'for r in records),6);self.assertEqual(sum(r[0]=='SEMANTIC_UNAVAILABLE'for r in records),0)
  records=core(frame+row('SB','2'));self.assertEqual(sum(r[0]=='REQUEST'for r in records),2)
  self.assertEqual([r[2] for r in records if r[0]=='SEMANTIC_UNAVAILABLE'],['Operation question budget exhausted; obligation remains unresolved.']*4)
  for bad in ['0','4097','x']:
   self.assertFalse(any(r[0]=='REQUEST' for r in core(frame+row('SB',bad))),bad)
 def test_declared_semantic_context_rejects_wrong_target_and_role(self):
  value={'schema':'ultragoal-semantic-context/2','target':'verification','scope':'one bounded run','clauses':[{'id':'c1','statement':'Check error preservation','required_roles':['asserted-test','executed-observation']}],'evidence':[{'id':'e1','role':'asserted-test','content':'assert error == PermissionDenied','provenance':'synthetic fixture','clause_ids':['c1']}]}
  def validate(v,family='verification'):return core('SEMANTIC_CONTEXT\n'+row(family,json.dumps(v)))
  self.assertEqual(validate(value),[['CONTEXT','valid']]);self.assertEqual(validate(value,'alignment'),[['CONTEXT','invalid']])
  value['evidence'][0]['role']='authenticated-host-authority';self.assertEqual(validate(value),[['CONTEXT','invalid']])
  value['evidence'][0]['role']='asserted-test';value['clauses']=[];self.assertEqual(validate(value),[['CONTEXT','invalid']])
 def test_semantic_endpoint_contract_is_versioned_and_distinct(self):
  prompts={family:core('SEMANTIC_RUBRIC\n'+row(family))[0]for family in ['alignment','verification','consistency','recovery-proposal','recovery-state','investigation-reuse']}
  self.assertTrue(all(r[2]=='4'and len(r)==10 for r in prompts.values()))
  self.assertIn('SUPPORTS verification',prompts['verification'][3]);self.assertIn('PROPOSED NEXT ACTION',prompts['recovery-proposal'][3]);self.assertIn('ACHIEVED RECOVERY',prompts['recovery-state'][3]);self.assertIn('CURRENT INVESTIGATION REUSE',prompts['investigation-reuse'][3])
  legacy='UG\t1\n'+obligation(rule='semantic',scope='case.txt',argument='recovery',assurance='semantic')+row('OP','now','running','revision','timely')+row('F','case.txt','digest','proposed action')
  records=core(legacy);self.assertFalse(any(r[0]=='REQUEST'for r in records));self.assertTrue(any(r[0]=='SEMANTIC_UNAVAILABLE'and 'migration'in r[2]for r in records))
 def test_implementation_test_and_dependency_survive_keyword_log_crowding(self):
  candidates=[('implementation','src/save.py','def save(x):\n    return persist(x)'),('test','checks/save.py','assert save(old) == old'),('dependency','src/caller.py','from save import save')]+[(f'log{i}',f'logs/{i}.md','cancellation original bytes save persist requirement query')for i in range(12)]
  rows=stages('cancellation original bytes save persist requirement query',candidates);ids=[r[1]for r in rows if r[0]=='CANDIDATE']
  self.assertTrue({'implementation','test','dependency'}.issubset(ids));self.assertEqual(len(ids),4);self.assertEqual(len(set(ids)),4);self.assertEqual(sum(r[0]=='ALIAS'for r in rows),11)
 def test_equal_scores_keep_path_order_up_to_the_shortlist(self):
  rows=stages('needle',[(str(i),f'{i}.md',f'needle{i}')for i in range(10)])
  self.assertEqual([r[1]for r in rows if r[0]=='CANDIDATE'],[str(i)for i in range(10)])
  candidates=[(str(i),f'{i}.md',f'needle{i:02}')for i in range(70)];first=sorted(candidates,key=lambda c:c[1].encode())[:64]
  self.assertEqual([r[1]for r in stages('needle',candidates,'64')if r[0]=='CANDIDATE'],[c[0]for c in first])
 def test_duplicate_excerpt_groups_keep_every_original_span_and_anchor(self):
  for i in range(24):(self.root/f'log{i:02}.md').write_text('cancellation original bytes save persist requirement query\n')
  idx=self.index();result=self.run_cli('select','--root',str(self.root),'--report',str(idx),'--query','cancellation original bytes save persist requirement query','--details')
  groups=[c for c in result['shortlist']if c['path'].startswith('log')]
  self.assertEqual(len(groups),1);group=groups[0];self.assertEqual(len(group['aliases']),23)
  self.assertEqual(group['assessment_provenance']['shown_aliases'],16);self.assertEqual(group['assessment_provenance']['unshown_aliases'],7)
  paths={group['path']}|{a['path']for a in group['aliases']};self.assertEqual(paths,{f'log{i:02}.md'for i in range(24)})
  self.assertTrue(any(a.get('path')=='AGENTS.md'for a in result['anchors']))
  self.assertEqual(result['measurements']['duplicate_candidates_represented_as_aliases'],23)
  self.assertTrue(all(p['in_shortlist']and p['represented_by']==group['id']for p in result['candidate_provenance']if p['path'].startswith('log')))
 def test_alias_pages_keep_complete_grammar_sensitive_membership(self):
  text='cancellation original bytes save persist requirement query\n'
  for i in range(80):(self.root/f'log{i:03}.md').write_text(text)
  (self.root/'same.py').write_text(text)
  path=self.index();args=[str(BIN),'select','--root',str(self.root),'--report',str(path),'--query','cancellation original bytes save persist requirement query','--details']
  def run(env):
   p=subprocess.run(args,env=env,text=True,capture_output=True,timeout=60);self.assertEqual(p.returncode,0,p.stdout+p.stderr);return json.loads(p.stdout)
  normal=run(os.environ.copy());paged=run({**os.environ,'UG_SELECT_CHUNK_BYTES':'1024','UG_SELECT_RANK_PAGE_BYTES':'1024'})
  self.assertEqual(paged['shortlist'],normal['shortlist'])
  self.assertEqual(paged['candidate_provenance'],normal['candidate_provenance'])
  md=[c for c in paged['shortlist']if c['path'].startswith('log')]
  self.assertEqual(len(md),1)
  self.assertEqual(len(md[0]['aliases']),79)
  self.assertTrue(any(c['path']=='same.py'for c in paged['shortlist']))
 def test_unshown_alias_change_invalidates_prior_prepared_evidence(self):
  for i in range(24):(self.root/f'log{i:02}.md').write_text('shared cancellation bytes\n')
  idx=self.index();a=self.run_cli('select','--root',str(self.root),'--report',str(idx),'--query','cancellation','--details');prior=self.p/'prior.json';prior.write_text(json.dumps(a))
  (self.root/'log23.md').rename(self.root/'log99.md');idx=self.index()
  b=self.run_cli('select','--root',str(self.root),'--report',str(idx),'--query','cancellation','--details','--prior',str(prior))
  self.assertEqual(a['request_sha256'],b['request_sha256'])
  self.assertNotEqual(a['prepared_evidence_sha256'],b['prepared_evidence_sha256'])
  self.assertFalse(b['reuse']['same_prior_question_and_prepared_evidence']);self.assertIsNone(b['assessment'])
 def test_duplicate_grouping_preserves_grammar_and_exact_unicode(self):
  candidates=[('a','a.md','needle é'),('b','b.md','needle e\u0301'),('c','c.py','needle é'),('d','d.md','needle é')]
  rows=stages('needle',candidates);self.assertEqual(sum(r[0]=='CANDIDATE'for r in rows),3)
  self.assertEqual([(r[1],r[2])for r in rows if r[0]=='ALIAS'],[('a','d')])
 def test_duplicate_group_representative_uses_highest_path_score(self):
  rows=stages('special needle',[('first','a.md','needle'),('best','special.md','needle')])
  self.assertEqual([r[1]for r in rows if r[0]=='CANDIDATE'],['best']);self.assertEqual([(r[1],r[2])for r in rows if r[0]=='ALIAS'],[('best','first')])
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.p=pathlib.Path(self.tmp.name);self.root=self.p/'repo';self.root.mkdir()
  (self.root/'writer.py').write_text('def save(cancelled, original):\n    if cancelled:\n        return original\n    return b"new"\n')
  (self.root/'AGENTS.md').write_text('Preserve cancellation and original bytes.\n')
  (self.root/'unrelated.md').write_text('# Lunch\nSoup and sandwiches.\n')
 def tearDown(self):self.tmp.cleanup()
 def run_cli(self,*args,code=0):
  p=subprocess.run([str(BIN),*args],capture_output=True,text=True,timeout=30);self.assertEqual(p.returncode,code,p.stdout+p.stderr);return json.loads(p.stdout)
 def index(self):
  v=self.run_cli('index','--root',str(self.root),'--scope','*','--no-cache');self.assertEqual(v['state'],'indexed');path=self.p/'index.json';path.write_text(json.dumps(v));return path
 def test_exact_spans_and_unranked_anchors(self):
  idx=self.index();v=self.run_cli('select','--root',str(self.root),'--report',str(idx),'--query','cancelled original bytes')
  self.assertTrue(v['shortlist']);self.assertTrue(any(a.get('path')=='AGENTS.md'for a in v['anchors']))
  self.assertFalse(any(c['path']=='AGENTS.md'for c in v['shortlist']))
  for c in v['shortlist']:self.assertEqual((self.root/c['path']).read_bytes()[c['start']:c['end']].decode(),c['excerpt'])
 def test_delta_and_prior_reuse_without_provider(self):
  idx=self.index();args=['select','--root',str(self.root),'--report',str(idx),'--query','cancelled original bytes'];a=self.run_cli(*args);prior=self.p/'prior.json';prior.write_text(json.dumps(a));b=self.run_cli(*args,'--prior',str(prior),'--disclosure','synthetic');self.assertTrue(b['reuse']['same_prior_question_and_prepared_evidence']);self.assertEqual(b['assessment']['state'],'reported-prior');self.assertEqual(b['assessment']['attempts'],[])
  (self.root/'writer.py').write_text('def save(): return None\n');(self.root/'new.py').write_text('pass\n');v=self.run_cli(*args);self.assertIn('writer.py',v['coverage']['changed_paths']);self.assertIn('new.py',v['coverage']['added_paths'])
 def test_fabricated_summary_rejected(self):
  idx=self.index();v=json.loads(idx.read_text());v['fact_sets'][1]['facts'][0][1]='Fabricated claim';idx.write_text(json.dumps(v));result=self.run_cli('select','--root',str(self.root),'--report',str(idx),'--query','Fabricated claim');self.assertTrue(result['coverage']['unsupported'])
 def test_json_scope_and_unsupported_language(self):
  (self.root/'data.json').write_text('{"key":1}');contract=self.p/'contract';contract.write_text('UG\t1\n'+obligation(rule='json-syntax',scope='data.json'));self.run_cli('check','--root',str(self.root),'--contract',str(contract));(self.root/'data.json').write_text('{"key":1,"key":2}');self.run_cli('check','--root',str(self.root),'--contract',str(contract),code=1)
  contract.write_text('UG\t1\n'+obligation(rule='fact-heading',scope='writer.py',argument='# Title'));self.run_cli('check','--root',str(self.root),'--contract',str(contract),code=2)
 def test_baseline_catches_weakened_requirement(self):
  c=self.p/'contract';b=self.p/'baseline';c.write_text('UG\t1\n'+obligation(rule='contains',scope='writer.py',argument='cancelled'));b.write_bytes(c.read_bytes());self.run_cli('check','--root',str(self.root),'--contract',str(c),'--baseline',str(b));c.write_text('UG\t1\n'+obligation(rule='member',scope='writer.py',projection='membership'));v=self.run_cli('check','--root',str(self.root),'--contract',str(c),'--baseline',str(b),code=2);self.assertFalse(v['baseline_requirements_preserved'])
 def test_journal_duplicate_late_conflict(self):
  start=row('EVENT','op','running','request');done=row('EVENT','op','completed','result')
  self.assertEqual(core('JOURNAL\n'+start+done+done)[0][2],'completed')
  self.assertEqual(core('JOURNAL\n'+start+row('EVENT','op','cancelled','')+done)[0][2],'cancelled')
  self.assertEqual(core('JOURNAL\n'+start+done+row('EVENT','op','completed','different'))[0][2],'conflict-or-corruption')
  self.assertEqual(core('JOURNAL\n'+start+done.rstrip())[0][2],'conflict-or-corruption')
 def test_fixed_native_syntax_adapters(self):
  for flag,suffix,good,bad in [('--rust','rs','fn main() {}','fn main('),('--python','py','def f(): return 1','def f('),('--typescript','ts','const x: number = 1;','const x: = ;')]:
   p=self.p/f'input.{suffix}';p.write_text(good);self.run_cli('verify',flag,str(p));p.write_text(bad);self.run_cli('verify',flag,str(p),code=1)
 def test_c_preprocessor_alternatives_refused(self):
  for text in ['#include "x"','%:include "x"','??=include "x"','%\\\n:include "x"','_Pragma("GCC dependency x")']:
   self.assertEqual(core('NATIVE_PLAN\n'+row(text))[0][0],'REFUSED')
 def test_unsupported_input_preserves_unaffected_obligation(self):
  (self.root/'bad.py').symlink_to(self.p/'outside')
  c=self.p/'contract';c.write_text('UG\t1\n'+obligation(id='good',scope='writer.py',argument='cancelled')+obligation(id='bad',scope='bad.py',argument='hello'))
  v=self.run_cli('check','--root',str(self.root),'--contract',str(c),code=2)
  self.assertEqual([o['state']for o in v['obligations']],['verified','unknown'])
 def test_excluded_required_path_is_unresolved(self):
  (self.root/'target').mkdir();(self.root/'target/file').write_text('hello')
  c=self.p/'contract';c.write_text('UG\t1\n'+obligation(rule='member',scope='target/file',projection='membership'))
  v=self.run_cli('check','--root',str(self.root),'--contract',str(c),code=2);self.assertEqual(v['obligations'][0]['state'],'unknown')
 def test_generated_python_caches_do_not_pollute_source_requirement(self):
  source=self.root/'libs';source.mkdir();(source/'module.py').write_text('def clean(): return 1\n')
  for name in ('__pycache__','.pytest_cache','.ruff_cache','.mypy_cache'):
   cache=source/name;cache.mkdir();(cache/'generated.pyc').write_bytes(b'breakpoint()\x00\xff')
  c=self.p/'contract';c.write_text('UG\t1\n'+obligation(rule='absent-text',scope='prefix:libs/',argument='breakpoint()'))
  result=self.run_cli('check','--root',str(self.root),'--contract',str(c))
  self.assertEqual(result['obligations'][0]['state'],'verified')
  self.assertTrue({'__pycache__','.pytest_cache','.ruff_cache','.mypy_cache'}.issubset(set(result['inputs']['excluded_basenames'])))
  (source/'module.py').write_text('breakpoint()\n')
  result=self.run_cli('check','--root',str(self.root),'--contract',str(c),code=1)
  self.assertEqual(result['obligations'][0]['state'],'failed')
 def test_empty_directory_and_absence_queries(self):
  (self.root/'empty').mkdir();c=self.p/'contract';c.write_text('UG\t1\n'+obligation(rule='path-member',scope='empty',projection='membership'))
  a=self.run_cli('check','--root',str(self.root),'--contract',str(c));(self.root/'empty').rmdir();self.run_cli('check','--root',str(self.root),'--contract',str(c),code=1)
  c.write_text('UG\t1\n'+obligation(rule='absent-path',scope='empty',projection='membership'));self.run_cli('check','--root',str(self.root),'--contract',str(c));(self.root/'empty').mkdir();self.run_cli('check','--root',str(self.root),'--contract',str(c),code=1)
 def test_native_rust_obligation_and_stale_observation(self):
  (self.root/'main.rs').write_text('fn main() {}\n');c=self.p/'contract';c.write_text('UG\t1\n'+obligation(rule='native',scope='main.rs',argument='rust-syntax'))
  self.run_cli('check','--root',str(self.root),'--contract',str(c));(self.root/'main.rs').write_text('fn main(');self.run_cli('check','--root',str(self.root),'--contract',str(c),code=1)
  base='UG\t1\n'+obligation(rule='native',scope='main.rs',argument='rust-syntax')+row('OP','now','running','revision','timely')+row('F','main.rs','digest','fn main() {}')
  self.assertEqual(core(base+row('N','main.rs','digest','verified','old','syn-2.0.117','producer'))[1][2],'unknown')
  success=row('N','main.rs','digest','verified','now','syn-2.0.117','producer')
  self.assertEqual(core(base+success+success)[1][2],'verified')
  self.assertEqual(core(base+success+row('N','main.rs','digest','failed','now','syn-2.0.117','producer'))[1][2],'needs-review')
 def test_raw_numeric_precision(self):
  for value in ['-1e-999','1.00000000000000000000001','1e999']:
   body='{"answers":{"x":{"type":"noul","noul":'+value+'}}}'
   self.assertEqual(core('NUMERIC\n'+row(body)),[['NUMERIC','invalid']])
  for value in ['0','1','1.0','0.000000000000000000001','1e-999','-0']:
   body='{"answers":{"x":{"type":"noul","noul":'+value+'}}}'
   self.assertEqual(core('NUMERIC\n'+row(body)),[['NUMERIC','valid']])
 def test_project_proof_positive_false_and_no_runtime_effects(self):
  path=self.p/'proof.bend';path.write_text('import Base\nlaw identity:\n  for +x: Nat\n  {x == x : Nat}\ndef identity(x):\n  {==}\ndef main() -> IO(Unit):\n  IO.print("MUST_NOT_EXECUTE")\n')
  v=self.run_cli('verify','--bend-proof',str(path));self.assertNotIn('MUST_NOT_EXECUTE',v['stdout'])
  path.write_text('import Base\nlaw false_claim:\n  {0n == 1n : Nat}\ndef false_claim():\n  {==}\n');self.run_cli('verify','--bend-proof',str(path),code=1)
  path.write_text('import Base\nlaw unfinished:\n  {0n == 0n : Nat}\ndef main() -> Nat:\n  0n\n');self.run_cli('verify','--bend-proof',str(path),code=1)
  path.write_text('import Base\nimport ./other.bend as Other\nlaw x:\n  Nat\ndef x():\n  0n\n');self.run_cli('verify','--bend-proof',str(path),code=2)
 def test_adopted_laws_proof_closure_and_transitive_unsafe(self):
  project=self.p/'bend';project.mkdir();laws=project/'LAWS.bend';proof=project/'PROOF.bend';impl=project/'Math.bend'
  impl.write_text('import Base\ndef identity(x: Nat) -> Nat:\n  x\n')
  laws.write_text('import Base\nimport ./Math.bend as M\nlaw identity:\n  for +x: Nat\n  {M.identity(x) == x : Nat}\n')
  proof.write_text('import Base\nimport ./LAWS.bend as L\ndef L.identity(x):\n  {==}\n')
  report=self.run_cli('verify','--bend-proof',str(proof));self.assertEqual(set(report['source_closure']),{'PROOF.bend','LAWS.bend','Math.bend'})
  proof.write_text('import Base\ndef main() -> Nat:\n  0n\n');self.run_cli('verify','--bend-proof',str(proof),code=1)
  proof.write_text('import Base\nimport ./LAWS.bend as L\ndef L.identity(x):\n  {==}\n');impl.write_text('import Base\n@unsafe def identity(x: Nat) -> Nat:\n  x\n');self.run_cli('verify','--bend-proof',str(proof),code=2)
  impl.write_text('import Base\nimport 0xabc/main.bend as Remote\ndef identity(x: Nat) -> Nat:\n  x\n');self.run_cli('verify','--bend-proof',str(proof),code=2)
if __name__=='__main__':unittest.main()
