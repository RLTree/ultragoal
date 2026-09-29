import Base
import ./Facts.bend as F
import ./StreamingSha.bend as Sha

# A logical source is one path/version, regardless of transport page count.
# Each page is admitted only at its exact next byte offset. Content is exposed
# only after END has consumed the declared byte length. An invalid transition
# drops the pending source, so a later operation cannot reuse a partial value.
type State is Data:
  Empty{}
  Open{operation: String, path: String, version: String, digest: String, expected: Nat, next: Nat, sequence: Nat, parts: List<&2,String>, hash: Sha.Stream}

type SegmentResult is Data:
  SegmentResult{state: State, status: String, source: Maybe<&2,String>}

def refused(reason: String) -> SegmentResult:
  SegmentResult{Empty{},reason,None{}}

def begun(operation: String,path: String,version: String,digest: String,expected: Nat) -> SegmentResult:
  SegmentResult{Open{operation,path,version,digest,expected,0n,0n,Nil{},Sha.empty()},"begun",None{}}

def begin(state: State,operation: String,path: String,version: String,digest: String,expected: Nat) -> SegmentResult:
  match state:
    case Empty{}: begun(operation,path,version,digest,expected)
    case Open{_,_,_,_,_,_,_,_,_}: refused("begin-before-end")

def page_state(operation: String,path: String,version: String,digest: String,expected: Nat,+next: Nat,+sequence: Nat,parts: List<&2,String>,hash: Sha.Stream,+text: String) -> SegmentResult:
  SegmentResult{Open{operation,path,version,digest,expected,Nat.add(next,U32.to_nat(F.bytes(text))),1n+sequence,text <> parts,Sha.feed(hash,text)},"page",None{}}

def page_open(ok: Bool,operation: String,path: String,version: String,digest: String,expected: Nat,next: Nat,sequence: Nat,parts: List<&2,String>,hash: Sha.Stream,text: String) -> SegmentResult:
  match ok:
    case True{}: page_state(operation,path,version,digest,expected,next,sequence,parts,hash,text)
    case False{}: refused("page-out-of-order-or-stale")

def page(state: State,+operation: String,+path: String,+version: String,+sequence: Nat,+offset: Nat,+text: String) -> SegmentResult:
  match state:
    case Empty{}: refused("page-without-begin")
    case Open{+op,+file,+ver,digest,+expected,+next,+number,parts,hash}:
      page_open(String.eq(op,operation) && String.eq(file,path) && String.eq(ver,version) && Nat.is_eq(number,sequence) && Nat.is_eq(next,offset) && Nat.is_le(Nat.add(next,U32.to_nat(F.bytes(text))),expected),op,file,ver,digest,expected,next,number,parts,hash,text)

def digest_ok(result: Maybe<&2,String>,expected: String) -> Bool:
  match result:
    case Some{digest}: String.eq(digest,expected)
    case None{}: False{}

def complete_digest(ok: Bool,parts: List<&2,String>) -> SegmentResult:
  match ok:
    case True{}: SegmentResult{Empty{},"complete",Some{String.concat(List.reverse(&2,String,parts))}}
    case False{}: refused("end-digest-mismatch")

def ended(ok: Bool,parts: List<&2,String>,hash: Sha.Stream,digest: String) -> SegmentResult:
  match ok:
    case True{}: complete_digest(digest_ok(Sha.finish(hash),digest),parts)
    case False{}: refused("end-incomplete-or-stale")

def end(state: State,+operation: String,+path: String,+version: String,+digest: String,+sequence: Nat) -> SegmentResult:
  match state:
    case Empty{}: refused("end-without-begin")
    case Open{+op,+file,+ver,+stored,+expected,+next,+number,parts,hash}:
      ended(String.eq(op,operation) && String.eq(file,path) && String.eq(ver,version) && String.eq(stored,digest) && Nat.is_eq(number,sequence) && Nat.is_eq(next,expected),parts,hash,stored)

def abort(state: State) -> SegmentResult:
  match state:
    case _: SegmentResult{Empty{},"aborted",None{}}

def state(result: SegmentResult) -> State:
  match result:
    case SegmentResult{state,_,_}: state

def status(result: SegmentResult) -> String:
  match result:
    case SegmentResult{_,status,_}: status

def source(result: SegmentResult) -> Maybe<&2,String>:
  match result:
    case SegmentResult{_,_,source}: source
