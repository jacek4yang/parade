use parade_common::{
    Confidence, InterfaceCounter, ObservationLease, SignedReport, TrafficCheckpoint,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentState {
    pub version: u16,
    pub server_id: String,
    pub agent_id: String,
    pub private_key_hex: String,
    pub sequence: u64,
    pub observed_rx: u64,
    pub observed_tx: u64,
    pub boot_id: String,
    pub interfaces: HashMap<String, RawInterface>,
    pub pending: Option<SignedReport>,
    #[serde(default)]
    pub active_lease: Option<ObservationLease>,
    #[serde(default)]
    pub inventory_hash: String,
    #[serde(default)]
    pub process_hash: String,
    #[serde(default)]
    pub listener_hash: String,
    #[serde(default)]
    pub selected_interfaces: Vec<String>,
    #[serde(default)]
    pub excluded_interfaces: Vec<String>,
    #[serde(default)]
    pub traffic_policy_version: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawInterface {
    pub name: String,
    pub boot_id: String,
    pub rx: u64,
    pub tx: u64,
    #[serde(default)]
    pub sampled_at: i64,
}

impl PersistentState {
    pub fn new(private_key_hex: String) -> Self {
        Self {
            version: 1,
            server_id: String::new(),
            agent_id: String::new(),
            private_key_hex,
            sequence: 0,
            observed_rx: 0,
            observed_tx: 0,
            boot_id: String::new(),
            interfaces: HashMap::new(),
            pending: None,
            active_lease: None,
            inventory_hash: String::new(),
            process_hash: String::new(),
            listener_hash: String::new(),
            selected_interfaces: Vec::new(),
            excluded_interfaces: Vec::new(),
            traffic_policy_version: 0,
        }
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = std::fs::read(path)
            .map_err(|e| format!("cannot read state {}: {e}", path.display()))?;
        let state: Self = serde_json::from_slice(&raw)
            .map_err(|e| format!("invalid state {}: {e}", path.display()))?;
        if state.version != 1 {
            return Err("unsupported Agent state version".into());
        }
        Ok(state)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or("state path has no parent")?;
        std::fs::create_dir_all(parent).map_err(|e| format!("create state directory: {e}"))?;
        let tmp = path.with_extension("tmp");
        let bytes = serde_json::to_vec(self).map_err(|e| format!("encode state: {e}"))?;
        let mut file = std::fs::File::create(&tmp).map_err(|e| format!("create state: {e}"))?;
        file.write_all(&bytes)
            .map_err(|e| format!("write state: {e}"))?;
        file.sync_all().map_err(|e| format!("sync state: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("commit state: {e}"))?;
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| format!("sync state directory: {e}"))
    }
    pub fn accumulate(
        &mut self,
        current: &[InterfaceCounter],
        boot_id: &str,
        sampled_at: i64,
    ) -> TrafficCheckpoint {
        let mut anomalies = Vec::new();
        if current.is_empty() {
            anomalies.push("interface counters unavailable".into());
        } else if !current.iter().any(|interface| interface.selected) {
            anomalies.push("no selected accounting interface".into());
        }
        for interface in current {
            if let Some(previous) = self.interfaces.get(&interface.identity) {
                if interface.selected && previous.boot_id == boot_id {
                    let (rx_delta, rx_transition) = counter_delta(interface.rx_bytes, previous.rx);
                    self.observed_rx = self.observed_rx.saturating_add(rx_delta);
                    if let Some(transition) = rx_transition {
                        anomalies.push(format!("{} rx counter {transition}", interface.name));
                    }
                    let (tx_delta, tx_transition) = counter_delta(interface.tx_bytes, previous.tx);
                    self.observed_tx = self.observed_tx.saturating_add(tx_delta);
                    if let Some(transition) = tx_transition {
                        anomalies.push(format!("{} tx counter {transition}", interface.name));
                    }
                } else if interface.selected {
                    self.observed_rx = self.observed_rx.saturating_add(interface.rx_bytes);
                    self.observed_tx = self.observed_tx.saturating_add(interface.tx_bytes);
                    anomalies.push(format!("{} boot changed", interface.name));
                }
            } else if interface.selected && !self.boot_id.is_empty() {
                self.observed_rx = self.observed_rx.saturating_add(interface.rx_bytes);
                self.observed_tx = self.observed_tx.saturating_add(interface.tx_bytes);
                anomalies.push(format!("{} new counter segment", interface.name));
            }
            self.interfaces.insert(
                interface.identity.clone(),
                RawInterface {
                    name: interface.name.clone(),
                    boot_id: boot_id.into(),
                    rx: interface.rx_bytes,
                    tx: interface.tx_bytes,
                    sampled_at,
                },
            );
        }
        // Identity includes the boot ID. Old boots can never contribute a
        // future delta, and same-boot churn is retained only as a bounded LRU
        // so transient interface disappearance does not cause backfill.
        self.interfaces.retain(|_, value| value.boot_id == boot_id);
        if self.interfaces.len() > 256 {
            let mut oldest = self
                .interfaces
                .iter()
                .map(|(identity, value)| (identity.clone(), value.sampled_at))
                .collect::<Vec<_>>();
            oldest.sort_by_key(|value| value.1);
            for (identity, _) in oldest.into_iter().take(self.interfaces.len() - 256) {
                self.interfaces.remove(&identity);
            }
        }
        self.boot_id = boot_id.into();
        anomalies.truncate(parade_common::MAX_TRAFFIC_ANOMALIES);
        TrafficCheckpoint {
            policy_version: self.traffic_policy_version,
            observed_rx: self.observed_rx,
            observed_tx: self.observed_tx,
            boot_id: boot_id.into(),
            sampled_at,
            confidence: if anomalies.is_empty() {
                Confidence::High
            } else {
                Confidence::Partial
            },
            anomaly_flags: anomalies,
            interfaces: current.to_vec(),
        }
    }
}

fn counter_delta(current: u64, previous: u64) -> (u64, Option<&'static str>) {
    if current >= previous {
        return (current - previous, None);
    }
    // Some older kernels/drivers expose 32-bit counters. Treat a drop from
    // the top quarter of that range as a wrap; ordinary lower-value drops are
    // a reset/new segment. Both remain explicitly Partial evidence.
    if previous <= u64::from(u32::MAX) && previous >= u64::from(u32::MAX) * 3 / 4 {
        return (
            u64::from(u32::MAX)
                .saturating_sub(previous)
                .saturating_add(1)
                .saturating_add(current),
            Some("32-bit wrap"),
        );
    }
    if previous >= u64::MAX - 1_000_000_000 {
        return (
            u64::MAX
                .saturating_sub(previous)
                .saturating_add(1)
                .saturating_add(current),
            Some("64-bit wrap"),
        );
    }
    (current, Some("reset"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn nic(id: &str, rx: u64, tx: u64) -> InterfaceCounter {
        InterfaceCounter {
            name: "eth0".into(),
            identity: id.into(),
            rx_bytes: rx,
            tx_bytes: tx,
            selected: true,
            ..Default::default()
        }
    }
    fn unselected(id: &str, rx: u64, tx: u64) -> InterfaceCounter {
        InterfaceCounter {
            selected: false,
            ..nic(id, rx, tx)
        }
    }
    #[test]
    fn traffic_is_monotonic_across_restart_reboot_and_reset() {
        let mut state = PersistentState::new("key".into());
        state.accumulate(&[nic("boot1:2:mac", 100, 200)], "boot1", 1);
        assert_eq!((state.observed_rx, state.observed_tx), (0, 0));
        state.accumulate(&[nic("boot1:2:mac", 150, 260)], "boot1", 2);
        assert_eq!((state.observed_rx, state.observed_tx), (50, 60));
        state.accumulate(&[nic("boot1:2:mac", 5, 6)], "boot1", 3);
        assert_eq!((state.observed_rx, state.observed_tx), (55, 66));
        state.accumulate(&[nic("boot2:2:mac", 7, 8)], "boot2", 4);
        assert_eq!((state.observed_rx, state.observed_tx), (62, 74));
    }

    #[test]
    fn reselecting_an_interface_does_not_backfill_excluded_bytes() {
        let mut state = PersistentState::new("key".into());
        state.accumulate(&[nic("boot:2:mac", 100, 100)], "boot", 1);
        state.accumulate(&[unselected("boot:2:mac", 200, 300)], "boot", 2);
        state.accumulate(&[nic("boot:2:mac", 250, 350)], "boot", 3);
        assert_eq!((state.observed_rx, state.observed_tx), (50, 50));
    }

    #[test]
    fn rename_keeps_identity_and_wraps_remain_monotonic_and_explicit() {
        let mut state = PersistentState::new("key".into());
        state.accumulate(
            &[nic("boot:2:mac", u64::from(u32::MAX) - 5, 100)],
            "boot",
            1,
        );
        let mut renamed = nic("boot:2:mac", 9, 150);
        renamed.name = "ens3-renamed".into();
        let checkpoint = state.accumulate(&[renamed], "boot", 2);
        assert_eq!(checkpoint.observed_rx, 15);
        assert_eq!(checkpoint.observed_tx, 50);
        assert_eq!(checkpoint.confidence, Confidence::Partial);
        assert!(checkpoint
            .anomaly_flags
            .iter()
            .any(|value| value.contains("32-bit wrap")));
    }

    #[test]
    fn absent_or_unselected_interfaces_never_claim_high_confidence() {
        let mut state = PersistentState::new("key".into());
        assert_eq!(
            state.accumulate(&[], "boot", 1).confidence,
            Confidence::Partial
        );
        assert_eq!(
            state
                .accumulate(&[unselected("boot:2:mac", 10, 20)], "boot", 2)
                .confidence,
            Confidence::Partial
        );
    }

    #[test]
    fn delayed_durable_checkpoint_recovers_same_boot_and_marks_reboot_uncertain() {
        let mut state = PersistentState::new("key".into());
        state.accumulate(&[nic("boot1:2:mac", 100, 100)], "boot1", 1);
        let durable_checkpoint = state.clone();

        state.accumulate(&[nic("boot1:2:mac", 180, 190)], "boot1", 2);
        let mut restarted_same_boot = durable_checkpoint.clone();
        let recovered = restarted_same_boot.accumulate(&[nic("boot1:2:mac", 220, 240)], "boot1", 3);
        assert_eq!((recovered.observed_rx, recovered.observed_tx), (120, 140));
        assert_eq!(recovered.confidence, Confidence::High);

        let mut restarted_after_reboot = durable_checkpoint;
        let uncertain =
            restarted_after_reboot.accumulate(&[nic("boot2:2:mac", 10, 20)], "boot2", 4);
        assert_eq!(uncertain.confidence, Confidence::Partial);
        assert!(uncertain
            .anomaly_flags
            .iter()
            .any(|value| value.contains("new counter segment")));

        // The main loop durably commits every partial transition. A second
        // process crash must therefore resume from this new-boot baseline and
        // add only the later delta, never the whole new segment again.
        let durable_transition = restarted_after_reboot.clone();
        let mut restarted_twice = durable_transition;
        let after_second_crash =
            restarted_twice.accumulate(&[nic("boot2:2:mac", 15, 27)], "boot2", 5);
        assert_eq!(
            (
                after_second_crash.observed_rx,
                after_second_crash.observed_tx
            ),
            (15, 27)
        );
        assert_eq!(after_second_crash.confidence, Confidence::High);
    }

    #[test]
    fn interface_baselines_and_anomalies_remain_bounded() {
        let mut state = PersistentState::new("key".into());
        for boot in 0..300 {
            let boot_id = format!("boot-{boot}");
            state.accumulate(
                &[nic(&format!("{boot_id}:2:mac"), 100, 100)],
                &boot_id,
                boot,
            );
            assert_eq!(state.interfaces.len(), 1);
        }

        let initial = (0..64)
            .map(|index| nic(&format!("boot-many:{index}:mac"), 100, 100))
            .collect::<Vec<_>>();
        state.accumulate(&initial, "boot-many", 400);
        let reset = (0..64)
            .map(|index| nic(&format!("boot-many:{index}:mac"), 1, 1))
            .collect::<Vec<_>>();
        let checkpoint = state.accumulate(&reset, "boot-many", 401);
        assert!(state.interfaces.len() <= 256);
        assert_eq!(checkpoint.interfaces.len(), 64);
        assert_eq!(checkpoint.anomaly_flags.len(), 32);
        assert_eq!(checkpoint.confidence, Confidence::Partial);
    }
}
