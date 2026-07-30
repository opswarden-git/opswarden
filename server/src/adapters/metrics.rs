use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy)]
pub enum AlertmanagerOutcome {
    Accepted,
    Rejected,
    Duplicate,
    Ignored,
    Failed,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AlertmanagerMetricsSnapshot {
    pub accepted: u64,
    pub rejected: u64,
    pub duplicate: u64,
    pub ignored: u64,
    pub failed: u64,
}

#[derive(Default)]
pub struct AlertmanagerWebhookMetrics {
    accepted: AtomicU64,
    rejected: AtomicU64,
    duplicate: AtomicU64,
    ignored: AtomicU64,
    failed: AtomicU64,
}

impl AlertmanagerWebhookMetrics {
    pub fn record(&self, outcome: AlertmanagerOutcome) {
        let counter = match outcome {
            AlertmanagerOutcome::Accepted => &self.accepted,
            AlertmanagerOutcome::Rejected => &self.rejected,
            AlertmanagerOutcome::Duplicate => &self.duplicate,
            AlertmanagerOutcome::Ignored => &self.ignored,
            AlertmanagerOutcome::Failed => &self.failed,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> AlertmanagerMetricsSnapshot {
        AlertmanagerMetricsSnapshot {
            accepted: self.accepted.load(Ordering::Relaxed),
            rejected: self.rejected.load(Ordering::Relaxed),
            duplicate: self.duplicate.load(Ordering::Relaxed),
            ignored: self.ignored.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
        }
    }

    pub fn render_prometheus(&self) -> String {
        let snapshot = self.snapshot();
        let mut output = String::from(
            "# HELP opswarden_alertmanager_webhook_deliveries_total Alertmanager alert transitions handled by outcome.\n\
             # TYPE opswarden_alertmanager_webhook_deliveries_total counter\n",
        );
        for (outcome, value) in [
            ("accepted", snapshot.accepted),
            ("rejected", snapshot.rejected),
            ("duplicate", snapshot.duplicate),
            ("ignored", snapshot.ignored),
            ("failed", snapshot.failed),
        ] {
            output.push_str(&format!(
                "opswarden_alertmanager_webhook_deliveries_total{{outcome=\"{outcome}\"}} {value}\n"
            ));
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_every_bounded_outcome() {
        let metrics = AlertmanagerWebhookMetrics::default();
        metrics.record(AlertmanagerOutcome::Accepted);
        metrics.record(AlertmanagerOutcome::Duplicate);

        let output = metrics.render_prometheus();
        assert!(output.contains("outcome=\"accepted\"} 1"));
        assert!(output.contains("outcome=\"rejected\"} 0"));
        assert!(output.contains("outcome=\"duplicate\"} 1"));
        assert!(output.contains("outcome=\"ignored\"} 0"));
        assert!(output.contains("outcome=\"failed\"} 0"));
    }
}
