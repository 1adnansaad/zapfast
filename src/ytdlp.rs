//! The picker's video tab runs the yt-dlp program: finding it and ffmpeg,
//! downloading them when asked, searching YouTube, reading a link, and
//! fetching a video as an MP4 WhatsApp plays. Everything here blocks; the
//! worker calls it on blocking threads.

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::model::ATTACHMENT_DOWNLOAD_LIMIT;

/// How many results a YouTube search asks for.
const SEARCH_RESULTS: usize = 12;
/// How long a yt-dlp downloaded here goes before it updates itself: sites
/// change, and an old yt-dlp stops reading them.
const REFRESH: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Fetched videos older than this are removed at the next fetch.
const KEEP: Duration = Duration::from_secs(24 * 60 * 60);
/// Formats WhatsApp plays everywhere, small enough to send: H.264 and AAC in
/// MP4, at most 720p.
const SORT: &str = "vcodec:h264,res:720,acodec:aac,ext:mp4:m4a";
/// Without ffmpeg nothing can be joined or remuxed, so a plain file beats a
/// stream in pieces.
const SORT_SINGLE: &str = "proto:https,vcodec:h264,res:720,acodec:aac,ext:mp4:m4a";
const EXE: &str = std::env::consts::EXE_SUFFIX;
const MISSING: &str = "yt-dlp is not installed";

const YTDLP_RELEASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const YTDLP_ASSET: Option<&str> = if cfg!(all(windows, target_arch = "x86_64")) {
    Some("yt-dlp.exe")
} else if cfg!(all(windows, target_arch = "aarch64")) {
    Some("yt-dlp_arm64.exe")
} else if cfg!(target_os = "macos") {
    Some("yt-dlp_macos")
} else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Some("yt-dlp_linux")
} else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
    Some("yt-dlp_linux_aarch64")
} else {
    None
};
const FFMPEG_RELEASE: &str = "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest";
/// The shared build: the programs and their libraries, half the size of the
/// static one.
const FFMPEG_ASSET: Option<&str> = if cfg!(all(windows, target_arch = "x86_64")) {
    Some("ffmpeg-master-latest-win64-gpl-shared.zip")
} else if cfg!(all(windows, target_arch = "aarch64")) {
    Some("ffmpeg-master-latest-winarm64-gpl-shared.zip")
} else {
    None
};
/// Whether ffmpeg can be downloaded here; elsewhere it comes from the
/// package manager.
pub const FFMPEG_DOWNLOAD: bool = FFMPEG_ASSET.is_some();

/// The sites with their own tab. yt-dlp reads links from many more, which
/// `Link` takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VideoSite {
    #[default]
    YouTube,
    X,
    Instagram,
    Facebook,
    TikTok,
    Link,
}

impl VideoSite {
    pub const ALL: [Self; 6] = [
        Self::YouTube,
        Self::X,
        Self::Instagram,
        Self::Facebook,
        Self::TikTok,
        Self::Link,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::YouTube => "YouTube",
            Self::X => "X",
            Self::Instagram => "Instagram",
            Self::Facebook => "Facebook",
            Self::TikTok => "TikTok",
            Self::Link => "Other sites",
        }
    }

    /// The search field's placeholder.
    pub fn hint(self) -> &'static str {
        match self {
            Self::YouTube => "Search YouTube or paste a link",
            Self::X => "Paste a link to a post on X",
            Self::Instagram => "Paste an Instagram link",
            Self::Facebook => "Paste a Facebook link",
            Self::TikTok => "Paste a TikTok link",
            Self::Link => "Paste a link from any site yt-dlp reads",
        }
    }

    /// yt-dlp searches only YouTube; the other tabs take links.
    pub fn searchable(self) -> bool {
        self == Self::YouTube
    }
}

/// A video yt-dlp found.
#[derive(Clone, Debug, PartialEq)]
pub struct WebVideo {
    pub id: String,
    /// Its page, which yt-dlp fetches it from.
    pub url: String,
    pub title: String,
    /// Downloaded thumbnail.
    pub thumbnail: Option<PathBuf>,
    /// Length in seconds.
    pub duration: Option<u32>,
}

