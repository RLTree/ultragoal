import Base
import ./Facts.bend as F

def compute(args: List<&2,String>) -> String:
  match args:
    case "spans" <> grammar <> source <> Nil{}: F.parse_spans(grammar,source)
    case "inline" <> grammar <> source <> Nil{}: F.parse(grammar,source)
    case _: ""

type Result<-Args: List<&2,String>> is Data:
  Result{value: String, proof: {value == compute(Args) : String}}

def arguments(compact: Bool,grammar: String,source: String) -> List<&2,String>:
  match compact:
    case True{}: ["spans",grammar,source]
    case False{}: ["inline",grammar,source]

def fresh_result(+args: List<&2,String>) -> Result<args>:
  Result{compute(args),{==}}

def value(-args: List<&2,String>,result: Result<args>) -> String:
  match result:
    case Result{text,_}: text

def reuse(-before: List<&2,String>,-after: List<&2,String>,result: Result<before>,same: {before == after : List<&2,String>}) -> Result<after>:
  match result:
    case Result{+text,proof}: Result{text,Equal.trans(String,text,compute(before),compute(after),proof,Equal.cong(List<&2,String>,String,compute,before,after,same))}

