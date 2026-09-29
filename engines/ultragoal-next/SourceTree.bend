import Base
import ./SourceLex.bend as Lex

type Tree is Data:
  Tree{kind: String, text: String, line: U32, start: U32, end: U32, children: List<&2,Tree>}

type Frame is Data:
  Frame{open: Lex.Token, previous: List<&2,Tree>}

type State is Data:
  State{current: List<&2,Tree>, stack: List<&2,Frame>, depth: U32, error: String}

type Parsed is Data:
  Parsed{trees: List<&2,Tree>, error: String}

def token(value: Lex.Token) -> Tree:
  match value:
    case Lex.Token{k,t,l,s,e}: Tree{k,t,l,s,e,Nil{}}

def leaf(value: Lex.Token,state: State) -> State:
  match state:
    case State{current,stack,depth,error}: State{token(value) <> current,stack,depth,error}

def opened(ok: Bool,value: Lex.Token,current: List<&2,Tree>,stack: List<&2,Frame>,depth: U32,error: String) -> State:
  match ok:
    case True{}: State{Nil{},Frame{value,current} <> stack,(depth + 1 : U32),error}
    case False{}: State{current,stack,depth,"delimiter nesting limit"}

def open(value: Lex.Token,state: State) -> State:
  match state:
    case State{current,stack,+depth,error}: opened(U32.is_lt(depth,64),value,current,stack,depth,error)

def group(open: Lex.Token,close: Lex.Token,children: List<&2,Tree>) -> Tree:
  match open close:
    case Lex.Token{_,t,l,s,_} Lex.Token{_,_,_,_,e}: Tree{"group",t,l,s,e,List.reverse(&2,Tree,children)}

def matching(open: Lex.Token,close: Lex.Token) -> Bool:
  match open close:
    case Lex.Token{_,"(",_,_,_} Lex.Token{_,")",_,_,_}: True{}
    case Lex.Token{_,"[",_,_,_} Lex.Token{_,"]",_,_,_}: True{}
    case Lex.Token{_,"{",_,_,_} Lex.Token{_,"}",_,_,_}: True{}
    case _ _: False{}

def closed(ok: Bool,first: Lex.Token,last: Lex.Token,current: List<&2,Tree>,previous: List<&2,Tree>,stack: List<&2,Frame>,depth: U32,error: String) -> State:
  match ok:
    case True{}: State{group(first,last,current) <> previous,stack,(depth - 1 : U32),error}
    case False{}: State{previous,stack,(depth - 1 : U32),"mismatched closing delimiter"}

def close(+value: Lex.Token,state: State) -> State:
  match state:
    case State{current,Nil{},depth,error}: State{current,Nil{},depth,"unmatched closing delimiter"}
    case State{current,Frame{+first,previous} <> stack,depth,error}: closed(matching(first,value),first,value,current,previous,stack,depth,error)

def symbol(text: String,value: Lex.Token,state: State) -> State:
  match text:
    case "(": open(value,state)
    case "[": open(value,state)
    case "{": open(value,state)
    case ")": close(value,state)
    case "]": close(value,state)
    case "}": close(value,state)
    case _: leaf(value,state)

def add(kind: String,text: String,value: Lex.Token,state: State) -> State:
  match kind:
    case "symbol": symbol(text,value,state)
    case _: leaf(value,state)

def step(value: Lex.Token,state: State) -> State:
  match value:
    case Lex.Token{+k,+t,l,s,e}: add(k,t,Lex.Token{k,t,l,s,e},state)

def finish(state: State) -> Parsed:
  match state:
    case State{trees,Nil{},_,error}: Parsed{List.reverse(&2,Tree,trees),error}
    case State{_,_,_,_}: Parsed{Nil{},"unclosed delimiter"}

def walk(tokens: List<&2,Lex.Token>,state: State) -> Parsed:
  match tokens:
    case Nil{}: finish(state)
    case h <> t: walk(t,step(h,state))

def lexed(input: Lex.Lexed) -> Parsed:
  match input:
    case Lex.Lexed{tokens,""}: walk(tokens,State{Nil{},Nil{},0,""})
    case Lex.Lexed{_,error}: Parsed{Nil{},error}

def parse(text: String,language: String) -> Parsed:
  lexed(Lex.run(text,language))
