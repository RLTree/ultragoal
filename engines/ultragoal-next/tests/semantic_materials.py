"""Canonical material-shape validation; no provider or native admission claim."""
import copy,json,unittest
from journey import core,row

def context():
 return {'schema':'ultragoal-semantic-context/3','target':'verification','scope':'the supplied assertion design',
  'material_contract':{'question':'verification-design','closed_world':'provided-materials','conditions':[]},
  'clauses':[{'id':'c1','statement':'The assertion distinguishes denied access from success','required_roles':['asserted-test']}],
  'evidence':[{'id':'e1','role':'asserted-test','material_type':'test-source','version':'v1','content':'assert load(denied) == PermissionDenied','provenance':'synthetic test source, not a run','clause_ids':['c1']}]}

POLICY='ug-semantic-fit-v3:sha256:1c0a12c9debbe922dfa3846007d7cefdbd99fd57634b571945a6a4ca768254ab'

def check(value,family='verification'):
 return core('SEMANTIC_CONTEXT\n'+row(family,json.dumps(value)))

class SemanticMaterials(unittest.TestCase):
 def test_canonical_material_and_declared_condition(self):
  v=context();self.assertEqual(check(v),[['CONTEXT','valid']])
  v['material_contract']['conditions']=[{'id':'d1','statement':'The supplied denied input is in scope','evidence_ids':['e1']}]
  self.assertEqual(check(v),[['CONTEXT','valid']])
 def test_types_versions_references_and_authority_smuggling_rejected(self):
  changes=[lambda v:v['evidence'][0].pop('version'),
   lambda v:v['evidence'][0].update(material_type='execution-report'),
   lambda v:v['evidence'][0].update(clause_ids=['missing']),
   lambda v:v['evidence'].append(copy.deepcopy(v['evidence'][0])),
   lambda v:v['clauses'].append(copy.deepcopy(v['clauses'][0])),
   lambda v:v.update(host_attested=True),
   lambda v:v['material_contract'].update(approved=True),
   lambda v:v['material_contract'].update(question='reported-recovery'),
   lambda v:v['material_contract'].update(closed_world='trusted-universal'),
   lambda v:v['material_contract'].update(conditions=[{'id':'d','statement':'Known','evidence_ids':['missing']}]),
   lambda v:v['material_contract'].update(conditions=[{'id':'d','statement':'Known','evidence_ids':[]}]),
   lambda v:v['evidence'][0].update(role='authenticated-host-authority')]
  for change in changes:
   v=context();change(v)
   with self.subTest(value=v):self.assertEqual(check(v),[['CONTEXT','invalid']])
 def test_explicit_dependency_world_needs_linked_inventory_material(self):
  v=context();v['target']='investigation-reuse';v['material_contract']['question']='finding-applicability';v['material_contract']['closed_world']='explicit-dependencies'
  self.assertEqual(check(v,'investigation-reuse'),[['CONTEXT','invalid']])
  v['evidence'].append({'id':'deps','role':'retrieval-metadata','material_type':'dependency-inventory','version':'snapshot1','content':'For this literal assertion: test source e1 and stated denied input only; no runtime completeness claim.','provenance':'synthetic inventory','clause_ids':['c1']})
  v['material_contract']['conditions']=[{'id':'d1','statement':'Only the listed materials are being compared','evidence_ids':['deps']}]
  self.assertEqual(check(v,'investigation-reuse'),[['CONTEXT','valid']]);v['material_contract']['conditions'][0]['evidence_ids']=['e1'];self.assertEqual(check(v,'investigation-reuse'),[['CONTEXT','invalid']])
 def test_report_is_material_not_native_authority(self):
  v=context();v['material_contract']['question']='reported-verification';v['evidence'][0].update(role='executed-observation',material_type='execution-report',content='Reported process exit=1; denied-input assertion executed and exposed the implementation bug.')
  v['clauses'][0]['required_roles']=['executed-observation']
  self.assertEqual(check(v),[['CONTEXT','valid']])
  v['evidence'][0]['authenticated']=True;self.assertEqual(check(v),[['CONTEXT','invalid']])
 def test_existing_v2_shape_remains_declared_compatibility(self):
  v=context();v['schema']='ultragoal-semantic-context/2';v.pop('material_contract')
  for e in v['evidence']:e.pop('material_type');e.pop('version')
  self.assertEqual(check(v),[['CONTEXT','valid']])
 def test_conflicting_valid_probes_preserve_a_visible_review_signal(self):
  r=core('SEMANTIC_CALIBRATED\n'+row('4','advisory-explicit-conflict','advisory-complete-support','','','','','','','alignment'))[0]
  self.assertEqual(r[1],'contradicted');self.assertEqual(r[4],'conflicting-valid-probes');self.assertEqual(r[3],'no-combined-probability')
 def test_probes_distinguish_semantic_surface_and_host_admission(self):
  r=core('SEMANTIC_PROBES\n'+row('verification'));self.assertEqual(len(r),2)
  self.assertTrue(all(p[2]=='4'and'authenticity'in p[3]and'verification-design'in p[3]for p in r))
  r=core('SEMANTIC_PROBES\n'+row('investigation-reuse'))
  self.assertTrue(all('Finding-usefulness'in p[3]and'Finding-applicability'in p[3]for p in r))

 def test_calibrated_threshold_decision(self):
  # Probabilities: conflict (explicit, no-explicit, uncertain), support (complete, missing, uncertain).
  def d(version,c,s,cp,sp,family='recovery-state'):return core('SEMANTIC_CALIBRATED\n'+row(version,c,s,*cp,*sp,family))[0]
  C,N,S,M,U='advisory-explicit-conflict','advisory-no-explicit-conflict','advisory-complete-support','advisory-missing-support','advisory-uncertain'
  low=('0.1','0.8','0.1')
  self.assertEqual(d('4',N,S,('0.81','0.17','0.02'),low)[1:3],['contradicted',POLICY])
  self.assertEqual(d('4',N,S,('0.80','0.18','0.02'),low)[1],'insufficient')
  self.assertEqual(d('4',N,S,('0.81','0.18','0.01'),low)[1],'insufficient')
  self.assertEqual(d('4',N,S,low,('0.92','0.08','0'))[1],'supported')
  self.assertEqual(d('4',N,S,low,('0.92','0.09','0'))[1],'insufficient')
  self.assertEqual(d('4',N,S,low,('0.91','0.05','0.04'))[1],'insufficient')
  # Conflict is decided first; a tied top never qualifies.
  self.assertEqual(d('4',N,S,('0.9','0.1','0'),('0.97','0.02','0.01'))[1],'contradicted')
  self.assertEqual(d('4',C,S,('0.5','0.5','0'),('0.5','0.5','0'))[1],'insufficient')
  # Probabilities, not categorical labels, decide under the fitted policy.
  self.assertEqual(d('4',C,M,('0.2','0.7','0.1'),('0.3','0.6','0.1'))[1],'insufficient')
  # Other versions and invalid numbers use the uncalibrated combination.
  self.assertEqual(d('3',N,S,('0.9','0.1','0'),low)[1:3],['supported','uncalibrated-advisory'])
  self.assertEqual(d('4',N,S,('','',''),low)[1:3],['supported','uncalibrated-advisory'])
  self.assertEqual(d('4',N,S,('1.5','0','0'),low)[1:3],['supported','uncalibrated-advisory'])
  self.assertEqual(d('4',N,S,('nan','0','0'),low)[2],'uncalibrated-advisory')
  # Unvalidated answers never become a decision.
  self.assertEqual(d('4','malformed',S,('0.1','0.8','0.1'),low)[1],'unavailable')
  self.assertEqual(d('4',N,'unavailable',('0.1','0.8','0.1'),low)[1],'unavailable')
  self.assertEqual(core('SEMANTIC_CALIBRATED\n'+row('4',N,S,'0.1'))[0][1],'unavailable')
  # The family field is required.
  self.assertEqual(core('SEMANTIC_CALIBRATED\n'+row('4',N,S,'0.9','0.1','0',*low))[0][1],'unavailable')
 def test_conflict_gates_are_per_family(self):
  N,S='advisory-no-explicit-conflict','advisory-complete-support'
  low=('0.1','0.8','0.1')
  d=lambda family,cp:core('SEMANTIC_CALIBRATED\n'+row('4',N,S,*cp,*low,family))[0][1]
  # family, the lowest qualifying conflict answer, and the same answer one grid step short
  for family,hit,miss in [('alignment',('0.51','0.47','0.02'),('0.51','0.48','0.01')),
                          ('verification',('0.63','0.36','0.01'),('0.62','0.36','0.02')),
                          ('consistency',('0.88','0.10','0.02'),('0.87','0.10','0.03')),
                          ('investigation-reuse',('0.75','0.23','0.02'),('0.74','0.23','0.03')),
                          ('recovery-state',('0.81','0.17','0.02'),('0.80','0.18','0.02')),
                          ('recovery-proposal',('0.81','0.17','0.02'),('0.80','0.18','0.02'))]:
   self.assertEqual((family,d(family,hit)),(family,'contradicted'))
   self.assertEqual((family,d(family,miss)),(family,'insufficient'))
 def test_only_qualified_contradictions_block(self):
  adm=lambda *f:core('SEMANTIC_ADMIT\n'+row(*f))
  blocked=adm('o1','unknown','recovery-state','contradicted',POLICY,'on')[0]
  self.assertEqual(blocked[:3]+blocked[4:],['SEMANTIC_ADMISSION','o1','failed','qualified semantic block under '+POLICY])
  self.assertIn('recovery-state',blocked[3])
  unchanged=lambda state:['SEMANTIC_ADMISSION','o1',state,'','']
  self.assertEqual(adm('o1','unknown','recovery-state','contradicted',POLICY,'off')[0],unchanged('unknown'))
  self.assertEqual(adm('o1','unknown','alignment','contradicted',POLICY,'on')[0],unchanged('unknown'))
  self.assertEqual(adm('o1','unknown','recovery-state','insufficient',POLICY,'on')[0],unchanged('unknown'))
  self.assertEqual(adm('o1','unknown','recovery-state','contradicted','uncalibrated-advisory','on')[0],unchanged('unknown'))
  self.assertEqual(adm('o1','verified','recovery-state','contradicted',POLICY,'on')[0],unchanged('verified'))
  self.assertEqual(adm('o1','unknown','recovery-state','contradicted',POLICY,'yes'),[['ERROR','invalid semantic blocking switch']])
  self.assertEqual(adm('o1','unknown','recovery-state','contradicted',POLICY),[['ERROR','invalid semantic admission row']])
  two=core('SEMANTIC_ADMIT\n'+row('a','unknown','recovery-state','contradicted',POLICY,'on')+row('b','unknown','verification','contradicted',POLICY,'on'))
  self.assertEqual([r[2] for r in two],['failed','unknown'])
 def test_select_relevance_hides_only_gated_irrelevant_answers(self):
  rel=lambda *p:core('RELEVANCE_DECISIONS\n'+row('item0',*p))[0]
  self.assertEqual(rel('0.05','0.9','0.05'),['RELEVANCE','item0','irrelevant',POLICY])
  self.assertEqual(rel('0.1','0.88','0.02')[2],'insufficient')
  self.assertEqual(rel('0.0','0.89','0.11')[2],'insufficient')
  self.assertEqual(rel('0.99','0.01','0')[2],'insufficient')
  self.assertEqual(rel('0.45','0.45','0.1')[2],'insufficient')
  self.assertEqual(rel('x','0.9','0.1')[2],'unassessed')

if __name__=='__main__':unittest.main()
