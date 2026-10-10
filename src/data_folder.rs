//! Fork: keeps ZapFast's data in a folder the user picks (FORK.md).
//!
//! A data folder is a whole profile, laid out as `AppDirs::under` lays it
//! out: `config` (settings, the account list, themes), `state` and `cache`.
//! The chosen folder is recorded beside the standard config directory, in
//! `<config>.folder`, and the runtime directory (the single-instance lock)
//! stays at the standard place. Choosing an empty folder moves the data
//! there; choosing a folder that already holds ZapFast data switches to it
//! and leaves the current data where it is. Both happen at a start, before
//! anything opens a file: a fresh install asks before its window opens, and
//! later Settings or the link screen leave the choice in `<config>.move` for
//! the next start. `ZAPFAST_DATA_DIR` puts everything, runtime included,
//! under one folder, so a development build runs beside the daily copy. A
//! lock file in the state directory keeps two copies off one folder.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::paths::AppDirs;

/// Beside the home config directory: the chosen folder, and a change
/// waiting for the next start.
const FOLDER: &str = ".folder";
const CHANGE: &str = ".move";
const CHOOSE: &str = "Choose folder…";

/// The directories to use, given the platform's standard ones.
pub fn relocate(standard: AppDirs) -> AppDirs {
    relocate_from(&home(standard, std::env::var_os("ZAPFAST_DATA_DIR")))
}

fn relocate_from(home: &AppDirs) -> AppDirs {
    match read_path(&marker(home, FOLDER)) {
        Some(folder) => inside(home, &folder),
        None => home.clone(),
    }
}

/// Where the data lives when no folder has been chosen.
fn home(standard: AppDirs, dev: Option<OsString>) -> AppDirs {
    match dev {
        Some(dir) if !dir.is_empty() => AppDirs::under(Path::new(&dir)),
        _ => standard,
    }
}

fn current_home() -> AppDirs {
    home(AppDirs::standard(), std::env::var_os("ZAPFAST_DATA_DIR"))
}

fn marker(home: &AppDirs, extension: &str) -> PathBuf {
    let mut name = home.config.file_name().unwrap_or_default().to_os_string();
    name.push(extension);
    home.config.with_file_name(name)
}

/// The profile in `folder`, sharing the home runtime directory.
fn inside(home: &AppDirs, folder: &Path) -> AppDirs {
    AppDirs {
        runtime: home.runtime.clone(),
        ..AppDirs::under(folder)
    }
}

/// Where a choice puts the data: `None` is the standard place.
fn destination(home: &AppDirs, folder: Option<&Path>) -> AppDirs {
    match folder {
        Some(folder) => inside(home, folder),
        None => home.clone(),
    }
}

/// The folder chosen for the data, if any.
pub fn chosen_folder() -> Option<PathBuf> {
    read_path(&marker(&current_home(), FOLDER))
}

/// A change waiting for the next start: `Some(folder)`, or `None` for the
/// standard place. The outer `None` means nothing is waiting.
pub fn pending_change() -> Option<Option<PathBuf>> {
    pending_in(&current_home())
}

fn pending_in(home: &AppDirs) -> Option<Option<PathBuf>> {
    let text = fs::read_to_string(marker(home, CHANGE)).ok()?;
    let text = text.trim();
    Some((!text.is_empty()).then(|| PathBuf::from(text)))
}

/// Asks the next start to keep the data in `folder`, or in the standard
/// place, after checking that it can, and says what will happen.
pub fn request_change(dirs: &AppDirs, folder: Option<&Path>) -> io::Result<String> {
    let home = current_home();
    let to = destination(&home, folder);
    let message = match plan(dirs, &to, folder, &has_key)? {
        Plan::Move => format!("Restart WhatZap to move your data to {}", shown(&to)),
        Plan::Switch => format!(
            "Restart WhatZap to use the data in {}; the current data stays in {}",
            shown(&to),
            shown(dirs)
        ),
    };
    let text = match folder {
        Some(folder) => folder
            .to_str()
            .ok_or_else(|| io::Error::other("the folder's name is not valid Unicode"))?,
        None => "",
    };
    write_atomic(&marker(&home, CHANGE), text)?;
    Ok(message)
}

