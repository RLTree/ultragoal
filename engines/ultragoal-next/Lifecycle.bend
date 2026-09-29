import Base
import ./Core.bend as C
import ./Wire.bend as W

type State is Data:
  State{operation: String,life: C.Lifecycle,fingerprint: String,conflict: Bool}

def label(life: C.Lifecycle) -> String:
  match life:
    case C.Planned{}: "planned"
    case C.Running{}: "running"
    case C.Completed{}: "completed"
    case C.Cancelled{}: "cancelled"
    case C.Superseded{}: "superseded"

def conflict(ok: Bool,operation: String,life: C.Lifecycle,fingerprint: String,prior: Bool) -> State:
  match ok:
    case True{}: State{operation,life,fingerprint,prior}
    case False{}: State{operation,life,fingerprint,True{}}

def complete(operation: String,life: C.Lifecycle,+prior: String,+next: String,bad: Bool) -> State:
  match life:
    case C.Running{}: State{operation,C.Completed{},next,bad}
    case C.Completed{}: conflict(String.eq(prior,next),operation,C.Completed{},prior,bad)
    case other: State{operation,other,prior,bad}

def apply_event(name: String,next: String,operation: String,life: C.Lifecycle,prior: String,bad: Bool) -> State:
  match name:
    case "completed": complete(operation,life,prior,next,bad)
    case "running": State{operation,C.terminal(life,C.Running{}),prior,bad}
    case "cancelled": State{operation,C.terminal(life,C.Cancelled{}),prior,bad}
    case "superseded": State{operation,C.terminal(life,C.Superseded{}),prior,bad}
    case _: State{operation,life,prior,True{}}

def apply_valid(ok: Bool,name: String,next: String,state: State) -> State:
  match ok state:
    case False{} State{op,life,fp,bad}: State{op,life,fp,True{}}
    case True{} State{op,life,fp,bad}: apply_event(name,next,op,life,fp,bad)

def apply(row: W.Record,state: State) -> State:
  match row state:
    case W.Row{"EVENT" <> +op <> name <> fp <> Nil{}} State{+current,life,prior,bad}:
      apply_valid(String.eq(op,current) || String.is_empty(current),name,fp,State{op,life,prior,bad})
    case _ State{op,life,fp,bad}: State{op,life,fp,True{}}

def replay(lines: List<&2,String>,state: State) -> State:
  match lines:
    case Nil{}: state
    case "" <> Nil{}: state
    case h <> Nil{}: apply(W.Invalid{"partial final journal record"},state)
    case h <> t: replay(t,apply(W.parse(h),state))

def output(state: State) -> String:
  match state:
    case State{op,life,fp,False{}}: W.row(["JOURNAL",op,label(life),fp,"reported-only; observe before any new external request; never replay automatically"])
    case State{op,life,fp,True{}}: W.row(["JOURNAL",op,"conflict-or-corruption",fp,"preserve original log and reconcile producer evidence; no automatic replay"])

def process(lines: List<&2,String>) -> String:
  output(replay(lines,State{"",C.Planned{},"",False{}}))
