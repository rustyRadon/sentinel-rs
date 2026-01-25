use metrics::{gauge, counter};

#[derive(Clone)]
pub struct Metrics;

impl Metrics {
    pub fn new() -> Self { Self }

    pub fn increment_connections(&self) {
        gauge!("sentinel_active_connections").increment(1.0);
        counter!("sentinel_total_connections").increment(1);
    }

    pub fn decrement_connections(&self) {
        gauge!("sentinel_active_connections").decrement(1.0);
    }
}