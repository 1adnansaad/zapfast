//! The picker's yt-dlp tab: a strip of sites like the sticker shelves, then
//! the chosen site's search or link field and the videos found, or the
//! download of yt-dlp itself when it is missing.

use egui::{Key, Modifiers, Rect, Sense, Stroke, Vec2, vec2};

use super::{SHELF_TAB, search_box};
use crate::app::App;
use crate::model::Action;
use crate::theme::{self, Icon, Palette};
use crate::ui::widgets;
use crate::ytdlp::{FFMPEG_DOWNLOAD, Tool, ToolKind, VideoSite, WebVideo, is_link};

const SEARCH_ID: &str = "ytdlp-search";

fn icon(site: VideoSite) -> Icon {
    match site {
        VideoSite::YouTube => Icon::YouTube,
        VideoSite::X => Icon::XLogo,
        VideoSite::Instagram => Icon::Instagram,
        VideoSite::Facebook => Icon::Facebook,
        VideoSite::TikTok => Icon::TikTok,
        VideoSite::Link => Icon::Link,
    }
}

/// Where a site's tab was drawn, so tests can click it.
pub fn site_tab_id(site: VideoSite) -> egui::Id {
    egui::Id::new(("ytdlp-site-tab", site))
}

/// Where the first video was drawn, so tests can click it.
pub fn first_video_id() -> egui::Id {
    egui::Id::new("ytdlp-first-video")
}

pub(super) fn tab(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    site_strip(app, ui, palette);
    ui.add_space(2.0);
    let Some(tools) = app.ytdlp.tools.clone() else {
        busy(ui, palette, "Looking for yt-dlp…");
        return;
    };
    match &tools.ytdlp {
        Tool::Ready => {}
        Tool::Installing => return busy(ui, palette, "Downloading yt-dlp…"),
        missing => return offer_ytdlp(app, ui, palette, missing),
    }
    let site = app.ytdlp.site;
    let submit = ui.memory(|memory| memory.has_focus(egui::Id::new(SEARCH_ID)))
        && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
    let mut query = app.picker_search.clone();
    search_box(ui, palette, SEARCH_ID, &mut query, site.hint());
    if query != app.picker_search {
        app.picker_search = query.clone();
    }
    let text = query.trim();
    let readable = site.searchable() || is_link(text);
    let repeat = text == app.ytdlp.query && app.ytdlp.error.is_none();
    if submit && readable && !text.is_empty() && !repeat && !app.ytdlp.pending {
        app.actions.push(Action::FindWebVideos(text.to_owned()));
    }
    ffmpeg_line(app, ui, palette, &tools.ffmpeg);
    if !readable && !text.is_empty() {
        theme::paragraph(
            ui,
            "Paste a link: yt-dlp searches only YouTube.",
            theme::regular(13.0),
            palette.secondary,
        );
    } else if app.ytdlp.pending {
        busy(ui, palette, "Looking…");
    } else if let Some(error) = &app.ytdlp.error {
        theme::paragraph(ui, error, theme::regular(13.0), palette.danger);
    }
    results(app, ui, palette);
}

/// The row of site tabs above the field.
fn site_strip(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let current = app.ytdlp.site;
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for site in VideoSite::ALL {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(SHELF_TAB), Sense::click());
            ui.ctx()
                .data_mut(|data| data.insert_temp(site_tab_id(site), rect));
            let selected = site == current;
            if ui.is_rect_visible(rect) {
                if selected {
                    ui.painter().rect_filled(
                        rect.shrink(1.0),
                        6.0,
                        palette.accent.gamma_multiply(0.22),
                    );
                } else if response.hovered() {
                    ui.painter()
                        .rect_filled(rect.shrink(1.0), 6.0, palette.surface_hover);
                }
                let color = if selected {
                    palette.text
                } else {
                    palette.secondary
                };
                theme::paint_icon(ui, icon(site), rect, 17.0, color);
                if selected {
                    ui.painter().hline(
                        rect.x_range().shrink(6.0),
                        rect.bottom() - 2.0,
                        Stroke::new(2.0, palette.accent),
                    );
                }
            }
            if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(site.label())
                .clicked()
                && !selected
            {
                picked = Some(site);
            }
        }
    });
    if let Some(site) = picked {
        app.actions.push(Action::SelectVideoSite(site));
    }
}