/// The data folder `here`, and the change waiting for the next start.
pub fn describe(here: &Path, pending: Option<&Option<PathBuf>>) -> String {
    match pending {
        Some(Some(folder)) => format!(
            "{} (changes to {} when WhatZap restarts)",
            here.display(),
            folder.display()
        ),
        Some(None) => format!(
            "{} (changes to the standard folder when WhatZap restarts)",
            here.display()
        ),
        None => here.display().to_string(),
    }
}

/// Forgets a change that has not happened yet.
pub fn cancel_change() -> io::Result<()> {
    remove_if_present(&marker(&current_home(), CHANGE))
}

/// On a fresh install, asks where to keep the data before anything is made,
/// so a folder set up earlier is used straight away. Returns the
/// directories to use.
pub fn ask_on_first_launch(dirs: AppDirs) -> AppDirs {
    let home = current_home();
    if std::env::var_os("ZAPFAST_DATA_DIR").is_some_and(|dir| !dir.is_empty()) || !fresh(&home) {
        return dirs;
    }
    loop {
        let choice = rfd::MessageDialog::new()
            .set_title("WhatZap")
            .set_description(
                "Where should WhatZap keep your chats and settings?\n\nChoose a folder that already holds WhatZap data to carry on from it, or an empty folder to start there.",
            )
            .set_buttons(rfd::MessageButtons::OkCancelCustom(
                "Standard location".into(),
                CHOOSE.into(),
            ))
            .show();
        if !matches!(&choice, rfd::MessageDialogResult::Custom(label) if label == CHOOSE) {
            return dirs;
        }
        let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose where WhatZap keeps its data")
            .pick_folder()
        else {
            continue;
        };
        let to = inside(&home, &folder);
        let chosen =
            plan(&dirs, &to, Some(&folder), &has_key).and_then(|_| record(&home, Some(&folder)));
        match chosen {
            Ok(()) => return to,
            Err(error) => {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Warning)
                    .set_title("WhatZap")
                    .set_description(format!("WhatZap cannot use that folder: {error}"))
                    .show();
            }
        }
    }
}

/// Nothing of ZapFast's on this computer yet, under this name or an
/// earlier one (whose data a normal start adopts).
fn fresh(home: &AppDirs) -> bool {
    let earlier = ["fastsapp", "fastwhatsapp"].iter().any(|name| {
        directories::ProjectDirs::from("me", "paolino", name).is_some_and(|project| {
            project.config_dir().exists() || project.data_local_dir().exists()
        })
    });
    !earlier
        && !marker(home, FOLDER).exists()
        && [&home.config, &home.state, &home.cache]
            .iter()
            .all(|dir| !dir.exists())
}

/// Makes a change asked for in Settings or on the link screen. Call while
/// holding the single-instance guard and before anything opens a file under
/// `dirs`. Returns the directories to use and, if a change was asked for,
/// what to tell the user. A change that fails leaves everything in place.
pub fn finish_pending_change(dirs: AppDirs) -> (AppDirs, Option<Result<String, String>>) {
    finish_with(&current_home(), dirs, &Ops::real())
}

/// The operations a change performs on the keyring and the disk.
struct Ops<'a> {
    copy_key: &'a dyn Fn(&Path, &Path) -> io::Result<()>,
    has_key: &'a dyn Fn(&Path) -> io::Result<bool>,
    rename: &'a dyn Fn(&Path, &Path) -> io::Result<()>,
}

impl Ops<'static> {
    fn real() -> Self {
        Self {
            copy_key: &copy_key,
            has_key: &has_key,
            rename: &rename,
        }
    }
}

