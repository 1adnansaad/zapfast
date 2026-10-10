//! The picker's yt-dlp tab on the worker. yt-dlp runs on blocking threads
//! and reports back through `Command::YtDlpReport`; a fetched video goes out
//! through `send_files`, like a file from the composer.

use super::*;
use crate::ytdlp::{Tool, ToolKind};

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

    /// Fetches the video, then sends it as the composer sends a file, which
    /// checks the reply and the connection when it goes.
    pub(super) fn send_web_video(&mut self, chat: ChatId, url: String, quoting: Option<String>) {
        let commands = self.commands.clone();
        let dir = self.dirs.tools_dir();
        let into = self.dirs.cache.join("yt-dlp").join("videos");
        tokio::task::spawn_blocking(move || {
            let next = match crate::ytdlp::download(&dir, &url, &into) {
                Ok(path) => Command::SendFiles {
                    chat,
                    paths: vec![path],
                    caption: None,
                    mentions: Vec::new(),
                    quoting,
                },
                Err(error) => Command::Sent {
                    chat,
                    id: String::new(),
                    error: Some(format!("Could not send the video: {error}")),
                },
            };
            let _ = commands.send(next);
        });
    }
}
