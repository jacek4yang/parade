//! Small evidence-first finding engine. It never performs remediation.

use crate::db::DbError;
use parade_common::{sha256_hex, SignedReport};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

pub fn evaluate(conn: &Connection, report: &SignedReport, now: i64) -> Result<(), DbError> {
    if report.body.processes.is_some() {
        mark_rule_not_observed(conn, &report.server_id, "PROC_WRITABLE_EXEC")?;
        mark_rule_not_observed(conn, &report.server_id, "PROC_DELETED_EXEC")?;
    }
    for process in report.body.processes.iter().flatten() {
        if process.suspicious_writable_path {
            upsert(conn,&report.server_id,"PROC_WRITABLE_EXEC",&subject_key(&process.executable),1,"review","high",now,&format!("pid={} executable={}",process.pid,process.executable),"Executables running from shared writable locations are easier to replace or stage.","Verify the executable path, owner, package provenance, parent process, and expected deployment workflow locally.","Process paths may be hidden or falsified by a privileged attacker.")?;
        }
        if process.deleted_executable {
            upsert(conn,&report.server_id,"PROC_DELETED_EXEC",&subject_key(&process.executable),1,"review","high",now,&format!("pid={} executable={} (deleted)",process.pid,process.executable),"A running deleted executable can be expected during upgrades, but also obscures the on-disk artifact.","Compare the process start time with maintenance history and inspect the binary through approved local tooling.","Executable links may be unreadable without additional permission.")?;
        }
    }
    mark_rule_not_observed(conn, &report.server_id, "RESOURCE_SUSTAINED_CPU")?;
    if report.body.resources.cpu_avg_pct >= 90.0 && report.body.resources.cpu_max_pct >= 95.0 {
        upsert(conn,&report.server_id,"RESOURCE_SUSTAINED_CPU","resource",1,"review","medium",now,&format!("avg={:.1}% max={:.1}% interval={}..{}",report.body.resources.cpu_avg_pct,report.body.resources.cpu_max_pct,report.body.resources.interval_start,report.body.resources.interval_end),"Sustained CPU can be ordinary workload or a cryptomining-compatible heuristic; it is not proof of compromise.","Compare the privacy-preserving top-process list and expected workload schedule locally.","This rule is a resource heuristic and does not identify intent.")?;
    }
    mark_rule_not_observed(conn, &report.server_id, "TELEMETRY_COVERAGE_REDUCED")?;
    if report
        .body
        .coverage
        .iter()
        .any(|c| !matches!(c.status, parade_common::Confidence::High))
    {
        let evidence = report
            .body
            .coverage
            .iter()
            .filter(|c| !matches!(c.status, parade_common::Confidence::High))
            .map(|c| format!("{}={:?}", c.collector, c.status))
            .collect::<Vec<_>>()
            .join(", ");
        upsert(conn,&report.server_id,"TELEMETRY_COVERAGE_REDUCED","coverage",1,"info","high",now,&evidence,"Reduced collection coverage limits the conclusions Parade can draw.","Check the Agent service user permissions and the collector coverage panel; grant only narrowly scoped read access if needed.","Missing telemetry is reported rather than interpreted as absence of risk.")?;
    }
    let previous:Option<String>=conn.query_row("SELECT payload_json FROM socket_summaries WHERE server_id=?1 AND observed_at<?2 ORDER BY observed_at DESC LIMIT 1",params![report.server_id,report.sent_at],|r|r.get(0)).optional()?;
    if report.body.listeners.is_some() {
        mark_rule_not_observed(conn, &report.server_id, "NET_NEW_LISTENER")?;
    }
    if let Some(previous) = previous {
        let old: Vec<parade_common::ListenerSummary> =
            serde_json::from_str(&previous).unwrap_or_default();
        let old: Set = Set::from(&old);
        for listener in report.body.listeners.iter().flatten() {
            let key = format!(
                "{}:{}:{}",
                listener.protocol, listener.local_address, listener.port
            );
            if !old.0.contains(&key) {
                upsert(conn,&report.server_id,"NET_NEW_LISTENER",&subject_key(&key),1,"review","high",now,&key,"A newly observed listening socket changes the host's exposed attack surface.","Confirm the owning service and intended bind address locally, then compare firewall/provider filtering separately.","Parade observes host sockets; it does not prove Internet reachability.")?;
            }
        }
    }
    Ok(())
}

fn mark_rule_not_observed(conn: &Connection, server: &str, rule: &str) -> Result<(), DbError> {
    // A fresh report provides the complete bounded evidence set for the
    // relevant collector. Retire prior active evidence before reactivating
    // subjects observed in this report. Suppressions remain operator-owned.
    conn.execute(
        "UPDATE security_findings SET state='not_observed' WHERE server_id=?1 AND rule_id=?2 AND state='active'",
        params![server, rule],
    )?;
    Ok(())
}

fn subject_key(subject: &str) -> String {
    sha256_hex(subject.as_bytes())
}

struct Set(HashSet<String>);
impl From<&Vec<parade_common::ListenerSummary>> for Set {
    fn from(value: &Vec<parade_common::ListenerSummary>) -> Self {
        Self(
            value
                .iter()
                .map(|l| format!("{}:{}:{}", l.protocol, l.local_address, l.port))
                .collect(),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn upsert(
    conn: &Connection,
    server: &str,
    rule: &str,
    requested_series: &str,
    version: i64,
    severity: &str,
    confidence: &str,
    now: i64,
    evidence: &str,
    explanation: &str,
    verification: &str,
    caveat: &str,
) -> Result<(), DbError> {
    const MAX_SERIES_PER_RULE: i64 = 32;
    let exists = conn
        .query_row(
            "SELECT 1 FROM security_findings WHERE server_id=?1 AND rule_id=?2 AND rule_version=?3 AND series_key=?4",
            params![server, rule, version, requested_series],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    let series_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM security_findings WHERE server_id=?1 AND rule_id=?2 AND rule_version=?3 AND series_key!='overflow'",
        params![server, rule, version],
        |row| row.get(0),
    )?;
    let series = if exists || series_count < MAX_SERIES_PER_RULE {
        requested_series
    } else {
        "overflow"
    };
    let evidence = if series == "overflow" {
        format!("additional distinct evidence (latest): {evidence}")
    } else {
        evidence.to_owned()
    };
    // Stable subject series preserve distinct evidence while the overflow row
    // caps adversarial/churning subjects at 33 durable rows per server/rule.
    conn.execute("INSERT INTO security_findings(server_id,rule_id,series_key,rule_version,severity,confidence,first_seen,last_seen,evidence,explanation,verification,coverage_caveat) VALUES(?1,?2,?3,?4,?5,?6,?7,?7,?8,?9,?10,?11) ON CONFLICT(server_id,rule_id,rule_version,series_key) DO UPDATE SET severity=excluded.severity,confidence=excluded.confidence,last_seen=excluded.last_seen,occurrence_count=security_findings.occurrence_count+1,evidence=excluded.evidence,explanation=excluded.explanation,verification=excluded.verification,coverage_caveat=excluded.coverage_caveat,state=CASE WHEN security_findings.state='suppressed' AND security_findings.suppression_expires_at>excluded.last_seen THEN 'suppressed' ELSE 'active' END",params![server,rule,series,version,severity,confidence,now,evidence,explanation,verification,caveat])?;
    Ok(())
}
