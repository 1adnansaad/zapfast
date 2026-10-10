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
  comment that stated the old 40pt height, and
  `a_copy_across_messages_survives_rows_skipped_above`, which now lets the
  rows settle under a pointer before measuring: a pointer arriving shifts
  them, and with the taller input its press missed the text it aimed at).
- **Cost:** the message list is about 7pt shorter.

### Own icon from a gitignored `branding/` folder

- **What:** `branding/icon.svg` (and optionally `icon-small.svg`,
  `icon.ico`) replaces the app icon: window, taskbar, tray, the in-app logo,
  and the exe's icon, which `build.rs` draws from the SVG at 16 to 256px.
  Without those files upstream's icon is the placeholder. Only
  `branding/README.md` is tracked.
- **Why:** a personal icon that never enters git, and upstream icon updates
  still arrive for the placeholder.
- **Files:** `build.rs` (picks the files, skips with a warning an SVG that
  draws nothing, such as one wrapping a bitmap, exports `ZAPFAST_ICON_SVG`
  and `ZAPFAST_ICON_SMALL_SVG`, writes `OUT_DIR/icon.ico`), `src/util.rs`
  (`MARK`/`SMALL_MARK` read those variables; unit tests keep upstream's mark,
  whose shape they check), `Cargo.toml` (resvg as a build-dependency),
  `.gitignore`, `branding/README.md`.
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

### Choose the data folder

- **What:** a data folder is a whole profile (`config` with settings, the
  account list and themes; `state`; `cache`), as `AppDirs::under` lays it
  out. A fresh install asks before its window opens: "Standard location" or a
  folder. Later, Settings > Files > Data folder > Change… (or Use default) and
  a line on the link screen leave a choice for the next start. An empty folder
  receives the current data (archive keys copied to the new paths and read
  back, folders renamed or, across drives, copied, checked and removed); a
  folder already holding ZapFast data is used as it is, and the current data
  stays where it was; anything else is refused. A folder whose archives have
  no key in this computer's keyring (copied by hand or from another computer)
  is refused too, as its history could not be read. A linked folder needs no
  new link. Changes run in `main.rs` while this copy holds the instance guard
  and before anything opens a file, and nothing changes if a step fails.
  `<standard config>.folder` (e.g. `%APPDATA%\paolino\zapfast\config.folder`)
  records the folder, `.move` beside it a pending change; the runtime lock
  stays at the standard place. `zapfast.lock` in the state folder keeps a
  second copy off a folder in use. `ZAPFAST_DATA_DIR` puts everything,
  runtime included, under one folder, so a dev build runs beside the daily
  copy with its own data.
- **Why:** keep the data where I choose, carry on from a folder set up earlier
  without linking again, and test builds without touching the real archive.
- **Files:** `src/data_folder.rs` (new: all logic and tests), `src/paths.rs`
  (`discover` = `relocate(standard())`), `src/main.rs` (first-launch choice,
  pending change, lock, toast), `src/lib.rs`, `src/archive/encryption.rs` and
  `src/archive.rs` (`has_archive_key`, a read-only keyring check),
  `src/model.rs` (`PickDataFolder`, `ChangeDataFolder`,
  `CancelDataFolderChange`), `src/backend.rs` (`Command::PickDataFolder`,
  `Event::DataFolderPicked`), `src/backend/worker.rs` (the folder picker),
  `src/app.rs` (the actions and event), `src/ui/settings.rs` (the row),
  `src/ui/login.rs` (the link-screen line),
  `docs/_reference/settings-and-files.md`.
- **Cost:** English-only labels. A change needs a restart (quit and reopen).
  Old keyring entries are kept, so a backup restored to the old place still
  opens. A chosen folder on a drive that is missing at start fails the start
  until it is back or `config.folder` is removed. An upstream build would not
  read `config.folder` and would start from the standard place.

### Paper plane in the account switcher

- **What:** each linked account in the menu under our own picture (beside
  "Chats") has a paper plane between its name and number and the check or
  unread count. It opens that account's chat with ourselves, switching to the
  account first when it is not the one on screen (`Action::MessageYourself`,
  the same as the New chat dialog's "Message yourself" row).
- **Why:** one click to the self chat.
- **Files:** `src/ui/accounts.rs` (`account_row` and `menu`), `src/demo.rs`
  (`the_switchers_paper_plane_messages_yourself`),
  `docs/_guide/getting-started.md`.
- **Cost:** the name and number column is 32pt narrower and ellipsizes
  sooner. The tooltip reuses the translated "Message yourself" string.

### Agents ask the user to run and look

- **What:** a section at the end of `AGENTS.md`, "What the agent runs (this
  fork)": agents ask the user to do anything that means running the app and
  watching it (including `--demo-shot`), and run only checks that give their
  result as text (type checking, lints, `cargo test`).
- **Why:** the user checks how things look and feel; agents check the code.
- **Files:** `AGENTS.md` (appended section only).
- **Cost:** none. Kept at the end of the file because upstream edits
  `AGENTS.md` often, so a sync should only conflict if upstream appends too.

### Named WhatZap, crediting ZapFast

