//! Outbound-only, unprivileged Parade Agent.

mod collect;
mod state;
mod syscall;

use collect::{Collector, ResourceSample};
use ed25519_dalek::SigningKey;
use parade_common::{
    sha256_hex, EnrollmentRequest, EnrollmentResponse, ObservationProfile, ReportAck,
    ResourceRollup, SignedReport, TelemetryReport, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use state::PersistentState;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    report_url: String,
    server_id: String,
    agent_id: String,
    state_file: String,
    #[serde(default = "sample_interval")]
    sample_interval_secs: u64,
    #[serde(default = "upload_interval")]
    upload_interval_secs: u64,
    #[serde(default = "jitter")]
    jitter_secs: u64,
    #[serde(default)]
    net_interfaces: Vec<String>,
    #[serde(default)]
    excluded_interfaces: Vec<String>,
}
fn sample_interval() -> u64 {
    10
}
fn upload_interval() -> u64 {
    300
}
fn jitter() -> u64 {
    30
}

#[derive(Default)]
struct Rollup {
    start: i64,
    samples: u32,
    cpu_sum: f64,
    cpu_max: f32,
    load_sum: f64,
    last: ResourceSample,
}
impl Rollup {
    fn add(&mut self, at: i64, sample: ResourceSample) {
        if self.samples == 0 {
            self.start = at
        }
        self.samples += 1;
        self.cpu_sum += sample.cpu_pct as f64;
        self.cpu_max = self.cpu_max.max(sample.cpu_pct);
        self.load_sum += sample.load1 as f64;
        self.last = sample;
    }
    fn finish(&mut self, end: i64) -> ResourceRollup {
        let n = self.samples.max(1) as f64;
        let value = ResourceRollup {
            interval_start: self.start,
            interval_end: end,
            samples: self.samples,
            cpu_avg_pct: (self.cpu_sum / n) as f32,
            cpu_max_pct: self.cpu_max,
            cpu_cores: self.last.cpu_cores,
            load1_avg: (self.load_sum / n) as f32,
            mem_total: self.last.mem_total,
            mem_used: self.last.mem_used,
            swap_total: self.last.swap_total,
            swap_used: self.last.swap_used,
            disk_total: self.last.disk_total,
            disk_used: self.last.disk_used,
            disk_inodes_total: self.last.disk_inodes_total,
            disk_inodes_used: self.last.disk_inodes_used,
            psi_cpu_some_avg10: self.last.psi_cpu,
            psi_mem_some_avg10: self.last.psi_mem,
            psi_io_some_avg10: self.last.psi_io,
            tcp_connections: self.last.tcp,
            udp_connections: self.last.udp,
        };
        *self = Self::default();
        value
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--version") => {
            println!("parade-agent {VERSION}");
            return;
        }
        Some("enroll") => {
            if let Err(error) = enroll(&args[2..]) {
                eprintln!("parade-agent: {error}");
                std::process::exit(1)
            }
            return;
        }
        Some("check-config") => {
            let path = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("/etc/parade/agent.toml");
            if let Err(error) = load_config(path)
                .and_then(|cfg| PersistentState::load(Path::new(&cfg.state_file)).map(|_| ()))
            {
                eprintln!("parade-agent: {error}");
                std::process::exit(1)
            }
            println!("configuration and identity are readable");
            return;
        }
        _ => {}
    }
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("/etc/parade/agent.toml");
    if let Err(error) = run(path) {
        eprintln!("parade-agent: {error}");
        std::process::exit(1)
    }
}

