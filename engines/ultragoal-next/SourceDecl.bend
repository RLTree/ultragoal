import Base
import ./SourceTree.bend as T
import ./FactRecord.bend as R
import ./SourceLex.bend as Lex
import ./SourcePyImport.bend as PyImport

type Body is Data:
  Body{kind: String, node: Maybe<&2,T.Tree>}

def join(a: List<&2,R.Fact>,b: List<&2,R.Fact>) -> List<&2,R.Fact>:
  List.append(&2,R.Fact,a,b)

def join3(a: List<&2,R.Fact>,b: List<&2,R.Fact>,c: List<&2,R.Fact>) -> List<&2,R.Fact>:
  join(a,join(b,c))

def fact(kind: String,value: String,node: T.Tree) -> List<&2,R.Fact>:
  match node:
    case T.Tree{_,_,l,s,e,_}: [R.Fact{kind,value,l,s,e}]

def limitation(message: String) -> List<&2,R.Fact>:
  [R.Fact{"structure-limitation",message,0,0,0}]

def valid_name(text: String) -> Bool:
  match text:
    case SCon{first,_}: Lex.identifier(first)
    case _: False{}

def body_fact(kind: String,node: Maybe<&2,T.Tree>) -> List<&2,R.Fact>:
  match node:
    case Some{tree}: fact(kind,"",tree)
    case None{}: Nil{}

def function_body(body: Body,name: String,node: T.Tree,params: T.Tree) -> List<&2,R.Fact>:
  match body:
    case Body{"unsupported",_}: limitation("function suffix outside supported header grammar")
    case Body{"signature",_}: join(fact("structure-function-signature-name",name,node),fact("structure-parameters","",params))
    case Body{kind,body}: join3(fact("structure-function-name",name,node),fact("structure-parameters","",params),body_fact(kind,body))

def inline_body(same_line: Bool,tree: T.Tree) -> Body:
  match same_line:
    case True{}: Body{"structure-inline-body-start",Some{tree}}
    case False{}: Body{"unsupported",None{}}

def py_body(trees: List<&2,T.Tree>,+line: U32) -> Body:
  match trees:
    case T.Tree{"newline",_,_,_,_,_} <> t: py_body(t,line)
    case T.Tree{"suite",text,l,s,e,children} <> _: Body{"structure-function-body",Some{T.Tree{"suite",text,l,s,e,children}}}
    case T.Tree{k,text,+l,s,e,children} <> _: inline_body(U32.is_eq(line,l),T.Tree{k,text,l,s,e,children})
    case _: Body{"unsupported",None{}}

def rust_body(fuel: Nat,trees: List<&2,T.Tree>) -> Body:
  match fuel trees:
    case _ T.Tree{"group","{",l,s,e,children} <> _: Body{"structure-function-body",Some{T.Tree{"group","{",l,s,e,children}}}
    case _ T.Tree{"symbol",";",_,_,_,_} <> _: Body{"signature",None{}}
    case _ T.Tree{"symbol","<",_,_,_,_} <> _: Body{"unsupported",None{}}
    case _ T.Tree{"symbol","!",_,_,_,_} <> _: Body{"unsupported",None{}}
    case 1n+p _ <> t: rust_body(p,t)
    case _ _: Body{"unsupported",None{}}

def js_body(trees: List<&2,T.Tree>) -> Body:
  match trees:
    case T.Tree{"newline",_,_,_,_,_} <> t: js_body(t)
    case T.Tree{"group","{",l,s,e,children} <> _: Body{"structure-function-body",Some{T.Tree{"group","{",l,s,e,children}}}
    case _: Body{"unsupported",None{}}

def suffix(language: String,trees: List<&2,T.Tree>) -> Body:
  match language:
    case "python":
      match trees:
        case T.Tree{"symbol",":",l,_,_,_} <> t: py_body(t,l)
        case _: Body{"unsupported",None{}}
    case "rust": rust_body(256n,trees)
    case "ecmascript": js_body(trees)
    case _: Body{"unsupported",None{}}

