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
        for interface in current {
            if let Some(previous) = self.interfaces.get(&interface.identity) {
                if interface.selected && previous.boot_id == boot_id {
                    if interface.rx_bytes >= previous.rx {
                        self.observed_rx = self
                            .observed_rx
                            .saturating_add(interface.rx_bytes - previous.rx)
                    } else {
                        self.observed_rx = self.observed_rx.saturating_add(interface.rx_bytes);
                        anomalies.push(format!("{} rx counter reset", interface.name))
                    }
                    if interface.tx_bytes >= previous.tx {
                        self.observed_tx = self
                            .observed_tx
                            .saturating_add(interface.tx_bytes - previous.tx)
                    } else {
                        self.observed_tx = self.observed_tx.saturating_add(interface.tx_bytes);
                        anomalies.push(format!("{} tx counter reset", interface.name))
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
                },
            );
        }
        self.boot_id = boot_id.into();
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
}
