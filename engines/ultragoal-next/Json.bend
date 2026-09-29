import Base

type Json is Data:
  Null{}
  Boolean{value: Bool}
  Number{lexical: String}
  Text{value: String}
  Array{values: List<&2,Json>}
  Object{fields: List<&2,Sigma<&2,&2,String,_ => Json>>}

type Token is Data:
  Literal{value: String}
  Quoted{value: String}
  Symbol{value: Char}

type Mode is Data:
  Bare{}
  Quote{}
  Escape{}
  Unicode{left: Nat,code: U32,high: U32}
  LowSlash{high: U32}
  LowU{high: U32}

type Lexer is Data:
  Lexer{mode: Mode,buffer: String,tokens: List<&2,Token>,valid: Bool}

def append_atom(buffer: String,tokens: List<&2,Token>) -> List<&2,Token>:
  match buffer:
    case SNil{}: tokens
    case b: Literal{String.reverse(b)} <> tokens

def hex(c: Char) -> Maybe<&2,U32>:
  match c:
    case '0': Some{0}
    case '1': Some{1}
    case '2': Some{2}
    case '3': Some{3}
    case '4': Some{4}
    case '5': Some{5}
    case '6': Some{6}
    case '7': Some{7}
    case '8': Some{8}
    case '9': Some{9}
    case 'a': Some{10}
    case 'b': Some{11}
    case 'c': Some{12}
    case 'd': Some{13}
    case 'e': Some{14}
    case 'f': Some{15}
    case 'A': Some{10}
    case 'B': Some{11}
    case 'C': Some{12}
    case 'D': Some{13}
    case 'E': Some{14}
    case 'F': Some{15}
    case _: None{}

def unicode_high(is_high: Bool,+code: U32,b: String,ts: List<&2,Token>,valid: Bool) -> Lexer:
  match is_high:
    case True{}: Lexer{LowSlash{code},b,ts,valid}
    case False{}: Lexer{Quote{},SCon{Chr{code},b},ts,valid && (U32.is_lt(code,56320) || U32.is_gt(code,57343))}

def unicode_pair(paired: Bool,+code: U32,high: U32,b: String,ts: List<&2,Token>,valid: Bool) -> Lexer:
  match paired:
    case False{}: unicode_high(U32.is_ge(code,55296) && U32.is_le(code,56319),code,b,ts,valid)
    case True{}: Lexer{Quote{},SCon{Chr{(65536 + (high - 55296) * 1024 + code - 56320 : U32)},b},ts,valid && U32.is_ge(code,56320) && U32.is_le(code,57343)}

def unicode_step(left: Nat,code: U32,+high: U32,b: String,ts: List<&2,Token>,valid: Bool) -> Lexer:
  match left:
    case 0n: unicode_pair(U32.is_gt(high,0),code,high,b,ts,valid)
    case n: Lexer{Unicode{n,code,high},b,ts,valid}

def unicode_digit(v: Maybe<&2,U32>,left: Nat,code: U32,high: U32,b: String,ts: List<&2,Token>,valid: Bool) -> Lexer:
  match v left:
    case Some{x} 1n+p: unicode_step(p,(code * 16 + x : U32),high,b,ts,valid)
    case _ _: Lexer{Bare{},b,ts,False{}}

def bare(c: Char,b: String,ts: List<&2,Token>,valid: Bool) -> Lexer:
  match c:
    case ' ': Lexer{Bare{},"",append_atom(b,ts),valid}
    case '\t': Lexer{Bare{},"",append_atom(b,ts),valid}
    case '\r': Lexer{Bare{},"",append_atom(b,ts),valid}
    case '\n': Lexer{Bare{},"",append_atom(b,ts),valid}
    case '"': Lexer{Quote{},"",append_atom(b,ts),valid}
    case '{': Lexer{Bare{},"",Symbol{'{'} <> append_atom(b,ts),valid}
    case '}': Lexer{Bare{},"",Symbol{'}'} <> append_atom(b,ts),valid}
    case '[': Lexer{Bare{},"",Symbol{'['} <> append_atom(b,ts),valid}
    case ']': Lexer{Bare{},"",Symbol{']'} <> append_atom(b,ts),valid}
    case ':': Lexer{Bare{},"",Symbol{':'} <> append_atom(b,ts),valid}
    case ',': Lexer{Bare{},"",Symbol{','} <> append_atom(b,ts),valid}
    case h: Lexer{Bare{},SCon{h,b},ts,valid}

def quoted(c: Char,b: String,ts: List<&2,Token>,valid: Bool) -> Lexer:
  match c:
    case '"': Lexer{Bare{},"",Quoted{String.reverse(b)} <> ts,valid}
    case '\\': Lexer{Escape{},b,ts,valid}
    case +h: Lexer{Quote{},SCon{h,b},ts,valid && U32.is_ge(Char.to_u32(h),32)}