fn copy_key(from: &Path, to: &Path) -> io::Result<()> {
    crate::archive::copy_archive_key(from, to)
        .map_err(|error| io::Error::other(format!("{error:#}")))
}

fn has_key(archive: &Path) -> io::Result<bool> {
    crate::archive::has_archive_key(archive).map_err(|error| io::Error::other(format!("{error:#}")))
}

fn rename(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

fn finish_with(
    home: &AppDirs,
    dirs: AppDirs,
    ops: &Ops,
) -> (AppDirs, Option<Result<String, String>>) {
    let Some(folder) = pending_in(home) else {
        return (dirs, None);
    };
    // Taken before trying, so a change that fails waits to be asked again
    // rather than failing at every start.
    if let Err(error) = remove_if_present(&marker(home, CHANGE)) {
        return (
            dirs,
            Some(Err(format!(
                "Could not take the data folder change: {error}"
            ))),
        );
    }
    let to = destination(home, folder.as_deref());
    let plan = match plan(&dirs, &to, folder.as_deref(), ops.has_key) {
        Ok(plan) => plan,
        Err(error) => {
            return (
                dirs,
                Some(Err(format!(
                    "WhatZap's data folder was not changed: {error}"
                ))),
            );
        }
    };
    let leftovers = match plan {
        Plan::Switch => Vec::new(),
        Plan::Move => match move_data(&dirs, &to, ops) {
            Ok(leftovers) => leftovers,
            Err(error) => {
                return (
                    dirs,
                    Some(Err(format!("WhatZap's data was not moved: {error}"))),
                );
            }
        },
    };
    if let Err(error) = record(home, folder.as_deref()) {
        return match plan {
            Plan::Switch => (
                dirs,
                Some(Err(format!("Could not switch the data folder: {error}"))),
            ),
            Plan::Move => (
                to.clone(),
                Some(Err(format!(
                    "WhatZap's data moved to {}, but that could not be recorded ({error}). Fix {} before the next start",
                    shown(&to),
                    marker(home, FOLDER).display()
                ))),
            ),
        };
    }
    let mut message = match plan {
        Plan::Move => format!("WhatZap's data is now in {}", shown(&to)),
        Plan::Switch => format!(
            "Using the WhatZap data in {}; the previous data stays in {}",
            shown(&to),
            shown(&dirs)
        ),
    };
    for old in leftovers {
        if fs::remove_dir_all(&old).is_err() {
            message.push_str(&format!(
                ". The old copy in {} could not be removed; delete it yourself",
                old.display()
            ));
        }
    }
    (to, Some(Ok(message)))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Plan {
    /// The destination is empty: the data moves there.
    Move,
    /// The destination holds ZapFast data: use it, and leave ours.
    Switch,
}

/// What changing from `from` to `to` would do, or why it cannot.
fn plan(
    from: &AppDirs,
    to: &AppDirs,
    folder: Option<&Path>,
    has_key: &dyn Fn(&Path) -> io::Result<bool>,
) -> io::Result<Plan> {
    if normal(&from.state) == normal(&to.state) {
        return Err(io::Error::other("WhatZap's data is already there"));
    }
    let ours = [&from.config, &from.state, &from.cache];
    for source in ours {
        for dest in [&to.config, &to.state, &to.cache] {
            let (source, dest) = (normal(source), normal(dest));
            if source.starts_with(&dest) || dest.starts_with(&source) {
                return Err(io::Error::other(format!(
                    "{} is inside WhatZap's data or holds it; choose another folder",
                    dest.display()
                )));
            }
        }
    }
    if has_session(&to.state) {
        // Each archive must open here: its key is tied to its folder's path,
        // so a folder copied by hand or from another computer has none.
        for archive in archives(&to.state) {
            if !has_key(&archive)? {
                return Err(io::Error::other(format!(
                    "this computer's keyring has no key for the archive in {}, so its history could not be read. Use a folder WhatZap set up or moved here",
                    archive.parent().unwrap_or(&archive).display()
                )));
            }
        }
        return Ok(Plan::Switch);
    }
    let files = match folder {
        Some(folder) => tally(folder)?.0,
        None => tally(&to.config)?.0 + tally(&to.state)?.0 + tally(&to.cache)?.0,
    };
    if files == 0 {
        Ok(Plan::Move)
    } else {
        Err(io::Error::other(format!(
            "{} holds other files; choose an empty folder, or a WhatZap data folder (the one with config, state and cache in it)",
            shown(to)
        )))
    }
}

/// Moves config, state and cache from `from` to `to`, all or nothing.
/// Returns the sources that were copied rather than renamed, for removal
/// once the new place is recorded.
fn move_data(from: &AppDirs, to: &AppDirs, ops: &Ops) -> io::Result<Vec<PathBuf>> {
    // Each archive's key goes to its new path and is read back before the
    // only copy of the history moves; the old key stays for backups.
    for archive in archives(&from.state) {
        let relative = archive
            .strip_prefix(&from.state)
            .map_err(io::Error::other)?;
        (ops.copy_key)(&archive, &to.state.join(relative))?;
    }
    let mut done: Vec<(&PathBuf, &PathBuf, bool)> = Vec::new();
    for (source, dest) in [
        (&from.config, &to.config),
        (&from.state, &to.state),
        (&from.cache, &to.cache),
    ] {
        if !source.exists() {
            continue;
        }
        match transfer(source, dest, ops.rename) {
            Ok(copied) => done.push((source, dest, copied)),
            Err(error) => {
                for (source, dest, copied) in done {
                    if copied {
                        let _ = fs::remove_dir_all(dest);
                    } else {
                        let _ = fs::rename(dest, source);
                    }
                }
                return Err(error);
            }
        }
    }
    Ok(done
        .into_iter()
        .filter(|(_, _, copied)| *copied)
        .map(|(source, _, _)| source.clone())
        .collect())
}

/// Renames `source` to `dest`, or, across drives, copies and checks it.
/// Reports whether it copied, leaving the source in place.
fn transfer(
    source: &Path,
    dest: &Path,
    rename: &dyn Fn(&Path, &Path) -> io::Result<()>,
) -> io::Result<bool> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    // Holds no files (checked), but copying a key makes its folder, and a
    // rename needs the name free.
    prune_empty(dest)?;
    if rename(source, dest).is_ok() {
        return Ok(false);
    }
    let copied = copy_tree(source, dest).and_then(|()| {
        if tally(source)? == tally(dest)? {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "the copy in {} does not match the original",
                dest.display()
            )))
        }
    });
    match copied {
        Ok(()) => Ok(true),
        Err(error) => {
            let _ = fs::remove_dir_all(dest);
            Err(error)
        }
    }
}

