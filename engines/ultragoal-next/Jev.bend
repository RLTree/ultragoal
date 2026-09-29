import Base
import ./Wire.bend as W

# Provider values cannot construct verification or authority evidence.
# A validated uncalibrated answer is retained only as advisory assessment.
def disposition(choice: String) -> String:
  match choice:
    case "supported": "advisory-supported"
    case "contradicted": "needs-review"
    case "insufficient": "unknown"
    case _: "unknown"

def numeric(+x: F32) -> Bool:
  F32.is_ge(x,0.0) && F32.is_le(x,1.0)

def number_valid(x: Maybe<&2,F32>) -> Bool:
  match x:
    case Some{+v}: numeric(v)
    case None{}: False{}

def probability(s: String) -> Bool:
  number_valid(F32.read(s))

def sum_value(x: Maybe<&2,F32>) -> F32:
  match x:
    case Some{v}: v
    case None{}: 0.0

def close(+a: F32, +b: F32) -> Bool:
  F32.is_le(a,(b + 0.0001 : F32)) && F32.is_ge(a,(b - 0.0001 : F32))

def all_probabilities(xs: List<&2,String>) -> Bool:
  match xs:
    case Nil{}: True{}
    case +h <> t: probability(h) && all_probabilities(t)

def total(xs: List<&2,String>) -> F32:
  match xs:
    case Nil{}: 0.0
    case h <> t: (sum_value(F32.read(h)) + total(t) : F32)

def distribution(+xs: List<&2,String>) -> Bool:
  all_probabilities(xs) && close(total(xs),1.0)

def weighted(xs: List<&2,String>, +level: F32) -> F32:
  match xs:
    case Nil{}: 0.0
    case h <> t: ((sum_value(F32.read(h)) * level) + weighted(t,(level + 1.0 : F32)) : F32)

def readable(value: Maybe<&2,F32>) -> Bool:
  match value:
    case Some{+v}: F32.is_eq(v,v)
    case None{}: False{}

def levels(xs: List<&2,String>) -> F32:
  match xs:
    case Nil{}: 0.0
    case _ <> t: (1.0 + levels(t) : F32)

# Jev reports each probability and the score rounded to 0.01, so the reported score
# may differ from the mean of the reported probabilities by up to 0.005 per unit of
# level weight plus the score's own rounding: (L(L-1)/2 + 1) * 0.005 (RR score profile v3).
# Probability normalization stays strict.
def wire_allowance(+n: F32) -> F32:
  ((((n * (n - 1.0 : F32) : F32) / 2.0 : F32) + 1.0 : F32) * 0.005 + 0.000001 : F32)

def within_allowance(+a: F32, +b: F32, +allowance: F32) -> Bool:
  F32.is_le(a,(b + allowance : F32)) && F32.is_ge(a,(b - allowance : F32))

# Choice probabilities are reported to two decimals, so their sum may miss 1 by up to
# 0.005 per option (the jev-codex rule); F32 needs the 1e-6 slack that 1e-9 gives in
# double precision. Score normalization stays strict.
def rounded_distribution(+xs: List<&2,String>) -> Bool:
  all_probabilities(xs) && within_allowance(total(xs),1.0,(levels(xs) * 0.005 + 0.000001 : F32))

def validate_score(+score: String, +probabilities: List<&2,String>) -> Bool:
  readable(F32.read(score)) && distribution(probabilities) && within_allowance(sum_value(F32.read(score)),weighted(probabilities,0.0),wire_allowance(levels(probabilities)))

def answer(ok: Bool, choice: String) -> String:
  match ok:
    case False{}: "malformed"
    case True{}: disposition(choice)

def choice_valid(s: String) -> Bool:
  match s:
    case "supported": True{}
    case "contradicted": True{}
    case "insufficient": True{}
    case _: False{}

def validation(ok: Bool) -> String:
  match ok:
    case True{}: "validated-advisory"
    case False{}: "malformed"

# Probability of the stated option; an unknown option can never be the top answer.
def chosen(choice: String,+p0: F32,+p1: F32,+p2: F32) -> F32:
  match choice:
    case "supported": p0
    case "contradicted": p1
    case "insufficient": p2
    case _: (0.0 - 1.0 : F32)

# The stated option must hold the highest probability exactly: no near-winner allowance.
def highest(+v: F32,+p0: F32,+p1: F32,+p2: F32) -> Bool:
  F32.is_ge(v,p0) && F32.is_ge(v,p1) && F32.is_ge(v,p2)

