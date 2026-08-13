//! Wall-clock timing for one-shot Denim sequencer builds.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base_protocol::BaseTimeUpdateTx;

/// Deadline for pulling pool transactions into a Denim-active block.
#[derive(Debug, Clone, Copy)]
pub struct TxCutoff {
    instant: Instant,
    unix_millis: u64,
}

impl TxCutoff {
    /// Computes `slot_start + seal_offset` for the block timestamp and converts it to a monotonic
    /// deadline.
    pub fn new(block_timestamp_ms: u64, seal_offset: Duration) -> Self {
        let slot_start = block_timestamp_ms - u64::from(BaseTimeUpdateTx::BLOCK_INTERVAL_MILLIS);
        let offset =
            u64::try_from(seal_offset.as_millis()).expect("seal offset milliseconds fit in u64");
        let unix_millis = slot_start + offset;
        let now = Instant::now();
        let unix_now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is after the Unix epoch")
            .as_millis()
            .try_into()
            .expect("Unix time milliseconds fit in u64");
        let instant = if unix_millis >= unix_now_millis {
            now.checked_add(Duration::from_millis(unix_millis - unix_now_millis))
                .expect("cutoff deadline fits in Instant")
        } else {
            now
        };

        Self { instant, unix_millis }
    }

    /// Returns `true` once the cutoff has been reached.
    pub fn is_past(&self) -> bool {
        Instant::now() >= self.instant
    }

    /// Returns the cutoff as Unix milliseconds for diagnostics.
    pub const fn unix_millis(&self) -> u64 {
        self.unix_millis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoff_is_slot_start_plus_seal_offset() {
        let cutoff = TxCutoff::new(1_800_000_001_200, Duration::from_millis(150));
        assert_eq!(cutoff.unix_millis(), 1_800_000_001_150);
    }

    #[test]
    fn past_cutoff_is_past() {
        assert!(
            TxCutoff::new(u64::from(BaseTimeUpdateTx::BLOCK_INTERVAL_MILLIS), Duration::ZERO)
                .is_past()
        );
    }

    #[test]
    fn future_cutoff_is_not_past() {
        let year_10k_ms = 253_402_300_800_000;
        assert!(!TxCutoff::new(year_10k_ms, Duration::from_millis(150)).is_past());
    }

    #[test]
    fn maximum_future_cutoff_does_not_overflow() {
        assert!(!TxCutoff::new(u64::MAX, Duration::from_millis(150)).is_past());
    }
}