/// Keeps other copies of ZapFast off this data folder while the returned
/// file stays open: two copies would share one linked device and break its
/// encryption sessions.
pub fn lock(dirs: &AppDirs) -> io::Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dirs.state.join("zapfast.lock"))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(fs::TryLockError::WouldBlock) => Err(io::Error::other(format!(
            "Another copy of WhatZap is using the data in {}",
            shown(dirs)
        ))),
        Err(fs::TryLockError::Error(error)) => Err(error),
    }
}

/// The folder a user would recognise: the one holding `state`.
fn shown(dirs: &AppDirs) -> String {
    dirs.state
        .parent()
        .unwrap_or(&dirs.state)
        .display()
        .to_string()
}

fn record(home: &AppDirs, folder: Option<&Path>) -> io::Result<()> {
    let file = marker(home, FOLDER);
    match folder {
        Some(folder) => write_atomic(
            &file,
            folder
                .to_str()
                .ok_or_else(|| io::Error::other("the folder's name is not valid Unicode"))?,
        ),
        None => remove_if_present(&file),
    }
}

/// A linked or linking account: a session database at the root (from before
/// accounts) or in an account folder.
fn has_session(state: &Path) -> bool {
    state.join("session.db").is_file()
        || fs::read_dir(state.join("accounts"))
            .into_iter()
            .flatten()
            .flatten()
            .any(|entry| entry.path().join("session.db").is_file())
}

