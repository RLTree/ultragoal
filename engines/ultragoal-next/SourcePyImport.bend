import Base
import ./SourceTree.bend as T
import ./FactRecord.bend as R
import ./SourceLex.bend as Lex

type Path is Data:
  Path{target: String, rest: List<&2,T.Tree>, first: T.Tree, last: T.Tree}

def bounded(fuel: Nat,text: String) -> Bool:
  match fuel text:
    case _ SNil{}: True{}
    case 0n _: False{}
    case 1n+p SCon{_,t}: bounded(p,t)

def name(text: String) -> Bool:
  match text:
    case SCon{+h,+t}: Lex.identifier(h) && Bool.not(Char.is_eq(h,'$')) && Bool.not(String.contains(t,"$")) && bounded(512n,t)
    case _: False{}

def joined(ok: Bool,next: String -> T.Tree -> Maybe<&2,Path>,target: String,node: T.Tree) -> Maybe<&2,Path>:
  match ok:
    case True{}: next(target,node)
    case False{}: None{}

def dotted(fuel: Nat,trees: List<&2,T.Tree>,target: String,+first: T.Tree,last: T.Tree) -> Maybe<&2,Path>:
  match fuel trees:
    case 1n+p T.Tree{"symbol",".",_,_,_,_} <> T.Tree{"word",+word,l,s,e,_} <> rest: joined(name(word),text => node => dotted(p,rest,text,first,node),target ++ "." ++ word,T.Tree{"word",word,l,s,e,Nil{}})
    case _ T.Tree{"symbol",".",_,_,_,_} <> _: None{}
    case _ rest: Some{Path{target,rest,first,last}}

def relative(fuel: Nat,trees: List<&2,T.Tree>,prefix: String,+first: T.Tree,last: T.Tree) -> Maybe<&2,Path>:
  match fuel trees:
    case 1n+p T.Tree{"symbol",".",l,s,e,_} <> rest: relative(p,rest,prefix ++ ".",first,T.Tree{"symbol",".",l,s,e,Nil{}})
    case _ T.Tree{"word","import",l,s,e,c} <> rest: Some{Path{prefix,T.Tree{"word","import",l,s,e,c} <> rest,first,last}}
    case _ T.Tree{"word",+word,l,s,e,_} <> rest: joined(name(word),text => node => dotted(128n,rest,text,first,node),prefix ++ word,T.Tree{"word",word,l,s,e,Nil{}})
    case _ _: None{}

def path(trees: List<&2,T.Tree>) -> Maybe<&2,Path>:
  match trees:
    case T.Tree{"word",+word,+l,+s,+e,_} <> rest: joined(name(word),text => node => dotted(128n,rest,text,T.Tree{"word",word,l,s,e,Nil{}},node),word,T.Tree{"word",word,l,s,e,Nil{}})
    case T.Tree{"symbol",".",+l,+s,+e,_} <> rest: relative(32n,rest,".",T.Tree{"symbol",".",l,s,e,Nil{}},T.Tree{"symbol",".",l,s,e,Nil{}})
    case _: None{}

def span(first: T.Tree,last: T.Tree) -> R.Fact:
  match first last:
    case T.Tree{_,_,l,s,_,_} T.Tree{_,_,_,_,e,_}: R.Fact{"structure-import-expression","",l,s,e}

def emitted(ok: Bool,target: String,first: T.Tree,last: T.Tree) -> List<&2,R.Fact>:
  match ok:
    case True{}: [R.Fact{"structure-python-import-target",target,0,0,0},span(first,last)]
    case False{}: [R.Fact{"structure-limitation","Python module path length bound",0,0,0}]

def facts(+target: String,first: T.Tree,last: T.Tree) -> List<&2,R.Fact>:
  emitted(bounded(4096n,target),target,first,last)

def imported_names(trees: List<&2,T.Tree>) -> Bool:
  match trees:
    case T.Tree{"newline",_,_,_,_,_} <> rest: imported_names(rest)
    case T.Tree{"word",_,_,_,_,_} <> _: True{}
    case T.Tree{"symbol","*",_,_,_,_} <> _: True{}
    case T.Tree{"group","(",_,_,_,children} <> _: imported_names(children)
    case _: False{}

def member_fact(ok: Bool,module: String,member: String,binding: String) -> List<&2,R.Fact>:
  match ok:
    case True{}: [R.Fact{"structure-python-import-member",module ++ "|" ++ member ++ "|" ++ binding,0,0,0}]
    case False{}: [R.Fact{"structure-limitation","Python imported member outside supported form",0,0,0}]

