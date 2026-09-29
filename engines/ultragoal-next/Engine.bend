import Base
import ./Core.bend as C
import ./Wire.bend as W
import ./Jev.bend as J
import ./JevLayout.bend as JevLayout
import ./Utf8.bend as U
import ./Facts.bend as F
import ./Policy.bend as Policy
import ./Json.bend as Json
import ./Fit.bend as Fit
import ./Lifecycle.bend as Lifecycle
import ./Identity.bend as Identity
import ./Select.bend as Select
import ./ExcerptPlan.bend as ExcerptPlan
import ./Decimal.bend as Decimal
import ./Semantic.bend as Semantic
import ./ParseWork.bend as ParseWork
import ./ParseCache.bend as ParseCache
import ./GroupedParse.bend as GroupedParse
import ./Advice.bend as Advice
import ./FrameIO.bend as FrameIO
import ./NativeRecord.bend as NativeRecord
import ./Digest.bend as Digest
import ./Transport.bend as Transport
import ./Source.bend as Source
import ./Predicate.bend as Predicate
import ./QueryGraph.bend as QG
import ./QueryPlan.bend as QP
import ./Invalidation.bend as Invalidation
import ./Equality.bend as E
import ./JevWindow.bend as JevWindow
import ./Retrieval.bend as Retrieval
import ./SegmentSession.bend as Segment
import ./RankSession.bend as Rank

type Input is Data:
  Input{rows: List<&2, List<&2, String>>, error: String}

type Batch is Data:
  Batch{frames: List<&2,FrameIO.Frame>, ended: Bool, count: U32, bytes: U32}

type SessionState is Data:
  SessionState{graph: QG.State,parses: ParseCache.State}

def empty_session() -> SessionState:
  SessionState{QG.State{MTip{},MTip{},MTip{}},ParseCache.State{MTip{},0,0}}

type Processed is Data:
  Processed{text: String, state: SessionState}

# A retained evaluation frame: each line with its parsed wire record.
type Cached is Data:
  Cached{line: String, record: W.Record}

type Base is Data:
  NoBase{}
  Base{seq: U32, lines: List<&2,Cached>}

type Retained is Data:
  Retained{state: SessionState, base: Base, segment: Segment.State, rank: Rank.State}

type Step is Data:
  Step{text: String, retained: Retained}

type DeltaOp is Data:
  Keep{}
  Skip{}
  Literal{line: String}

type SessionCommand is Data:
  Single{lines: List<&2,String>}
  Index{frames: Nat}
  InvalidCommand{reason: String}

def add_row(row: W.Record, rest: Input) -> Input:
  match row rest:
    case W.Row{xs} Input{rs,e}: Input{xs <> rs,e}
    case W.Invalid{e} _: Input{Nil{},e}

def parse_rows(lines: List<&2, String>) -> Input:
  match lines:
    case Nil{}: Input{Nil{},""}
    case "" <> t: parse_rows(t)
    case h <> t: add_row(W.parse(h), parse_rows(t))

def is_tag(+r: List<&2, String>, tag: String) -> Bool:
  Predicate.is_tag(r,tag)

def or_else(value: Bool,rest: Unit -> Bool) -> Bool:
  Predicate.or_else(value,rest)

def and_then(value: Bool,rest: Unit -> Bool) -> Bool:
  Predicate.and_then(value,rest)

def choose_string(ok: Bool, a: String, b: String) -> String:
  Predicate.choose_string(ok,a,b)

def selected(+path: String, +selector: String) -> Bool:
  Predicate.selected(path,selector)

def selection(+r: List<&2, String>, +scope: String) -> Bool:
  Predicate.selection(r,scope)

def path_selection(+r: List<&2,String>,+scope: String) -> Bool:
  Predicate.path_selection(r,scope)

def fingerprint_if(include: Bool,+r: List<&2,String>,projection: String) -> String:
  match include:
    case True{}: W.row([W.field(r,1n),choose_string(E.text_eq(projection,"membership"),"file",W.field(r,2n))])
    case False{}: ""

def other_fingerprint(include: Bool,+r: List<&2,String>) -> String:
  match include:
    case True{}: W.row([W.field(r,1n),choose_string(is_tag(r,"D"),"directory",W.field(r,2n))])
    case False{}: ""

def fingerprint_row(+r: List<&2, String>, +scope: String, +projection: String) -> String:
  fingerprint_if(selection(r,scope),r,projection) ++ other_fingerprint((is_tag(r,"D") || is_tag(r,"U")) && selected(W.field(r,1n),scope),r)

def fingerprint(rs: List<&2, List<&2, String>>, +scope: String, +projection: String) -> String:
  match rs:
    case Nil{}: ""
    case +h <> t: fingerprint_row(h,scope,projection) ++ fingerprint(t,scope,projection)

def any_member(rs: List<&2, List<&2, String>>, +scope: String) -> Bool:
  Predicate.any_member(rs,scope)

def find(rs: List<&2, List<&2, String>>, +tag: String) -> List<&2, String>:
  Predicate.find(rs,tag)

def input_complete_row(+r: List<&2,String>,+scope: String) -> Bool:
  Bool.not(selection(r,scope)) || (Bool.not(E.text_eq(W.field(r,2n),"unread")) && Bool.not(E.text_eq(W.field(r,2n),"unavailable")) && Bool.not(String.is_empty(W.field(r,2n))))

def inputs_complete(rs: List<&2,List<&2,String>>,+scope: String) -> Bool:
  match rs:
    case Nil{}: True{}
    case +h <> t: input_complete_row(h,scope) && inputs_complete(t,scope)

def evaluate(+rule: String, +scope: String, +arg: String, +rs: List<&2, List<&2, String>>) -> C.Status:
  Predicate.evaluate_rows(rule,scope,arg,rs)

def assurance(s: String) -> C.Assurance:
  match s:
    case "semantic": C.Semantic{}
    case "runtime": C.Observed{}
    case "formal": C.Formal{}
    case _: C.Exact{}

def lifecycle(s: String) -> C.Lifecycle:
  match s:
    case "running": C.Running{}
    case "cancelled": C.Cancelled{}
    case "superseded": C.Superseded{}
    case "completed": C.Completed{}
    case _: C.Planned{}

# Rule tables: one membership test per table instead of a string-literal branch
# per rule, which each expanded into its own character-matching C code.
def RULES() -> List<&2,String>:
  ["member","path-member","absent-path","contains","absent-text","native","semantic","fact-heading","fact-import","fact-import-member","fact-definition","fact-json-key","fact-json-valid","json-syntax"]

# Rules whose projection must be content (or unknown).
def CONTENT_RULES() -> List<&2,String>:
  ["native","semantic","contains","absent-text","fact-heading","fact-import","fact-import-member","fact-definition","fact-json-key","fact-json-valid","json-syntax"]

# Rules that require a nonempty argument.
def ARGUMENT_RULES() -> List<&2,String>:
  ["contains","absent-text","fact-heading","fact-import","fact-import-member","fact-definition"]

def rule_valid(+rule: String) -> Bool:
  E.text_member(RULES(),rule)

def assurance_valid(a: String) -> Bool:
  match a:
    case "exact": True{}
    case "semantic": True{}
    case "runtime": True{}
    case "formal": True{}
    case _: False{}

def projection_valid(p: String) -> Bool:
  match p:
    case "membership": True{}
    case "content": True{}
    case "unknown": True{}
    case _: False{}

def rule_projection(+rule: String, +projection: String) -> Bool:
  Bool.not(E.text_member(CONTENT_RULES(),rule)) || E.text_eq(projection,"content") || E.text_eq(projection,"unknown")

def argument_valid(+rule: String,argument: String) -> Bool:
  Bool.not(E.text_member(ARGUMENT_RULES(),rule)) || Bool.not(String.is_empty(argument))

def obligation_valid(+r: List<&2, String>) -> Bool:
  Nat.is_eq(W.length(r),11n) && Bool.not(String.is_empty(W.field(r,1n))) && Bool.not(String.is_empty(W.field(r,2n))) && Bool.not(String.is_empty(W.field(r,3n))) && Bool.not(String.is_empty(W.field(r,4n))) && Bool.not(String.is_empty(W.field(r,5n))) && projection_valid(W.field(r,6n)) && rule_projection(W.field(r,7n),W.field(r,6n)) && rule_valid(W.field(r,7n)) && argument_valid(W.field(r,7n),W.field(r,8n)) && assurance_valid(W.field(r,9n)) && E.text_eq(W.field(r,10n),"mandatory")

def member_key(+r: List<&2, String>, +key: String) -> Bool:
  is_tag(r,"O") && E.text_eq(W.field(r,1n),key)

