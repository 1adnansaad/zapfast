//! The picker's yt-dlp tab on the worker. yt-dlp runs on blocking threads
//! and reports back through `Command::YtDlpReport`; a fetched video goes out
//! through `send_files`, like a file from the composer.

use super::*;
use crate::ytdlp::{Fetch, Tool, ToolKind};

impl Worker {
    /// Reports which tools can run, after downloading one if asked. A yt-dlp
    /// downloaded here then gets its weekly update.
    pub(super) fn ytdlp_tools(&mut self, install: Option<ToolKind>) {
        let commands = self.commands.clone();
        let dir = self.dirs.tools_dir();
        tokio::task::spawn_blocking(move || {
            let failed = install.and_then(|kind| {
                crate::ytdlp::install(kind, &dir)
                    .err()
                    .map(|error| (kind, error))
            });
            let mut tools = crate::ytdlp::tools(&dir);
            if let Some((kind, error)) = failed {
                let tool = tools.get_mut(kind);
                if *tool == Tool::Missing {
                    *tool = Tool::Failed(error);
                }
            }
            let _ = commands.send(Command::YtDlpReport(Box::new(Event::YtDlpTools(tools))));
            if install.is_none() {
                crate::ytdlp::refresh(&dir);
            }
        });
    }

    pub(super) fn find_web_videos(&mut self, query: String) {
        let commands = self.commands.clone();
        let dir = self.dirs.tools_dir();
        let thumbs = self.dirs.cache.join("yt-dlp").join("thumbs");
        tokio::task::spawn_blocking(move || {
            let results = crate::ytdlp::find(&dir, &query, &thumbs);
            let _ = commands.send(Command::YtDlpReport(Box::new(Event::WebVideos {
                query,
                results,
            })));
        });
    }

    /// Fetches the video, reporting its progress to its card. With a chat it
    /// is then sent as the composer sends a file, which checks the reply and
    /// the connection when it goes; without one it is copied to the download
    /// folder.
    pub(super) fn fetch_web_video(
        &mut self,
        url: String,
        send: Option<(ChatId, Option<crate::model::ReplyTarget>)>,
    ) {
        let commands = self.commands.clone();
        let dir = self.dirs.tools_dir();
        let into = self.dirs.cache.join("yt-dlp").join("videos");
        // The folder chosen in Settings itself, else the system's Downloads:
        // the cache attachments fall back to is no place for a video asked
        // for, and it came from no chat, so no account or chat folder.
        let folder = match &self.download_folder {
            Some(root) => root.clone(),
            None => directories::UserDirs::new()
                .and_then(|dirs| dirs.download_dir().map(Path::to_path_buf))
                .unwrap_or_else(|| self.dirs.media_cache_dir()),
        };
        tokio::task::spawn_blocking(move || {
            let report = |fetch| {
                let event = Event::WebVideo {
                    url: url.clone(),
                    fetch,
                };
                let _ = commands.send(Command::YtDlpReport(Box::new(event)));
            };
            let sending = send.is_some();
            let fetched = crate::ytdlp::download(&dir, &url, &into, |fraction| {
                report(Fetch::Running {
                    send: sending,
                    fraction: Some(fraction),
                })
            });
            let fetch = match (fetched, send) {
                (Err(error), _) => Fetch::Failed(error),
                (Ok(path), Some((chat, quoting))) => {
                    let _ = commands.send(Command::SendFiles {
                        chat,
                        paths: vec![path],
                        documents: false,
                        view_once: false,
                        caption: None,
                        mentions: Vec::new(),
                        quoting,
                    });
                    Fetch::Sent
                }
                (Ok(path), None) => save(&path, &folder).map_or_else(Fetch::Failed, Fetch::Saved),
            };
            report(fetch);
        });
    }
}

/// Copies a fetched video into `folder` under its own name. The name is the
/// site and the video's id, so a copy already there is the same video.
fn save(path: &Path, folder: &Path) -> Result<PathBuf, String> {
    let name = path.file_name().ok_or("yt-dlp named no file")?;
    let target = folder.join(name);
    std::fs::create_dir_all(folder)
        .and_then(|()| std::fs::copy(path, &target))
        .map(|_| target)
        .map_err(|error| format!("Could not save the video: {error}"))
}
