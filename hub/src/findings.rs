//! Small evidence-first finding engine. It never performs remediation.

use crate::db::DbError;
use parade_common::SignedReport;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;

pub fn evaluate(conn: &Connection, report: &SignedReport, now: i64) -> Result<(), DbError> {
    for process in report.body.processes.iter().flatten() {
        if process.suspicious_writable_path {
            upsert(conn,&report.server_id,"PROC_WRITABLE_EXEC",1,"review","high",now,&format!("pid={} executable={}",process.pid,process.executable),"Executables running from shared writable locations are easier to replace or stage.","Verify the executable path, owner, package provenance, parent process, and expected deployment workflow locally.","Process paths may be hidden or falsified by a privileged attacker.")?;
        }
        if process.deleted_executable {
            upsert(conn,&report.server_id,"PROC_DELETED_EXEC",1,"review","high",now,&format!("pid={} executable={} (deleted)",process.pid,process.executable),"A running deleted executable can be expected during upgrades, but also obscures the on-disk artifact.","Compare the process start time with maintenance history and inspect the binary through approved local tooling.","Executable links may be unreadable without additional permission.")?;
        }
    }
    if report.body.resources.cpu_avg_pct >= 90.0 && report.body.resources.cpu_max_pct >= 95.0 {
        upsert(conn,&report.server_id,"RESOURCE_SUSTAINED_CPU",1,"review","medium",now,&format!("avg={:.1}% max={:.1}% interval={}..{}",report.body.resources.cpu_avg_pct,report.body.resources.cpu_max_pct,report.body.resources.interval_start,report.body.resources.interval_end),"Sustained CPU can be ordinary workload or a cryptomining-compatible heuristic; it is not proof of compromise.","Compare the privacy-preserving top-process list and expected workload schedule locally.","This rule is a resource heuristic and does not identify intent.")?;
    }
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
        upsert(conn,&report.server_id,"TELEMETRY_COVERAGE_REDUCED",1,"info","high",now,&evidence,"Reduced collection coverage limits the conclusions Parade can draw.","Check the Agent service user permissions and the collector coverage panel; grant only narrowly scoped read access if needed.","Missing telemetry is reported rather than interpreted as absence of risk.")?;
    }
    let previous:Option<String>=conn.query_row("SELECT payload_json FROM socket_summaries WHERE server_id=?1 AND observed_at<?2 ORDER BY observed_at DESC LIMIT 1",params![report.server_id,report.sent_at],|r|r.get(0)).optional()?;
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
                upsert(conn,&report.server_id,"NET_NEW_LISTENER",1,"review","high",now,&key,"A newly observed listening socket changes the host's exposed attack surface.","Confirm the owning service and intended bind address locally, then compare firewall/provider filtering separately.","Parade observes host sockets; it does not prove Internet reachability.")?;
            }
        }
    }
    Ok(())
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
    version: i64,
    severity: &str,
    confidence: &str,
    now: i64,
    evidence: &str,
    explanation: &str,
    verification: &str,
    caveat: &str,
) -> Result<(), DbError> {
    conn.execute("INSERT INTO security_findings(server_id,rule_id,rule_version,severity,confidence,first_seen,last_seen,evidence,explanation,verification,coverage_caveat) VALUES(?1,?2,?3,?4,?5,?6,?6,?7,?8,?9,?10) ON CONFLICT(server_id,rule_id,evidence) DO UPDATE SET last_seen=excluded.last_seen,occurrence_count=security_findings.occurrence_count+1,state=CASE WHEN security_findings.state='suppressed' AND security_findings.suppression_expires_at>excluded.last_seen THEN 'suppressed' ELSE 'active' END",params![server,rule,version,severity,confidence,now,evidence,explanation,verification,caveat])?;
    Ok(())
}