/// What the picker's yt-dlp tab shows.
#[derive(Debug, Default)]
pub struct Picker {
    pub site: VideoSite,
    /// The search or link last asked for; answers to older ones are dropped.
    pub query: String,
    pub results: Vec<WebVideo>,
    pub pending: bool,
    pub error: Option<String>,
    /// `None` until the worker has looked.
    pub tools: Option<Tools>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    YtDlp,
    Ffmpeg,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Missing,
    Installing,
    Ready,
    Failed(String),
}

/// Whether yt-dlp and ffmpeg can run.
#[derive(Clone, Debug, PartialEq)]
pub struct Tools {
    pub ytdlp: Tool,
    pub ffmpeg: Tool,
}

impl Tools {
    pub fn get_mut(&mut self, kind: ToolKind) -> &mut Tool {
        match kind {
            ToolKind::YtDlp => &mut self.ytdlp,
            ToolKind::Ffmpeg => &mut self.ffmpeg,
        }
    }
}

/// Whether `text` is a web link rather than words to search for.
pub fn is_link(text: &str) -> bool {
    let text = text.trim().to_ascii_lowercase();
    text.starts_with("https://") || text.starts_with("http://")
}

/// Finds `name` on `path` first, then in `dir`, where downloads go.
pub fn locate(path: Option<&OsStr>, dir: &Path, name: &str) -> Option<PathBuf> {
    let file = format!("{name}{EXE}");
    // A Mac app opened from the Finder does not get Homebrew's PATH.
    let extra: &[&str] = if cfg!(target_os = "macos") {
        &["/opt/homebrew/bin", "/usr/local/bin"]
    } else {
        &[]
    };
    path.into_iter()
        .flat_map(std::env::split_paths)
        .chain(extra.iter().map(PathBuf::from))
        .chain([dir.to_path_buf()])
        .map(|folder| folder.join(&file))
        .find(|candidate| candidate.is_file())
}

fn program(dir: &Path, name: &str) -> Option<PathBuf> {
    locate(std::env::var_os("PATH").as_deref(), dir, name)
}

/// Which of yt-dlp and ffmpeg are here, with `dir` holding downloaded ones.
pub fn tools(dir: &Path) -> Tools {
    let state = |name| match program(dir, name) {
        Some(_) => Tool::Ready,
        None => Tool::Missing,
    };
    Tools {
        ytdlp: state("yt-dlp"),
        ffmpeg: state("ffmpeg"),
    }
}

/// Lets a yt-dlp downloaded into `dir` update itself once a week. One found
/// on PATH belongs to whatever installed it.
pub fn refresh(dir: &Path) {
    let ours = dir.join(format!("yt-dlp{EXE}"));
    if program(dir, "yt-dlp").as_ref() != Some(&ours) {
        return;
    }
    let stale = std::fs::metadata(&ours)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > REFRESH);
    if !stale {
        return;
    }
    let _ = base(&ours).arg("-U").output();
    // Already current, or the update failed: ask again in a week.
    let _ = std::fs::File::options()
        .write(true)
        .open(&ours)
        .and_then(|file| file.set_modified(SystemTime::now()));
}

/// Downloads a tool into `dir`, checked against its release's checksums.
pub fn install(kind: ToolKind, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    match kind {
        ToolKind::YtDlp => {
            let asset = YTDLP_ASSET
                .ok_or("There is no yt-dlp download for this system. Install yt-dlp yourself.")?;
            let target = dir.join(format!("yt-dlp{EXE}"));
            fetch_verified(YTDLP_RELEASE, "SHA2-256SUMS", asset, &target)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
        ToolKind::Ffmpeg => {
            let asset = FFMPEG_ASSET
                .ok_or("Install ffmpeg with your package manager; WhatZap finds it on PATH.")?;
            let archive = dir.join("ffmpeg.zip");
            fetch_verified(FFMPEG_RELEASE, "checksums.sha256", asset, &archive)?;
            let unpacked = unpack_ffmpeg(&archive, dir);
            let _ = std::fs::remove_file(&archive);
            unpacked
        }
    }
}

/// The hash `sums` lists for `name`, in `sha256sum` format.
fn checksum(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.trim().split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name).then(|| hash.to_ascii_lowercase())
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Streams `asset` from `release` to `target`, keeping it only if its hash
/// matches the one the release's `sums` file lists.
fn fetch_verified(release: &str, sums: &str, asset: &str, target: &Path) -> Result<(), String> {
    let agent = crate::proxy::agent();
    let listed = agent
        .get(&format!("{release}/{sums}"))
        .call()
        .and_then(|mut response| response.body_mut().read_to_string())
        .map_err(|error| format!("Could not download the checksums: {error}"))?;
    let expected =
        checksum(&listed, asset).ok_or_else(|| format!("{sums} does not list {asset}"))?;
    let part = target.with_extension("part");
    match stream(&agent, &format!("{release}/{asset}"), &part) {
        Ok(hash) if hash == expected => {
            std::fs::rename(&part, target).map_err(|error| error.to_string())
        }
        outcome => {
            let _ = std::fs::remove_file(&part);
            Err(outcome.map_or_else(
                |error| format!("Could not download {asset}: {error}"),
                |_| format!("{asset} did not match its published checksum"),
            ))
        }
    }
}

/// Writes `url` to `to` without holding it in memory, returning its SHA-256.
fn stream(agent: &ureq::Agent, url: &str, to: &Path) -> Result<String, String> {
    let mut response = agent.get(url).call().map_err(|error| error.to_string())?;
    let mut reader = response.body_mut().as_reader();
    let mut file = std::fs::File::create(to).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Ok(hex(&hasher.finalize()));
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
    }
}

