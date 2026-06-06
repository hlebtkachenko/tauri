// budget.rs — per-process cost guard. Two ceilings, both fail-closed (the caller
// abstains on Err): a call-count cap (runaway-loop guard) and a USD spend cap (from
// 2026-06-15 programmatic subscription use is metered at API rates, so a chatty agent can
// burn real money). `charge_call()` is the pre-call gate; `record_cost()` accumulates the
// actual total_cost_usd reported by each `claude` call.
//
// Scope: per process / per struct instance. A long-lived host (the Tauri app) holds one
// `Budget` and gets a real running budget; a one-shot path makes a fresh one.
//
// TODO(P2): persist call count + spend to SQLite so budgets survive across runs.

use super::types::EngineError;

const DEFAULT_MAX_CALLS: u64 = 50;
const DEFAULT_MAX_SPEND_USD: f64 = 1.0;

#[derive(Debug)]
pub struct Budget {
    calls: u64,
    spent_usd: f64,
    max_calls: u64,
    max_spend_usd: f64,
}

impl Budget {
    /// Build a budget from env (MAX_LLM_CALLS / MAX_SPEND_USD), falling back to defaults.
    pub fn from_env() -> Self {
        let max_calls = std::env::var("MAX_LLM_CALLS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_MAX_CALLS);
        let max_spend_usd = std::env::var("MAX_SPEND_USD")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(DEFAULT_MAX_SPEND_USD);
        Self::new(max_calls, max_spend_usd)
    }

    pub fn new(max_calls: u64, max_spend_usd: f64) -> Self {
        Budget {
            calls: 0,
            spent_usd: 0.0,
            max_calls,
            max_spend_usd,
        }
    }

    /// Pre-call gate. Increments the call count and refuses (Err) when either ceiling is
    /// reached. Mirrors the TS `chargeCall()` order: count first, then spend.
    pub fn charge_call(&mut self) -> Result<(), EngineError> {
        self.calls += 1;
        if self.calls > self.max_calls {
            return Err(EngineError::Budget(format!(
                "LLM call budget exceeded ({}). Raise MAX_LLM_CALLS or check for a loop.",
                self.max_calls
            )));
        }
        if self.spent_usd >= self.max_spend_usd {
            return Err(EngineError::Budget(format!(
                "LLM spend budget exceeded (${:.2}; spent ${:.4}). Raise MAX_SPEND_USD.",
                self.max_spend_usd, self.spent_usd
            )));
        }
        Ok(())
    }

    /// Accumulate the actual cost reported by a completed `claude` call.
    pub fn record_cost(&mut self, usd: f64) {
        if usd.is_finite() && usd > 0.0 {
            self.spent_usd += usd;
        }
    }

    pub fn calls_used(&self) -> u64 {
        self.calls
    }

    pub fn spent_usd(&self) -> f64 {
        self.spent_usd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_calls_under_the_cap() {
        let mut b = Budget::new(3, 1.0);
        assert!(b.charge_call().is_ok());
        assert!(b.charge_call().is_ok());
        assert!(b.charge_call().is_ok());
        assert_eq!(b.calls_used(), 3);
    }

    #[test]
    fn refuses_when_call_cap_exceeded() {
        let mut b = Budget::new(2, 1.0);
        assert!(b.charge_call().is_ok());
        assert!(b.charge_call().is_ok());
        assert!(matches!(b.charge_call(), Err(EngineError::Budget(_))));
    }

    #[test]
    fn refuses_when_spend_cap_reached() {
        let mut b = Budget::new(50, 0.50);
        assert!(b.charge_call().is_ok());
        b.record_cost(0.60);
        assert!(matches!(b.charge_call(), Err(EngineError::Budget(_))));
    }

    #[test]
    fn record_cost_ignores_nonpositive_and_nan() {
        let mut b = Budget::new(50, 1.0);
        b.record_cost(-1.0);
        b.record_cost(f64::NAN);
        b.record_cost(0.0);
        assert_eq!(b.spent_usd(), 0.0);
        b.record_cost(0.25);
        assert_eq!(b.spent_usd(), 0.25);
    }
}
