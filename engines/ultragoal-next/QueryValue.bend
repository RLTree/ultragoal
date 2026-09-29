import Base
import ./QueryCompute.bend as Q
import ./Equality.bend as E

type Value<-Arg: List<&2,String>> is Data:
  Value{value: String, proof: {value == Q.compute(Arg) : String}, reused: Bool}

type Entry is Data:
  Entry{arg: List<&2,String>, result: Value<arg>}

def fresh(+arg: List<&2,String>) -> Value<arg>:
  Value{Q.compute(arg),{==},False{}}

def value(-arg: List<&2,String>,result: Value<arg>) -> String:
  match result:
    case Value{value,_,_}: value

def reuse(-before: List<&2,String>,-after: List<&2,String>,result: Value<before>,same: {before == after : List<&2,String>}) -> Value<after>:
  match result:
    case Value{+v,proof,_}: Value{v,Equal.trans(String,v,Q.compute(before),Q.compute(after),proof,Equal.cong(List<&2,String>,String,Q.compute,before,after,same)),True{}}

def reused(-arg: List<&2,String>,result: Value<arg>) -> Bool:
  match result:
    case Value{_,_,flag}: flag

def matched(+arg: List<&2,String>,before: List<&2,String>,result: Value<before>,test: Maybe<&2,{before == arg : List<&2,String>}>) -> Value<arg>:
  match test:
    case None{}: fresh(arg)
    case Some{same}: reuse(before,arg,result,same)

def choose(+arg: List<&2,String>,old: Maybe<&2,Entry>) -> Value<arg>:
  match old:
    case None{}: fresh(arg)
    case Some{Entry{+before,result}}: matched(arg,before,result,E.decide_strings(before,arg))

