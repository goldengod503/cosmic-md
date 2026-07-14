use std::path::PathBuf;

use cosmic::app::Settings;
use cosmic::iced::border;
use cosmic::iced_core::text::Highlight;
use cosmic::iced_core::{padding, Background, Color, Length};
use cosmic::iced_runtime::Appearance;
use cosmic::iced_widget::container;
use cosmic::iced_widget::text_editor as ied_text_editor;
use cosmic::theme;
use cosmic::widget::{markdown, scrollable, text_editor};
use cosmic::{executor, prelude::*, Core};

// Tokyo Night palette (Night variant) — mirrors enkia.tokyo-night.
const fn tn(hex: u32) -> Color {
    Color {
        r: ((hex >> 16) & 0xff) as f32 / 255.0,
        g: ((hex >> 8) & 0xff) as f32 / 255.0,
        b: (hex & 0xff) as f32 / 255.0,
        a: 1.0,
    }
}

const TN_BG: Color = tn(0x1a1b26);
const TN_BG_DARK: Color = tn(0x16161e);
const TN_BG_LIGHT: Color = tn(0x24283b);
const TN_FG: Color = tn(0xc0caf5);
const TN_COMMENT: Color = tn(0x565f89);
const TN_BLUE: Color = tn(0x7aa2f7);
const TN_ORANGE: Color = tn(0xcc8966);
const TN_SELECTION: Color = Color {
    r: 0x36 as f32 / 255.0,
    g: 0x4a as f32 / 255.0,
    b: 0x82 as f32 / 255.0,
    a: 0.6,
};

struct App {
    core: Core,
    items: Vec<markdown::Item>,
    editor_content: text_editor::Content,
    source: String,
    path: PathBuf,
    selectable_mode: bool,
}

#[derive(Clone, Debug)]
enum Message {
    LinkClicked(markdown::Url),
    FileChanged,
    EditorAction(text_editor::Action),
    ToggleSelectable,
}

// Group markdown items into sections (heading + following body) so the body
// can be wrapped in a lighter container — visually breaks up content under
// each heading.
fn render_sections<'a>(
    items: &'a [markdown::Item],
    settings: markdown::Settings,
    style: markdown::Style,
) -> Element<'a, markdown::Url> {
    use cosmic::widget::Column;

    let mut sections: Vec<Element<'a, markdown::Url>> = Vec::new();
    let mut pending_heading: Option<Element<'a, markdown::Url>> = None;
    let mut pending_body: Vec<Element<'a, markdown::Url>> = Vec::new();

    fn flush<'a>(
        sections: &mut Vec<Element<'a, markdown::Url>>,
        heading: Option<Element<'a, markdown::Url>>,
        body: Vec<Element<'a, markdown::Url>>,
    ) {
        match (heading, body.is_empty()) {
            (Some(h), true) => sections.push(h),
            (Some(h), false) => {
                let body_col = Column::with_children(body).spacing(8);
                let boxed = cosmic::widget::container(body_col)
                    .padding([14, 18])
                    .width(Length::Fill)
                    .style(|_theme| container::Style {
                        background: Some(Background::Color(TN_BG_LIGHT)),
                        text_color: Some(TN_FG),
                        border: border::rounded(8),
                        ..Default::default()
                    });
                sections.push(
                    Column::with_children(vec![h, boxed.into()])
                        .spacing(8)
                        .into(),
                );
            }
            (None, true) => {}
            (None, false) => {
                sections.push(Column::with_children(body).spacing(8).into());
            }
        }
    }

    for (idx, item) in items.iter().enumerate() {
        match item {
            markdown::Item::Heading(_, _) => {
                flush(
                    &mut sections,
                    pending_heading.take(),
                    std::mem::take(&mut pending_body),
                );
                pending_heading = Some(item.view(settings, style, idx));
            }
            _ => {
                pending_body.push(item.view(settings, style, idx));
            }
        }
    }
    flush(&mut sections, pending_heading.take(), pending_body);

    Column::with_children(sections).spacing(16).into()
}

// Mirror the option set used by cosmic::widget::markdown::parse so that the
// Select Text view interprets the same syntax the formatted view does.
// Source: libcosmic iced/widget/src/markdown.rs:586-589 (rev a37be90).
fn parser_options() -> pulldown_cmark::Options {
    use pulldown_cmark::Options;
    Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
}

