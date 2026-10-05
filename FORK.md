# Fork ledger

This checkout is a personal fork of [crmne/zapfast](https://github.com/crmne/zapfast).
Remotes: `upstream` is crmne/zapfast, `origin` is 1adnansaad/zapfast.

Every way this fork differs from upstream is listed below, with why, so an
upstream release can be taken without losing anything. The fork's commits
sit on top of upstream (`git log --oneline upstream/main..main`); this file
says what each one is for.

**Agents and contributors:** update this file in the same commit as any
change that makes the fork differ from upstream, and when a sync drops or
reworks one.

Synced to: upstream `v0.19.0-10-g63ed17c` (2026-10-06).

## Changes

## Dropped (upstream does it now)

- **Ctrl+1 to Ctrl+9 open the nth chat.** Written here first; upstream
  shipped the same in #345 (b8c8aa5), so ours was discarded before it was
  committed.
