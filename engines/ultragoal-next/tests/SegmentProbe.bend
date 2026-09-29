import Base
import ./../SourceSegment.bend as S

def text(value: Maybe<&2,String>) -> String:
  match value:
    case Some{source}: source
    case None{}: "none"

def started() -> S.State:
  S.state(S.begin(S.Empty{},"op","src/a.py","v1","4de629f6f1b33b78093e59885c93e5be58b9ccfb8889d7d5e7dcc73c4d3e34e4",8n))

def first() -> S.State:
  S.state(S.page(started(),"op","src/a.py","v1",0n,0n,"π\r\n"))

def valid() -> String:
  text(S.source(S.end(S.state(S.page(first(),"op","src/a.py","v1",1n,4n,"tail")),"op","src/a.py","v1","4de629f6f1b33b78093e59885c93e5be58b9ccfb8889d7d5e7dcc73c4d3e34e4",2n)))

def duplicate() -> String:
  S.status(S.page(first(),"op","src/a.py","v1",0n,4n,"tail"))

def stale() -> String:
  S.status(S.page(first(),"op","src/a.py","v2",1n,4n,"tail"))

def early() -> String:
  S.status(S.end(first(),"op","src/a.py","v1","4de629f6f1b33b78093e59885c93e5be58b9ccfb8889d7d5e7dcc73c4d3e34e4",1n))

def cancelled() -> String:
  S.status(S.abort(first()))

def main() -> String:
  valid() ++ "|" ++ duplicate() ++ "|" ++ stale() ++ "|" ++ early() ++ "|" ++ cancelled()