fn busy(ui: &mut egui::Ui, palette: &Palette, label: &str) {
    ui.horizontal(|ui| {
        theme::spinner(ui, 16.0, palette.accent);
        theme::text(ui, label, theme::regular(12.5), palette.secondary);
    });
}

/// In place of the tab while yt-dlp is missing.
fn offer_ytdlp(app: &mut App, ui: &mut egui::Ui, palette: &Palette, tool: &Tool) {
    ui.add_space(8.0);
    if let Tool::Failed(error) = tool {
        theme::paragraph(ui, error, theme::regular(13.0), palette.danger);
        ui.add_space(6.0);
    }
    theme::paragraph(
        ui,
        "Videos from YouTube, X, Instagram, Facebook, TikTok and other sites come through yt-dlp, which is not on this computer. Install it yourself (WhatZap finds it on PATH), or download it from yt-dlp's GitHub releases, about 18 MB.",
        theme::regular(13.0),
        palette.text,
    );
    ui.add_space(6.0);
    if theme::soft_button(ui, palette, Some(Icon::Download), "Download yt-dlp", false).clicked() {
        app.actions.push(Action::InstallTool(ToolKind::YtDlp));
    }
}

/// A quiet line while ffmpeg, which joins the best video and sound, is
/// missing.
fn ffmpeg_line(app: &mut App, ui: &mut egui::Ui, palette: &Palette, tool: &Tool) {
    match tool {
        Tool::Ready => {}
        Tool::Installing => busy(ui, palette, "Downloading ffmpeg…"),
        Tool::Missing | Tool::Failed(_) => {
            if let Tool::Failed(error) = tool {
                theme::paragraph(ui, error, theme::regular(12.5), palette.danger);
            }
            ui.horizontal(|ui| {
                if FFMPEG_DOWNLOAD {
                    theme::text(
                        ui,
                        "Without ffmpeg, YouTube comes in 360p.",
                        theme::regular(12.5),
                        palette.dim,
                    );
                    if theme::link(ui, "Download it (85 MB)", theme::medium(12.5), palette.link)
                        .clicked()
                    {
                        app.actions.push(Action::InstallTool(ToolKind::Ffmpeg));
                    }
                } else {
                    theme::text(
                        ui,
                        "Install ffmpeg for YouTube above 360p.",
                        theme::regular(12.5),
                        palette.dim,
                    );
                }
            });
        }
    }
}

/// Two columns of thumbnails with their length and title; a click sends.
fn results(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let videos = app.ytdlp.results.clone();
    let columns = 2;
    let gap = 8.0;
    let width = (ui.available_width() - gap * (columns as f32 - 1.0)) / columns as f32;
    let picture = vec2(width, width * 9.0 / 16.0);
    let mut picked = None;
    egui::ScrollArea::vertical()
        .id_salt("ytdlp-grid")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            for (row, pair) in videos.chunks(columns).enumerate() {
                ui.horizontal(|ui| {
                    for (column, video) in pair.iter().enumerate() {
                        let (rect, response) =
                            ui.allocate_exact_size(vec2(width, picture.y + 20.0), Sense::click());
                        if row == 0 && column == 0 {
                            ui.ctx()
                                .data_mut(|data| data.insert_temp(first_video_id(), rect));
                        }
                        if ui.is_rect_visible(rect) {
                            tile(ui, palette, video, rect, picture, response.hovered());
                        }
                        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let response = if video.title.is_empty() {
                            response
                        } else {
                            response.on_hover_text(&video.title)
                        };
                        if response.clicked() {
                            picked = Some(video.clone());
                        }
                    }
                });
            }
            if videos.is_empty() && !app.ytdlp.pending && app.ytdlp.error.is_none() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    theme::paragraph(
                        ui,
                        if app.ytdlp.site.searchable() {
                            "Search, or paste a link. Videos up to 64 MB send to the open chat."
                        } else {
                            "Paste a link. Videos up to 64 MB send to the open chat."
                        },
                        theme::regular(13.0),
                        palette.secondary,
                    );
                });
            }
        });
    if let Some(video) = picked {
        app.actions.push(Action::SendWebVideo(video));
    }
}

