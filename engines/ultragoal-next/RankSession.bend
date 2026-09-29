import Base
import ./Wire.bend as W
import ./Retrieval.bend as R
import ./Advice.bend as A

# Ordered physical pages retain one task-owned Bend decision for global rank,
# advice order/rank and the final check. A failed or interrupted operation
# cannot publish a partial result. Rank output itself is read in pages.
type State is Data:
  RankEmpty{}
  RankOpen{operation: String, query: String, shortlist: String, expected: Nat, next: Nat, seen: Nat, pages: List<&2,List<&2,String>>}
  AdviceOpen{operation: String, mode: String, initial: String, expected: Nat, next: Nat, seen: Nat, pages: List<&2,List<&2,String>>}
  CheckOpen{operation: String, expected: Nat, next: Nat, seen: Nat, pages: List<&2,List<&2,String>>}
  CheckReady{operation: String, rows: List<&2,String>}
  RankOutput{operation: String, rest: String, next: Nat}

type Outcome is Data:
  Outcome{state: State, reply: String}

def empty() -> State:
  RankEmpty{}

def refused(reason: String) -> Outcome:
  Outcome{RankEmpty{},W.row(["RANK_ERROR",reason])}

def begun(state: State,op: String,query: String,shortlist: String,total: Maybe<&2,Nat>) -> Outcome:
  match state total:
    case RankEmpty{} Some{count}: Outcome{RankOpen{op,query,shortlist,count,0n,0n,Nil{}},W.row(["RANK_ACK","begin"])}
    case RankEmpty{} None{}: refused("invalid-count")
    case _ _: refused("begin-before-end")

def advice_begin_mode(ok: Bool,op: String,mode: String,initial: String,+count: Nat) -> Outcome:
  match ok:
    case True{}: Outcome{AdviceOpen{op,mode,initial,count,0n,0n,Nil{}},W.row(["ADV_ACK","begin"])}
    case False{}: refused("invalid-advice-mode")

def advice_begun(state: State,op: String,+mode: String,initial: String,total: Maybe<&2,Nat>) -> Outcome:
  match state total:
    case RankEmpty{} Some{count}: advice_begin_mode(String.eq(mode,"order") || String.eq(mode,"rank"),op,mode,initial,count)
    case RankEmpty{} None{}: refused("invalid-count")
    case _ _: refused("begin-before-end")

def check_begun(state: State,op: String,total: Maybe<&2,Nat>) -> Outcome:
  match state total:
    case RankEmpty{} Some{count}: Outcome{CheckOpen{op,count,0n,0n,Nil{}},W.row(["CHECK_ACK","begin"])}
    case RankEmpty{} None{}: refused("invalid-count")
    case _ _: refused("begin-before-end")

def row_ok(row: W.Record) -> Bool:
  match row:
    case W.Row{"FILE" <> _}: True{}
    case W.Row{"WINDOWS" <> _}: True{}
    case W.Row{"X" <> _}: True{}
    case W.Row{"" <> Nil{}}: True{}
    case _: False{}

def rows_ok(lines: List<&2,String>) -> Bool:
  match lines:
    case Nil{}: True{}
    case h <> t: row_ok(W.parse(h)) && rows_ok(t)

def advice_row_ok(row: W.Record) -> Bool:
  match row:
    case W.Row{_ <> _ <> Nil{}}: True{}
    case W.Row{"" <> Nil{}}: True{}
    case _: False{}

def advice_rows_ok(lines: List<&2,String>) -> Bool:
  match lines:
    case Nil{}: True{}
    case h <> t: advice_row_ok(W.parse(h)) && advice_rows_ok(t)

def row_count(lines: List<&2,String>) -> Nat:
  match lines:
    case Nil{}: 0n
    case "" <> t: row_count(t)
    case _ <> t: 1n+row_count(t)

def paged(ok: Bool,op: String,query: String,shortlist: String,+expected: Nat,+next: Nat,+seen: Nat,pages: List<&2,List<&2,String>>,+rows: List<&2,String>) -> Outcome:
  match ok:
    case True{}: Outcome{RankOpen{op,query,shortlist,expected,1n+next,Nat.add(seen,row_count(rows)),rows <> pages},W.row(["RANK_ACK","page"])}
    case False{}: refused("page-out-of-order-or-malformed")

