import Base
import ../SourceTree.bend as Tree
import ../SourceIndent.bend as Indent
import ../SourceDecl.bend as Decl
import ../FactRecord.bend as R
import ../SourceLex.bend as Lex
import ../Wire.bend as W
import ../Utf8.bend as U
import ../FrameIO.bend as FrameIO

def rows(fuel: Nat,trees: List<&2,Tree.Tree>,+depth: U32) -> String:
  match fuel trees:
    case _ Nil{}: ""
    case 0n _: "ERROR\toutput depth\n"
    case 1n+ +p Tree.Tree{k,t,l,s,e,children} <> tail: W.row([k,t,U32.show(l),U32.show(s),U32.show(e),U32.show(depth)]) ++ rows(p,children,(depth + 1 : U32)) ++ rows(1n+p,tail,depth)

def report(value: Tree.Parsed) -> IO(Unit):
  match value:
    case Tree.Parsed{trees,""}: IO.write("STATE\tbalanced-token-tree\n" ++ rows(130n,trees,0))
    case Tree.Parsed{_,error}: IO.write(W.row(["STATE","unsupported-or-malformed",error]))

def mode_report(mode: String,language: String,input: Tree.Parsed) -> IO(Unit):
  match mode:
    case "tree": report(input)
    case "facts": IO.write(R.encoded(False{},Decl.extract(language,input)))
    case "spans": IO.write(R.encoded(True{},Decl.extract(language,input)))
    case _: IO.die(Unit,3,"invalid mode")

def decoded(value: Maybe<&2,String>,+language: String,mode: String) -> IO(Unit):
  match value:
    case Some{text}: mode_report(mode,language,Indent.run(language,Tree.parse(text,language)))
    case None{}: IO.die(Unit,3,"invalid UTF-8")

def join(chunks: List<&2,List<&2,U32>>) -> List<&2,U32>:
  match chunks:
    case Nil{}: Nil{}
    case h <> t: List.append(&2,U32,h,join(t))

def terminal(language: String,mode: String,chunks: List<&2,List<&2,U32>>,result: File & FrameIO.Frame) -> IO(Unit):
  match result:
    case (file,FrameIO.End{}): IO.bind(Unit,Unit,File.close(file),u => decoded(U.decode(join(chunks)),language,mode))
    case (file,_): IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,"one source frame required"))

def read(language: String,mode: String,result: File & FrameIO.Frame) -> IO(Unit):
  match result:
    case (file,FrameIO.Payload{_,chunks}): IO.bind(File & FrameIO.Frame,Unit,FrameIO.read(file),terminal(language,mode,chunks))
    case (file,_): IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,"missing source frame"))

def start(args: List<&1,String>,file: File) -> IO(Unit):
  match args:
    case language <> mode <> Nil{}: IO.bind(File & FrameIO.Frame,Unit,FrameIO.read(file),read(language,mode))
    case language <> Nil{}: IO.bind(File & FrameIO.Frame,Unit,FrameIO.read(file),read(language,"tree"))
    case _: IO.bind(Unit,Unit,File.close(file),u => IO.die(Unit,3,"language required"))

def main() -> IO(Unit):
  do IO<Unit>:
    args : List<&1,String> <- IO.args()
    file : File <- IO.try(File,File.open("/dev/stdin","r"))
    start(args,file)
