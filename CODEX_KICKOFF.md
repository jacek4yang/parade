# CODEX_KICKOFF.md

Paste the following into Codex after restarting it from the Parade repository root:

```text
Read @AGENTS.md, @CODEX_GOAL.md, @TRAFFIC_ACCOUNTING_SPEC.md, and @UI_SPEC.md completely before making major design decisions.

Execute @CODEX_GOAL.md end to end. Verify Git, origin, the default branch, and `gh auth status`; preserve existing changes; create or switch to `codex/read-only-vps-observability`.

Create and maintain PLANS.md. Audit, design, implement, test, review, commit, push, and open a draft pull request. Do not stop after planning.

Keep the monitored-host boundary strictly read-only. Never add arbitrary execution or host mutation. Provider APIs are out of scope. Implement the manual current-cycle traffic seed, locally observed accumulation, automatic monthly cycle rollover, audited corrections, restart/reboot recovery, and explicit uncertainty exactly as specified.

Implement the polished large-fleet UI in UI_SPEC.md. Use subagents for independent reviews when useful, but coordinate writes. Do not ask broad questions; choose safe defaults and continue. Do not merge.

Begin now.
```
