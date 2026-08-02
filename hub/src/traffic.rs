//! Audited manual-seed traffic accounting and calendar-cycle rollover.

use crate::db::{audit, DbError};
use chrono::{Datelike, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrafficUsage {
    pub cycle_id: i64,
    pub cycle_start: i64,
    pub cycle_end: i64,
    pub seed_bytes: i64,
    pub has_manual_seed: bool,
    pub seed_effective_at: Option<i64>,
    pub seed_checkpoint_at: Option<i64>,
    pub seed_note: Option<String>,
    pub observed_rx_bytes: i64,
    pub observed_tx_bytes: i64,
    pub observed_bytes: i64,
    pub adjustment_bytes: i64,
    pub total_bytes: i64,
    pub limit_bytes: Option<i64>,
    pub confidence: String,
    pub checkpoint_at: i64,
    pub agent_observed_total_bytes: i64,
    pub observation_start_at: i64,
    pub projected_bytes: Option<i64>,
    pub selected_interfaces: serde_json::Value,
    pub timezone: String,
    pub anchor_day: u32,
    pub anchor_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycleRuleInput {
    pub timezone: String,
    pub anchor_day: u32,
    pub anchor_time: String,
    pub selected_interfaces: Vec<String>,
    pub excluded_interfaces: Vec<String>,
    pub traffic_limit_bytes: Option<u64>,
}

pub fn cycle_bounds(
    at: i64,
    timezone: &str,
    anchor_day: u32,
    anchor_time: &str,
) -> Result<(i64, i64), DbError> {
    let tz: Tz = timezone
        .parse()
        .map_err(|_| DbError::Invalid("invalid IANA timezone"))?;
    let time = NaiveTime::parse_from_str(anchor_time, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(anchor_time, "%H:%M"))
        .map_err(|_| DbError::Invalid("anchor_time must be HH:MM or HH:MM:SS"))?;
    let instant = Utc
        .timestamp_opt(at, 0)
        .single()
        .ok_or(DbError::Invalid("invalid timestamp"))?;
    let local = instant.with_timezone(&tz);
    let current = local_anchor(tz, local.year(), local.month(), anchor_day, time)?;
    let (start_y, start_m) = if current <= instant {
        (local.year(), local.month())
    } else {
        previous_month(local.year(), local.month())
    };
    let (next_y, next_m) = next_month(start_y, start_m);
    Ok((
        local_anchor(tz, start_y, start_m, anchor_day, time)?.timestamp(),
        local_anchor(tz, next_y, next_m, anchor_day, time)?.timestamp(),
    ))
}

fn local_anchor(
    tz: Tz,
    year: i32,
    month: u32,
    day: u32,
    time: NaiveTime,
) -> Result<chrono::DateTime<Utc>, DbError> {
    let day = day.clamp(1, 31).min(days_in_month(year, month));
    let mut naive = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or(DbError::Invalid("invalid cycle date"))?
        .and_time(time);
    // A DST gap has no local instant. Advancing to the first valid minute is
    // deterministic and recorded as calendar normalization, not hidden drift.
    for _ in 0..=180 {
        match tz.from_local_datetime(&naive) {
            LocalResult::Single(value) => return Ok(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(first, _) => return Ok(first.with_timezone(&Utc)),
            LocalResult::None => naive += chrono::Duration::minutes(1),
        }
    }
    Err(DbError::Invalid(
        "cycle boundary falls in an unsupported timezone gap",
    ))
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = next_month(year, month);
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
        .day()
}
fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}
fn previous_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

pub fn ensure_cycle_tx(
    conn: &Connection,
    server_id: &str,
    at: i64,
    rx: i64,
    tx_bytes: i64,
) -> Result<(), DbError> {
    let rule:Option<(String,i64,String)>=conn.query_row(
        "SELECT timezone,anchor_day,anchor_time FROM billing_cycle_rule WHERE server_id=?1 AND enabled=1",
        [server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).optional()?;
    let (timezone, day, time) = match rule {
        Some(value) => value,
        None => {
            conn.execute("INSERT INTO billing_cycle_rule(server_id,timezone,anchor_day,anchor_time,interface_policy_json,traffic_limit_bytes,enabled,version,updated_at,updated_by) VALUES(?1,'UTC',1,'00:00','{\"mode\":\"auto\"}',NULL,1,1,?2,'system')",params![server_id,at])?;
            ("UTC".to_string(), 1, "00:00".to_string())
        }
    };
    let (start, end) = cycle_bounds(at, &timezone, day as u32, &time)?;
    let open:Option<(i64,i64,i64,String)>=conn.query_row(
        "SELECT id,start_at,end_at,confidence FROM billing_cycle_instance WHERE server_id=?1 AND state='open' ORDER BY start_at DESC LIMIT 1",
        [server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).optional()?;
    if let Some((_, old_start, _, confidence)) = &open {
        if *old_start == start {
            // A timer may have rolled a cycle while the Agent was offline. A
            // later checkpoint that spans the boundary lets us refine both
            // adjacent cycle checkpoints, but the split remains explicitly
            // estimated because Linux supplies no boundary-time counter.
            if confidence != "high" {
                reconcile_boundaries(conn, server_id, at)?;
            }
            return Ok(());
        }
    }
    if open
        .as_ref()
        .is_some_and(|(_, old_start, _, _)| *old_start > start)
    {
        return Ok(());
    }
    if let Some((cycle_id, _, old_end, _)) = open {
        let before:Option<(i64,i64,i64)>=conn.query_row("SELECT observed_rx,observed_tx,checkpoint_at FROM traffic_observed_checkpoint WHERE server_id=?1 AND checkpoint_at<=?2 ORDER BY checkpoint_at DESC LIMIT 1",params![server_id,old_end],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let (boundary_rx, boundary_tx, state, confidence) =
            if let Some((brx, btx, before_at)) = before {
                if before_at == old_end {
                    (brx, btx, "closed", "high")
                } else if at > before_at {
                    let numerator = (old_end - before_at).clamp(0, at - before_at) as i128;
                    let denominator = (at - before_at) as i128;
                    let erx = brx as i128 + (rx - brx) as i128 * numerator / denominator;
                    let etx = btx as i128 + (tx_bytes - btx) as i128 * numerator / denominator;
                    (erx as i64, etx as i64, "estimated", "estimated")
                } else {
                    (brx, btx, "estimated", "estimated")
                }
            } else {
                (rx, tx_bytes, "estimated", "estimated")
            };
        conn.execute("UPDATE billing_cycle_instance SET ending_observed_rx=?2,ending_observed_tx=?3,state=?4,confidence=?5,closed_at=?6 WHERE id=?1",params![cycle_id,boundary_rx,boundary_tx,state,confidence,at])?;
        conn.execute("INSERT OR IGNORE INTO billing_cycle_instance(server_id,start_at,end_at,starting_observed_rx,starting_observed_tx,state,confidence,created_at) VALUES(?1,?2,?3,?4,?5,'open',?6,?7)",params![server_id,start,end,boundary_rx,boundary_tx,confidence,at])?;
        conn.execute("INSERT INTO events(server_id,occurred_at,category,severity,summary,evidence) VALUES(?1,?2,'traffic','info','Billing cycle rolled over',?3)",params![server_id,at,format!("boundary={start}; confidence={confidence}")])?;
    } else {
        conn.execute("INSERT OR IGNORE INTO billing_cycle_instance(server_id,start_at,end_at,starting_observed_rx,starting_observed_tx,state,confidence,created_at) VALUES(?1,?2,?3,?4,?5,'open','partial',?6)",params![server_id,start,end,rx,tx_bytes,at])?;
    }
    Ok(())
}

fn reconcile_boundaries(conn: &Connection, server_id: &str, through: i64) -> Result<(), DbError> {
    let boundaries = {
        let mut statement = conn.prepare(
            "SELECT id,start_at FROM billing_cycle_instance WHERE server_id=?1 AND confidence!='high' AND start_at<=?2 ORDER BY start_at",
        )?;
        let values = statement
            .query_map(params![server_id, through], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        values
    };
    for (cycle_id, boundary) in boundaries {
        reconcile_boundary(conn, server_id, cycle_id, boundary)?;
    }
    Ok(())
}

fn reconcile_boundary(
    conn: &Connection,
    server_id: &str,
    current_cycle_id: i64,
    boundary: i64,
) -> Result<(), DbError> {
    let before: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT observed_rx,observed_tx,checkpoint_at FROM traffic_observed_checkpoint WHERE server_id=?1 AND checkpoint_at<=?2 ORDER BY checkpoint_at DESC LIMIT 1",
            params![server_id, boundary],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let after: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT observed_rx,observed_tx,checkpoint_at FROM traffic_observed_checkpoint WHERE server_id=?1 AND checkpoint_at>=?2 ORDER BY checkpoint_at ASC LIMIT 1",
            params![server_id, boundary],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (Some((brx, btx, before_at)), Some((arx, atx, after_at))) = (before, after) else {
        return Ok(());
    };
    if after_at < before_at {
        return Ok(());
    }
    let (boundary_rx, boundary_tx, confidence) = if after_at == before_at {
        (brx, btx, "high")
    } else {
        let numerator = (boundary - before_at).clamp(0, after_at - before_at) as i128;
        let denominator = (after_at - before_at) as i128;
        (
            (brx as i128 + (arx - brx) as i128 * numerator / denominator) as i64,
            (btx as i128 + (atx - btx) as i128 * numerator / denominator) as i64,
            "estimated",
        )
    };
    conn.execute(
        "UPDATE billing_cycle_instance SET starting_observed_rx=?2,starting_observed_tx=?3,confidence=?4 WHERE id=?1",
        params![current_cycle_id, boundary_rx, boundary_tx, confidence],
    )?;
    conn.execute(
        "UPDATE billing_cycle_instance SET ending_observed_rx=?2,ending_observed_tx=?3,state=?4,confidence=?5 WHERE server_id=?1 AND end_at=?6 AND id!=?7",
        params![server_id, boundary_rx, boundary_tx, if confidence == "high" { "closed" } else { "estimated" }, confidence, boundary, current_cycle_id],
    )?;
    Ok(())
}

impl crate::db::Database {
    /// Calendar rollover runs independently of reports. When a boundary falls
    /// inside an Agent outage the provisional split is estimated and is
    /// refined by `ensure_cycle_tx` after the next checkpoint arrives.
    pub fn rollover_due_cycles(&self, now: i64) -> Result<usize, DbError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let mut count = 0usize;
        loop {
            let due = {
                let mut stmt = tx.prepare(
                    "SELECT c.server_id,c.end_at,p.observed_rx,p.observed_tx FROM billing_cycle_instance c JOIN traffic_observed_checkpoint p ON p.server_id=c.server_id WHERE c.state='open' AND c.end_at<=?1 AND p.checkpoint_at=(SELECT MAX(p2.checkpoint_at) FROM traffic_observed_checkpoint p2 WHERE p2.server_id=c.server_id)",
                )?;
                let values = stmt
                    .query_map([now], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                values
            };
            if due.is_empty() {
                break;
            }
            for (server, boundary, rx, tx_bytes) in due {
                ensure_cycle_tx(&tx, &server, boundary.saturating_add(1), rx, tx_bytes)?;
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    pub fn set_cycle_rule(
        &self,
        server_id: &str,
        input: &CycleRuleInput,
        now: i64,
        operator: &str,
    ) -> Result<(), DbError> {
        let _: Tz = input
            .timezone
            .parse()
            .map_err(|_| DbError::Invalid("invalid IANA timezone"))?;
        cycle_bounds(now, &input.timezone, input.anchor_day, &input.anchor_time)?;
        if input.selected_interfaces.len() > 32 || input.excluded_interfaces.len() > 32 {
            return Err(DbError::Invalid("too many interfaces"));
        }
        let limit = match input.traffic_limit_bytes {
            Some(v) => Some(i64::try_from(v).map_err(|_| DbError::Invalid("limit too large"))?),
            None => None,
        };
        let policy=serde_json::json!({"mode":if input.selected_interfaces.is_empty(){"auto"}else{"manual"},"selected":input.selected_interfaces,"excluded":input.excluded_interfaces}).to_string();
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let locked: i64 = tx.query_row(
            "SELECT (SELECT COUNT(*) FROM billing_cycle_instance WHERE server_id=?1 AND state!='open') + (SELECT COUNT(*) FROM traffic_seed s JOIN billing_cycle_instance c ON c.id=s.cycle_id WHERE c.server_id=?1) + (SELECT COUNT(*) FROM traffic_adjustment a JOIN billing_cycle_instance c ON c.id=a.cycle_id WHERE c.server_id=?1)",
            [server_id],
            |row| row.get(0),
        )?;
        if locked > 0 {
            return Err(DbError::Conflict(
                "cycle rules are immutable after seeded or closed history exists; create a new server accounting epoch",
            ));
        }
        tx.execute("INSERT INTO billing_cycle_rule(server_id,timezone,anchor_day,anchor_time,interface_policy_json,traffic_limit_bytes,enabled,version,updated_at,updated_by) VALUES(?1,?2,?3,?4,?5,?6,1,1,?7,?8) ON CONFLICT(server_id) DO UPDATE SET timezone=excluded.timezone,anchor_day=excluded.anchor_day,anchor_time=excluded.anchor_time,interface_policy_json=excluded.interface_policy_json,traffic_limit_bytes=excluded.traffic_limit_bytes,version=billing_cycle_rule.version+1,updated_at=excluded.updated_at,updated_by=excluded.updated_by",params![server_id,input.timezone,input.anchor_day,input.anchor_time,policy,limit,now,operator])?;
        tx.execute(
            "DELETE FROM billing_cycle_instance WHERE server_id=?1 AND state='open'",
            [server_id],
        )?;
        let checkpoint: Option<(i64, i64, i64)> = tx
            .query_row(
                "SELECT observed_rx,observed_tx,checkpoint_at FROM traffic_observed_checkpoint WHERE server_id=?1 ORDER BY checkpoint_at DESC LIMIT 1",
                [server_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        if let Some((rx, tx_bytes, at)) = checkpoint {
            ensure_cycle_tx(&tx, server_id, at, rx, tx_bytes)?;
        }
        audit(
            &tx,
            now,
            operator,
            "traffic.rule.update",
            Some(server_id),
            &serde_json::to_string(input).map_err(|_| DbError::Invalid("rule encoding"))?,
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_traffic_seed(
        &self,
        server_id: &str,
        combined_bytes: u64,
        effective_at: i64,
        note: &str,
        now: i64,
        operator: &str,
    ) -> Result<TrafficUsage, DbError> {
        if note.len() > 500 {
            return Err(DbError::Invalid("note is too long"));
        }
        let bytes =
            i64::try_from(combined_bytes).map_err(|_| DbError::Invalid("seed too large"))?;
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let checkpoint:(i64,i64,i64,String)=tx.query_row("SELECT observed_rx,observed_tx,checkpoint_at,confidence FROM traffic_observed_checkpoint WHERE server_id=?1 AND checkpoint_at<=?2 ORDER BY checkpoint_at DESC LIMIT 1",params![server_id,effective_at],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.ok_or(DbError::Invalid("no reliable checkpoint at or before effective timestamp"))?;
        if checkpoint.2 != effective_at {
            return Err(DbError::Invalid(
                "manual seed must use an exact Agent checkpoint",
            ));
        }
        ensure_cycle_tx(&tx, server_id, checkpoint.2, checkpoint.0, checkpoint.1)?;
        let cycle_id:i64=tx.query_row("SELECT id FROM billing_cycle_instance WHERE server_id=?1 AND start_at<=?2 AND end_at>?2 ORDER BY start_at DESC LIMIT 1",params![server_id,effective_at],|r|r.get(0))?;
        if tx
            .query_row(
                "SELECT 1 FROM traffic_seed WHERE cycle_id=?1 AND active_primary=1",
                [cycle_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            return Err(DbError::Conflict(
                "a primary seed already exists; use an audited adjustment",
            ));
        }
        tx.execute("INSERT INTO traffic_seed(cycle_id,rx_bytes,tx_bytes,combined_bytes,effective_at,checkpoint_at,observed_rx_at_seed,observed_tx_at_seed,operator,note,created_at) VALUES(?1,0,0,?2,?3,?4,?5,?6,?7,?8,?9)",params![cycle_id,bytes,effective_at,checkpoint.2,checkpoint.0,checkpoint.1,operator,note,now])?;
        tx.execute(
            "UPDATE billing_cycle_instance SET confidence=?2 WHERE id=?1",
            params![cycle_id, checkpoint.3],
        )?;
        audit(&tx,now,operator,"traffic.seed.create",Some(server_id),&serde_json::json!({"cycle_id":cycle_id,"bytes":bytes,"effective_at":effective_at,"checkpoint_at":checkpoint.2,"note":note}).to_string())?;
        tx.commit()?;
        self.traffic_usage(server_id)
    }

    pub fn add_traffic_adjustment(
        &self,
        server_id: &str,
        signed_bytes: i64,
        effective_at: i64,
        reason: &str,
        now: i64,
        operator: &str,
    ) -> Result<TrafficUsage, DbError> {
        if reason.trim().len() < 3 || reason.len() > 500 {
            return Err(DbError::Invalid("a concise adjustment reason is required"));
        }
        if effective_at > now {
            return Err(DbError::Invalid(
                "adjustment effective time cannot be future",
            ));
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let usage = traffic_usage_conn(&tx, server_id)?;
        if effective_at < usage.cycle_start || effective_at >= usage.cycle_end {
            return Err(DbError::Invalid(
                "adjustment effective time is outside the current open cycle",
            ));
        }
        if usage.total_bytes.saturating_add(signed_bytes) < 0 {
            return Err(DbError::Invalid("adjustment would make usage negative"));
        }
        tx.execute("INSERT INTO traffic_adjustment(cycle_id,signed_bytes,effective_at,reason,operator,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![usage.cycle_id,signed_bytes,effective_at,reason,operator,now])?;
        audit(
            &tx,
            now,
            operator,
            "traffic.adjustment.create",
            Some(server_id),
            &serde_json::json!({"cycle_id":usage.cycle_id,"bytes":signed_bytes,"reason":reason})
                .to_string(),
        )?;
        tx.commit()?;
        self.traffic_usage(server_id)
    }

    pub fn traffic_usage(&self, server_id: &str) -> Result<TrafficUsage, DbError> {
        traffic_usage_conn(&self.connection()?, server_id)
    }
}

fn traffic_usage_conn(conn: &Connection, server_id: &str) -> Result<TrafficUsage, DbError> {
    let (cycle_id,start,end,start_rx,start_tx,confidence):(i64,i64,i64,i64,i64,String)=conn.query_row("SELECT id,start_at,end_at,starting_observed_rx,starting_observed_tx,confidence FROM billing_cycle_instance WHERE server_id=?1 AND state='open' ORDER BY start_at DESC LIMIT 1",[server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?.ok_or(DbError::NotFound)?;
    let (current_rx,current_tx,checkpoint_at):(i64,i64,i64)=conn.query_row("SELECT observed_rx,observed_tx,checkpoint_at FROM traffic_observed_checkpoint WHERE server_id=?1 ORDER BY checkpoint_at DESC LIMIT 1",[server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
    let seed:Option<(i64,i64,i64,i64,i64,String)>=conn.query_row("SELECT combined_bytes,observed_rx_at_seed,observed_tx_at_seed,effective_at,checkpoint_at,note FROM traffic_seed WHERE cycle_id=?1 AND active_primary=1",[cycle_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
    let has_manual_seed = seed.is_some();
    let seed_effective_at = seed.as_ref().map(|value| value.3);
    let seed_checkpoint_at = seed.as_ref().map(|value| value.4);
    let seed_note = seed.as_ref().map(|value| value.5.clone());
    let (seed_bytes, base_rx, base_tx) = seed
        .map(|value| (value.0, value.1, value.2))
        .unwrap_or((0, start_rx, start_tx));
    let observed_rx = current_rx.saturating_sub(base_rx);
    let observed_tx = current_tx.saturating_sub(base_tx);
    let observed = observed_rx.saturating_add(observed_tx);
    let adjustments: i64 = conn.query_row(
        "SELECT COALESCE(SUM(signed_bytes),0) FROM traffic_adjustment WHERE cycle_id=?1",
        [cycle_id],
        |r| r.get(0),
    )?;
    let (limit,policy,timezone,anchor_day,anchor_time):(Option<i64>,String,String,i64,String)=conn.query_row("SELECT traffic_limit_bytes,interface_policy_json,timezone,anchor_day,anchor_time FROM billing_cycle_rule WHERE server_id=?1",[server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
    let observation_start_at = seed_checkpoint_at.unwrap_or_else(|| {
        conn.query_row(
            "SELECT MIN(checkpoint_at) FROM traffic_observed_checkpoint WHERE server_id=?1 AND checkpoint_at>=?2",
            params![server_id, start],
            |row| row.get::<_, Option<i64>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or(checkpoint_at)
    });
    let total = seed_bytes
        .saturating_add(observed)
        .saturating_add(adjustments)
        .max(0);
    let elapsed = checkpoint_at.saturating_sub(observation_start_at);
    let remaining = end.saturating_sub(checkpoint_at).max(0);
    let projected_bytes = (elapsed >= 300).then(|| {
        let future = (observed as i128)
            .saturating_mul(remaining as i128)
            .checked_div(elapsed as i128)
            .unwrap_or(0);
        total.saturating_add(i64::try_from(future).unwrap_or(i64::MAX))
    });
    Ok(TrafficUsage {
        cycle_id,
        cycle_start: start,
        cycle_end: end,
        seed_bytes,
        has_manual_seed,
        seed_effective_at,
        seed_checkpoint_at,
        seed_note,
        observed_rx_bytes: observed_rx,
        observed_tx_bytes: observed_tx,
        observed_bytes: observed,
        adjustment_bytes: adjustments,
        total_bytes: total,
        limit_bytes: limit,
        confidence,
        checkpoint_at,
        agent_observed_total_bytes: current_rx.saturating_add(current_tx),
        observation_start_at,
        projected_bytes,
        selected_interfaces: serde_json::from_str(&policy)
            .unwrap_or_else(|_| serde_json::json!({"mode":"unknown"})),
        timezone,
        anchor_day: u32::try_from(anchor_day).unwrap_or(1),
        anchor_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    fn db() -> (Database, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "parade-traffic-{}.sqlite",
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        (Database::open(&path).unwrap(), path)
    }
    fn checkpoint(db: &Database, server: &str, rx: i64, tx: i64, at: i64, seq: i64) {
        let conn = db.connection().unwrap();
        conn.execute("INSERT INTO traffic_observed_checkpoint(server_id,observed_rx,observed_tx,checkpoint_at,agent_sequence,confidence,last_boot_id) VALUES(?1,?2,?3,?4,?5,'high','boot')",params![server,rx,tx,at,seq]).unwrap();
        ensure_cycle_tx(&conn, server, at, rx, tx).unwrap();
    }
    #[test]
    fn seed_plus_observed_and_adjustments_is_transparent_and_durable() {
        let (db, path) = db();
        db.create_server("s", "S", "", 1, "admin").unwrap();
        checkpoint(&db, "s", 1_000, 2_000, 1_700_000_000, 1);
        db.add_traffic_seed(
            "s",
            100 * 1024 * 1024 * 1024,
            1_700_000_000,
            "provider dashboard",
            1_700_000_001,
            "admin",
        )
        .unwrap();
        checkpoint(
            &db,
            "s",
            1_000 + 2 * 1024 * 1024 * 1024,
            2_000 + 3 * 1024 * 1024 * 1024,
            1_700_000_300,
            2,
        );
        let usage = db
            .add_traffic_adjustment(
                "s",
                -1024 * 1024 * 1024,
                1_700_000_300,
                "provider correction",
                1_700_000_301,
                "admin",
            )
            .unwrap();
        assert_eq!(usage.seed_bytes, 100 * 1024 * 1024 * 1024);
        assert_eq!(usage.observed_bytes, 5 * 1024 * 1024 * 1024);
        assert_eq!(usage.total_bytes, 104 * 1024 * 1024 * 1024);
        drop(db);
        let reopened = Database::open(&path).unwrap();
        assert_eq!(reopened.traffic_usage("s").unwrap(), usage);
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn day_31_clamps_and_dst_is_deterministic() {
        let feb = Utc
            .with_ymd_and_hms(2027, 2, 28, 12, 0, 0)
            .unwrap()
            .timestamp();
        let (start, end) = cycle_bounds(feb, "UTC", 31, "00:00").unwrap();
        assert_eq!(
            Utc.timestamp_opt(start, 0).unwrap().date_naive(),
            NaiveDate::from_ymd_opt(2027, 2, 28).unwrap()
        );
        assert_eq!(
            Utc.timestamp_opt(end, 0).unwrap().date_naive(),
            NaiveDate::from_ymd_opt(2027, 3, 31).unwrap()
        );
        let gap = Utc
            .with_ymd_and_hms(2026, 3, 15, 12, 0, 0)
            .unwrap()
            .timestamp();
        assert!(cycle_bounds(gap, "America/New_York", 8, "02:30").is_ok());
    }

    #[test]
    fn scheduled_rollover_is_persistent_and_marks_unsplittable_interval_estimated() {
        let (db, path) = db();
        db.create_server("s", "S", "", 1, "admin").unwrap();
        let before = Utc
            .with_ymd_and_hms(2027, 1, 31, 23, 55, 0)
            .unwrap()
            .timestamp();
        let boundary = Utc
            .with_ymd_and_hms(2027, 2, 1, 0, 0, 0)
            .unwrap()
            .timestamp();
        checkpoint(&db, "s", 500, 500, before, 1);
        assert_eq!(db.rollover_due_cycles(boundary + 60).unwrap(), 1);
        checkpoint(&db, "s", 1_000, 1_000, boundary + 300, 2);

        let conn = db.connection().unwrap();
        let previous: (String, String, i64, i64) = conn
            .query_row(
                "SELECT state,confidence,ending_observed_rx,ending_observed_tx FROM billing_cycle_instance WHERE server_id='s' AND end_at=?1",
                [boundary],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(previous, ("estimated".into(), "estimated".into(), 750, 750));
        let usage = db.traffic_usage("s").unwrap();
        assert_eq!(usage.observed_bytes, 500);
        assert_eq!(usage.confidence, "estimated");
        drop(conn);
        drop(db);
        let reopened = Database::open(&path).unwrap();
        assert_eq!(reopened.traffic_usage("s").unwrap(), usage);
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn scheduled_rollover_materializes_every_missed_month() {
        let (db, path) = db();
        db.create_server("s", "S", "", 1, "admin").unwrap();
        let january = Utc
            .with_ymd_and_hms(2027, 1, 15, 0, 0, 0)
            .unwrap()
            .timestamp();
        let may = Utc
            .with_ymd_and_hms(2027, 5, 15, 0, 0, 0)
            .unwrap()
            .timestamp();
        checkpoint(&db, "s", 100, 100, january, 1);
        assert_eq!(db.rollover_due_cycles(may).unwrap(), 4);
        let cycles: i64 = db
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM billing_cycle_instance WHERE server_id='s'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cycles, 5);
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
