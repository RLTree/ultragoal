import Base
import ./Sha256.bend as Sha256
import ./NativeRecord.bend as NativeRecord
import ./Policy.bend as Policy
import ./QueryCompute.bend as QueryCompute
import ./QueryGraphSpec.bend as Spec
import ./QueryGraph.bend as QueryGraph
import ./ParseCache.bend as ParseCache
import ./ParsedValue.bend as Parsed
import ./GroupedParse.bend as Grouped
import ./Core.bend as C
import ./Wire.bend as W
import ./Engine.bend as E
import ./Facts.bend as F
import ./Predicate.bend as P
import ./Retrieval.bend as Rt
import ./Lexicon.bend as Lx

law trusted_span_parse_reuse:
  for +blob: String
  for +grammar: String
  for +source: String
  for witness: {blob == F.parse_spans(grammar,source) : String}
  {E.parse_spans_or_reuse(Some{blob},grammar,source) == F.parse_spans(grammar,source) : String}

law cancelled_rejects:
  for b: C.Binding
  for a: C.Assurance
  for g: C.Assurance
  for s: C.Status
  {C.admit(C.Cancelled{},b,a,g,s) == C.Unknown{} : C.Status}

law superseded_rejects:
  for b: C.Binding
  for a: C.Assurance
  for g: C.Assurance
  for s: C.Status
  {C.admit(C.Superseded{},b,a,g,s) == C.Unknown{} : C.Status}

law unknown_preserved:
  for ok: Bool
  {C.admit_if(ok,C.Unknown{}) == C.Unknown{} : C.Status}

law mandatory_preserved:
  {C.mandatory(True{},C.NotApplicable{}) == C.Unknown{} : C.Status}

law semantic_not_runtime:
  {C.assurance_ok(C.Observed{},C.Semantic{}) == False{} : Bool}

law semantic_not_formal:
  {C.assurance_ok(C.Formal{},C.Semantic{}) == False{} : Bool}

law semantic_never_verified:
  for life: C.Lifecycle
  for binding: C.Binding
  {C.admit(life,binding,C.Semantic{},C.Semantic{},C.Verified{}) == C.Unknown{} : C.Status}

law cancellation_terminal:
  for event: C.Lifecycle
  {C.terminal(C.Cancelled{},event) == C.Cancelled{} : C.Lifecycle}

law quota_empty:
  for cost: Nat
  {C.reserve(0n,1n+cost) == None{} : Maybe<&2,Nat>}

law stale_rejects:
  for life: C.Lifecycle
  for a: C.Assurance
  for g: C.Assurance
  for s: C.Status
  {C.admit(life,C.Bound{True{},False{},True{},True{},True{},True{},True{},True{}},a,g,s) == C.Unknown{} : C.Status}

law membership_branch_ignores_content:
  for +include: Bool
  for +path: String
  for digest_a: String
  for digest_b: String
  for text_a: String
  for text_b: String
  {E.fingerprint_if(include,["F",path,digest_a,text_a],"membership") == E.fingerprint_if(include,["F",path,digest_b,text_b],"membership") : String}

law membership_projection_ignores_content:
  for +path: String
  for digest_a: String
  for digest_b: String
  for text_a: String
  for text_b: String
  for +scope: String
  {E.fingerprint_row(["F",path,digest_a,text_a],scope,"membership") == E.fingerprint_row(["F",path,digest_b,text_b],scope,"membership") : String}

law operation_not_computation_key:
  for +requirement: List<&2,String>
  for +rows: List<&2,List<&2,String>>
  for operation: String
  {E.key(requirement,["OP",operation,"running","revision","timely"] <> rows) == E.key(requirement,rows) : String}

law producer_operation_independent:
  for +native: Bool
  for +rows: List<&2,List<&2,String>>
  for +scope: String
  for operation: String
  {E.producer_key(native,["OP",operation,"running","revision","timely"] <> rows,scope) == E.producer_key(native,rows,scope) : String}

law trusted_parse_reuse:
  for +blob: String
  for +grammar: String
  for +source: String
  for witness: {blob == F.parse(grammar,source) : String}
  {E.parse_or_reuse(Some{blob},grammar,source) == F.parse(grammar,source) : String}

law wire_roundtrip:
  for +source: String
  {W.decode(W.encode(source)) == Some{source} : Maybe<&2,String>}

law wire_tail:
  for +head: Char
  for +tail: String
  for witness: {W.decode(W.encode(tail)) == Some{tail} : Maybe<&2,String>}
  {W.prepend(head,W.decode(W.encode(tail))) == Some{SCon{head,tail}} : Maybe<&2,String>}