fn run(config_path: &str) -> Result<(), String> {
    let cfg = load_config(config_path)?;
    validate_hub_url(&cfg.report_url)?;
    let state_path = PathBuf::from(&cfg.state_file);
    let mut state = PersistentState::load(&state_path)?;
    if state.server_id != cfg.server_id || state.agent_id != cfg.agent_id {
        return Err("identity is not bound to this configuration".into());
    }
    if state.selected_interfaces.is_empty() && state.excluded_interfaces.is_empty() {
        state.selected_interfaces = cfg.net_interfaces.clone();
        state.excluded_interfaces = cfg.excluded_interfaces.clone();
    }
    let key_bytes: [u8; 32] = parade_common::hex_decode(&state.private_key_hex)
        .map_err(|_| "invalid private key")?
        .try_into()
        .map_err(|_| "invalid private key length")?;
    let signing = SigningKey::from_bytes(&key_bytes);
    let client = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    let collector = Collector::new("/");
    let mut previous = collector.cpu_sample().unwrap_or_default();
    let mut rollup = Rollup::default();
    let mut next_upload = now()
        + cfg.upload_interval_secs as i64
        + jitter_for(&state.agent_id, cfg.jitter_secs) as i64;
    let mut retry_secs = 10u64;
    eprintln!(
        "parade-agent {VERSION} · {} · normal upload every {}s + jitter · read-only",
        cfg.server_id, cfg.upload_interval_secs
    );
    loop {
        std::thread::sleep(Duration::from_secs(cfg.sample_interval_secs.clamp(5, 60)));
        let at = now();
        let current = collector.cpu_sample().unwrap_or(previous);
        let sample = collector.resources(previous, current);
        previous = current;
        rollup.add(at, sample);
        let boot = collector.boot_id();
        let interfaces = collector.interfaces(
            &state.selected_interfaces,
            &state.excluded_interfaces,
            &boot,
        );
        let checkpoint = state.accumulate(&interfaces, &boot, at);
        state.save(&state_path)?;
        if let Some(lease) = &state.active_lease {
            if lease.validate(at).is_err() {
                state.active_lease = None;
            }
        }
        if state.pending.is_some() {
            match upload(
                &client,
                &cfg.report_url,
                state.pending.as_ref().expect("pending"),
            ) {
                Ok(ack) => {
                    state.pending = None;
                    // `None` is authoritative too: it ends a cancelled Hub
                    // lease at the next successful outbound acknowledgement.
                    state.active_lease = ack.lease;
                    if state.traffic_policy_version != ack.traffic_policy.version
                        || state.selected_interfaces != ack.traffic_policy.selected_interfaces
                        || state.excluded_interfaces != ack.traffic_policy.excluded_interfaces
                    {
                        state.traffic_policy_version = ack.traffic_policy.version;
                        state.selected_interfaces = ack.traffic_policy.selected_interfaces;
                        state.excluded_interfaces = ack.traffic_policy.excluded_interfaces;
                    }
                    retry_secs = 10;
                    state.save(&state_path)?
                }
                Err(UploadError::Retry) => {
                    std::thread::sleep(Duration::from_secs(retry_secs));
                    retry_secs = (retry_secs * 2).min(300);
                    continue;
                }
                Err(UploadError::Rejected) => {
                    return Err("Hub rejected this Agent identity or sequence; re-enrollment or operator review is required".into())
                }
            }
        }
        if at < next_upload {
            continue;
        }
        let uptime_secs = rollup.last.uptime;
        let resources = rollup.finish(at);
        let inventory = format!(
            "{}\n{}\n{}\n{}",
            collector.os(),
            collector.kernel(),
            std::env::consts::ARCH,
            resources.cpu_cores
        );
        let inventory_hash = sha256_hex(inventory.as_bytes());
        let process_values = collector.processes();
        let process_bytes =
            postcard::to_allocvec(&process_values).map_err(|e| format!("encode processes: {e}"))?;
        let process_hash = sha256_hex(&process_bytes);
        let processes = if process_hash != state.process_hash {
            state.process_hash = process_hash;
            Some(process_values)
        } else {
            None
        };
        let listener_values = collector.listeners();
        let listener_bytes = postcard::to_allocvec(&listener_values)
            .map_err(|e| format!("encode listeners: {e}"))?;
        let listener_hash = sha256_hex(&listener_bytes);
        let listeners = if listener_hash != state.listener_hash {
            state.listener_hash = listener_hash;
            Some(listener_values)
        } else {
            None
        };
        state.inventory_hash = inventory_hash.clone();
        let profile = state
            .active_lease
            .as_ref()
            .map(|lease| lease.profile.clone())
            .unwrap_or(ObservationProfile::Normal);
        let lease_id = state
            .active_lease
            .as_ref()
            .map(|lease| lease.lease_id.clone());
        let body = TelemetryReport {
            agent_version: VERSION.into(),
            profile,
            uptime_secs,
            os: collector.os(),
            kernel: collector.kernel(),
            arch: std::env::consts::ARCH.into(),
            inventory_hash,
            lease_id,
            resources,
            traffic: checkpoint,
            processes,
            listeners,
            coverage: collector.coverage(),
        };
        state.sequence = state.sequence.checked_add(1).ok_or("sequence exhausted")?;
        let signed = SignedReport::new(
            cfg.server_id.clone(),
            cfg.agent_id.clone(),
            at,
            state.sequence,
            random_hex::<32>(),
            body,
            &signing,
        )
        .map_err(|e| format!("build report: {e:?}"))?;
        state.pending = Some(signed);
        state.save(&state_path)?;
        next_upload = at
            + cfg.upload_interval_secs.clamp(60, 3600) as i64
            + jitter_for(&random_hex::<8>(), cfg.jitter_secs.min(120)) as i64;
    }
}