def page(state: State,op: String,sequence: Maybe<&2,Nat>,body: String) -> Outcome:
  match state sequence:
    case RankOpen{+held,+query,+shortlist,+expected,+next,+seen,pages} Some{index}:
      +rows = String.lines(body)
      paged(String.eq(op,held) && Nat.is_eq(index,next) && rows_ok(rows) && Nat.is_le(Nat.add(seen,row_count(rows)),expected),held,query,shortlist,expected,next,seen,pages,rows)
    case _ _: refused("page-without-begin")

def advice_paged(ok: Bool,op: String,mode: String,initial: String,+expected: Nat,+next: Nat,+seen: Nat,pages: List<&2,List<&2,String>>,+rows: List<&2,String>) -> Outcome:
  match ok:
    case True{}: Outcome{AdviceOpen{op,mode,initial,expected,1n+next,Nat.add(seen,row_count(rows)),rows <> pages},W.row(["ADV_ACK","page"])}
    case False{}: refused("page-out-of-order-or-malformed")

def advice_page(state: State,op: String,sequence: Maybe<&2,Nat>,body: String) -> Outcome:
  match state sequence:
    case AdviceOpen{+held,mode,initial,+expected,+next,+seen,pages} Some{index}:
      +rows = String.lines(body)
      advice_paged(String.eq(op,held) && Nat.is_eq(index,next) && advice_rows_ok(rows) && Nat.is_le(Nat.add(seen,row_count(rows)),expected),held,mode,initial,expected,next,seen,pages,rows)
    case _ _: refused("page-without-begin")

def check_paged(ok: Bool,op: String,+expected: Nat,+next: Nat,+seen: Nat,pages: List<&2,List<&2,String>>,+rows: List<&2,String>) -> Outcome:
  match ok:
    case True{}: Outcome{CheckOpen{op,expected,1n+next,Nat.add(seen,row_count(rows)),rows <> pages},W.row(["CHECK_ACK","page"])}
    case False{}: refused("page-out-of-order-or-malformed")

def check_page(state: State,op: String,sequence: Maybe<&2,Nat>,body: String) -> Outcome:
  match state sequence:
    case CheckOpen{+held,+expected,+next,+seen,pages} Some{index}:
      +rows = String.lines(body)
      check_paged(String.eq(op,held) && Nat.is_eq(index,next) && Nat.is_le(Nat.add(seen,row_count(rows)),expected),held,expected,next,seen,pages,rows)
    case _ _: refused("page-without-begin")

def flatten(pages: List<&2,List<&2,String>>) -> List<&2,String>:
  match pages:
    case Nil{}: Nil{}
    case h <> t: List.append(&2,String,h,flatten(t))

def output_paged(done: Bool,op: String,rest: String,+at: Nat,+next: Nat,chunk: String) -> Outcome:
  match done:
    case True{}: Outcome{RankEmpty{},W.row(["RANK_DATA",Nat.show(at),Nat.show(next),"done",chunk])}
    case False{}: Outcome{RankOutput{op,rest,next},W.row(["RANK_DATA",Nat.show(at),Nat.show(next),"more",chunk])}

def output_page(op: String,+text: String,+at: Nat) -> Outcome:
  +chunk = String.take(text,262144n)
  +rest = String.drop(text,262144n)
  +next = Nat.add(at,String.length(chunk))
  output_paged(String.is_empty(rest),op,rest,at,next,chunk)

def completed(op: String,query: String,shortlist: String,pages: List<&2,List<&2,String>>) -> Outcome:
  output_page(op,R.rank_process(W.field(W.lines(W.row(["QUERY",query])),0n) <> W.field(W.lines(W.row(["SHORTLIST",shortlist])),0n) <> flatten(List.reverse(&2,List<&2,String>,pages))),0n)

def ended(ok: Bool,op: String,query: String,shortlist: String,pages: List<&2,List<&2,String>>) -> Outcome:
  match ok:
    case True{}: completed(op,query,shortlist,pages)
    case False{}: refused("end-incomplete-or-stale")

def end(state: State,op: String,sequence: Maybe<&2,Nat>) -> Outcome:
  match state sequence:
    case RankOpen{+held,query,shortlist,+expected,+next,+seen,pages} Some{index}:
      ended(String.eq(op,held) && Nat.is_eq(index,next) && Nat.is_eq(seen,expected),held,query,shortlist,pages)
    case _ _: refused("end-without-begin")

