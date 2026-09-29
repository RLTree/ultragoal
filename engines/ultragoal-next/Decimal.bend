import Base
import ./Json.bend as J

# Exact decimal range checks before any floating-point approximation.
type Scan is Data:
  Scan{negative: Bool,before: U32,leading: U32,first: U32,tail_nonzero: Bool,fraction: Bool,exponent: Bool,exp_negative: Bool,exp: U32}

def choose(ok: Bool,a: U32,b: U32) -> U32:
  match ok:
    case True{}: a
    case False{}: b

def clamped(+n: U32) -> U32:
  choose(U32.is_gt(n,100000000),100000000,n)

def digit(in_exp: Bool,+d: U32,negative: Bool,+before: U32,+leading: U32,+first: U32,tail: Bool,+fraction: Bool,exp_negative: Bool,exp: U32) -> Scan:
  match in_exp:
    case True{}: Scan{negative,before,leading,first,tail,fraction,True{},exp_negative,clamped((exp * 10 + d : U32))}
    case False{}: Scan{negative,(before + choose(fraction,0,1) : U32),(leading + choose(U32.is_eq(first,0) && U32.is_eq(d,0),1,0) : U32),choose(U32.is_eq(first,0),d,first),tail || (U32.is_gt(first,0) && U32.is_gt(d,0)),fraction,False{},exp_negative,exp}

def minus(in_exp: Bool,negative: Bool,before: U32,leading: U32,first: U32,tail: Bool,fraction: Bool,exp: U32) -> Scan:
  match in_exp:
    case True{}: Scan{negative,before,leading,first,tail,fraction,True{},True{},exp}
    case False{}: Scan{True{},before,leading,first,tail,fraction,False{},False{},exp}

def step(c: Char,state: Scan) -> Scan:
  match c state:
    case '-' Scan{n,b,l,f,t,frac,e,en,x}: minus(e,n,b,l,f,t,frac,x)
    case '+' s: s
    case '.' Scan{n,b,l,f,t,frac,e,en,x}: Scan{n,b,l,f,t,True{},e,en,x}
    case 'e' Scan{n,b,l,f,t,frac,e,en,x}: Scan{n,b,l,f,t,frac,True{},en,x}
    case 'E' Scan{n,b,l,f,t,frac,e,en,x}: Scan{n,b,l,f,t,frac,True{},en,x}
    case c Scan{n,b,l,f,t,frac,e,en,x}: digit(e,(Char.to_u32(c) - 48 : U32),n,b,l,f,t,frac,en,x)

def scan(text: String,state: Scan) -> Scan:
  match text:
    case SNil{}: state
    case SCon{h,t}: scan(t,step(h,state))

def range_nonzero(negative: Bool,+left: U32,+right: U32,+first: U32,tail: Bool,+maximum: U32) -> Bool:
  Bool.not(negative) && U32.is_gt(maximum,0) && (U32.is_le(left,right) || (U32.is_eq(left,(right + 1 : U32)) && (U32.is_lt(first,maximum) || (U32.is_eq(first,maximum) && Bool.not(tail)))))

def range(state: Scan,maximum: U32) -> Bool:
  match state:
    case Scan{n,b,l,0,t,frac,e,en,x}: True{}
    case Scan{n,b,l,f,t,frac,e,+en,+x}: range_nonzero(n,(b + choose(en,0,x) : U32),(l + choose(en,x,0) : U32),f,t,maximum)

def bounded(value: J.Json,maximum: U32) -> Bool:
  match value:
    case J.Number{s}: range(scan(s,Scan{False{},0,0,0,False{},False{},False{},False{},0}),maximum)
    case _: False{}

def get_if(found: Bool,value: J.Json,other: J.Json) -> J.Json:
  match found:
    case True{}: value
    case False{}: other

def fields_get(fields: List<&2,Sigma<&2,&2,String,_ => J.Json>>,+key: String) -> J.Json:
  match fields:
    case Nil{}: J.Null{}
    case (+k,+v) <> t: get_if(String.eq(k,key),v,fields_get(t,key))

def get(value: J.Json,key: String) -> J.Json:
  match value:
    case J.Object{fields}: fields_get(fields,key)
    case _: J.Null{}

def probabilities(fields: List<&2,Sigma<&2,&2,String,_ => J.Json>>) -> Bool:
  match fields:
    case Nil{}: True{}
    case (k,v) <> t: bounded(v,1) && probabilities(t)

def distribution(value: J.Json) -> Bool:
  match value:
    case J.Object{fields}: probabilities(fields)
    case _: False{}

def count(fields: List<&2,Sigma<&2,&2,String,_ => J.Json>>) -> U32:
  match fields:
    case Nil{}: 0
    case h <> t: (1 + count(t) : U32)

def score_max(value: J.Json) -> U32:
  match value:
    case J.Object{Nil{}}: 0
    case J.Object{h <> t}: count(t)
    case _: 0

def answer(kind: J.Json,+value: J.Json) -> Bool:
  match kind:
    case J.Text{"noul"}: bounded(get(value,"noul"),1)
    case J.Text{"choice"}: bounded(get(value,"confidence"),1) && distribution(get(value,"probabilities"))
    case J.Text{"score"}: bounded(get(value,"confidence"),1) && distribution(get(value,"probabilities")) && bounded(get(value,"score"),score_max(get(value,"legend")))
    case _: False{}

def answers(fields: List<&2,Sigma<&2,&2,String,_ => J.Json>>) -> Bool:
  match fields:
    case Nil{}: True{}
    case (k,+v) <> t: answer(get(v,"type"),v) && answers(t)

def response(value: J.Json) -> Bool:
  match value:
    case J.Object{fields}: answers(fields)
    case _: False{}

def parsed(value: Maybe<&2,J.Json>) -> Bool:
  match value:
    case Some{v}: response(get(v,"answers"))
    case None{}: False{}

def validate(text: String) -> Bool:
  parsed(J.parse(text))