fn enroll(args: &[String]) -> Result<(), String> {
    let mut hub = None;
    let mut token = std::env::var("PARADE_ENROLL_TOKEN").ok();
    let mut config = None;
    let mut state_path = None;
    let mut i = 0;
    while i < args.len() {
        let value = args
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {}", args[i]))?
            .clone();
        match args[i].as_str() {
            "--hub" => hub = Some(value),
            "--token" => token = Some(value),
            "--config" => config = Some(PathBuf::from(value)),
            "--state" => state_path = Some(PathBuf::from(value)),
            other => return Err(format!("unknown enrollment option {other}")),
        }
        i += 2
    }
    let hub = hub.ok_or("--hub is required")?;
    validate_hub_url(&hub)?;
    let token = token.ok_or("PARADE_ENROLL_TOKEN or --token is required")?;
    let config_path = config.ok_or("--config is required")?;
    let state_path = state_path.ok_or("--state is required")?;
    let mut private = [0u8; 32];
    getrandom::getrandom(&mut private).map_err(|e| format!("operating-system RNG: {e}"))?;
    let signing = SigningKey::from_bytes(&private);
    let request = EnrollmentRequest {
        protocol_version: PROTOCOL_VERSION,
        token,
        public_key_hex: parade_common::hex_encode(signing.verifying_key().as_bytes()),
        agent_nonce: random_hex::<32>(),
    };
    let endpoint = format!("{}/api/v1/enroll", hub.trim_end_matches('/'));
    let response = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
        .post(&endpoint)
        .set("Content-Type", "application/json")
        .send_json(&request)
        .map_err(|e| format!("enrollment request failed: {e}"))?;
    let enrolled: EnrollmentResponse = response
        .into_json()
        .map_err(|e| format!("invalid enrollment response: {e}"))?;
    if enrolled.protocol_version != PROTOCOL_VERSION {
        return Err("Hub returned an unsupported protocol".into());
    }
    let cfg = Config {
        report_url: enrolled.report_url,
        server_id: enrolled.server_id.clone(),
        agent_id: enrolled.agent_id.clone(),
        state_file: state_path.to_string_lossy().into_owned(),
        sample_interval_secs: sample_interval(),
        upload_interval_secs: upload_interval(),
        jitter_secs: jitter(),
        net_interfaces: enrolled.traffic_policy.selected_interfaces.clone(),
        excluded_interfaces: enrolled.traffic_policy.excluded_interfaces.clone(),
    };
    // Re-enrollment is credential rotation, not a traffic-accounting reset.
    // Preserve the server's raw baselines and monotonic accumulator when the
    // existing state belongs to the same server, while starting a fresh replay
    // stream for the new independent Agent identity.
    let existing = PersistentState::load(&state_path).ok();
    let mut state = existing
        .filter(|value| value.server_id == enrolled.server_id)
        .unwrap_or_else(|| PersistentState::new(parade_common::hex_encode(&private)));
    state.private_key_hex = parade_common::hex_encode(&private);
    state.server_id = enrolled.server_id;
    state.agent_id = enrolled.agent_id;
    state.sequence = 0;
    state.pending = None;
    state.active_lease = None;
    state.selected_interfaces = enrolled.traffic_policy.selected_interfaces.clone();
    state.excluded_interfaces = enrolled.traffic_policy.excluded_interfaces.clone();
    state.traffic_policy_version = enrolled.traffic_policy.version;
    state.save(&state_path)?;
    write_atomic(
        &config_path,
        toml::to_string_pretty(&cfg)
            .map_err(|e| format!("encode config: {e}"))?
            .as_bytes(),
    )?;
    println!("enrolled {} as {}", cfg.server_id, cfg.agent_id);
    Ok(())
}

