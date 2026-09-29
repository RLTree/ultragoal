import Base
import ./Facts.bend as F
import ./GroupedParse.bend as G
import ./ParseCache.bend as PC

type ParseBatch is Data:
  ParseBatch{rows: Map<&2,Maybe<&2,List<&2,String>>>, valid: Bool}

# Legacy helper laws remain explicit conditional checks. Production reuse goes
# through PC.run and its constructor-proven job-to-source associations.
def parse_or_reuse(hit: Maybe<&2,String>,g: String,source: String) -> String:
  match hit:
    case Some{blob}: blob
    case None{}: F.parse(g,source)

def parse_spans_or_reuse(hit: Maybe<&2,String>,g: String,source: String) -> String:
  match hit:
    case Some{blob}: blob
    case None{}: F.parse_spans(g,source)

def completed(result: PC.Work) -> ParseBatch:
  match result:
    case PC.Work{rows,_,valid}: ParseBatch{G.materialize(rows,MTip{}),valid}

def run(rows: List<&2,List<&2,String>>,compact: Bool) -> ParseBatch:
  completed(PC.run(rows,compact,PC.State{MTip{},0,0}))
