# TRAFFIC_ACCOUNTING_SPEC.md

## Purpose

This document defines the required traffic-accounting behavior for Parade.

The primary workflow is:

1. The operator enrolls a VPS.
2. The operator enters the provider dashboard's current traffic usage for the active billing cycle.
3. Parade stores that value as a manual seed.
4. Parade adds network traffic observed after the seed.
5. At the configured billing-cycle boundary, Parade starts a new cycle automatically.
6. The operator may apply audited corrections when necessary.

Provider API integration is explicitly optional and out of scope for the current milestone.

## Terminology

### Raw interface counter

The cumulative receive or transmit byte counter exposed by Linux for one interface.

### Selected traffic interfaces

The interfaces whose counters are treated as billable external traffic.

Default auto-detection should prefer interfaces associated with default routes and exclude obvious non-billable or duplicate-counting interfaces such as:

- loopback;
- Docker/veth pairs;
- Linux bridges;
- container-only interfaces;
- local tunnels;
- explicitly excluded interfaces.

The UI must show the selected interfaces and allow an operator to override the selection.

### Agent observed total

A monotonically increasing Parade-maintained total derived from selected Linux counters across:

- Agent restarts;
- Hub restarts;
- host reboots;
- raw counter resets;
- interface replacement or rename where identity can be inferred.

This total must never decrease.

### Billing cycle

A recurring time interval configured by:

- IANA timezone;
- anchor day from 1 through 31;
- anchor local time.

For months without the requested day, use the last valid day of that month. Document and test daylight-saving transitions even though most VPS billing timezones are UTC.

### Manual seed

An operator-entered byte count representing traffic already used in the current provider billing cycle at an exact effective timestamp.

### Adjustment

An append-only signed correction to a cycle. Adjustments may be positive or negative but must not make the displayed usage negative. Every adjustment requires a reason and audit record.

## Required data model

Use transactional storage. Names may differ, but the model must preserve equivalent semantics.

### `traffic_interface_state`

Per server and interface identity:

- server ID;
- interface name;
- stable identity attributes where available;
- boot ID;
- last raw RX bytes;
- last raw TX bytes;
- last sample timestamp;
- active/excluded state;
- reset/rename metadata.

### `traffic_observed_checkpoint`

Per server:

- monotonically increasing observed RX total;
- monotonically increasing observed TX total;
- checkpoint timestamp;
- Agent sequence;
- confidence/status;
- last boot ID.

### `billing_cycle_rule`

Per server:

- timezone;
- anchor day;
- anchor time;
- selected interface policy;
- traffic limit;
- enabled state;
- version;
- created/updated audit metadata.

### `billing_cycle_instance`

Per server and concrete cycle:

- cycle ID;
- start instant;
- end instant;
- starting Agent-observed total;
- ending Agent-observed total when closed;
- state: open/closed/estimated;
- confidence;
- creation and closure metadata.

### `traffic_seed`

- cycle ID;
- manually supplied used RX, TX, or combined bytes;
- effective timestamp;
- Agent-observed total at or nearest before the effective timestamp;
- operator identity;
- reason/source note;
- immutable audit metadata.

Only one active primary seed should normally exist per cycle. Replacing a mistake should create a reversal/adjustment, not silently delete history.

### `traffic_adjustment`

- cycle ID;
- signed byte amount;
- effective timestamp;
- reason;
- operator;
- immutable audit metadata.

### `traffic_rollup`

- server ID;
- interval start/end;
- observed RX/TX delta;
- selected-interface details;
- confidence;
- anomaly flags.

Retain detailed rollups for a bounded period and downsample older data.

## Core calculation

For a cycle with a manual seed:

```text
displayed_cycle_usage =
    max(
        0,
        manual_seed_bytes
        + observed_bytes_after_seed
        + sum(audited_adjustments)
    )
```

Where:

```text
observed_bytes_after_seed =
    current_agent_observed_total
    - agent_observed_total_at_seed
```

The seed and observed checkpoint must be tied to explicit timestamps.

The UI must show the components separately:

- manually entered starting usage;
- traffic observed by Parade since that entry;
- adjustments;
- resulting cycle total;
- timestamp and source;
- confidence/uncertainty.

Do not show only a single unexplained number.

## Raw-counter accumulation algorithm

Persist Agent-side state atomically.

For each selected interface sample:

### Same boot and non-decreasing counter

```text
delta = current_raw - previous_raw
```

Add the delta to the monotonic observed total.

### Same boot but counter decreased

Treat this as a counter reset, interface recreation, or data discontinuity.

- never subtract;
- close the old counter segment;
- begin a new segment;
- add only safely attributable new bytes;
- create an anomaly flag;
- reduce confidence until a stable segment exists.

### Boot ID changed

Linux interface counters restart after reboot.

When previous persistent Agent state exists:

- close the previous boot segment;
- begin a new segment;
- count current raw bytes as traffic accumulated since the new boot;
- persist the new boot ID;
- never double count the old segment.

### Agent restart without host reboot

Use the persisted last raw values and current raw values to recover traffic that occurred while the Agent process was not running, provided counters and interface identity remain continuous.

### Hub restart or outage

The Agent's monotonic observed total and local checkpoints must remain authoritative enough that Hub downtime does not lose traffic.

