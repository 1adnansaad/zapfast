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

### Ctrl+0 opens the search

- **What:** Ctrl+0 (Cmd+0 on macOS) focuses the chat and message search,
  like Ctrl+K. Upstream binds it to reset zoom.
- **Why:** a one-hand key for search next to Ctrl+1 to Ctrl+9.
- **Files:** `src/ui/keys.rs` (binding, `SHORTCUTS` row, and upstream's test
  `numbered_shortcuts_do_not_wrap_missing_positions_and_zero_searches`),
  `src/macos.rs` (Actual Size menu item loses Cmd+0, and its accelerator
  test), `src/ui/settings.rs` (Zoom hint no longer mentions Ctrl+0),
  `docs/_guide/using-zapfast.md`, `docs/_reference/settings-and-files.md`.
- **Cost:** reset zoom has no shortcut; step back to 100% with Ctrl+Plus and
  Ctrl+Minus or the Settings buttons. The Zoom hint is a new English string,
  so other languages show it untranslated; the `.po` catalogs are left alone
  on purpose (upstream rewrites them often).

### Roomier message input

- **What:** `COMPOSER_PADDING` is 24 instead of 14, so a one-line message
  field is about 43pt tall on Windows instead of 36 (the 36pt
  `COMPOSER_CONTROL` floor used to win). The send and record buttons and the
  recording strip grow with it.
- **Why:** more breathing room around the text being typed.
- **Files:** `src/ui/conversation.rs` (the constant), `src/demo.rs` (a test
  comment that stated the old 40pt height).
- **Cost:** the message list is about 7pt shorter.

### Own icon from a gitignored `branding/` folder

- **What:** `branding/icon.svg` (and optionally `icon-small.svg`,
  `icon.ico`) replaces the app icon: window, taskbar, tray, the in-app logo,
  and the exe's icon, which `build.rs` draws from the SVG at 16 to 256px.
  Without those files upstream's icon is the placeholder. Only
  `branding/README.md` is tracked.
- **Why:** a personal icon that never enters git, and upstream icon updates
  still arrive for the placeholder.
- **Files:** `build.rs` (picks the files, exports `ZAPFAST_ICON_SVG` and
  `ZAPFAST_ICON_SMALL_SVG`, writes `OUT_DIR/icon.ico`), `src/util.rs`
  (`MARK`/`SMALL_MARK` read those variables), `Cargo.toml` (resvg as a
  Windows build-dependency), `.gitignore`, `branding/README.md`.
- **Cost:** macOS (`packaging/macos/icon-1024.*`, the menu-bar template,
  which assumes upstream's colours), the Windows installer and Linux packages
  keep upstream's icon. Merge conflicts are likely only if upstream rewrites
  `build.rs` or the `MARK` constants.

### Updates copy a merge prompt instead of installing

- **What:** the update dialog's one button is always "Copy merge prompt". It
  copies `assets/fork-merge-prompt.md` with this checkout's path, the running
  version and the release's version and URL filled in, asking an assistant to
  rebase this fork onto the release tag while keeping what this file lists.
  Upstream's Download and "Restart to update" buttons never appear.
- **Why:** installing upstream's release would replace this build and drop
  every change here.
- **Files:** `src/updates.rs` (`merge_prompt` and its test),
  `src/ui/update.rs` (sets upstream's chosen action aside just before the
  button is drawn), `assets/fork-merge-prompt.md` (the wording; edit freely,
  keeping the `{placeholders}`).
- **Cost:** with Settings > "Download updates automatically" on, upstream's
  update is still downloaded in the background, though nothing installs it:
  keep that setting off.

## Dropped (upstream does it now)

- **Ctrl+1 to Ctrl+9 open the nth chat.** Written here first; upstream
  shipped the same in #345 (b8c8aa5), so ours was discarded before it was
  committed.
