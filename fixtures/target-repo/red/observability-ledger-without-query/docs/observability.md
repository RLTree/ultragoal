# Observability

This red fixture intentionally has local run events, agent context, and a gate
marker, but no `scripts/observe` query command. The target audit must block it
instead of treating an event-ledger stub as an observability stack.