/// Takes ffmpeg, ffprobe and the libraries beside them out of a build's `bin`
/// folder. Nothing lands in `dir` until every file is out.
fn unpack_ffmpeg(archive: &Path, dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|error| error.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut unpacked = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| error.to_string())?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        let in_bin = path.parent().and_then(Path::file_name) == Some(OsStr::new("bin"));
        let Some(name) = path.file_name().map(OsStr::to_owned) else {
            continue;
        };
        if !entry.is_file() || !in_bin || name.to_string_lossy().starts_with("ffplay") {
            continue;
        }
        let part = dir.join(format!("{}.part", name.to_string_lossy()));
        let mut out = std::fs::File::create(&part).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|error| error.to_string())?;
        unpacked.push((part, dir.join(name)));
    }
    if !unpacked
        .iter()
        .any(|(_, target)| target.file_name() == Some(OsStr::new(&format!("ffmpeg{EXE}"))))
    {
        for (part, _) in &unpacked {
            let _ = std::fs::remove_file(part);
        }
        return Err("The ffmpeg download did not contain ffmpeg".to_owned());
    }
    for (part, target) in unpacked {
        std::fs::rename(&part, &target).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// yt-dlp with the options every call shares.
fn base(program: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .args(["--ignore-config", "--no-warnings", "--socket-timeout", "30"])
        .stdin(Stdio::null());
    if let Some(proxy) = crate::proxy::from_settings() {
        command.arg("--proxy").arg(proxy.url());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Release builds have no console, so each run would flash one.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Runs yt-dlp, returning what it printed or the error it ended with.
fn run(mut command: Command) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("Could not run yt-dlp: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(failure(&String::from_utf8_lossy(&output.stderr))
        .unwrap_or_else(|| format!("yt-dlp stopped ({})", output.status)))
}

/// The last `ERROR:` line yt-dlp printed, else its last line.
fn failure(stderr: &str) -> Option<String> {
    let mut lines = stderr
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let last = lines.clone().next();
    lines
        .find_map(|line| line.strip_prefix("ERROR: "))
        .or(last)
        .map(str::to_owned)
}

/// Searches YouTube for `query`, or reads the video (or videos) behind a
/// link, fetching thumbnails into `thumbs`.
pub fn find(dir: &Path, query: &str, thumbs: &Path) -> Result<Vec<WebVideo>, String> {
    let ytdlp = program(dir, "yt-dlp").ok_or(MISSING)?;
    let query = query.trim();
    let target = if is_link(query) {
        query.to_owned()
    } else {
        format!("ytsearch{SEARCH_RESULTS}:{query}")
    };
    let mut command = base(&ytdlp);
    command
        .args([
            "--dump-single-json",
            "--flat-playlist",
            "--no-playlist",
            "--",
        ])
        .arg(&target);
    let found = parse(&run(command)?)?;
    if found.is_empty() {
        return Err("Nothing found".to_owned());
    }
    Ok(with_thumbnails(found, thumbs))
}

/// The videos in yt-dlp's JSON: one video, or a playlist's entries, each with
/// its thumbnail's address.
fn parse(json: &str) -> Result<Vec<(WebVideo, Option<String>)>, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|_| "yt-dlp gave an answer WhatZap could not read".to_owned())?;
    let entries: Vec<&serde_json::Value> = match value["entries"].as_array() {
        Some(entries) => entries.iter().collect(),
        None => vec![&value],
    };
    Ok(entries.into_iter().filter_map(entry).collect())
}

