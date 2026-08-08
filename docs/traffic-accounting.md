# Manual provider traffic accounting

English | [简体中文](traffic-accounting.zh-CN.md)

Provider APIs are intentionally outside this milestone. Parade lets an operator
enter the provider dashboard's current-cycle usage once, binds it to an exact
Agent checkpoint, and then adds later locally observed traffic.

```text
immutable provider seed at checkpoint
+ selected-interface traffic after checkpoint
+ append-only audited corrections
= current-cycle usage
```

Linux counters cannot reconstruct traffic from before observation started.
Parade preserves that limitation instead of silently inventing a value.

## Five closed billing modes

| Mode | Provider seed | Parade result | Limit |
| --- | --- | --- | --- |
| Inbound + outbound | one combined value | seed + later RX + later TX | one total limit |
| Inbound only | inbound value | inbound seed + later RX | one inbound limit |
| Outbound only | outbound value | outbound seed + later TX | one outbound limit |
| Larger direction | both inbound and outbound | `max(current RX, current TX)` | one total limit |
| Separate directions | both inbound and outbound | independent RX and TX totals | independent RX/TX limits |

Larger-direction and separate-direction modes reject a combined seed. For
example, if RX starts at 100, TX at 90, and later TX grows by 20, the correct
larger-direction result is 110. Adding a combined seed to only the larger delta
would incorrectly produce 120.

The mode is a versioned enum. There is no custom formula, script, provider code,
SQL, path, command, or plugin input.

## Configure a server

Before the first seed, set:

- an IANA timezone such as `Asia/Shanghai`;
- a billing day from 1 through 31;
- the local boundary time;
- the billing mode and optional limit(s); and
- an automatic or explicit interface-selection policy.

Days 29–31 clamp to the final day of shorter months. Daylight-saving gaps and
ambiguities are handled deterministically in the configured timezone. Once
history is seeded, timezone, anchor and billing mode are frozen so old cycles
cannot be reinterpreted. Interface policy and limits remain editable and are
audited.

Automatic interface selection follows IPv4 and IPv6 default routes and excludes
loopback, bridge, veth/container and tunnel-like devices. The UI shows the
interfaces the Agent actually selected, plus anomalies and partial/estimated
reasons. If the intended provider scope differs, choose an explicit allowlist.

## Record the current-cycle seed

1. Wait for the first reliable signed checkpoint.
2. Read the provider dashboard at approximately the same time.
3. Enter the current-cycle value(s), unit and a useful source note.
4. Review the exact checkpoint, effective time, cycle and preview formula.
5. Confirm once.

The primary seed is immutable and only one is accepted for a cycle. It records
the operator, creation/effective times, source note and checkpoint. Do not use a
seed to correct a later mistake.

## Correct a value

Add a positive or negative adjustment with a reason. Adjustments are append-only
audit records; existing seeds and adjustments are never edited in place.
Separate-direction mode requires the adjustment direction. Review the component
breakdown and audit history after saving.

## Automatic monthly rollover

At the configured local boundary, the Hub creates the next cycle with a zero
seed. It does not write, reset, wrap, or otherwise alter Linux counters. Closed
cycle history remains available.

If two trustworthy checkpoints surround the boundary, Parade can refine the
split. If the Agent was offline or a counter reset/boot/rename makes the split
impossible, the affected interval remains visibly `estimated` or `partial`.
An operator may later add an audited provider correction, but Parade does not
silently upgrade uncertain local evidence to exact.

## Understand differences from the provider

Provider totals may include link-layer overhead, rounding, private traffic,
virtual-switch traffic, direction weighting, shared NAT gateway use, or another
policy that Linux interface counters do not reproduce. Under shared NAT, one
Agent cannot attribute other machines' traffic to itself.

The Traffic page therefore exposes:

- provider seed and source note;
- seed checkpoint/effective time;
- selected interfaces and observed RX/TX components;
- every correction and operator;
- cycle boundaries and closed history; and
- confidence, anomalies, gaps and explicit uncertainty.

Treat Parade as a transparent reconciliation ledger, not as a replacement for
the provider's billing system.

## Related references

- Normative model and edge cases: [TRAFFIC_ACCOUNTING_SPEC.md](../TRAFFIC_ACCOUNTING_SPEC.md)
- Full operator lifecycle: [operations.md](operations.md)
- Resource retention: [resource-budgets.md](resource-budgets.md)
- Safe diagnosis: [troubleshooting.md](troubleshooting.md)
