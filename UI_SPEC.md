# UI_SPEC.md

## Goal

Create a polished, compact operations console for inspecting many Linux VPSes.

The UI is not a generic hosting control panel. It is a read-only observability and security-posture console.

The visual design should communicate:

- calm;
- precision;
- hierarchy;
- trustworthy evidence;
- low operational friction.

Avoid decorative cyberpunk styling, excessive gradients, glass effects, glowing charts, animated backgrounds, or fake terminal aesthetics.

## Technology constraints

Preferred frontend:

- Preact;
- TypeScript;
- Vite;
- a small charting library such as uPlot;
- CSS variables and component-scoped styles;
- embedded production assets served by the Rust Hub.

Requirements:

- no CDN;
- no external font;
- no analytics;
- no tracker;
- no runtime dependency on third-party web services;
- no heavyweight component framework unless clearly justified;
- production JS and CSS should remain small and measured;
- all routes should remain usable on slow links.

## Visual system

### Typography

Use a high-quality system font stack.

Suggested:

```css
font-family:
  Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont,
  "Segoe UI", sans-serif;
```

Do not fetch Inter externally. Use local/system availability only.

Use a monospace system stack for:

- IP addresses;
- process IDs;
- ports;
- byte values;
- hashes;
- timestamps where alignment matters.

### Color

Use semantic CSS variables rather than hard-coded component colors.

Suggested dark direction:

- page background: near-black blue/graphite;
- panel background: slightly raised graphite;
- border: low-contrast cool gray;
- primary text: warm near-white;
- secondary text: muted gray;
- accent: restrained brass/amber;
- normal: green;
- review: amber;
- critical: red;
- informational: blue.

Suggested light direction:

- warm white page;
- white panels;
- gray borders;
- charcoal text;
- same restrained accent and semantic status colors.

Meet WCAG AA contrast for normal text.

Do not use color as the only status signal. Combine color with labels, icons, and shape.

### Density

Provide:

- comfortable mode;
- compact mode for large fleets.

Store the preference locally or in the user profile.

### Motion

Use only short transitions for focus, drawers, and route changes.

Honor `prefers-reduced-motion`.

Do not animate continuously unless the animation conveys active loading.

## Global shell

Desktop:

- persistent left navigation approximately 220–248 px;
- top bar containing page title, global search, data freshness, theme control, and account menu;
- main content constrained for readability but capable of full-width fleet tables.

Mobile:

- collapsible navigation drawer or compact bottom navigation;
- no horizontal page overflow;
- tables switch to priority columns, cards, or controlled horizontal scrolling;
- preserve filters and server search.

Navigation:

- Overview
- Fleet
- Security
- Traffic
- Events
- Settings

Display a visible global `Read-only monitoring` indicator.

## Overview page

The overview should answer:

1. Is the fleet reachable?
2. What needs attention now?
3. Which machines are under resource pressure?
4. Which machines may have security-relevant changes?
5. Which machines are nearing traffic or renewal limits?

### Header summary

Show:

- Online
- Stale
- Offline
- Critical findings
- Review findings
- Traffic nearing limit
- Expiring soon

Do not show a single fleet “security score”.

### Attention queue

A prioritized list combining:

- critical/review findings;
- offline/stale machines;
- rapidly rising resource pressure;
- traffic projection above limit;
- expiring services.

Each row should include:

- server;
- event/finding;
- severity;
- evidence summary;
- age;
- direct link to details.

### Resource overview

Show compact fleet distributions or ranked lists for:

- CPU;
- memory;
- disk capacity;
- PSI;
- network throughput.

Prefer readable sparklines and ranked tables over large decorative charts.

### Recent timeline

Show recent:

- online/offline transitions;
- login anomalies;
- new listening ports;
- suspicious process observations;
- OOM/kernel/filesystem events;
- traffic-cycle rollover;
- configuration/audit changes.

## Fleet page

This is the primary large-scale management surface.

### Table columns

Recommended defaults:

- status;
- server name;
- group/tags;
- provider label;
- location;
- security state;
- CPU;
- memory;
- disk;
- PSI;
- current-cycle traffic;
- traffic limit projection;
- expiry;
- last report;
- Agent version/coverage indicator.