fn entry(value: &serde_json::Value) -> Option<(WebVideo, Option<String>)> {
    let id = value["id"].as_str()?;
    // A full answer's `url` is the stream; a flat entry's is the page.
    let url = ["webpage_url", "url"]
        .iter()
        .filter_map(|key| value[*key].as_str())
        .find(|url| is_link(url))?;
    let site = ["extractor_key", "ie_key"]
        .iter()
        .find_map(|key| value[*key].as_str())
        .unwrap_or("video");
    let thumbnail = value["thumbnail"]
        .as_str()
        .or_else(|| value["thumbnails"].as_array()?.last()?["url"].as_str())
        .map(str::to_owned);
    Some((
        WebVideo {
            id: format!("{site}-{id}"),
            url: url.to_owned(),
            title: value["title"].as_str().unwrap_or_default().to_owned(),
            thumbnail: None,
            duration: value["duration"]
                .as_f64()
                .filter(|seconds| *seconds >= 0.0)
                .map(|seconds| seconds.round() as u32),
        },
        thumbnail,
    ))
}

/// Fetches each thumbnail once, in parallel, into `dir`.
fn with_thumbnails(found: Vec<(WebVideo, Option<String>)>, dir: &Path) -> Vec<WebVideo> {
    let _ = std::fs::create_dir_all(dir);
    let mut found = found;
    std::thread::scope(|scope| {
        for (video, url) in &mut found {
            let Some(url) = url.take() else {
                continue;
            };
            let path = dir.join(format!(
                "{}.{}",
                file_name(&video.id),
                picture_extension(&url)
            ));
            if path.exists() {
                video.thumbnail = Some(path);
                continue;
            }
            let slot = &mut video.thumbnail;
            scope.spawn(move || {
                let fetched = crate::proxy::agent()
                    .get(&url)
                    .call()
                    .and_then(|mut response| response.body_mut().read_to_vec());
                if let Ok(bytes) = fetched
                    && std::fs::write(&path, bytes).is_ok()
                {
                    *slot = Some(path);
                }
            });
        }
    });
    found.into_iter().map(|(video, _)| video).collect()
}

fn file_name(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The picture type a thumbnail address names, so the image loader takes it.
fn picture_extension(url: &str) -> &'static str {
    let path = url
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    ["webp", "png", "jpeg"]
        .into_iter()
        .find(|extension| path.ends_with(&format!(".{extension}")))
        .unwrap_or("jpg")
}

/// Fetches the video at `url` into `into` as an MP4 small enough to send.
pub fn download(dir: &Path, url: &str, into: &Path) -> Result<PathBuf, String> {
    let ytdlp = program(dir, "yt-dlp").ok_or(MISSING)?;
    std::fs::create_dir_all(into).map_err(|error| error.to_string())?;
    forget_old(into);
    let limit = format!("{}M", ATTACHMENT_DOWNLOAD_LIMIT / (1024 * 1024));
    let mut command = base(&ytdlp);
    command
        .args([
            "--no-playlist",
            "--restrict-filenames",
            "--force-overwrites",
            "--no-simulate",
            "--print",
            "after_move:filepath",
            "--max-filesize",
            &limit,
            "-o",
        ])
        .arg(into.join("%(extractor_key)s-%(id)s.%(ext)s"));
    match program(dir, "ffmpeg") {
        Some(ffmpeg) => {
            command
                .args(["-f", "bv*+ba/b", "-S", SORT, "--merge-output-format", "mp4"])
                .arg("--ffmpeg-location")
                .arg(ffmpeg);
        }
        None => {
            command.args(["-f", "b", "-S", SORT_SINGLE]);
        }
    }
    command.arg("--").arg(url);
    let printed = run(command)?;
    let too_large = || {
        format!(
            "yt-dlp skipped it. It may be larger than {limit}B, the most WhatZap sends; try a shorter video."
        )
    };
    let path = printed
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .map(PathBuf::from)
        .ok_or_else(too_large)?;
    let size = std::fs::metadata(&path)
        .map_err(|error| error.to_string())?
        .len();
    if size > ATTACHMENT_DOWNLOAD_LIMIT {
        let _ = std::fs::remove_file(&path);
        return Err(too_large());
    }
    Ok(path)
}

