import Base

type Token is Data:
  Token{kind: String, text: String, line: U32, start: U32, end: U32}

type Piece is Data:
  Piece{text: String, rest: String, bytes: U32, lines: U32, error: String}

type Lexed is Data:
  Lexed{tokens: List<&2,Token>, error: String}

def choose(ok: Bool,a: U32,b: U32) -> U32:
  match ok:
    case True{}: a
    case False{}: b

def width(c: Char) -> U32:
  match c:
    case Chr{+n}: choose(U32.is_le(n,127),1,choose(U32.is_le(n,2047),2,choose(U32.is_le(n,65535),3,4)))

def identifier(c: Char) -> Bool:
  match c:
    case Chr{+n}: (U32.is_ge(n,65) && U32.is_le(n,90)) || (U32.is_ge(n,97) && U32.is_le(n,122)) || U32.is_eq(n,95) || U32.is_eq(n,36)

def digit(c: Char) -> Bool:
  match c:
    case Chr{+n}: U32.is_ge(n,48) && U32.is_le(n,57)

def word_step(ok: Bool,next: String -> U32 -> Piece,rev: String,+c: Char,rest: String,bytes: U32) -> Piece:
  match ok:
    case True{}: next(SCon{c,rev},(bytes + width(c) : U32))
    case False{}: Piece{String.reverse(rev),SCon{c,rest},bytes,0,""}

def word(input: String) -> String -> U32 -> Piece:
  match input:
    case SNil{}: rev => bytes => Piece{String.reverse(rev),"",bytes,0,""}
    case SCon{+c,+t}: rev => bytes => word_step(identifier(c) || digit(c),word(t),rev,c,t,bytes)

def line_comment(input: String,bytes: U32) -> Piece:
  match input:
    case SNil{}: Piece{"","",bytes,0,""}
    case SCon{'\n',t}: Piece{"",SCon{'\n',t},bytes,0,""}
    case SCon{c,t}: line_comment(t,(bytes + width(c) : U32))

def block_closed(depth: Nat,next: Nat -> U32 -> U32 -> Piece,rest: String,bytes: U32,lines: U32) -> Piece:
  match depth:
    case 0n: Piece{"",rest,bytes,lines,""}
    case 1n+p: next(p,bytes,lines)

def nested_open(nested: Bool,next: Nat -> U32 -> U32 -> Piece,depth: Nat,bytes: U32,lines: U32) -> Piece:
  match nested:
    case True{}: next(1n+depth,(bytes + 2 : U32),lines)
    case False{}: next(depth,(bytes + 2 : U32),lines)

def block_comment(input: String,+nested: Bool) -> Nat -> U32 -> U32 -> Piece:
  match input:
    case SNil{}: depth => bytes => lines => Piece{"","",bytes,lines,"unterminated block comment"}
    case SCon{'/',SCon{'*',t}}: depth => bytes => lines => nested_open(nested,block_comment(t,nested),depth,bytes,lines)
    case SCon{'*',SCon{'/',+t}}: depth => bytes => lines => block_closed(depth,block_comment(t,nested),t,(bytes + 2 : U32),lines)
    case SCon{'\n',t}: depth => bytes => lines => block_comment(t,nested,depth,(bytes + 1 : U32),(lines + 1 : U32))
    case SCon{c,t}: depth => bytes => lines => block_comment(t,nested,depth,(bytes + width(c) : U32),lines)

def quoted_step(closed: Bool,escaped: Bool,next: Bool -> String -> U32 -> Piece,rev: String,+c: Char,rest: String,bytes: U32) -> Piece:
  match closed escaped:
    case True{} _: Piece{String.reverse(SCon{c,rev}),rest,(bytes + width(c) : U32),0,""}
    case False{} True{}: next(False{},SCon{c,rev},(bytes + width(c) : U32))
    case False{} False{}: next(Char.is_eq(c,'\\'),SCon{c,rev},(bytes + width(c) : U32))

def quoted(input: String,+quote: Char) -> Bool -> String -> U32 -> Piece:
  match input:
    case SNil{}: escaped => rev => bytes => Piece{String.reverse(rev),"",bytes,0,"unterminated quote or unsupported lifetime/raw string"}
    case SCon{'\n',t}: escaped => rev => bytes => Piece{String.reverse(rev),t,(bytes + 1 : U32),1,"multiline quote requires a supported literal form"}
    case SCon{+c,+t}: +escaped => rev => bytes => quoted_step(Char.is_eq(c,quote) && Bool.not(escaped),escaped,quoted(t,quote),rev,c,t,bytes)

def triple_close(rest: String,c: Char,rev: String,bytes: U32,lines: U32) -> Piece:
  match rest:
    case SCon{b,SCon{a,t}}: Piece{String.reverse(SCon{a,SCon{b,SCon{c,rev}}}),t,(bytes + 3 : U32),lines,""}
    case _: Piece{"","",bytes,lines,"truncated triple quote"}

