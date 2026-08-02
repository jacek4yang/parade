//! Read-only Linux collectors with a replaceable fixture root.

use crate::syscall;
use parade_common::{
    Confidence, CoverageItem, InterfaceCounter, ListenerSummary, PackageOwnership, ProcessSummary,
    MAX_LISTENERS, MAX_PROCESSES,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Collector {
    root: PathBuf,
}

#[derive(Clone, Copy, Default)]
pub struct CpuSample {
    total: u64,
    idle: u64,
}

#[derive(Default)]
pub struct ResourceSample {
    pub cpu_pct: f32,
    pub cpu_cores: u32,
    pub load1: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disk_total: u64,
    pub disk_used: u64,
    pub disk_inodes_total: u64,
    pub disk_inodes_used: u64,
    pub psi_cpu: Option<f32>,
    pub psi_mem: Option<f32>,
    pub psi_io: Option<f32>,
    pub tcp: u32,
    pub udp: u32,
    pub uptime: u64,
}

impl Collector {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    fn path(&self, path: &str) -> PathBuf {
        self.root.join(path.trim_start_matches('/'))
    }
    fn read(&self, path: &str) -> Option<String> {
        fs::read_to_string(self.path(path)).ok()
    }

    pub fn cpu_sample(&self) -> Option<CpuSample> {
        let value = self.read("proc/stat")?;
        let mut fields = value.lines().next()?.split_whitespace();
        if fields.next()? != "cpu" {
            return None;
        }
        let values: Vec<u64> = fields.filter_map(|v| v.parse().ok()).collect();
        if values.len() < 4 {
            return None;
        }
        let idle = values[3] + values.get(4).copied().unwrap_or(0);
        let guest = values.get(8).copied().unwrap_or(0) + values.get(9).copied().unwrap_or(0);
        Some(CpuSample {
            total: values.iter().sum::<u64>().saturating_sub(guest),
            idle,
        })
    }
    pub fn cpu_pct(previous: CpuSample, current: CpuSample) -> f32 {
        let total = current.total.saturating_sub(previous.total);
        if total == 0 {
            return 0.0;
        }
        let idle = current.idle.saturating_sub(previous.idle);
        ((total.saturating_sub(idle)) as f32 / total as f32 * 100.0).clamp(0.0, 100.0)
    }
    pub fn cpu_cores(&self) -> u32 {
        self.read("proc/stat")
            .map(|v| {
                v.lines()
                    .filter(|line| {
                        line.strip_prefix("cpu").is_some_and(|tail| {
                            tail.as_bytes().first().is_some_and(u8::is_ascii_digit)
                        })
                    })
                    .count() as u32
            })
            .filter(|v| *v > 0)
            .unwrap_or(1)
    }
    pub fn boot_id(&self) -> String {
        self.read("proc/sys/kernel/random/boot_id")
            .map(|v| v.trim().to_owned())
            .unwrap_or_else(|| "unknown".into())
    }
    pub fn os(&self) -> String {
        self.read("etc/os-release")
            .and_then(|v| {
                v.lines().find_map(|line| {
                    line.strip_prefix("PRETTY_NAME=")
                        .map(|value| value.trim_matches('"').to_owned())
                })
            })
            .unwrap_or_else(|| "Linux".into())
    }
    pub fn kernel(&self) -> String {
        self.read("proc/sys/kernel/osrelease")
            .map(|v| v.trim().to_owned())
            .unwrap_or_default()
    }

    pub fn resources(&self, previous: CpuSample, current: CpuSample) -> ResourceSample {
        let mut sample = ResourceSample {
            cpu_pct: Self::cpu_pct(previous, current),
            cpu_cores: self.cpu_cores(),
            ..Default::default()
        };
        if let Some(load) = self.read("proc/loadavg") {
            sample.load1 = load
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
        }
        if let Some(memory) = self.read("proc/meminfo") {
            let kb = |key: &str| {
                memory
                    .lines()
                    .find(|line| line.starts_with(key))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0)
                    * 1024
            };
            sample.mem_total = kb("MemTotal:");
            sample.mem_used = sample.mem_total.saturating_sub(kb("MemAvailable:"));
            sample.swap_total = kb("SwapTotal:");
            sample.swap_used = sample.swap_total.saturating_sub(kb("SwapFree:"));
        }
        sample.uptime = self
            .read("proc/uptime")
            .and_then(|v| v.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap_or(0.0) as u64;
        sample.psi_cpu = self.psi("proc/pressure/cpu");
        sample.psi_mem = self.psi("proc/pressure/memory");
        sample.psi_io = self.psi("proc/pressure/io");
        (sample.tcp, sample.udp) = self.socket_counts();
        if self.root == Path::new("/") {
            (sample.disk_total, sample.disk_used) = self.disks();
        }
        sample
    }

    fn psi(&self, path: &str) -> Option<f32> {
        self.read(path)?
            .lines()
            .find(|line| line.starts_with("some "))?
            .split_whitespace()
            .find_map(|part| part.strip_prefix("avg10=")?.parse().ok())
    }
    fn socket_counts(&self) -> (u32, u32) {
        let (mut tcp, mut udp) = (0u32, 0u32);
        for path in ["proc/net/sockstat", "proc/net/sockstat6"] {
            if let Some(value) = self.read(path) {
                for line in value.lines() {
                    let fields: Vec<_> = line.split_whitespace().collect();
                    if fields.len() >= 3 && fields[1] == "inuse" {
                        let count = fields[2].parse::<u32>().unwrap_or(0);
                        if fields[0].starts_with("TCP") {
                            tcp = tcp.saturating_add(count)
                        } else if fields[0].starts_with("UDP") {
                            udp = udp.saturating_add(count)
                        }
                    }
                }
            }
        }
        (tcp, udp)
    }
    fn disks(&self) -> (u64, u64) {
        let Some(mounts) = self.read("proc/mounts") else {
            return (0, 0);
        };
        let mut devices = HashSet::new();
        let (mut total, mut used) = (0u64, 0u64);
        for line in mounts.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() < 3 || !fields[0].starts_with('/') || !devices.insert(fields[0]) {
                continue;
            }
            if let Some((size, free)) = syscall::statfs_bytes(fields[1]) {
                total = total.saturating_add(size);
                used = used.saturating_add(size.saturating_sub(free));
            }
        }
        (total, used)
    }

    pub fn interfaces(
        &self,
        allow: &[String],
        deny: &[String],
        boot_id: &str,
    ) -> Vec<InterfaceCounter> {
        let defaults: self::Set = self.default_interfaces();
        let mut result = Vec::new();
        let Some(value) = self.read("proc/net/dev") else {
            return result;
        };
        for line in value.lines().skip(2) {
            let Some((raw_name, raw_fields)) = line.split_once(':') else {
                continue;
            };
            let name = raw_name.trim();
            let fields: Vec<u64> = raw_fields
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            if fields.len() < 16 {
                continue;
            }
            let selected = if deny.iter().any(|item| item == name) {
                false
            } else if allow.is_empty() {
                defaults.0.contains(name) && !excluded(name)
            } else {
                allow.iter().any(|item| item == name)
            };
            let ifindex = self
                .read(&format!("sys/class/net/{name}/ifindex"))
                .map(|v| v.trim().to_owned())
                .unwrap_or_default();
            let mac = self
                .read(&format!("sys/class/net/{name}/address"))
                .map(|v| v.trim().to_owned())
                .unwrap_or_default();
            let identity = if ifindex.is_empty() && mac.is_empty() {
                format!("{boot_id}:{name}")
            } else {
                format!("{boot_id}:{ifindex}:{mac}")
            };
            result.push(InterfaceCounter {
                name: name.into(),
                identity,
                rx_bytes: fields[0],
                rx_packets: fields[1],
                rx_errors: fields[2],
                rx_drops: fields[3],
                tx_bytes: fields[8],
                tx_packets: fields[9],
                tx_errors: fields[10],
                tx_drops: fields[11],
                selected,
            });
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }
    fn default_interfaces(&self) -> Set {
        let mut set = HashSet::new();
        if let Some(routes) = self.read("proc/net/route") {
            for line in routes.lines().skip(1) {
                let fields: Vec<_> = line.split_whitespace().collect();
                if fields.len() > 3
                    && fields[1] == "00000000"
                    && u32::from_str_radix(fields[3], 16).is_ok_and(|flags| flags & 0x2 != 0)
                {
                    set.insert(fields[0].to_owned());
                }
            }
        }
        Set(set)
    }

    pub fn processes(&self) -> Vec<ProcessSummary> {
        let mut output = Vec::new();
        let Ok(entries) = fs::read_dir(self.path("proc")) else {
            return output;
        };
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
                continue;
            };
            let Ok(pid) = name.parse::<u32>() else {
                continue;
            };
            let base = format!("proc/{pid}");
            let Some(stat) = self.read(&format!("{base}/stat")) else {
                continue;
            };
            let Some(close) = stat.rfind(')') else {
                continue;
            };
            let fields: Vec<_> = stat[close + 1..].split_whitespace().collect();
            if fields.len() < 22 {
                continue;
            }
            let status = self.read(&format!("{base}/status")).unwrap_or_default();
            let uid = status
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let rss = status
                .lines()
                .find(|line| line.starts_with("VmRSS:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024;
            let exe = fs::read_link(self.path(&format!("{base}/exe")))
                .ok()
                .map(|v| v.to_string_lossy().into_owned())
                .unwrap_or_else(|| stat[stat.find('(').unwrap_or(0) + 1..close].to_owned());
            let deleted = exe.ends_with(" (deleted)");
            let clean = exe.trim_end_matches(" (deleted)").to_owned();
            let suspicious = ["/tmp/", "/var/tmp/", "/dev/shm/"]
                .iter()
                .any(|prefix| clean.starts_with(prefix));
            let cgroup = self.read(&format!("{base}/cgroup")).and_then(|value| {
                value
                    .lines()
                    .next()
                    .and_then(|line| line.rsplit(':').next())
                    .map(ToOwned::to_owned)
            });
            let unit = cgroup.as_ref().and_then(|value| {
                value
                    .split('/')
                    .find(|part| part.ends_with(".service"))
                    .map(ToOwned::to_owned)
            });
            output.push(ProcessSummary {
                pid,
                ppid: fields[1].parse().unwrap_or(0),
                uid,
                state: fields[0].into(),
                executable: clean,
                cpu_ticks: fields[11]
                    .parse::<u64>()
                    .unwrap_or(0)
                    .saturating_add(fields[12].parse().unwrap_or(0)),
                rss_bytes: rss,
                virtual_bytes: fields[20].parse().unwrap_or(0),
                started_ticks: fields[19].parse().unwrap_or(0),
                cgroup,
                systemd_unit: unit,
                listening_sockets: 0,
                deleted_executable: deleted,
                suspicious_writable_path: suspicious,
                package_ownership: PackageOwnership::Unknown,
            });
        }
        output.sort_by(|a, b| {
            b.suspicious_writable_path
                .cmp(&a.suspicious_writable_path)
                .then_with(|| b.deleted_executable.cmp(&a.deleted_executable))
                .then_with(|| b.cpu_ticks.cmp(&a.cpu_ticks))
                .then_with(|| a.pid.cmp(&b.pid))
        });
        output.truncate(MAX_PROCESSES);
        output
    }

    pub fn listeners(&self) -> Vec<ListenerSummary> {
        let mut output = Vec::new();
        for (protocol, path, listen_state) in [
            ("tcp", "proc/net/tcp", "0A"),
            ("tcp6", "proc/net/tcp6", "0A"),
            ("udp", "proc/net/udp", "07"),
            ("udp6", "proc/net/udp6", "07"),
        ] {
            let Some(value) = self.read(path) else {
                continue;
            };
            for line in value.lines().skip(1) {
                let fields: Vec<_> = line.split_whitespace().collect();
                if fields.len() < 10 || fields[3] != listen_state {
                    continue;
                }
                let Some((address, port)) = fields[1].split_once(':') else {
                    continue;
                };
                let Ok(port) = u16::from_str_radix(port, 16) else {
                    continue;
                };
                output.push(ListenerSummary {
                    protocol: protocol.into(),
                    local_address: address.into(),
                    port,
                    uid: fields.get(7).and_then(|v| v.parse().ok()),
                    inode: fields.get(9).and_then(|v| v.parse().ok()),
                });
            }
        }
        output.sort_by(|a, b| {
            a.protocol
                .cmp(&b.protocol)
                .then(a.port.cmp(&b.port))
                .then(a.local_address.cmp(&b.local_address))
        });
        output.dedup_by(|a, b| {
            a.protocol == b.protocol && a.port == b.port && a.local_address == b.local_address
        });
        output.truncate(MAX_LISTENERS);
        output
    }

    pub fn coverage(&self) -> Vec<CoverageItem> {
        vec![
            self.coverage_item("resources", "proc/stat"),
            self.coverage_item("traffic", "proc/net/dev"),
            self.coverage_item("processes", "proc"),
            self.coverage_item("listeners", "proc/net/tcp"),
            self.coverage_item("psi", "proc/pressure/cpu"),
        ]
    }
    fn coverage_item(&self, name: &str, path: &str) -> CoverageItem {
        let ok = self.path(path).exists();
        CoverageItem {
            collector: name.into(),
            status: if ok {
                Confidence::High
            } else {
                Confidence::Unsupported
            },
            detail: if ok {
                "available".into()
            } else {
                format!("{} is unavailable", path)
            },
        }
    }
}

struct Set(HashSet<String>);
fn excluded(name: &str) -> bool {
    [
        "lo",
        "docker",
        "veth",
        "br-",
        "virbr",
        "podman",
        "kube",
        "cni",
        "flannel",
        "tun",
        "tap",
        "wg",
        "tailscale",
        "zt",
        "warp",
        "dummy",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/basic")
    }
    #[test]
    fn fixture_root_collects_default_interface_and_never_reads_environment() {
        let c = Collector::new(fixture());
        let interfaces = c.interfaces(&[], &[], "boot");
        assert_eq!(
            interfaces
                .iter()
                .filter(|v| v.selected)
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>(),
            vec!["eth0"]
        );
        let processes = c.processes();
        assert!(processes
            .iter()
            .all(|p| !p.executable.contains("SECRET_SENTINEL")));
        assert!(processes.iter().any(|p| p.suspicious_writable_path));
    }
}
