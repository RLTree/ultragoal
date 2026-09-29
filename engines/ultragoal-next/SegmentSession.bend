import Base
import ./Wire.bend as W
import ./SourceSegment.bend as S
import ./Policy.bend as P

type State is Data:
  Empty{}
  Loading{source: S.State}
  Ready{operation: String, path: String, version: String, digest: String, source: String}
  Draining{operation: String, path: String, version: String, digest: String, rest: String, next: Nat}

type Outcome is Data:
  Outcome{state: State, reply: String}

def empty() -> State:
  Empty{}

def refused(reason: String) -> Outcome:
  Outcome{Empty{},W.row(["SEG_ERROR",reason])}

def begun(value: S.SegmentResult) -> Outcome:
  match value:
    case S.SegmentResult{source,"begun",_}: Outcome{Loading{source},W.row(["SEG_ACK","begin"])}
    case S.SegmentResult{_,reason,_}: refused(reason)

def begin(state: State,op: String,path: String,version: String,digest: String,expected: Maybe<&2,Nat>) -> Outcome:
  match state expected:
    case Empty{} Some{size}: begun(S.begin(S.Empty{},op,path,version,digest,size))
    case Empty{} None{}: refused("invalid-source-length")
    case _ _: refused("begin-before-clear")

def paged(value: S.SegmentResult) -> Outcome:
  match value:
    case S.SegmentResult{source,"page",_}: Outcome{Loading{source},W.row(["SEG_ACK","page"])}
    case S.SegmentResult{_,reason,_}: refused(reason)

def page(state: State,op: String,path: String,version: String,sequence: Maybe<&2,Nat>,offset: Maybe<&2,Nat>,text: String) -> Outcome:
  match state sequence offset:
    case Loading{source} Some{index} Some{at}: paged(S.page(source,op,path,version,index,at,text))
    case _ _ _: refused("invalid-page")

def completed(value: S.SegmentResult,op: String,path: String,version: String,digest: String) -> Outcome:
  match value:
    case S.SegmentResult{_,"complete",Some{content}}: Outcome{Ready{op,path,version,digest,content},W.row(["SEG_ACK","complete"])}
    case S.SegmentResult{_,reason,_}: refused(reason)

def end(state: State,+op: String,+path: String,+version: String,+digest: String,sequence: Maybe<&2,Nat>) -> Outcome:
  match state sequence:
    case Loading{source} Some{index}: completed(S.end(source,op,path,version,digest,index),op,path,version,digest)
    case _ _: refused("invalid-end")

def read_ready(ok: Bool,+op: String,+path: String,+version: String,+digest: String,+rest: String,+next: Nat,+count: Nat) -> Outcome:
  match ok:
    case True{}: Outcome{Draining{op,path,version,digest,String.drop(rest,count),Nat.add(next,count)},W.row(["SEG_DATA",Nat.show(next),String.take(rest,count)])}
    case False{}: refused("read-out-of-order-or-stale")

def read(state: State,op: String,path: String,version: String,digest: String,offset: Maybe<&2,Nat>,count: Maybe<&2,Nat>) -> Outcome:
  match state offset count:
    case Ready{+held_op,+held_path,+held_version,+held_digest,+source} Some{+at} Some{+n}:
      read_ready(String.eq(op,held_op) && String.eq(path,held_path) && String.eq(version,held_version) && Nat.is_eq(at,0n) && String.eq(digest,held_digest) && Nat.is_ge(n,1n) && Nat.is_le(n,262144n) && Nat.is_le(n,String.length(source)),held_op,held_path,held_version,held_digest,source,0n,n)
    case Draining{+held_op,+held_path,+held_version,+held_digest,+rest,+next} Some{+at} Some{+n}:
      read_ready(String.eq(op,held_op) && String.eq(path,held_path) && String.eq(version,held_version) && String.eq(digest,held_digest) && Nat.is_eq(at,next) && Nat.is_ge(n,1n) && Nat.is_le(n,262144n) && Nat.is_le(n,String.length(rest)),held_op,held_path,held_version,held_digest,rest,next,n)
    case _ _ _: refused("read-before-complete")

def disclosed(ok: Bool,path: String,source: String) -> Outcome:
  match ok:
    case True{}: Outcome{Empty{},W.row(["DISCLOSURE",Bool.pick(String,P.disclosable(path,source),"eligible","refused")])}
    case False{}: refused("disclosure-stale-or-unbound")

def disclosure(state: State,op: String,path: String,version: String,digest: String) -> Outcome:
  match state:
    case Ready{+held_op,+held_path,+held_version,+held_digest,source}:
      disclosed(String.eq(op,held_op) && String.eq(path,held_path) && String.eq(version,held_version) && String.eq(digest,held_digest),held_path,source)
    case _: refused("disclosure-before-complete")

def native_planned(ok: Bool,source: String) -> Outcome:
  match ok:
    case True{}: Outcome{Empty{},P.native_plan(W.Row{source <> Nil{}})}
    case False{}: refused("native-plan-stale-or-unbound")

def native_plan(state: State,op: String,path: String,version: String,digest: String) -> Outcome:
  match state:
    case Ready{+held_op,+held_path,+held_version,+held_digest,source}:
      native_planned(String.eq(op,held_op) && String.eq(path,held_path) && String.eq(version,held_version) && String.eq(digest,held_digest),source)
    case _: refused("native-plan-before-complete")

def parsed(row: W.Record,state: State) -> Outcome:
  match row:
    case W.Row{"SEG_BEGIN" <> op <> path <> version <> digest <> size <> Nil{}}: begin(state,op,path,version,digest,Nat.read(size))
    case W.Row{"SEG_PAGE" <> op <> path <> version <> seq <> offset <> text <> Nil{}}: page(state,op,path,version,Nat.read(seq),Nat.read(offset),text)
    case W.Row{"SEG_END" <> op <> path <> version <> digest <> seq <> Nil{}}: end(state,op,path,version,digest,Nat.read(seq))
    case W.Row{"SEG_READ" <> op <> path <> version <> digest <> offset <> count <> Nil{}}: read(state,op,path,version,digest,Nat.read(offset),Nat.read(count))
    case W.Row{"SEG_DISCLOSURE" <> op <> path <> version <> digest <> Nil{}}: disclosure(state,op,path,version,digest)
    case W.Row{"SEG_NATIVE_PLAN" <> op <> path <> version <> digest <> Nil{}}: native_plan(state,op,path,version,digest)
    case W.Row{"SEG_ABORT" <> Nil{}}: Outcome{Empty{},W.row(["SEG_ACK","aborted"])}
    case _: refused("invalid-segment-command")

def process(line: String,state: State) -> Outcome:
  parsed(W.parse(line),state)

def process_lines(lines: List<&2,String>,state: State) -> Outcome:
  match lines:
    case line <> Nil{}: process(line,state)
    case line <> "" <> Nil{}: process(line,state)
    case _: refused("segment-command-must-be-one-row")

def state(value: Outcome) -> State:
  match value:
    case Outcome{state,_}: state

def reply(value: Outcome) -> String:
  match value:
    case Outcome{_,reply}: reply