def output_read_ready(ok: Bool,op: String,rest: String,+next: Nat) -> Outcome:
  match ok:
    case True{}: output_page(op,rest,next)
    case False{}: refused("output-read-stale")

def output_read(state: State,op: String,offset: Maybe<&2,Nat>) -> Outcome:
  match state offset:
    case RankOutput{+held,rest,+next} Some{at}:
      output_read_ready(String.eq(op,held) && Nat.is_eq(at,next),held,rest,next)
    case _ _: refused("output-read-without-result")

def advice_completed(mode: String,initial: String,pages: List<&2,List<&2,String>>) -> Outcome:
  +rows = flatten(List.reverse(&2,List<&2,String>,pages))
  Outcome{RankEmpty{},Bool.pick(String,String.eq(mode,"order"),A.order(W.field(W.lines(W.row([initial])),0n) <> rows),A.rank(rows))}

def advice_ended(ok: Bool,mode: String,initial: String,pages: List<&2,List<&2,String>>) -> Outcome:
  match ok:
    case True{}: advice_completed(mode,initial,pages)
    case False{}: refused("end-incomplete-or-stale")

def advice_end(state: State,op: String,sequence: Maybe<&2,Nat>) -> Outcome:
  match state sequence:
    case AdviceOpen{+held,mode,initial,+expected,+next,+seen,pages} Some{index}:
      advice_ended(String.eq(op,held) && Nat.is_eq(index,next) && Nat.is_eq(seen,expected),mode,initial,pages)
    case _ _: refused("end-without-begin")

def check_ended(ok: Bool,op: String,pages: List<&2,List<&2,String>>) -> Outcome:
  match ok:
    case True{}: Outcome{CheckReady{op,flatten(List.reverse(&2,List<&2,String>,pages))},W.row(["CHECK_ACK","ready"])}
    case False{}: refused("end-incomplete-or-stale")

def check_end(state: State,op: String,sequence: Maybe<&2,Nat>) -> Outcome:
  match state sequence:
    case CheckOpen{+held,+expected,+next,+seen,pages} Some{index}:
      check_ended(String.eq(op,held) && Nat.is_eq(index,next) && Nat.is_eq(seen,expected),held,pages)
    case _ _: refused("end-without-begin")

def parsed(record: W.Record,state: State) -> Outcome:
  match record:
    case W.Row{"RANK_BEGIN" <> op <> query <> shortlist <> count <> Nil{}}: begun(state,op,query,shortlist,Nat.read(count))
    case W.Row{"RANK_PAGE" <> op <> sequence <> body <> Nil{}}: page(state,op,Nat.read(sequence),body)
    case W.Row{"RANK_END" <> op <> sequence <> Nil{}}: end(state,op,Nat.read(sequence))
    case W.Row{"RANK_READ" <> op <> offset <> Nil{}}: output_read(state,op,Nat.read(offset))
    case W.Row{"RANK_ABORT" <> Nil{}}: Outcome{RankEmpty{},W.row(["RANK_ACK","aborted"])}
    case W.Row{"ADV_BEGIN" <> op <> mode <> initial <> count <> Nil{}}: advice_begun(state,op,mode,initial,Nat.read(count))
    case W.Row{"ADV_PAGE" <> op <> sequence <> body <> Nil{}}: advice_page(state,op,Nat.read(sequence),body)
    case W.Row{"ADV_END" <> op <> sequence <> Nil{}}: advice_end(state,op,Nat.read(sequence))
    case W.Row{"ADV_ABORT" <> Nil{}}: Outcome{RankEmpty{},W.row(["ADV_ACK","aborted"])}
    case W.Row{"CHECK_BEGIN" <> op <> count <> Nil{}}: check_begun(state,op,Nat.read(count))
    case W.Row{"CHECK_PAGE" <> op <> sequence <> body <> Nil{}}: check_page(state,op,Nat.read(sequence),body)
    case W.Row{"CHECK_END" <> op <> sequence <> Nil{}}: check_end(state,op,Nat.read(sequence))
    case W.Row{"CHECK_ABORT" <> Nil{}}: Outcome{RankEmpty{},W.row(["CHECK_ACK","aborted"])}
    case _: refused("invalid-rank-command")

def process(lines: List<&2,String>,state: State) -> Outcome:
  match lines:
    case line <> Nil{}: parsed(W.parse(line),state)
    case line <> "" <> Nil{}: parsed(W.parse(line),state)
    case _: refused("rank-command-must-be-one-row")
