import Base
import ./Wire.bend as W
import ./Predicate.bend as P
import ./Core.bend as C
import ./Equality.bend as E

def decoded(row: W.Record) -> List<&2,String>:
  match row:
    case W.Row{fields}: fields
    case W.Invalid{_}: ["INVALID"]

def rows(lines: List<&2,String>) -> List<&2,List<&2,String>>:
  match lines:
    case Nil{}: Nil{}
    case "" <> t: rows(t)
    case h <> t: decoded(W.parse(h)) <> rows(t)

def predicate(scope: String,request: W.Record,source: String) -> String:
  match request:
    case W.Row{rule <> arg <> Nil{}}: C.status(P.evaluate_rows(rule,scope,arg,rows(W.lines(source))))
    case _: "unknown"

# Route the operation separately from structural list matching. This avoids
# multiplying long literal-pattern branches during native code generation.
def projection(payload: String,deps: List<&2,String>) -> String:
  match deps:
    case Nil{}: payload
    case _: "unknown"

def dependent(scope: String,request: String,deps: List<&2,String>) -> String:
  match deps:
    case _ <> source <> Nil{}: predicate(scope,W.parse(W.field(String.lines(request),0n)),source)
    case _: "unknown"

def operation(op: String,scope: String,payload: String,deps: List<&2,String>) -> String:
  match op:
    case "projection": projection(payload,deps)
    case "predicate": dependent(scope,payload,deps)
    case _: "unknown"

def compute(args: List<&2,String>) -> String:
  match args:
    case _ <> scope <> _ <> _ <> op <> payload <> deps: operation(op,scope,payload,deps)
    case _: "unknown"

# Projection mode is derived from the rule, never from an untrusted cache tag.
def mode(rule: String) -> String:
  match rule:
    case "member": "files"
    case "path-member": "paths"
    case "absent-path": "paths"
    case "contains": "text"
    case "absent-text": "text"
    case "native": "native"
    case "semantic": "none"
    case _: "facts"

def keep(include: Bool,fields: List<&2,String>,rest: String) -> String:
  match include:
    case True{}: W.row_onto(fields,rest)
    case False{}: rest

# Parse construction/reuse labels describe work, not predicate inputs.
# Predicate fact rules consume only tag, path, input identity, grammar and facts.
def fact_projection(+row: List<&2,String>) -> List<&2,String>:
  match row:
    case "P" <> path <> digest <> grammar <> facts <> rest: ["P",path,digest,grammar,facts]
    case row: row

def project_plain(mode: String,+scope: String,+row: List<&2,String>,rest: String) -> String:
  match mode:
    case "files": keep(P.selection(row,scope),["F",W.field(row,1n),"",""],rest)
    case "paths": keep(P.path_selection(row,scope),[W.field(row,0n),W.field(row,1n),"",""],rest)
    case "text": keep(P.selection(row,scope),["F",W.field(row,1n),"",W.field(row,3n)],rest)
    case "facts": keep((P.is_tag(row,"F") || P.is_tag(row,"P")) && P.selected(W.field(row,1n),scope),fact_projection(row),rest)
    case "native": keep((P.is_tag(row,"F") || P.is_tag(row,"N")) && P.selected(W.field(row,1n),scope) || P.is_tag(row,"OP"),row,rest)
    case _: rest

# A chunked projection keeps every CH status and id in original order.
# Only PC rows with another selector are dropped: Predicate.partial_rows already
# ignores them for this query, while the complete CH/PC fold stays aligned.
def chunk_projection(+row: List<&2,String>) -> List<&2,String>:
  ["CH",W.field(row,1n),W.field(row,2n)]

def content_mode(mode: String) -> Bool:
  match mode:
    case "text": True{}
    case "facts": True{}
    case _: False{}

def project_chunk_row(is_ch: Bool,mode: String,+scope: String,+row: List<&2,String>,rest: String) -> String:
  match is_ch:
    case True{}: keep(content_mode(mode),chunk_projection(row),rest)
    case False{}: keep(content_mode(mode) && E.text_eq(W.field(row,3n),scope),row,rest)

def project_chunk(chunk: Bool,mode: String,+scope: String,+row: List<&2,String>,rest: String) -> String:
  match chunk:
    case True{}: project_chunk_row(P.is_tag(row,"CH"),mode,scope,row,rest)
    case False{}: project_plain(mode,scope,row,rest)

def project_row(mode: String,+scope: String,+row: List<&2,String>,rest: String) -> String:
  project_chunk(P.is_tag(row,"CH") || P.is_tag(row,"PC"),mode,scope,row,rest)

# Each selected row is encoded directly in front of the remaining projection.
def project(rows: List<&2,List<&2,String>>,+scope: String,+mode: String) -> String:
  match rows:
    case Nil{}: ""
    case h <> t: project_row(mode,scope,h,project(t,scope,mode))
