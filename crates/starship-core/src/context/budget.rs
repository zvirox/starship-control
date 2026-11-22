/// Token budget manager ensuring prompt injection does not exceed context limits (GAP-CLOSURE-PLAN.md §12.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenBudget {
    pub max_tokens: usize,
    pub used_tokens: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_tokens: 2048,
            used_tokens: 0,
        }
    }
}

impl TokenBudget {
    #[must_use]
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            used_tokens: 0,
        }
    }

    /// Estimates token count using standard heuristic (~4 characters per token).
    #[must_use]
    pub fn estimate_tokens(text: &str) -> usize {
        let chars = text.chars().count();
        chars.div_ceil(4)
    }

    /// Returns remaining tokens available in the budget.
    #[must_use]
    pub fn remaining_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.used_tokens)
    }

    /// Returns true if the given text fits within remaining token budget.
    #[must_use]
    pub fn can_fit(&self, text: &str) -> bool {
        Self::estimate_tokens(text) <= self.remaining_tokens()
    }

    /// Consumes tokens if text fits, returning true; otherwise returns false without consuming.
    pub fn try_consume(&mut self, text: &str) -> bool {
        let cost = Self::estimate_tokens(text);
        if cost <= self.remaining_tokens() {
            self.used_tokens += cost;
            true
        } else {
            false
        }
    }

    /// Resets used tokens back to 0.
    pub fn reset(&mut self) {
        self.used_tokens = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_budget_tracking_and_consumption() {
        let mut budget = TokenBudget::new(100);
        assert_eq!(budget.remaining_tokens(), 100);

        let snippet = "This is a test string containing approximately eight tokens.";
        let cost = TokenBudget::estimate_tokens(snippet);
        assert!(cost > 0 && cost < 25);

        assert!(budget.try_consume(snippet));
        assert_eq!(budget.remaining_tokens(), 100 - cost);

        let huge_text = "x".repeat(500); // ~125 tokens
        assert!(!budget.can_fit(&huge_text));
        assert!(!budget.try_consume(&huge_text));
        assert_eq!(budget.remaining_tokens(), 100 - cost);
    }
}