def requested_fact_row(request: Grouped.Request,first: Bool,key: String,value: String) -> List<&2,String>:
  match request:
    case Grouped.Request{+path,digest,compact,source}: [Grouped.choose(compact,"P_SPANS","P"),path,digest,F.grammar(path),value,Grouped.choose(first,"parsed","reused"),key]

def requested_bound_record(request: Grouped.Request,first: Bool,key: String,value: String) -> List<&2,String>:
  match request:
    case Grouped.Request{+path,digest,compact,source}: path <> requested_fact_row(Grouped.Request{path,digest,compact,source},first,key,value)

law grouped_parse_row_correct:
  for -canonical: List<&2,String>
  for +result: Parsed.Result<canonical>
  for +alias: Grouped.Alias<canonical>
  for +key: String
  {Grouped.bound_record(Grouped.bind(canonical,result,alias,key)) == requested_bound_record(Grouped.alias_request(canonical,alias),Grouped.alias_first(canonical,alias),key,Parsed.compute(Grouped.requested(canonical,alias))) : List<&2,String>}

law query_graph_revision_correct:
  for +nodes: List<&2,QueryGraph.Node>
  for +old: QueryGraph.State
  {QueryGraph.values(QueryGraph.run(nodes,old)) == Spec.full(nodes,MTip{}) : Map<&2,Maybe<&2,String>>}

# Regression for a real producer-adapter error: the record terminator is not
# part of the search argument, while an encoded newline inside that argument is.
law query_record_terminator_excluded:
  {QueryCompute.dependent("a.py",W.row(["contains","hello"]),["dep",W.row(["F","a.py","digest","hello"])]) == "verified" : String}

law query_argument_newline_preserved:
  {QueryCompute.dependent("a.py",W.row(["contains","hello\n"]),["dep",W.row(["F","a.py","digest","hello\n"])]) == "verified" : String}

law reported_native_cannot_be_current:
  for request: List<&2,String>
  for observation: List<&2,String>
  {NativeRecord.origin("reported-import",request,observation) == Policy.native_status(False{},False{}) : String}

law fit_requirement_change_has_no_patch:
  for before: String
  for after: String
  for exists: Bool
  {E.fit_validated(True{},False{},before,after,exists) == W.row(["FIT","requirement-change","","","","The proposed contract removes or changes a known requirement. Use an explicitly authorized adoption/migration boundary; no patch emitted."]) : String}

law digest_hex_length:
  for state: Sha256.State
  {String.length(Sha256.hex(state)) == 64n : Nat}

law digest_wrong_length_refused:
  {Sha256.hash(Nil{},1) == None{} : Maybe<&2,String>}

law digest_non_byte_refused:
  {Sha256.hash([256],1) == None{} : Maybe<&2,String>}

law digest_oversized_length_refused:
  {Sha256.hash(Nil{},16777217) == None{} : Maybe<&2,String>}

law retained_parse_choice_sound:
  for +args: List<&2,String>
  for entry: Maybe<&2,ParseCache.Entry>
  {ParseCache.chosen_value(args,ParseCache.choose(args,entry)) == Parsed.compute(args) : String}

law retained_parse_row_sound:
  for -canonical: List<&2,String>
  for result: Parsed.Result<canonical>
  for +alias: Grouped.Alias<canonical>
  for +key: String
  for +reused: Bool
  {Grouped.bound_record(ParseCache.bind(canonical,result,alias,key,reused)) == requested_bound_record(Grouped.alias_request(canonical,alias),ParseCache.cache_first(reused,Grouped.alias_first(canonical,alias)),key,Parsed.compute(Grouped.requested(canonical,alias))) : List<&2,String>}

# A content rule over consecutive row chunks equals the finish of the right fold
# of the chunk partials; the main check folds CHUNK partials in that order.
law chunked_evaluation_exact:
  for +rule: P.ContentRule
  for +scope: String
  for +arg: String
  for +parts: List<&2,List<&2,List<&2,String>>>
  {P.evaluate(P.content_name(rule),scope,arg,P.concat(parts)) == P.finish(P.resolve(P.content_name(rule),arg),P.combine(P.resolve(P.content_name(rule),arg),scope,arg,parts)) : C.Status}

law chunk_partial_wire_roundtrip:
  for p: P.Partial
  for +chunk: String
  for +rule: String
  for +scope: String
  for +arg: String
  {P.unbits(List.append(&2,String,["PC",chunk,rule,scope,arg],P.bits(p))) == p : P.Partial}