- **What:** the window title, tray, notifications (and the Windows toast
  name), link screen, About, macOS menus, dialogs, messages and shortcut list
  say WhatZap, as do the exe's product name, the pack name on stickers made
  here, and the name the phone lists under Linked devices (WhatsApp reads it
  only when linking, so it shows after the next link). With chats loaded, the
  pane without an open chat says "A modified fork of ZapFast" instead of
  "Select a chat on the left.", "ZapFast" linking to the original repository
  (`CARGO_PKG_REPOSITORY`).
- **Why:** this fork's own name, with credit to the project it comes from.
- **Files:** user-visible strings in `src/main.rs`, `src/app.rs`,
  `src/notify.rs`, `src/notify/windows.rs` (`DisplayName` only),
  `src/macos.rs`, `src/autostart.rs` (Linux `Name=`/`Comment=`),
  `src/data_folder.rs`, `src/paths.rs`, `src/archive/encryption.rs` (messages,
  with their copies in `src/ui/login.rs`'s test), `src/backend/worker.rs`,
  `src/backend/sticker_maker.rs`, `src/backend/sticker_store.rs`,
  `src/backend/worker/interactive/replies.rs` and `src/ui/` (`conversation.rs`
  also has `fork_line`), tests that quote them (`app.rs`, `main.rs`,
  `demo.rs`), `build.rs` (`ProductName`, `FileDescription`).
- **Kept as upstream on purpose:** every identifier, so the link, archive key
  and data stay put: crate and exe `zapfast`, `run_native("ZapFast")`,
  `app_id`, data folders, keyring service `rocks.zapfast.ZapFast`, AUMID,
  autostart entry names, `ZAPFAST_*` variables, the update slug, the OGG
  vendor tag. Update toasts, the update dialog and its settings say ZapFast
  because they name upstream's release. `docs/`, `packaging/` and
  `assets/i18n/` are untouched. After an upstream sync, rename any new
  user-visible "ZapFast" string the same way.
- **Cost:** translated strings that named ZapFast (lock screen, the labels
  limit, read-only notes, locked-chat dialogs) are new msgids, so other
  languages show them in English. Installers, the macOS bundle, Linux
  packages and the docs site still say ZapFast.

### yt-dlp tab in the picker

- **What:** a fourth picker tab, yt-dlp, beside Emoji, GIF and Stickers. A
  strip of site tabs like the sticker shelves (YouTube, X, Instagram,
  Facebook, TikTok, and a link icon for any other site yt-dlp reads) sits
  over a field: YouTube searches or takes a link, the others take a link.
  Results show as thumbnails with length and title; a click fetches the video
  (H.264/AAC MP4, at most 720p, under the 64 MB attachment limit) and sends
  it to the open chat with the reply that was open, through `send_files`.
  The yt-dlp program is run directly, no Rust crate: from `PATH` first, else
  a copy downloaded from yt-dlp's GitHub release when the tab's button is
  clicked, checked against `SHA2-256SUMS`, and updated with `yt-dlp -U` once
  a week. ffmpeg is optional (needed above YouTube's 360p single files): from
  `PATH`, or on Windows yt-dlp's shared FFmpeg build (about 85 MB) on request,
  checked against `checksums.sha256`. Runs hide their console window on
  Windows and pass the Proxy setting as `--proxy`.
- **Why:** send videos from the sites I use without saving them by hand.
- **Files:** `src/ytdlp.rs` (new: types, finding and installing the tools,
  search, download, tests), `src/backend/worker/ytdlp.rs` (new: the worker
  side), `src/ui/picker/ytdlp.rs` (new: the tab and its tests),
  `assets/icons/{youtube,instagram,facebook,link}.svg` (Lucide) and
  `{x-logo,tiktok}.svg` (drawn here), `src/theme.rs` (icon table),
  `src/ui/picker.rs` (tab entry and match arm), `src/model.rs`
  (`PickerTab::YtDlp`, four actions), `src/backend.rs` (commands and
  events), `src/backend/worker.rs` (dispatch, read-only check), `src/app.rs`
  (`ytdlp` state, actions, events, test), `src/paths.rs`
  (`AccountDirs::tools_dir`), `src/proxy.rs` (`Proxy::url`), `src/lib.rs`,
  `src/demo.rs` (`ytdlp`, `ytdlp-missing`),
  `docs/_reference/settings-and-files.md`, `docs/_reference/how-it-links.md`.
- **Cost:** only YouTube can be searched (yt-dlp searches no other of these
  sites). The ffmpeg download is Windows only; elsewhere install it with the
  package manager. Downloads of the tools use the Proxy setting; yt-dlp gets
  it on its command line, where other local users could see it. A video
  over 64 MB is refused. Without ffmpeg, a site that serves only streams in
  pieces may not send. No browser cookies are passed, so posts that need a
  login (common on Instagram and Facebook) fail with yt-dlp's message.
  English-only labels. Tools live in `cache/yt-dlp/`, so clearing the cache
  means downloading them again.

## Dropped (upstream does it now)

- **Ctrl+1 to Ctrl+9 open the nth chat.** Written here first; upstream
  shipped the same in #345 (b8c8aa5), so ours was discarded before it was
  committed.