def duplicate(rs: List<&2, List<&2, String>>, +key: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: member_key(h,key) || duplicate(t,key)

def record_valid(tag: String, +r: List<&2, String>, +tail: List<&2, List<&2, String>>) -> Bool:
  match tag:
    case "UG": E.text_eq(W.field(r,1n),"1") && Nat.is_eq(W.length(r),2n) && Bool.not(is_tag(find(tail,"UG"),"UG"))
    case "O": obligation_valid(r) && Bool.not(duplicate(tail,W.field(r,1n)))
    case "F": Nat.is_eq(W.length(r),4n)
    case "OP": Nat.is_eq(W.length(r),5n) && Bool.not(is_tag(find(tail,"OP"),"OP"))
    case "CACHE": Nat.is_eq(W.length(r),3n)
    case "PCACHE": Nat.is_eq(W.length(r),3n)
    case "Q": Nat.is_eq(W.length(r),2n) && Bool.not(String.is_empty(W.field(r,1n)))
    case "U": Nat.is_eq(W.length(r),4n)
    case "I": Nat.is_eq(W.length(r),3n)
    case "X": Nat.is_eq(W.length(r),2n)
    case "D": Nat.is_eq(W.length(r),2n)
    case "N": Nat.is_eq(W.length(r),7n)
    case "SB": Nat.is_eq(W.length(r),2n) && Policy.within(U32.read(W.field(r,1n)),4096) && Policy.nonzero(U32.read(W.field(r,1n))) && Bool.not(is_tag(find(tail,"SB"),"SB"))
    case "CH": Nat.is_eq(W.length(r),7n) && Bool.not(String.is_empty(W.field(r,1n)))
    case "CF": Nat.is_eq(W.length(r),3n)
    case "FC": Nat.is_eq(W.length(r),1n)
    case "PC": Nat.is_eq(W.length(r),9n)
    case "PW": Nat.is_eq(W.length(r),7n)
    case "CI": Nat.is_eq(W.length(r),2n) && Bool.not(String.is_empty(W.field(r,1n))) && Bool.not(is_tag(find(tail,"CI"),"CI"))
    case _: False{}

def valid_rows(rs: List<&2, List<&2, String>>) -> Bool:
  match rs:
    case Nil{}: True{}
    case +h <> +t: record_valid(W.field(h,0n),h,t) && valid_rows(t)

def semantic_scope(ok: Bool,+r: List<&2,String>,tail: List<&2,String>) -> List<&2,String>:
  match ok:
    case True{}: W.field(r,5n) <> tail
    case False{}: tail

# Selectors of the semantic obligations; their files keep content in a chunked frame.
def semantic_scopes(rows: List<&2,List<&2,String>>) -> List<&2,String>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: semantic_scope(is_tag(h,"O") && E.text_eq(W.field(h,7n),"semantic"),h,semantic_scopes(t))

def any_selects(scopes: List<&2,String>,+path: String) -> Bool:
  match scopes:
    case Nil{}: False{}
    case h <> t: or_else(selected(path,h),u => any_selects(t,path))

def content_allowed(+r: List<&2,String>,+scopes: List<&2,String>) -> Bool:
  Bool.not(is_tag(r,"F")) || String.is_empty(W.field(r,3n)) || any_selects(scopes,W.field(r,1n))

def content_semantic(rows: List<&2,List<&2,String>>,+scopes: List<&2,String>) -> Bool:
  match rows:
    case Nil{}: True{}
    case +h <> t: content_allowed(h,scopes) && content_semantic(t,scopes)

def any_chunk_row(rows: List<&2,List<&2,String>>) -> Bool:
  match rows:
    case Nil{}: False{}
    case +h <> t: or_else(is_tag(h,"PC") || is_tag(h,"CF") || is_tag(h,"FC"),u => any_chunk_row(t))

# A chunked frame carries content only for files a semantic obligation selects;
# its F rows come from the chunk listing (expand_chunked). Partials, listings and
# the listing marker never appear without chunks.
def chunks_checked(chunked: Bool,+rows: List<&2,List<&2,String>>) -> Bool:
  match chunked:
    case True{}: content_semantic(rows,semantic_scopes(rows))
    case False{}: Bool.not(any_chunk_row(rows))

def chunks_valid(+rows: List<&2,List<&2,String>>) -> Bool:
  chunks_checked(Predicate.any_chunk(rows),rows)

# A chunked evaluation frame states its files once, as the chunk core's CF rows in
# chunk order. One FC marker stands where the F rows go, and the frame's own F
# rows are only those that keep content for a semantic obligation. Expansion puts
# one F row per CF row after the marker (LAWS.chunk_listing_exact), substituting
# each kept row where path and digest are exactly equal. Every CH row must be
# followed directly by its stated count of CF rows, and every kept row consumed.
type Listing is Data:
  Listing{files: List<&2,List<&2,String>>, kept: List<&2,List<&2,String>>, markers: Nat, run: U32, ok: Bool}

def stated(value: Maybe<&2,U32>,+raw: String,+run: U32) -> Bool:
  match value:
    case Some{+n}: U32.is_eq(n,run) && E.text_eq(raw,U32.show(n))
    case None{}: False{}

# Folded from the right: `run` counts the CF rows directly after the current row.
def listing_row(tag: String,+r: List<&2,String>,state: Listing) -> Listing:
  match tag state:
    case "CF" Listing{files,kept,markers,run,ok}: Listing{r <> files,kept,markers,(run + 1 : U32),ok}
    case "CH" Listing{files,kept,markers,+run,ok}: Listing{files,kept,markers,0,ok && stated(U32.read(W.field(r,5n)),W.field(r,5n),run)}
    case "F" Listing{files,kept,markers,+run,ok}: Listing{files,r <> kept,markers,0,ok && U32.is_eq(run,0)}
    case "FC" Listing{files,kept,markers,+run,ok}: Listing{files,kept,1n+markers,0,ok && U32.is_eq(run,0)}
    case _ Listing{files,kept,markers,+run,ok}: Listing{files,kept,markers,0,ok && U32.is_eq(run,0)}

def frame_listing(rows: List<&2,List<&2,String>>) -> Listing:
  match rows:
    case Nil{}: Listing{Nil{},Nil{},0n,0,True{}}
    case +h <> t: listing_row(W.field(h,0n),h,frame_listing(t))

type Merged is Data:
  Merged{rows: List<&2,List<&2,String>>, ok: Bool}

def merged_row(+r: List<&2,String>,same: Bool,m: Merged) -> Merged:
  match m:
    case Merged{rows,ok}: Merged{r <> rows,ok && same}

def kept_at(same: Bool,+f: List<&2,String>,+k: List<&2,String>,kt: List<&2,List<&2,String>>,next: List<&2,List<&2,String>> -> Merged) -> Merged:
  match same:
    case True{}: merged_row(k,E.text_eq(W.field(f,2n),W.field(k,2n)),next(kt))
    case False{}: merged_row(f,True{},next(k <> kt))

def substituted(files: List<&2,List<&2,String>>,kept: List<&2,List<&2,String>>) -> Merged:
  match files kept:
    case files Nil{}: Merged{files,True{}}
    case Nil{} _: Merged{Nil{},False{}}
    case +f <> ft +k <> kt: kept_at(E.text_eq(W.field(f,1n),W.field(k,1n)),f,k,kt,ks => substituted(ft,ks))

def kept_selected(rows: List<&2,List<&2,String>>,+scopes: List<&2,String>) -> Bool:
  match rows:
    case Nil{}: True{}
    case +h <> t: any_selects(scopes,W.field(h,1n)) && kept_selected(t,scopes)

def spliced_row(tag: String,+r: List<&2,String>,+fs: List<&2,List<&2,String>>,rest: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match tag:
    case "F": rest
    case "FC": r <> List.append(&2,List<&2,String>,fs,rest)
    case _: r <> rest

def spliced(rows: List<&2,List<&2,String>>,+fs: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: spliced_row(W.field(h,0n),h,fs,spliced(t,fs))

def expand_merged(ok: Bool,merged: Merged,+rows: List<&2,List<&2,String>>,error: String) -> Input:
  match ok merged:
    case True{} Merged{fs,True{}}: Input{spliced(rows,fs),error}
    case _ _: Input{Nil{},"invalid chunk listing"}

def expand_listed(listing: Listing,+rows: List<&2,List<&2,String>>,error: String) -> Input:
  match listing:
    case Listing{files,+kept,markers,+run,ok}: expand_merged(ok && U32.is_eq(run,0) && Nat.is_eq(markers,1n) && kept_selected(kept,semantic_scopes(rows)),substituted(Predicate.expand_listing(files),kept),rows,error)

def expand_if(chunked: Bool,+rows: List<&2,List<&2,String>>,error: String) -> Input:
  match chunked:
    case True{}: expand_listed(frame_listing(rows),rows,error)
    case False{}: Input{rows,error}

def expand_chunked(input: Input) -> Input:
  match input:
    case Input{+rows,error}: expand_if(Predicate.any_chunk(rows),rows,error)

def exclusion_if(include: Bool,r: List<&2,String>) -> String:
  match include:
    case True{}: W.row(r)
    case False{}: ""

def exclusion_row(+r: List<&2,String>) -> String:
  exclusion_if(is_tag(r,"X"),r)

def exclusion_key(rows: List<&2,List<&2,String>>) -> String:
  match rows:
    case Nil{}: ""
    case +h <> t: exclusion_row(h) ++ exclusion_key(t)

def native_key_row(include: Bool,+r: List<&2,String>) -> String:
  match include:
    case True{}: W.row([W.field(r,1n),W.field(r,2n),W.field(r,5n),W.field(r,6n)])
    case False{}: ""

def native_key(rows: List<&2,List<&2,String>>,+scope: String) -> String:
  match rows:
    case Nil{}: ""
    case +h <> t: native_key_row(is_tag(h,"N") && selected(W.field(h,1n),scope),h) ++ native_key(t,scope)

def producer_key(native: Bool,rows: List<&2,List<&2,String>>,scope: String) -> String:
  match native:
    case True{}: native_key(rows,scope)
    case False{}: ""

def key(+r: List<&2, String>, +rs: List<&2, List<&2, String>>) -> String:
  W.row([Identity.rules(),W.field(r,2n),W.field(r,5n),W.field(r,6n),W.field(r,7n),W.field(r,8n)]) ++ exclusion_key(rs) ++ fingerprint(rs,W.field(r,5n),W.field(r,6n)) ++ producer_key(String.eq(W.field(r,7n),"native"),rs,W.field(r,5n))

def affected(+selector: String,+path: String) -> Bool:
  Predicate.affected(selector,path)

def excluded_row(+r: List<&2,String>,+selector: String) -> Bool:
  and_then(is_tag(r,"X"),u => Policy.component(String.split(Policy.scope_path(selector),'/'),W.field(r,1n)))

def excluded(rows: List<&2,List<&2,String>>,+selector: String) -> Bool:
  match rows:
    case Nil{}: False{}
    case +h <> t: or_else(excluded_row(h,selector),u => excluded(t,selector))

def uncertain_row(+r: List<&2,String>,+selector: String) -> Bool:
  Predicate.uncertain_row(r,selector)

def uncertain(rows: List<&2,List<&2,String>>,+selector: String) -> Bool:
  match rows:
    case Nil{}: False{}
    case +h <> t: or_else(uncertain_row(h,selector),u => uncertain(t,selector))

def positive(rule: String) -> Bool:
  match rule:
    case "member": True{}
    case "path-member": True{}
    case "contains": True{}
    case +r: String.starts_with(r,"fact-")

def cached(rs: List<&2, List<&2, String>>, +k: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: (is_tag(h,"CACHE") && E.text_eq(W.field(h,1n),k)) || cached(t,k)

def action(rule: String) -> String:
  match rule:
    case "native": "Run the named verifier on a qualified immutable snapshot; imported text cannot discharge runtime assurance."
    case "semantic": "Run the authorized versioned semantic rubric; uncalibrated results remain advisory."
    case _: "Inspect the exact selected inputs and correct the requirement mismatch."

def excerpt_row(+r: List<&2,String>, +scope: String) -> String:
  choose_string(selection(r,scope) && Policy.disclosable(W.field(r,1n),W.field(r,3n)),W.row([W.field(r,1n),W.field(r,2n),W.field(r,3n)]),"")

def excerpts(rs: List<&2,List<&2,String>>, +scope: String) -> String:
  match rs:
    case Nil{}: ""
    case +h <> t: excerpt_row(h,scope) ++ excerpts(t,scope)

def rubric(name: String) -> String:
  Semantic.rubric(name)

def disclosure_row(+r: List<&2,String>,+scope: String) -> Bool:
  Bool.not(selection(r,scope)) || Policy.disclosable(W.field(r,1n),W.field(r,3n))

def disclosure_complete(rows: List<&2,List<&2,String>>,+scope: String) -> Bool:
  match rows:
    case Nil{}: True{}
    case +h <> t: disclosure_row(h,scope) && disclosure_complete(t,scope)

def semantic_packet(complete: Bool,+r: List<&2,String>,+rs: List<&2,List<&2,String>>,digest: String) -> String:
  match complete:
    case True{}: W.row(["REQUEST",W.field(r,1n),W.field(r,3n),W.field(r,4n),W.field(r,8n),excerpts(rs,W.field(r,5n)),digest,rubric(W.field(r,8n))])
    case False{}: W.row(["SEMANTIC_UNAVAILABLE",W.field(r,1n),"Selected evidence or supported explicit assessment target is unavailable. Legacy recovery requires migration to recovery-proposal, recovery-state or investigation-reuse; no packet emitted."])

# A chunked frame may withhold a selected file's content; an empty content field
# stands for an empty file only when the digest is that of empty content.
def content_present_row(+r: List<&2,String>,+scope: String) -> Bool:
  Bool.not(selection(r,scope)) || Bool.not(String.is_empty(W.field(r,3n))) || E.text_eq(W.field(r,2n),"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")

def contents_present(rs: List<&2,List<&2,String>>,+scope: String) -> Bool:
  match rs:
    case Nil{}: True{}
    case +h <> t: content_present_row(h,scope) && contents_present(t,scope)

def semantic_budget(remaining: Nat,+r: List<&2,String>,+rs: List<&2,List<&2,String>>,digest: String) -> String:
  match remaining:
    case 0n: W.row(["SEMANTIC_UNAVAILABLE",W.field(r,1n),"Operation question budget exhausted; obligation remains unresolved."])
    case 1n: W.row(["SEMANTIC_UNAVAILABLE",W.field(r,1n),"Two probe slots required; obligation remains unresolved."])
    case 1n+p: semantic_packet(contents_present(rs,W.field(r,5n)) && Bool.not(E.text_eq(rubric(W.field(r,8n)),"")) && any_member(rs,W.field(r,5n)) && inputs_complete(rs,W.field(r,5n)) && disclosure_complete(rs,W.field(r,5n)) && Bool.not(uncertain(rs,W.field(r,5n))) && Bool.not(excluded(rs,W.field(r,5n))),r,rs,digest)

def semantic_request(rule: String, +r: List<&2,String>, +rs: List<&2,List<&2,String>>,remaining: Nat,digest: String) -> String:
  match rule:
    case "semantic": semantic_budget(remaining,r,rs,digest)
    case _: ""

def coverage_tail(proved_positive: Bool,+r: List<&2,String>,+rs: List<&2,List<&2,String>>) -> Bool:
  match proved_positive:
    case True{}: True{}
    case False{}: and_then(Bool.not(uncertain(rs,W.field(r,5n))),u => or_else(E.text_eq(W.field(r,6n),"membership"),u => inputs_complete(rs,W.field(r,5n))))

def native_predicate_surface(argument: String) -> String:
  match argument:
    case "rust-syntax": "captured Rust syntax predicate; not type checking or tests"
    case _: "qualified runtime observation required; currently unavailable"

def predicate_surface(rule: String,argument: String) -> String:
  match rule:
    case "member": "captured file membership predicate"
    case "path-member": "captured path membership predicate"
    case "absent-path": "captured path absence predicate"
    case "contains": "captured text search predicate"
    case "absent-text": "captured text absence predicate"
    case "fact-heading": "bounded heading candidate predicate"
    case "fact-import": "bounded import candidate predicate"
    case "fact-import-member": "bounded Python module/member/local-binding predicate"
    case "fact-definition": "bounded definition candidate predicate"
    case "fact-json-key": "captured JSON key predicate"
    case "fact-json-valid": "captured JSON grammar predicate"
    case "json-syntax": "captured JSON grammar predicate"
    case "native": native_predicate_surface(argument)
    case "semantic": "supplied material meaning; native admission remains separate"
    case _: "unsupported predicate surface"

def dependency_projection(rule: String) -> String:
  match rule:
    case "member": "membership"
    case "path-member": "membership"
    case "absent-path": "membership"
    case _: "content"

def chunk_coverage(id: String,fields: List<&2,String>) -> String:
  match fields:
    case Nil{}: ""
    case fields: W.row("CHUNK_COVERAGE" <> id <> fields)

def witness_fields(id: String,w: List<&2,String>) -> String:
  match w:
    case Nil{}: ""
    case w: W.row("WITNESS" <> id <> w)

def witness_row(failed: Bool,id: String,+rule: String,+scope: String,+arg: String,+rs: List<&2,List<&2,String>>) -> String:
  match failed:
    case True{}: witness_fields(id,Predicate.witness(rule,scope,arg,rs))
    case False{}: ""

def result_computed(+computed: C.Status,computation: String,+r: List<&2, String>, +rs: List<&2, List<&2, String>>, +op: List<&2, String>,remaining: Nat) -> String:
  +computation_key = Digest.identity(key(r,rs))
  complete = Bool.not(E.text_eq(W.field(r,6n),"unknown")) && Bool.not(excluded(rs,W.field(r,5n))) && coverage_tail(positive(W.field(r,7n)) && E.text_eq(C.status(computed),"verified"),r,rs)
  admitted = C.admit(lifecycle(W.field(op,2n)), C.Bound{True{},True{},True{},True{},complete,True{},True{},E.text_eq(W.field(op,4n),"timely")},assurance(W.field(r,9n)),C.Exact{},computed)
  W.row(["RESULT",W.field(r,1n),C.status(C.mandatory(True{},admitted)),W.field(r,3n),W.field(r,4n),action(W.field(r,7n)),computation_key,computation,W.field(r,5n),W.field(r,8n),W.field(r,9n),W.field(r,7n),W.field(r,2n),predicate_surface(W.field(r,7n),W.field(r,8n)),dependency_projection(W.field(r,7n))]) ++ chunk_coverage(W.field(r,1n),Predicate.coverage(W.field(r,7n),W.field(r,5n),W.field(r,8n),rs)) ++ witness_row(E.text_eq(C.status(computed),"failed"),W.field(r,1n),W.field(r,7n),W.field(r,5n),W.field(r,8n),rs) ++ semantic_request(W.field(r,7n),r,rs,remaining,computation_key)

def result(+r: List<&2,String>,+rs: List<&2,List<&2,String>>,op: List<&2,String>,remaining: Nat) -> String:
  result_computed(evaluate(W.field(r,7n),W.field(r,5n),W.field(r,8n),rs),choose_string(cached(rs,key(r,rs)),"advisory-match","miss"),r,rs,op,remaining)

def output_if(ok: Bool, +r: List<&2, String>, +rs: List<&2, List<&2, String>>, +op: List<&2, String>,remaining: Nat) -> String:
  match ok:
    case True{}: result(r,rs,op,remaining)
    case False{}: ""

def remaining(value: Maybe<&2,Nat>) -> Nat:
  match value:
    case Some{n}: n
    case None{}: 0n

def consume_question(semantic: Bool,budget: Nat) -> Nat:
  match semantic:
    case True{}: remaining(C.reserve(budget,2n))
    case False{}: budget

# Semantic questions an operation may use: two per obligation. Without an `SB` row
# (`--semantic-max-requests N`) every semantic obligation gets its own request.
def budget_of(n: Maybe<&2,U32>) -> Nat:
  match n:
    case Some{v}: Nat.mul(U32.to_nat(v),2n)
    case None{}: 0n

def budget_or_all(present: Bool,n: String) -> Nat:
  match present:
    case True{}: budget_of(U32.read(n))
    case False{}: 8192n

def question_budget(+sb: List<&2,String>) -> Nat:
  budget_or_all(is_tag(sb,"SB"),W.field(sb,1n))

def results(rows: List<&2, List<&2, String>>, +rs: List<&2, List<&2, String>>, +op: List<&2, String>,+budget: Nat) -> String:
  match rows:
    case Nil{}: ""
    case +h <> t: output_if(is_tag(h,"O"),h,rs,op,budget) ++ results(t,rs,op,consume_question(is_tag(h,"O") && E.text_eq(W.field(h,7n),"semantic"),budget))

def parse_or_reuse(hit: Maybe<&2,String>,g: String,text: String) -> String:
  ParseWork.parse_or_reuse(hit,g,text)

def parse_spans_or_reuse(hit: Maybe<&2,String>,g: String,text: String) -> String:
  ParseWork.parse_spans_or_reuse(hit,g,text)

def fact_needed_row(+r: List<&2,String>,+path: String) -> Bool:
  (is_tag(r,"Q") && selected(path,W.field(r,1n))) || (is_tag(r,"O") && (String.starts_with(W.field(r,7n),"fact-") || E.text_eq(W.field(r,7n),"json-syntax")) && selected(path,W.field(r,5n)))

def fact_needed(rs: List<&2,List<&2,String>>,+path: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: fact_needed_row(h,path) || fact_needed(t,path)

def eligible_file(+r: List<&2,String>,qs: List<&2,List<&2,String>>) -> Bool:
  and_then(is_tag(r,"F") && fact_needed(qs,W.field(r,1n)),u => Policy.disclosable(W.field(r,1n),W.field(r,3n)) && input_complete_row(r,W.field(r,1n)))

def keep_file(ok: Bool,row: List<&2,String>,tail: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match ok:
    case True{}: row <> tail
    case False{}: tail

def eligible_files(rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: keep_file(eligible_file(h,qs),h,eligible_files(t,qs))

def mapped_fact(found: Maybe<&2,List<&2,String>>,+r: List<&2,String>) -> List<&2,List<&2,String>>:
  match found:
    case Some{value}: [value]
    case None{}: [["P",W.field(r,1n),W.field(r,2n),"private-source-excluded","","excluded",Digest.identity(W.row([Identity.parsers(),"excluded",W.field(r,2n)]))]]

def mapped_result(result: Map<&2,Maybe<&2,List<&2,String>>> & Maybe<&2,List<&2,String>>,r: List<&2,String>) -> List<&2,List<&2,String>>:
  match result:
    case (memo,found): mapped_fact(found,r)

def mapped_if(needed: Bool,+r: List<&2,String>,memo: Map<&2,Maybe<&2,List<&2,String>>>) -> List<&2,List<&2,String>>:
  match needed:
    case False{}: Nil{}
    case True{}: mapped_result(Map.get(Maybe<&2,List<&2,String>>,None{},memo,W.field(r,1n)),r)

def materialize(rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>,+memo: Map<&2,Maybe<&2,List<&2,String>>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: List.append(&2,List<&2,String>,mapped_if(is_tag(h,"F") && fact_needed(qs,W.field(h,1n)),h,memo),h <> materialize(t,qs,memo))

def emit_fact_if(include: Bool,r: List<&2,String>) -> String:
  match include:
    case True{}: W.row(r)
    case False{}: ""

def fact_report(mixed: Bool,+r: List<&2,String>) -> String:
  match mixed r:
    case True{} _ <> tail: W.row("P_MIXED" <> tail)
    case _ row: emit_fact_if(is_tag(row,"P") || is_tag(row,"P_SPANS") || is_tag(row,"ERROR"),row)

def emit_fact_row(+r: List<&2,String>) -> String:
  fact_report(is_tag(r,"P") && Source.supported(W.field(r,3n)),r)

def emit_facts(rows: List<&2,List<&2,String>>) -> String:
  match rows:
    case Nil{}: ""
    case +h <> t: emit_fact_row(h) ++ emit_facts(t)

def normalized(+rs: List<&2,List<&2,String>>,qs: List<&2,List<&2,String>>) -> String:
  W.row(["REPORT","1","advisory-adoption"]) ++ results(qs,rs,find(rs,"OP"),question_budget(find(rs,"SB"))) ++ emit_facts(rs)

def parsed_work(work: ParseWork.ParseBatch,rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> String:
  match work:
    case ParseWork.ParseBatch{_,False{}}: "ERROR\tconflicting bytes for one declared parse identity\n"
    case ParseWork.ParseBatch{memo,True{}}: normalized(materialize(rows,qs,memo),qs)

def normalize_parallel(+rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>,compact: Bool) -> String:
  parsed_work(ParseWork.run(eligible_files(rows,qs),compact),rows,qs)

def keep_query(ok: Bool,h: List<&2,String>,tail: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match ok:
    case True{}: h <> tail
    case False{}: tail

def queries(rows: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: keep_query(is_tag(h,"O") || is_tag(h,"Q"),h,queries(t))

def needs_parse(qs: List<&2,List<&2,String>>) -> Bool:
  match qs:
    case Nil{}: False{}
    case +h <> t: is_tag(h,"Q") || String.starts_with(W.field(h,7n),"fact-") || E.text_eq(W.field(h,7n),"json-syntax") || needs_parse(t)

def prepared(parse: Bool,+rs: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> String:
  match parse:
    case True{}: normalize_parallel(rs,qs,False{})
    case False{}: W.row(["REPORT","1","advisory-adoption"]) ++ results(qs,rs,find(rs,"OP"),question_budget(find(rs,"SB")))

def prepare(+rs: List<&2,List<&2,String>>) -> String:
  +qs = queries(rs)
  prepared(needs_parse(qs) && Bool.not(Predicate.any_chunk(rs)),rs,qs)

def validated(ok: Bool, +rs: List<&2, List<&2, String>>) -> String:
  match ok:
    case False{}: W.row(["ERROR","invalid contract or wire record"])
    case True{}: prepare(rs)

def file_order(is_file: Bool,path: String,previous: String) -> Bool:
  match is_file:
    case True{}: E.text_gt(path,previous)
    case False{}: True{}

def ordered(rows: List<&2,List<&2,String>>,+previous: String) -> Bool:
  match rows:
    case Nil{}: True{}
    case +h <> t: file_order(is_tag(h,"F"),W.field(h,1n),previous) && ordered(t,choose_string(is_tag(h,"F"),W.field(h,1n),previous))

def run_rows(input: Input) -> String:
  match input:
    case Input{+rs,+e}: validated(String.is_empty(e) && valid_rows(rs) && ordered(rs,"") && E.text_eq(W.field(find(rs,"UG"),1n),"1") && Bool.not(String.is_empty(W.field(find(rs,"OP"),1n))) && Bool.not(is_tag(find(rs,"CI"),"CI")) && chunks_valid(rs),rs)

def run(input: Input) -> String:
  run_rows(expand_chunked(input))

def index_tags(rows: List<&2,List<&2,String>>) -> Bool:
  match rows:
    case Nil{}: True{}
    case +h <> t: (is_tag(h,"UG") || is_tag(h,"Q") || is_tag(h,"OP") || is_tag(h,"F") || is_tag(h,"X")) && index_tags(t)

def index_validated(ok: Bool,+rows: List<&2,List<&2,String>>) -> String:
  match ok:
    case False{}: "ERROR\tinvalid index frame\n"
    case True{}: normalize_parallel(rows,queries(rows),True{})

def index_run(input: Input) -> String:
  match input:
    case Input{+rows,+error}: index_validated(Bool.not(String.is_empty(W.field(find(rows,"Q"),1n))) && String.is_empty(error) && index_tags(rows) && valid_rows(rows) && ordered(rows,"") && E.text_eq(W.field(find(rows,"UG"),1n),"1") && Bool.not(String.is_empty(W.field(find(rows,"OP"),1n))),rows)

def index_expanded(value: Transport.Expanded,error: String) -> String:
  match value:
    case Transport.Expanded{rows,reason}: index_run(Input{rows,error ++ reason})

def index_interned(input: Input) -> String:
  match input:
    case Input{rows,error}: index_expanded(Transport.expand(rows),error)

# CHUNK: one transport chunk of a chunked check. It reports the chunk's state,
# content bytes, CF row count and parse count, then one CF row (path, digest) per
# F row in order, then one PC row with the chunk Partial per distinct content
# query. A chunk beyond the byte budget is not evaluated and is reported
# omitted, never silently skipped; its CF rows still name its files.
def row_bytes(file: Bool,+r: List<&2,String>) -> U32:
  match file:
    case True{}: F.bytes(W.field(r,3n))
    case False{}: 0

def chunk_bytes(rows: List<&2,List<&2,String>>,+n: U32) -> U32:
  match rows:
    case Nil{}: n
    case +h <> t: chunk_bytes(t,(n + row_bytes(is_tag(h,"F"),h) : U32))

def parsed_count(rows: List<&2,List<&2,String>>,+n: U32) -> U32:
  match rows:
    case Nil{}: n
    case +h <> t: parsed_count(t,(n + Bool.pick(U32,is_tag(h,"P") && E.text_eq(W.field(h,5n),"parsed"),1,0) : U32))

def chunk_query(+q: List<&2,String>) -> Bool:
  is_tag(q,"O") && Predicate.content(Predicate.resolve(W.field(q,7n),W.field(q,8n)))

def chunk_key(+q: List<&2,String>) -> String:
  W.row([W.field(q,7n),W.field(q,5n),W.field(q,8n)])

def chunk_value(evaluated: Bool,+q: List<&2,String>,+rows: List<&2,List<&2,String>>) -> Predicate.Partial:
  match evaluated:
    case True{}: Predicate.partial(Predicate.resolve(W.field(q,7n),W.field(q,8n)),W.field(q,5n),W.field(q,8n),rows)
    case False{}: Predicate.Partial{Predicate.any_member(rows,W.field(q,5n)),False{},True{},True{}}

def witness_line(hit: Bool,+q: List<&2,String>,+rows: List<&2,List<&2,String>>,+chunk: String) -> String:
  match hit:
    case True{}: W.row(List.append(&2,String,["PW",chunk,W.field(q,7n),W.field(q,5n),W.field(q,8n)],Predicate.witness(W.field(q,7n),W.field(q,5n),W.field(q,8n),rows)))
    case False{}: ""

def partial_emitted(+p: Predicate.Partial,+q: List<&2,String>,+rows: List<&2,List<&2,String>>,+chunk: String) -> String:
  W.row(List.append(&2,String,["PC",chunk,W.field(q,7n),W.field(q,5n),W.field(q,8n)],Predicate.bits(p))) ++ witness_line(Predicate.witnessed(Predicate.resolve(W.field(q,7n),W.field(q,8n)),p),q,rows,chunk)

def partial_line(+q: List<&2,String>,+rows: List<&2,List<&2,String>>,+chunk: String,evaluated: Bool) -> String:
  partial_emitted(chunk_value(evaluated,q,rows),q,rows,chunk)

def partial_emit(fresh: Bool,+q: List<&2,String>,+rows: List<&2,List<&2,String>>,+chunk: String,evaluated: Bool,rest: String) -> String:
  match fresh:
    case True{}: partial_line(q,rows,chunk,evaluated) ++ rest
    case False{}: rest

def partial_step(pair: Map<&2,Bool> & Bool,+q: List<&2,String>,+rows: List<&2,List<&2,String>>,+chunk: String,+evaluated: Bool,next: Map<&2,Bool> -> String) -> String:
  match pair:
    case (seen,dup): partial_emit(chunk_query(q) && Bool.not(dup),q,rows,chunk,evaluated,next(Map.set(&2,Bool,seen,chunk_key(q),True{})))

def chunk_partials(qs: List<&2,List<&2,String>>,+rows: List<&2,List<&2,String>>,+chunk: String,+evaluated: Bool,seen: Map<&2,Bool>) -> String:
  match qs:
    case Nil{}: ""
    case +q <> t: partial_step(Map.get(Bool,False{},seen,chunk_key(q)),q,rows,chunk,evaluated,m => chunk_partials(t,rows,chunk,evaluated,m))

def listing_text(rows: List<&2,List<&2,String>>) -> String:
  match rows:
    case Nil{}: ""
    case h <> t: W.row_onto(h,listing_text(t))

def row_count(rows: List<&2,List<&2,String>>,+n: U32) -> U32:
  match rows:
    case Nil{}: n
    case _ <> t: row_count(t,(n + 1 : U32))

def chunk_output(+chunk: String,+evaluated: Bool,+bytes: U32,+rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> String:
  +files = Predicate.listing(rows)
  W.row(["CH",chunk,choose_string(evaluated,"complete","omitted"),choose_string(evaluated,"","chunk content byte bound"),U32.show(bytes),U32.show(row_count(files,0)),U32.show(parsed_count(rows,0))]) ++ listing_text(files) ++ chunk_partials(qs,rows,chunk,evaluated,MTip{})

def chunk_parsed(work: ParseWork.ParseBatch,+chunk: String,+bytes: U32,rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> String:
  match work:
    case ParseWork.ParseBatch{_,False{}}: "ERROR\tconflicting bytes for one declared parse identity\n"
    case ParseWork.ParseBatch{memo,True{}}: chunk_output(chunk,True{},bytes,materialize(rows,qs,memo),qs)

def chunk_prepared(parse: Bool,+chunk: String,+bytes: U32,+rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> String:
  match parse:
    case True{}: chunk_parsed(ParseWork.run(eligible_files(rows,qs),False{}),chunk,bytes,rows,qs)
    case False{}: chunk_output(chunk,True{},bytes,rows,qs)

def chunk_budgeted(within: Bool,+chunk: String,+bytes: U32,+rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> String:
  match within:
    case True{}: chunk_prepared(needs_parse(qs),chunk,bytes,rows,qs)
    case False{}: chunk_output(chunk,False{},bytes,rows,qs)

def chunk_tags(rows: List<&2,List<&2,String>>) -> Bool:
  match rows:
    case Nil{}: True{}
    case +h <> t: (is_tag(h,"UG") || is_tag(h,"O") || is_tag(h,"CI") || is_tag(h,"F")) && chunk_tags(t)

def chunk_validated(ok: Bool,+rows: List<&2,List<&2,String>>) -> String:
  match ok:
    case False{}: "ERROR\tinvalid content chunk\n"
    case True{}:
      +bytes = chunk_bytes(rows,0)
      chunk_budgeted(U32.is_le(bytes,8388608),W.field(find(rows,"CI"),1n),bytes,rows,queries(rows))

def chunk(input: Input) -> String:
  match input:
    case Input{+rows,+error}: chunk_validated(String.is_empty(error) && chunk_tags(rows) && valid_rows(rows) && ordered(rows,"") && E.text_eq(W.field(find(rows,"UG"),1n),"1") && is_tag(find(rows,"CI"),"CI"),rows)

def contract_tags(rs: List<&2,List<&2,String>>) -> Bool:
  match rs:
    case Nil{}: True{}
    case +h <> t: (is_tag(h,"UG") || is_tag(h,"O")) && contract_tags(t)

def count_if(ok: Bool, count: Nat) -> Nat:
  match ok:
    case True{}: 1n+count
    case False{}: count

def count(rs: List<&2,List<&2,String>>, +tag: String) -> Nat:
  match rs:
    case Nil{}: 0n
    case h <> t: count_if(is_tag(h,tag),count(t,tag))

def contract(input: Input) -> String:
  match input:
    case Input{+rs,e}: choose_string(String.is_empty(e) && valid_rows(rs) && contract_tags(rs) && Nat.is_eq(count(rs,"UG"),1n) && Nat.is_gt(count(rs,"O"),0n),"VALID\n","ERROR\tinvalid contract\n")

def direct_path(explicit: Bool,selector: String) -> String:
  match explicit:
    case True{}: String.drop(selector,5n)
    case False{}: selector

def direct_selector(broad: Bool,native: Bool,semantic: Bool,+selector: String) -> String:
  match broad:
    case True{}: "PLAN_NEEDS_MEMBERSHIP\n"
    case False{}: W.row([choose_string(native,"NATIVE_READ_IF_FILE",choose_string(semantic,"SEMANTIC_READ_IF_FILE","READ_IF_FILE")),direct_path(String.starts_with(selector,"file:"),selector)])

def direct_row(read: Bool,+r: List<&2,String>) -> String:
  match read:
    case False{}: ""
    case True{}: direct_selector(E.text_eq(W.field(r,5n),"*") || String.starts_with(W.field(r,5n),"prefix:"),E.text_eq(W.field(r,7n),"native") && E.text_eq(W.field(r,8n),"rust-syntax"),E.text_eq(W.field(r,7n),"semantic"),W.field(r,5n))

def direct_queries(rows: List<&2,List<&2,String>>) -> String:
  match rows:
    case Nil{}: ""
    case +h <> t: direct_row(is_tag(h,"O") && Bool.not(E.text_eq(W.field(h,6n),"membership")),h) ++ direct_queries(t)

def contract_plan_valid(ok: Bool,rows: List<&2,List<&2,String>>) -> String:
  match ok:
    case False{}: "ERROR\tinvalid contract\n"
    case True{}: "VALID\n" ++ direct_queries(rows)

def contract_plan(input: Input) -> String:
  match input:
    case Input{+rs,e}: contract_plan_valid(String.is_empty(e) && valid_rows(rs) && contract_tags(rs) && Nat.is_eq(count(rs,"UG"),1n) && Nat.is_gt(count(rs,"O"),0n),rs)

def summary_states(xs: List<&2,String>, failed: Bool, unknown: Bool) -> String:
  match xs:
    case Nil{}: choose_string(failed,"STATE\tfailed\t1\n",choose_string(unknown,"STATE\tunknown\t2\n","STATE\tverified\t0\n"))
    case "failed" <> t: summary_states(t,True{},unknown)
    case "verified" <> t: summary_states(t,failed,unknown)
    case "" <> t: summary_states(t,failed,unknown)
    case _ <> t: summary_states(t,failed,True{})

def summary(xs: List<&2,String>) -> String:
  match xs:
    case "current" <> t: summary_states(t,False{},False{})
    case _: "STATE\tunknown\t2\n"

def json_record(r: W.Record) -> String:
  match r:
    case W.Row{text <> Nil{}}: choose_string(Json.valid(Json.parse(text)),"JSON\tvalid\n","JSON\tinvalid\n")
    case _: "JSON\tinvalid\n"

def numeric_record(r: W.Record) -> String:
  match r:
    case W.Row{text <> Nil{}}: choose_string(Decimal.validate(text),"NUMERIC\tvalid\n","NUMERIC\tinvalid\n")
    case _: "NUMERIC\tinvalid\n"

def read_needed_row(+r: List<&2,String>,+path: String) -> Bool:
  and_then(is_tag(r,"Q"),u => selected(path,W.field(r,1n))) || and_then(is_tag(r,"O") && Bool.not(E.text_eq(W.field(r,6n),"membership")),u => selected(path,W.field(r,5n)))

def read_needed(rs: List<&2,List<&2,String>>,+path: String) -> Bool:
  match rs:
    case Nil{}: False{}
    case +h <> t: read_needed_row(h,path) || read_needed(t,path)

def read_row(include: Bool,r: List<&2,String>) -> String:
  match include:
    case True{}: W.row(["READ",W.field(r,1n)])
    case False{}: ""

def native_needed(rows: List<&2,List<&2,String>>,+path: String) -> Bool:
  match rows:
    case Nil{}: False{}
    case +h <> t: (is_tag(h,"O") && E.text_eq(W.field(h,7n),"native") && E.text_eq(W.field(h,8n),"rust-syntax") && selected(path,W.field(h,5n))) || native_needed(t,path)

def native_read_row(include: Bool,r: List<&2,String>) -> String:
  match include:
    case True{}: W.row(["NATIVE_READ",W.field(r,1n)])
    case False{}: ""

def native_queries(rows: List<&2,List<&2,String>>) -> List<&2,List<&2,String>>:
  match rows:
    case Nil{}: Nil{}
    case +h <> t: keep_query(is_tag(h,"O") && E.text_eq(W.field(h,7n),"native") && E.text_eq(W.field(h,8n),"rust-syntax"),h,native_queries(t))

def semantic_read_row(include: Bool,r: List<&2,String>) -> String:
  match include:
    case True{}: W.row(["SEMANTIC_READ",W.field(r,1n)])
    case False{}: ""

def read_plan(rows: List<&2,List<&2,String>>,+rs: List<&2,List<&2,String>>,+native: List<&2,List<&2,String>>,+semantic: List<&2,String>) -> String:
  match rows:
    case Nil{}: ""
    case +h <> t: read_row(is_tag(h,"F") && read_needed(rs,W.field(h,1n)),h) ++ native_read_row(is_tag(h,"F") && native_needed(native,W.field(h,1n)),h) ++ semantic_read_row(is_tag(h,"F") && any_selects(semantic,W.field(h,1n)),h) ++ read_plan(t,rs,native,semantic)

def read_plan_queries(rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>) -> String:
  read_plan(rows,qs,native_queries(qs),semantic_scopes(qs))

def plan(input: Input) -> String:
  match input:
    case Input{+rs,e}: choose_string(String.is_empty(e) && valid_rows(rs),read_plan_queries(rs,queries(rs)),"ERROR\tinvalid plan inputs\n")

def revalidate_row(current: Bool,r: W.Record) -> String:
  match r:
    case W.Row{id <> status <> Nil{}}: W.row(["ADMISSION",id,choose_string(current,status,"unknown")])
    case _: "ERROR\tinvalid admission record\n"

def revalidate_rows(rows: List<&2,String>,+current: Bool) -> String:
  match rows:
    case Nil{}: ""
    case "" <> t: revalidate_rows(t,current)
    case h <> t: revalidate_row(current,W.parse(h)) ++ revalidate_rows(t,current)

def revalidate(rows: List<&2,String>) -> String:
  match rows:
    case state <> t: revalidate_rows(t,E.text_eq(state,"current"))
    case _: "ERROR\tmissing current observation\n"

def changed_row(+r: List<&2,String>,+selector: String,+projection: String) -> Bool:
  is_tag(r,"C") && affected(selector,W.field(r,1n)) && (Bool.not(E.text_eq(projection,"membership")) || Bool.not(E.text_eq(W.field(r,2n),"content")))

def changed(rows: List<&2,List<&2,String>>,+selector: String,+projection: String) -> Bool:
  match rows:
    case Nil{}: False{}
    case +h <> t: changed_row(h,selector,projection) || changed(t,selector,projection)

def failed_action(rule: String,+scope: String,+argument: String,+path: String,line: String) -> String:
  match rule:
    case "fact-json-key": "Inspect JSON parser diagnostics and restore required key '" ++ argument ++ "' within " ++ scope ++ "; preserve other keys, then rerun this check."
    case "json-syntax": choose_string(String.is_empty(path),"Correct the JSON grammar within " ++ scope ++ " while preserving required values, then rerun this check.","Correct the JSON grammar in " ++ path ++ ", the first invalid file within " ++ scope ++ ", while preserving required values, then rerun this check.")
    case "fact-json-valid": "Correct the JSON grammar within " ++ scope ++ ", then rerun this check."
    case "contains": "Inspect " ++ scope ++ " for required literal '" ++ argument ++ "'; repair the requirement rather than weaken its check, then rerun."
    case "member": "Restore the required file membership for " ++ scope ++ ", then rerun; review any requirement change separately."
    case "path-member": "Restore the required path membership for " ++ scope ++ ", then rerun."
    case "absent-path": "Inspect the unexpected path " ++ choose_string(String.is_empty(path),"",path ++ " ") ++ "in " ++ scope ++ "; have the native host review any removal or rename before rerunning."
    case "absent-text": choose_string(String.is_empty(path),"Inspect " ++ scope ++ " for forbidden text '" ++ argument ++ "', remove it, then rerun this check.","Remove '" ++ argument ++ "' at " ++ path ++ ":" ++ line ++ ", the first match within " ++ scope ++ " (other selected files may also contain it), then rerun this check.")
    case "native": "Inspect the captured diagnostics for " ++ argument ++ " within " ++ scope ++ "; repair and rerun at the required surface."
    case _: "Inspect the failed declared predicate within " ++ scope ++ ", repair the original requirement, and rerun the same check."

def unresolved_action(rule: String,scope: String,argument: String) -> String:
  match rule:
    case "native": "Obtain a qualified native/CI observation for '" ++ argument ++ "' on the required inputs and surface; imported text alone cannot discharge it."
    case "semantic": "Review the versioned advisory material judgment and unresolved support for " ++ scope ++ "; only a qualified policy or acceptance boundary can resolve the required assurance."
    case _: "Resolve unavailable input, coverage or required assurance for " ++ scope ++ " before rerunning; missing evidence is not absence."

def current_action(state: String,rule: String,scope: String,argument: String,path: String,line: String) -> String:
  match state:
    case "verified": "No repair is required for this predicate; continue with remaining obligations."
    case "failed": failed_action(rule,scope,argument,path,line)
    case "needs-review": "Inspect the conflicting available observations for " ++ scope ++ " before another verification attempt."
    case _: unresolved_action(rule,scope,argument)

def admission_detail(current: Bool,changed: Bool,+r: List<&2,String>,+state: String) -> String:
  match current changed:
    case False{} _: W.row(["ADMISSION",W.field(r,1n),state,"Re-read the current contract and baseline requirements before rerunning; do not reuse a prior success.","contract-or-baseline-changed"])
    case True{} True{}: W.row(["ADMISSION",W.field(r,1n),state,"Capture current inputs for " ++ W.field(r,3n) ++ " and rerun the same predicate; preserve unrelated work.","selected-input-changed-or-unavailable"])
    case _ _: W.row(["ADMISSION",W.field(r,1n),state,current_action(state,W.field(r,5n),W.field(r,3n),W.field(r,6n),W.field(r,8n),W.field(r,9n)),choose_string(E.text_eq(state,"verified"),"declared-predicate-supported",choose_string(E.text_eq(state,"failed"),"declared-predicate-failed","required-surface-unresolved"))])

def scoped_result(extended: Bool,+r: List<&2,String>,+current: Bool,+change: Bool) -> String:
  match extended:
    case True{}: admission_detail(current,change,r,choose_string(current && Bool.not(change),W.field(r,2n),"unknown"))
    case False{}: W.row(["ADMISSION",W.field(r,1n),choose_string(current && Bool.not(change),W.field(r,2n),"unknown")])

def scoped_admission(+r: List<&2,String>,rows: List<&2,List<&2,String>>,current: Bool) -> String:
  scoped_result(Nat.is_eq(W.length(r),10n),r,current,changed(rows,W.field(r,3n),W.field(r,4n)))

def scoped_row(ok: Bool,r: List<&2,String>,rows: List<&2,List<&2,String>>,current: Bool) -> String:
  match ok:
    case True{}: scoped_admission(r,rows,current)
    case False{}: ""

def scoped_rows(xs: List<&2,List<&2,String>>,+rows: List<&2,List<&2,String>>,+current: Bool) -> String:
  match xs:
    case Nil{}: ""
    case +h <> t: scoped_row(is_tag(h,"R"),h,rows,current) ++ scoped_rows(t,rows,current)

def scoped_input(input: Input,current: Bool) -> String:
  match input:
    case Input{+rs,e}: scoped_rows(rs,rs,current && String.is_empty(e))

def scoped(lines: List<&2,String>) -> String:
  match lines:
    case state <> t: scoped_input(parse_rows(t),E.text_eq(state,"current"))
    case _: "ERROR\tmissing current contract\n"

def same_obligation(+old: List<&2,String>,rows: List<&2,List<&2,String>>) -> Bool:
  match rows:
    case Nil{}: False{}
    case +h <> t: E.text_eq(W.row(old),W.row(h)) || same_obligation(old,t)

def preserved(before: List<&2,List<&2,String>>,+after: List<&2,List<&2,String>>) -> Bool:
  match before:
    case Nil{}: True{}
    case h <> t: same_obligation(h,after) && preserved(t,after)

def compare_inputs(before: Input,after: Input) -> String:
  match before after:
    case Input{+a,ae} Input{+b,be}: choose_string(String.is_empty(ae) && String.is_empty(be) && valid_rows(a) && valid_rows(b) && contract_tags(a) && contract_tags(b) && preserved(queries(a),queries(b)),"BASELINE\tpreserved\n","BASELINE\tchanged-or-invalid\n")

def compare_contracts(r: W.Record) -> String:
  match r:
    case W.Row{before <> after <> Nil{}}: compare_inputs(parse_rows(String.lines(before)),parse_rows(String.lines(after)))
    case _: "BASELINE\tchanged-or-invalid\n"

def disclosure_check(record: W.Record) -> String:
  match record:
    case W.Row{path <> content <> Nil{}}: choose_string(Policy.disclosable(path,content),"DISCLOSURE\teligible\n","DISCLOSURE\trefused\n")
    case _: "DISCLOSURE\trefused\n"

# Long compound string patterns caused large native decision trees. Resolve
# the wire command through data, then dispatch on a bounded machine integer.
def query_input(input: Input) -> List<&2,List<&2,String>>:
  match input:
    case Input{rows,_}: queries(rows)

def queries_rows(text: String) -> List<&2,List<&2,String>>:
  query_input(parse_rows(String.lines(text)))

def fit_validated(valid: Bool,preserved: Bool,+before: String,+after: String,exists: Bool) -> String:
  match valid preserved:
    case True{} True{}: Fit.contract_proposal(E.text_eq(before,after),before,after,exists)
    case False{} _: W.row(["FIT","invalid-contract","","","","Current or supplied contract grammar is invalid; preserve the existing file."])
    case _ False{}: W.row(["FIT","requirement-change","","","","The proposed contract removes or changes a known requirement. Use an explicitly authorized adoption/migration boundary; no patch emitted."])

def fit_existing(state: String,+before: String,+after: String) -> String:
  match state:
    case "absent": fit_validated(String.is_empty(before) && E.text_eq(contract(parse_rows(String.lines(after))),"VALID\n"),True{},before,after,False{})
    case "existing": fit_validated(E.text_eq(contract(parse_rows(String.lines(before))),"VALID\n") && E.text_eq(contract(parse_rows(String.lines(after))),"VALID\n"),preserved(queries_rows(before),queries_rows(after)),before,after,True{})
    case _: W.row(["FIT","unavailable-target","","","","Target custody is unavailable; do not overwrite or follow a linked target."])

def fit_contract(row: W.Record) -> String:
  match row:
    case W.Row{state <> before <> after <> Nil{}}: fit_existing(state,before,after)
    case _: W.row(["FIT","invalid-proposal","","","","Malformed fit proposal input; no patch emitted."])

def command_match(equal: Bool,index: U32,otherwise: Unit -> U32) -> U32:
  match equal:
    case True{}: index
    case False{}: otherwise(Unit{})

def command_index(names: List<&2,String>,+name: String,+index: U32) -> U32:
  match names:
    case Nil{}: 0
    case h <> t: command_match(E.text_eq(h,name),index,u => command_index(t,name,(index + 1 : U32)))

def commands() -> List<&2,String>:
  ["INDEX","INDEX_INTERNED","JEV","CONTRACT","CONTRACT_PLAN","SUMMARY","ADMIT_REQUEST","ADVICE_PLAN","ADVICE_RANK","DISCLOSURE_CHECK","RETRY","NATIVE_PLAN","NATIVE","JSON","PLAN","FIT","REVALIDATE","REVALIDATE_SCOPED","JOURNAL","BASELINE","INDEX_SUMMARY","SEMANTIC_RUBRIC","SEMANTIC_PROBES","SEMANTIC_CONTEXT","SELECT_STATUS","NUMERIC","EXCLUSIONS","PROOF_PLAN","PROOF_RESULT","PROOF_SOURCE","VERIFIER_PLAN","VERIFIER_OBSERVE","FIT_PATHS","FIT_CONTRACT","HASH","JEV_LAYOUTS","WINDOW_PLAN","SEMANTIC_CALIBRATED","JEV_WINDOW","JEV_WINDOW_MERGE","ADVICE_ORDER","SECRET_SCREEN","RELEVANCE_DECISIONS","SEMANTIC_BUDGET","CHUNK","SEMANTIC_ADMIT","SELECT_MATCH","FILE_RANK","SELECT_PASSAGE","SELECT_FUSE","SELECT_ALIASES"]

def dispatch_command(index: U32,header: String,tail: List<&2,String>) -> String:
  match index:
    case 1: index_run(parse_rows(tail))
    case 2: index_interned(parse_rows(tail))
    case 3: J.process(tail)
    case 4: contract(parse_rows(tail))
    case 5: contract_plan(parse_rows(tail))
    case 6: summary(tail)
    case 7: Policy.provider(Policy.first(tail))
    case 8: Advice.plan(Policy.first(tail))
    case 9: Advice.rank(tail)
    case 10: disclosure_check(Policy.first(tail))
    case 11: Policy.retry(Policy.first(tail))
    case 12: Policy.native_plan(Policy.first(tail))
    case 13: Policy.native(Policy.first(tail))
    case 14: json_record(Policy.first(tail))
    case 15: plan(parse_rows(tail))
    case 16: Fit.process(tail)
    case 17: revalidate(tail)
    case 18: scoped(tail)
    case 19: Lifecycle.process(tail)
    case 20: compare_contracts(Policy.first(tail))
    case 21: choose_string(E.text_eq(W.field(tail,0n),"current"),"STATE\tindexed\t0\n","STATE\tunknown\t2\n")
    case 22: Semantic.record(Policy.first(tail))
    case 23: Semantic.probes(Policy.first(tail))
    case 24: Semantic.validate(Policy.first(tail))
    case 25: Select.status(tail,False{},False{})
    case 26: numeric_record(Policy.first(tail))
    case 27: Policy.exclusions()
    case 28: Policy.proof_plan(Policy.first(tail))
    case 29: Policy.proof_result(Policy.first(tail))
    case 30: Policy.proof_source(Policy.first(tail))
    case 31: NativeRecord.plan(Policy.first(tail))
    case 32: NativeRecord.observe(Policy.first(tail))
    case 33: Fit.paths()
    case 34: fit_contract(Policy.first(tail))
    case 35: Digest.process(Policy.first(tail))
    case 36: JevLayout.process(tail)
    case 37: ExcerptPlan.process(tail)
    case 38: Semantic.calibrated_record(Policy.first(tail))
    case 39: JevWindow.process(tail)
    case 40: JevWindow.merge(tail)
    case 41: Advice.order(tail)
    case 42: Policy.secret_screen(Policy.first(tail))
    case 43: Semantic.relevance_decisions(tail)
    case 44: Policy.semantic_budget(Policy.first(tail))
    case 45: chunk(parse_rows(tail))
    case 46: Semantic.admissions(tail)
    case 47: Retrieval.match_process(tail)
    case 48: Retrieval.rank_process(tail)
    case 49: Retrieval.passage_process(tail)
    case 50: Retrieval.fuse_process(tail)
    case 51: Retrieval.alias_process(tail)
    case _: run(parse_rows(header <> tail))

def process_lines(lines: List<&2,String>) -> String:
  match lines:
    case +header <> tail: dispatch_command(command_index(commands(),header,1),header,tail)
    case Nil{}: run(parse_rows(Nil{}))

def process(text: String) -> String:
  process_lines(String.lines(text))

def decoded(input: Maybe<&2,String>) -> IO(Unit):
  match input:
    case Some{text}: IO.write(process(text))
    case None{}: IO.die(Unit,3,"invalid UTF-8 frame")

def read_step(follow: File -> List<&2,List<&2,U32>> -> IO(Unit),chunks: List<&2,List<&2,U32>>,read: File & Result<&1,&1,U32 & String,List<&2,U32>>) -> IO(Unit):
  match read:
    case (f,Fail{(code,message)}): IO.bind(Unit,Unit,File.close(f),u => IO.die(Unit,code,message))
    case (f,Done{Nil{}}): IO.bind(Unit,Unit,File.close(f),u => decoded(U.decode_all(List.reverse(&2,List<&2,U32>,chunks))))
    case (f,Done{bytes}): follow(f,bytes <> chunks)

def read_loop(fuel: Nat) -> File -> List<&2,List<&2,U32>> -> IO(Unit):
  match fuel:
    case 0n: file => chunks => IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,"frame read limit"))
    case 1n+p:
      file => chunks => IO.bind(File & Result<&1,&1,U32 & String,List<&2,U32>>,Unit,File.read_bytes(file,65536),read_step(read_loop(p),chunks))

def fail(file: File,message: String) -> IO(File & Batch):
  IO.bind(Unit,File & Batch,File.close(file),u => IO.die(File & Batch,3,message))

def accepted(ok: Bool,next: File -> List<&2,FrameIO.Frame> -> U32 -> U32 -> IO(File & Batch),file: File,frames: List<&2,FrameIO.Frame>,frame: FrameIO.Frame,count: U32,bytes: U32) -> IO(File & Batch):
  match ok:
    case False{}: fail(file,"stream count or total byte limit")
    case True{}: next(file,frame <> frames,count,bytes)

def step(next: File -> List<&2,FrameIO.Frame> -> U32 -> U32 -> IO(File & Batch),frames: List<&2,FrameIO.Frame>,+count: U32,+bytes: U32,result: File & FrameIO.Frame) -> IO(File & Batch):
  match result:
    case (file,FrameIO.End{}): IO.pure(File & Batch,(file,Batch{List.reverse(&2,FrameIO.Frame,frames),True{},count,bytes}))
    case (file,FrameIO.Payload{+size,chunks}): accepted(U32.is_lt(count,128) && U32.is_le((bytes + size : U32),536870912),next,file,frames,FrameIO.Payload{size,chunks},(count + 1 : U32),(bytes + size : U32))

def collect(fuel: Nat) -> File -> List<&2,FrameIO.Frame> -> U32 -> U32 -> IO(File & Batch):
  match fuel:
    case 0n: file => frames => count => bytes => IO.pure(File & Batch,(file,Batch{List.reverse(&2,FrameIO.Frame,frames),False{},count,bytes}))
    case 1n+p: file => frames => count => bytes => IO.bind(File & FrameIO.Frame,File & Batch,FrameIO.read(file),step(collect(p),frames,count,bytes))

def stream_decoded(input: Maybe<&2,String>) -> Maybe<&2,String>:
  match input:
    case None{}: None{}
    case Some{text}: Some{process(text)}

def one(frame: FrameIO.Frame) -> Maybe<&2,String>:
  match frame:
    case FrameIO.End{}: None{}
    case FrameIO.Payload{_,chunks}: stream_decoded(U.decode_all(chunks))

def combine(a: Maybe<&2,String>,b: Maybe<&2,String>) -> Maybe<&2,String>:
  match a b:
    case Some{x} Some{y}: Some{x ++ y}
    case _ _: None{}

def split(n: Nat,frames: List<&2,FrameIO.Frame>,left: List<&2,FrameIO.Frame>) -> List<&2,FrameIO.Frame> & List<&2,FrameIO.Frame>:
  match n frames:
    case 1n+p h <> t: split(p,t,h <> left)
    case _ rest: (List.reverse(&2,FrameIO.Frame,left),rest)

def branches(left: List<&2,FrameIO.Frame> -> Maybe<&2,String>,right: List<&2,FrameIO.Frame> -> Maybe<&2,String>,parts: List<&2,FrameIO.Frame> & List<&2,FrameIO.Frame>) -> Maybe<&2,String>:
  match parts:
    case (xs,ys):
      a b = left(xs) right(ys)
      combine(a,b)

def parallel(depth: Nat,frames: List<&2,FrameIO.Frame>,+half: Nat) -> Maybe<&2,String>:
  match depth:
    case 0n:
      match frames:
        case Nil{}: Some{""}
        case h <> Nil{}: one(h)
        case _: None{}
    case 1n+ +p: branches(xs => parallel(p,xs,Nat.div(half,2n)),xs => parallel(p,xs,Nat.div(half,2n)),split(half,frames,Nil{}))

def emitted(ended: Bool,next: File -> U32 -> U32 -> IO(Unit),file: File,count: U32,bytes: U32) -> IO(Unit):
  match ended:
    case True{}: File.close(file)
    case False{}: next(file,count,bytes)

def output(result: Maybe<&2,String>,ended: Bool,next: File -> U32 -> U32 -> IO(Unit),file: File,count: U32,bytes: U32) -> IO(Unit):
  match result:
    case None{}: IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,"invalid UTF-8 stream frame"))
    case Some{text}: IO.bind(Unit,Unit,IO.write(text),u => emitted(ended,next,file,count,bytes))

def batch(next: File -> U32 -> U32 -> IO(Unit),result: File & Batch) -> IO(Unit):
  match result:
    case (file,Batch{frames,ended,count,bytes}): output(parallel(3n,frames,4n),ended,next,file,count,bytes)

def loop(fuel: Nat) -> File -> U32 -> U32 -> IO(Unit):
  match fuel:
    case 0n: file => count => bytes => IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,"stream batch limit"))
    case 1n+p: file => count => bytes => IO.bind(File & Batch,Unit,collect(8n,file,Nil{},count,bytes),batch(loop(p)))

def stream_run(file: File) -> IO(Unit):
  loop(17n,file,0,0)

def graph_status(text: String) -> C.Status:
  match text:
    case "verified": C.Verified{}
    case "failed": C.Failed{}
    case "needs-review": C.Review{}
    case _: C.Unknown{}

def graph_result(include: Bool,+r: List<&2,String>,rows: List<&2,List<&2,String>>,op: List<&2,String>,budget: Nat,+state: QG.State) -> String:
  match include:
    case True{}: result_computed(graph_status(QG.status(QP.query_id(r),state)),choose_string(QG.reused(QP.query_id(r),state),"reused","computed"),r,rows,op,budget)
    case False{}: ""

def graph_results(qs: List<&2,List<&2,String>>,+rows: List<&2,List<&2,String>>,+op: List<&2,String>,+budget: Nat,+state: QG.State) -> String:
  match qs:
    case Nil{}: ""
    case +r <> t: graph_result(is_tag(r,"O"),r,rows,op,budget,state) ++ graph_results(t,rows,op,consume_question(is_tag(r,"O") && E.text_eq(W.field(r,7n),"semantic"),budget),state)

def graph_dirty_row(include: Bool,id: String) -> String:
  match include:
    case True{}: W.row(["GRAPH_DIRTY",id])
    case False{}: ""

def graph_dirty(nodes: List<&2,QG.Node>,+dirty: Map<&2,Bool>) -> String:
  match nodes:
    case Nil{}: ""
    case +h <> t: graph_dirty_row(Invalidation.flag(Map.get(Bool,False{},dirty,QG.id(h))),QG.id(h)) ++ graph_dirty(t,dirty)

def graph_local(ids: List<&2,String>) -> String:
  match ids:
    case Nil{}: ""
    case h <> t: W.row(["GRAPH_LOCAL",h]) ++ graph_local(t)

def graph_trace(nodes: List<&2,QG.Node>,plan: Invalidation.Plan) -> String:
  match plan:
    case Invalidation.Plan{_,local,dirty,complete}: W.row(["GRAPH_INVALIDATION",choose_string(complete,"complete","conservative-full-recompute")]) ++ graph_local(local) ++ graph_dirty(nodes,dirty)

def graph_ready(work: QG.RevisionWork,parses: ParseCache.State,nodes: List<&2,QG.Node>,+rows: List<&2,List<&2,String>>,qs: List<&2,List<&2,String>>) -> Processed:
  match work:
    case QG.RevisionWork{+state,plan}: Processed{W.row(["REPORT","1","advisory-adoption"]) ++ graph_results(qs,rows,find(rows,"OP"),question_budget(find(rows,"SB")),state) ++ emit_facts(rows) ++ graph_trace(nodes,plan),SessionState{state,parses}}

def graph_planned(plan: QP.Plan,state: SessionState,rows: List<&2,List<&2,String>>,qs: List<&2,List<&2,String>>) -> Processed:
  match plan state:
    case QP.Plan{+nodes,_,_,_,""} SessionState{graph,parses}: graph_ready(QG.run_work(nodes,graph),parses,nodes,rows,qs)
    case QP.Plan{_,_,_,_,error} _: Processed{W.row(["ERROR",error]),empty_session()}

def graph_evaluate(+rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>,state: SessionState) -> Processed:
  graph_planned(QP.build(qs,rows),state,rows,qs)

def graph_parsed(work: ParseCache.Work,rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>,state: QG.State) -> Processed:
  match work:
    case ParseCache.Work{_,_,False{}}: Processed{"ERROR\tconflicting bytes for one declared parse identity\n",empty_session()}
    case ParseCache.Work{bounds,parses,True{}}: graph_evaluate(materialize(rows,qs,GroupedParse.materialize(bounds,MTip{})),qs,SessionState{state,parses})

def graph_prepared(parse: Bool,+rows: List<&2,List<&2,String>>,+qs: List<&2,List<&2,String>>,state: SessionState) -> Processed:
  match parse state:
    case True{} SessionState{graph,parses}: graph_parsed(ParseCache.run(eligible_files(rows,qs),False{},parses),rows,qs,graph)
    case False{} state: graph_evaluate(rows,qs,state)

def graph_validated(ok: Bool,+rows: List<&2,List<&2,String>>,state: SessionState) -> Processed:
  match ok:
    case True{}: graph_prepared(needs_parse(queries(rows)) && Bool.not(Predicate.any_chunk(rows)),rows,queries(rows),state)
    case False{}: Processed{"ERROR\tinvalid contract or wire record\n",empty_session()}

def graph_rows(input: Input,state: SessionState) -> Processed:
  match input:
    case Input{+rows,+error}: graph_validated(String.is_empty(error) && valid_rows(rows) && ordered(rows,"") && E.text_eq(W.field(find(rows,"UG"),1n),"1") && Bool.not(String.is_empty(W.field(find(rows,"OP"),1n))) && Bool.not(is_tag(find(rows,"CI"),"CI")) && chunks_valid(rows),rows,state)

def graph_input(input: Input,state: SessionState) -> Processed:
  graph_rows(expand_chunked(input),state)

def ordinary_state_if(error: Bool,text: String,state: SessionState) -> Processed:
  match error:
    case True{}: Processed{text,empty_session()}
    case False{}: Processed{text,state}

def ordinary_state(+text: String,state: SessionState) -> Processed:
  ordinary_state_if(String.starts_with(text,"ERROR\t"),text,state)

def cache_lines(lines: List<&2,String>) -> List<&2,Cached>:
  match lines:
    case Nil{}: Nil{}
    case +h <> t: Cached{h,W.parse(h)} <> cache_lines(t)

# Same rows and first error as parse_rows over the cached lines.
def cached_input(xs: List<&2,Cached>) -> Input:
  match xs:
    case Nil{}: Input{Nil{},""}
    case Cached{"",_} <> t: cached_input(t)
    case Cached{_,r} <> t: add_row(r,cached_input(t))

def evaluated(result: Processed,+sequence: U32,+lines: List<&2,Cached>) -> Step:
  match result:
    case Processed{text,state}: Step{text,Retained{state,Base{sequence,lines},Segment.empty(),Rank.empty()}}

def evaluate_cached(+lines: List<&2,Cached>,+sequence: U32,state: SessionState) -> Step:
  evaluated(graph_input(cached_input(lines),state),sequence,lines)

def repeat_op(n: Nat,+op: DeltaOp,rest: List<&2,DeltaOp>) -> List<&2,DeltaOp>:
  match n:
    case 0n: rest
    case 1n+p: op <> repeat_op(p,op,rest)

def choose_ops(ok: Bool,n: U32,op: DeltaOp,tail: List<&2,DeltaOp>) -> Maybe<&2,List<&2,DeltaOp>>:
  match ok:
    case True{}: Some{repeat_op(U32.to_nat(n),op,tail)}
    case False{}: None{}

def counted(count: Maybe<&2,U32>,+raw: String,op: DeltaOp,rest: Maybe<&2,List<&2,DeltaOp>>) -> Maybe<&2,List<&2,DeltaOp>>:
  match count rest:
    case Some{+n} Some{tail}: choose_ops(U32.is_ge(n,1) && U32.is_le(n,16777216) && E.text_eq(raw,U32.show(n)),n,op,tail)
    case _ _: None{}

def literal_op(line: String,rest: Maybe<&2,List<&2,DeltaOp>>) -> Maybe<&2,List<&2,DeltaOp>>:
  match rest:
    case Some{tail}: Some{Literal{line} <> tail}
    case None{}: None{}

def op_line(line: String,rest: Maybe<&2,List<&2,DeltaOp>>) -> Maybe<&2,List<&2,DeltaOp>>:
  match line:
    case "": rest
    case SCon{'K',SCon{'\t',+n}}: counted(U32.read(n),n,Keep{},rest)
    case SCon{'D',SCon{'\t',+n}}: counted(U32.read(n),n,Skip{},rest)
    case SCon{'L',SCon{'\t',text}}: literal_op(text,rest)
    case _: None{}

def delta_ops(lines: List<&2,String>) -> Maybe<&2,List<&2,DeltaOp>>:
  match lines:
    case Nil{}: Some{Nil{}}
    case h <> t: op_line(h,delta_ops(t))

# Rebuilds a frame from retained lines: every base line is kept or skipped in
# order and literal lines are parsed; the base must be consumed exactly.
def apply_ops(ops: List<&2,DeltaOp>,base: List<&2,Cached>,acc: List<&2,Cached>) -> Maybe<&2,List<&2,Cached>>:
  match ops base:
    case Nil{} Nil{}: Some{List.reverse(&2,Cached,acc)}
    case Keep{} <> rest c <> bt: apply_ops(rest,bt,c <> acc)
    case Skip{} <> rest c <> bt: apply_ops(rest,bt,acc)
    case Literal{+line} <> rest base: apply_ops(rest,base,Cached{line,W.parse(line)} <> acc)
    case _ _: None{}

def rebuilt(ops: Maybe<&2,List<&2,DeltaOp>>,base: List<&2,Cached>) -> Maybe<&2,List<&2,Cached>>:
  match ops:
    case Some{xs}: apply_ops(xs,base,Nil{})
    case None{}: None{}

def delta_applied(lines: Maybe<&2,List<&2,Cached>>,+sequence: U32,retained: Retained) -> Step:
  match lines retained:
    case Some{+xs} Retained{state,_,_,_}: evaluate_cached(xs,sequence,state)
    case None{} retained: Step{"ERROR\tdelta base unavailable\n",retained}

def delta_base(same: Bool,ops: List<&2,String>,+sequence: U32,retained: Retained) -> Step:
  match same retained:
    case True{} Retained{state,Base{seq,+base},segment,rank}: delta_applied(rebuilt(delta_ops(ops),base),sequence,Retained{state,Base{seq,base},segment,rank})
    case _ retained: Step{"ERROR\tdelta base unavailable\n",retained}

def base_seq(retained: Retained) -> Maybe<&2,U32>:
  match retained:
    case Retained{_,Base{+seq,_},_,_}: Some{seq}
    case Retained{_,NoBase{},_,_}: None{}

def same_base(expected: Maybe<&2,U32>,raw: String) -> Bool:
  match expected:
    case Some{+seq}: E.text_eq(raw,U32.show(seq))
    case None{}: False{}

def delta_frame(header: W.Record,ops: List<&2,String>,+sequence: U32,+retained: Retained) -> Step:
  match header:
    case W.Row{"UG_DELTA" <> raw <> Nil{}}: delta_base(same_base(base_seq(retained),raw),ops,sequence,retained)
    case _: Step{"ERROR\tdelta base unavailable\n",retained}

def ordinary_step(result: Processed,base: Base,segment: Segment.State,rank: Rank.State) -> Step:
  match result:
    case Processed{text,state}: Step{text,Retained{state,base,segment,rank}}

def session_kind(full: Bool,delta: Bool,lines: List<&2,String>,+sequence: U32,retained: Retained) -> Step:
  match full delta lines retained:
    case True{} _ lines Retained{state,_,_,_}: evaluate_cached(cache_lines(lines),sequence,state)
    case False{} True{} h <> t retained: delta_frame(W.parse(h),t,sequence,retained)
    case False{} _ lines Retained{state,base,segment,rank}: ordinary_step(ordinary_state(process(String.join(lines,"\n")),state),base,segment,rank)

def segment_step(result: Segment.Outcome,state: SessionState,base: Base,rank: Rank.State) -> Step:
  match result:
    case Segment.Outcome{segment,reply}: Step{reply,Retained{state,base,segment,rank}}

def segment_lines(lines: List<&2,String>,retained: Retained) -> Step:
  match retained:
    case Retained{state,base,segment,rank}: segment_step(Segment.process_lines(lines,segment),state,base,rank)

type SegmentMode is Data:
  SegEval{}
  SegIndex{}
  SegChunk{}

def segment_chunk_checked(ok: Bool,+rows: List<&2,List<&2,String>>) -> String:
  match ok:
    case True{}: chunk_prepared(needs_parse(queries(rows)),W.field(find(rows,"CI"),1n),chunk_bytes(rows,0),rows,queries(rows))
    case False{}: "ERROR\tinvalid segmented content chunk\n"

def segment_chunk(input: Input) -> String:
  match input:
    case Input{+rows,+error}:
      segment_chunk_checked(String.is_empty(error) && chunk_tags(rows) && valid_rows(rows) && ordered(rows,"") && E.text_eq(W.field(find(rows,"UG"),1n),"1") && is_tag(find(rows,"CI"),"CI"),rows)

def segment_assembled_mode(mode: SegmentMode,input: Input) -> String:
  match mode:
    case SegIndex{}: index_run(input)
    case SegEval{}: run(input)
    case SegChunk{}: segment_chunk(input)

def segment_combined(before: Input,after: Input,path: String,digest: String,source: String) -> Input:
  match before after:
    case Input{left,first} Input{right,second}: Input{List.append(&2,List<&2,String>,left,["F",path,digest,source] <> right),first ++ second}

def segment_assembled(mode: SegmentMode,prefix: String,suffix: String,path: String,digest: String,source: String) -> String:
  segment_assembled_mode(mode,segment_combined(parse_rows(W.lines(prefix)),parse_rows(W.lines(suffix)),path,digest,source))

def segment_prefix_op(input: Input) -> String:
  match input:
    case Input{rows,_}: W.field(find(rows,"OP"),1n)

def segment_op(prefix: String) -> String:
  segment_prefix_op(parse_rows(W.lines(prefix)))

def segment_prefix_matches(mode: SegmentMode,prefix: String,op: String) -> Bool:
  match mode:
    case SegChunk{}: True{}
    case _: String.eq(segment_op(prefix),op)

def segment_eval_ok(ok: Bool,mode: SegmentMode,prefix: String,suffix: String,path: String,digest: String,source: String,state: SessionState,base: Base,rank: Rank.State) -> Step:
  match ok:
    case True{}: Step{segment_assembled(mode,prefix,suffix,path,digest,source),Retained{state,base,Segment.empty(),rank}}
    case False{}: Step{"SEG_ERROR\tstale-or-unbound-evaluation\n",Retained{state,base,Segment.empty(),rank}}

def segment_eval_ready(+mode: SegmentMode,record: W.Record,segment: Segment.State,state: SessionState,base: Base,rank: Rank.State) -> Step:
  match record segment:
    case W.Row{tag <> +op <> +path <> +version <> +digest <> +prefix <> suffix <> Nil{}} Segment.Ready{held_op,held_path,held_version,held_digest,source}:
      segment_eval_ok(String.eq(op,held_op) && String.eq(path,held_path) && String.eq(version,held_version) && String.eq(digest,held_digest) && segment_prefix_matches(mode,prefix,op),mode,prefix,suffix,path,digest,source,state,base,rank)
    case _ _: Step{"SEG_ERROR\tinvalid-evaluation-command\n",Retained{state,base,Segment.empty(),rank}}

def segment_eval_lines(mode: SegmentMode,lines: List<&2,String>,retained: Retained) -> Step:
  match lines retained:
    case line <> Nil{} Retained{state,base,segment,rank}: segment_eval_ready(mode,W.parse(line),segment,state,base,rank)
    case line <> "" <> Nil{} Retained{state,base,segment,rank}: segment_eval_ready(mode,W.parse(line),segment,state,base,rank)
    case _ Retained{state,base,_,rank}: Step{"SEG_ERROR\tinvalid-evaluation-envelope\n",Retained{state,base,Segment.empty(),rank}}

def segment_kind(evaluate: Bool,index: Bool,chunk: Bool,lines: List<&2,String>,retained: Retained) -> Step:
  match evaluate index chunk:
    case True{} _ _: segment_eval_lines(SegEval{},lines,retained)
    case False{} True{} _: segment_eval_lines(SegIndex{},lines,retained)
    case False{} False{} True{}: segment_eval_lines(SegChunk{},lines,retained)
    case False{} False{} False{}: segment_lines(lines,retained)

def session_route(segment: Bool,+lines: List<&2,String>,+sequence: U32,retained: Retained) -> Step:
  match segment:
    case True{}: segment_kind(String.starts_with(W.field(lines,0n),"SEG_EVAL\t"),String.starts_with(W.field(lines,0n),"SEG_INDEX\t"),String.starts_with(W.field(lines,0n),"SEG_CHUNK\t"),lines,retained)
    case False{}: session_kind(String.starts_with(W.field(lines,0n),"UG\t"),String.starts_with(W.field(lines,0n),"UG_DELTA\t"),lines,sequence,retained)

def check_rank_ready(operation: String,input: Input,state: SessionState,base: Base,segment: Segment.State) -> Step:
  match input:
    case Input{+rs,e}: Step{choose_string(String.eq(operation,W.field(find(rs,"OP"),1n)),run(Input{rs,e}),"CHECK_ERROR\toperation-mismatch\n"),Retained{state,base,segment,Rank.empty()}}

def rank_step(result: Rank.Outcome,state: SessionState,base: Base,segment: Segment.State) -> Step:
  match result:
    case Rank.Outcome{Rank.CheckReady{operation,rows},_}: check_rank_ready(operation,parse_rows(rows),state,base,segment)
    case Rank.Outcome{rank,reply}: Step{reply,Retained{state,base,segment,rank}}

def rank_lines(lines: List<&2,String>,retained: Retained) -> Step:
  match retained:
    case Retained{state,base,segment,rank}: rank_step(Rank.process(lines,rank),state,base,segment)

def session_top(rank: Bool,+lines: List<&2,String>,+sequence: U32,retained: Retained) -> Step:
  match rank:
    case True{}: rank_lines(lines,retained)
    case False{}: session_route(String.starts_with(W.field(lines,0n),"SEG_"),lines,sequence,retained)

# The lines come from a split on newline, so an evaluation frame is parsed from
# them directly instead of joining and splitting the request text again.
def session_lines(+lines: List<&2,String>,+sequence: U32,retained: Retained) -> Step:
  session_top(String.starts_with(W.field(lines,0n),"RANK_") || String.starts_with(W.field(lines,0n),"ADV_") || String.starts_with(W.field(lines,0n),"CHECK_"),lines,sequence,retained)

def session_emit(sequence: U32,done: Bool,+text: String,next: File -> IO(Unit),file: File) -> IO(Unit):
  do IO<Unit>:
    u : Unit <- IO.write(W.row(["S1",U32.show(sequence),choose_string(done,"done","more"),U32.show(F.bytes(text))]))
    u : Unit <- IO.write(text)
    next(file)

def session_processed(result: Step,sequence: U32,next: Retained -> File -> IO(Unit),file: File) -> IO(Unit):
  match result:
    case Step{text,retained}: session_emit(sequence,True{},text,next(retained),file)

def session_fail(file: File,message: String) -> IO(Unit):
  IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,message))

def empty_lines(lines: List<&2,String>) -> Bool:
  match lines:
    case Nil{}: True{}
    case "" <> t: empty_lines(t)
    case _: False{}

def session_index_count(ok: Bool,count: U32) -> SessionCommand:
  match ok:
    case True{}: Index{U32.to_nat(count)}
    case False{}: InvalidCommand{"invalid session index count or trailing control data"}

def session_index_number(value: Maybe<&2,U32>,raw: String,valid: Bool) -> SessionCommand:
  match value:
    case Some{+n}: session_index_count(valid && U32.is_ge(n,1) && U32.is_le(n,128) && E.text_eq(raw,U32.show(n)),n)
    case None{}: InvalidCommand{"invalid session index count"}

def session_single(ok: Bool,lines: List<&2,String>) -> SessionCommand:
  match ok:
    case True{}: Single{lines}
    case False{}: InvalidCommand{"session sequence mismatch"}

def session_control(expected: U32,header: W.Record,lines: List<&2,String>) -> SessionCommand:
  match header:
    case W.Row{"S1" <> sequence <> "ONE" <> Nil{}}: session_single(E.text_eq(sequence,U32.show(expected)),lines)
    case W.Row{"S1" <> sequence <> "INDEX" <> +count <> Nil{}}: session_index_number(U32.read(count),count,E.text_eq(sequence,U32.show(expected)) && empty_lines(lines))
    case _: InvalidCommand{"invalid session control frame"}

def session_command(expected: U32,lines: List<&2,String>) -> SessionCommand:
  match lines:
    case header <> body: session_control(expected,W.parse(header),body)
    case Nil{}: InvalidCommand{"missing session control frame"}

def session_index_continue(done: Bool,next: File -> IO(Unit),more: (File -> IO(Unit)) -> File -> U32 -> U32 -> IO(Unit),count: U32,bytes: U32,file: File) -> IO(Unit):
  match done:
    case True{}: next(file)
    case False{}: more(next,file,count,bytes)

def session_index_result(result: Maybe<&2,String>,sequence: U32,+done: Bool,next: File -> IO(Unit),more: (File -> IO(Unit)) -> File -> U32 -> U32 -> IO(Unit),file: File,count: U32,bytes: U32) -> IO(Unit):
  match result:
    case None{}: session_fail(file,"invalid UTF-8 index session frame")
    case Some{text}: session_emit(sequence,done,text,session_index_continue(done,next,more,count,bytes),file)

def session_index_collected(sequence: U32,done: Bool,next: File -> IO(Unit),more: (File -> IO(Unit)) -> File -> U32 -> U32 -> IO(Unit),result: File & Batch) -> IO(Unit):
  match result:
    case (file,Batch{_,True{},_,_}): session_fail(file,"index session ended before declared frames")
    case (file,Batch{frames,False{},count,bytes}): session_index_result(parallel(3n,frames,4n),sequence,done,next,more,file,count,bytes)

def session_take(small: Bool,remaining: Nat) -> Nat:
  match small:
    case True{}: remaining
    case False{}: 8n

def session_index_loop(fuel: Nat,+sequence: U32,+remaining: Nat) -> (File -> IO(Unit)) -> File -> U32 -> U32 -> IO(Unit):
  match fuel:
    case 0n: next => file => count => bytes => session_fail(file,"session index batch limit")
    case 1n+p:
      next => file => count => bytes =>
        IO.bind(File & Batch,Unit,collect(session_take(Nat.is_le(remaining,8n),remaining),file,Nil{},count,bytes),session_index_collected(sequence,Nat.is_le(remaining,8n),next,session_index_loop(p,sequence,Nat.sub(remaining,8n))))

def session_execute(command: SessionCommand,+sequence: U32,next: Retained -> File -> IO(Unit),state: Retained,file: File) -> IO(Unit):
  match command:
    case InvalidCommand{reason}: session_fail(file,reason)
    case Single{lines}: session_processed(session_lines(lines,sequence,state),sequence,next,file)
    case Index{frames}: session_index_loop(17n,sequence,frames,next(state),file,0,0)

def session_decoded(value: Maybe<&2,String>,+sequence: U32,next: Retained -> File -> IO(Unit),state: Retained,file: File) -> IO(Unit):
  match value:
    case None{}: session_fail(file,"invalid UTF-8 session frame")
    case Some{text}: session_execute(session_command(sequence,W.lines(text)),sequence,next,state,file)

def session_frame(sequence: U32,next: Retained -> File -> IO(Unit),state: Retained,result: File & FrameIO.Frame) -> IO(Unit):
  match result:
    case (file,FrameIO.End{}): File.close(file)
    case (file,FrameIO.Payload{_,chunks}): session_decoded(U.decode_all(chunks),sequence,next,state,file)

def session_loop(fuel: Nat,+sequence: U32) -> Retained -> File -> IO(Unit):
  match fuel:
    case 0n: state => file => session_fail(file,"session request limit")
    case 1n+p: state => file => IO.bind(File & FrameIO.Frame,Unit,FrameIO.read(file),session_frame(sequence,session_loop(p,(sequence + 1 : U32)),state))

def start(args: List<&1,String>,file: File) -> IO(Unit):
  match args:
    case "--stream" <> Nil{}: stream_run(file)
    case "--session" <> Nil{}: session_loop(4294967295n,0,Retained{empty_session(),NoBase{},Segment.empty(),Rank.empty()},file)
    case Nil{}: read_loop(4096n,file,Nil{})
    case _: IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,"unknown core arguments"))

def main() -> IO(Unit):
  do IO<Unit>:
    args : List<&1,String> <- IO.args()
    file : File <- IO.try(File,File.open("/dev/stdin","r"))
    start(args,file)
