# rules/dph-reverse-charge — §92a–§92e (přenesená daňová povinnost)

Hand-encoded ASP rules for the v1 slice (ADR-0005). The moat and the gold standard — encoded once, by hand; the LLM only maps fact patterns onto the fixed predicate vocabulary defined here.

Will contain:
- `predicates.md` — the fixed predicate vocabulary (the LLM's only target).
- `rules.lp` — ASP defaults: base rule + enumerated exceptions, each stamped `valid_from`/`valid_to`.
- `versions/` — historical rule sets (categories/thresholds changed over years → as-of-date).
- `gold/` — advisor-labeled cases (30–60) for the eval harness.

Decision shape: `reverse_charge` applies iff (both parties VAT payers) ∧ (place of supply in CZ) ∧ (supply in an enumerated §92e category), subject to dated exceptions. No arithmetic in v1.

**Phase:** authored in Phase 3 (the statute-encoding work — the real weight of v1).
