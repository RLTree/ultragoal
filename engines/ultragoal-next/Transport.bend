import Base
import ./Wire.bend as W
import ./Facts.bend as F

# Wire interning only: restore exact F rows before any selection or parsing.
type Expanded is Data:
  Expanded{rows: List<&2,List<&2,String>>, error: String}

type Atom is Data:
  Atom{digest: String, source: String, size: U32}

type State is Data:
  State{atoms: Map<&2,Maybe<&2,Atom>>, rows: List<&2,List<&2,String>>, bytes: U32, count: U32, error: String}

def invalid(state: State,message: String) -> State:
  match state:
    case State{atoms,rows,bytes,count,_}: State{atoms,rows,bytes,count,message}

def atom(result: Map<&2,Maybe<&2,Atom>> & Maybe<&2,Atom>,id: String,digest: String,+source: String,rows: List<&2,List<&2,String>>,bytes: U32,count: U32,error: String) -> State:
  match result:
    case (atoms,None{}): State{Map.set(&2,Maybe<&2,Atom>,atoms,id,Some{Atom{digest,source,F.bytes(source)}}),rows,bytes,count,error}
    case (atoms,Some{_}): State{atoms,rows,bytes,count,"duplicate transport atom"}

def referenced(ok: Bool,atoms: Map<&2,Maybe<&2,Atom>>,rows: List<&2,List<&2,String>>,path: String,digest: String,source: String,bytes: U32,count: U32,error: String) -> State:
  match ok:
    case True{}: State{atoms,["F",path,digest,source] <> rows,bytes,count,error}
    case False{}: State{atoms,rows,bytes,count,"invalid or oversized transport reference"}

def found(result: Map<&2,Maybe<&2,Atom>> & Maybe<&2,Atom>,path: String,+digest: String,rows: List<&2,List<&2,String>>,+bytes: U32,+count: U32,error: String) -> State:
  match result:
    case (atoms,Some{Atom{stored,source,+size}}): referenced(String.eq(digest,stored) && U32.is_le((bytes + size : U32),134217728) && U32.is_lt(count,100000),atoms,rows,path,digest,source,(bytes + size : U32),(count + 1 : U32),error)
    case (atoms,_): State{atoms,rows,bytes,count,"missing transport atom"}

def row(fields: List<&2,String>,state: State) -> State:
  match fields state:
    case "D" <> +id <> digest <> source <> Nil{} State{atoms,rows,bytes,count,error}: atom(Map.get(Maybe<&2,Atom>,None{},atoms,id),id,digest,source,rows,bytes,count,error)
    case "F_REF" <> path <> digest <> id <> Nil{} State{atoms,rows,bytes,count,error}: found(Map.get(Maybe<&2,Atom>,None{},atoms,id),path,digest,rows,bytes,count,error)
    case "D" <> _ state: invalid(state,"malformed transport atom")
    case "F_REF" <> _ state: invalid(state,"malformed transport reference")
    case fields State{atoms,rows,bytes,count,error}: State{atoms,fields <> rows,bytes,count,error}

def walk(rows: List<&2,List<&2,String>>,state: State) -> Expanded:
  match rows:
    case Nil{}:
      match state:
        case State{_,out,_,_,error}: Expanded{List.reverse(&2,List<&2,String>,out),error}
    case h <> t: walk(t,row(h,state))

def expand(rows: List<&2,List<&2,String>>) -> Expanded:
  walk(rows,State{MTip{},Nil{},0,0,""})
