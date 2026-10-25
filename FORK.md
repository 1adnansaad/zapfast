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

Synced to: upstream `v0.20.1` (2026-10-25).

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
  them, and with the taller input its press missed the text it aimed at;
  upstream's `preview_navigation_overlays_the_photo_on_hover`, which finds
  the viewer's arrows as 40pt circles, leaves out the send button, now
  about that size, by finding it once with the arrows hidden).
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
  rebase this fork onto the release tag while keeping what this file lists,
  type checking only and leaving the other checks to me (as AGENTS.md's fork
  section says). Upstream's Download and "Restart to update" buttons never
  appear.
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
  fork)": agents write the code and its tests and run only `cargo check`
  (with `--all-features` when feature-gated code changed). The user runs
  fmt, clippy, tests and doc, and anything that means running the app and
  watching it (including `--demo-shot`); agents hand over those steps.
- **Why:** the user checks how things look and feel, and runs the slow
  checks; agents check that the code compiles.
- **Files:** `AGENTS.md` (appended section only).
- **Cost:** none. Kept at the end of the file because upstream edits
  `AGENTS.md` often, so a sync should only conflict if upstream appends too
  (0.20 appended an account-wide "Pull requests" section; ours follows it).

### Named WhatZap, crediting ZapFast

- **What:** the window title, tray, notifications (and the Windows toast
  name), link screen, About, macOS menus, dialogs, messages and shortcut list
  say WhatZap, as do calls' messages, the volume mixer's name for our sounds,
  the exe's product name, the pack name on stickers made here, and the name
  the phone lists under Linked devices, `WhatZap (hostname)` with upstream's
  hostname suffix (WhatsApp reads it only when linking, so it shows after
  the next link). With chats loaded, the
  pane without an open chat says "A modified fork of ZapFast" instead of
  "Select a chat on the left.", "ZapFast" linking to the original repository
  (`CARGO_PKG_REPOSITORY`).