def escaped(c: Char,b: String,ts: List<&2,Token>,valid: Bool) -> Lexer:
  match c:
    case '"': Lexer{Quote{},SCon{'"',b},ts,valid}
    case '\\': Lexer{Quote{},SCon{'\\',b},ts,valid}
    case '/': Lexer{Quote{},SCon{'/',b},ts,valid}
    case 'b': Lexer{Quote{},SCon{Chr{8},b},ts,valid}
    case 'f': Lexer{Quote{},SCon{Chr{12},b},ts,valid}
    case 'n': Lexer{Quote{},SCon{'\n',b},ts,valid}
    case 'r': Lexer{Quote{},SCon{'\r',b},ts,valid}
    case 't': Lexer{Quote{},SCon{'\t',b},ts,valid}
    case 'u': Lexer{Unicode{4n,0,0},b,ts,valid}
    case _: Lexer{Quote{},b,ts,False{}}

def lex_step(c: Char,state: Lexer) -> Lexer:
  match state:
    case Lexer{Bare{},b,ts,v}: bare(c,b,ts,v)
    case Lexer{Quote{},b,ts,v}: quoted(c,b,ts,v)
    case Lexer{Escape{},b,ts,v}: escaped(c,b,ts,v)
    case Lexer{Unicode{n,code,high},b,ts,v}: unicode_digit(hex(c),n,code,high,b,ts,v)
    case Lexer{LowSlash{high},b,ts,v}: Lexer{LowU{high},b,ts,v && Char.is_eq(c,'\\')}
    case Lexer{LowU{high},b,ts,v}: Lexer{Unicode{4n,0,high},b,ts,v && Char.is_eq(c,'u')}

def lex(text: String,state: Lexer) -> Lexer:
  match text:
    case SNil{}: state
    case SCon{h,t}: lex(t,lex_step(h,state))

def select(ok: Bool,a: U32,b: U32) -> U32:
  match ok:
    case True{}: a
    case False{}: b

def numdigit(state: U32,+c: Char) -> U32:
  match state:
    case 0: select(Char.is_digit(c),select(Char.is_eq(c,'0'),2,3),99)
    case 1: select(Char.is_digit(c),select(Char.is_eq(c,'0'),2,3),99)
    case 3: select(Char.is_digit(c),3,99)
    case 4: select(Char.is_digit(c),5,99)
    case 5: select(Char.is_digit(c),5,99)
    case 6: select(Char.is_digit(c),8,99)
    case 7: select(Char.is_digit(c),8,99)
    case 8: select(Char.is_digit(c),8,99)
    case _: 99

def numstep(+state: U32,c: Char) -> U32:
  match c:
    case '-': select(U32.is_eq(state,0),1,select(U32.is_eq(state,6),7,99))
    case '+': select(U32.is_eq(state,6),7,99)
    case '.': select(U32.is_eq(state,2) || U32.is_eq(state,3),4,99)
    case 'e': select(U32.is_eq(state,2) || U32.is_eq(state,3) || U32.is_eq(state,5),6,99)
    case 'E': select(U32.is_eq(state,2) || U32.is_eq(state,3) || U32.is_eq(state,5),6,99)
    case h: numdigit(state,h)

def number(text: String,+state: U32) -> Bool:
  match text:
    case SNil{}: U32.is_eq(state,2) || U32.is_eq(state,3) || U32.is_eq(state,5) || U32.is_eq(state,8)
    case SCon{h,t}: number(t,numstep(state,h))

def numeric(ok: Bool,s: String) -> Maybe<&2,Json>:
  match ok:
    case True{}: Some{Number{s}}
    case False{}: None{}

def atom(s: String) -> Maybe<&2,Json>:
  match s:
    case "true": Some{Boolean{True{}}}
    case "false": Some{Boolean{False{}}}
    case "null": Some{Null{}}
    case +text: numeric(number(text,0),text)

type Frame is Data:
  ArrayFrame{values: List<&2,Json>,stage: U32}
  ObjectFrame{fields: List<&2,Sigma<&2,&2,String,_ => Json>>,key: String,stage: U32}

type Parser is Data:
  Parser{root: Maybe<&2,Json>,stack: List<&2,Frame>,valid: Bool}

def has(fields: List<&2,Sigma<&2,&2,String,_ => Json>>,+key: String) -> Bool:
  match fields:
    case Nil{}: False{}
    case (k,v) <> t: String.eq(k,key) || has(t,key)

def attach(value: Json,root: Maybe<&2,Json>,stack: List<&2,Frame>,valid: Bool) -> Parser:
  match root stack:
    case None{} Nil{}: Parser{Some{value},Nil{},valid}
    case r Nil{}: Parser{r,Nil{},False{}}
    case r ArrayFrame{vs,+stage} <> t: Parser{r,ArrayFrame{value <> vs,1} <> t,valid && (U32.is_eq(stage,0) || U32.is_eq(stage,2))}
    case r ObjectFrame{fs,k,stage} <> t: Parser{r,ObjectFrame{(k,value) <> fs,"",3} <> t,valid && U32.is_eq(stage,2)}

