import Base
import ./../SegmentSession.bend as S

def digest() -> String:
  "4de629f6f1b33b78093e59885c93e5be58b9ccfb8889d7d5e7dcc73c4d3e34e4"

def begun() -> S.State:
  S.state(S.process("SEG_BEGIN\top\tsrc/a.py\tv1\t" ++ digest() ++ "\t8",S.empty()))

def first() -> S.State:
  S.state(S.process("SEG_PAGE\top\tsrc/a.py\tv1\t0\t0\tπ%0D%0A",begun()))

def second() -> S.State:
  S.state(S.process("SEG_PAGE\top\tsrc/a.py\tv1\t1\t4\ttail",first()))

def complete() -> S.State:
  S.state(S.process("SEG_END\top\tsrc/a.py\tv1\t" ++ digest() ++ "\t2",second()))

def main() -> String:
  S.reply(S.process("SEG_READ\top\tsrc/a.py\tv1\t" ++ digest() ++ "\t0\t7",complete())) ++ S.reply(S.process("SEG_PAGE\top\tsrc/a.py\tv1\t0\t0\tduplicate",first()))