### Behavior

- fast global search;
- column sorting;
- multi-filter;
- saved views;
- column visibility;
- comfortable/compact density;
- sticky header;
- virtualized or otherwise measured for thousands of rows;
- keyboard navigation;
- URL-addressable filter state where useful;
- clear stale-data timestamps.

Multi-select may support only:

- comparison;
- tagging/metadata updates in the Hub;
- export;
- acknowledgment of Hub-side findings.

It must never expose monitored-host actions.

### Status display

Differentiate:

- online;
- stale;
- offline;
- revoked;
- enrollment pending;
- partial telemetry;
- unsupported collector;
- Agent upgrade available.

Use explicit labels and tooltips.

## Server detail

Top header:

- server name;
- online/stale/offline;
- read-only badge;
- OS/kernel/architecture;
- public/private address metadata;
- group/tags;
- last report;
- telemetry coverage;
- active security findings;
- traffic cycle summary.

Tabs:

- Overview
- Resources
- Processes
- Network
- Security
- Events
- Traffic
- Inventory

### Server overview

Show:

- health summary;
- recent resource sparklines;
- active findings;
- uptime and boot;
- top processes;
- listening-port changes;
- traffic usage;
- telemetry coverage;
- recent events.

### Resources tab

Charts and summaries:

- CPU total and per core;
- load average;
- CPU PSI;
- memory and swap;
- memory PSI;
- disk capacity and inodes;
- disk I/O and I/O PSI;
- network rate, errors, and drops;
- OOM events.

Requirements:

- consistent synchronized time cursor;
- explicit units;
- downsampled data;
- gaps shown as gaps, not interpolated;
- stale intervals visibly marked;
- no misleading smoothing.

### Processes tab

Default periodic view should use top-N and changed/suspicious summaries.

Full snapshots require a typed temporary observation lease.

Columns:

- state marker;
- PID;
- PPID;
- user/UID;
- executable;
- CPU;
- RSS;
- virtual memory;
- start time;
- elapsed time;
- cgroup/container/systemd unit;
- listening/open socket counts;
- deleted executable marker;
- suspicious path marker;
- package ownership state where available.

Features:

- search;
- sort;
- user filter;
- cgroup/unit grouping;
- process-tree mode;
- suspicious-only filter;
- snapshot timestamp;
- coverage/privacy explanation.

Do not add kill, restart, renice, attach, shell, or file-open actions.

Full command lines must be absent by default. If a future explicit opt-in exists, visibly show that redaction is applied.

### Network tab

Show:

- selected traffic interfaces;
- per-interface counters and rates;
- errors/drops;
- listening ports;
- TCP state summary;
- remote endpoint aggregation;
- process/socket association where readable;
- newly exposed ports;
- connection-volume anomalies.

Do not stream every connection continuously.

Use aggregate periodic data and on-demand snapshots.

### Security tab

Layout:

- finding list/sidebar;
- evidence detail panel;
- timeline;
- baseline comparison;
- acknowledgment/suppression controls;
- safe manual verification guidance.

Each finding must show:

- severity;
- confidence;
- rule ID/version;
- first/last seen;
- evidence;
- explanation;
- affected process/port/user/event;
- data-coverage caveats.

Display an explicit statement:

> No finding is proof that the host is safe or compromised. Host-local telemetry may be falsified by a sufficiently privileged attacker.

### Events tab

Filterable event stream:

- availability;
- security;
- resource;
- traffic;
- enrollment/identity;
- Hub audit.

Support time range, category, severity, and free-text search.

### Traffic tab

This page must implement `TRAFFIC_ACCOUNTING_SPEC.md`.

Top card:

- current cycle total;
- limit;
- percentage;
- projection;
- cycle start/end;
- last update;
- confidence.

Breakdown:

- manual starting usage;
- Parade-observed addition;
- adjustments;
- inbound/outbound;
- selected interfaces.

Provide a clear form for:

