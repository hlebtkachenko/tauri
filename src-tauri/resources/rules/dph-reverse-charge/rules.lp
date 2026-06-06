% DPH domestic reverse-charge (přenesená daňová povinnost)
% v1 ruleset: §92a general mechanism + §92e construction/assembly.
% valid_from 2016-01-01 (encode historical versions under versions/ in Phase 3).
% NOTE: v0 encoding — pending tax-advisor legal review (see OUTSTANDING.md).
%
% Decision atoms: applies / not_applies. Neither derivable => caller ABSTAINS.

% --- enumerated reverse-charge categories (v1 subset) ---
enumerated_category(construction_assembly).   % §92e

% --- §92a general conditions ---
cond_payers   :- vat_status(supplier, payer), vat_status(customer, payer).
cond_place    :- place_of_supply(czech_republic).
cond_category :- supply_category(C), enumerated_category(C).

% --- positive conclusion (all conditions known true) ---
applies :- cond_payers, cond_place, cond_category.

% --- negative conclusion (any condition KNOWN false) ---
not_applies :- vat_status(supplier, nonpayer).
not_applies :- vat_status(customer, nonpayer).
not_applies :- place_of_supply(other).
not_applies :- supply_category(other).

% --- decision (emitted by the rule; the solver reads decision(...), topic-agnostic) ---
decision(reverse_charge_applies)     :- applies.
decision(reverse_charge_not_applies) :- not_applies.

% --- citations (grounded by construction — every answer cites its authority) ---
cite("§92a") :- applies.
cite("§92e") :- applies, supply_category(construction_assembly).
cite("§92a") :- not_applies.

% --- justification (surface the reasoning) ---
because(payers_both_vat)        :- cond_payers, applies.
because(place_in_cz)            :- cond_place, applies.
because(enumerated_category)    :- cond_category, applies.
because_not(supplier_nonpayer)  :- vat_status(supplier, nonpayer).
because_not(customer_nonpayer)  :- vat_status(customer, nonpayer).
because_not(place_outside_cz)   :- place_of_supply(other).
because_not(category_excluded)  :- supply_category(other).

#show decision/1.
#show cite/1.
#show because/1.
#show because_not/1.
