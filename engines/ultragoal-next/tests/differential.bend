import Base
import ../Engine.bend as E
import ../Facts.bend as F
import ../Core.bend as C

# Deterministic mutation stream tests actual key and parse-reuse functions.
# This supplements, not replaces, the filesystem/CLI mutation journeys.
def text(choice: U32) -> String:
  match choice:
    case 0: "import pathlib\n"
    case 1: "def f(): return 1\n"
    case 2: "# unrelated comment\n"
    case _: ""

def hit(same: Bool,value: String) -> Maybe<&2,String>:
  match same:
    case True{}: Some{value}
    case False{}: None{}

def step(+seed: U32,+previous: String,+cached: String) -> Bool:
  +source = text(((seed >> 8n) % 4 : U32))
  +r = {["O","id","1","requirement","origin","*","membership","member","","exact","mandatory"] : List<&2,String>}
  +files = {[["F","a.py",source,source]] : List<&2,List<&2,String>>}
  +key = E.key(r,files)
  +same = String.eq(previous,source)
  +parsed = E.parse_or_reuse(hit(same,cached),"python-lexical/1",source)
  String.eq(parsed,F.parse("python-lexical/1",source)) && String.eq(key,E.key(r,[["F","a.py","different","different"]])) && Bool.not(String.eq(key,E.key(r,[["F","a.py",source,source],["F","b.py","new","new"]]))) && String.eq(C.status(C.admit(C.Cancelled{},C.Bound{True{},True{},True{},True{},True{},True{},True{},True{}},C.Exact{},C.Exact{},C.Verified{})),"unknown")

def add(ok: Bool,n: U32) -> U32:
  match ok:
    case True{}: n
    case False{}: (n + 1 : U32)

def run(fuel: Nat,+seed: U32,+previous: String,+cached: String) -> U32:
  match fuel:
    case 0n: 0
    case 1n+p:
      +next = (seed * 1664525 + 1013904223 : U32)
      +source = text(((next >> 8n) % 4 : U32))
      add(step(next,previous,cached),run(p,next,source,F.parse("python-lexical/1",source)))

def main() -> IO(Unit):
  IO.print(U32.show(run(100000n,1937,"",F.parse("python-lexical/1",""))))
