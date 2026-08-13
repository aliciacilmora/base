//! Wall-clock timing for one-shot Denim sequencer builds.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base_protocol::BaseTimeUpdateTx;

/// Deadline for pulling pool transactions into a Denim-active block.
#[derive(Debug, Clone, Copy)]
pub struct TxCutoff {
    deadline: Instant,
    cutoff_time_ms: u64,
}

/// Error converting a Denim transaction cutoff into a system deadline.
#[derive(Debug, thiserror::Error)]
pub enum TxCutoffError {
    /// The configured seal offset cannot be represented in milliseconds.
    #[error("seal offset exceeds u64 milliseconds")]
    SealOffsetOverflow,
    /// The cutoff cannot be represented by the system wall clock.
    #[error("cutoff exceeds the system wall-clock range")]
    WallClockOverflow,
    /// The cutoff cannot be represented by the system monotonic clock.
    #[error("cutoff exceeds the system monotonic-clock range")]
    MonotonicClockOverflow,
}

impl TxCutoff {
    /// Computes `slot_start + seal_offset` for the block timestamp and converts it to a monotonic
    /// deadline.
    pub fn new(block_timestamp_ms: u64, seal_offset: Duration) -> Result<Self, TxCutoffError> {
        let slot_start = block_timestamp_ms - u64::from(BaseTimeUpdateTx::BLOCK_INTERVAL_MILLIS);
        let seal_offset_ms = u64::try_from(seal_offset.as_millis())
            .map_err(|_| TxCutoffError::SealOffsetOverflow)?;
        let cutoff_time_ms = slot_start + seal_offset_ms;
        let cutoff_time = UNIX_EPOCH
            .checked_add(Duration::from_millis(cutoff_time_ms))
            .ok_or(TxCutoffError::WallClockOverflow)?;
        let monotonic_now = Instant::now();
        let wall_clock_now = SystemTime::now();
        let deadline = match cutoff_time.duration_since(wall_clock_now) {
            Ok(remaining) => {
                monotonic_now.checked_add(remaining).ok_or(TxCutoffError::MonotonicClockOverflow)?
            }
            Err(_) => monotonic_now,
        };

        Ok(Self { deadline, cutoff_time_ms })
    }

    /// Returns `true` once the cutoff has been reached.
    pub fn is_past(&self) -> bool {
        Instant::now() >= self.deadline
    }

    /// Returns the cutoff as Unix milliseconds for diagnostics.
    pub const fn cutoff_time_ms(&self) -> u64 {
        self.cutoff_time_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoff_is_slot_start_plus_seal_offset() {
        let cutoff = TxCutoff::new(1_800_000_001_200, Duration::from_millis(150)).unwrap();
        assert_eq!(cutoff.cutoff_time_ms(), 1_800_000_001_150);
    }

    #[test]
    fn past_cutoff_is_past() {
        assert!(
            TxCutoff::new(u64::from(BaseTimeUpdateTx::BLOCK_INTERVAL_MILLIS), Duration::ZERO)
                .unwrap()
                .is_past()
        );
    }

    #[test]
    fn future_cutoff_is_not_past() {
        let year_10k_ms = 253_402_300_800_000;
        assert!(!TxCutoff::new(year_10k_ms, Duration::from_millis(150)).unwrap().is_past());
    }

    #[test]
    fn maximum_future_cutoff_does_not_overflow() {
        assert!(!TxCutoff::new(u64::MAX, Duration::from_millis(150)).unwrap().is_past());
    }

    #[test]
    fn oversized_seal_offset_is_an_error() {
        assert!(matches!(
            TxCutoff::new(u64::from(BaseTimeUpdateTx::BLOCK_INTERVAL_MILLIS), Duration::MAX),
            Err(TxCutoffError::SealOffsetOverflow)
        ));
    }
}