/// Every archive under a state directory: one per account, and the root one
/// of a setup from before accounts.
fn archives(state: &Path) -> Vec<PathBuf> {
    let accounts = fs::read_dir(state.join("accounts"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path().join("archive.db"));
    std::iter::once(state.join("archive.db"))
        .chain(accounts)
        .filter(|path| path.is_file())
        .collect()
}

/// Files and bytes under `dir`; nothing when it does not exist.
fn tally(dir: &Path) -> io::Result<(u64, u64)> {
    if !dir.exists() {
        return Ok((0, 0));
    }
    let mut total = (0, 0);
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let (files, bytes) = tally(&entry.path())?;
            total = (total.0 + files, total.1 + bytes);
        } else {
            total = (total.0 + 1, total.1 + entry.metadata()?.len());
        }
    }
    Ok(total)
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Removes `dir` if it holds only empty folders.
fn prune_empty(dir: &Path) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            prune_empty(&path)?;
        }
    }
    if fs::read_dir(dir)?.next().is_none() {
        fs::remove_dir(dir)?;
    }
    Ok(())
}

/// The path with its existing part resolved, so paths compare whatever
/// their spelling.
fn normal(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(path) => path,
        Err(_) => match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) => normal(parent).join(name),
            _ => path.to_path_buf(),
        },
    }
}

fn read_path(file: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(file).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| PathBuf::from(text))
}

