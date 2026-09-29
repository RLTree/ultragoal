import Base
import ./Core.bend as C
import ./Wire.bend as W
import ./Facts.bend as F
import ./Policy.bend as Policy
import ./Source.bend as Source
import ./Equality.bend as E

def is_tag(+r: List<&2, String>, +tag: String) -> Bool:
  E.text_eq(W.field(r,0n),tag)

def or_else(value: Bool,rest: Unit -> Bool) -> Bool:
  match value:
    case True{}: True{}
    case False{}: rest(Unit{})

def and_then(value: Bool,rest: Unit -> Bool) -> Bool:
  match value:
    case True{}: rest(Unit{})
    case False{}: False{}

def choose_string(ok: Bool, a: String, b: String) -> String:
  match ok:
    case True{}: a
    case False{}: b

def selected_file(explicit: Bool,path: String,selector: String) -> Bool:
  match explicit:
    case True{}: E.text_eq(path,String.drop(selector,5n))
    case False{}: E.text_eq(path,selector)

def selected_prefix(prefix: Bool,path: String,+selector: String) -> Bool:
  match prefix:
    case True{}: String.starts_with(path,String.drop(selector,7n))
    case False{}: selected_file(String.starts_with(selector,"file:"),path,selector)

def selected_all(all: Bool,path: String,+selector: String) -> Bool:
  match all:
    case True{}: True{}
    case False{}: selected_prefix(String.starts_with(selector,"prefix:"),path,selector)

def selected(+path: String, +selector: String) -> Bool:
  selected_all(E.text_eq(selector,"*"),path,selector)

def selection(+r: List<&2, String>, +scope: String) -> Bool:
  is_tag(r,"F") && selected(W.field(r,1n),scope)

def path_selection(+r: List<&2,String>,+scope: String) -> Bool:
  (is_tag(r,"F") || is_tag(r,"D") || is_tag(r,"U")) && selected(W.field(r,1n),scope)