- **Why:** this fork's own name, with credit to the project it comes from.
- **Files:** user-visible strings in `src/main.rs`, `src/app.rs`,
  `src/notify.rs` (including the name fastframe-notifications registers as
  the Windows toast's `DisplayName`), `src/app/calls.rs`, `src/calls.rs`,
  `src/camera.rs`, `src/backend/worker/calls.rs`,
  `Cargo.toml` (`[[bin]]` is `WhatZap`, so the file is `WhatZap.exe`; the
  package and library stay `zapfast`), `src/main.rs` (`allow(non_snake_case)`
  for that crate name), `tests/update_flags.rs` (`CARGO_BIN_EXE_WhatZap`),
  `src/macos.rs`,
  `src/data_folder.rs`, `src/paths.rs`, `src/archive/encryption.rs` (messages,
  with their copies in `src/ui/login.rs`'s test), `src/backend/worker.rs`,
  `src/backend/sticker_maker.rs`, `src/backend/sticker_store.rs`,
  `src/backend/worker/interactive/replies.rs` and `src/ui/` (`conversation.rs`
  also has `fork_line`), tests that quote them (`app.rs`, `main.rs`,
  `demo.rs`), `build.rs` (`ProductName`, `FileDescription`).
- **Kept as upstream on purpose:** every identifier, so the link, archive key
  and data stay put: crate and exe `zapfast`, `run_native("ZapFast")`,
  `app_id`, data folders, keyring service `rocks.zapfast.ZapFast`, AUMID,
  autostart entry names (`Autostart::new("zapfast", "ZapFast")` in
  `src/autostart.rs`: fastframe-autostart uses the name as the Windows Run
  value too, so since 0.20 the Linux entry's `Name=` says ZapFast again),
  `ZAPFAST_*` variables, the update slug, the OGG vendor tag. Update toasts,
  the update dialog and its settings say ZapFast because they name
  upstream's release; the macOS Login Items and camera-permission messages
  do because they name the bundle. `docs/`, `packaging/` and
  `assets/i18n/` are untouched. After an upstream sync, rename any new
  user-visible "ZapFast" string the same way.
- **Cost:** translated strings that named ZapFast (lock screen, the labels
  limit, read-only notes, locked-chat dialogs) are new msgids, so other
  languages show them in English. The group-call messages (0.20) instead
  translate upstream's text and then swap the name, so they stay translated
  (upstream's test checks the German one). Installers, the macOS bundle, Linux
  packages and the docs site still say ZapFast. The release workflow and
  `packaging/windows/zapfast.iss` still expect `zapfast.exe`. A Windows
  start-at-login entry made by `zapfast.exe` points at that file until it is
  turned off and on again in Settings.

### yt-dlp tab in the picker

- **What:** a fourth picker tab, yt-dlp, beside Emoji, GIF and Stickers. A
  strip of site tabs like the sticker shelves (YouTube, X, Instagram,
  Facebook, TikTok, and a link icon for any other site yt-dlp reads) sits
  over a field: YouTube searches or takes a link, the others take a link.
  Results show as cards with thumbnail, length and title, and two buttons.
  Send fetches the video (H.264/AAC MP4, at most 720p, under the 64 MB
  attachment limit) and sends it to the open chat with the reply that was
  open, through `send_files`. Download fetches it into the download folder
  (the Settings one, else the system's Downloads); a saved card's button
  then shows its folder, and Send uses the saved file. The picker stays open
  and the card shows the download's progress, read from yt-dlp's
  `--progress-template` lines; a failure replaces the card's title.
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
  `assets/icons/{youtube,instagram,facebook}.svg` (Lucide) and
  `{x-logo,tiktok}.svg` (drawn here; the link icon is upstream's since 0.20),
  `src/theme.rs` (icon table),
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
  means downloading them again. With ffmpeg the picture and the sound come
  down one after the other, so the bar steps back a little when the sound
  starts. A download cannot be cancelled.

### Long text in one-line fields scrolls sideways

- **What:** `bidi::layout_field` laid single-line fields out wrapped at the
  field's width, so a long search (any picker tab, the chat search, poll
  answers, names) broke onto a second line. It now lays them out on one row,
  and egui's single-line `TextEdit` scrolls the text to keep the cursor in
  view.
- **Why:** reported on the yt-dlp tab's search; upstream has the same code.
- **Files:** `src/bidi.rs` (one line, doc, test).
- **Cost:** none known; worth offering upstream.

### Red dot on the Windows taskbar button

- **What:** the unread overlay on the taskbar button is a solid red dot,
  antialiased, instead of upstream's pixel-font count (`99+` above 99).
  `DOT_RADIUS` sets its size.
- **Why:** the hand-drawn 16px digits looked blocky, more so once Windows
  scales the overlay for the display.
- **Files:** since 0.20 upstream draws the overlay in fastframe-dock, so
  `src/notify/badge_windows.rs` is a copy of fastframe-dock v0.14.1's Windows
  `Taskbar` (same API, its retry schedule and recreated-button hook) with the
  dot's `overlay_rgba`, and its tests. `src/notify.rs` exports it in place of
  `fastframe_dock::Taskbar`; `Cargo.toml` adds the `Win32_Graphics_Gdi` and
  `Win32_UI_Shell` features to `windows`. `docs/_guide/using-zapfast.md`.
- **Cost:** the button no longer shows how many; the overlay's accessible
  text still says "N unread messages". Fixes upstream makes to
  fastframe-dock's Windows overlay must be copied by hand (compare
  `crates/fastframe-dock/src/windows.rs` at each sync), and a change to its
  `Taskbar` API breaks the build until this copy follows.

### Paste file path in the plus menu

- **What:** the composer's **+** menu has "Paste file path" under "Send
  files" and "Send as document". It reads the clipboard's text, one path per line with or without
  quotes (Windows Explorer's "Copy as path"), and stages every line that names
  an existing file, as the file picker would. Nothing usable shows a toast.
  Ctrl+V of such text into an empty message box still pastes it, and a strip
  above the box offers "Attach file.pdf?" (or "Attach N files?") while the
  box holds that text: Attach clears the text and stages the files, × keeps
  the text. Only a paste whose every non-blank line names a file offers it.
- **Why:** attach a file whose path I already copied without browsing to it.
- **Files:** `src/model.rs` (`Action::AttachCopiedPaths`),
  `src/ui/conversation.rs` (`composer_tools_menu`, `path_suggestion_strip`),
  `src/app.rs` (the action, `path_suggestion`, the `else if` branch at the
  end of upstream's `take_clipboard_paste`, `copied_paths`,
  `only_copied_paths`, `holds_pasted` and their tests),
  `src/demo.rs` (`paste-path`, and upstream's
  `the_plus_menu_sends_files_or_creates_a_poll_and_closes`, which clicks
  rows by position and now knows the fifth), `docs/_guide/using-zapfast.md`.
- **Cost:** English-only labels. Folders are skipped. A paste into a box that
  already has text offers nothing (use the menu).

### F11 full screen

- **What:** F11 toggles full screen (winit's borderless one). On Windows and
  Linux the title bar goes with it, so, as in Edge, the pointer at the top
  edge slides down a 32pt strip with "WhatZap", Minimize, Exit full screen
  and Close; it slides away once the pointer moves below it. Close goes
  through `Action::CloseWindow`, so it keeps WhatZap in the tray as the
  native button does. macOS keeps its own reveal. While a call fills the
  screen (0.20's call full screen), F11 and Exit full screen go through the
  call's own toggle (`Action::ToggleCallFullscreen`), so its button agrees.
- **Why:** a distraction-free window, like Edge's F11.
- **Files:** `src/ui/mod.rs` (`full_screen`, `strip_shown`, their tests),
  `src/ui/keys.rs` (`SHORTCUTS` row), `docs/_guide/using-zapfast.md`.
- **Cost:** English-only labels. No "press F11 to exit" hint on entry. The
  strip is drawn by the app, not Windows, so it has no snap layouts or
  window menu.

### Tray menu kept off the taskbar (patched tray-icon)

- **What:** `[patch.crates-io]` swaps tray-icon 0.25.1 (used through
  fastframe-tray, and directly for the macOS menu bar) for a copy whose
  Windows `show_tray_menu` calls `TrackPopupMenuEx` and excludes the
  rectangle of the taskbar (or overflow flyout) under the click. The menu
  then opens just above the taskbar instead of bottom-aligned at the
  pointer, partly over it.
- **Why:** with the taskbar set to auto-hide, the taskbar that pops up for
  the click stays above the menu and covered its bottom.
- **Files:** `Cargo.toml` (the patch line), `Cargo.lock`; the change itself
  is one commit (84edf73) on branch `menu-above-taskbar` of
  [1adnansaad/tray-icon](https://github.com/1adnansaad/tray-icon), made from
  upstream's tag `tray-icon-v0.25.1` (local clone: `D:\dev\tray-icon`).
- **Cost:** goes against AGENTS.md's "do not patch upstream crates" rule,
  and its rule that patches point only at the maintainer's repositories, by
  choice. Keep that branch on GitHub: builds fetch the pinned rev. A fastframe
  or tray-icon bump past 0.25 makes the patch stop applying (Cargo warns
  "patch was not used"): rebase the commit onto the new tag or drop it once
  tray-icon fixes this. Worth offering upstream (tauri-apps/tray-icon).

### Message any number from New chat

- **What:** typing or pasting a number into New chat's search (`+1 (555)
  123-4567`, `0044 ...`, plain digits) lists contacts by its digits, and from
  7 digits offers a "Message this number" row on top. Clicking it, or Enter in
  the field, runs New contact's Message (`Action::NewContact` without a name):
  WhatsApp's registration check, then the chat opens, or a "not on WhatsApp"
  toast. No row for a listed contact's exact number, ourselves, or a locked
  chat.
- **Why:** start a chat with a number not in contacts without opening Add
  contact first.
- **Files:** `src/util.rs` (`typed_number` and its test), `src/ui/chats.rs`
  (`number_row`, `message_number`; `person_row` takes `fetch_picture`),
  `src/ui/dialogs.rs` (`new_chat` hands the digits to upstream's
  `contact_choices`, which New call and New group share, unchanged),
  `docs/_guide/using-zapfast.md`.
- **Cost:** English-only label. A local number without its country code
  (`0300...`) is not converted and fails the check. Upstream's
  `util::international_phone` (0.20, for New contact) rejects the `00`
  prefix, so `typed_number` stays separate. No profile picture on the
  row, so typing never asks for a stranger's.

## Dropped (upstream does it now)

- **Ctrl+1 to Ctrl+9 open the nth chat.** Written here first; upstream
  shipped the same in #345 (b8c8aa5), so ours was discarded before it was
  committed.
