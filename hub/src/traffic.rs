//! Audited manual-seed traffic accounting and calendar-cycle rollover.

use crate::db::{audit, DbError};
use chrono::{Datelike, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use parade_common::InterfaceCounter;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

type SeedRow = (i64, i64, i64, i64, i64, i64, String, bool);
type RuleRow = (
    Option<i64>,
    String,
    String,
    i64,
    String,
    String,
    Option<i64>,
    Option<i64>,
);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrafficBillingMode {
    #[default]
    Sum,
    InboundOnly,
    OutboundOnly,
    MaxDirection,
    SeparateDirections,
}

impl TrafficBillingMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sum => "sum",
            Self::InboundOnly => "inbound_only",
            Self::OutboundOnly => "outbound_only",
            Self::MaxDirection => "max_direction",
            Self::SeparateDirections => "separate_directions",
        }
    }

    fn parse(value: &str) -> Result<Self, DbError> {
        match value {
            "sum" => Ok(Self::Sum),
            "inbound_only" => Ok(Self::InboundOnly),
            "outbound_only" => Ok(Self::OutboundOnly),
            "max_direction" => Ok(Self::MaxDirection),
            "separate_directions" => Ok(Self::SeparateDirections),
            _ => Err(DbError::Invalid("invalid stored traffic billing mode")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrafficAdjustmentDirection {
    #[default]
    Billed,
    Inbound,
    Outbound,
}

impl TrafficAdjustmentDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Billed => "billed",
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficSeedInput {
    #[serde(default)]
    pub combined_bytes: Option<u64>,
    #[serde(default)]
    pub rx_bytes: Option<u64>,
    #[serde(default)]
    pub tx_bytes: Option<u64>,
    pub effective_at: i64,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficAdjustmentInput {
    pub signed_bytes: i64,
    #[serde(default)]
    pub direction: TrafficAdjustmentDirection,
    pub effective_at: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrafficUsage {
    pub cycle_id: i64,
    pub cycle_start: i64,
    pub cycle_end: i64,
    pub seed_bytes: i64,
    pub seed_rx_bytes: Option<i64>,
    pub seed_tx_bytes: Option<i64>,
    pub seed_combined_bytes: i64,
    pub directional_seed_known: bool,
    pub has_manual_seed: bool,
    pub seed_effective_at: Option<i64>,
    pub seed_checkpoint_at: Option<i64>,
    pub seed_note: Option<String>,
    pub observed_rx_bytes: i64,
    pub observed_tx_bytes: i64,
    pub observed_bytes: i64,
    pub adjustment_bytes: i64,
    pub billed_adjustment_bytes: i64,
    pub rx_adjustment_bytes: i64,
    pub tx_adjustment_bytes: i64,
    pub total_bytes: i64,
    pub billed_total_bytes: Option<i64>,
    pub limit_bytes: Option<i64>,
    pub rx_total_bytes: Option<i64>,
    pub tx_total_bytes: Option<i64>,
    pub rx_limit_bytes: Option<i64>,
    pub tx_limit_bytes: Option<i64>,
    pub billing_mode: TrafficBillingMode,
    pub confidence: String,
    pub checkpoint_at: i64,
    pub agent_observed_total_bytes: i64,
    pub observation_start_at: i64,
    pub projected_bytes: Option<i64>,
    pub projected_rx_bytes: Option<i64>,
    pub projected_tx_bytes: Option<i64>,
    pub selected_interfaces: serde_json::Value,
    pub actual_selected_interfaces: Vec<String>,
    pub traffic_anomalies: Vec<String>,
    pub uncertainty_reason: Option<String>,
    pub timezone: String,
    pub anchor_day: u32,
    pub anchor_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrafficHistoryAdjustment {
    pub direction: String,
    pub signed_bytes: i64,
    pub effective_at: i64,
    pub reason: String,
    pub created_at: i64,
    pub operator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrafficCycleHistory {
    pub cycle_id: i64,
    pub cycle_start: i64,
    pub cycle_end: i64,
    pub state: String,
    pub confidence: String,
    pub billing_mode: TrafficBillingMode,
    pub has_manual_seed: bool,
    pub seed_bytes: i64,
    pub seed_note: Option<String>,
    pub seed_effective_at: Option<i64>,
    pub seed_checkpoint_at: Option<i64>,
    pub seed_operator: Option<String>,
    pub observed_rx_bytes: i64,
    pub observed_tx_bytes: i64,
    pub adjustment_bytes: i64,
    pub total_bytes: i64,
    pub billed_total_bytes: Option<i64>,
    pub rx_total_bytes: Option<i64>,
    pub tx_total_bytes: Option<i64>,
    pub adjustments: Vec<TrafficHistoryAdjustment>,
    pub adjustments_truncated: bool,
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
    #[serde(default)]
    pub billing_mode: TrafficBillingMode,
    #[serde(default)]
    pub rx_limit_bytes: Option<u64>,
    #[serde(default)]
    pub tx_limit_bytes: Option<u64>,
}

fn stored_bytes(value: Option<u64>, label: &'static str) -> Result<Option<i64>, DbError> {
    value
        .map(|bytes| i64::try_from(bytes).map_err(|_| DbError::Invalid(label)))
        .transpose()
}

fn billed_value(mode: TrafficBillingMode, combined: i64, rx: i64, tx: i64) -> i64 {
    match mode {
        TrafficBillingMode::Sum => combined,
        TrafficBillingMode::InboundOnly => rx,
        TrafficBillingMode::OutboundOnly => tx,
        TrafficBillingMode::MaxDirection => rx.max(tx),
        TrafficBillingMode::SeparateDirections => rx.saturating_add(tx),
    }
}

fn validate_seed(
    mode: TrafficBillingMode,
    input: &TrafficSeedInput,
) -> Result<(i64, i64, i64, bool), DbError> {
    let combined = stored_bytes(input.combined_bytes, "seed too large")?;
    let rx = stored_bytes(input.rx_bytes, "inbound seed too large")?;
    let tx = stored_bytes(input.tx_bytes, "outbound seed too large")?;
    match mode {
        TrafficBillingMode::Sum => match (combined, rx, tx) {
            (Some(value), None, None) => Ok((0, 0, value, false)),
            (None, Some(rx), Some(tx)) => Ok((rx, tx, rx.saturating_add(tx), true)),
            _ => Err(DbError::Invalid(
                "sum billing requires either one combined seed or both directional seeds",
            )),
        },
        TrafficBillingMode::InboundOnly => match (combined, rx, tx) {
            (None, Some(rx), None) => Ok((rx, 0, rx, true)),
            _ => Err(DbError::Invalid(
                "inbound-only billing requires an inbound seed",
            )),
        },
        TrafficBillingMode::OutboundOnly => match (combined, rx, tx) {
            (None, None, Some(tx)) => Ok((0, tx, tx, true)),
            _ => Err(DbError::Invalid(
                "outbound-only billing requires an outbound seed",
            )),
        },
        TrafficBillingMode::MaxDirection | TrafficBillingMode::SeparateDirections => {
            match (combined, rx, tx) {
                (None, Some(rx), Some(tx)) => Ok((rx, tx, billed_value(mode, 0, rx, tx), true)),
                _ => Err(DbError::Invalid(
                    "this billing mode requires both inbound and outbound seeds",
                )),
            }
        }
    }
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
        let rx_limit = stored_bytes(input.rx_limit_bytes, "inbound limit too large")?;
        let tx_limit = stored_bytes(input.tx_limit_bytes, "outbound limit too large")?;
        if input.billing_mode == TrafficBillingMode::SeparateDirections {
            if limit.is_some() {
                return Err(DbError::Invalid(
                    "separate-direction billing uses inbound and outbound limits",
                ));
            }
        } else if rx_limit.is_some() || tx_limit.is_some() {
            return Err(DbError::Invalid(
                "directional limits require separate-direction billing",
            ));
        }
        let policy=serde_json::json!({"mode":if input.selected_interfaces.is_empty(){"auto"}else{"manual"},"selected":input.selected_interfaces,"excluded":input.excluded_interfaces}).to_string();
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let locked: i64 = tx.query_row(
            "SELECT (SELECT COUNT(*) FROM billing_cycle_instance WHERE server_id=?1 AND state!='open') + (SELECT COUNT(*) FROM traffic_seed s JOIN billing_cycle_instance c ON c.id=s.cycle_id WHERE c.server_id=?1) + (SELECT COUNT(*) FROM traffic_adjustment a JOIN billing_cycle_instance c ON c.id=a.cycle_id WHERE c.server_id=?1)",
            [server_id],
            |row| row.get(0),
        )?;
        if locked > 0 {
            let existing: (String, i64, String, String) = tx.query_row(
                "SELECT timezone,anchor_day,anchor_time,billing_mode FROM billing_cycle_rule WHERE server_id=?1",
                [server_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;
            if existing.0 != input.timezone
                || existing.1 != i64::from(input.anchor_day)
                || existing.2 != input.anchor_time
                || existing.3 != input.billing_mode.as_str()
            {
                return Err(DbError::Conflict(
                    "timezone, cycle anchor, and billing mode are immutable after seeded or closed history exists; interface policy and limits remain editable",
                ));
            }
        }
        tx.execute("INSERT INTO billing_cycle_rule(server_id,timezone,anchor_day,anchor_time,interface_policy_json,traffic_limit_bytes,enabled,version,updated_at,updated_by,billing_mode,rx_limit_bytes,tx_limit_bytes) VALUES(?1,?2,?3,?4,?5,?6,1,1,?7,?8,?9,?10,?11) ON CONFLICT(server_id) DO UPDATE SET timezone=excluded.timezone,anchor_day=excluded.anchor_day,anchor_time=excluded.anchor_time,interface_policy_json=excluded.interface_policy_json,traffic_limit_bytes=excluded.traffic_limit_bytes,billing_mode=excluded.billing_mode,rx_limit_bytes=excluded.rx_limit_bytes,tx_limit_bytes=excluded.tx_limit_bytes,version=billing_cycle_rule.version+1,updated_at=excluded.updated_at,updated_by=excluded.updated_by",params![server_id,input.timezone,input.anchor_day,input.anchor_time,policy,limit,now,operator,input.billing_mode.as_str(),rx_limit,tx_limit])?;
        if locked == 0 {
            tx.execute(
                "DELETE FROM billing_cycle_instance WHERE server_id=?1 AND state='open'",
                [server_id],
            )?;
        }
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
        input: &TrafficSeedInput,
        now: i64,
        operator: &str,
    ) -> Result<TrafficUsage, DbError> {
        if input.note.len() > 500 {
            return Err(DbError::Invalid("note is too long"));
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let mode_value: String = tx.query_row(
            "SELECT billing_mode FROM billing_cycle_rule WHERE server_id=?1",
            [server_id],
            |row| row.get(0),
        )?;
        let mode = TrafficBillingMode::parse(&mode_value)?;
        let (rx_bytes, tx_bytes, combined_bytes, directional) = validate_seed(mode, input)?;
        let checkpoint:(i64,i64,i64,String)=tx.query_row("SELECT observed_rx,observed_tx,checkpoint_at,confidence FROM traffic_observed_checkpoint WHERE server_id=?1 AND checkpoint_at<=?2 ORDER BY checkpoint_at DESC LIMIT 1",params![server_id,input.effective_at],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.ok_or(DbError::Invalid("no reliable checkpoint at or before effective timestamp"))?;
        if checkpoint.2 != input.effective_at {
            return Err(DbError::Invalid(
                "manual seed must use an exact Agent checkpoint",
            ));
        }
        ensure_cycle_tx(&tx, server_id, checkpoint.2, checkpoint.0, checkpoint.1)?;
        let cycle_id:i64=tx.query_row("SELECT id FROM billing_cycle_instance WHERE server_id=?1 AND start_at<=?2 AND end_at>?2 ORDER BY start_at DESC LIMIT 1",params![server_id,input.effective_at],|r|r.get(0))?;
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
        tx.execute("INSERT INTO traffic_seed(cycle_id,rx_bytes,tx_bytes,combined_bytes,effective_at,checkpoint_at,observed_rx_at_seed,observed_tx_at_seed,operator,note,created_at,directional_seed) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![cycle_id,rx_bytes,tx_bytes,combined_bytes,input.effective_at,checkpoint.2,checkpoint.0,checkpoint.1,operator,input.note,now,i64::from(directional)])?;
        tx.execute(
            "UPDATE billing_cycle_instance SET confidence=?2 WHERE id=?1",
            params![cycle_id, checkpoint.3],
        )?;
        audit(&tx,now,operator,"traffic.seed.create",Some(server_id),&serde_json::json!({"cycle_id":cycle_id,"billing_mode":mode,"combined_bytes":combined_bytes,"rx_bytes":rx_bytes,"tx_bytes":tx_bytes,"effective_at":input.effective_at,"checkpoint_at":checkpoint.2,"note":input.note}).to_string())?;
        tx.commit()?;
        self.traffic_usage(server_id)
    }

    pub fn add_traffic_adjustment(
        &self,
        server_id: &str,
        input: &TrafficAdjustmentInput,
        now: i64,
        operator: &str,
    ) -> Result<TrafficUsage, DbError> {
        if input.reason.trim().len() < 3 || input.reason.len() > 500 {
            return Err(DbError::Invalid("a concise adjustment reason is required"));
        }
        if input.effective_at > now {
            return Err(DbError::Invalid(
                "adjustment effective time cannot be future",
            ));
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let usage = traffic_usage_conn(&tx, server_id)?;
        if input.effective_at < usage.cycle_start || input.effective_at >= usage.cycle_end {
            return Err(DbError::Invalid(
                "adjustment effective time is outside the current open cycle",
            ));
        }
        match usage.billing_mode {
            TrafficBillingMode::SeparateDirections => {
                if input.direction == TrafficAdjustmentDirection::Billed {
                    return Err(DbError::Invalid(
                        "separate-direction billing requires an inbound or outbound adjustment",
                    ));
                }
            }
            _ if input.direction != TrafficAdjustmentDirection::Billed => {
                return Err(DbError::Invalid(
                    "this billing mode accepts billed-total adjustments only",
                ));
            }
            _ => {}
        }
        match input.direction {
            TrafficAdjustmentDirection::Billed => {
                if usage.total_bytes.saturating_add(input.signed_bytes) < 0 {
                    return Err(DbError::Invalid("adjustment would make usage negative"));
                }
            }
            TrafficAdjustmentDirection::Inbound => {
                if usage.rx_total_bytes.is_none() {
                    return Err(DbError::Invalid(
                        "inbound adjustment requires a directional provider seed",
                    ));
                }
                if usage
                    .rx_total_bytes
                    .unwrap_or(0)
                    .saturating_add(input.signed_bytes)
                    < 0
                {
                    return Err(DbError::Invalid(
                        "adjustment would make inbound usage negative",
                    ));
                }
            }
            TrafficAdjustmentDirection::Outbound => {
                if usage.tx_total_bytes.is_none() {
                    return Err(DbError::Invalid(
                        "outbound adjustment requires a directional provider seed",
                    ));
                }
                if usage
                    .tx_total_bytes
                    .unwrap_or(0)
                    .saturating_add(input.signed_bytes)
                    < 0
                {
                    return Err(DbError::Invalid(
                        "adjustment would make outbound usage negative",
                    ));
                }
            }
        }
        tx.execute("INSERT INTO traffic_adjustment(cycle_id,signed_bytes,effective_at,reason,operator,created_at,direction) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![usage.cycle_id,input.signed_bytes,input.effective_at,input.reason,operator,now,input.direction.as_str()])?;
        audit(
            &tx,
            now,
            operator,
            "traffic.adjustment.create",
            Some(server_id),
            &serde_json::json!({"cycle_id":usage.cycle_id,"bytes":input.signed_bytes,"direction":input.direction,"reason":input.reason})
                .to_string(),
        )?;
        tx.commit()?;
        self.traffic_usage(server_id)
    }

    pub fn traffic_usage(&self, server_id: &str) -> Result<TrafficUsage, DbError> {
        traffic_usage_conn(&self.connection()?, server_id)
    }

    pub fn traffic_history(&self, server_id: &str) -> Result<Vec<TrafficCycleHistory>, DbError> {
        let conn = self.connection()?;
        let mode_value: String = conn.query_row(
            "SELECT billing_mode FROM billing_cycle_rule WHERE server_id=?1",
            [server_id],
            |row| row.get(0),
        )?;
        let mode = TrafficBillingMode::parse(&mode_value)?;
        let latest_checkpoint: Option<(i64, i64)> = conn
            .query_row(
                "SELECT observed_rx,observed_tx FROM traffic_observed_checkpoint WHERE server_id=?1 ORDER BY checkpoint_at DESC LIMIT 1",
                [server_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let mut statement = conn.prepare("SELECT id,start_at,end_at,starting_observed_rx,starting_observed_tx,ending_observed_rx,ending_observed_tx,state,confidence FROM billing_cycle_instance WHERE server_id=?1 AND state!='open' ORDER BY start_at DESC LIMIT 24")?;
        let cycles = statement
            .query_map([server_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        let mut history = Vec::with_capacity(cycles.len());
        for (cycle_id, start, end, start_rx, start_tx, end_rx, end_tx, state, confidence) in cycles
        {
            let seed: Option<SeedRow> = conn
                .query_row(
                    "SELECT rx_bytes,tx_bytes,combined_bytes,observed_rx_at_seed,observed_tx_at_seed,effective_at,note,directional_seed FROM traffic_seed WHERE cycle_id=?1 AND active_primary=1",
                    [cycle_id],
                    |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get::<_,i64>(7)? != 0)),
                )
                .optional()?;
            let has_manual_seed = seed.is_some();
            let seed_note = seed.as_ref().map(|value| value.6.clone());
            let seed_effective_at = seed.as_ref().map(|value| value.5);
            let seed_audit: Option<(i64, String)> = conn
                .query_row(
                    "SELECT checkpoint_at,operator FROM traffic_seed WHERE cycle_id=?1 AND active_primary=1",
                    [cycle_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let (seed_rx, seed_tx, seed_combined, base_rx, base_tx, directional_known) = seed
                .map(|value| (value.0, value.1, value.2, value.3, value.4, value.7))
                .unwrap_or((0, 0, 0, start_rx, start_tx, true));
            let (fallback_rx, fallback_tx) = latest_checkpoint.unwrap_or((start_rx, start_tx));
            let current_rx = end_rx.unwrap_or(fallback_rx).max(base_rx);
            let current_tx = end_tx.unwrap_or(fallback_tx).max(base_tx);
            let observed_rx = current_rx.saturating_sub(base_rx);
            let observed_tx = current_tx.saturating_sub(base_tx);
            let (billed_adjustments, rx_adjustments, tx_adjustments): (i64, i64, i64) =
                conn.query_row(
                    "SELECT COALESCE(SUM(CASE WHEN direction='billed' THEN signed_bytes ELSE 0 END),0),COALESCE(SUM(CASE WHEN direction='inbound' THEN signed_bytes ELSE 0 END),0),COALESCE(SUM(CASE WHEN direction='outbound' THEN signed_bytes ELSE 0 END),0) FROM traffic_adjustment WHERE cycle_id=?1",
                    [cycle_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;
            let unadjusted_rx = seed_rx.saturating_add(observed_rx);
            let unadjusted_tx = seed_tx.saturating_add(observed_tx);
            let adjusted_rx = unadjusted_rx.saturating_add(rx_adjustments).max(0);
            let adjusted_tx = unadjusted_tx.saturating_add(tx_adjustments).max(0);
            let unadjusted_billed = match mode {
                TrafficBillingMode::Sum if !directional_known && has_manual_seed => seed_combined
                    .saturating_add(observed_rx)
                    .saturating_add(observed_tx),
                _ => billed_value(
                    mode,
                    unadjusted_rx.saturating_add(unadjusted_tx),
                    unadjusted_rx,
                    unadjusted_tx,
                ),
            };
            let directionally_adjusted = match mode {
                TrafficBillingMode::Sum if !directional_known && has_manual_seed => {
                    unadjusted_billed
                }
                _ => billed_value(
                    mode,
                    adjusted_rx.saturating_add(adjusted_tx),
                    adjusted_rx,
                    adjusted_tx,
                ),
            };
            let total = directionally_adjusted
                .saturating_add(billed_adjustments)
                .max(0);
            let mut adjustment_statement = conn.prepare("SELECT direction,signed_bytes,effective_at,reason,created_at,operator FROM traffic_adjustment WHERE cycle_id=?1 ORDER BY created_at,id LIMIT 51")?;
            let mut adjustments = adjustment_statement
                .query_map([cycle_id], |row| {
                    Ok(TrafficHistoryAdjustment {
                        direction: row.get(0)?,
                        signed_bytes: row.get(1)?,
                        effective_at: row.get(2)?,
                        reason: row.get(3)?,
                        created_at: row.get(4)?,
                        operator: row.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            let adjustments_truncated = adjustments.len() > 50;
            adjustments.truncate(50);
            history.push(TrafficCycleHistory {
                cycle_id,
                cycle_start: start,
                cycle_end: end,
                state,
                confidence,
                billing_mode: mode,
                has_manual_seed,
                seed_bytes: billed_value(mode, seed_combined, seed_rx, seed_tx),
                seed_note,
                seed_effective_at,
                seed_checkpoint_at: seed_audit.as_ref().map(|value| value.0),
                seed_operator: seed_audit.map(|value| value.1),
                observed_rx_bytes: observed_rx,
                observed_tx_bytes: observed_tx,
                adjustment_bytes: total.saturating_sub(unadjusted_billed),
                total_bytes: total,
                billed_total_bytes: (mode != TrafficBillingMode::SeparateDirections)
                    .then_some(total),
                rx_total_bytes: directional_known
                    .then_some(adjusted_rx)
                    .filter(|_| mode != TrafficBillingMode::OutboundOnly),
                tx_total_bytes: directional_known
                    .then_some(adjusted_tx)
                    .filter(|_| mode != TrafficBillingMode::InboundOnly),
                adjustments,
                adjustments_truncated,
            });
        }
        Ok(history)
    }
}

fn traffic_usage_conn(conn: &Connection, server_id: &str) -> Result<TrafficUsage, DbError> {
    let (cycle_id,start,end,start_rx,start_tx,confidence):(i64,i64,i64,i64,i64,String)=conn.query_row("SELECT id,start_at,end_at,starting_observed_rx,starting_observed_tx,confidence FROM billing_cycle_instance WHERE server_id=?1 AND state='open' ORDER BY start_at DESC LIMIT 1",[server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?.ok_or(DbError::NotFound)?;
    let (current_rx,current_tx,checkpoint_at):(i64,i64,i64)=conn.query_row("SELECT observed_rx,observed_tx,checkpoint_at FROM traffic_observed_checkpoint WHERE server_id=?1 ORDER BY checkpoint_at DESC LIMIT 1",[server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
    let seed:Option<SeedRow>=conn.query_row("SELECT rx_bytes,tx_bytes,combined_bytes,observed_rx_at_seed,observed_tx_at_seed,effective_at,note,directional_seed FROM traffic_seed WHERE cycle_id=?1 AND active_primary=1",[cycle_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get::<_,i64>(7)? != 0))).optional()?;
    let has_manual_seed = seed.is_some();
    let seed_effective_at = seed.as_ref().map(|value| value.5);
    let seed_checkpoint_at: Option<i64> = conn
        .query_row(
            "SELECT checkpoint_at FROM traffic_seed WHERE cycle_id=?1 AND active_primary=1",
            [cycle_id],
            |row| row.get(0),
        )
        .optional()?;
    let seed_note = seed.as_ref().map(|value| value.6.clone());
    let (seed_rx, seed_tx, seed_combined, base_rx, base_tx, directional_seed_known) = seed
        .map(|value| (value.0, value.1, value.2, value.3, value.4, value.7))
        .unwrap_or((0, 0, 0, start_rx, start_tx, true));
    let observed_rx = current_rx.saturating_sub(base_rx);
    let observed_tx = current_tx.saturating_sub(base_tx);
    let (billed_adjustments,rx_adjustments,tx_adjustments):(i64,i64,i64)=conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN direction='billed' THEN signed_bytes ELSE 0 END),0),COALESCE(SUM(CASE WHEN direction='inbound' THEN signed_bytes ELSE 0 END),0),COALESCE(SUM(CASE WHEN direction='outbound' THEN signed_bytes ELSE 0 END),0) FROM traffic_adjustment WHERE cycle_id=?1",
        [cycle_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let (limit,policy,timezone,anchor_day,anchor_time,mode_value,rx_limit,tx_limit):RuleRow=conn.query_row("SELECT traffic_limit_bytes,interface_policy_json,timezone,anchor_day,anchor_time,billing_mode,rx_limit_bytes,tx_limit_bytes FROM billing_cycle_rule WHERE server_id=?1",[server_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?)))?;
    let latest_detail: Option<(String, String, String)> = conn
        .query_row(
            "SELECT interfaces_json,anomaly_flags_json,confidence FROM traffic_rollup WHERE server_id=?1 ORDER BY interval_end DESC LIMIT 1",
            [server_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let actual_selected_interfaces = latest_detail
        .as_ref()
        .and_then(|value| serde_json::from_str::<Vec<InterfaceCounter>>(&value.0).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| interface.selected)
        .map(|interface| interface.name)
        .collect::<Vec<_>>();
    let traffic_anomalies = latest_detail
        .as_ref()
        .and_then(|value| serde_json::from_str::<Vec<String>>(&value.1).ok())
        .unwrap_or_default();
    let confidence = if confidence == "estimated" {
        confidence
    } else if latest_detail
        .as_ref()
        .is_some_and(|value| value.2 != "high")
    {
        latest_detail
            .as_ref()
            .map(|value| value.2.clone())
            .unwrap_or(confidence)
    } else {
        confidence
    };
    let uncertainty_reason = if confidence == "high" {
        None
    } else if traffic_anomalies.is_empty() {
        Some(
            "The latest checkpoint is incomplete or the cycle boundary could not be split exactly."
                .into(),
        )
    } else {
        Some(traffic_anomalies.join("; "))
    };
    let mode = TrafficBillingMode::parse(&mode_value)?;
    let seed_bytes = billed_value(mode, seed_combined, seed_rx, seed_tx);
    let unadjusted_rx = seed_rx.saturating_add(observed_rx);
    let unadjusted_tx = seed_tx.saturating_add(observed_tx);
    let adjusted_rx = unadjusted_rx.saturating_add(rx_adjustments).max(0);
    let adjusted_tx = unadjusted_tx.saturating_add(tx_adjustments).max(0);
    let unadjusted_billed = match mode {
        TrafficBillingMode::Sum if !directional_seed_known && has_manual_seed => seed_combined
            .saturating_add(observed_rx)
            .saturating_add(observed_tx),
        _ => billed_value(
            mode,
            unadjusted_rx.saturating_add(unadjusted_tx),
            unadjusted_rx,
            unadjusted_tx,
        ),
    };
    let directionally_adjusted_billed = match mode {
        TrafficBillingMode::Sum if !directional_seed_known && has_manual_seed => unadjusted_billed,
        _ => billed_value(
            mode,
            adjusted_rx.saturating_add(adjusted_tx),
            adjusted_rx,
            adjusted_tx,
        ),
    };
    let observed = unadjusted_billed.saturating_sub(seed_bytes).max(0);
    let total = directionally_adjusted_billed
        .saturating_add(billed_adjustments)
        .max(0);
    let adjustments = total.saturating_sub(unadjusted_billed);
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
    let elapsed = checkpoint_at.saturating_sub(observation_start_at);
    let remaining = end.saturating_sub(checkpoint_at).max(0);
    let projections = (elapsed >= 300).then(|| {
        let project = |value: i64| {
            let future = (value as i128)
                .saturating_mul(remaining as i128)
                .checked_div(elapsed as i128)
                .unwrap_or(0);
            value.saturating_add(i64::try_from(future).unwrap_or(i64::MAX))
        };
        let projected_rx = seed_rx
            .saturating_add(project(observed_rx))
            .saturating_add(rx_adjustments);
        let projected_tx = seed_tx
            .saturating_add(project(observed_tx))
            .saturating_add(tx_adjustments);
        let billed = match mode {
            TrafficBillingMode::Sum if !directional_seed_known && has_manual_seed => seed_combined
                .saturating_add(project(observed_rx))
                .saturating_add(project(observed_tx)),
            _ => billed_value(
                mode,
                projected_rx.saturating_add(projected_tx),
                projected_rx,
                projected_tx,
            ),
        };
        (
            billed.saturating_add(billed_adjustments).max(0),
            projected_rx.max(0),
            projected_tx.max(0),
        )
    });
    let (rx_total_bytes, tx_total_bytes) = if directional_seed_known {
        let rx_known = !matches!(mode, TrafficBillingMode::OutboundOnly);
        let tx_known = !matches!(mode, TrafficBillingMode::InboundOnly);
        (
            rx_known.then_some(adjusted_rx),
            tx_known.then_some(adjusted_tx),
        )
    } else {
        (None, None)
    };
    Ok(TrafficUsage {
        cycle_id,
        cycle_start: start,
        cycle_end: end,
        seed_bytes,
        seed_rx_bytes: directional_seed_known
            .then_some(seed_rx)
            .filter(|_| !matches!(mode, TrafficBillingMode::OutboundOnly)),
        seed_tx_bytes: directional_seed_known
            .then_some(seed_tx)
            .filter(|_| !matches!(mode, TrafficBillingMode::InboundOnly)),
        seed_combined_bytes: seed_combined,
        directional_seed_known,
        has_manual_seed,
        seed_effective_at,
        seed_checkpoint_at,
        seed_note,
        observed_rx_bytes: observed_rx,
        observed_tx_bytes: observed_tx,
        observed_bytes: observed,
        adjustment_bytes: adjustments,
        billed_adjustment_bytes: billed_adjustments,
        rx_adjustment_bytes: rx_adjustments,
        tx_adjustment_bytes: tx_adjustments,
        total_bytes: total,
        limit_bytes: limit,
        rx_total_bytes,
        tx_total_bytes,
        rx_limit_bytes: rx_limit,
        tx_limit_bytes: tx_limit,
        billing_mode: mode,
        billed_total_bytes: (mode != TrafficBillingMode::SeparateDirections).then_some(total),
        confidence,
        checkpoint_at,
        agent_observed_total_bytes: current_rx.saturating_add(current_tx),
        observation_start_at,
        projected_bytes: projections.map(|value| value.0),
        projected_rx_bytes: projections
            .map(|value| value.1)
            .filter(|_| directional_seed_known && mode != TrafficBillingMode::OutboundOnly),
        projected_tx_bytes: projections
            .map(|value| value.2)
            .filter(|_| directional_seed_known && mode != TrafficBillingMode::InboundOnly),
        selected_interfaces: serde_json::from_str(&policy)
            .unwrap_or_else(|_| serde_json::json!({"mode":"unknown"})),
        actual_selected_interfaces,
        traffic_anomalies,
        uncertainty_reason,
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
            &TrafficSeedInput {
                combined_bytes: Some(100 * 1024 * 1024 * 1024),
                rx_bytes: None,
                tx_bytes: None,
                effective_at: 1_700_000_000,
                note: "provider dashboard".into(),
            },
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
                &TrafficAdjustmentInput {
                    signed_bytes: -1024 * 1024 * 1024,
                    direction: TrafficAdjustmentDirection::Billed,
                    effective_at: 1_700_000_300,
                    reason: "provider correction".into(),
                },
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
        reopened.rollover_due_cycles(usage.cycle_end + 1).unwrap();
        let history = reopened.traffic_history("s").unwrap();
        assert_eq!(history.len(), 1);
        let cycle = &history[0];
        assert_eq!(cycle.billing_mode, TrafficBillingMode::Sum);
        assert!(cycle.has_manual_seed);
        assert_eq!(cycle.seed_bytes, 100 * 1024 * 1024 * 1024);
        assert_eq!(cycle.observed_rx_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(cycle.observed_tx_bytes, 3 * 1024 * 1024 * 1024);
        assert_eq!(cycle.adjustment_bytes, -1024 * 1024 * 1024);
        assert_eq!(cycle.total_bytes, 104 * 1024 * 1024 * 1024);
        assert_eq!(cycle.adjustments.len(), 1);
        assert_eq!(cycle.adjustments[0].direction, "billed");
        assert_eq!(cycle.adjustments[0].reason, "provider correction");
        assert!(!cycle.adjustments_truncated);
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn seeded_history_locks_accounting_semantics_but_not_limits_or_interfaces() {
        let (db, path) = db();
        db.create_server("policy", "Policy", "", 1, "admin")
            .unwrap();
        checkpoint(&db, "policy", 10, 20, 1_700_000_000, 1);
        db.add_traffic_seed(
            "policy",
            &TrafficSeedInput {
                combined_bytes: Some(100),
                rx_bytes: None,
                tx_bytes: None,
                effective_at: 1_700_000_000,
                note: "provider dashboard".into(),
            },
            1_700_000_001,
            "admin",
        )
        .unwrap();
        let editable = CycleRuleInput {
            timezone: "UTC".into(),
            anchor_day: 1,
            anchor_time: "00:00".into(),
            selected_interfaces: vec!["eth0".into()],
            excluded_interfaces: vec!["docker0".into()],
            traffic_limit_bytes: Some(1_000),
            billing_mode: TrafficBillingMode::Sum,
            rx_limit_bytes: None,
            tx_limit_bytes: None,
        };
        db.set_cycle_rule("policy", &editable, 1_700_000_002, "admin")
            .unwrap();
        assert_eq!(
            db.traffic_usage("policy").unwrap().selected_interfaces["selected"][0],
            "eth0"
        );
        let mut reinterpret = editable;
        reinterpret.billing_mode = TrafficBillingMode::InboundOnly;
        assert!(matches!(
            db.set_cycle_rule("policy", &reinterpret, 1_700_000_003, "admin"),
            Err(DbError::Conflict(_))
        ));
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    fn configure_mode(db: &Database, server: &str, mode: TrafficBillingMode) {
        db.set_cycle_rule(
            server,
            &CycleRuleInput {
                timezone: "UTC".into(),
                anchor_day: 1,
                anchor_time: "00:00".into(),
                selected_interfaces: vec![],
                excluded_interfaces: vec![],
                traffic_limit_bytes: (mode != TrafficBillingMode::SeparateDirections)
                    .then_some(10_000),
                billing_mode: mode,
                rx_limit_bytes: (mode == TrafficBillingMode::SeparateDirections).then_some(5_000),
                tx_limit_bytes: (mode == TrafficBillingMode::SeparateDirections).then_some(6_000),
            },
            1_699_999_900,
            "admin",
        )
        .unwrap();
    }

    #[test]
    fn provider_billing_modes_apply_exact_directional_formulas() {
        let cases = [
            (
                TrafficBillingMode::Sum,
                TrafficSeedInput {
                    combined_bytes: Some(1_000),
                    rx_bytes: None,
                    tx_bytes: None,
                    effective_at: 1_700_000_000,
                    note: "combined".into(),
                },
                1_120,
                120,
            ),
            (
                TrafficBillingMode::InboundOnly,
                TrafficSeedInput {
                    combined_bytes: None,
                    rx_bytes: Some(1_000),
                    tx_bytes: None,
                    effective_at: 1_700_000_000,
                    note: "inbound".into(),
                },
                1_050,
                50,
            ),
            (
                TrafficBillingMode::OutboundOnly,
                TrafficSeedInput {
                    combined_bytes: None,
                    rx_bytes: None,
                    tx_bytes: Some(1_200),
                    effective_at: 1_700_000_000,
                    note: "outbound".into(),
                },
                1_270,
                70,
            ),
            (
                TrafficBillingMode::MaxDirection,
                TrafficSeedInput {
                    combined_bytes: None,
                    rx_bytes: Some(1_000),
                    tx_bytes: Some(1_200),
                    effective_at: 1_700_000_000,
                    note: "maximum direction".into(),
                },
                1_270,
                70,
            ),
            (
                TrafficBillingMode::SeparateDirections,
                TrafficSeedInput {
                    combined_bytes: None,
                    rx_bytes: Some(1_000),
                    tx_bytes: Some(1_200),
                    effective_at: 1_700_000_000,
                    note: "separate directions".into(),
                },
                2_320,
                120,
            ),
        ];
        for (index, (mode, seed, expected_total, expected_observed)) in
            cases.into_iter().enumerate()
        {
            let (db, path) = db();
            let server = format!("mode-{index}");
            db.create_server(&server, &server, "", 1, "admin").unwrap();
            configure_mode(&db, &server, mode);
            checkpoint(&db, &server, 100, 200, 1_700_000_000, 1);
            db.add_traffic_seed(&server, &seed, 1_700_000_001, "admin")
                .unwrap();
            checkpoint(&db, &server, 150, 270, 1_700_000_600, 2);
            let usage = db.traffic_usage(&server).unwrap();
            assert_eq!(usage.total_bytes, expected_total, "mode={mode:?}");
            assert_eq!(usage.observed_bytes, expected_observed, "mode={mode:?}");
            assert_eq!(usage.billing_mode, mode);
            assert_eq!(
                usage.billed_total_bytes.is_none(),
                mode == TrafficBillingMode::SeparateDirections
            );
            drop(db);
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn max_direction_rejects_ambiguous_combined_seed() {
        let (db, path) = db();
        db.create_server("max", "Max", "", 1, "admin").unwrap();
        configure_mode(&db, "max", TrafficBillingMode::MaxDirection);
        checkpoint(&db, "max", 100, 200, 1_700_000_000, 1);
        let result = db.add_traffic_seed(
            "max",
            &TrafficSeedInput {
                combined_bytes: Some(1_000),
                rx_bytes: None,
                tx_bytes: None,
                effective_at: 1_700_000_000,
                note: String::new(),
            },
            1_700_000_001,
            "admin",
        );
        assert!(matches!(result, Err(DbError::Invalid(_))));
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn separate_direction_adjustments_preserve_independent_totals() {
        let (db, path) = db();
        db.create_server("separate", "Separate", "", 1, "admin")
            .unwrap();
        configure_mode(&db, "separate", TrafficBillingMode::SeparateDirections);
        checkpoint(&db, "separate", 100, 200, 1_700_000_000, 1);
        db.add_traffic_seed(
            "separate",
            &TrafficSeedInput {
                combined_bytes: None,
                rx_bytes: Some(1_000),
                tx_bytes: Some(1_200),
                effective_at: 1_700_000_000,
                note: "directional".into(),
            },
            1_700_000_001,
            "admin",
        )
        .unwrap();
        checkpoint(&db, "separate", 150, 270, 1_700_000_600, 2);
        let usage = db
            .add_traffic_adjustment(
                "separate",
                &TrafficAdjustmentInput {
                    signed_bytes: -100,
                    direction: TrafficAdjustmentDirection::Outbound,
                    effective_at: 1_700_000_600,
                    reason: "provider outbound correction".into(),
                },
                1_700_000_601,
                "admin",
            )
            .unwrap();
        assert_eq!(usage.rx_total_bytes, Some(1_050));
        assert_eq!(usage.tx_total_bytes, Some(1_170));
        assert_eq!(usage.total_bytes, 2_220);
        assert_eq!(usage.adjustment_bytes, -100);
        assert_eq!(usage.billed_total_bytes, None);
        assert!(matches!(
            db.add_traffic_adjustment(
                "separate",
                &TrafficAdjustmentInput {
                    signed_bytes: 1,
                    direction: TrafficAdjustmentDirection::Billed,
                    effective_at: 1_700_000_600,
                    reason: "invalid combined correction".into(),
                },
                1_700_000_602,
                "admin"
            ),
            Err(DbError::Invalid(_))
        ));
        drop(db);
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
