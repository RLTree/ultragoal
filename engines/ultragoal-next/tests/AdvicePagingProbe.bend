import Base
import ./../RankSession.bend as R

def output(result: R.Outcome) -> String:
  match result:
    case R.Outcome{_,reply}: reply

def main() -> String:
  output(R.advice_completed("order","cache reload after rename",[["2\twatch rename events for reload","3\tcheck disk"],["0\tpip freeze","1\tsend reload signal"]]))