enum UploadError {
    Retry,
    Rejected,
}
fn upload(
    client: &ureq::Agent,
    url: &str,
    report: &SignedReport,
) -> Result<ReportAck, UploadError> {
    let bytes = postcard::to_allocvec(report).map_err(|_| UploadError::Rejected)?;
    match client
        .post(url)
        .set("Content-Type", "application/x-parade")
        .send_bytes(&bytes)
    {
        Ok(response) => response.into_json().map_err(|_| UploadError::Retry),
        Err(ureq::Error::Status(409, _)) => Err(UploadError::Rejected),
        Err(ureq::Error::Status(401 | 403 | 404, _)) => Err(UploadError::Rejected),
        Err(_) => Err(UploadError::Retry),
    }
}
fn load_config(path: &str) -> Result<Config, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read config {path}: {e}"))?;
    toml::from_str(&raw).map_err(|e| format!("invalid config {path}: {e}"))
}
fn validate_hub_url(value: &str) -> Result<(), String> {
    if value.starts_with("https://") || exact_loopback_http(value) {
        Ok(())
    } else {
        Err("Hub URL must use HTTPS (HTTP is allowed only for loopback development)".into())
    }
}
fn exact_loopback_http(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    authority == "localhost"
        || authority
            .strip_prefix("localhost:")
            .is_some_and(|port| port.parse::<u16>().is_ok())
        || authority == "127.0.0.1"
        || authority
            .strip_prefix("127.0.0.1:")
            .is_some_and(|port| port.parse::<u16>().is_ok())
        || authority == "[::1]"
        || authority
            .strip_prefix("[::1]:")
            .is_some_and(|port| port.parse::<u16>().is_ok())
}
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write as _;
    let parent = path.parent().ok_or("path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("create directory: {e}"))?;
    let tmp = path.with_extension("tmp");
    let mut file =
        std::fs::File::create(&tmp).map_err(|e| format!("create {}: {e}", tmp.display()))?;
    file.write_all(bytes)
        .map_err(|e| format!("write {}: {e}", tmp.display()))?;
    file.sync_all()
        .map_err(|e| format!("sync {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("commit {}: {e}", path.display()))?;
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|e| format!("sync {}: {e}", parent.display()))
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn random_hex<const N: usize>() -> String {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).expect("operating-system RNG");
    parade_common::hex_encode(&bytes)
}
fn jitter_for(value: &str, max: u64) -> u64 {
    if max == 0 {
        return 0;
    }
    let digest = parade_common::hex_decode(&sha256_hex(value.as_bytes())).expect("digest");
    u64::from_be_bytes(digest[..8].try_into().expect("digest size")) % (max + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use parade_common::{Confidence, CoverageItem, TrafficCheckpoint};
    #[test]
    fn default_monthly_bandwidth_is_below_target() {
        let key = SigningKey::from_bytes(&[3; 32]);
        let report = SignedReport::new(
            "server".into(),
            "agent".into(),
            1,
            1,
            "a".repeat(64),
            TelemetryReport {
                agent_version: "0.1.0".into(),
                profile: ObservationProfile::Normal,
                uptime_secs: 1,
                os: "Debian GNU/Linux 13".into(),
                kernel: "6.12.0".into(),
                arch: "x86_64".into(),
                inventory_hash: "b".repeat(64),
                lease_id: None,
                resources: ResourceRollup {
                    interval_start: 1,
                    interval_end: 301,
                    samples: 30,
                    cpu_avg_pct: 2.0,
                    cpu_max_pct: 8.0,
                    cpu_cores: 2,
                    mem_total: 1024,
                    mem_used: 512,
                    ..Default::default()
                },
                traffic: TrafficCheckpoint {
                    observed_rx: 1_000,
                    observed_tx: 2_000,
                    boot_id: "boot".into(),
                    sampled_at: 301,
                    confidence: Confidence::High,
                    ..Default::default()
                },
                processes: None,
                listeners: None,
                coverage: vec![CoverageItem {
                    collector: "resources".into(),
                    status: Confidence::High,
                    detail: "available".into(),
                }],
            },
            &key,
        )
        .unwrap();
        let encoded = postcard::to_allocvec(&report).unwrap().len() as u64;
        let reports = 30 * 24 * 60 / 5;
        let request_overhead = 250u64;
        let daily_tls_handshake = 2048u64;
        let monthly = reports * (encoded + request_overhead) + 30 * daily_tls_handshake;
        eprintln!(
            "BANDWIDTH encoded_body_bytes={encoded} reports_per_30d={reports} request_overhead_bytes={request_overhead} daily_tls_handshake_bytes={daily_tls_handshake} monthly_bytes={monthly} monthly_mib={:.3}",
            monthly as f64 / (1024.0 * 1024.0)
        );
        assert!(
            monthly < 10 * 1024 * 1024,
            "{} bytes/month (body {} bytes)",
            monthly,
            encoded
        );
        assert!(monthly < 20 * 1024 * 1024);
    }
    #[test]
    fn remote_plain_http_is_rejected() {
        assert!(validate_hub_url("http://example.com").is_err());
        assert!(validate_hub_url("https://example.com").is_ok());
        assert!(validate_hub_url("http://127.0.0.1:8008").is_ok());
        assert!(validate_hub_url("http://localhost.attacker.example").is_err());
        assert!(validate_hub_url("http://127.0.0.1.evil").is_err());
    }
}
