use crate::sample::HealthSample;
use std::collections::VecDeque;

/// Standard ring buffer capacity: 15 minutes at 1 Hz = 900 samples (GAP-CLOSURE-PLAN.md §14.5).
pub const DEFAULT_RING_CAPACITY: usize = 900;

/// In-memory bounded circular buffer storing recent `HealthSample`s.
#[derive(Debug, Clone)]
pub struct HealthRingBuffer {
    capacity: usize,
    buffer: VecDeque<HealthSample>,
}

impl Default for HealthRingBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_RING_CAPACITY)
    }
}

impl HealthRingBuffer {
    /// Create a new ring buffer with the given capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            capacity: cap,
            buffer: VecDeque::with_capacity(cap),
        }
    }

    /// Push a new sample into the ring buffer, evicting the oldest sample if at capacity.
    pub fn push(&mut self, sample: HealthSample) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(sample);
    }

    /// Number of samples currently held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// True if no samples are currently recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Maximum capacity of the ring buffer.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The most recent sample recorded, if any.
    #[must_use]
    pub fn latest(&self) -> Option<&HealthSample> {
        self.buffer.back()
    }

    /// Return all samples currently in chronological order (oldest to newest).
    #[must_use]
    pub fn all_samples(&self) -> Vec<HealthSample> {
        self.buffer.iter().cloned().collect()
    }

    /// Return samples with timestamp >= `since_timestamp_ms`.
    #[must_use]
    pub fn samples_since(&self, since_timestamp_ms: u64) -> Vec<HealthSample> {
        self.buffer
            .iter()
            .filter(|s| s.timestamp_ms >= since_timestamp_ms)
            .cloned()
            .collect()
    }

    /// Return samples from the last N minutes.
    #[must_use]
    pub fn last_n_minutes(&self, minutes: usize) -> Vec<HealthSample> {
        let span_ms = (minutes as u64) * 60 * 1000;
        let latest_ts = self.latest().map_or(0, |s| s.timestamp_ms);
        let cutoff_ms = latest_ts.saturating_sub(span_ms);
        self.samples_since(cutoff_ms)
    }

    /// Clears the ring buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_capacity_eviction_and_queries() {
        let mut ring = HealthRingBuffer::new(5);
        assert_eq!(ring.capacity(), 5);
        assert_eq!(ring.len(), 0);
        assert!(ring.is_empty());

        // Push 7 samples with timestamps 1000..7000
        for i in 1..=7 {
            ring.push(HealthSample {
                timestamp_ms: i * 1000,
                ..Default::default()
            });
        }

        // Bounded at 5
        assert_eq!(ring.len(), 5);
        let all = ring.all_samples();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].timestamp_ms, 3000); // 1000 and 2000 evicted
        assert_eq!(all[4].timestamp_ms, 7000);

        // Query since
        let since_5000 = ring.samples_since(5000);
        assert_eq!(since_5000.len(), 3);
        assert_eq!(since_5000[0].timestamp_ms, 5000);

        // Query last 1 minute
        let last_1m = ring.last_n_minutes(1);
        assert_eq!(last_1m.len(), 5);
    }
}
