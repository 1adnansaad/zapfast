I keep a personal fork of ZapFast at {checkout} (remote `origin` is my fork,
`upstream` is crmne/zapfast). Upstream released ZapFast {new} ({url}); I run
{running}. Bring my fork to that release without losing my changes:

1. Read FORK.md (my changes and why) and AGENTS.md (the project's rules). Run
   `git log --oneline upstream/main..main` to see my commits.
2. If `git status` is not clean, stop and ask me. When the changes are fork
   work of mine, offer to commit them first as their own commit with a FORK.md
   entry, so the rebase carries them. Once it is clean, create the branch
   `backup/pre-{new}` at the current `main`.
3. Run `git fetch upstream --tags`, then `git rebase v{new}`. In PowerShell,
   set `$env:GIT_EDITOR = 'true'` first so `git rebase --continue` does not
   wait on an editor.
4. Resolve each conflict so upstream's new behaviour stays and my change still
   does what FORK.md says it is for. If upstream now does what one of my
   changes did, drop mine and move it to FORK.md's Dropped list. If both
   cannot hold, stop and ask me. A clean merge can still collide: also read
   what upstream changed near each entry's files, and do every check an entry
   asks for at a sync (search FORK.md for "sync": copied upstream code to
   compare, patched crate versions, new user-visible "ZapFast" strings).
5. Type check only, as AGENTS.md's fork section says: run
   `cargo check --all-targets` and `cargo check --all-targets --all-features`
   from PowerShell rather than Git Bash (MSYS perl cannot build the bundled
   OpenSSL). Leave out `--locked`: the lock then takes my dependencies on top
   of upstream's, and the updated `Cargo.lock` is committed. Run none of the
   other Definition of done checks (fmt, clippy, tests, doc); list them for
   me to run, with what to try in the app for each entry the sync touched.
6. Update FORK.md: the "Synced to" line, and any entry that changed.
7. Show me `git log --oneline v{new}..main` and summarize what changed and
   what you decided. Do not push; I will run
   `git push --force-with-lease origin main` after reviewing.
