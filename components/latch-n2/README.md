# `latch-n2`

HTTP latch that aggregates 2 latches, any latch can deny a request.

The latches are asked in order, `latch0`, `latch1`, and the first denial is the decision. Latches after a denial are not asked, but every latch observes the final decision with `observe-decision`, even when an earlier latch fails to observe it. The first failure is returned.

## Interfaces

Imports:

- `latch0`: `componentized:http/latch@0.1.0-dev`, a latch to aggregate
- `latch1`: `componentized:http/latch@0.1.0-dev`, a latch to aggregate

Exports:

- `componentized:http/latch@0.1.0-dev`: the latch