# The CF rows the chunk core lists for consecutive chunks expand in the main
# evaluation to exactly those chunks' F rows with their content emptied, in order.
law chunk_listing_exact:
  for +parts: List<&2,List<&2,List<&2,String>>>
  {P.expand_listing(P.concat(P.listings(parts))) == P.emptied(P.concat(parts)) : List<&2,List<&2,String>>}

# ---- select retrieval (Retrieval.bend, Lexicon.bend)

# Stage-1 window rows are exact partial sums: any split of the rows into chunks sums to the
# same totals, in any order of the chunks.
law select_window_sums_split:
  for xs: List<&2,Rt.WStat>
  for +ys: List<&2,Rt.WStat>
  {Rt.wsum(List.append(&2,Rt.WStat,xs,ys)) == Rt.wadd(Rt.wsum(xs),Rt.wsum(ys)) : Rt.WStat}

law select_window_sums_commute:
  for a: Rt.WStat
  for b: Rt.WStat
  {Rt.wadd(a,b) == Rt.wadd(b,a) : Rt.WStat}

# A file's partial rows from different chunks merge alike in any grouping and order.
law select_file_counts_commute:
  for a: Rt.FStat
  for b: Rt.FStat
  {Rt.fadd(a,b) == Rt.fadd(b,a) : Rt.FStat}

law select_file_counts_associate:
  for a: Rt.FStat
  for b: Rt.FStat
  for c: Rt.FStat
  {Rt.fadd(Rt.fadd(a,b),c) == Rt.fadd(a,Rt.fadd(b,c)) : Rt.FStat}

def tri_path(t: Rt.Tri) -> Nat:
  match t:
    case Rt.Tri{p,_,_}: p

# Every chunk holding part of a file tokenises its path, so its path counts once.
law select_path_counts_once:
  for +p: Nat
  for d1: Nat
  for b1: Nat
  for d2: Nat
  for b2: Nat
  {tri_path(Rt.tri_add(Rt.Tri{p,d1,b1},Rt.Tri{p,d2,b2})) == p : Nat}

def flip(c: Cmp) -> Cmp:
  match c:
    case LT{}: GT{}
    case EQ{}: EQ{}
    case GT{}: LT{}

# Paths are totally ordered by code point: swapping the arguments flips the order, only
# equal paths compare equal, and the order is transitive; file ranks break ties by it.
law select_path_order_flips:
  for a: String
  for b: String
  {Rt.order(b,a) == flip(Rt.order(a,b)) : Cmp}

law select_path_order_equal:
  for a: String
  for b: String
  for e: {Rt.order(a,b) == EQ{} : Cmp}
  {a == b : String}

law select_path_order_transitive:
  for a: String
  for b: String
  for c: String
  for ab: {Rt.order(a,b) == LT{} : Cmp}
  for bc: {Rt.order(b,c) == LT{} : Cmp}
  {Rt.order(a,c) == LT{} : Cmp}

# Grouping FILE rows by path does not depend on their order: two insertions commute, so
# every permutation of the stage-1 rows groups to the same files.
law select_grouping_swap:
  for +a: Rt.FileRow
  for +b: Rt.FileRow
  for xs: List<&2,Rt.FileRow>
  {Rt.group(a,Rt.group(b,xs)) == Rt.group(b,Rt.group(a,xs)) : List<&2,Rt.FileRow>}

law select_rows_permute:
  for xs: List<&2,Rt.FileRow>
  for +a: Rt.FileRow
  for +b: Rt.FileRow
  for +ys: List<&2,Rt.FileRow>
  for acc: List<&2,Rt.FileRow>
  {Rt.grouped(List.append(&2,Rt.FileRow,xs,a <> b <> ys),acc) == Rt.grouped(List.append(&2,Rt.FileRow,xs,b <> a <> ys),acc) : List<&2,Rt.FileRow>}

def carry(s: Lx.Scan) -> String & U32:
  match s:
    case Lx.Scan{run,h,_,_,_}: (run,h)

# Scanning is a left fold over characters, and a line break resets the identifier run and
# the definition-name recognizer: windows that never split a line tokenise as their
# concatenation does, and the counts of the parts add up.
law select_scan_concat:
  for a: String
  for b: String
  for st: Lx.Scan
  for +node: Lx.Trie
  {Lx.walk(String.append(a,b),st,node) == Lx.walk(b,Lx.walk(a,st,node),node) : Lx.Scan}

law select_line_resets:
  for st: Lx.Scan
  for +node: Lx.Trie
  {carry(Lx.step(st,10,node)) == ("",0) : String & U32}
