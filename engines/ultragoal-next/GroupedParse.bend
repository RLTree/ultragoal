import Base
import ./Facts.bend as F
import ./ParsedValue.bend as P
import ./Equality.bend as E
import ./Wire.bend as W
import ./Identity.bend as Identity

type Request is Data:
  Request{path: String,digest: String,compact: Bool,source: String}

def request_args(request: Request) -> List<&2,String>:
  match request:
    case Request{path,_,compact,source}: P.arguments(compact,F.grammar(path),source)

def request(+row: List<&2,String>,compact: Bool) -> Request:
  Request{W.field(row,1n),W.field(row,2n),compact,W.field(row,3n)}

type Alias<-Args: List<&2,String>> is Data:
  Alias{request: Request,same: {Args == request_args(request) : List<&2,String>},first: Bool}

type Group is Data:
  Group{key: String, args: List<&2,String>, aliases: List<&2,Alias<args>>, weight: U32}

type Plan is Data:
  Plan{groups: Map<&2,Maybe<&2,Group>>, keys: List<&2,String>, valid: Bool}

type Bound is Data:
  Bound{path: String, row: List<&2,String>}

def choose(flag: Bool,a: String,b: String) -> String:
  match flag:
    case True{}: a
    case False{}: b

def key(+row: List<&2,String>,compact: Bool) -> String:
  W.row([Identity.parsers(),F.grammar(W.field(row,1n)),W.field(row,2n),choose(compact,"source-spans/2","inline/1")])

def first(+row: List<&2,String>,k: String,compact: Bool) -> Group:
  +req = request(row,compact)
  Group{k,request_args(req),[Alias{req,{==},True{}}],F.bytes(W.field(row,3n))}

def joined(+canonical: List<&2,String>,req: Request,proof: Maybe<&2,{canonical == request_args(req) : List<&2,String>}>,+k: String,aliases: List<&2,Alias<canonical>>,weight: U32,groups: Map<&2,Maybe<&2,Group>>,keys: List<&2,String>,valid: Bool) -> Plan:
  match proof:
    case None{}: Plan{groups,keys,False{}}
    case Some{same}: Plan{Map.set(&2,Maybe<&2,Group>,groups,k,Some{Group{k,canonical,Alias{req,same,False{}} <> aliases,weight}}),keys,valid}

def existing(group: Group,+req: Request,groups: Map<&2,Maybe<&2,Group>>,keys: List<&2,String>,valid: Bool) -> Plan:
  match group:
    case Group{k,+canonical,aliases,weight}: joined(canonical,req,E.decide_strings(canonical,request_args(req)),k,aliases,weight,groups,keys,valid)

def found(hit: Map<&2,Maybe<&2,Group>> & Maybe<&2,Group>,+k: String,row: List<&2,String>,compact: Bool,keys: List<&2,String>,valid: Bool) -> Plan:
  match hit:
    case (groups,None{}): Plan{Map.set(&2,Maybe<&2,Group>,groups,k,Some{first(row,k,compact)}),k <> keys,valid}
    case (groups,Some{group}): existing(group,request(row,compact),groups,keys,valid)

def add(plan: Plan,+row: List<&2,String>,+compact: Bool) -> Plan:
  match plan:
    case Plan{groups,keys,valid}: found(Map.get(Maybe<&2,Group>,None{},groups,key(row,compact)),key(row,compact),row,compact,keys,valid)

def collect(rows: List<&2,List<&2,String>>,+compact: Bool,plan: Plan) -> Plan:
  match rows:
    case Nil{}: plan
    case h <> t: collect(t,compact,add(plan,h,compact))

def alias_value(-canonical: List<&2,String>,result: P.Result<canonical>,+requested: List<&2,String>,same: {canonical == requested : List<&2,String>}) -> String:
  P.value(requested,P.reuse(canonical,requested,result,same))

def bind_request(req: Request,value: String,first: Bool,k: String) -> Bound:
  match req:
    case Request{+path,digest,compact,source}: Bound{path,[choose(compact,"P_SPANS","P"),path,digest,F.grammar(path),value,choose(first,"parsed","reused"),k]}

def bind(-canonical: List<&2,String>,result: P.Result<canonical>,alias: Alias<canonical>,k: String) -> Bound:
  match alias:
    case Alias{+req,same,first}: bind_request(req,alias_value(canonical,result,request_args(req),same),first,k)

def requested(-canonical: List<&2,String>,alias: Alias<canonical>) -> List<&2,String>:
  match alias:
    case Alias{req,_,_}: request_args(req)

def alias_request(-canonical: List<&2,String>,alias: Alias<canonical>) -> Request:
  match alias:
    case Alias{req,_,_}: req

def alias_first(-canonical: List<&2,String>,alias: Alias<canonical>) -> Bool:
  match alias:
    case Alias{_,_,first}: first

def row(bound: Bound) -> List<&2,String>:
  match bound:
    case Bound{_,row}: row

def bound_record(bound: Bound) -> List<&2,String>:
  match bound:
    case Bound{path,row}: path <> row

def blob(bound: Bound) -> String:
  W.field(row(bound),4n)

def take_group(value: Map<&2,Maybe<&2,Group>> & Maybe<&2,Group>,tail: List<&2,Group>) -> List<&2,Group>:
  match value:
    case (_,Some{g}): g <> tail
    case (_,None{}): tail

def jobs(keys: List<&2,String>,+groups: Map<&2,Maybe<&2,Group>>) -> List<&2,Group>:
  match keys:
    case Nil{}: Nil{}
    case h <> t: take_group(Map.get(Maybe<&2,Group>,None{},groups,h),jobs(t,groups))

def count(groups: List<&2,Group>) -> Nat:
  match groups:
    case Nil{}: 0n
    case _ <> t: 1n+count(t)

type Buckets is Data:
  Buckets{left: List<&2,Group>,right: List<&2,Group>,wl: U32,wr: U32}

def weight(group: Group) -> U32:
  match group:
    case Group{_,_,_,0}: 1
    case Group{_,_,_,w}: w

def total(groups: List<&2,Group>) -> U32:
  match groups:
    case Nil{}: 0
    case h <> t: (weight(h) + total(t) : U32)

def place(left: Bool,+group: Group,buckets: Buckets) -> Buckets:
  match left buckets:
    case True{} Buckets{l,r,wl,wr}: Buckets{group <> l,r,(wl + weight(group) : U32),wr}
    case False{} Buckets{l,r,wl,wr}: Buckets{l,group <> r,wl,(wr + weight(group) : U32)}

def balance(group: Group,buckets: Buckets) -> Buckets:
  match buckets:
    case Buckets{l,r,+wl,+wr}: place(U32.is_le(wl,wr),group,Buckets{l,r,wl,wr})

def split(groups: List<&2,Group>,buckets: Buckets) -> List<&2,Group> & List<&2,Group>:
  match groups:
    case Nil{}:
      match buckets:
        case Buckets{l,r,_,_}: (l,r)
    case h <> t: split(t,balance(h,buckets))

def materialize(rows: List<&2,Bound>,out: Map<&2,Maybe<&2,List<&2,String>>>) -> Map<&2,Maybe<&2,List<&2,String>>>:
  match rows:
    case Nil{}: out
    case Bound{path,row} <> t: materialize(t,Map.set(&2,Maybe<&2,List<&2,String>>,out,path,Some{row}))

