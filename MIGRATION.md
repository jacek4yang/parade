# Migration and upgrade policy

## Current schema

The Hub owns one SQLite database in WAL mode. On startup it creates
`schema_migrations`, applies each pending migration inside an immediate
transaction, enables foreign keys, and refuses to listen if configuration or
migration fails. Migration 1 is the new security boundary: independent Agent
identities, durable replay protection, tombstones, sessions/audit, findings,
typed leases, resource/process/socket records, and checkpoint-based traffic
cycles.

Migration 2 adds observation-lease response linkage, encoded-byte accounting,
cancellation/expiry timestamps, and a lease lookup index on accepted report
messages. Migration 3 adds the per-credential delivered interface-policy
version used to make acknowledgement loss and policy changes idempotent. All
migrations are transactional and checked before the listener starts.

Wire protocol version 2 adds the traffic-policy version to every signed
checkpoint. Protocol-1 Agents from an intermediate development build are not
wire-compatible and must be upgraded/re-enrolled together with this Hub. The
Hub rejects unsupported versions rather than guessing field layouts.

Before every upgrade:

1. Stop the Hub or take a consistent SQLite online backup.
2. Copy the Hub configuration and release manifest separately.
3. Record `parade-hub --version` and `SELECT MAX(version) FROM schema_migrations`.
4. Start the new binary and review logs before exposing it through the proxy.
5. Test login, one Fleet query, a signed Agent report, and a traffic cycle view.

Binary rollback is supported only while the older binary understands the
database's current schema. Schema down-migrations are not promised. Restore the
pre-upgrade backup for a true rollback; do not manually decrement migration
versions.

## Boundary from the delivered JSON implementation

There is intentionally no automatic importer for the legacy mutable JSON state.
That format used a fleet bearer token, lacked per-Agent keys and replay cursors,
and stored mutable month totals without checkpoint-tied manual seeds. Importing
those values as if they satisfied the new invariants would manufacture false
identity and accounting confidence.

Safe migration is:

1. Preserve the old JSON file offline for historical reference.
2. Deploy a fresh SQLite Hub and pre-create each server record.
3. Re-enroll every Agent with its independent one-use command.
4. Wait for the first reliable monotonic traffic checkpoint.
5. Read current-cycle used bytes from the provider dashboard and enter that as
   the new immutable seed at the displayed checkpoint.
6. Keep old monthly values clearly labeled as legacy/unverified external
   records; do not inject them into current cycle arithmetic.

Deleted legacy IDs should be tombstoned explicitly before enrollment commands
are issued. There is no unknown-server auto-registration fallback.

## Backup and restore

For a stopped Hub, copy `parade.sqlite3` and any `-wal`/`-shm` files as one set.
For an online Hub use SQLite `.backup` or an equivalent API; copying only the
main file while WAL is active is not a backup. Restore into a private test Hub,
check integrity/migration version and authentication, then switch production.
Agent identities and replay positions are database-bound, so restoring an old
backup may cause newer Agent sequences to be accepted only if its replay cursor
is older; investigate the rollback window and rotate credentials if integrity
is uncertain.