fn tile(
    ui: &mut egui::Ui,
    palette: &Palette,
    video: &WebVideo,
    rect: Rect,
    picture: Vec2,
    hovered: bool,
) {
    let frame = Rect::from_min_size(rect.min, picture);
    ui.painter().rect_filled(frame, 6.0, palette.surface);
    match &video.thumbnail {
        Some(path) => {
            ui.put(
                frame,
                widgets::file_image(ui, path)
                    .fit_to_exact_size(picture)
                    .corner_radius(6.0),
            );
        }
        None => theme::paint_icon(ui, Icon::Video, frame, 22.0, palette.dim),
    }
    if let Some(seconds) = video.duration {
        let galley =
            ui.painter()
                .layout_no_wrap(clock(seconds), theme::medium(11.0), egui::Color32::WHITE);
        let badge = Rect::from_min_size(
            frame.right_bottom() - galley.size() - vec2(10.0, 7.0),
            galley.size() + vec2(6.0, 2.0),
        );
        ui.painter()
            .rect_filled(badge, 3.0, egui::Color32::from_black_alpha(170));
        ui.painter()
            .galley(badge.min + vec2(3.0, 1.0), galley, egui::Color32::WHITE);
    }
    if hovered {
        ui.painter().rect_stroke(
            frame,
            6.0,
            Stroke::new(2.0, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
    let title = Rect::from_min_size(
        frame.left_bottom() + vec2(0.0, 2.0),
        vec2(picture.x, rect.bottom() - frame.bottom() - 2.0),
    );
    let mut line = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(title)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    theme::text(&mut line, &video.title, theme::regular(12.5), palette.text);
}

/// A length as a video player shows it: `4:05`, `1:02:03`.
fn clock(seconds: u32) -> String {
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ytdlp::Tools;

    #[test]
    fn lengths_read_like_a_player() {
        assert_eq!(clock(0), "0:00");
        assert_eq!(clock(245), "4:05");
        assert_eq!(clock(3723), "1:02:03");
    }

    fn click(pos: egui::Pos2) -> Vec<egui::Event> {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        vec![egui::Event::PointerMoved(pos), button(true), button(false)]
    }

    fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) {
        let palette = app.palette;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 420.0));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            },
            |ui| tab(app, ui, &palette),
        );
        output.textures_delta.clear();
    }

    #[test]
    fn a_site_tab_is_chosen_and_a_found_video_is_sent() {
        let directory = tempfile::tempdir().expect("creates a temporary directory");
        let (mut app, _events) = App::headless(
            crate::paths::AppDirs::under(directory.path()),
            crate::settings::Settings::default(),
        );
        app.ytdlp.tools = Some(Tools {
            ytdlp: Tool::Ready,
            ffmpeg: Tool::Missing,
        });
        let video = WebVideo {
            id: "Youtube-a1".into(),
            url: "https://www.youtube.com/watch?v=a1".into(),
            title: "Fixture".into(),
            thumbnail: None,
            duration: Some(61),
        };
        app.ytdlp.results = vec![video.clone()];
        let ctx = egui::Context::default();
        app.attach(&ctx);
        frame(&mut app, &ctx, Vec::new());
        let tiktok = ctx
            .data(|data| data.get_temp::<Rect>(site_tab_id(VideoSite::TikTok)))
            .expect("the strip draws a tab per site");
        frame(&mut app, &ctx, click(tiktok.center()));
        assert!(
            app.actions
                .contains(&Action::SelectVideoSite(VideoSite::TikTok))
        );
        app.actions.clear();
        let first = ctx
            .data(|data| data.get_temp::<Rect>(first_video_id()))
            .expect("a result is drawn");
        frame(&mut app, &ctx, click(first.center()));
        assert!(app.actions.contains(&Action::SendWebVideo(video)));
    }
}
