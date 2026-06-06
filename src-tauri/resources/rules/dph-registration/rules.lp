% DPH registration obligation by turnover — §6 ZDPH.
% DRAFT (v0) — pending tax-advisor legal sign-off (see .context/review-queue.md).
% The threshold figure is NOT encoded here (it changes); whether turnover exceeded it is
% an input fact. Decision atoms: must_register / not_required. Neither => caller ABSTAINS.

decision(must_register) :- turnover_over_threshold(yes).
decision(not_required)  :- turnover_over_threshold(no).

% --- citation (grounded by construction) ---
cite("§6") :- decision(_).

% --- justification ---
because(turnover_over_threshold)      :- turnover_over_threshold(yes).
because_not(turnover_under_threshold) :- turnover_over_threshold(no).

#show decision/1.
#show cite/1.
#show because/1.
#show because_not/1.