def hits(+v: F32,+p: F32) -> U32:
  Bool.pick(U32,F32.is_eq(v,p),1,0)

# A top answer shared with another option is valid but never qualifies as a decision.
def tied(+v: F32,+p0: F32,+p1: F32,+p2: F32) -> Bool:
  U32.is_gt((hits(v,p0) + hits(v,p1) + hits(v,p2) : U32),1)

def top(+choice: String,+p0: F32,+p1: F32,+p2: F32) -> Bool:
  highest(chosen(choice,p0,p1,p2),p0,p1,p2)

def top_tied(+choice: String,+p0: F32,+p1: F32,+p2: F32) -> Bool:
  tied(chosen(choice,p0,p1,p2),p0,p1,p2)

def relevance_choice(choice: String) -> String:
  match choice:
    case "relevant": "supported"
    case "irrelevant": "contradicted"
    case "insufficient": "insufficient"
    case _: "invalid"

def relevance_result(ok: Bool,tie: Bool,choice: String) -> String:
  match ok tie:
    case True{} True{}: "advisory-tied"
    case True{} False{}: "advisory-" ++ choice
    case False{} _: "malformed"

def choice_row(id: String,+option: String,+choice: String,confidence: String,+p0: String,+p1: String,+p2: String) -> String:
  W.row(["ANSWER",id,relevance_result(probability(confidence) && rounded_distribution([p0,p1,p2]) && top(option,sum_value(F32.read(p0)),sum_value(F32.read(p1)),sum_value(F32.read(p2))),top_tied(option,sum_value(F32.read(p0)),sum_value(F32.read(p1)),sum_value(F32.read(p2))),choice)])

def probe_choice(kind: String,choice: String) -> String:
  match kind choice:
    case "CONFLICT" "explicit-conflict": "supported"
    case "CONFLICT" "no-explicit-conflict": "contradicted"
    case "CONFLICT" "uncertain": "insufficient"
    case "SUPPORT" "complete-support": "supported"
    case "SUPPORT" "missing-support": "contradicted"
    case "SUPPORT" "uncertain": "insufficient"
    case _ _: "invalid"

def probe_row(kind: String,id: String,+choice: String,confidence: String,+p0: String,+p1: String,+p2: String) -> String:
  choice_row(id,probe_choice(kind,choice),choice,confidence,p0,p1,p2)

def validate_row(xs: List<&2,String>) -> String:
  match xs:
    case "CONFLICT" <> id <> choice <> confidence <> p0 <> p1 <> p2 <> Nil{}: probe_row("CONFLICT",id,choice,confidence,p0,p1,p2)
    case "SUPPORT" <> id <> choice <> confidence <> p0 <> p1 <> p2 <> Nil{}: probe_row("SUPPORT",id,choice,confidence,p0,p1,p2)
    case "RELEVANCE" <> id <> +choice <> confidence <> +p0 <> +p1 <> +p2 <> Nil{}:
      choice_row(id,relevance_choice(choice),choice,confidence,p0,p1,p2)
    case "CHOICE" <> id <> +choice <> confidence <> +p0 <> +p1 <> +p2 <> Nil{}:
      W.row(["ANSWER",id,answer(choice_valid(choice) && probability(confidence) && rounded_distribution([p0,p1,p2]) && top(choice,sum_value(F32.read(p0)),sum_value(F32.read(p1)),sum_value(F32.read(p2))),Bool.pick(String,top_tied(choice,sum_value(F32.read(p0)),sum_value(F32.read(p1)),sum_value(F32.read(p2))),"insufficient",choice))])
    case "NOUL" <> id <> p <> Nil{}:
      W.row(["ANSWER",id,validation(probability(p))])
    case "SCORE" <> id <> score <> confidence <> ps:
      W.row(["ANSWER",id,validation(probability(confidence) && validate_score(score,ps))])
    case _: W.row(["ANSWER","invalid","malformed"])

def parsed(r: W.Record) -> String:
  match r:
    case W.Row{xs}: validate_row(xs)
    case _: W.row(["ANSWER","invalid","malformed"])

def process(lines: List<&2,String>) -> String:
  match lines:
    case Nil{}: ""
    case "" <> t: process(t)
    case h <> t: parsed(W.parse(h)) ++ process(t)
