import Base
import ./Wire.bend as W
import ./Policy.bend as P

# Conditional admission of facts supplied by the current trusted bridge call.
# This module cannot authenticate a wire string or create native host authority.
def surface(kind: String) -> String:
  match kind:
    case "c11-stdin": "Clang C11-mode syntax of the captured translation unit"
    case "python-ast": "Python syntax of captured source; no source execution"
    case "typescript-parse": "TypeScript syntax of captured source; no source execution"
    case _: ""

def known_kind(kind: String) -> Bool:
  match kind:
    case "c11-stdin": True{}
    case "python-ast": True{}
    case "typescript-parse": True{}
    case _: False{}

def present(value: String) -> Bool:
  Bool.not(String.is_empty(value))

def digit(+value: U32) -> Bool:
  (U32.is_ge(value,48) && U32.is_le(value,57)) || (U32.is_ge(value,97) && U32.is_le(value,102))

def hex(text: String) -> Bool:
  match text:
    case SNil{}: True{}
    case SCon{h,t}: digit(Char.to_u32(h)) && hex(t)

def identity(+value: String) -> Bool:
  Nat.is_eq(String.length(value),64n) && hex(value)

# Native source size is a current host-capture decision, not a universal rule
# ceiling. Bend still requires a well-formed nonnegative declared byte count
# and exact request/observation equality before admission.
def size_valid(value: Maybe<&2,Nat>) -> Bool:
  match value:
    case Some{_}: True{}
    case None{}: False{}

def planned(ok: Bool,fields: List<&2,String>) -> String:
  match ok:
    case True{}: W.row(fields)
    case False{}: "REFUSED\tunsupported or incomplete closed-input verifier request\n"

def plan(row: W.Record) -> String:
  match row:
    case W.Row{+op <> +request <> +kind <> +input <> +tool <> +invocation <> +bytes <> Nil{}}: planned(present(op) && present(request) && identity(input) && identity(tool) && identity(invocation) && known_kind(kind) && size_valid(Nat.read(bytes)),["VERIFICATION_REQUEST","1",op,request,kind,input,tool,invocation,bytes,surface(kind),"10000","captured-bytes"])
    case _: planned(False{},Nil{})

def record(text: String) -> W.Record:
  W.parse(W.field(String.lines(text),0n))

def matched(+request: List<&2,String>,+observation: List<&2,String>) -> Bool:
  Nat.is_eq(W.length(request),12n) && Nat.is_eq(W.length(observation),12n) && String.eq(W.field(request,0n),"VERIFICATION_REQUEST") && String.eq(W.field(request,1n),"1") && String.eq(W.field(observation,0n),"LOCAL_OBSERVATION") && known_kind(W.field(request,4n)) && String.eq(W.field(request,9n),surface(W.field(request,4n))) && String.eq(W.field(request,10n),"10000") && String.eq(W.field(request,11n),"captured-bytes") && identity(W.field(request,5n)) && identity(W.field(request,6n)) && identity(W.field(request,7n)) && size_valid(Nat.read(W.field(request,8n))) && String.eq(W.field(request,2n),W.field(observation,1n)) && String.eq(W.field(request,3n),W.field(observation,2n)) && String.eq(W.field(request,4n),W.field(observation,3n)) && String.eq(W.field(request,5n),W.field(observation,4n)) && String.eq(W.field(request,6n),W.field(observation,5n)) && String.eq(W.field(request,7n),W.field(observation,6n))

def status_present(value: Maybe<&2,U32>) -> Bool:
  match value:
    case Some{n}: U32.is_le(n,255)
    case None{}: False{}

def origin(origin: String,+request: List<&2,String>,+observation: List<&2,String>) -> String:
  match origin:
    case "current-pending-call": P.native_status(matched(request,observation) && status_present(U32.read(W.field(observation,7n))) && String.eq(W.field(observation,8n),"complete") && String.eq(W.field(observation,9n),"timely") && String.eq(W.field(observation,10n),"current"),String.eq(W.field(observation,7n),"0"))
    case _: P.native_status(False{},False{})

def observed(request: W.Record,observation: W.Record) -> String:
  match request observation:
    case W.Row{req} W.Row{+obs}: origin(W.field(obs,11n),req,obs)
    case _ _: P.native_status(False{},False{})

def observe(row: W.Record) -> String:
  match row:
    case W.Row{request <> observation <> Nil{}}: observed(record(request),record(observation))
    case _: P.native_status(False{},False{})
