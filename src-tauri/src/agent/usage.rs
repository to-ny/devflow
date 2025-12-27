use serde::Serialize;
use std::sync::atomic::{AtomicU32, Ordering};
use ts_rs::TS;

/// Token counts from a single API response.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Thread-safe token usage tracker using atomics for lock-free updates.
#[derive(Default)]
pub struct SessionUsageTracker {
    input_tokens: AtomicU32,
    output_tokens: AtomicU32,
}

impl SessionUsageTracker {
    pub fn new() -> Self {
        Self {
            input_tokens: AtomicU32::new(0),
            output_tokens: AtomicU32::new(0),
        }
    }

    /// Add tokens and return new cumulative totals.
    pub fn add_tokens(&self, input: u32, output: u32) -> UsageTotals {
        let new_input = self.input_tokens.fetch_add(input, Ordering::SeqCst) + input;
        let new_output = self.output_tokens.fetch_add(output, Ordering::SeqCst) + output;

        UsageTotals {
            input_tokens: new_input,
            output_tokens: new_output,
        }
    }

    /// Get current cumulative totals.
    pub fn get_totals(&self) -> UsageTotals {
        UsageTotals {
            input_tokens: self.input_tokens.load(Ordering::SeqCst),
            output_tokens: self.output_tokens.load(Ordering::SeqCst),
        }
    }

    /// Reset all counters to zero.
    pub fn reset(&self) {
        self.input_tokens.store(0, Ordering::SeqCst);
        self.output_tokens.store(0, Ordering::SeqCst);
    }
}

/// Cumulative token usage totals.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export)]
pub struct UsageTotals {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Payload for agent-usage events.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AgentUsagePayload {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub source: UsageSource,
}

/// Source of the token usage.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum UsageSource {
    Main,
    SubAgent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_new_tracker_starts_at_zero() {
        let tracker = SessionUsageTracker::new();
        let totals = tracker.get_totals();
        assert_eq!(totals.input_tokens, 0);
        assert_eq!(totals.output_tokens, 0);
    }

    #[test]
    fn test_add_tokens_accumulates() {
        let tracker = SessionUsageTracker::new();

        let totals1 = tracker.add_tokens(100, 50);
        assert_eq!(totals1.input_tokens, 100);
        assert_eq!(totals1.output_tokens, 50);

        let totals2 = tracker.add_tokens(200, 100);
        assert_eq!(totals2.input_tokens, 300);
        assert_eq!(totals2.output_tokens, 150);

        let totals3 = tracker.get_totals();
        assert_eq!(totals3.input_tokens, 300);
        assert_eq!(totals3.output_tokens, 150);
    }

    #[test]
    fn test_reset_clears_totals() {
        let tracker = SessionUsageTracker::new();
        tracker.add_tokens(100, 50);

        let totals = tracker.get_totals();
        assert_eq!(totals.input_tokens, 100);
        assert_eq!(totals.output_tokens, 50);

        tracker.reset();

        let totals = tracker.get_totals();
        assert_eq!(totals.input_tokens, 0);
        assert_eq!(totals.output_tokens, 0);
    }

    #[test]
    fn test_thread_safety() {
        let tracker = Arc::new(SessionUsageTracker::new());
        let mut handles = vec![];

        // Spawn 10 threads, each adding tokens
        for _ in 0..10 {
            let t = Arc::clone(&tracker);
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    t.add_tokens(10u32, 5u32);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let totals = tracker.get_totals();
        // 10 threads * 10 iterations * 10 input = 1000
        // 10 threads * 10 iterations * 5 output = 500
        assert_eq!(totals.input_tokens, 1000u32);
        assert_eq!(totals.output_tokens, 500u32);
    }

    #[test]
    fn test_default_impl() {
        let tracker = SessionUsageTracker::default();
        let totals = tracker.get_totals();
        assert_eq!(totals.input_tokens, 0);
        assert_eq!(totals.output_tokens, 0);
    }
}
