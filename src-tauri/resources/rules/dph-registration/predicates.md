# Predicates — dph-registration

Fixed fact vocabulary for Czech VAT (DPH) registration obligation by turnover (§6 ZDPH).
DRAFT (v0) — pending tax-advisor legal sign-off.

| Predicate | Allowed values | Meaning | Statute |
|---|---|---|---|
| `turnover_over_threshold(V)` | `yes` \| `no` | did 12-month turnover exceed the registration threshold | §6 ZDPH |

Decision atoms (emitted by `rules.lp`, read by the solver):

| Decision | Fires when | Citation |
|---|---|---|
| `must_register` | `turnover_over_threshold(yes)` | §6 |
| `not_required` | `turnover_over_threshold(no)` | §6 |

The threshold figure is intentionally NOT encoded (it changes over time); whether turnover
exceeded it is supplied as a fact. Missing fact → abstain.
