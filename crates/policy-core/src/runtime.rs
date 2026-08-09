use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EwmaRuntimeEstimator {
    alpha: f64,
    estimates_ns: HashMap<String, f64>,
}

impl EwmaRuntimeEstimator {
    pub fn new(alpha: f64) -> Result<Self, String> {
        if !(0.0..=1.0).contains(&alpha) || alpha == 0.0 {
            return Err("EWMA alpha must be in (0, 1]".into());
        }
        Ok(Self {
            alpha,
            estimates_ns: HashMap::new(),
        })
    }

    pub fn seed(&mut self, class_key: impl Into<String>, estimate_ns: u64) {
        self.estimates_ns
            .insert(class_key.into(), estimate_ns.max(1) as f64);
    }

    pub fn observe(&mut self, class_key: &str, actual_ns: u64) -> u64 {
        let actual = actual_ns.max(1) as f64;
        let next = match self.estimates_ns.get(class_key) {
            Some(previous) => self.alpha * actual + (1.0 - self.alpha) * *previous,
            None => actual,
        };
        self.estimates_ns.insert(class_key.to_string(), next);
        next.round().max(1.0) as u64
    }

    pub fn estimate(&self, class_key: &str, fallback_ns: u64) -> u64 {
        self.estimates_ns
            .get(class_key)
            .copied()
            .unwrap_or(fallback_ns.max(1) as f64)
            .round()
            .max(1.0) as u64
    }

    pub fn snapshot(&self) -> HashMap<String, u64> {
        self.estimates_ns
            .iter()
            .map(|(key, value)| (key.clone(), value.round().max(1.0) as u64))
            .collect()
    }
}

pub fn estimated_remaining_ns(total_estimate_ns: u64, consumed_cpu_ns: u64) -> u64 {
    total_estimate_ns.saturating_sub(consumed_cpu_ns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_saturates() {
        assert_eq!(estimated_remaining_ns(100, 30), 70);
        assert_eq!(estimated_remaining_ns(100, 130), 0);
    }

    #[test]
    fn ewma_updates() {
        let mut e = EwmaRuntimeEstimator::new(0.5).unwrap();
        e.seed("cpu", 100);
        assert_eq!(e.observe("cpu", 200), 150);
    }
}
