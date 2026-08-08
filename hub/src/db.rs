//! Transactional Hub persistence. Connections are short-lived and SQLite WAL
//! coordinates them; no application-global mutex is held across I/O.

use parade_common::{sha256_hex, Confidence, SignedReport};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::path::{Path, PathBuf};
use std::sync::Arc;

type LeaseBinding = (String, i64, i64, String, Option<i64>, Option<i64>);

const MIGRATION_1: &str = r#"
CREATE TABLE servers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  group_name TEXT NOT NULL DEFAULT '',
  tags_json TEXT NOT NULL DEFAULT '[]',
  provider_label TEXT NOT NULL DEFAULT '',
  location TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL CHECK(status IN ('pending','active','revoked','deleted')),
  created_at INTEGER NOT NULL,
  deleted_at INTEGER,
  last_seen INTEGER,
  report_ip TEXT,
  os TEXT, kernel TEXT, arch TEXT, agent_version TEXT,
  inventory_hash TEXT,
  coverage_json TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE server_tombstones (
  server_id TEXT PRIMARY KEY,
  deleted_at INTEGER NOT NULL,
  reason TEXT NOT NULL,
  audit_id INTEGER NOT NULL
);
CREATE TABLE agent_credentials (
  agent_id TEXT PRIMARY KEY,
  server_id TEXT NOT NULL REFERENCES servers(id),
  public_key_hex TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('active','revoked')),
  created_at INTEGER NOT NULL,
  revoked_at INTEGER,
  UNIQUE(server_id, agent_id)
);
CREATE INDEX agent_credentials_server ON agent_credentials(server_id, status);
CREATE TABLE enrollment_tokens (
  token_hash TEXT PRIMARY KEY,
  server_id TEXT NOT NULL REFERENCES servers(id),
  expires_at INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  used_at INTEGER
);
CREATE INDEX enrollment_expiry ON enrollment_tokens(expires_at, used_at);
CREATE TABLE agent_replay (
  server_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  last_sequence INTEGER NOT NULL,
  last_sent_at INTEGER NOT NULL,
  PRIMARY KEY(server_id, agent_id)
);
CREATE TABLE report_messages (
  message_id TEXT PRIMARY KEY,
  server_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  sequence INTEGER NOT NULL,
  received_at INTEGER NOT NULL
);
CREATE INDEX report_messages_received ON report_messages(received_at);
CREATE TABLE resource_rollups (
  server_id TEXT NOT NULL,
  interval_start INTEGER NOT NULL,
  interval_end INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  PRIMARY KEY(server_id, interval_end)
);
CREATE INDEX resource_rollups_range ON resource_rollups(server_id, interval_end DESC);
CREATE TABLE process_summaries (
  server_id TEXT NOT NULL,
  observed_at INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  PRIMARY KEY(server_id, observed_at)
);
CREATE TABLE socket_summaries (
  server_id TEXT NOT NULL,
  observed_at INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  PRIMARY KEY(server_id, observed_at)
);
CREATE TABLE security_findings (
  id INTEGER PRIMARY KEY,
  server_id TEXT NOT NULL REFERENCES servers(id),
  rule_id TEXT NOT NULL,
  rule_version INTEGER NOT NULL,
  severity TEXT NOT NULL,
  confidence TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'active',
  first_seen INTEGER NOT NULL,
  last_seen INTEGER NOT NULL,
  occurrence_count INTEGER NOT NULL DEFAULT 1,
  evidence TEXT NOT NULL,
  explanation TEXT NOT NULL,
  verification TEXT NOT NULL,
  coverage_caveat TEXT NOT NULL DEFAULT '',
  suppression_expires_at INTEGER,
  UNIQUE(server_id, rule_id, evidence)
);
CREATE INDEX findings_active ON security_findings(state, severity, last_seen DESC);
CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  server_id TEXT,
  occurred_at INTEGER NOT NULL,
  category TEXT NOT NULL,
  severity TEXT NOT NULL,
  summary TEXT NOT NULL,
  evidence TEXT NOT NULL DEFAULT ''
);
CREATE INDEX events_time ON events(occurred_at DESC);
CREATE INDEX events_server_time ON events(server_id, occurred_at DESC);
CREATE TABLE audit_events (
  id INTEGER PRIMARY KEY,
  occurred_at INTEGER NOT NULL,
  operator TEXT NOT NULL,
  action TEXT NOT NULL,
  server_id TEXT,
  detail_json TEXT NOT NULL
);
CREATE INDEX audit_time ON audit_events(occurred_at DESC);
CREATE TABLE sessions (
  token_hash TEXT PRIMARY KEY,
  csrf_hash TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  revoked_at INTEGER,
  source_ip TEXT NOT NULL
);
CREATE INDEX sessions_expiry ON sessions(expires_at, revoked_at);
CREATE TABLE traffic_interface_state (
  server_id TEXT NOT NULL,
  identity TEXT NOT NULL,
  interface_name TEXT NOT NULL,
  boot_id TEXT NOT NULL,
  last_raw_rx INTEGER NOT NULL,
  last_raw_tx INTEGER NOT NULL,
  last_sample_at INTEGER NOT NULL,
  active INTEGER NOT NULL,
  selected INTEGER NOT NULL,
  reset_metadata TEXT NOT NULL DEFAULT '',
  PRIMARY KEY(server_id, identity)
);
CREATE TABLE traffic_observed_checkpoint (
  server_id TEXT NOT NULL,
  observed_rx INTEGER NOT NULL,
  observed_tx INTEGER NOT NULL,
  checkpoint_at INTEGER NOT NULL,
  agent_sequence INTEGER NOT NULL,
  confidence TEXT NOT NULL,
  last_boot_id TEXT NOT NULL,
  PRIMARY KEY(server_id, checkpoint_at)
);
CREATE INDEX traffic_checkpoint_latest ON traffic_observed_checkpoint(server_id, checkpoint_at DESC);
CREATE TABLE billing_cycle_rule (
  server_id TEXT PRIMARY KEY REFERENCES servers(id),
  timezone TEXT NOT NULL,
  anchor_day INTEGER NOT NULL CHECK(anchor_day BETWEEN 1 AND 31),
  anchor_time TEXT NOT NULL,
  interface_policy_json TEXT NOT NULL,
  traffic_limit_bytes INTEGER,
  enabled INTEGER NOT NULL,
  version INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  updated_by TEXT NOT NULL
);
CREATE TABLE billing_cycle_instance (
  id INTEGER PRIMARY KEY,
  server_id TEXT NOT NULL REFERENCES servers(id),
  start_at INTEGER NOT NULL,
  end_at INTEGER NOT NULL,
  starting_observed_rx INTEGER NOT NULL,
  starting_observed_tx INTEGER NOT NULL,
  ending_observed_rx INTEGER,
  ending_observed_tx INTEGER,
  state TEXT NOT NULL CHECK(state IN ('open','closed','estimated')),
  confidence TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  closed_at INTEGER,
  UNIQUE(server_id, start_at)
);
CREATE INDEX billing_cycle_open ON billing_cycle_instance(server_id, state, end_at);
CREATE TABLE traffic_seed (
  id INTEGER PRIMARY KEY,
  cycle_id INTEGER NOT NULL REFERENCES billing_cycle_instance(id),
  rx_bytes INTEGER NOT NULL,
  tx_bytes INTEGER NOT NULL,
  combined_bytes INTEGER NOT NULL,
  effective_at INTEGER NOT NULL,
  checkpoint_at INTEGER NOT NULL,
  observed_rx_at_seed INTEGER NOT NULL,
  observed_tx_at_seed INTEGER NOT NULL,
  operator TEXT NOT NULL,
  note TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  active_primary INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX one_primary_seed ON traffic_seed(cycle_id) WHERE active_primary = 1;
CREATE TABLE traffic_adjustment (
  id INTEGER PRIMARY KEY,
  cycle_id INTEGER NOT NULL REFERENCES billing_cycle_instance(id),
  signed_bytes INTEGER NOT NULL,
  effective_at INTEGER NOT NULL,
  reason TEXT NOT NULL,
  operator TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE traffic_rollup (
  server_id TEXT NOT NULL,
  interval_start INTEGER NOT NULL,
  interval_end INTEGER NOT NULL,
  observed_rx_delta INTEGER NOT NULL,
  observed_tx_delta INTEGER NOT NULL,
  interfaces_json TEXT NOT NULL,
  confidence TEXT NOT NULL,
  anomaly_flags_json TEXT NOT NULL,
  PRIMARY KEY(server_id, interval_end)
);
CREATE TABLE observation_leases (
  lease_id TEXT PRIMARY KEY,
  server_id TEXT NOT NULL REFERENCES servers(id),
  profile_json TEXT NOT NULL,
  issued_at INTEGER NOT NULL,
  expires_at INTEGER NOT NULL,
  state TEXT NOT NULL,
  issued_by TEXT NOT NULL
);
"#;

const MIGRATION_2: &str = r#"
ALTER TABLE observation_leases ADD COLUMN response_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE observation_leases ADD COLUMN encoded_response_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE observation_leases ADD COLUMN last_response_at INTEGER;
ALTER TABLE observation_leases ADD COLUMN ended_at INTEGER;
ALTER TABLE report_messages ADD COLUMN lease_id TEXT;
ALTER TABLE report_messages ADD COLUMN encoded_bytes INTEGER NOT NULL DEFAULT 0;
CREATE INDEX report_messages_lease ON report_messages(lease_id, received_at);
"#;

const MIGRATION_3: &str = r#"
ALTER TABLE agent_credentials ADD COLUMN traffic_policy_version INTEGER NOT NULL DEFAULT 0;
"#;

const MIGRATION_4: &str = r#"
ALTER TABLE billing_cycle_rule ADD COLUMN billing_mode TEXT NOT NULL DEFAULT 'sum' CHECK(billing_mode IN ('sum','inbound_only','outbound_only','max_direction','separate_directions'));
ALTER TABLE billing_cycle_rule ADD COLUMN rx_limit_bytes INTEGER CHECK(rx_limit_bytes IS NULL OR rx_limit_bytes >= 0);
ALTER TABLE billing_cycle_rule ADD COLUMN tx_limit_bytes INTEGER CHECK(tx_limit_bytes IS NULL OR tx_limit_bytes >= 0);
ALTER TABLE traffic_seed ADD COLUMN directional_seed INTEGER NOT NULL DEFAULT 0 CHECK(directional_seed IN (0,1));
ALTER TABLE traffic_adjustment ADD COLUMN direction TEXT NOT NULL DEFAULT 'billed' CHECK(direction IN ('billed','inbound','outbound'));
CREATE INDEX traffic_rollup_retention ON traffic_rollup(interval_end);
CREATE INDEX events_retention ON events(occurred_at);
CREATE INDEX process_retention ON process_summaries(observed_at);
CREATE INDEX socket_retention ON socket_summaries(observed_at);
CREATE INDEX checkpoint_retention ON traffic_observed_checkpoint(checkpoint_at);
CREATE INDEX lease_retention ON observation_leases(state,expires_at);
CREATE INDEX enrollment_retention ON enrollment_tokens(expires_at,used_at);
ALTER TABLE security_findings ADD COLUMN series_key TEXT NOT NULL DEFAULT 'legacy';
UPDATE security_findings SET series_key=printf('legacy-%d',id);
CREATE TEMP TABLE migration4_finding_rank AS
SELECT id,server_id,rule_id,rule_version,
       ROW_NUMBER() OVER (
           PARTITION BY server_id,rule_id,rule_version
           ORDER BY last_seen DESC,id DESC
       ) AS rank
FROM security_findings;
UPDATE security_findings AS keeper
SET first_seen=(
        SELECT MIN(f.first_seen) FROM security_findings f
        JOIN migration4_finding_rank r ON r.id=f.id
        WHERE r.server_id=keeper.server_id AND r.rule_id=keeper.rule_id
          AND r.rule_version=keeper.rule_version AND r.rank>=33
    ),
    last_seen=(
        SELECT MAX(f.last_seen) FROM security_findings f
        JOIN migration4_finding_rank r ON r.id=f.id
        WHERE r.server_id=keeper.server_id AND r.rule_id=keeper.rule_id
          AND r.rule_version=keeper.rule_version AND r.rank>=33
    ),
    occurrence_count=(
        SELECT SUM(f.occurrence_count) FROM security_findings f
        JOIN migration4_finding_rank r ON r.id=f.id
        WHERE r.server_id=keeper.server_id AND r.rule_id=keeper.rule_id
          AND r.rule_version=keeper.rule_version AND r.rank>=33
    ),
    series_key='overflow'
WHERE keeper.id IN (SELECT id FROM migration4_finding_rank WHERE rank=33);
DELETE FROM security_findings
WHERE id IN (SELECT id FROM migration4_finding_rank WHERE rank>33);
DROP TABLE migration4_finding_rank;
CREATE UNIQUE INDEX one_finding_series ON security_findings(server_id,rule_id,rule_version,series_key);
"#;

#[derive(Clone)]
pub struct Database {
    path: Arc<PathBuf>,
}

#[derive(Debug)]
pub enum DbError {
    Sql(rusqlite::Error),
    Invalid(&'static str),
    Conflict(&'static str),
    NotFound,
    Unauthorized,
    Duplicate,
    Replay,
}

impl From<rusqlite::Error> for DbError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql(error) => write!(f, "database error: {error}"),
            Self::Invalid(message) | Self::Conflict(message) => f.write_str(message),
            Self::NotFound => f.write_str("not found"),
            Self::Unauthorized => f.write_str("unauthorized"),
            Self::Duplicate => f.write_str("duplicate report"),
            Self::Replay => f.write_str("replayed or out-of-order report"),
        }
    }
}

impl std::error::Error for DbError {}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        let db = Self {
            path: Arc::new(path.as_ref().to_path_buf()),
        };
        let mut conn = db.connection()?;
        migrate(&mut conn)?;
        Ok(db)
    }

    pub fn connection(&self) -> Result<Connection, DbError> {
        let conn = Connection::open(self.path.as_ref())?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5_000)?;
        conn.pragma_update(None, "wal_autocheckpoint", 1_000)?;
        conn.pragma_update(None, "journal_size_limit", 16 * 1024 * 1024)?;
        Ok(conn)
    }

    pub fn create_server(
        &self,
        id: &str,
        name: &str,
        group: &str,
        now: i64,
        operator: &str,
    ) -> Result<(), DbError> {
        validate_id(id)?;
        if name.len() > 100 || group.len() > 100 {
            return Err(DbError::Invalid("server metadata is too long"));
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx
            .query_row(
                "SELECT 1 FROM server_tombstones WHERE server_id=?1",
                [id],
                |_| Ok(()),
            )
            .optional()?
            .is_some()
        {
            return Err(DbError::Conflict(
                "server id is tombstoned; restore must be explicit",
            ));
        }
        tx.execute(
            "INSERT INTO servers(id,name,group_name,status,created_at) VALUES(?1,?2,?3,'pending',?4)",
            params![id, if name.is_empty() { id } else { name }, group, now],
        )?;
        audit(&tx, now, operator, "server.create", Some(id), "{}")?;
        tx.commit()?;
        Ok(())
    }

    pub fn tombstone_server(
        &self,
        id: &str,
        now: i64,
        operator: &str,
        reason: &str,
    ) -> Result<(), DbError> {
        if reason.trim().len() < 3 {
            return Err(DbError::Invalid("deletion reason is required"));
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE servers SET status='deleted',deleted_at=?2 WHERE id=?1 AND status!='deleted'",
            params![id, now],
        )? != 1
        {
            return Err(DbError::NotFound);
        }
        tx.execute("UPDATE agent_credentials SET status='revoked',revoked_at=?2 WHERE server_id=?1 AND status='active'", params![id, now])?;
        // A server deletion is terminal for every outstanding enrollment
        // capability. Marking tokens consumed keeps that invariant durable
        // across Hub restarts and avoids retaining usable bearer material.
        tx.execute(
            "UPDATE enrollment_tokens SET used_at=?2 WHERE server_id=?1 AND used_at IS NULL",
            params![id, now],
        )?;
        audit(
            &tx,
            now,
            operator,
            "server.delete",
            Some(id),
            &serde_json::json!({"reason":reason}).to_string(),
        )?;
        let audit_id = tx.last_insert_rowid();
        tx.execute("INSERT INTO server_tombstones(server_id,deleted_at,reason,audit_id) VALUES(?1,?2,?3,?4)", params![id, now, reason, audit_id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn mint_enrollment(
        &self,
        server_id: &str,
        token: &str,
        expires_at: i64,
        now: i64,
        operator: &str,
    ) -> Result<(), DbError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let status: Option<String> = tx
            .query_row("SELECT status FROM servers WHERE id=?1", [server_id], |r| {
                r.get(0)
            })
            .optional()?;
        if !matches!(status.as_deref(), Some("pending" | "active")) {
            return Err(DbError::NotFound);
        }
        tx.execute(
            "DELETE FROM enrollment_tokens WHERE expires_at<=?1 OR used_at IS NOT NULL",
            [now],
        )?;
        tx.execute("INSERT INTO enrollment_tokens(token_hash,server_id,expires_at,created_at) VALUES(?1,?2,?3,?4)", params![sha256_hex(token.as_bytes()),server_id,expires_at,now])?;
        audit(
            &tx,
            now,
            operator,
            "agent.enrollment_token.create",
            Some(server_id),
            &serde_json::json!({"expires_at":expires_at}).to_string(),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn enroll_agent(
        &self,
        token: &str,
        public_key: &str,
        agent_id: &str,
        now: i64,
    ) -> Result<String, DbError> {
        if parade_common::hex_decode(public_key)
            .map_err(|_| DbError::Invalid("bad public key"))?
            .len()
            != 32
        {
            return Err(DbError::Invalid("bad public key"));
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let hash = sha256_hex(token.as_bytes());
        let row: Option<(String, i64, Option<i64>)> = tx
            .query_row(
                "SELECT server_id,expires_at,used_at FROM enrollment_tokens WHERE token_hash=?1",
                [&hash],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (server_id, expiry, used) = row.ok_or(DbError::Unauthorized)?;
        if used.is_some() || expiry <= now {
            return Err(DbError::Unauthorized);
        }
        let status: String = tx.query_row(
            "SELECT status FROM servers WHERE id=?1",
            [&server_id],
            |r| r.get(0),
        )?;
        if status == "deleted" || status == "revoked" {
            return Err(DbError::Unauthorized);
        }
        tx.execute(
            "UPDATE enrollment_tokens SET used_at=?2 WHERE token_hash=?1 AND used_at IS NULL",
            params![hash, now],
        )?;
        tx.execute("UPDATE agent_credentials SET status='revoked',revoked_at=?2 WHERE server_id=?1 AND status='active'", params![server_id,now])?;
        tx.execute("INSERT INTO agent_credentials(agent_id,server_id,public_key_hex,status,created_at,traffic_policy_version) VALUES(?1,?2,?3,'active',?4,COALESCE((SELECT version FROM billing_cycle_rule WHERE server_id=?2),0))", params![agent_id,server_id,public_key,now])?;
        tx.execute(
            "UPDATE servers SET status='active' WHERE id=?1",
            [&server_id],
        )?;
        audit(
            &tx,
            now,
            "enrollment",
            "agent.enroll",
            Some(&server_id),
            &serde_json::json!({"agent_id":agent_id}).to_string(),
        )?;
        tx.commit()?;
        Ok(server_id)
    }

    pub fn public_key_for(&self, server_id: &str, agent_id: &str) -> Result<String, DbError> {
        self.connection()?.query_row(
            "SELECT a.public_key_hex FROM agent_credentials a JOIN servers s ON s.id=a.server_id WHERE a.server_id=?1 AND a.agent_id=?2 AND a.status='active' AND s.status='active'",
            params![server_id,agent_id], |r| r.get(0),
        ).optional()?.ok_or(DbError::Unauthorized)
    }

    pub fn ingest_verified(
        &self,
        report: &SignedReport,
        ip: &str,
        received_at: i64,
        encoded_bytes: usize,
    ) -> Result<(), DbError> {
        report
            .body
            .validate()
            .map_err(|_| DbError::Invalid("invalid report"))?;
        let sequence =
            i64::try_from(report.sequence).map_err(|_| DbError::Invalid("sequence overflow"))?;
        let rx = to_i64(report.body.traffic.observed_rx)?;
        let tx_bytes = to_i64(report.body.traffic.observed_tx)?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let binding: Option<()> = tx
            .query_row(
                "SELECT 1 FROM agent_credentials a JOIN servers s ON s.id=a.server_id WHERE a.server_id=?1 AND a.agent_id=?2 AND a.status='active' AND s.status='active'",
                params![report.server_id, report.agent_id],
                |_| Ok(()),
            )
            .optional()?;
        if binding.is_none() {
            return Err(DbError::Unauthorized);
        }
        let replay: Option<(i64,i64)> = tx.query_row(
            "SELECT last_sequence,last_sent_at FROM agent_replay WHERE server_id=?1 AND agent_id=?2",
            params![report.server_id,report.agent_id], |r| Ok((r.get(0)?,r.get(1)?)),
        ).optional()?;
        if let Some((server, agent, existing_sequence)) = tx
            .query_row(
                "SELECT server_id,agent_id,sequence FROM report_messages WHERE message_id=?1",
                [&report.message_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?
        {
            return if server == report.server_id
                && agent == report.agent_id
                && existing_sequence == sequence
            {
                Err(DbError::Duplicate)
            } else {
                Err(DbError::Replay)
            };
        }
        let policy_transition = validate_interface_policy(&tx, report)?;
        if replay.is_some_and(|(last, _)| sequence <= last) {
            return Err(DbError::Replay);
        }
        let encoded_bytes = i64::try_from(encoded_bytes)
            .map_err(|_| DbError::Invalid("report size exceeds SQLite integer range"))?;
        if let Some(lease_id) = &report.body.lease_id {
            let lease: Option<LeaseBinding> = tx
                .query_row(
                    "SELECT profile_json,issued_at,expires_at,state,last_response_at,ended_at FROM observation_leases WHERE lease_id=?1 AND server_id=?2",
                    params![lease_id, report.server_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                )
                .optional()?;
            let Some((profile, issued_at, expires_at, state, last_response_at, ended_at)) = lease
            else {
                return Err(DbError::Invalid(
                    "report references an unknown observation lease",
                ));
            };
            let expected: parade_common::ObservationProfile = serde_json::from_str(&profile)
                .map_err(|_| DbError::Invalid("stored observation lease is malformed"))?;
            let cancelling_response = state == "cancelled"
                && ended_at.is_some_and(|ended| last_response_at.is_none_or(|last| last < ended));
            if (state != "active" && !cancelling_response)
                || report.sent_at < issued_at
                || report.sent_at > expires_at
                || report.body.profile != expected
            {
                return Err(DbError::Invalid(
                    "report does not match an active observation lease",
                ));
            }
            tx.execute(
                "UPDATE observation_leases SET response_count=response_count+1,encoded_response_bytes=encoded_response_bytes+?2,last_response_at=?3 WHERE lease_id=?1",
                params![lease_id, encoded_bytes, received_at],
            )?;
        } else if !matches!(
            report.body.profile,
            parade_common::ObservationProfile::Normal
        ) {
            return Err(DbError::Invalid(
                "non-normal observation profile requires a valid lease",
            ));
        }
        let previous: Option<(i64,i64,i64)> = tx.query_row(
            "SELECT observed_rx,observed_tx,checkpoint_at FROM traffic_observed_checkpoint WHERE server_id=?1 ORDER BY checkpoint_at DESC LIMIT 1",
            [&report.server_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional()?;
        if previous.is_some_and(|(_, _, previous_at)| report.body.traffic.sampled_at <= previous_at)
        {
            return Err(DbError::Replay);
        }
        if previous.is_some_and(|(old_rx, old_tx, _)| rx < old_rx || tx_bytes < old_tx) {
            return Err(DbError::Invalid("monotonic traffic checkpoint decreased"));
        }
        tx.execute("INSERT INTO report_messages(message_id,server_id,agent_id,sequence,received_at,lease_id,encoded_bytes) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![report.message_id,report.server_id,report.agent_id,sequence,received_at,report.body.lease_id,encoded_bytes])?;
        tx.execute(
            "INSERT INTO agent_replay(server_id,agent_id,last_sequence,last_sent_at) VALUES(?1,?2,?3,?4) ON CONFLICT(server_id,agent_id) DO UPDATE SET last_sequence=excluded.last_sequence,last_sent_at=excluded.last_sent_at",
            params![report.server_id,report.agent_id,sequence,report.sent_at],
        )?;
        tx.execute(
            "UPDATE servers SET last_seen=?2,report_ip=?3,os=?4,kernel=?5,arch=?6,agent_version=?7,inventory_hash=?8,coverage_json=?9 WHERE id=?1 AND status='active'",
            params![report.server_id,received_at,ip,report.body.os,report.body.kernel,report.body.arch,report.body.agent_version,report.body.inventory_hash,serde_json::to_string(&report.body.coverage).map_err(|_|DbError::Invalid("coverage encoding"))?],
        )?;
        tx.execute("INSERT OR REPLACE INTO resource_rollups(server_id,interval_start,interval_end,payload_json) VALUES(?1,?2,?3,?4)", params![report.server_id,report.body.resources.interval_start,report.body.resources.interval_end,serde_json::to_string(&report.body.resources).map_err(|_|DbError::Invalid("resource encoding"))?])?;
        if let Some(processes) = &report.body.processes {
            tx.execute("INSERT OR REPLACE INTO process_summaries(server_id,observed_at,payload_json) VALUES(?1,?2,?3)", params![report.server_id,report.sent_at,serde_json::to_string(processes).map_err(|_|DbError::Invalid("process encoding"))?])?;
        }
        if let Some(listeners) = &report.body.listeners {
            tx.execute("INSERT OR REPLACE INTO socket_summaries(server_id,observed_at,payload_json) VALUES(?1,?2,?3)", params![report.server_id,report.sent_at,serde_json::to_string(listeners).map_err(|_|DbError::Invalid("socket encoding"))?])?;
        }
        tx.execute("INSERT INTO traffic_observed_checkpoint(server_id,observed_rx,observed_tx,checkpoint_at,agent_sequence,confidence,last_boot_id) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![report.server_id,rx,tx_bytes,report.body.traffic.sampled_at,sequence,confidence(&report.body.traffic.confidence),report.body.traffic.boot_id])?;
        let (drx, dtx, start) = previous
            .map(|(a, b, t)| (rx - a, tx_bytes - b, t))
            .unwrap_or((0, 0, report.body.resources.interval_start));
        tx.execute("INSERT OR REPLACE INTO traffic_rollup(server_id,interval_start,interval_end,observed_rx_delta,observed_tx_delta,interfaces_json,confidence,anomaly_flags_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![report.server_id,start,report.body.traffic.sampled_at,drx,dtx,serde_json::to_string(&report.body.traffic.interfaces).map_err(|_|DbError::Invalid("interface encoding"))?,confidence(&report.body.traffic.confidence),serde_json::to_string(&report.body.traffic.anomaly_flags).map_err(|_|DbError::Invalid("anomaly encoding"))?])?;
        tx.execute(
            "UPDATE traffic_interface_state SET active=0 WHERE server_id=?1",
            [&report.server_id],
        )?;
        for interface in &report.body.traffic.interfaces {
            tx.execute("INSERT INTO traffic_interface_state(server_id,identity,interface_name,boot_id,last_raw_rx,last_raw_tx,last_sample_at,active,selected) VALUES(?1,?2,?3,?4,?5,?6,?7,1,?8) ON CONFLICT(server_id,identity) DO UPDATE SET interface_name=excluded.interface_name,boot_id=excluded.boot_id,last_raw_rx=excluded.last_raw_rx,last_raw_tx=excluded.last_raw_tx,last_sample_at=excluded.last_sample_at,active=1,selected=excluded.selected", params![report.server_id,interface.identity,interface.name,report.body.traffic.boot_id,to_i64(interface.rx_bytes)?,to_i64(interface.tx_bytes)?,report.body.traffic.sampled_at,interface.selected])?;
        }
        // Historical interface evidence remains in bounded traffic rollups;
        // the mutable baseline needs only the identities in the latest report.
        tx.execute(
            "DELETE FROM traffic_interface_state WHERE server_id=?1 AND active=0",
            [&report.server_id],
        )?;
        crate::findings::evaluate(&tx, report, received_at)?;
        crate::traffic::ensure_cycle_tx(
            &tx,
            &report.server_id,
            report.body.traffic.sampled_at,
            rx,
            tx_bytes,
        )?;
        let active_policy_version: i64 = tx.query_row(
            "SELECT version FROM billing_cycle_rule WHERE server_id=?1",
            [&report.server_id],
            |row| row.get(0),
        )?;
        tx.execute(
            "UPDATE agent_credentials SET traffic_policy_version=?3 WHERE server_id=?1 AND agent_id=?2 AND status='active'",
            params![report.server_id, report.agent_id, active_policy_version],
        )?;
        if policy_transition {
            tx.execute(
                "INSERT INTO events(server_id,occurred_at,category,severity,summary,evidence) VALUES(?1,?2,'traffic','info','Interface accounting policy delivered',?3)",
                params![report.server_id, received_at, format!("checkpoint policy v{}; acknowledgement delivers v{active_policy_version}", report.body.traffic.policy_version)],
            )?;
        }
        if !matches!(report.body.traffic.confidence, Confidence::High) {
            tx.execute(
                "UPDATE billing_cycle_instance SET confidence=?2 WHERE server_id=?1 AND state='open'",
                params![report.server_id, confidence(&report.body.traffic.confidence)],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn create_session(
        &self,
        token: &str,
        csrf: &str,
        source_ip: &str,
        now: i64,
        expires: i64,
    ) -> Result<(), DbError> {
        self.connection()?.execute("INSERT INTO sessions(token_hash,csrf_hash,created_at,expires_at,source_ip) VALUES(?1,?2,?3,?4,?5)", params![sha256_hex(token.as_bytes()),sha256_hex(csrf.as_bytes()),now,expires,source_ip])?;
        Ok(())
    }

    pub fn verify_session(&self, token: &str, now: i64) -> Result<bool, DbError> {
        Ok(self.connection()?.query_row("SELECT 1 FROM sessions WHERE token_hash=?1 AND expires_at>?2 AND revoked_at IS NULL", params![sha256_hex(token.as_bytes()),now], |_|Ok(())).optional()?.is_some())
    }

    pub fn verify_csrf(&self, token: &str, csrf: &str, now: i64) -> Result<bool, DbError> {
        Ok(self.connection()?.query_row("SELECT 1 FROM sessions WHERE token_hash=?1 AND csrf_hash=?2 AND expires_at>?3 AND revoked_at IS NULL", params![sha256_hex(token.as_bytes()),sha256_hex(csrf.as_bytes()),now], |_|Ok(())).optional()?.is_some())
    }

    pub fn revoke_session(&self, token: &str, now: i64) -> Result<(), DbError> {
        self.connection()?.execute(
            "UPDATE sessions SET revoked_at=?2 WHERE token_hash=?1",
            params![sha256_hex(token.as_bytes()), now],
        )?;
        Ok(())
    }

    /// Remove only bounded operational history. Traffic seeds, adjustments,
    /// cycle history, findings, tombstones, identities and audit events are
    /// deliberately excluded because they are durable evidence.
    pub fn prune_operational_history(&self, now: i64) -> Result<(), DbError> {
        const BATCH: i64 = 10_000;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "DELETE FROM report_messages WHERE rowid IN (SELECT rowid FROM report_messages WHERE received_at < ?1 LIMIT ?2)",
            params![now - 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM resource_rollups WHERE rowid IN (SELECT rowid FROM resource_rollups WHERE interval_end < ?1 LIMIT ?2)",
            params![now - 30 * 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM traffic_rollup WHERE rowid IN (SELECT rowid FROM traffic_rollup WHERE interval_end < ?1 LIMIT ?2)",
            params![now - 90 * 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM events WHERE id IN (SELECT id FROM events WHERE occurred_at < ?1 LIMIT ?2)",
            params![now - 180 * 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM process_summaries WHERE rowid IN (SELECT old.rowid FROM process_summaries old WHERE old.observed_at < ?1 AND old.observed_at != (SELECT MAX(latest.observed_at) FROM process_summaries latest WHERE latest.server_id=old.server_id) LIMIT ?2)",
            params![now - 7 * 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM socket_summaries WHERE rowid IN (SELECT old.rowid FROM socket_summaries old WHERE old.observed_at < ?1 AND old.observed_at != (SELECT MAX(latest.observed_at) FROM socket_summaries latest WHERE latest.server_id=old.server_id) LIMIT ?2)",
            params![now - 7 * 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM sessions WHERE rowid IN (SELECT rowid FROM sessions WHERE expires_at < ?1 OR revoked_at < ?1 LIMIT ?2)",
            params![now - 7 * 86_400, BATCH],
        )?;
        tx.execute(
            "UPDATE observation_leases SET state='expired' WHERE state='active' AND expires_at<=?1",
            [now],
        )?;
        tx.execute(
            "DELETE FROM traffic_observed_checkpoint WHERE rowid IN (SELECT old.rowid FROM traffic_observed_checkpoint old WHERE old.checkpoint_at < ?1 AND NOT EXISTS (SELECT 1 FROM traffic_seed seed JOIN billing_cycle_instance cycle ON cycle.id=seed.cycle_id WHERE cycle.server_id=old.server_id AND seed.checkpoint_at=old.checkpoint_at) AND old.checkpoint_at != (SELECT MAX(newer.checkpoint_at) FROM traffic_observed_checkpoint newer WHERE newer.server_id=old.server_id) LIMIT ?2)",
            params![now - 400 * 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM observation_leases WHERE rowid IN (SELECT rowid FROM observation_leases WHERE state!='active' AND expires_at < ?1 LIMIT ?2)",
            params![now - 30 * 86_400, BATCH],
        )?;
        tx.execute(
            "DELETE FROM enrollment_tokens WHERE rowid IN (SELECT rowid FROM enrollment_tokens WHERE expires_at < ?1 AND (used_at IS NOT NULL OR expires_at < ?2) LIMIT ?3)",
            params![now - 7 * 86_400, now - 30 * 86_400, BATCH],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn migrate(conn: &mut Connection) -> Result<(), DbError> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);")?;
    let mut version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )?;
    if version > 4 {
        return Err(DbError::Invalid(
            "database schema is newer than this Parade Hub binary",
        ));
    }
    if version < 1 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_1)?;
        tx.execute(
            "INSERT INTO schema_migrations(version,applied_at) VALUES(1,strftime('%s','now'))",
            [],
        )?;
        tx.commit()?;
        version = 1;
    }
    if version < 2 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_2)?;
        tx.execute(
            "INSERT INTO schema_migrations(version,applied_at) VALUES(2,strftime('%s','now'))",
            [],
        )?;
        tx.commit()?;
        version = 2;
    }
    if version < 3 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_3)?;
        tx.execute(
            "INSERT INTO schema_migrations(version,applied_at) VALUES(3,strftime('%s','now'))",
            [],
        )?;
        tx.commit()?;
        version = 3;
    }
    if version < 4 {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(MIGRATION_4)?;
        tx.execute(
            "INSERT INTO schema_migrations(version,applied_at) VALUES(4,strftime('%s','now'))",
            [],
        )?;
        tx.commit()?;
    }
    Ok(())
}

pub fn audit(
    conn: &Connection,
    now: i64,
    operator: &str,
    action: &str,
    server: Option<&str>,
    detail: &str,
) -> Result<(), DbError> {
    conn.execute("INSERT INTO audit_events(occurred_at,operator,action,server_id,detail_json) VALUES(?1,?2,?3,?4,?5)", params![now,operator,action,server,detail])?;
    Ok(())
}

fn validate_id(id: &str) -> Result<(), DbError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(DbError::Invalid("invalid server id"));
    }
    Ok(())
}

fn validate_interface_policy(conn: &Connection, report: &SignedReport) -> Result<bool, DbError> {
    let delivered: i64 = conn.query_row(
        "SELECT traffic_policy_version FROM agent_credentials WHERE server_id=?1 AND agent_id=?2 AND status='active'",
        params![report.server_id, report.agent_id],
        |row| row.get(0),
    )?;
    if i64::from(report.body.traffic.policy_version) != delivered {
        return Err(DbError::Invalid(
            "report traffic policy version does not match the delivered version",
        ));
    }
    let policy: Option<(i64, String)> = conn
        .query_row(
            "SELECT version,interface_policy_json FROM billing_cycle_rule WHERE server_id=?1 AND enabled=1",
            [&report.server_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((active_version, raw)) = policy else {
        return Ok(false);
    };
    if active_version < delivered {
        return Err(DbError::Invalid("stored traffic policy version regressed"));
    }
    if active_version > delivered {
        // The report was constructed before the Agent could receive this new
        // version. Accept exactly this delivered-version transition. The
        // credential cursor advances in the same ingest transaction, and an
        // exact duplicate can still retrieve the lost acknowledgement.
        return Ok(true);
    }
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|_| DbError::Invalid("invalid stored interface policy"))?;
    let mut reported: Vec<&str> = report
        .body
        .traffic
        .interfaces
        .iter()
        .filter(|item| item.selected)
        .map(|item| item.name.as_str())
        .collect();
    reported.sort_unstable();
    let mut selected: Vec<&str> = value
        .get("selected")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect();
    selected.sort_unstable();
    let excluded: Vec<&str> = value
        .get("excluded")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let manual = value.get("mode").and_then(serde_json::Value::as_str) == Some("manual");
    if (manual && reported != selected) || reported.iter().any(|name| excluded.contains(name)) {
        return Err(DbError::Invalid(
            "reported selected interfaces do not match the active policy",
        ));
    }
    Ok(false)
}

pub fn to_i64(value: u64) -> Result<i64, DbError> {
    i64::try_from(value).map_err(|_| DbError::Invalid("counter exceeds SQLite integer range"))
}

pub fn confidence(value: &Confidence) -> &'static str {
    match value {
        Confidence::High => "high",
        Confidence::Partial => "partial",
        Confidence::Estimated => "estimated",
        Confidence::Unsupported => "unsupported",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use parade_common::{
        Confidence, ObservationProfile, ProcessSummary, ResourceRollup, TelemetryReport,
        TrafficCheckpoint,
    };

    fn temp_db() -> (Database, PathBuf) {
        let path = std::env::temp_dir().join(format!("parade-db-test-{}.sqlite", random()));
        (Database::open(&path).unwrap(), path)
    }

    fn random() -> String {
        let mut bytes = [0u8; 8];
        getrandom::getrandom(&mut bytes).unwrap();
        parade_common::hex_encode(&bytes)
    }

    fn report(server: &str, agent: &str, sequence: u64, at: i64, key: &SigningKey) -> SignedReport {
        SignedReport::new(
            server.into(),
            agent.into(),
            at,
            sequence,
            parade_common::sha256_hex(format!("{server}:{agent}:{sequence}").as_bytes()),
            TelemetryReport {
                agent_version: "test".into(),
                profile: ObservationProfile::Normal,
                uptime_secs: 1,
                os: "Synthetic Linux".into(),
                kernel: "6.12".into(),
                arch: "x86_64".into(),
                inventory_hash: "f".repeat(64),
                lease_id: None,
                resources: ResourceRollup {
                    interval_start: at - 300,
                    interval_end: at,
                    samples: 30,
                    cpu_cores: 2,
                    mem_total: 1024,
                    mem_used: 512,
                    ..Default::default()
                },
                traffic: TrafficCheckpoint {
                    observed_rx: sequence * 1_000,
                    observed_tx: sequence * 500,
                    boot_id: "boot".into(),
                    sampled_at: at,
                    confidence: Confidence::High,
                    ..Default::default()
                },
                processes: None,
                listeners: None,
                coverage: vec![],
            },
            key,
        )
        .unwrap()
    }

    #[test]
    fn migrations_are_idempotent_and_enable_wal_and_foreign_keys() {
        let (db, path) = temp_db();
        let db2 = Database::open(&path).unwrap();
        let conn = db2.connection().unwrap();
        assert_eq!(
            conn.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert_eq!(
            conn.query_row("PRAGMA foreign_keys", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(conn);
        drop(db);
        drop(db2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn finding_subject_churn_is_bounded_with_an_overflow_series() {
        let (db, path) = temp_db();
        let conn = db.connection().unwrap();
        conn.execute(
            "INSERT INTO servers(id,name,status,created_at) VALUES('bounded','Bounded','active',1)",
            [],
        )
        .unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        for index in 0..40 {
            let mut value = report("bounded", "agent", index + 1, index as i64 + 10, &key);
            value.body.processes = Some(vec![ProcessSummary {
                executable: format!("/tmp/churn-{index}"),
                suspicious_writable_path: true,
                ..Default::default()
            }]);
            crate::findings::evaluate(&conn, &value, index as i64 + 10).unwrap();
        }
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM security_findings WHERE server_id='bounded' AND rule_id='PROC_WRITABLE_EXEC'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            33
        );
        assert_eq!(
            conn.query_row(
                "SELECT occurrence_count FROM security_findings WHERE server_id='bounded' AND rule_id='PROC_WRITABLE_EXEC' AND series_key='overflow'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            8
        );
        let mut cleared = report("bounded", "agent", 41, 100, &key);
        cleared.body.processes = Some(vec![]);
        crate::findings::evaluate(&conn, &cleared, 100).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM security_findings WHERE server_id='bounded' AND rule_id='PROC_WRITABLE_EXEC' AND state='active'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            0
        );
        drop(conn);
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn newer_database_schema_fails_closed() {
        let (db, path) = temp_db();
        db.connection()
            .unwrap()
            .execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES(5,1)",
                [],
            )
            .unwrap();
        drop(db);
        assert!(matches!(Database::open(&path), Err(DbError::Invalid(_))));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn migration_four_preserves_legacy_combined_accounting() {
        let path = std::env::temp_dir().join(format!("parade-db-v3-{}.sqlite", random()));
        let mut conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);")
            .unwrap();
        for (version, sql) in [(1, MIGRATION_1), (2, MIGRATION_2), (3, MIGRATION_3)] {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(sql).unwrap();
            tx.execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES(?1,1)",
                [version],
            )
            .unwrap();
            tx.commit().unwrap();
        }
        conn.execute(
            "INSERT INTO servers(id,name,status,created_at) VALUES('legacy','Legacy','active',1)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO billing_cycle_rule(server_id,timezone,anchor_day,anchor_time,interface_policy_json,traffic_limit_bytes,enabled,version,updated_at,updated_by) VALUES('legacy','UTC',1,'00:00','{\"mode\":\"auto\"}',1000,1,1,1,'admin')", []).unwrap();
        conn.execute("INSERT INTO billing_cycle_instance(id,server_id,start_at,end_at,starting_observed_rx,starting_observed_tx,state,confidence,created_at) VALUES(1,'legacy',100,1000,10,20,'open','high',100)", []).unwrap();
        conn.execute("INSERT INTO traffic_observed_checkpoint(server_id,observed_rx,observed_tx,checkpoint_at,agent_sequence,confidence,last_boot_id) VALUES('legacy',10,20,200,1,'high','boot')", []).unwrap();
        conn.execute("INSERT INTO traffic_seed(cycle_id,rx_bytes,tx_bytes,combined_bytes,effective_at,checkpoint_at,observed_rx_at_seed,observed_tx_at_seed,operator,note,created_at) VALUES(1,0,0,100,200,200,10,20,'admin','legacy seed',201)", []).unwrap();
        conn.execute("INSERT INTO traffic_adjustment(cycle_id,signed_bytes,effective_at,reason,operator,created_at) VALUES(1,-10,200,'legacy correction','admin',202)", []).unwrap();
        conn.execute_batch("INSERT INTO security_findings(server_id,rule_id,rule_version,severity,confidence,first_seen,last_seen,occurrence_count,evidence,explanation,verification) VALUES('legacy','RESOURCE_SUSTAINED_CPU',1,'review','medium',100,200,2,'first','why','verify'); INSERT INTO security_findings(server_id,rule_id,rule_version,severity,confidence,first_seen,last_seen,occurrence_count,evidence,explanation,verification) VALUES('legacy','RESOURCE_SUSTAINED_CPU',1,'review','medium',300,400,3,'latest','why','verify');").unwrap();
        drop(conn);

        let db = Database::open(&path).unwrap();
        let usage = db.traffic_usage("legacy").unwrap();
        assert_eq!(usage.billing_mode, crate::traffic::TrafficBillingMode::Sum);
        assert_eq!(usage.seed_bytes, 100);
        assert_eq!(usage.adjustment_bytes, -10);
        assert_eq!(usage.total_bytes, 90);
        assert!(!usage.directional_seed_known);
        let preserved_findings: (i64, i64) = db
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*),SUM(occurrence_count) FROM security_findings WHERE server_id='legacy'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(preserved_findings, (2, 5));
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn retention_is_batched_and_checkpoint_seed_matching_is_server_scoped() {
        let (db, path) = temp_db();
        let now = 50_000_000;
        let conn = db.connection().unwrap();
        conn.execute_batch(
            "WITH RECURSIVE n(value) AS (SELECT 1 UNION ALL SELECT value+1 FROM n WHERE value<=10000) INSERT INTO report_messages(message_id,server_id,agent_id,sequence,received_at) SELECT printf('old-%d',value),'s','a',value,1 FROM n;",
        )
        .unwrap();
        for server in ["seeded", "other"] {
            conn.execute(
                "INSERT INTO servers(id,name,status,created_at) VALUES(?1,?1,'active',1)",
                [server],
            )
            .unwrap();
            conn.execute("INSERT INTO billing_cycle_rule(server_id,timezone,anchor_day,anchor_time,interface_policy_json,traffic_limit_bytes,enabled,version,updated_at,updated_by,billing_mode) VALUES(?1,'UTC',1,'00:00','{\"mode\":\"auto\"}',NULL,1,1,1,'admin','sum')", [server]).unwrap();
            conn.execute("INSERT INTO billing_cycle_instance(server_id,start_at,end_at,starting_observed_rx,starting_observed_tx,state,confidence,created_at) VALUES(?1,1,99999999,0,0,'open','high',1)", [server]).unwrap();
            conn.execute("INSERT INTO traffic_observed_checkpoint(server_id,observed_rx,observed_tx,checkpoint_at,agent_sequence,confidence,last_boot_id) VALUES(?1,1,1,10,1,'high','boot')", [server]).unwrap();
            conn.execute("INSERT INTO traffic_observed_checkpoint(server_id,observed_rx,observed_tx,checkpoint_at,agent_sequence,confidence,last_boot_id) VALUES(?1,2,2,20,2,'high','boot')", [server]).unwrap();
        }
        let seeded_cycle: i64 = conn
            .query_row(
                "SELECT id FROM billing_cycle_instance WHERE server_id='seeded'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        conn.execute("INSERT INTO traffic_seed(cycle_id,rx_bytes,tx_bytes,combined_bytes,effective_at,checkpoint_at,observed_rx_at_seed,observed_tx_at_seed,operator,note,created_at,directional_seed) VALUES(?1,0,0,5,10,10,1,1,'admin','seed',1,0)", [seeded_cycle]).unwrap();
        drop(conn);

        db.prune_operational_history(now).unwrap();
        let conn = db.connection().unwrap();
        let first_remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM report_messages", [], |row| row.get(0))
            .unwrap();
        assert_eq!(first_remaining, 1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM traffic_observed_checkpoint WHERE server_id='seeded' AND checkpoint_at=10", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM traffic_observed_checkpoint WHERE server_id='other' AND checkpoint_at=10", [], |row| row.get::<_,i64>(0)).unwrap(), 0);
        drop(conn);
        db.prune_operational_history(now).unwrap();
        assert_eq!(
            db.connection()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM report_messages", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn enrollment_is_single_use_bound_and_tombstones_are_durable() {
        let (db, path) = temp_db();
        db.create_server("a", "A", "", 1, "admin").unwrap();
        db.create_server("b", "B", "", 1, "admin").unwrap();
        db.mint_enrollment("a", "secret", 100, 2, "admin").unwrap();
        let key = parade_common::hex_encode(&[8; 32]);
        assert_eq!(db.enroll_agent("secret", &key, "agent-a", 3).unwrap(), "a");
        assert!(matches!(
            db.enroll_agent("secret", &key, "agent-b", 4),
            Err(DbError::Unauthorized)
        ));
        assert!(db.public_key_for("b", "agent-a").is_err());
        db.tombstone_server("a", 5, "admin", "retired host")
            .unwrap();
        assert!(matches!(
            db.create_server("a", "A2", "", 6, "admin"),
            Err(DbError::Conflict(_))
        ));
        drop(db);
        let reopened = Database::open(&path).unwrap();
        assert!(reopened.public_key_for("a", "agent-a").is_err());
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn report_binding_replay_state_findings_and_identity_survive_restart() {
        let (db, path) = temp_db();
        db.create_server("a", "A", "", 1, "admin").unwrap();
        db.create_server("b", "B", "", 1, "admin").unwrap();
        let key_a = SigningKey::from_bytes(&[8; 32]);
        let key_b = SigningKey::from_bytes(&[9; 32]);
        db.mint_enrollment("a", "token-a", 100, 2, "admin").unwrap();
        db.mint_enrollment("b", "token-b", 100, 2, "admin").unwrap();
        let agent_a = "agent-a";
        let agent_b = "agent-b";
        db.enroll_agent(
            "token-a",
            &parade_common::hex_encode(key_a.verifying_key().as_bytes()),
            agent_a,
            3,
        )
        .unwrap();
        db.enroll_agent(
            "token-b",
            &parade_common::hex_encode(key_b.verifying_key().as_bytes()),
            agent_b,
            3,
        )
        .unwrap();
        assert!(db.public_key_for("b", agent_a).is_err());

        let mut body = report("a", agent_a, 1, 10, &key_a).body;
        body.processes = Some(vec![ProcessSummary {
            executable: "/tmp/unowned-worker".into(),
            suspicious_writable_path: true,
            ..Default::default()
        }]);
        let accepted = SignedReport::new(
            "a".into(),
            agent_a.into(),
            10,
            1,
            parade_common::sha256_hex(b"a:agent-a:1"),
            body,
            &key_a,
        )
        .unwrap();
        accepted
            .verify(&db.public_key_for("a", agent_a).unwrap())
            .unwrap();
        db.ingest_verified(&accepted, "192.0.2.1", 10, 256).unwrap();
        assert!(matches!(
            db.ingest_verified(&accepted, "192.0.2.1", 11, 256),
            Err(DbError::Duplicate)
        ));
        drop(db);

        let reopened = Database::open(&path).unwrap();
        assert!(matches!(
            reopened.ingest_verified(&accepted, "192.0.2.1", 12, 256),
            Err(DbError::Duplicate)
        ));
        let mut recurrence_body = report("a", agent_a, 2, 20, &key_a).body;
        // The first accepted report delivered billing policy v1. A restarted
        // Agent must persist and echo that cursor; replay durability must not
        // be tested by bypassing the policy-version invariant.
        recurrence_body.traffic.policy_version = 1;
        recurrence_body.processes = Some(vec![ProcessSummary {
            pid: 42,
            executable: "/tmp/unowned-worker".into(),
            suspicious_writable_path: true,
            ..Default::default()
        }]);
        let recurrence = SignedReport::new(
            "a".into(),
            agent_a.into(),
            20,
            2,
            parade_common::sha256_hex(b"a:agent-a:2"),
            recurrence_body,
            &key_a,
        )
        .unwrap();
        reopened
            .ingest_verified(&recurrence, "192.0.2.1", 20, 256)
            .unwrap();
        assert_eq!(
            reopened
                .connection()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM resource_rollups", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        let conn = reopened.connection().unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM security_findings", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        let (occurrences, evidence): (i64, String) = conn
            .query_row(
                "SELECT occurrence_count,evidence FROM security_findings WHERE server_id='a' AND rule_id='PROC_WRITABLE_EXEC'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(occurrences, 2);
        assert!(evidence.contains("pid=42"));
        drop(conn);
        reopened
            .tombstone_server("a", 13, "admin", "retired host")
            .unwrap();
        assert!(reopened.public_key_for("a", agent_a).is_err());
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn leased_responses_are_profile_bound_and_byte_accounted() {
        let (db, path) = temp_db();
        let key = SigningKey::from_bytes(&[7; 32]);
        db.create_server("leased", "Leased", "", 1, "admin")
            .unwrap();
        db.mint_enrollment("leased", "lease-token", 100, 2, "admin")
            .unwrap();
        db.enroll_agent(
            "lease-token",
            &parade_common::hex_encode(key.verifying_key().as_bytes()),
            "lease-agent",
            3,
        )
        .unwrap();
        db.connection()
            .unwrap()
            .execute(
                "INSERT INTO observation_leases(lease_id,server_id,profile_json,issued_at,expires_at,state,issued_by) VALUES('abcd','leased','\"process_snapshot\"',5,100,'active','admin')",
                [],
            )
            .unwrap();

        let mut leased = report("leased", "lease-agent", 1, 10, &key);
        leased.body.profile = ObservationProfile::ProcessSnapshot;
        leased.body.lease_id = Some("abcd".into());
        db.ingest_verified(&leased, "192.0.2.1", 10, 333).unwrap();
        let conn = db.connection().unwrap();
        let accounting: (i64, i64, Option<String>, i64) = conn
            .query_row(
                "SELECT l.response_count,l.encoded_response_bytes,m.lease_id,m.encoded_bytes FROM observation_leases l JOIN report_messages m ON m.lease_id=l.lease_id WHERE l.lease_id='abcd'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(accounting, (1, 333, Some("abcd".into()), 333));
        drop(conn);

        let mut mismatch = report("leased", "lease-agent", 2, 20, &key);
        mismatch.body.profile = ObservationProfile::SocketSnapshot;
        mismatch.body.lease_id = Some("abcd".into());
        assert!(matches!(
            db.ingest_verified(&mismatch, "192.0.2.1", 20, 444),
            Err(DbError::Invalid(_))
        ));
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn traffic_policy_delivery_accepts_one_transition_then_enforces_version() {
        let (db, path) = temp_db();
        let key = SigningKey::from_bytes(&[6; 32]);
        db.create_server("policy", "Policy", "", 1, "admin")
            .unwrap();
        db.mint_enrollment("policy", "policy-token", 100, 2, "admin")
            .unwrap();
        db.enroll_agent(
            "policy-token",
            &parade_common::hex_encode(key.verifying_key().as_bytes()),
            "policy-agent",
            3,
        )
        .unwrap();
        let first = report("policy", "policy-agent", 1, 10, &key);
        db.ingest_verified(&first, "192.0.2.1", 10, 256).unwrap();
        db.set_cycle_rule(
            "policy",
            &crate::traffic::CycleRuleInput {
                timezone: "UTC".into(),
                anchor_day: 1,
                anchor_time: "00:00".into(),
                selected_interfaces: vec![],
                excluded_interfaces: vec!["wg0".into()],
                traffic_limit_bytes: None,
                billing_mode: crate::traffic::TrafficBillingMode::Sum,
                rx_limit_bytes: None,
                tx_limit_bytes: None,
            },
            11,
            "admin",
        )
        .unwrap();

        let mut transitioning = report("policy", "policy-agent", 2, 20, &key);
        transitioning.body.traffic.policy_version = 1;
        db.ingest_verified(&transitioning, "192.0.2.1", 20, 257)
            .unwrap();
        assert!(matches!(
            db.ingest_verified(&transitioning, "192.0.2.1", 21, 257),
            Err(DbError::Duplicate)
        ));

        let mut stale = report("policy", "policy-agent", 3, 30, &key);
        stale.body.traffic.policy_version = 1;
        assert!(matches!(
            db.ingest_verified(&stale, "192.0.2.1", 30, 258),
            Err(DbError::Invalid(_))
        ));
        stale.body.traffic.policy_version = 2;
        db.ingest_verified(&stale, "192.0.2.1", 30, 258).unwrap();
        let delivered: i64 = db
            .connection()
            .unwrap()
            .query_row(
                "SELECT traffic_policy_version FROM agent_credentials WHERE agent_id='policy-agent'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(delivered, 2);
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn synthetic_fleet_load_1000_agents_is_bounded() {
        const AGENTS: usize = 1_000;
        let (db, path) = temp_db();
        let start = std::time::Instant::now();
        {
            let mut conn = db.connection().unwrap();
            let tx = conn.transaction().unwrap();
            for index in 0..AGENTS {
                let server = format!("load-{index:04}");
                let agent = format!("agent-{index:04}");
                let mut seed = [0u8; 32];
                seed[..8].copy_from_slice(&(index as u64 + 1).to_le_bytes());
                let key = SigningKey::from_bytes(&seed);
                tx.execute(
                    "INSERT INTO servers(id,name,status,created_at) VALUES(?1,?1,'active',1)",
                    [&server],
                )
                .unwrap();
                tx.execute("INSERT INTO agent_credentials(agent_id,server_id,public_key_hex,status,created_at) VALUES(?1,?2,?3,'active',1)",params![agent,server,parade_common::hex_encode(key.verifying_key().as_bytes())]).unwrap();
            }
            tx.commit().unwrap();
        }
        let setup_elapsed = start.elapsed();
        let ingest_start = std::time::Instant::now();
        for index in 0..AGENTS {
            let server = format!("load-{index:04}");
            let agent = format!("agent-{index:04}");
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&(index as u64 + 1).to_le_bytes());
            let key = SigningKey::from_bytes(&seed);
            let value = report(&server, &agent, 1, 2_000_000_000, &key);
            value
                .verify(&parade_common::hex_encode(key.verifying_key().as_bytes()))
                .unwrap();
            db.ingest_verified(&value, "192.0.2.1", 2_000_000_000, 256)
                .unwrap();
        }
        let ingest_elapsed = ingest_start.elapsed();
        let query_start = std::time::Instant::now();
        let count: i64 = db
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM servers WHERE status!='deleted'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let query_elapsed = query_start.elapsed();
        assert_eq!(count, AGENTS as i64);
        let db_bytes = std::fs::metadata(&path).unwrap().len();
        assert!(db_bytes < 64 * 1024 * 1024, "database grew to {db_bytes}");
        assert!(
            ingest_elapsed < std::time::Duration::from_secs(30),
            "synthetic ingestion took {ingest_elapsed:?}"
        );
        eprintln!(
            "SYNTHETIC_FLEET agents={AGENTS} setup_ms={} ingest_ms={} reports_per_sec={:.1} fleet_count_query_ms={:.3} db_bytes={db_bytes}",
            setup_elapsed.as_millis(),
            ingest_elapsed.as_millis(),
            AGENTS as f64 / ingest_elapsed.as_secs_f64(),
            query_elapsed.as_secs_f64() * 1000.0,
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
