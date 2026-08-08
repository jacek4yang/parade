//! Versioned, closed wire protocol shared by Parade's Hub and Agent.
//!
//! The protocol deliberately contains observations only. In particular there
//! is no stringly typed command, path, script, task, or generic action field.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROTOCOL_VERSION: u16 = 2;
pub const MAX_REPORT_BYTES: usize = 256 * 1024;
pub const MAX_PROCESSES: usize = 32;
pub const MAX_LISTENERS: usize = 128;
pub const MAX_COVERAGE_ITEMS: usize = 64;
pub const MAX_INTERFACES: usize = 64;
pub const MAX_TRAFFIC_ANOMALIES: usize = 32;
pub const MAX_LIVE_DETAIL_SECS: i64 = 600;

/// The complete set of detail levels the Hub may lease from an Agent.
/// Serde's externally tagged representation makes unknown variants fail
/// closed. No variant carries executable text or a user-selected host path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ObservationProfile {
    #[default]
    Normal,
    ResourceDetail,
    ProcessSnapshot,
    SocketSnapshot,
    SecurityLogSummary,
    LiveDetail {
        expires_at: i64,
    },
}

impl ObservationProfile {
    pub fn validate(&self, now: i64) -> Result<(), ProtocolError> {
        if let Self::LiveDetail { expires_at } = self {
            if *expires_at <= now || *expires_at > now + MAX_LIVE_DETAIL_SECS {
                return Err(ProtocolError::InvalidLease);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationLease {
    pub lease_id: String,
    pub profile: ObservationProfile,
    pub issued_at: i64,
    pub expires_at: i64,
}

impl ObservationLease {
    pub fn validate(&self, now: i64) -> Result<(), ProtocolError> {
        if self.lease_id.is_empty()
            || self.lease_id.len() > 64
            || !self.lease_id.bytes().all(|byte| byte.is_ascii_hexdigit())
            || self.issued_at > now + 60
            || self.expires_at <= now
            || self.expires_at <= self.issued_at
            || self.expires_at - self.issued_at > MAX_LIVE_DETAIL_SECS
        {
            return Err(ProtocolError::InvalidLease);
        }
        self.profile.validate(now)?;
        if matches!(self.profile, ObservationProfile::LiveDetail { expires_at } if expires_at != self.expires_at)
        {
            return Err(ProtocolError::InvalidLease);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRequest {
    pub protocol_version: u16,
    pub token: String,
    pub public_key_hex: String,
    pub agent_nonce: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentResponse {
    pub protocol_version: u16,
    pub server_id: String,
    pub agent_id: String,
    pub report_url: String,
    pub next_boundary: Option<i64>,
    pub traffic_policy: TrafficPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TrafficPolicy {
    pub version: u32,
    pub selected_interfaces: Vec<String>,
    pub excluded_interfaces: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportAck {
    pub accepted: bool,
    pub duplicate: bool,
    pub sequence: u64,
    pub lease: Option<ObservationLease>,
    pub traffic_policy: TrafficPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ResourceRollup {
    pub interval_start: i64,
    pub interval_end: i64,
    pub samples: u32,
    pub cpu_avg_pct: f32,
    pub cpu_max_pct: f32,
    pub cpu_cores: u32,
    pub load1_avg: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disk_total: u64,
    pub disk_used: u64,
    pub disk_inodes_total: u64,
    pub disk_inodes_used: u64,
    pub psi_cpu_some_avg10: Option<f32>,
    pub psi_mem_some_avg10: Option<f32>,
    pub psi_io_some_avg10: Option<f32>,
    pub tcp_connections: u32,
    pub udp_connections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct InterfaceCounter {
    pub name: String,
    pub identity: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_drops: u64,
    pub tx_drops: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct TrafficCheckpoint {
    /// Version of the Hub interface policy used for this checkpoint. Version
    /// zero is the enrollment/default policy before the first Hub rule ack.
    pub policy_version: u32,
    /// Monotonic Parade-maintained counters; raw Linux counters may reset.
    pub observed_rx: u64,
    pub observed_tx: u64,
    pub boot_id: String,
    pub sampled_at: i64,
    pub confidence: Confidence,
    pub anomaly_flags: Vec<String>,
    pub interfaces: Vec<InterfaceCounter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    #[default]
    Partial,
    Estimated,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProcessSummary {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub state: String,
    pub executable: String,
    pub cpu_ticks: u64,
    pub rss_bytes: u64,
    pub virtual_bytes: u64,
    pub started_ticks: u64,
    pub cgroup: Option<String>,
    pub systemd_unit: Option<String>,
    pub listening_sockets: u16,
    pub deleted_executable: bool,
    pub suspicious_writable_path: bool,
    pub package_ownership: PackageOwnership,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PackageOwnership {
    Owned,
    Unowned,
    #[default]
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListenerSummary {
    pub protocol: String,
    pub local_address: String,
    pub port: u16,
    pub uid: Option<u32>,
    pub inode: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageItem {
    pub collector: String,
    pub status: Confidence,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryReport {
    pub agent_version: String,
    pub profile: ObservationProfile,
    pub uptime_secs: u64,
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub inventory_hash: String,
    /// Present when this report was produced for a Hub-issued lease.
    pub lease_id: Option<String>,
    pub resources: ResourceRollup,
    pub traffic: TrafficCheckpoint,
    /// Stable top-N and suspicious facts only. Never command lines or env.
    pub processes: Option<Vec<ProcessSummary>>,
    pub listeners: Option<Vec<ListenerSummary>>,
    pub coverage: Vec<CoverageItem>,
}

impl TelemetryReport {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self
            .processes
            .as_ref()
            .is_some_and(|values| values.len() > MAX_PROCESSES)
            || self
                .listeners
                .as_ref()
                .is_some_and(|values| values.len() > MAX_LISTENERS)
            || self.coverage.len() > MAX_COVERAGE_ITEMS
            || self.traffic.interfaces.len() > MAX_INTERFACES
            || self.traffic.anomaly_flags.len() > MAX_TRAFFIC_ANOMALIES
        {
            return Err(ProtocolError::TooLarge);
        }
        let bounded = [
            (&self.agent_version, 64usize),
            (&self.os, 256),
            (&self.kernel, 128),
            (&self.arch, 32),
            (&self.inventory_hash, 128),
        ];
        if bounded.iter().any(|(value, max)| value.len() > *max)
            || self.lease_id.as_ref().is_some_and(|value| value.len() > 64)
            || self.traffic.boot_id.len() > 128
            || self
                .traffic
                .anomaly_flags
                .iter()
                .any(|value| value.len() > 128)
            || self
                .coverage
                .iter()
                .any(|value| value.collector.len() > 64 || value.detail.len() > 256)
            || self.processes.iter().flatten().any(|value| {
                value.state.len() > 32
                    || value.executable.len() > 512
                    || value.cgroup.as_ref().is_some_and(|item| item.len() > 256)
                    || value
                        .systemd_unit
                        .as_ref()
                        .is_some_and(|item| item.len() > 128)
            })
            || self
                .listeners
                .iter()
                .flatten()
                .any(|value| value.protocol.len() > 16 || value.local_address.len() > 128)
            || self
                .traffic
                .interfaces
                .iter()
                .any(|value| value.name.len() > 64 || value.identity.len() > 256)
        {
            return Err(ProtocolError::TooLarge);
        }
        if self.resources.interval_end < self.resources.interval_start {
            return Err(ProtocolError::Malformed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedReport {
    pub protocol_version: u16,
    pub server_id: String,
    pub agent_id: String,
    pub sent_at: i64,
    pub sequence: u64,
    pub message_id: String,
    pub body: TelemetryReport,
    pub signature_hex: String,
}

#[derive(Serialize)]
struct SignableReport<'a> {
    protocol_version: u16,
    server_id: &'a str,
    agent_id: &'a str,
    sent_at: i64,
    sequence: u64,
    message_id: &'a str,
    body: &'a TelemetryReport,
}

impl SignedReport {
    pub fn new(
        server_id: String,
        agent_id: String,
        sent_at: i64,
        sequence: u64,
        message_id: String,
        body: TelemetryReport,
        key: &SigningKey,
    ) -> Result<Self, ProtocolError> {
        body.validate()?;
        let mut report = Self {
            protocol_version: PROTOCOL_VERSION,
            server_id,
            agent_id,
            sent_at,
            sequence,
            message_id,
            body,
            signature_hex: String::new(),
        };
        report.signature_hex = hex_encode(&key.sign(&report.signing_bytes()?).to_bytes());
        Ok(report)
    }

    pub fn signing_bytes(&self) -> Result<Vec<u8>, ProtocolError> {
        postcard::to_allocvec(&SignableReport {
            protocol_version: self.protocol_version,
            server_id: &self.server_id,
            agent_id: &self.agent_id,
            sent_at: self.sent_at,
            sequence: self.sequence,
            message_id: &self.message_id,
            body: &self.body,
        })
        .map_err(|_| ProtocolError::Malformed)
    }

    pub fn verify(&self, public_key_hex: &str) -> Result<(), ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion);
        }
        self.body.validate()?;
        if self.server_id.is_empty()
            || self.server_id.len() > 64
            || self.agent_id.is_empty()
            || self.agent_id.len() > 64
            || self.message_id.len() != 64
            || !self.message_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(ProtocolError::Malformed);
        }
        let pk: [u8; 32] = hex_decode(public_key_hex)?
            .try_into()
            .map_err(|_| ProtocolError::BadKey)?;
        let sig: [u8; 64] = hex_decode(&self.signature_hex)?
            .try_into()
            .map_err(|_| ProtocolError::BadSignature)?;
        let key = VerifyingKey::from_bytes(&pk).map_err(|_| ProtocolError::BadKey)?;
        key.verify(&self.signing_bytes()?, &Signature::from_bytes(&sig))
            .map_err(|_| ProtocolError::BadSignature)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    UnsupportedVersion,
    BadKey,
    BadSignature,
    InvalidLease,
    TooLarge,
    Malformed,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex_encode(&Sha256::digest(bytes))
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

pub fn hex_decode(value: &str) -> Result<Vec<u8>, ProtocolError> {
    if !value.len().is_multiple_of(2) {
        return Err(ProtocolError::Malformed);
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16).map_err(|_| ProtocolError::Malformed)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> TelemetryReport {
        TelemetryReport {
            agent_version: "test".into(),
            profile: ObservationProfile::Normal,
            uptime_secs: 1,
            os: "Linux".into(),
            kernel: "test".into(),
            arch: "x86_64".into(),
            inventory_hash: "inventory".into(),
            lease_id: None,
            resources: ResourceRollup {
                interval_start: 1,
                interval_end: 2,
                ..Default::default()
            },
            traffic: TrafficCheckpoint::default(),
            processes: Some(vec![]),
            listeners: Some(vec![]),
            coverage: vec![],
        }
    }

    #[test]
    fn signed_report_detects_body_and_identity_tampering() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let mut signed = SignedReport::new(
            "server-a".into(),
            "agent-a".into(),
            100,
            1,
            "a".repeat(64),
            report(),
            &key,
        )
        .unwrap();
        let public = hex_encode(key.verifying_key().as_bytes());
        assert_eq!(signed.verify(&public), Ok(()));
        signed.server_id = "server-b".into();
        assert_eq!(signed.verify(&public), Err(ProtocolError::BadSignature));
    }

    #[test]
    fn protocol_has_no_generic_execution_variant() {
        let encoded = serde_json::to_string(&ObservationProfile::Normal).unwrap();
        assert!(serde_json::from_str::<ObservationProfile>("\"shell\"").is_err());
        assert!(serde_json::from_str::<ObservationProfile>("{\"command\":\"id\"}").is_err());
        assert_eq!(encoded, "\"normal\"");
    }

    #[test]
    fn live_detail_is_short_and_expiring() {
        assert!(ObservationProfile::LiveDetail { expires_at: 1_599 }
            .validate(1_000)
            .is_ok());
        assert_eq!(
            ObservationProfile::LiveDetail { expires_at: 1_601 }.validate(1_000),
            Err(ProtocolError::InvalidLease)
        );
    }
}
