import Base

type Frame is Data:
  End{}
  Payload{size: U32,chunks: List<&2,List<&2,U32>>}

def length(bytes: List<&2,U32>) -> U32:
  match bytes:
    case Nil{}: 0
    case h <> t: (1 + length(t) : U32)

def smaller_cmp(order: Cmp,a: U32,b: U32) -> U32:
  match order:
    case GT{}: b
    case _: a

def smaller(+a: U32,+b: U32) -> U32:
  smaller_cmp(U32.cmp(a,b),a,b)

def fail(file: File,message: String) -> IO(File & Frame):
  IO.bind(Unit,File & Frame,File.close(file),u => IO.die(File & Frame,3,message))

def body_step(next: U32 -> File -> List<&2,List<&2,U32>> -> IO(File & Frame),+left: U32,chunks: List<&2,List<&2,U32>>,result: File & Result<&1,&1,U32 & String,List<&2,U32>>) -> IO(File & Frame):
  match result:
    case (file,Fail{(code,message)}): fail(file,message)
    case (file,Done{Nil{}}): fail(file,"truncated stream frame")
    case (file,Done{+bytes}): next((left - length(bytes) : U32),file,bytes <> chunks)

def body_done(done: Bool,next: U32 -> File -> List<&2,List<&2,U32>> -> IO(File & Frame),+left: U32,+size: U32,file: File,chunks: List<&2,List<&2,U32>>) -> IO(File & Frame):
  match done:
    case True{}: IO.pure(File & Frame,(file,Payload{size,List.reverse(&2,List<&2,U32>,chunks)}))
    case False{}: IO.bind(File & Result<&1,&1,U32 & String,List<&2,U32>>,File & Frame,File.read_bytes(file,smaller(left,65536)),body_step(next,left,chunks))

# Every successful read returns at least one byte, so a frame of `size` bytes needs at
# most size + 1 steps; the fuel is rounds of 4,096 reads (size / 4096 + 1 of them), so a
# frame never runs out however small the reads a loaded machine delivers.
def read_round(fuel: Nat,+size: U32,next_round: U32 -> File -> List<&2,List<&2,U32>> -> IO(File & Frame)) -> U32 -> File -> List<&2,List<&2,U32>> -> IO(File & Frame):
  match fuel:
    case 0n: next_round
    case 1n+p: +left => file => chunks => body_done(U32.is_eq(left,0),read_round(p,size,next_round),left,size,file,chunks)

def read_body(rounds: Nat,+size: U32) -> U32 -> File -> List<&2,List<&2,U32>> -> IO(File & Frame):
  match rounds:
    case 0n: left => file => chunks => fail(file,"stream read work limit")
    case 1n+p: read_round(4096n,size,read_body(p,size))

def terminal(result: File & Result<&1,&1,U32 & String,List<&2,U32>>) -> IO(File & Frame):
  match result:
    case (file,Done{Nil{}}): IO.pure(File & Frame,(file,End{}))
    case (file,Done{_}): fail(file,"data after stream terminator")
    case (file,Fail{(code,message)}): fail(file,message)

def begin(size: U32,file: File) -> IO(File & Frame):
  match size:
    case 0: IO.bind(File & Result<&1,&1,U32 & String,List<&2,U32>>,File & Frame,File.read_bytes(file,1),terminal)
    case +n: read_body(U32.to_nat((n / 4096 + 1 : U32)),n,n,file,Nil{})

def digit(ok: Bool,next: U32 -> Bool -> File -> IO(File & Frame),value: U32,byte: U32,file: File) -> IO(File & Frame):
  match ok:
    case False{}: fail(file,"invalid stream frame length")
    case True{}: next((value * 10 + byte - 48 : U32),True{},file)

def ended(ok: Bool,value: U32,file: File) -> IO(File & Frame):
  match ok:
    case True{}: begin(value,file)
    case False{}: fail(file,"empty or oversized stream frame length")

def header_byte(next: U32 -> Bool -> File -> IO(File & Frame),+value: U32,seen: Bool,file: File,+byte: U32) -> IO(File & Frame):
  match byte:
    case 10: ended(seen && U32.is_le(value,16777216),value,file)
    case b: digit(U32.is_ge(b,48) && U32.is_le(b,57) && U32.is_le(value,1677721),next,value,b,file)

def header_step(next: U32 -> Bool -> File -> IO(File & Frame),value: U32,seen: Bool,result: File & Result<&1,&1,U32 & String,List<&2,U32>>) -> IO(File & Frame):
  match result:
    case (file,Done{byte <> Nil{}}): header_byte(next,value,seen,file,byte)
    case (file,Done{_}): fail(file,"missing stream terminator or truncated header")
    case (file,Fail{(code,message)}): fail(file,message)

def header(fuel: Nat) -> U32 -> Bool -> File -> IO(File & Frame):
  match fuel:
    case 0n: value => seen => file => fail(file,"stream header work limit")
    case 1n+p: value => seen => file => IO.bind(File & Result<&1,&1,U32 & String,List<&2,U32>>,File & Frame,File.read_bytes(file,1),header_step(header(p),value,seen))

def read(file: File) -> IO(File & Frame):
  header(9n,0,False{},file)
