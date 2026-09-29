import Base
import ./Wire.bend as W

type Fact is Data:
  Fact{kind: String, value: String, line: U32, start: U32, end: U32}

def metadata(zero_line: Bool,value: String) -> String:
  match zero_line:
    case True{}: value
    case False{}: ""

def value(compact: Bool,line: U32,text: String) -> String:
  match compact:
    case True{}: metadata(U32.is_eq(line,0),text)
    case False{}: text

def encode(compact: Bool,fact: Fact) -> String:
  match fact:
    case Fact{k,v,+l,s,e}: W.row([k,value(compact,l,v),U32.show(l),U32.show(s),U32.show(e)])

def encoded(+compact: Bool,facts: List<&2,Fact>) -> String:
  match facts:
    case Nil{}: ""
    case h <> t: encode(compact,h) ++ encoded(compact,t)