def named_function(valid: Bool,language: String,name: String,node: T.Tree,params: T.Tree,rest: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match valid:
    case True{}: function_body(suffix(language,rest),name,node,params)
    case False{}: limitation("function name outside ASCII identifier profile")

def function(language: String,trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"word",+name,l,s,e,_} <> T.Tree{"group","(",pl,ps,pe,params} <> rest: named_function(valid_name(name),language,name,T.Tree{"word",name,l,s,e,Nil{}},T.Tree{"group","(",pl,ps,pe,params},rest)
    case _: limitation("function header outside supported named parameter-list form")

def js_function(trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"symbol","*",_,_,_,_} <> rest: function("ecmascript",rest)
    case rest: function("ecmascript",rest)

def python_class(trees: List<&2,T.Tree>) -> Body:
  match trees:
    case T.Tree{"group","(",_,_,_,_} <> rest: suffix("python",rest)
    case rest: suffix("python",rest)

def type_body(kind: String,body: Body,name: String,node: T.Tree) -> List<&2,R.Fact>:
  match body:
    case Body{"unsupported",_}: limitation("type declaration outside supported body form")
    case Body{_,body}: join(fact(kind,name,node),body_fact("structure-type-body",body))

def checked_type(valid: Bool,kind: String,body: Body,name: String,node: T.Tree) -> List<&2,R.Fact>:
  match valid:
    case True{}: type_body(kind,body,name,node)
    case False{}: limitation("type name outside ASCII identifier-shaped word profile")

def required_body(allow_signature: Bool,body: Body) -> Body:
  match allow_signature body:
    case False{} Body{"signature",_}: Body{"unsupported",None{}}
    case _ value: value

def class_suffix(language: String,trees: List<&2,T.Tree>) -> Body:
  match language:
    case "python": python_class(trees)
    case "ecmascript": required_body(False{},rust_body(64n,trees))
    case _: Body{"unsupported",None{}}

def class(language: String,trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"word",+name,l,s,e,_} <> rest: checked_type(valid_name(name),"structure-class-name",class_suffix(language,rest),name,T.Tree{"word",name,l,s,e,Nil{}})
    case _: limitation("class name missing")

def named_type(+kind: String,trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"word",+name,l,s,e,_} <> rest: checked_type(valid_name(name),kind,required_body(String.eq(kind,"structure-struct-name"),rust_body(64n,rest)),name,T.Tree{"word",name,l,s,e,Nil{}})
    case _: limitation("type name missing")

def literal_fact(kind: String,node: T.Tree) -> List<&2,R.Fact>:
  match node:
    case T.Tree{"literal",+text,l,s,e,_}: fact(kind,text,T.Tree{"literal",text,l,s,e,Nil{}})
    case _: limitation("non-literal module specifier")

def from_literal(fuel: Nat,trees: List<&2,T.Tree>,+kind: String) -> List<&2,R.Fact>:
  match fuel trees:
    case _ T.Tree{"word","from",_,_,_,_} <> literal <> _: literal_fact(kind,literal)
    case _ T.Tree{"symbol",";",_,_,_,_} <> _: limitation("module from-clause missing")
    case _ T.Tree{"symbol","=",_,_,_,_} <> _: limitation("module from-clause missing")
    case _ T.Tree{"word","function",_,_,_,_} <> _: limitation("module from-clause missing")
    case _ T.Tree{"word","const",_,_,_,_} <> _: limitation("module from-clause missing")
    case 1n+p _ <> t: from_literal(p,t,kind)
    case _ _: limitation("module from-clause outside bound")

def js_import(trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"literal",text,l,s,e,c} <> _: literal_fact("structure-import-specifier",T.Tree{"literal",text,l,s,e,c})
    case T.Tree{"group","(",l,s,e,c} <> _: join(fact("structure-dynamic-import","",T.Tree{"group","(",l,s,e,c}),limitation("dynamic import requires separate resolution"))
    case rest: from_literal(128n,rest,"structure-import-specifier")

def js_export(trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match trees:
    case T.Tree{"symbol","*",_,_,_,_} <> rest: from_literal(128n,rest,"structure-reexport-specifier")
    case T.Tree{"group","{",_,_,_,_} <> T.Tree{"word","from",_,_,_,_} <> literal <> _: literal_fact("structure-reexport-specifier",literal)
    case _: Nil{}

def keyword(language: String,word: String,trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match language word:
    case "python" "def": function("python",trees)
    case "python" "class": class("python",trees)
    case "rust" "fn": function("rust",trees)
    case "rust" "struct": named_type("structure-struct-name",trees)
    case "rust" "enum": named_type("structure-enum-name",trees)
    case "rust" "trait": named_type("structure-trait-name",trees)
    case "ecmascript" "function": js_function(trees)
    case "ecmascript" "class": class("ecmascript",trees)
    case "ecmascript" "interface": named_type("structure-interface-name",trees)
    case "ecmascript" "import": js_import(trees)
    case "ecmascript" "export": js_export(trees)
    case _ _: Nil{}

def head(language: String,tree: T.Tree,tail: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match tree:
    case T.Tree{"word",word,_,_,_,_}: keyword(language,word,tail)
    case _: Nil{}

def children(tree: T.Tree) -> List<&2,T.Tree>:
  match tree:
    case T.Tree{_,_,_,_,_,xs}: xs

def after_statement(trees: List<&2,T.Tree>) -> List<&2,T.Tree>:
  match trees:
    case T.Tree{"newline",_,_,_,_,_} <> t: t
    case T.Tree{"symbol",";",_,_,_,_} <> t: t
    case _ <> t: after_statement(t)
    case Nil{}: Nil{}

type Route is Data:
  Route{facts: List<&2,R.Fact>, children: List<&2,T.Tree>, rest: List<&2,T.Tree>}

def ordinary(language: String,+node: T.Tree,+tail: List<&2,T.Tree>) -> Route:
  Route{head(language,node,tail),children(node),tail}

def text(node: T.Tree) -> String:
  match node:
    case T.Tree{_,value,_,_,_,_}: value

def macro_group(kind: String,+node: T.Tree,trees: List<&2,T.Tree>,fallback: Route) -> Route:
  match trees:
    case T.Tree{"group",_,_,_,_,_} <> rest: Route{join(fact(kind,text(node),node),limitation("macro expansion requires a separate producer")),Nil{},rest}
    case _: fallback

def macro_definition(trees: List<&2,T.Tree>,fallback: Route) -> Route:
  match trees:
    case T.Tree{"word",name,l,s,e,c} <> tail: macro_group("structure-macro-definition",T.Tree{"word",name,l,s,e,c},tail,fallback)
    case _: fallback

def macro_name(definition: Bool,node: T.Tree,tail: List<&2,T.Tree>,fallback: Route) -> Route:
  match definition:
    case True{}: macro_definition(tail,fallback)
    case False{}: macro_group("structure-macro-invocation",node,tail,fallback)

def rust_word(+node: T.Tree,+tail: List<&2,T.Tree>) -> Route:
  match tail:
    case T.Tree{"symbol","!",l,s,e,c} <> +rest: macro_name(String.eq(text(node),"macro_rules"),node,rest,ordinary("rust",node,T.Tree{"symbol","!",l,s,e,c} <> rest))
    case trees: ordinary("rust",node,trees)

def attribute(trees: List<&2,T.Tree>,fallback: Route) -> Route:
  match trees:
    case T.Tree{"group","[",l,s,e,c} <> rest: Route{join(fact("structure-attribute","",T.Tree{"group","[",l,s,e,c}),limitation("attribute effects require separate interpretation")),Nil{},rest}
    case _: fallback

def rust_attribute(+node: T.Tree,+tail: List<&2,T.Tree>) -> Route:
  match tail:
    case T.Tree{"symbol","!",l,s,e,c} <> +rest: attribute(rest,ordinary("rust",node,T.Tree{"symbol","!",l,s,e,c} <> rest))
    case +trees: attribute(trees,ordinary("rust",node,trees))

def rust(kind: String,value: String,node: T.Tree,tail: List<&2,T.Tree>) -> Route:
  match kind value:
    case "word" _: rust_word(node,tail)
    case "symbol" "#": rust_attribute(node,tail)
    case _ _: ordinary("rust",node,tail)

def python(value: String,node: T.Tree,+tail: List<&2,T.Tree>) -> Route:
  match value:
    case "from": Route{PyImport.from(tail),Nil{},after_statement(tail)}
    case "import": Route{PyImport.imports(128n,tail),Nil{},after_statement(tail)}
    case _: ordinary("python",node,tail)

def python_kind(kind: String,value: String,node: T.Tree,tail: List<&2,T.Tree>) -> Route:
  match kind:
    case "word": python(value,node,tail)
    case _: ordinary("python",node,tail)

def dispatch(language: String,kind: String,value: String,node: T.Tree,tail: List<&2,T.Tree>) -> Route:
  match language:
    case "rust": rust(kind,value,node,tail)
    case "python": python_kind(kind,value,node,tail)
    case name: ordinary(name,node,tail)

def route(language: String,node: T.Tree,tail: List<&2,T.Tree>) -> Route:
  match node:
    case T.Tree{+k,+t,l,s,e,c}: dispatch(language,k,t,T.Tree{k,t,l,s,e,c},tail)

def routed(value: Route,left: List<&2,T.Tree> -> List<&2,R.Fact>,right: List<&2,T.Tree> -> List<&2,R.Fact>) -> List<&2,R.Fact>:
  match value:
    case Route{facts,children,rest}: join3(facts,left(children),right(rest))

def walk(fuel: Nat,+language: String,trees: List<&2,T.Tree>) -> List<&2,R.Fact>:
  match fuel trees:
    case _ Nil{}: Nil{}
    case 0n _: limitation("structure extraction work/depth bound")
    case 1n+ +p h <> t: routed(route(language,h,t),xs => walk(p,language,xs),xs => walk(p,language,xs))

def extract(language: String,input: T.Parsed) -> List<&2,R.Fact>:
  match input:
    case T.Parsed{trees,""}: R.Fact{"structure-profile","bounded headers and module specifier syntax; not native language validity",0,0,0} <> R.Fact{"structure-dependency-closure","unknown; extracted imports do not establish complete resolution",0,0,0} <> walk(200000n,language,trees)
    case T.Parsed{_,error}: limitation(error)