fn markdown_to_plain_text(source: &str) -> String {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let parser = Parser::new_ext(source, parser_options());
    let mut output = String::new();
    let mut list_index: Option<u64> = None;
    // Front matter is parsed (so it isn't mistaken for a rule/heading) but
    // hidden, mirroring the formatted view which renders nothing for it.
    let mut in_metadata = false;
    // Table cells accumulate into `cell`; each completed row is joined with
    // " | " so a table copies as readable pipe-separated rows rather than a
    // run-on string.
    let mut row: Vec<String> = Vec::new();
    let mut cell: Option<String> = None;

    for event in parser {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => in_metadata = true,
            Event::End(TagEnd::MetadataBlock(_)) => in_metadata = false,
            _ if in_metadata => {}

            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::End(TagEnd::TableCell) => {
                row.push(cell.take().unwrap_or_default().trim().to_string());
            }
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                output.push_str(&row.join(" | "));
                output.push('\n');
                row.clear();
            }
            Event::End(TagEnd::Table) => output.push('\n'),

            Event::Text(text) | Event::Code(text) => match cell.as_mut() {
                Some(c) => c.push_str(&text),
                None => output.push_str(&text),
            },
            Event::SoftBreak => match cell.as_mut() {
                Some(c) => c.push(' '),
                None => output.push(' '),
            },
            Event::HardBreak => match cell.as_mut() {
                Some(c) => c.push(' '),
                None => output.push('\n'),
            },
            Event::Start(Tag::Heading { .. }) => {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                output.push('\n');
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                output.push_str("\n\n");
            }
            Event::Start(Tag::CodeBlock(_)) => {}
            Event::End(TagEnd::CodeBlock) => {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push('\n');
            }
            Event::Start(Tag::List(ordered)) => {
                list_index = ordered;
            }
            Event::End(TagEnd::List(_)) => {
                list_index = None;
                if !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            Event::Start(Tag::Item) => {
                if let Some(idx) = &mut list_index {
                    output.push_str(&format!("  {idx}. "));
                    *idx += 1;
                } else {
                    output.push_str("  • ");
                }
            }
            Event::End(TagEnd::Item) => {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

impl cosmic::Application for App {
    type Executor = executor::Default;
    type Flags = (String, Vec<markdown::Item>, String, PathBuf);
    type Message = Message;

    const APP_ID: &'static str = "com.galaxy.md-viewer";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Self::Flags) -> (Self, cosmic::app::Task<Self::Message>) {
        let (title, items, source, path) = flags;
        core.set_header_title(title);
        let plain_text = markdown_to_plain_text(&source);
        let editor_content = text_editor::Content::with_text(&plain_text);
        let app = App {
            core,
            items,
            editor_content,
            source,
            path,
            selectable_mode: false,
        };
        (app, cosmic::app::Task::none())
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let label = if self.selectable_mode {
            "Formatted"
        } else {
            "Select Text"
        };
        vec![
            cosmic::widget::button::text(label)
                .on_press(Message::ToggleSelectable)
                .into(),
        ]
    }

    fn style(&self) -> Option<Appearance> {
        Some(Appearance {
            background_color: TN_BG,
            text_color: TN_FG,
            icon_color: TN_FG,
        })
    }

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        match message {
            Message::LinkClicked(url) => {
                let _ = open::that_in_background(url.to_string());
            }
            Message::FileChanged => {
                if let Ok(source) = std::fs::read_to_string(&self.path) {
                    self.items = markdown::parse(&source).collect();
                    let plain_text = markdown_to_plain_text(&source);
                    self.editor_content = text_editor::Content::with_text(&plain_text);
                    self.source = source;
                }
            }
            Message::EditorAction(action) => {
                if !action.is_edit() {
                    self.editor_content.perform(action);
                }
            }
            Message::ToggleSelectable => {
                self.selectable_mode = !self.selectable_mode;
                if self.selectable_mode {
                    let plain_text = markdown_to_plain_text(&self.source);
                    self.editor_content = text_editor::Content::with_text(&plain_text);
                }
            }
        }
        cosmic::app::Task::none()
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        let target = self.path.clone();
        cosmic::iced::Subscription::run_with_id(
            "file-watcher",
            cosmic::iced_futures::stream::channel(1, move |mut sender| {
                let target = target.clone();
                async move {
                    use cosmic::iced_futures::futures::SinkExt;
                    use notify::Watcher;

                    // Watch the PARENT directory rather than the file itself.
                    // Editors that save via write-temp+rename (Vim safe-write,
                    // VS Code) replace the file's inode; a watch bound to the
                    // old inode goes deaf after the first save. Watching the
                    // directory and filtering by the target path survives the
                    // rename, so Create/Modify/Remove-then-Create at the path
                    // all keep reload working.
                    let dir = target
                        .parent()
                        .map(std::path::Path::to_path_buf)
                        .unwrap_or_else(|| std::path::PathBuf::from("."));

                    let (tx, mut rx) = cosmic::iced::futures::channel::mpsc::channel(1);
                    let filter_target = target.clone();
                    let watcher = notify::recommended_watcher(
                        move |event: Result<notify::Event, notify::Error>| {
                            if let Ok(event) = event {
                                let touches_target =
                                    event.paths.iter().any(|p| p == &filter_target);
                                let relevant_kind = matches!(
                                    event.kind,
                                    notify::EventKind::Modify(_)
                                        | notify::EventKind::Create(_)
                                        | notify::EventKind::Remove(_)
                                );
                                if touches_target && relevant_kind {
                                    let _ = tx.clone().try_send(());
                                }
                            }
                        },
                    );

                    // Degrade to a static viewer rather than panicking: inotify
                    // watch exhaustion (ENOSPC) is a real failure mode and the
                    // app is still fully usable without live reload.
                    let mut watcher = match watcher {
                        Ok(w) => w,
                        Err(e) => {
                            eprintln!("Live reload disabled: {e}");
                            return;
                        }
                    };

                    if let Err(e) = watcher.watch(&dir, notify::RecursiveMode::NonRecursive) {
                        eprintln!("Live reload disabled: {e}");
                        return;
                    }

                    loop {
                        use cosmic::iced::futures::StreamExt;
                        if rx.next().await.is_some() {
                            let _ = sender.send(Message::FileChanged).await;
                        }
                    }
                }
            }),
        )
    }

    fn view(&self) -> Element<'_, Self::Message> {
        if self.selectable_mode {
            let editor = cosmic::widget::TextEditor::new(&self.editor_content)
                .on_action(Message::EditorAction)
                .padding(24)
                .size(16)
                .class(theme::iced::TextEditor::Custom(Box::new(
                    |_theme, _status| ied_text_editor::Style {
                        background: Background::Color(TN_BG),
                        border: border::rounded(0),
                        icon: TN_FG,
                        placeholder: TN_COMMENT,
                        value: TN_FG,
                        selection: TN_SELECTION,
                    },
                )));

            cosmic::widget::container(editor)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(TN_BG)),
                    text_color: Some(TN_FG),
                    ..Default::default()
                })
                .into()
        } else {
            let style = markdown::Style {
                inline_code_highlight: Highlight {
                    background: Background::Color(TN_BG_DARK),
                    border: border::rounded(4),
                },
                inline_code_padding: padding::left(6).right(6),
                inline_code_color: TN_ORANGE,
                link_color: TN_BLUE,
            };

            let settings = markdown::Settings::with_text_size(16);
            let content = render_sections(&self.items, settings, style)
                .map(Message::LinkClicked);

            let body = cosmic::widget::container(content)
                .padding(24)
                .width(Length::Fill);

            cosmic::widget::container(scrollable(body))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(Background::Color(TN_BG)),
                    text_color: Some(TN_FG),
                    ..Default::default()
                })
                .into()
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            eprintln!("Usage: galaxy-md <file.md>");
            std::process::exit(1);
        });

    let path = std::fs::canonicalize(&path).unwrap_or_else(|e| {
        eprintln!("Failed to resolve {}: {e}", path.display());
        std::process::exit(1);
    });

    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Failed to read {}: {e}", path.display());
        std::process::exit(1);
    });

    let title = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Markdown Viewer".into());

    let items: Vec<markdown::Item> = markdown::parse(&source).collect();

    let settings = Settings::default()
        .size(cosmic::iced::Size::new(900.0, 700.0));

    cosmic::app::run::<App>(settings, (title, items, source, path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::markdown_to_plain_text;

    #[test]
    fn table_serializes_as_pipe_separated_rows() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";

        let out = markdown_to_plain_text(md);

        assert_eq!(out, "A | B\n1 | 2");
    }

    #[test]
    fn table_does_not_run_cells_together() {
        // Regression: with tables disabled a 2x2 table copied as the run-on
        // string "AB12". Cells must stay separated.
        let out = markdown_to_plain_text("| A | B |\n|---|---|\n| 1 | 2 |");

        assert!(!out.contains("AB12"), "cells ran together: {out:?}");
        assert!(out.contains(" | "), "missing cell separator: {out:?}");
    }

    #[test]
    fn strikethrough_keeps_inner_text_without_markers() {
        let out = markdown_to_plain_text("~~gone~~ and here");

        assert_eq!(out, "gone and here");
    }

    #[test]
    fn task_list_items_render_literal_checkboxes() {
        // The formatted view does not enable ENABLE_TASKLISTS, so checkboxes
        // stay literal text here too — the two views agree.
        let out = markdown_to_plain_text("- [ ] todo\n- [x] done");

        assert_eq!(out, "  • [ ] todo\n  • [x] done");
    }

    #[test]
    fn nested_lists_render_every_item() {
        let out = markdown_to_plain_text("- a\n  - b\n- c");

        assert!(out.contains("• a"), "{out:?}");
        assert!(out.contains("• b"), "{out:?}");
        assert!(out.contains("• c"), "{out:?}");
    }

    #[test]
    fn ordered_list_numbers_items() {
        let out = markdown_to_plain_text("1. first\n2. second");

        assert_eq!(out, "  1. first\n  2. second");
    }

    #[test]
    fn code_block_preserves_content() {
        let out = markdown_to_plain_text("```rust\nlet x = 1;\n```");

        assert_eq!(out, "let x = 1;");
    }

    #[test]
    fn yaml_front_matter_is_hidden() {
        // Mirrors the formatted view, which renders nothing for front matter.
        let out = markdown_to_plain_text("---\ntitle: Hidden\n---\n\nBody text");

        assert_eq!(out, "Body text");
    }
}

