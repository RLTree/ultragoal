import Base
import ./SourceTree.bend as T
import ./SourceIndent.bend as I
import ./SourceDecl.bend as D
import ./FactRecord.bend as R

def language(grammar: String) -> String:
  match grammar:
    case "python-structure/1": "python"
    case "rust-structure/1": "rust"
    case "ecmascript-structure/1": "ecmascript"
    case _: ""

def supported(grammar: String) -> Bool:
  Bool.not(String.is_empty(language(grammar)))

def facts(compact: Bool,+language: String,text: String) -> String:
  R.encoded(compact,D.extract(language,I.run(language,T.parse(text,language))))

def parsed(compact: Bool,language: String,text: String) -> String:
  match language:
    case "": ""
    case name: facts(compact,name,text)

def parse(compact: Bool,grammar: String,text: String) -> String:
  parsed(compact,language(grammar),text)