Reports should include the monotonic total and a sequence number. The Hub must idempotently accept new checkpoints and reject duplicate/replayed ones.

### Interface rename

Attempt to match interfaces using stable attributes where available, such as:

- interface index with boot context;
- MAC address;
- default-route identity;
- device path/type.

Do not automatically merge interfaces when the match is ambiguous. Show a coverage warning and require operator review.

### Interface added or removed

- New selected interfaces start a new counter segment.
- Removed interfaces retain their accumulated historical contribution.
- Avoid counting both a bridge and its member interface by default.

### Counter wrap

Support counter-width-aware wrap handling where relevant. Modern 64-bit counters should not be assumed to wrap during ordinary operation, but tests must cover the logic.

## Billing-cycle rollover

The Agent and Hub must agree on exact cycle boundaries.

Preferred behavior:

- The Hub stores the canonical cycle rule.
- The Agent receives the normalized next boundary.
- The Agent maintains local rollups and can cross a billing boundary while the Hub is unavailable.
- At the boundary, the old cycle is closed and a new cycle begins with a default manual seed of zero.
- Existing Linux counters are not reset.
- The new cycle records the current Agent-observed total as its starting checkpoint.

The operator may later enter a non-zero manual seed for the new cycle if the provider dashboard already reports usage not captured by Parade.

## Accuracy limitations

The implementation must be honest about cases that are not mathematically reconstructable.

### Installation after cycle start

Traffic before Agent observation cannot be derived from Linux counters with certainty after arbitrary resets/reboots. The manual seed solves this by importing the provider dashboard's current used value.

### Agent stopped across a billing boundary

If the Agent process is stopped while the host remains active and raw counters remain continuous, it may recover total bytes, but it may not be able to split them exactly between the old and new cycle.

Required behavior:

- never pretend the split is exact;
- mark affected cycles as estimated;
- use the narrowest available local rollup/checkpoint interval;
- provide an audited manual correction mechanism.

### Counter discontinuity across the boundary

If a reboot, interface recreation, or counter reset occurs while no sufficient checkpoint exists, mark the affected interval as uncertain.

### Provider accounting differences

Provider-reported traffic may differ because of:

- layer-2/layer-3 overhead;
- inbound/outbound weighting;
- excluded/private traffic;
- provider-side rounding;
- delayed accounting;
- multiple interfaces or addresses.

The UI must state that Parade measures selected Linux interface bytes and may differ from provider billing semantics.

## Manual seed workflow

The server Traffic page must provide a form containing:

- current provider-used amount;
- unit selector using binary and decimal units clearly;
- effective timestamp, defaulting to the latest Agent report time;
- billing-cycle timezone;
- cycle anchor day/time;
- monthly traffic limit;
- selected interfaces;
- optional note.

Before saving, show a confirmation preview:

```text
Provider usage entered: 123.40 GiB
Parade observed checkpoint: 8.12 TiB total at 2026-08-01 17:20:00 UTC
Current cycle: 2026-07-15 00:00 UTC – 2026-08-15 00:00 UTC
Result after saving: 123.40 GiB + future observed traffic
```

Saving the seed must create an audit event.

## Reset behavior

“Reset” must never reset Linux counters.

Provide these distinct operations:

### Start a new scheduled cycle

Automatic at the configured boundary.

### Correct current usage

Creates an audited adjustment or seed correction.

### Reinitialize tracking

A rare administrative operation that starts a new accounting epoch. It must require explicit confirmation and preserve old history.

Avoid ambiguous buttons named only “Reset traffic”.

## Presentation

Show:

- current cycle usage;
- traffic limit and percentage;
- projected end-of-cycle usage;
- inbound/outbound breakdown where useful;
- seed contribution;
- observed contribution;
- adjustment contribution;
- cycle start/end;
- observation start;
- last update;
- selected interfaces;
- data confidence;
- uncertainty explanation.

Use IEC units consistently for binary counters unless the operator selects decimal provider units. Store bytes internally.

## Retention

Suggested defaults:

- 10-second local samples: memory only or very short bounded spool;
- 5-minute rollups: 90 days;
- hourly rollups: 2 years;
- cycle totals, seeds, adjustments, and audit records: retained indefinitely unless explicitly exported/deleted by a Hub administrator;
- retention jobs must be transactional and bounded.

## Required tests

At minimum:

1. Seed 100 GiB, observed delta 5 GiB, result 105 GiB.
2. Positive and negative audited adjustments.
3. Hub restart does not change usage.
4. Agent restart on same boot recovers counter delta.
5. Host reboot starts a new segment without losing prior accumulation.
6. Raw counter decrease never creates negative usage.
7. Interface rename with confident identity match.
8. Ambiguous interface replacement produces a warning.
9. Bridge/member interfaces are not double counted by default.
10. Monthly rollover at ordinary month boundary.
11. Anchor day 31 in a shorter month.
12. UTC and daylight-saving timezone cases.
13. Agent/Hub outage across a cycle boundary.
14. Duplicate/replayed report is idempotently rejected.
15. Seed effective timestamp uses the correct checkpoint.
16. Manual correction preserves immutable history.
17. Traffic projection handles insufficient history.
18. Database migration preserves prior totals.
19. Concurrent reports cannot double count.
20. Property tests: the Agent-observed total and displayed cycle usage never decrease except through an explicit negative audited adjustment, and never become negative.
