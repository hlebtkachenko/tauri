# Fixed predicate vocabulary — DPH reverse-charge

The LLM extractor's **only** target. It maps a natural-language case onto these atoms; anything outside this vocabulary → reject → abstain (compile-or-refuse on the neural side). Absent fact = *unknown* (drives abstention, never a guess).

## Input facts (emitted by the extractor)

| Predicate | Args | Meaning |
|---|---|---|
| `vat_status(P, S)` | `P ∈ {supplier, customer}`, `S ∈ {payer, nonpayer}` | VAT-payer status of a party. Omit if unknown. |
| `place_of_supply(L)` | `L ∈ {czech_republic, other}` | Place of taxable supply. Omit if unknown. |
| `supply_category(C)` | `C ∈ {construction_assembly, other}` | Supply type. `construction_assembly` = §92e enumerated. `other` = known-but-not-enumerated. Omit if unknown. (More §92 categories added in Phase 3.) |

## Output atoms (produced by the engine, read by the solver)

| Atom | Meaning |
|---|---|
| `applies` | reverse-charge applies |
| `not_applies` | reverse-charge does not apply (standard regime) |
| `cite(S)` | governing section, e.g. `"§92a"`, `"§92e"` |
| `because(R)` | satisfied condition supporting `applies` |
| `because_not(R)` | reason supporting `not_applies` |

Neither `applies` nor `not_applies` derivable → the solver returns **abstain** (insufficient facts). Both derivable (contradictory input) → **conflict** → refuse.
