"""Adversarial proof admission: expected failures must actually fail."""
import pathlib,subprocess,tempfile,shutil,os,json
ROOT=pathlib.Path(__file__).resolve().parents[1]
# Same Bun heap cap as build.py: check-only peaks stay near 9 GiB instead of ~14 GiB.
env={**os.environ,'BEND_NO_TELEMETRY':'1','BUN_JSC_forceRAMSize':str(8*2**30)}
def check(root):
 p=subprocess.run(['bend',str(root/'PROOF.bend'),'--check-only'],capture_output=True,text=True,env=env,timeout=900)
 return p.returncode==0 and 'All terms check.'in p.stdout and not any(x in (p.stdout+p.stderr).lower()for x in ['unsafe','open claim','foreign dependency'])
assert check(ROOT)
mutations=[
 ('cancel-revival','Core.bend','case Cancelled{}: False{}','UNUSED'),
 ('stale-admission','Core.bend','case False{}: Unknown{}','case False{}: result'),
 ('mandatory-loss','Core.bend','case NotApplicable{}: Unknown{}','case NotApplicable{}: NotApplicable{}'),
 ('quota-underflow','Core.bend','case 0n 1n+c: None{}','case 0n 1n+c: Some{0n}'),
 ('semantic-runtime','Core.bend','case _ _: False{}','case _ _: True{}'),
 ('open-proof','PROOF.bend','def L.cancelled_rejects(b,a,g,s):\n  {==}','def L.cancelled_rejects(b,a,g,s):\n  ?TODO'),
 ('omitted-laws','PROOF.bend','import ./LAWS.bend as L',''),
 ('membership-content-leak','Engine.bend','E.text_eq(projection,"membership"),"file",W.field(r,2n)','E.text_eq(projection,"membership"),W.field(r,2n),W.field(r,2n)'),
 # Retained reuse is only as sound as the equality decision; weakening it must break the proofs.
 ('string-equality-unsound','Equality.bend','    case SNil{} SNil{}: ok','    case SNil{} SNil{}: True{}'),
 ('list-equality-unsound','Equality.bend','    case Nil{} Nil{}: ok','    case Nil{} Nil{}: True{}'),
 ('trusted-parse-corruption','ParseWork.bend','case Some{blob}: blob','case Some{blob}: "corrupt"'),
 # Chunk partials must fold to the unchunked verdict and survive the PC row encoding.
 ('chunk-merge-unsound','Predicate.bend','Bool.and(s,s2)','Bool.or(s,s2)'),
 ('chunk-absent-partial-dropped','Predicate.bend','case Absent{}: Partial{False{},contains_row(r,scope,arg),True{},True{}}','case Absent{}: Partial{False{},False{},True{},True{}}'),
 ('chunk-bit-order','Predicate.bend','case Partial{m,f,s,v}: [bit(m),bit(f),bit(s),bit(v)]','case Partial{m,f,s,v}: [bit(f),bit(m),bit(s),bit(v)]'),
 # The chunk listing must expand to exactly the chunk's F rows (chunk_listing_exact).
 ('chunk-listing-empty-digest','Predicate.bend','case True{}: ["CF",W.field(r,1n),W.field(r,2n)] <> rest','case True{}: ["CF",W.field(r,1n),""] <> rest'),
 ('chunk-listing-drops-last-row','Predicate.bend','    case +h <> t: listed(is_tag(h,"F"),h,listing(t))','    case h <> Nil{}: Nil{}\n    case +h <> t: listed(is_tag(h,"F"),h,listing(t))'),
 # select retrieval: one plausible wrong implementation per law of LAWS.bend.
 ('select-window-sum-empty-counts-one','Retrieval.bend','    case Nil{}: WStat{0n,0n,Nil{}}','    case Nil{}: WStat{1n,0n,Nil{}}'),
 ('select-window-tokens-keep-left','Retrieval.bend','WStat{Nat.add(n1,n2),Nat.add(t1,t2),nadd(c1,c2)}','WStat{Nat.add(n1,n2),Nat.add(t1,1n),nadd(c1,c2)}'),
 ('select-file-length-keep-left','Retrieval.bend','FStat{Nat.add(l1,l2),vadd(v1,v2)}','FStat{l1,vadd(v1,v2)}'),
 ('select-file-vector-drops-shorter','Retrieval.bend','    case h <> t Nil{}: h <> t\n    case h1 <> t1 h2 <> t2: tri_add(h1,h2) <> vadd(t1,t2)','    case h <> t Nil{}: Nil{}\n    case h1 <> t1 h2 <> t2: tri_add(h1,h2) <> vadd(t1,t2)'),
 ('select-path-counted-per-chunk','Retrieval.bend','Tri{Nat.max(p1,p2),Nat.add(d1,d2),Nat.add(b1,b2)}','Tri{Nat.add(p1,p2),Nat.add(d1,d2),Nat.add(b1,b2)}'),
 ('select-order-prefix-before-equal','Retrieval.bend','    case EQ{}: rest\n    case GT{}: GT{}','    case EQ{}: LT{}\n    case GT{}: GT{}'),
 ('select-order-shorter-equal','Retrieval.bend','    case SNil{} SCon{_,_}: LT{}','    case SNil{} SCon{_,_}: EQ{}'),
 ('select-order-cyclic-characters','Retrieval.bend','    case Chr{x} Chr{y}: U32.cmp(x,y)','    case Chr{+x} Chr{+y}: Bool.pick(Cmp,U32.is_eq((x % 3 : U32),(y % 3 : U32)),U32.cmp(x,y),Bool.pick(Cmp,U32.is_eq((y % 3 : U32),(((x % 3 : U32) + 1 : U32) % 3 : U32)),LT{},GT{}))'),
 ('select-group-equal-paths-unmerged','Retrieval.bend','    case EQ{} FileRow{_,rs} FileRow{hp,hs}: FileRow{hp,fadd(rs,hs)} <> t','    case EQ{} x y: x <> y <> t'),
 ('select-group-stops-after-head','Retrieval.bend','    case LT{} _ y: y <> rest(Unit{})','    case LT{} x y: y <> x <> t'),
 ('select-rows-not-grouped','Retrieval.bend','    case h <> t: grouped(t,group(h,acc))','    case h <> t: grouped(t,h <> acc)'),
 ('select-scan-lookahead-across-windows','Lexicon.bend','    case SCon{Chr{+x},t}: walk(t,step(st,x,node),node)','    case SCon{\'a\',SCon{Chr{+x},t}}: walk(t,step(st,x,node),node)\n    case SCon{Chr{+x},t}: walk(t,step(st,x,node),node)'),
 ('select-scan-resets-on-carriage-return','Lexicon.bend','  Bool.pick(U32,U32.is_eq(x,10),0,h)','  Bool.pick(U32,U32.is_eq(x,13),0,h)'),
]
results=[]
for name,file,old,new in mutations:
 if name=='cancel-revival':old='case _: False{}';new='case _: True{}'
 with tempfile.TemporaryDirectory()as d:
  p=pathlib.Path(d)
  for src in ROOT.glob('*.bend'):shutil.copy2(src,p/src.name)
  target=p/file;s=target.read_text();assert old in s,name;target.write_text(s.replace(old,new,1))
  rejected=not check(p);assert rejected,name;results.append({'mutation':name,'rejected':rejected})
print(json.dumps({'law_suite':True,'mutations':results},indent=2))
