import Base
import ./SourceTree.bend as T

# Python's supported indentation profile uses spaces and LF/CRLF. Token groups
# already hide physical newlines inside brackets and multi-line literals.
type Frame is Data:
  Frame{indent: U32, previous: List<&2,T.Tree>, line: U32, start: U32}

type State is Data:
  State{current: List<&2,T.Tree>, stack: List<&2,Frame>, indent: U32, depth: U32, line_start: U32, at_start: Bool, error: String}

def pending(trees: List<&2,T.Tree>) -> Bool:
  match trees:
    case T.Tree{"newline",_,_,_,_,_} <> t: pending(t)
    case T.Tree{"symbol",":",_,_,_,_} <> _: True{}
    case _: False{}

def end(trees: List<&2,T.Tree>) -> U32:
  match trees:
    case T.Tree{_,_,_,_,e,_} <> _: e
    case _: 0

def line(tree: T.Tree) -> U32:
  match tree:
    case T.Tree{_,_,l,_,_,_}: l

def start(tree: T.Tree) -> U32:
  match tree:
    case T.Tree{_,_,_,s,_,_}: s

def fail(state: State,message: String) -> State:
  match state:
    case State{current,stack,indent,depth,base,at,_}: State{current,stack,indent,depth,base,at,message}

def append(tree: T.Tree,state: State) -> State:
  match state:
    case State{current,stack,indent,depth,base,_,error}: State{tree <> current,stack,indent,depth,base,False{},error}

def newline(+tree: T.Tree,state: State) -> State:
  match state:
    case State{current,stack,indent,depth,_,_,error}: State{tree <> current,stack,indent,depth,end([tree]),True{},error}

def equal(pending: Bool,tree: T.Tree,state: State) -> State:
  match pending:
    case True{}: fail(state,"expected an indented suite")
    case False{}: append(tree,state)

def open(ok: Bool,col: U32,+tree: T.Tree,state: State) -> State:
  match ok:
    case False{}: fail(state,"unexpected indentation or suite nesting limit")
    case True{}:
      match state:
        case State{current,stack,indent,depth,base,_,error}: State{[tree],Frame{indent,current,line(tree),start(tree)} <> stack,col,(depth + 1 : U32),base,False{},error}

def close(state: State) -> State:
  match state:
    case State{+current,Frame{outer,previous,l,s} <> stack,_,depth,base,at,error}: State{T.Tree{"suite","",l,s,end(current),List.reverse(&2,T.Tree,current)} <> previous,stack,outer,(depth - 1 : U32),base,at,error}
    case state: fail(state,"inconsistent dedent")

def dedented(required: Bool,next: State -> T.Tree -> State,state: State,tree: T.Tree) -> State:
  match required:
    case True{}: fail(state,"expected an indented suite before dedent")
    case False{}: next(close(state),tree)

def compare(order: Cmp,next: State -> T.Tree -> State,col: U32,tree: T.Tree,+state: State,+current: List<&2,T.Tree>,depth: U32) -> State:
  match order:
    case EQ{}: equal(pending(current),tree,state)
    case GT{}: open(pending(current) && U32.is_lt(depth,64),col,tree,state)
    case LT{}: dedented(pending(current),next,state,tree)

def aligned(next: State -> T.Tree -> State,+col: U32,+state: State,tree: T.Tree) -> State:
  match state:
    case State{+current,stack,+indent,+depth,base,at,error}: compare(U32.cmp(col,indent),next,col,tree,State{current,stack,indent,depth,base,at,error},current,depth)

def align(fuel: Nat,+col: U32) -> State -> T.Tree -> State:
  match fuel:
    case 0n: state => tree => fail(state,"inconsistent dedent or nesting limit")
    case 1n+p: state => tree => aligned(align(p,col),col,state,tree)

def first(at: Bool,+tree: T.Tree,+state: State,base: U32) -> State:
  match at:
    case True{}: align(65n,(start(tree) - base : U32),state,tree)
    case False{}: append(tree,state)

def ordinary(tree: T.Tree,state: State) -> State:
  match state:
    case State{current,stack,indent,depth,+base,+at,error}: first(at,tree,State{current,stack,indent,depth,base,at,error},base)

def step(kind: String,tree: T.Tree,state: State) -> State:
  match kind:
    case "newline": newline(tree,state)
    case _: ordinary(tree,state)

def add(tree: T.Tree,state: State) -> State:
  match tree:
    case T.Tree{+k,t,l,s,e,c}: step(k,T.Tree{k,t,l,s,e,c},state)

def walk(trees: List<&2,T.Tree>,state: State) -> State:
  match trees:
    case Nil{}: state
    case h <> t: walk(t,add(h,state))

def finish(fuel: Nat,state: State) -> T.Parsed:
  match fuel state:
    case _ State{current,Nil{},_,_,_,_,error}: T.Parsed{List.reverse(&2,T.Tree,current),error}
    case 0n _: T.Parsed{Nil{},"suite nesting limit"}
    case 1n+p state: finish(p,close(state))

def eof(pending: Bool,state: State) -> T.Parsed:
  match pending:
    case True{}: T.Parsed{Nil{},"expected an indented suite at EOF"}
    case False{}: finish(65n,state)

def completed(state: State) -> T.Parsed:
  match state:
    case State{+current,stack,indent,depth,base,at,error}: eof(pending(current),State{current,stack,indent,depth,base,at,error})

def parsed(input: T.Parsed) -> T.Parsed:
  match input:
    case T.Parsed{trees,""}: completed(walk(trees,State{Nil{},Nil{},0,0,0,True{},""}))
    case result: result

def run(language: String,input: T.Parsed) -> T.Parsed:
  match language:
    case "python": parsed(input)
    case _: input