def triple_step(closed: Bool,escaped: Bool,next: Bool -> String -> U32 -> U32 -> Piece,rev: String,+c: Char,rest: String,bytes: U32,lines: U32) -> Piece:
  match closed escaped:
    case True{} _: triple_close(rest,c,rev,bytes,lines)
    case False{} True{}: next(False{},SCon{c,rev},(bytes + width(c) : U32),(lines + choose(Char.is_eq(c,'\n'),1,0) : U32))
    case False{} False{}: next(Char.is_eq(c,'\\'),SCon{c,rev},(bytes + width(c) : U32),(lines + choose(Char.is_eq(c,'\n'),1,0) : U32))

def triple(input: String,+quote: Char) -> Bool -> String -> U32 -> U32 -> Piece:
  match input:
    case SNil{}: escaped => rev => bytes => lines => Piece{String.reverse(rev),"",bytes,lines,"unterminated triple quote"}
    case SCon{+c,+t}: +escaped => rev => bytes => lines => triple_step(Char.is_eq(c,quote) && Bool.not(escaped) && String.starts_with(t,SCon{quote,SCon{quote,""}}),escaped,triple(t,quote),rev,c,t,bytes,lines)

def emitted(kind: String,+offset: U32,+line: U32,out: List<&2,Token>,piece: Piece,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match piece:
    case Piece{text,rest,+bytes,lines,""}: next(rest,(offset + bytes : U32),(line + lines : U32),Token{kind,text,line,offset,(offset + bytes : U32)} <> out)
    case Piece{_,_,_,_,error}: Lexed{List.reverse(&2,Token,out),error}

def skipped(+offset: U32,+line: U32,out: List<&2,Token>,piece: Piece,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match piece:
    case Piece{_,rest,bytes,lines,""}: next(rest,(offset + bytes : U32),(line + lines : U32),out)
    case Piece{_,_,_,_,error}: Lexed{List.reverse(&2,Token,out),error}

def printable(c: Char) -> Bool:
  match c:
    case Chr{+n}: U32.is_ge(n,32) && U32.is_le(n,126)

def punctuation(ok: Bool,+c: Char,rest: String,+offset: U32,+line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match ok:
    case True{}: next(rest,(offset + width(c) : U32),line,Token{"symbol",SCon{c,""},line,offset,(offset + width(c) : U32)} <> out)
    case False{}: Lexed{List.reverse(&2,Token,out),"non-ASCII identifier/operator or control outside current envelope"}

def ordinary(is_word: Bool,+c: Char,+rest: String,+offset: U32,+line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match is_word:
    case True{}: emitted("word",offset,line,out,word(rest,SCon{c,""},width(c)),next)
    case False{}: punctuation(printable(c),c,rest,offset,line,out,next)

def hash(python: Bool,rest: String,+offset: U32,+line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match python:
    case True{}: skipped(offset,line,out,line_comment(rest,1),next)
    case False{}: ordinary(False{},'#',rest,offset,line,out,next)

def double_slash(python: Bool,rest: String,+offset: U32,+line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match python:
    case True{}: next(rest,(offset + 2 : U32),line,Token{"symbol","//",line,offset,(offset + 2 : U32)} <> out)
    case False{}: skipped(offset,line,out,line_comment(rest,2),next)

def block_start(python: Bool,nested: Bool,rest: String,offset: U32,line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match python:
    case True{}: ordinary(False{},'/',SCon{'*',rest},offset,line,out,next)
    case False{}: skipped(offset,line,out,block_comment(rest,nested,0n,2,0),next)

def slash(ecma: Bool,rest: String,offset: U32,line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match ecma:
    case True{}: Lexed{List.reverse(&2,Token,out),"ECMAScript division/regex disambiguation outside current envelope"}
    case False{}: ordinary(False{},'/',rest,offset,line,out,next)

def multiline(python: Bool,quote: Char,opening: String,rest: String,offset: U32,line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match python:
    case True{}: emitted("literal",offset,line,out,triple(rest,quote,False{},opening,3,0),next)
    case False{}: Lexed{List.reverse(&2,Token,out),"triple quoted literal outside language envelope"}

# Python raw prefixes preserve backslashes in the value, but an odd backslash
# still shields a closing quote lexically. The existing quoted scanner has
# exactly that boundary behavior; the token retains its prefix and byte span.
def raw_quoted(python: Bool,quote: Char,opening: String,rest: String,offset: U32,line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match python:
    case True{}: emitted("literal",offset,line,out,quoted(rest,quote,False{},String.reverse(opening),2),next)
    case False{}: Lexed{List.reverse(&2,Token,out),"raw literal outside current envelope"}

def continued(python: Bool,rest: String,offset: U32,line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match python:
    case True{}: next(rest,(offset + 2 : U32),(line + 1 : U32),out)
    case False{}: Lexed{List.reverse(&2,Token,out),"line continuation outside language envelope"}

def tab(python: Bool,rest: String,offset: U32,line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match python:
    case True{}: Lexed{List.reverse(&2,Token,out),"Python tabs outside spaces-only indentation profile"}
    case False{}: next(rest,(offset + 1 : U32),line,out)

def angle(ecma: Bool,rest: String,offset: U32,line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match ecma:
    case True{}: Lexed{List.reverse(&2,Token,out),"ECMAScript angle/JSX/generic disambiguation outside current envelope"}
    case False{}: ordinary(False{},'<',rest,offset,line,out,next)

def step(input: String,+language: String,+offset: U32,+line: U32,out: List<&2,Token>,next: String -> U32 -> U32 -> List<&2,Token> -> Lexed) -> Lexed:
  match input:
    case SNil{}: Lexed{List.reverse(&2,Token,out),""}
    case SCon{' ',t}: next(t,(offset + 1 : U32),line,out)
    case SCon{'\t',t}: tab(String.eq(language,"python"),t,offset,line,out,next)
    case SCon{'\r',SCon{'\n',t}}: next(t,(offset + 2 : U32),(line + 1 : U32),Token{"newline","\r\n",line,offset,(offset + 2 : U32)} <> out)
    case SCon{'\r',_}: Lexed{List.reverse(&2,Token,out),"standalone CR line break outside current envelope"}
    case SCon{'\n',t}: next(t,(offset + 1 : U32),(line + 1 : U32),Token{"newline","\n",line,offset,(offset + 1 : U32)} <> out)
    case SCon{'#',t}: hash(String.eq(language,"python"),t,offset,line,out,next)
    case SCon{'/',SCon{'/',t}}: double_slash(String.eq(language,"python"),t,offset,line,out,next)
    case SCon{'/',SCon{'*',t}}: block_start(String.eq(language,"python"),String.eq(language,"rust"),t,offset,line,out,next)
    case SCon{'/',t}: slash(String.eq(language,"ecmascript"),t,offset,line,out,next)
    case SCon{'<',t}: angle(String.eq(language,"ecmascript"),t,offset,line,out,next)
    case SCon{'`',_}: Lexed{List.reverse(&2,Token,out),"template literal outside current envelope"}
    case SCon{'\\',SCon{'\n',t}}: continued(String.eq(language,"python"),t,offset,line,out,next)
    case SCon{'\\',_}: Lexed{List.reverse(&2,Token,out),"escape outside supported literal or line continuation"}
    case SCon{'r',SCon{'#',_}}: Lexed{List.reverse(&2,Token,out),"raw identifier or raw literal outside current envelope"}
    case SCon{'r',SCon{'"',SCon{'"',SCon{'"',_}}}}: Lexed{List.reverse(&2,Token,out),"raw triple quote outside current envelope"}
    case SCon{'r',SCon{'\'',SCon{'\'',SCon{'\'',_}}}}: Lexed{List.reverse(&2,Token,out),"raw triple quote outside current envelope"}
    case SCon{'R',SCon{'"',SCon{'"',SCon{'"',_}}}}: Lexed{List.reverse(&2,Token,out),"raw triple quote outside current envelope"}
    case SCon{'R',SCon{'\'',SCon{'\'',SCon{'\'',_}}}}: Lexed{List.reverse(&2,Token,out),"raw triple quote outside current envelope"}
    case SCon{'r',SCon{'"',t}}: raw_quoted(String.eq(language,"python"),'"',"r\"",t,offset,line,out,next)
    case SCon{'r',SCon{'\'',t}}: raw_quoted(String.eq(language,"python"),'\'',"r'",t,offset,line,out,next)
    case SCon{'R',SCon{'"',t}}: raw_quoted(String.eq(language,"python"),'"',"R\"",t,offset,line,out,next)
    case SCon{'R',SCon{'\'',t}}: raw_quoted(String.eq(language,"python"),'\'',"R'",t,offset,line,out,next)
    case SCon{'f',SCon{'"',_}}: Lexed{List.reverse(&2,Token,out),"interpolated literal outside current envelope"}
    case SCon{'f',SCon{'\'',_}}: Lexed{List.reverse(&2,Token,out),"interpolated literal outside current envelope"}
    case SCon{'"',SCon{'"',SCon{'"',t}}}: multiline(String.eq(language,"python"),'"',"\"\"\"",t,offset,line,out,next)
    case SCon{'\'',SCon{'\'',SCon{'\'',t}}}: multiline(String.eq(language,"python"),'\'',"'''",t,offset,line,out,next)
    case SCon{'"',t}: emitted("literal",offset,line,out,quoted(t,'"',False{},"\"",1),next)
    case SCon{'\'',t}: emitted("literal",offset,line,out,quoted(t,'\'',False{},"'",1),next)
    case SCon{+c,t}: ordinary(identifier(c) || digit(c),c,t,offset,line,out,next)

def scan(fuel: Nat,+language: String) -> String -> U32 -> U32 -> List<&2,Token> -> Lexed:
  match fuel:
    case 0n: input => offset => line => out => Lexed{List.reverse(&2,Token,out),"token work limit"}
    case 1n+p: input => offset => line => out => step(input,language,offset,line,out,scan(p,language))

def run(text: String,language: String) -> Lexed:
  scan(200000n,language,text,0,1,Nil{})