def value_expected(root: Maybe<&2,Json>,stack: List<&2,Frame>) -> Bool:
  match root stack:
    case None{} Nil{}: True{}
    case _ ArrayFrame{_,+s} <> _: U32.is_eq(s,0) || U32.is_eq(s,2)
    case _ ObjectFrame{_,_,s} <> _: U32.is_eq(s,2)
    case _ _: False{}

def depth(stack: List<&2,Frame>) -> U32:
  match stack:
    case Nil{}: 0
    case h <> t: (1 + depth(t) : U32)

def push_ok(ok: Bool,f: Frame,root: Maybe<&2,Json>,stack: List<&2,Frame>,valid: Bool) -> Parser:
  match ok:
    case True{}: Parser{root,f <> stack,valid}
    case False{}: Parser{root,stack,False{}}

def push(f: Frame,+root: Maybe<&2,Json>,+stack: List<&2,Frame>,valid: Bool) -> Parser:
  push_ok(value_expected(root,stack) && U32.is_lt(depth(stack),64),f,root,stack,valid)

def key_or_value(is_key: Bool,+value: String,root: Maybe<&2,Json>,+fields: List<&2,Sigma<&2,&2,String,_ => Json>>,key: String,stage: U32,tail: List<&2,Frame>,valid: Bool) -> Parser:
  match is_key:
    case True{}: Parser{root,ObjectFrame{fields,value,1} <> tail,valid && Bool.not(has(fields,value))}
    case False{}: attach(Text{value},root,ObjectFrame{fields,key,stage} <> tail,valid)

def text(value: String,state: Parser) -> Parser:
  match state:
    case Parser{r,ObjectFrame{fs,k,+stage} <> t,v}: key_or_value(U32.is_eq(stage,0) || U32.is_eq(stage,4),value,r,fs,k,stage,t,v)
    case Parser{r,stack,v}: attach(Text{value},r,stack,v)

def literal(value: Maybe<&2,Json>,state: Parser) -> Parser:
  match value state:
    case Some{x} Parser{r,stack,v}: attach(x,r,stack,v)
    case None{} Parser{r,stack,v}: Parser{r,stack,False{}}

def symbol(c: Char,state: Parser) -> Parser:
  match c state:
    case '[' Parser{r,stack,v}: push(ArrayFrame{Nil{},0},r,stack,v)
    case '{' Parser{r,stack,v}: push(ObjectFrame{Nil{},"",0},r,stack,v)
    case ']' Parser{r,ArrayFrame{vs,+stage} <> t,v}: attach(Array{List.reverse(&2,Json,vs)},r,t,v && (U32.is_eq(stage,0) || U32.is_eq(stage,1)))
    case '}' Parser{r,ObjectFrame{fs,k,+stage} <> t,v}: attach(Object{List.reverse(&2,Sigma<&2,&2,String,_ => Json>,fs)},r,t,v && (U32.is_eq(stage,0) || U32.is_eq(stage,3)))
    case ',' Parser{r,ArrayFrame{vs,s} <> t,v}: Parser{r,ArrayFrame{vs,2} <> t,v && U32.is_eq(s,1)}
    case ',' Parser{r,ObjectFrame{fs,k,s} <> t,v}: Parser{r,ObjectFrame{fs,k,4} <> t,v && U32.is_eq(s,3)}
    case ':' Parser{r,ObjectFrame{fs,k,s} <> t,v}: Parser{r,ObjectFrame{fs,k,2} <> t,v && U32.is_eq(s,1)}
    case _ Parser{r,stack,v}: Parser{r,stack,False{}}

def token(t: Token,state: Parser) -> Parser:
  match t:
    case Quoted{s}: text(s,state)
    case Literal{s}: literal(atom(s),state)
    case Symbol{c}: symbol(c,state)

def tokens(ts: List<&2,Token>,state: Parser) -> Parser:
  match ts:
    case Nil{}: state
    case h <> t: tokens(t,token(h,state))

def finish(state: Parser) -> Maybe<&2,Json>:
  match state:
    case Parser{Some{v},Nil{},True{}}: Some{v}
    case _: None{}

def parsed(state: Lexer) -> Maybe<&2,Json>:
  match state:
    case Lexer{Bare{},b,ts,True{}}: finish(tokens(List.reverse(&2,Token,append_atom(b,ts)),Parser{None{},Nil{},True{}}))
    case _: None{}

def parse(input: String) -> Maybe<&2,Json>:
  parsed(lex(input,Lexer{Bare{},"",Nil{},True{}}))

def valid(value: Maybe<&2,Json>) -> Bool:
  match value:
    case Some{_}: True{}
    case None{}: False{}