/// Removes fetched videos older than [`KEEP`]; sent ones were copied to the
/// media folder.
fn forget_old(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > KEEP);
        if old {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_told_from_searches() {
        assert!(is_link("https://www.tiktok.com/@a/video/1"));
        assert!(is_link("  HTTP://x.com/a/status/1 "));
        assert!(!is_link("cats playing piano"));
        assert!(!is_link("--exec calc"));
    }

    #[test]
    fn tools_come_from_path_first_then_the_download_folder() {
        let path_dir = tempfile::tempdir().unwrap();
        let ours = tempfile::tempdir().unwrap();
        let tool = |dir: &Path| dir.join(format!("whatzap-fixture-tool{EXE}"));
        let path = std::env::join_paths([path_dir.path()]).unwrap();
        let find = || locate(Some(&path), ours.path(), "whatzap-fixture-tool");
        assert_eq!(find(), None);
        std::fs::write(tool(ours.path()), b"").unwrap();
        assert_eq!(find(), Some(tool(ours.path())));
        std::fs::write(tool(path_dir.path()), b"").unwrap();
        assert_eq!(find(), Some(tool(path_dir.path())));
    }

    #[test]
    fn a_checksum_is_read_for_its_file_only() {
        let sums = "AB12  yt-dlp.exe\ncd34 *yt-dlp_linux\nef56  yt-dlp.exe.zip\n";
        assert_eq!(checksum(sums, "yt-dlp.exe").as_deref(), Some("ab12"));
        assert_eq!(checksum(sums, "yt-dlp_linux").as_deref(), Some("cd34"));
        assert_eq!(checksum(sums, "yt-dlp_macos"), None);
        assert_eq!(hex(&[0, 15, 255]), "000fff");
    }

    #[test]
    fn a_search_lists_its_entries() {
        let json = r#"{"_type": "playlist", "entries": [
            {"id": "a1", "ie_key": "Youtube", "url": "https://www.youtube.com/watch?v=a1",
             "title": "First", "duration": 61.4,
             "thumbnails": [{"url": "https://i.ytimg.com/vi/a1/small.jpg"},
                            {"url": "https://i.ytimg.com/vi/a1/large.webp?x=1"}]},
            {"id": "no-page", "title": "Skipped"}
        ]}"#;
        let found = parse(json).unwrap();
        assert_eq!(found.len(), 1);
        let (video, thumbnail) = &found[0];
        assert_eq!(video.id, "Youtube-a1");
        assert_eq!(video.url, "https://www.youtube.com/watch?v=a1");
        assert_eq!(video.title, "First");
        assert_eq!(video.duration, Some(61));
        let thumbnail = thumbnail.as_deref().unwrap();
        assert_eq!(thumbnail, "https://i.ytimg.com/vi/a1/large.webp?x=1");
        assert_eq!(picture_extension(thumbnail), "webp");
    }

    #[test]
    fn a_link_reads_its_page_not_its_stream() {
        let json = r#"{"id": "99", "extractor_key": "Twitter", "title": "Post",
            "url": "https://video.twimg.com/stream.mp4",
            "webpage_url": "https://x.com/a/status/99",
            "thumbnail": "https://pbs.twimg.com/thumb.jpg"}"#;
        let (video, thumbnail) = parse(json).unwrap().remove(0);
        assert_eq!(video.url, "https://x.com/a/status/99");
        assert_eq!(video.id, "Twitter-99");
        assert_eq!(video.duration, None);
        assert_eq!(
            thumbnail.as_deref(),
            Some("https://pbs.twimg.com/thumb.jpg")
        );
        assert!(parse("not json").is_err());
    }

    #[test]
    fn the_error_yt_dlp_ended_with_is_reported() {
        let stderr = "WARNING: slow\nERROR: [youtube] a1: Video unavailable\n\n";
        assert_eq!(
            failure(stderr).as_deref(),
            Some("[youtube] a1: Video unavailable")
        );
        assert_eq!(failure("Traceback\nboom\n").as_deref(), Some("boom"));
        assert_eq!(failure(""), None);
    }
}