fn write_atomic(file: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = file.as_os_str().to_os_string();
    temporary.push(".tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, file)
}

fn remove_if_present(file: &Path) -> io::Result<()> {
    match fs::remove_file(file) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A linked one-account profile with settings, an archive, a log and a
    /// cached picture.
    fn profile(root: &Path, history: &[u8]) -> AppDirs {
        let dirs = AppDirs::under(root);
        let account = dirs.state.join("accounts").join("1");
        fs::create_dir_all(&account).unwrap();
        fs::write(account.join("session.db"), b"device").unwrap();
        fs::write(account.join("archive.db"), history).unwrap();
        fs::write(dirs.state.join("zapfast.log"), b"log").unwrap();
        fs::create_dir_all(&dirs.config).unwrap();
        fs::write(dirs.config.join("settings.json"), b"{}").unwrap();
        fs::create_dir_all(dirs.cache.join("accounts/1/media")).unwrap();
        fs::write(dirs.cache.join("accounts/1/media/photo.jpg"), b"photo").unwrap();
        dirs
    }

    fn ask(home: &AppDirs, folder: Option<&Path>) {
        let text = folder.map(|folder| folder.to_str().unwrap()).unwrap_or("");
        write_atomic(&marker(home, CHANGE), text).unwrap();
    }

    fn same_drive(from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn other_drive(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::other("not the same device"))
    }

    /// Makes the new archive's folder, as the keyring copy does.
    fn copy_key_here(_: &Path, to: &Path) -> io::Result<()> {
        fs::create_dir_all(to.parent().unwrap())
    }

    fn key_here(_: &Path) -> io::Result<bool> {
        Ok(true)
    }

    fn ops<'a>(rename: &'a dyn Fn(&Path, &Path) -> io::Result<()>) -> Ops<'a> {
        Ops {
            copy_key: &copy_key_here,
            has_key: &key_here,
            rename,
        }
    }

    fn history(dirs: &AppDirs) -> Vec<u8> {
        fs::read(dirs.state.join("accounts/1/archive.db")).unwrap()
    }

    fn assert_moved(home: &AppDirs, dirs: &AppDirs, folder: &Path) {
        assert_eq!(dirs.state, folder.join("state"));
        assert_eq!(dirs.config, folder.join("config"));
        assert_eq!(dirs.runtime, home.runtime, "the instance lock stays put");
        assert_eq!(history(dirs), b"history");
        assert!(dirs.config.join("settings.json").is_file());
        assert!(dirs.cache.join("accounts/1/media/photo.jpg").is_file());
        for dir in [&home.config, &home.state, &home.cache] {
            assert!(!dir.exists(), "{} was left behind", dir.display());
        }
        assert_eq!(read_path(&marker(home, FOLDER)), Some(folder.to_path_buf()));
        assert_eq!(pending_in(home), None);
        assert_eq!(relocate_from(home).state, dirs.state);
    }

    #[test]
    fn a_chosen_folder_holds_the_whole_profile_but_not_the_lock() {
        let root = tempfile::tempdir().unwrap();
        let home = AppDirs::under(&root.path().join("home"));
        let folder = root.path().join("chosen");
        record(&home, Some(&folder)).unwrap();
        assert_eq!(
            marker(&home, FOLDER),
            root.path().join("home/config.folder")
        );
        let dirs = relocate_from(&home);
        assert_eq!(dirs.config, folder.join("config"));
        assert_eq!(dirs.state, folder.join("state"));
        assert_eq!(dirs.cache, folder.join("cache"));
        assert_eq!(dirs.runtime, home.runtime);
    }

    #[test]
    fn the_development_variable_moves_everything_including_the_lock() {
        let root = tempfile::tempdir().unwrap();
        let standard = AppDirs::under(&root.path().join("standard"));
        let dev = root.path().join("dev");
        let dirs = home(standard.clone(), Some(dev.clone().into_os_string()));
        assert_eq!(dirs.config, dev.join("config"));
        assert_eq!(dirs.runtime, dev.join("run"));
        assert_eq!(
            home(standard.clone(), Some(OsString::new())).state,
            standard.state,
            "an empty variable is unset"
        );
    }

    #[test]
    fn an_empty_folder_on_the_same_drive_receives_the_data() {
        let root = tempfile::tempdir().unwrap();
        let home = profile(&root.path().join("home"), b"history");
        let folder = root.path().join("chosen");
        fs::create_dir_all(&folder).unwrap();
        ask(&home, Some(&folder));
        let copies = std::cell::RefCell::new(Vec::new());
        let copy = |from: &Path, to: &Path| {
            copies.borrow_mut().push(to.to_path_buf());
            copy_key_here(from, to)
        };
        let ops = Ops {
            copy_key: &copy,
            has_key: &key_here,
            rename: &same_drive,
        };
        let (dirs, said) = finish_with(&home, home.clone(), &ops);
        assert!(matches!(said, Some(Ok(_))), "{said:?}");
        assert_eq!(
            copies.into_inner(),
            [folder.join("state/accounts/1/archive.db")]
        );
        assert_moved(&home, &dirs, &folder);
    }

    #[test]
    fn an_empty_folder_on_another_drive_gets_a_checked_copy() {
        let root = tempfile::tempdir().unwrap();
        let home = profile(&root.path().join("home"), b"history");
        let folder = root.path().join("chosen");
        ask(&home, Some(&folder));
        let (dirs, said) = finish_with(&home, home.clone(), &ops(&other_drive));
        assert!(matches!(said, Some(Ok(_))), "{said:?}");
        assert_moved(&home, &dirs, &folder);
    }

    #[test]
    fn a_folder_with_zapfast_data_is_used_and_ours_stays() {
        let root = tempfile::tempdir().unwrap();
        let home = profile(&root.path().join("home"), b"history");
        let folder = root.path().join("earlier");
        profile(&folder, b"earlier history");
        ask(&home, Some(&folder));
        let (dirs, said) = finish_with(&home, home.clone(), &ops(&same_drive));
        assert!(matches!(said, Some(Ok(_))), "{said:?}");
        assert_eq!(dirs.state, folder.join("state"));
        assert_eq!(history(&dirs), b"earlier history");
        assert_eq!(history(&home), b"history", "our data is untouched");
        assert_eq!(relocate_from(&home).state, dirs.state);
        // And back: the standard place still holds the data, so it is used.
        ask(&home, None);
        let (back, said) = finish_with(&home, dirs, &ops(&same_drive));
        assert!(matches!(said, Some(Ok(_))), "{said:?}");
        assert_eq!(back.state, home.state);
        assert_eq!(history(&back), b"history");
        assert_eq!(history(&inside(&home, &folder)), b"earlier history");
        assert_eq!(read_path(&marker(&home, FOLDER)), None);
    }

    #[test]
    fn a_folder_whose_archive_key_is_elsewhere_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let home = profile(&root.path().join("home"), b"history");
        let folder = root.path().join("copied");
        profile(&folder, b"copied history");
        ask(&home, Some(&folder));
        let ops = Ops {
            copy_key: &copy_key_here,
            has_key: &|_| Ok(false),
            rename: &same_drive,
        };
        let (dirs, said) = finish_with(&home, home.clone(), &ops);
        assert!(matches!(said, Some(Err(_))), "{said:?}");
        assert_eq!(dirs.state, home.state);
        assert_eq!(read_path(&marker(&home, FOLDER)), None);
    }

    #[test]
    fn other_files_or_a_failed_key_copy_change_nothing() {
        let root = tempfile::tempdir().unwrap();
        let home = profile(&root.path().join("home"), b"history");
        let busy = root.path().join("busy");
        fs::create_dir_all(&busy).unwrap();
        fs::write(busy.join("notes.txt"), b"theirs").unwrap();
        let empty = root.path().join("empty");
        let failing = |_: &Path, _: &Path| Err(io::Error::other("keyring locked"));
        for (folder, copy_key) in [
            (
                &busy,
                &copy_key_here as &dyn Fn(&Path, &Path) -> io::Result<()>,
            ),
            (&empty, &failing),
        ] {
            ask(&home, Some(folder));
            let ops = Ops {
                copy_key,
                has_key: &key_here,
                rename: &same_drive,
            };
            let (dirs, said) = finish_with(&home, home.clone(), &ops);
            assert!(matches!(said, Some(Err(_))), "{said:?}");
            assert_eq!(dirs.state, home.state);
            assert_eq!(history(&home), b"history");
            assert_eq!(pending_in(&home), None, "the change is not retried");
            assert_eq!(read_path(&marker(&home, FOLDER)), None);
        }
        assert_eq!(fs::read(busy.join("notes.txt")).unwrap(), b"theirs");
    }

    #[test]
    fn a_move_back_to_an_empty_standard_place_takes_everything() {
        let root = tempfile::tempdir().unwrap();
        let home = profile(&root.path().join("home"), b"history");
        let folder = root.path().join("chosen");
        ask(&home, Some(&folder));
        let (moved, _) = finish_with(&home, home.clone(), &ops(&same_drive));
        ask(&home, None);
        let (back, said) = finish_with(&home, moved, &ops(&same_drive));
        assert!(matches!(said, Some(Ok(_))), "{said:?}");
        assert_eq!(back.state, home.state);
        assert_eq!(history(&back), b"history");
        assert!(home.config.join("settings.json").is_file());
        assert_eq!(read_path(&marker(&home, FOLDER)), None);
        assert!(!folder.join("state").exists());
    }

    #[test]
    fn a_second_copy_cannot_take_a_folder_in_use() {
        let root = tempfile::tempdir().unwrap();
        let dirs = profile(root.path(), b"history");
        let held = lock(&dirs).unwrap();
        assert!(lock(&dirs).is_err());
        drop(held);
        assert!(lock(&dirs).is_ok());
    }
}
