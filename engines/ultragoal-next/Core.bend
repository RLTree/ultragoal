import Base

type Status is Data:
  Verified{}
  Assessed{}
  Review{}
  Failed{}
  Unknown{}
  NotApplicable{}

type Assurance is Data:
  Exact{}
  Semantic{}
  Observed{}
  Formal{}

type Lifecycle is Data:
  Planned{}
  Running{}
  Completed{}
  Cancelled{}
  Superseded{}

type Binding is Data:
  Bound{subject: Bool, inputs: Bool, rule: Bool, producer: Bool, complete: Bool, authorized: Bool, scope: Bool, timely: Bool}

def binding_ok(b: Binding) -> Bool:
  match b:
    case Bound{s,i,r,p,c,a,sc,t}: s && i && r && p && c && a && sc && t

def permitted(life: Lifecycle) -> Bool:
  match life:
    case Running{}: True{}
    case _: False{}

def assurance_ok(want: Assurance, got: Assurance) -> Bool:
  match want got:
    case Exact{} Exact{}: True{}
    case Semantic{} Semantic{}: True{}
    case Observed{} Observed{}: True{}
    case Formal{} Formal{}: True{}
    case _ _: False{}

def admit_if(ok: Bool, result: Status) -> Status:
  match ok:
    case True{}: result
    case False{}: Unknown{}

def assurance_result(want: Assurance,result: Status) -> Status:
  match want result:
    case Semantic{} Verified{}: Unknown{}
    case Exact{} Assessed{}: Unknown{}
    case Observed{} Assessed{}: Unknown{}
    case Formal{} Assessed{}: Unknown{}
    case _ status: status

def admit(life: Lifecycle, b: Binding, +want: Assurance, got: Assurance, result: Status) -> Status:
  admit_if(permitted(life) && binding_ok(b) && assurance_ok(want, got), assurance_result(want,result))

def terminal(life: Lifecycle, event: Lifecycle) -> Lifecycle:
  match life:
    case Cancelled{}: Cancelled{}
    case Superseded{}: Superseded{}
    case Completed{}: Completed{}
    case _: event

def reserve(budget: Nat, cost: Nat) -> Maybe<&2, Nat>:
  match budget cost:
    case b 0n: Some{b}
    case 0n 1n+c: None{}
    case 1n+b 1n+c: reserve(b,c)

def mandatory(applicable: Bool, result: Status) -> Status:
  match applicable:
    case True{}:
      match result:
        case NotApplicable{}: Unknown{}
        case r: r
    case False{}: NotApplicable{}

def status(s: Status) -> String:
  match s:
    case Verified{}: "verified"
    case Assessed{}: "assessed-clear"
    case Review{}: "needs-review"
    case Failed{}: "failed"
    case Unknown{}: "unknown"
    case NotApplicable{}: "not-applicable"
