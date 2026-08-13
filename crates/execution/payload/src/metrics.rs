//! Metrics for the Base payload builder.

base_metrics::define_metrics! {
    payload_builder,
    struct = PayloadBuilderMetrics,
    #[describe("Payload build duration in seconds, from build start through state root")]
    build_duration: histogram,
    #[describe("Builds whose pool-transaction phase stopped at the wall-clock cutoff")]
    cutoff_truncated_builds: counter,
    #[describe("Builds started past their cutoff with zero pool transactions")]
    zero_pool_tx_builds: counter,
}
