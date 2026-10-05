I keep a personal fork of ZapFast at {checkout} (remote `origin` is my fork,
`upstream` is crmne/zapfast). Upstream released ZapFast {new} ({url}); I run
{running}. Bring my fork to that release without losing my changes:

1. Read FORK.md (my changes and why) and AGENTS.md (the project's rules). Run
   `git log --oneline upstream/main..main` to see my commits.
2. If `git status` is not clean, stop and ask me. Otherwise create the branch
   `backup/pre-{new}` at the current `main`.
3. Run `git fetch upstream --tags`, then `git rebase v{new}`.
4. Resolve each conflict so upstream's new behaviour stays and my change still
   does what FORK.md says it is for. If upstream now does what one of my
   changes did, drop mine and move it to FORK.md's Dropped list. If both
   cannot hold, stop and ask me.
5. Run the checks in AGENTS.md's Definition of done, from PowerShell rather
   than Git Bash (MSYS perl cannot build the bundled OpenSSL).
6. Update FORK.md: the "Synced to" line, and any entry that changed.
7. Show me `git log --oneline v{new}..main` and summarize what changed and
   what you decided. Do not push; I will run
   `git push --force-with-lease origin main` after reviewing.