def newline_after(inside: Bool,rest: List<&2,T.Tree>,next: List<&2,T.Tree> -> List<&2,R.Fact>) -> List<&2,R.Fact>:
  match inside:
    case True{}: next(rest)
    case False{}: Nil{}

def member_after(inside: Bool,trees: List<&2,T.Tree>,next: List<&2,T.Tree> -> List<&2,R.Fact>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"symbol",",",_,_,_,_} <> rest: next(rest)
    case T.Tree{"newline",_,_,_,_,_} <> rest: newline_after(inside,rest,next)
    case Nil{}: Nil{}
    case _: [R.Fact{"structure-limitation","Python imported member tail outside supported form",0,0,0}]

def member_emitted(ok: Bool,module: String,member: String,binding: String,inside: Bool,rest: List<&2,T.Tree>,next: List<&2,T.Tree> -> List<&2,R.Fact>) -> List<&2,R.Fact>:
  List.append(&2,R.Fact,member_fact(ok,module,member,binding),member_after(inside,rest,next))

def grouped(facts: List<&2,R.Fact>,tail: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match tail:
    case Nil{}: facts
    case T.Tree{"newline",_,_,_,_,_} <> _: facts
    case T.Tree{"symbol",";",_,_,_,_} <> _: facts
    case _: List.append(&2,R.Fact,facts,[R.Fact{"structure-limitation","Python parenthesized import tail outside supported form",0,0,0}])

def members(fuel: Nat,+module: String,+inside: Bool,trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match fuel trees:
    case 0n _: [R.Fact{"structure-limitation","Python imported member work bound",0,0,0}]
    case _ Nil{}: Nil{}
    case 1n+p T.Tree{"newline",_,_,_,_,_} <> rest: newline_after(inside,rest,xs => members(p,module,inside,xs))
    case 1n+p T.Tree{"group","(",_,_,_,children} <> rest: grouped(members(p,module,True{},children),rest)
    case 1n+p T.Tree{"word",+word,_,_,_,_} <> T.Tree{"word","as",_,_,_,_} <> T.Tree{"word",+alias,_,_,_,_} <> rest:
      member_emitted(name(word) && name(alias),module,word,alias,inside,rest,xs => members(p,module,inside,xs))
    case 1n+p T.Tree{"word",+word,_,_,_,_} <> rest:
      member_emitted(name(word),module,word,word,inside,rest,xs => members(p,module,inside,xs))
    case _ _: [R.Fact{"structure-limitation","Python imported member outside supported form",0,0,0}]

def from_path(result: Maybe<&2,Path>) -> List<&2,R.Fact>:
  match result:
    case Some{Path{+target,T.Tree{"word","import",_,_,_,_} <> +rest,first,last}}:
      List.append(&2,R.Fact,emitted(imported_names(rest) && bounded(4096n,target),target,first,last),members(128n,target,False{},rest))
    case _: [R.Fact{"structure-limitation","Python from-import outside supported form",0,0,0}]

def from(trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  from_path(path(trees))

def rest(trees: List<&2,T.Tree>) -> List<&2,T.Tree>:
  match trees:
    case T.Tree{"word","as",_,_,_,_} <> T.Tree{"word",_,_,_,_,_} <> remaining: remaining
    case remaining: remaining

def follow(trees: List<&2,T.Tree>,next: List<&2,T.Tree> -> List<&2,R.Fact>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"symbol",",",_,_,_,_} <> remaining: next(remaining)
    case Nil{}: Nil{}
    case T.Tree{"newline",_,_,_,_,_} <> _: Nil{}
    case T.Tree{"symbol",";",_,_,_,_} <> _: Nil{}
    case _: [R.Fact{"structure-limitation","Python import tail outside supported form",0,0,0}]

def import_path(result: Maybe<&2,Path>,next: List<&2,T.Tree> -> List<&2,R.Fact>) -> List<&2,R.Fact>:
  match result:
    case Some{Path{+target,remaining,first,last}}: List.append(&2,R.Fact,emitted(Bool.not(String.starts_with(target,".")) && bounded(4096n,target),target,first,last),follow(rest(remaining),next))
    case _: [R.Fact{"structure-limitation","Python import path outside supported form",0,0,0}]

def imports(fuel: Nat) -> List<&2,T.Tree> -> List<&2,R.Fact>:
  match fuel:
    case 0n: trees => [R.Fact{"structure-limitation","Python import list work bound",0,0,0}]
    case 1n+p: trees => import_path(path(trees),imports(p))