- entering current provider-used traffic;
- configuring cycle boundary;
- configuring traffic limit;
- selecting interfaces;
- adding an audited correction.

Never name an ambiguous operation merely “Reset”.

Show history by billing cycle and preserve audit details.

### Inventory tab

Show:

- OS distribution/version;
- kernel;
- architecture;
- virtualization/container hints;
- CPU and memory inventory;
- filesystems;
- network interfaces;
- Agent version;
- supported/unsupported collectors;
- privilege/coverage mode;
- observation start.

## Security center

Fleet-wide security page.

Views:

- Active findings
- New since last review
- By rule
- By server
- Suppressed
- Acknowledged
- Coverage gaps

Filters:

- severity;
- confidence;
- rule;
- server/group;
- first/last seen;
- status;
- coverage.

Avoid alarm fatigue:

- group repeated identical findings;
- preserve occurrence count and timeline;
- support expiry-based suppression;
- show why a finding resurfaced.

## Traffic page

Fleet-wide traffic view.

Show:

- usage against limit;
- projected overage;
- cycle reset date;
- seed versus observed contribution;
- data confidence;
- last update;
- selected interface policy.

Do not add incompatible cycles without clear labeling.

Allow sorting by:

- percent used;
- projected percent;
- absolute usage;
- days remaining;
- stale/uncertain data.

## Settings

Sections:

- Hub status;
- administration and sessions;
- trusted proxies;
- retention/downsampling;
- alert destinations;
- Agent enrollment and revocation;
- Agent versions;
- telemetry profiles;
- traffic defaults;
- audit log;
- backup/restore guidance.

Provider integrations should not be implemented in the current milestone.

## Interaction states

Design all states:

- initial loading;
- skeleton loading;
- empty fleet;
- no findings;
- disconnected browser;
- stale Hub data;
- Agent offline;
- partial collector permission;
- unsupported kernel/distro feature;
- server revoked;
- enrollment pending;
- database/read failure;
- request timeout;
- observation lease pending;
- observation lease active with countdown;
- observation lease expired.

Errors should explain:

- what failed;
- what data may be stale;
- whether retry is safe;
- where to inspect logs.

## Observation lease UX

A server detail page may offer:

`Request temporary live detail`

This is not a remote-control action.

Before activation, explain:

- collectors to be enabled;
- expected additional bandwidth;
- duration;
- automatic expiry;
- read-only nature.

During activation:

- show countdown;
- show measured bandwidth;
- allow early cancellation by stopping the Hub-side lease;
- Agent must automatically return to normal mode even if the browser disappears.

Default maximum: 10 minutes.

## Accessibility

- semantic HTML;
- keyboard-complete navigation;
- visible focus;
- correct labels;
- appropriate ARIA only where native semantics are insufficient;
- no hover-only information;
- screen-reader-friendly status text;
- minimum practical touch target sizes;
- contrast checked in both themes;
- charts have textual summaries or tables.

## Performance acceptance

Add measured checks for:

- first meaningful render on a modest VPS and ordinary browser;
- thousands of fleet rows;
- large event/finding datasets;
- route changes;
- chart rendering;
- websocket/event update behavior;
- mobile layout.

Set a measured frontend bundle budget. A suggested starting ceiling is 500 KiB gzip for application JS and CSS combined, excluding source maps. Reduce it when practical.

## Visual testing

Use Playwright to capture and validate at least:

- Overview desktop dark;
- Overview desktop light;
- Fleet with many rows;
- Server detail;
- Processes;
- Security evidence;
- Traffic seed/cycle form;
- approximately 390x844 mobile;
- loading;
- empty;
- stale/partial coverage;
- critical finding.

Include representative screenshots in the draft pull request.

## UI acceptance criteria

- A new operator can find an offline or risky server in under a few interactions.
- A fleet with at least 1,000 synthetic servers remains usable.
- Traffic seed, observed addition, cycle boundary, and confidence are understandable without reading backend code.
- No monitored-host mutation control exists.
- Security findings always expose evidence and uncertainty.
- The interface remains useful when some collectors are unavailable.
- Dark/light and desktop/mobile layouts are intentionally designed.