def any_member(rs: List<&2, List<&2, String>>, +scope: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: or_else(selection(h,scope),u => any_member(t,scope))

def any_path(rs: List<&2,List<&2,String>>,+scope: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: or_else(path_selection(h,scope),u => any_path(t,scope))

def contains_row(+r: List<&2, String>, +scope: String, +needle: String) -> Bool:
  selection(r,scope) && String.contains(W.field(r,3n),needle)

def any_contains(rs: List<&2, List<&2, String>>, +scope: String, +needle: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: or_else(contains_row(h,scope,needle),u => any_contains(t,scope,needle))

def truth(v: Bool) -> C.Status:
  match v:
    case True{}: C.Verified{}
    case False{}: C.Failed{}

def native_status(status: String) -> C.Status:
  match status:
    case "verified": C.Verified{}
    case "failed": C.Failed{}
    case _: C.Unknown{}

def native_duplicate_if(same: Bool,status: C.Status) -> Maybe<&2,C.Status>:
  match same:
    case True{}: Some{status}
    case False{}: Some{C.Review{}}

def native_duplicate(+status: C.Status,other: Maybe<&2,C.Status>) -> Maybe<&2,C.Status>:
  match other:
    case None{}: Some{status}
    case Some{+previous}: native_duplicate_if(E.text_eq(C.status(status),C.status(previous)),status)

def native_found(ok: Bool,status: String,other: Maybe<&2,C.Status>) -> Maybe<&2,C.Status>:
  match ok:
    case True{}: native_duplicate(native_status(status),other)
    case False{}: other

def native_observation(rows: List<&2,List<&2,String>>,+path: String,+digest: String,+operation: String) -> Maybe<&2,C.Status>:
  match rows:
    case Nil{}: None{}
    case +h <> t: native_found(is_tag(h,"N") && E.text_eq(W.field(h,1n),path) && E.text_eq(W.field(h,2n),digest) && E.text_eq(W.field(h,4n),operation) && E.text_eq(W.field(h,5n),"syn-2.0.117"),W.field(h,3n),native_observation(t,path,digest,operation))

def native_merge(a: C.Status,b: C.Status) -> C.Status:
  match a b:
    case C.Unknown{} _: C.Unknown{}
    case _ C.Unknown{}: C.Unknown{}
    case C.Review{} _: C.Review{}
    case _ C.Review{}: C.Review{}
    case C.Failed{} _: C.Failed{}
    case _ C.Failed{}: C.Failed{}
    case _ _: C.Verified{}

def native_optional(value: Maybe<&2,C.Status>) -> C.Status:
  match value:
    case Some{status}: status
    case None{}: C.Unknown{}

def native_file(selected: Bool,+file: List<&2,String>,+rows: List<&2,List<&2,String>>,+operation: String) -> C.Status:
  match selected:
    case True{}: native_optional(native_observation(rows,W.field(file,1n),W.field(file,2n),operation))
    case False{}: C.Verified{}

def native_all(files: List<&2,List<&2,String>>,+rows: List<&2,List<&2,String>>,+scope: String,+operation: String) -> C.Status:
  match files:
    case Nil{}: C.Verified{}
    case +h <> t: native_merge(native_file(selection(h,scope),h,rows,operation),native_all(t,rows,scope,operation))

def find_if(ok: Bool, r: List<&2, String>, other: List<&2, String>) -> List<&2, String>:
  match ok:
    case True{}: r
    case False{}: other

def find(rs: List<&2, List<&2, String>>, +tag: String) -> List<&2, String>:
  match rs:
    case Nil{}: Nil{}
    case +h <> t: find_if(is_tag(h,tag),h,find(t,tag))

def native_rule(present: Bool,argument: String,+scope: String,+rows: List<&2,List<&2,String>>) -> C.Status:
  match present argument:
    case True{} "rust-syntax": native_all(rows,rows,scope,W.field(find(rows,"OP"),1n))
    case _ _: C.Unknown{}

def fact_row(+r: List<&2,String>,+scope: String,+kind: String,+arg: String) -> Bool:
  is_tag(r,"P") && selected(W.field(r,1n),scope) && F.contains(String.lines(W.field(r,4n)),kind,arg)

def fact_exists(rs: List<&2,List<&2,String>>,+scope: String,+kind: String,+arg: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: fact_row(h,scope,kind,arg) || fact_exists(t,scope,kind,arg)

def grammar_supported(rule: String,+grammar: String) -> Bool:
  match rule:
    case "fact-import-member": E.text_eq(grammar,"python-structure/1")
    case "fact-heading": E.text_eq(grammar,"markdown-lines/1")
    case "fact-json-valid": E.text_eq(grammar,"json/1")
    case "fact-json-key": E.text_eq(grammar,"json/1")
    case _: Source.supported(grammar) || E.text_eq(grammar,"rust-lexical/1") || E.text_eq(grammar,"python-lexical/1") || E.text_eq(grammar,"ecmascript-lexical/1")

def file_supported(+r: List<&2,String>,+scope: String,+rule: String) -> Bool:
  Bool.not(selection(r,scope)) || (grammar_supported(rule,F.grammar(W.field(r,1n))) && Policy.disclosable(W.field(r,1n),W.field(r,3n)))

def supported(rs: List<&2,List<&2,String>>,+scope: String,+rule: String) -> Bool:
  match rs:
    case Nil{}: True{}
    case +h <> t: file_supported(h,scope,rule) && supported(t,scope,rule)

def import_member_limited_row(+r: List<&2,String>,+scope: String) -> Bool:
  is_tag(r,"P") && selected(W.field(r,1n),scope) && F.has_kind(String.lines(W.field(r,4n)),"structure-limitation")

def import_member_limited(rs: List<&2,List<&2,String>>,+scope: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: import_member_limited_row(h,scope) || import_member_limited(t,scope)

def member_support(+r: List<&2,String>,+scope: String,+grammar: String) -> Bool:
  file_supported(r,scope,grammar) && (Bool.not(E.text_eq(grammar,"fact-import-member")) || Bool.not(import_member_limited_row(r,scope)))

def member_supported(rs: List<&2,List<&2,String>>,+scope: String) -> Bool:
  match rs:
    case Nil{}: True{}
    case +h <> t: member_support(h,scope,"fact-import-member") && member_supported(t,scope)

def fact_result(covered: Bool,found: Bool) -> C.Status:
  match covered:
    case True{}: truth(found)
    case False{}: C.Unknown{}

def json_row_valid(+r: List<&2,String>,+scope: String) -> Bool:
  Bool.not(is_tag(r,"P") && selected(W.field(r,1n),scope)) || F.contains(String.lines(W.field(r,4n)),"json-valid","true")

def json_all(rs: List<&2,List<&2,String>>,+scope: String) -> Bool:
  match rs:
    case Nil{}: True{}
    case +h <> t: json_row_valid(h,scope) && json_all(t,scope)

def evaluate(rule: String, +scope: String, +arg: String, +rs: List<&2, List<&2, String>>) -> C.Status:
  match rule:
    case "member": truth(any_member(rs,scope))
    case "path-member": truth(any_path(rs,scope))
    case "absent-path": truth(Bool.not(any_path(rs,scope)))
    case "contains": truth(any_contains(rs,scope,arg))
    case "absent-text": truth(Bool.not(any_contains(rs,scope,arg)))
    case "fact-heading": fact_result(supported(rs,scope,"fact-heading"),fact_exists(rs,scope,"heading-candidate",arg))
    case "fact-import": fact_result(supported(rs,scope,"fact-import"),fact_exists(rs,scope,"import-candidate",arg))
    case "fact-import-member": fact_result(member_supported(rs,scope),fact_exists(rs,scope,"structure-python-import-member",arg))
    case "fact-definition": fact_result(supported(rs,scope,"fact-definition"),fact_exists(rs,scope,"definition-candidate",arg))
    case "fact-json-key": fact_result(supported(rs,scope,"fact-json-key"),fact_exists(rs,scope,"json-key",arg))
    case "fact-json-valid": fact_result(supported(rs,scope,"fact-json-valid"),fact_exists(rs,scope,"json-valid","true"))
    case "json-syntax": fact_result(supported(rs,scope,"fact-json-valid"),any_member(rs,scope) && json_all(rs,scope))
    case "native": native_rule(any_member(rs,scope),arg,scope,rs)
    case _: C.Unknown{}


# Chunked content evaluation. Every content rule above is a fold of per-row
# facts, so a chunk of rows reduces to one Partial and the chunk partials fold
# to the verdict over all rows (LAWS.chunked_evaluation_exact).
type Rule is Data:
  Contains{}
  Absent{}
  Fact{kind: String, value: String, grammar: String}
  ImportMember{value: String}
  JsonSyntax{}
  Other{}

def resolve(rule: String,+arg: String) -> Rule:
  match rule:
    case "contains": Contains{}
    case "absent-text": Absent{}
    case "fact-heading": Fact{"heading-candidate",arg,"fact-heading"}
    case "fact-import": Fact{"import-candidate",arg,"fact-import"}
    case "fact-import-member": ImportMember{arg}
    case "fact-definition": Fact{"definition-candidate",arg,"fact-definition"}
    case "fact-json-key": Fact{"json-key",arg,"fact-json-key"}
    case "fact-json-valid": Fact{"json-valid","true","fact-json-valid"}
    case "json-syntax": JsonSyntax{}
    case _: Other{}

def content(rule: Rule) -> Bool:
  match rule:
    case Other{}: False{}
    case _: True{}

type Partial is Data:
  Partial{member: Bool, found: Bool, supported: Bool, valid: Bool}

def empty_partial() -> Partial:
  Partial{False{},False{},True{},True{}}

def merge(a: Partial,b: Partial) -> Partial:
  match a b:
    case Partial{m,f,s,v} Partial{m2,f2,s2,v2}: Partial{Bool.or(m,m2),Bool.or(f,f2),Bool.and(s,s2),Bool.and(v,v2)}

def row_partial(rule: Rule,+scope: String,+arg: String,+r: List<&2,String>) -> Partial:
  match rule:
    case Contains{}: Partial{False{},contains_row(r,scope,arg),True{},True{}}
    case Absent{}: Partial{False{},contains_row(r,scope,arg),True{},True{}}
    case Fact{+kind,+value,+g}: Partial{False{},fact_row(r,scope,kind,value),file_supported(r,scope,g),True{}}
    case ImportMember{+value}: Partial{False{},fact_row(r,scope,"structure-python-import-member",value),member_support(r,scope,"fact-import-member"),True{}}
    case JsonSyntax{}: Partial{selection(r,scope),False{},file_supported(r,scope,"fact-json-valid"),json_row_valid(r,scope)}
    case Other{}: empty_partial()

def partial(+rule: Rule,+scope: String,+arg: String,rs: List<&2,List<&2,String>>) -> Partial:
  match rs:
    case Nil{}: empty_partial()
    case +h <> t: merge(row_partial(rule,scope,arg,h),partial(rule,scope,arg,t))

def finish(rule: Rule,p: Partial) -> C.Status:
  match rule p:
    case Contains{} Partial{_,f,_,_}: truth(f)
    case Absent{} Partial{_,f,_,_}: truth(Bool.not(f))
    case Fact{_,_,_} Partial{_,f,s,_}: fact_result(s,f)
    case ImportMember{_} Partial{_,f,s,_}: fact_result(s,f)
    case JsonSyntax{} Partial{m,_,s,v}: fact_result(s,Bool.and(m,v))
    case Other{} _: C.Unknown{}

# Chunk listing: a chunk states its F rows as CF rows (path, digest), and the main
# evaluation expands them back into F rows with empty content.
def listed(file: Bool,+r: List<&2,String>,rest: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match file:
    case True{}: ["CF",W.field(r,1n),W.field(r,2n)] <> rest
    case False{}: rest

def listing(rows: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: listed(is_tag(h,"F"),h,listing(t))

def expand_listing(rows: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: ["F",W.field(h,1n),W.field(h,2n),""] <> expand_listing(t)

# Specification helpers for the chunk laws: the rows of consecutive chunks, the
# right fold of their partials (the order the main evaluation folds in), and the
# F rows with content emptied that the main frame carried before chunks listed
# their own files.
def concat(parts: List<&2,List<&2,List<&2,String>>>) -> List<&2,List<&2,String>>:
  match parts:
    case Nil{}: Nil{}
    case h <> t: List.append(&2,List<&2,String>,h,concat(t))

def combine(+rule: Rule,+scope: String,+arg: String,parts: List<&2,List<&2,List<&2,String>>>) -> Partial:
  match parts:
    case Nil{}: empty_partial()
    case h <> t: merge(partial(rule,scope,arg,h),combine(rule,scope,arg,t))

def listings(parts: List<&2,List<&2,List<&2,String>>>) -> List<&2,List<&2,List<&2,String>>>:
  match parts:
    case Nil{}: Nil{}
    case h <> t: listing(h) <> listings(t)

def emptied_row(file: Bool,+r: List<&2,String>,rest: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match file:
    case True{}: ["F",W.field(r,1n),W.field(r,2n),""] <> rest
    case False{}: rest

def emptied(rows: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: emptied_row(is_tag(h,"F"),h,emptied(t))

type ContentRule is Data:
  RContains{}
  RAbsent{}
  RHeading{}
  RImport{}
  RImportMember{}
  RDefinition{}
  RJsonKey{}
  RJsonValid{}
  RJsonSyntax{}

def content_name(rule: ContentRule) -> String:
  match rule:
    case RContains{}: "contains"
    case RAbsent{}: "absent-text"
    case RHeading{}: "fact-heading"
    case RImport{}: "fact-import"
    case RImportMember{}: "fact-import-member"
    case RDefinition{}: "fact-definition"
    case RJsonKey{}: "fact-json-key"
    case RJsonValid{}: "fact-json-valid"
    case RJsonSyntax{}: "json-syntax"

# Partial wire bits: fields 5..8 of a PC row.
def bit(b: Bool) -> String:
  match b:
    case True{}: "1"
    case False{}: "0"

def set_bit(s: String) -> Bool:
  match s:
    case "1": True{}
    case _: False{}

def bits(p: Partial) -> List<&2,String>:
  match p:
    case Partial{m,f,s,v}: [bit(m),bit(f),bit(s),bit(v)]

def unbits(+r: List<&2,String>) -> Partial:
  Partial{set_bit(W.field(r,5n)),set_bit(W.field(r,6n)),set_bit(W.field(r,7n)),set_bit(W.field(r,8n))}

def any_chunk(rs: List<&2,List<&2,String>>) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: or_else(is_tag(h,"CH"),u => any_chunk(t))

def keep_row(ok: Bool,row: List<&2,String>,tail: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match ok:
    case True{}: row <> tail
    case False{}: tail

def chunk_rows(rs: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rs:
    case Nil{}: Nil{}
    case +h <> t: keep_row(is_tag(h,"CH"),h,chunk_rows(t))

def partial_for(+r: List<&2,String>,+rule: String,+scope: String,+arg: String) -> Bool:
  is_tag(r,"PC") && E.text_eq(W.field(r,2n),rule) && E.text_eq(W.field(r,3n),scope) && E.text_eq(W.field(r,4n),arg)

def partial_rows(rs: List<&2,List<&2,String>>,+rule: String,+scope: String,+arg: String) -> List<&2,List<&2,String>>:
  match rs:
    case Nil{}: Nil{}
    case +h <> t: keep_row(partial_for(h,rule,scope,arg),h,partial_rows(t,rule,scope,arg))

# One PC row per CH row, in chunk order. An omitted chunk's PC row carries only
# whether any of its F rows is in scope: a chunk with none has the empty partial,
# and one with some is named omitted and contributes no facts.
type Folded is Data:
  Folded{partial: Partial, omitted: List<&2,String>, aligned: Bool, chunks: U32}

def selected_bit(p: Partial) -> Bool:
  match p:
    case Partial{m,_,_,_}: m

def fold_omitted(selected: Bool,chunk: String,rest: Folded) -> Folded:
  match selected rest:
    case True{} Folded{p,o,a,n}: Folded{p,chunk <> o,a,(n + 1 : U32)}
    case False{} Folded{p,o,a,n}: Folded{p,o,a,(n + 1 : U32)}

def fold_step(same: Bool,complete: Bool,chunk: String,+pc: List<&2,String>,rest: Folded) -> Folded:
  match same complete rest:
    case False{} _ Folded{p,o,_,n}: Folded{p,o,False{},(n + 1 : U32)}
    case True{} True{} Folded{p,o,a,n}: Folded{merge(unbits(pc),p),o,a,(n + 1 : U32)}
    case True{} False{} rest: fold_omitted(selected_bit(unbits(pc)),chunk,rest)

def fold_chunks(chs: List<&2,List<&2,String>>,pcs: List<&2,List<&2,String>>) -> Folded:
  match chs pcs:
    case Nil{} Nil{}: Folded{empty_partial(),Nil{},True{},0}
    case +c <> ct +p <> pt: fold_step(E.text_eq(W.field(c,1n),W.field(p,1n)),E.text_eq(W.field(c,2n),"complete"),W.field(c,1n),p,fold_chunks(ct,pt))
    case _ _: Folded{empty_partial(),Nil{},False{},0}

# With omitted chunks only an existential witness decides; everything else stays unknown.
def omitted_status(rule: Rule,p: Partial) -> C.Status:
  match rule p:
    case Contains{} Partial{_,True{},_,_}: C.Verified{}
    case Absent{} Partial{_,True{},_,_}: C.Failed{}
    case _ _: C.Unknown{}

def folded_status(+rule: Rule,f: Folded) -> C.Status:
  match f:
    case Folded{_,_,False{},_}: C.Unknown{}
    case Folded{p,Nil{},True{},_}: finish(rule,p)
    case Folded{p,_,True{},_}: omitted_status(rule,p)

def folded(+rule: String,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> Folded:
  fold_chunks(chunk_rows(rs),partial_rows(rs,rule,scope,arg))

def chunked_mode(+rule: String,+arg: String,+rs: List<&2,List<&2,String>>) -> Bool:
  content(resolve(rule,arg)) && any_chunk(rs)

def routed(chunked: Bool,+rule: String,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> C.Status:
  match chunked:
    case True{}: folded_status(resolve(rule,arg),folded(rule,scope,arg,rs))
    case False{}: evaluate(rule,scope,arg,rs)

# Frames carrying CH rows hold content only in their chunk partials.
def evaluate_rows(+rule: String,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> C.Status:
  routed(chunked_mode(rule,arg,rs),rule,scope,arg,rs)

# Counterexample for a failed prohibition, as [path, line]: the selected file with the
# smallest path that holds the forbidden text (with the 1-based line where its first
# match starts), the unexpected path, or the invalid JSON file. It explains a verdict
# and never decides one. Chunked frames carry each chunk's own counterexample as a PW
# row (chunk, rule, scope, argument, path, line), so both routes name the same file.
# Counts the newlines before the first position where `p` starts.
def line_from(+s: String,+p: String,+found: Bool,+n: U32) -> U32:
  match s:
    case SNil{}: n
    case SCon{c,t}: line_from(t,p,found || String.starts_with(t,p),Bool.pick(U32,found,n,(n + Bool.pick(U32,U32.is_eq(E.code(c),10),1,0) : U32)))

def line_of(+s: String,+p: String) -> U32:
  line_from(s,p,String.starts_with(s,p),1)

def hit_text(hit: Bool,+r: List<&2,String>,+arg: String) -> List<&2,String>:
  match hit:
    case True{}: [W.field(r,1n),U32.show(line_of(W.field(r,3n),arg))]
    case False{}: Nil{}

def hit_path(hit: Bool,+r: List<&2,String>) -> List<&2,String>:
  match hit:
    case True{}: [W.field(r,1n),""]
    case False{}: Nil{}

def row_witness(rule: String,+scope: String,+arg: String,+r: List<&2,String>) -> List<&2,String>:
  match rule:
    case "absent-text": hit_text(contains_row(r,scope,arg),r,arg)
    case "absent-path": hit_path(path_selection(r,scope),r)
    case "json-syntax": hit_path(Bool.not(json_row_valid(r,scope)),r)
    case _: Nil{}

def chunk_witness(hit: Bool,+r: List<&2,String>) -> List<&2,String>:
  match hit:
    case True{}: [W.field(r,5n),W.field(r,6n)]
    case False{}: Nil{}

def candidate(chunked: Bool,+rule: String,+scope: String,+arg: String,+r: List<&2,String>) -> List<&2,String>:
  match chunked:
    case True{}: chunk_witness(is_tag(r,"PW") && E.text_eq(W.field(r,2n),rule) && E.text_eq(W.field(r,3n),scope) && E.text_eq(W.field(r,4n),arg),r)
    case False{}: row_witness(rule,scope,arg,r)

def earlier(+w: List<&2,String>,+best: List<&2,String>) -> Bool:
  Bool.not(Nat.is_eq(W.length(w),0n)) && (Nat.is_eq(W.length(best),0n) || E.text_gt(W.field(best,0n),W.field(w,0n)))

def lower(take: Bool,+w: List<&2,String>,+best: List<&2,String>) -> List<&2,String>:
  match take:
    case True{}: w
    case False{}: best

def least(rs: List<&2,List<&2,String>>,+chunked: Bool,+rule: String,+scope: String,+arg: String,+best: List<&2,String>) -> List<&2,String>:
  match rs:
    case Nil{}: best
    case +h <> t:
      +w = candidate(chunked,rule,scope,arg,h)
      least(t,chunked,rule,scope,arg,lower(earlier(w,best),w,best))

def witness(+rule: String,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> List<&2,String>:
  least(rs,chunked_mode(rule,arg,rs),rule,scope,arg,Nil{})

# Whether a chunk's partial holds a counterexample its PW row must name.
def witnessed(rule: Rule,p: Partial) -> Bool:
  match rule p:
    case Absent{} Partial{_,f,_,_}: f
    case JsonSyntax{} Partial{_,_,_,v}: Bool.not(v)
    case _ _: False{}

def affected(+selector: String,+path: String) -> Bool:
  String.is_empty(path) || selected(path,selector) || String.starts_with(Policy.scope_path(selector),path ++ "/")

def uncertain_row(+r: List<&2,String>,+selector: String) -> Bool:
  and_then(is_tag(r,"U") || is_tag(r,"I"),u => affected(selector,W.field(r,1n)))

# An `unavailable` digest always comes with the U row that names its file, so
# only unread and undigested F rows add to the U and I rows.
def unread_row(+r: List<&2,String>,+scope: String) -> Bool:
  and_then(selection(r,scope),u => E.text_eq(W.field(r,2n),"unread") || String.is_empty(W.field(r,2n)))

def unavailable_count(rs: List<&2,List<&2,String>>,+scope: String,+n: U32) -> U32:
  match rs:
    case Nil{}: n
    case +h <> t: unavailable_count(t,scope,(n + Bool.pick(U32,uncertain_row(h,scope) || unread_row(h,scope),1,0) : U32))

def omitted_path(include: Bool,+r: List<&2,String>,rest: List<&2,String>) -> List<&2,String>:
  match include:
    case True{}: W.field(r,1n) <> rest
    case False{}: rest

def chunk_omitted(chunk: Bool,+r: List<&2,String>,previous: Bool) -> Bool:
  match chunk:
    case True{}: Bool.not(E.text_eq(W.field(r,2n),"complete"))
    case False{}: previous

# In-scope paths of omitted chunks, from the CF rows that follow each CH row.
def omitted_paths(rs: List<&2,List<&2,String>>,+scope: String,+omitted: Bool) -> List<&2,String>:
  match rs:
    case Nil{}: Nil{}
    case +h <> t: omitted_path(and_then(omitted && is_tag(h,"CF"),u => selected(W.field(h,1n),scope)),h,omitted_paths(t,scope,chunk_omitted(is_tag(h,"CH"),h,omitted)))

def path_row(paths: List<&2,String>) -> String:
  match paths:
    case Nil{}: ""
    case paths: W.row(paths)

def coverage_fields(f: Folded,paths: List<&2,String>,+missing: U32) -> List<&2,String>:
  match f:
    case Folded{_,+o,a,+n}: [U32.show(n),U32.show((n - U32.from_nat(W.length(o)) : U32)),path_row(paths),U32.show(missing),choose_string(a,"aligned","partials-missing")]

def coverage_if(chunked: Bool,+rule: String,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> List<&2,String>:
  match chunked:
    case True{}: coverage_fields(folded(rule,scope,arg,rs),omitted_paths(rs,scope,False{}),unavailable_count(rs,scope,0))
    case False{}: Nil{}

# Chunk count, evaluated chunks, the in-scope paths of omitted chunks as one
# encoded row, the in-scope unavailable input count and PC alignment; empty
# when unchunked.
def coverage(+rule: String,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> List<&2,String>:
  coverage_if(chunked_mode(rule,arg,rs),rule,scope,arg,rs)
