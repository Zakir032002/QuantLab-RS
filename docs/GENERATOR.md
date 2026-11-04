# Part 2 generator distributions

generate_shape(shape, seed, n_events, defect) reuses D01-D11 injectors. S1 uses
ceil(n_events/5) orders; other shapes use ceil(n_events/4). Thus event budgets are
lower bounds rather than exact counts; the gate records actual events, including
bootstrap, claims and two removable context events. Minimum is 16 orders.
All prices/fees and scheduling derive deterministically from the seed. The header
already supports multiple instruments; S5 adds no new financial model.

| Shape | Scheduling |
|---|---|
| S1 | Sequential Part 1 lifecycles and claims after each fill. |
| S2 | 2+(seed mod 15) concurrent orders; reverse first partials and seeded cyclic completion order. |
| S3 | One clean parent, one-lot partial per ordinary batch, first and final partials spanning the trace; claims every 5 fills. Defect is on a middle ordinary order. |
| S4 | Claims every 5+(seed mod 46) fills. |
| S5 | 2+(seed mod 7) instruments, independently bootstrapped inventory and buy/buy/sell cycles. |
| S6 | A one-currency-unit early cash error carries forward; select the later planted defect as target. |
| S7 | Claims after every fill; every position observation gains an additional one-lot offset, producing local errors rather than a constant carried offset. |
| S8 | 10+(seed mod 491) buys (bounded by half the order count), followed by a sell of 20 times that many shares; D04 targets this sell. |
| S9 | Cash phase 1 mod 3; first instrument phase 2 mod 3; others have independent periods. No required common frontier. Remove the final cash observation, retaining uncovered-tail diagnostics alongside the proven violation. |

Every violating generated fixture inserts a submitted/accepted unfilled NOISE
order immediately before the target. Its instrument matches an I2 target. It is
removable and must disappear in every gate cell. Shape S7 has financial violations
even without a planted operator. S9 clean traces can be INCONCLUSIVE at the tail;
D01-D11 traces remain VIOLATION with those findings retained.

The separate long-lived R1 test also targets an order first partially filled
near the beginning and completed at the end. Standalone bundle tests cover the
checkpoint status, filled quantity and duplicate-trade context at its cut.
