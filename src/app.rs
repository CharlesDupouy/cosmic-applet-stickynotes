// SPDX-License-Identifier: MIT

use crate::config::{NoteColor, NotesStore};
use crate::fl;
use cosmic::applet::cosmic_panel_config::PanelAnchor;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::widget::{container, text_editor as iced_text_editor};
use cosmic::iced::{window::Id, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::text_editor;

/// Unique identifier for this applet (reverse domain name notation).
const APP_ID: &str = "com.github.CharlesDupouy.CosmicAppletStickynotes";

/// Embedded custom sticky note symbolic icon SVG.
const ICON_BYTES: &[u8] = include_bytes!(
    "../res/icons/hicolor/scalable/apps/com.github.CharlesDupouy.CosmicAppletStickynotes.svg"
);

/// SVG for window resize grip handle in the bottom-right corner.
const RESIZE_GRIP_SVG: &[u8] = br##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg" fill="none" stroke="#888888" stroke-width="1.5" stroke-linecap="round">
  <line x1="14" y1="4" x2="4" y2="14" />
  <line x1="14" y1="9" x2="9" y2="14" />
  <line x1="14" y1="14" x2="14" y2="14" />
</svg>"##;

/// Application state.
pub struct AppModel {
    /// Runtime core managed by libcosmic.
    core: cosmic::Core,
    /// The current popup window ID, if open.
    popup: Option<Id>,
    /// All notes and persistence.
    store: NotesStore,
    /// Text content managed by the TextEditor widget (mirrors active note content).
    editor_content: text_editor::Content,
    /// Buffer for the "new note" name input field.
    new_note_name: String,
    /// Whether the "new note" name input is visible.
    show_new_note_input: bool,
    /// Whether the delete-confirmation dialog is shown.
    show_delete_confirm: bool,
    /// Whether user is currently dragging the corner resize handle.
    is_resizing: bool,
    /// Last recorded cursor position during resize drag.
    resize_last_pos: Option<cosmic::iced::Point>,
}

impl Default for AppModel {
    fn default() -> Self {
        let store = NotesStore::load();
        let editor_content = text_editor::Content::with_text(
            store.active().map(|n| n.content.as_str()).unwrap_or(""),
        );
        Self {
            core: cosmic::Core::default(),
            popup: None,
            store,
            editor_content,
            new_note_name: String::new(),
            show_new_note_input: false,
            show_delete_confirm: false,
            is_resizing: false,
            resize_last_pos: None,
        }
    }
}

/// All messages emitted by the applet and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    /// Toggle the popup open or closed.
    TogglePopup,
    /// Fired by the runtime when the popup window closes externally.
    PopupClosed(Id),
    /// User selected a different note in the dropdown.
    SelectNote(usize),
    /// Change the background color of the active sticky note.
    ChangeNoteColor(NoteColor),
    /// Toggle visibility of the "new note" name input.
    ToggleNewNoteInput,
    /// User is typing in the new-note name field.
    NewNoteNameChanged(String),
    /// User confirmed creating a new note (submitted the text input).
    ConfirmNewNote,
    /// User clicked the Delete button.
    RequestDeleteNote,
    /// User confirmed deletion in the dialog.
    ConfirmDeleteNote,
    /// User cancelled deletion.
    CancelDeleteNote,
    /// User performed an action in the text editor (typing, cursor movement…).
    EditorAction(text_editor::Action),
    /// Debounced save: fired ~500ms after the last edit.
    SaveNotes,
    /// User started dragging the bottom-right resize handle.
    StartResize,
    /// Mouse moved while dragging the resize handle.
    ResizeMove(cosmic::iced::Point),
    /// User released the mouse button, finishing the resize drag.
    ResizeEnd,
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let store = NotesStore::load();
        let editor_content = text_editor::Content::with_text(
            store.active().map(|n| n.content.as_str()).unwrap_or(""),
        );
        let app = Self {
            core,
            popup: None,
            store,
            editor_content,
            new_note_name: String::new(),
            show_new_note_input: false,
            show_delete_confirm: false,
            is_resizing: false,
            resize_last_pos: None,
        };
        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            // ── Popup ──────────────────────────────────────────────────────────
            Message::TogglePopup => {
                self.is_resizing = false;
                self.resize_last_pos = None;
                return if let Some(popup_id) = self.popup.take() {
                    destroy_popup(popup_id)
                } else {
                    let new_id = Id::unique();
                    self.popup = Some(new_id);
                    // Reset transient UI state each time popup opens.
                    self.show_new_note_input = false;
                    self.show_delete_confirm = false;
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        Some((
                            self.store.popup_width.round() as u32,
                            self.store.popup_height.round() as u32,
                        )),
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = cosmic::iced::Limits::NONE
                        .min_width(260.0)
                        .max_width(1200.0)
                        .min_height(200.0)
                        .max_height(1200.0);
                    get_popup(popup_settings)
                };
            }

            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                    self.is_resizing = false;
                    self.resize_last_pos = None;
                }
            }

            // ── Note switching ─────────────────────────────────────────────────
            Message::SelectNote(index) => {
                if index < self.store.notes.len() {
                    self.store.active_index = index;
                    self.reload_editor();
                    self.show_new_note_input = false;
                    self.show_delete_confirm = false;
                }
            }

            // ── Color switching ────────────────────────────────────────────────
            Message::ChangeNoteColor(color) => {
                if let Some(note) = self.store.active_mut() {
                    note.color = color;
                    self.store.save();
                }
            }

            // ── New note flow ──────────────────────────────────────────────────
            Message::ToggleNewNoteInput => {
                self.show_new_note_input = !self.show_new_note_input;
                self.show_delete_confirm = false;
                if self.show_new_note_input {
                    self.new_note_name.clear();
                }
            }

            Message::NewNoteNameChanged(name) => {
                self.new_note_name = name;
            }

            Message::ConfirmNewNote => {
                let raw_name = self.new_note_name.trim().to_owned();
                let name = if raw_name.is_empty() {
                    format!("Note {}", self.store.next_id)
                } else {
                    raw_name
                };
                let id = self.store.next_id;
                self.store.next_id += 1;
                let note = crate::config::StickyNote::new(id, name);
                self.store.notes.push(note);
                self.store.active_index = self.store.notes.len() - 1;
                self.reload_editor();
                self.new_note_name.clear();
                self.show_new_note_input = false;
                self.store.save();
            }

            // ── Delete flow ────────────────────────────────────────────────────
            Message::RequestDeleteNote => {
                // Guard: never delete the last remaining note.
                if self.store.notes.len() > 1 {
                    self.show_delete_confirm = true;
                }
            }

            Message::ConfirmDeleteNote => {
                if self.store.notes.len() > 1 {
                    self.store.notes.remove(self.store.active_index);
                    // Keep active_index in bounds.
                    if self.store.active_index >= self.store.notes.len() {
                        self.store.active_index = self.store.notes.len() - 1;
                    }
                    self.reload_editor();
                    self.store.save();
                }
                self.show_delete_confirm = false;
            }

            Message::CancelDeleteNote => {
                self.show_delete_confirm = false;
            }

            // ── Editing & auto-save ───────────────────────────────────────────
            Message::EditorAction(action) => {
                // Let the editor widget process the action.
                let is_edit = action.is_edit();
                self.editor_content.perform(action);

                if is_edit {
                    // Sync content back to the store.
                    if let Some(note) = self.store.active_mut() {
                        note.content = self.editor_content.text();
                    }
                    // Schedule a debounced save (fire-and-forget 500ms task).
                    return Task::perform(
                        async {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        },
                        |_| cosmic::Action::App(Message::SaveNotes),
                    );
                }
            }

            Message::SaveNotes => {
                self.store.save();
            }

            // ── Resizing ──────────────────────────────────────────────────────
            Message::StartResize => {
                self.is_resizing = true;
                self.resize_last_pos = None;
            }

            Message::ResizeMove(position) => {
                if self.is_resizing {
                    if let Some(last) = self.resize_last_pos {
                        let dx = position.x - last.x;
                        let dy = position.y - last.y;
                        self.store.popup_width =
                            (self.store.popup_width + dx).clamp(260.0, 1200.0);
                        self.store.popup_height =
                            (self.store.popup_height + dy).clamp(200.0, 1000.0);
                    }
                    self.resize_last_pos = Some(position);
                }
            }

            Message::ResizeEnd => {
                if self.is_resizing {
                    self.is_resizing = false;
                    self.resize_last_pos = None;
                    self.store.save();
                }
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // High-definition custom sticky note symbolic icon for the panel bar.
        let mut handle = cosmic::widget::icon::from_svg_bytes(ICON_BYTES);
        handle.symbolic = true;

        self.core
            .applet
            .icon_button_from_handle(handle)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        // Build the list of note names for the dropdown.
        let note_names: Vec<String> = self.store.notes.iter().map(|n| n.name.clone()).collect();
        let active_index = self.store.active_index;

        let current_color = self
            .store
            .active()
            .map(|n| n.color)
            .unwrap_or_default();

        let (bg_color, border_color, text_color, placeholder_color) = current_color.palette();

        // ── Dropdown ──────────────────────────────────────────────────────────
        let dropdown = widget::dropdown(
            note_names,
            Some(active_index),
            Message::SelectNote,
        );

        // ── New / Delete buttons ───────────────────────────────────────────────
        let new_btn = widget::button::standard(fl!("new-note"))
            .on_press(Message::ToggleNewNoteInput);

        let can_delete = self.store.notes.len() > 1;
        let delete_btn = if can_delete {
            widget::button::destructive(fl!("delete")).on_press(Message::RequestDeleteNote)
        } else {
            // Disabled: no on_press handler.
            widget::button::destructive(fl!("delete"))
        };

        // ── Controls row ──────────────────────────────────────────────────────
        let controls_row = widget::row::with_capacity(5)
            .push(dropdown)
            .push(widget::Space::new().width(cosmic::iced::Length::Fill))
            .push(new_btn)
            .push(widget::Space::new().width(8))
            .push(delete_btn)
            .align_y(cosmic::iced::Alignment::Center)
            .spacing(4);

        // ── Color Swatches ────────────────────────────────────────────────────
        let mut color_swatches = widget::row::with_capacity(NoteColor::ALL.len())
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center);

        for &color in &NoteColor::ALL {
            let (swatch_bg, swatch_border, _, _) = color.palette();
            let is_selected = current_color == color;

            let swatch_dot = widget::button::custom(
                widget::container(widget::Space::new().width(18).height(18))
                    .style(move |_theme| {
                        container::Style {
                            background: Some(swatch_bg.into()),
                            border: cosmic::iced::Border {
                                radius: 9.0.into(),
                                width: if is_selected { 2.5 } else { 1.0 },
                                color: if is_selected {
                                    cosmic::iced::Color::from_rgb(0.2, 0.2, 0.2)
                                } else {
                                    swatch_border
                                },
                            },
                            shadow: if is_selected {
                                cosmic::iced::Shadow {
                                    color: cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                                    offset: cosmic::iced::Vector::new(0.0, 1.0),
                                    blur_radius: 3.0,
                                }
                            } else {
                                cosmic::iced::Shadow::default()
                            },
                            ..Default::default()
                        }
                    })
            )
            .padding(1)
            .class(cosmic::theme::Button::Text)
            .on_press(Message::ChangeNoteColor(color));

            color_swatches = color_swatches.push(swatch_dot);
        }

        let header = widget::column::with_capacity(2)
            .push(controls_row)
            .push(color_swatches)
            .spacing(8);

        let header_padded = cosmic::applet::padded_control(header);

        // ── New-note input row (conditionally shown) ──────────────────────────
        let new_note_section: Option<Element<Self::Message>> = if self.show_new_note_input {
            let name_input = widget::text_input(fl!("new-note-name"), &self.new_note_name)
                .on_input(Message::NewNoteNameChanged)
                // on_submit requires Fn(String) -> Message; wrap in a closure.
                .on_submit(|_| Message::ConfirmNewNote);

            let create_btn =
                widget::button::suggested(fl!("create")).on_press(Message::ConfirmNewNote);

            let cancel_btn =
                widget::button::standard(fl!("cancel")).on_press(Message::ToggleNewNoteInput);

            let row = widget::row::with_capacity(5)
                .push(name_input)
                .push(widget::Space::new().width(4))
                .push(create_btn)
                .push(widget::Space::new().width(4))
                .push(cancel_btn)
                .align_y(cosmic::iced::Alignment::Center)
                .spacing(4);

            Some(cosmic::applet::padded_control(row).into())
        } else {
            None
        };

        // Compute estimated content width based on the longest line in the note.
        let max_line_len = self
            .editor_content
            .lines()
            .map(|l| l.text.len())
            .max()
            .unwrap_or(0);
        // Note width available inside the card (popup_width minus outer padding and card padding).
        let note_inner_width = (self.store.popup_width - 48.0).max(200.0);
        let editor_width = (max_line_len as f32 * 9.5 + 32.0).max(note_inner_width);

        // ── Text editor inside styled virtual sticky note ────────────────────
        let editor = widget::TextEditor::new(&self.editor_content)
            .on_action(Message::EditorAction)
            .width(editor_width)
            .height(cosmic::iced::Length::Fill)
            .wrapping(cosmic::iced::core::text::Wrapping::None)
            .padding(cosmic::iced::Padding::new(4.0))
            .style(move |_theme, _status| {
                iced_text_editor::Style {
                    background: cosmic::iced::Background::Color(cosmic::iced::Color::TRANSPARENT),
                    border: cosmic::iced::Border::default(),
                    placeholder: placeholder_color,
                    value: text_color,
                    selection: cosmic::iced::Color::from_rgba(0.2, 0.4, 0.8, 0.3),
                }
            });

        let scrollable_editor = widget::scrollable::horizontal(editor);

        // The virtual sticky note box: thinner border, rounded corners, pastel background
        let note_card = widget::container(scrollable_editor)
            .padding(10)
            .height(cosmic::iced::Length::Fill)
            .width(cosmic::iced::Length::Fill)
            .style(move |_theme| {
                container::Style {
                    background: Some(bg_color.into()),
                    border: cosmic::iced::Border {
                        radius: 8.0.into(),
                        width: 1.0, // Thinner, elegant border
                        color: border_color,
                    },
                    shadow: cosmic::iced::Shadow {
                        color: cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.07),
                        offset: cosmic::iced::Vector::new(0.0, 2.0),
                        blur_radius: 8.0,
                    },
                    text_color: Some(text_color),
                    icon_color: Some(text_color),
                    snap: true,
                }
            });

        let editor_padded = cosmic::applet::padded_control(note_card);

        // ── Resize handle grip in bottom-right corner ─────────────────────────
        let grip_svg = cosmic::widget::svg(cosmic::iced::widget::svg::Handle::from_memory(
            RESIZE_GRIP_SVG,
        ))
        .width(14)
        .height(14);

        let resize_grip = widget::mouse_area(
            widget::container(grip_svg).padding(cosmic::iced::Padding {
                top: 0.0,
                bottom: 2.0,
                left: 4.0,
                right: 4.0,
            }),
        )
        .interaction(cosmic::iced::mouse::Interaction::ResizingDiagonallyDown)
        .on_press(Message::StartResize);

        let footer_row = widget::row::with_capacity(2)
            .push(widget::Space::new().width(cosmic::iced::Length::Fill))
            .push(resize_grip)
            .align_y(cosmic::iced::Alignment::Center);

        let footer_padded = cosmic::applet::padded_control(footer_row);

        // ── Assemble content column ───────────────────────────────────────────
        let mut content_col = widget::column::with_capacity(5)
            .push(header_padded)
            .push(widget::divider::horizontal::default());

        if let Some(new_section) = new_note_section {
            content_col = content_col
                .push(new_section)
                .push(widget::divider::horizontal::default());
        }

        content_col = content_col
            .push(editor_padded)
            .push(footer_padded)
            .height(cosmic::iced::Length::Fill)
            .width(cosmic::iced::Length::Fill);

        // ── Delete confirmation dialog overlay ────────────────────────────────
        let active_name = self
            .store
            .active()
            .map(|n| n.name.as_str())
            .unwrap_or("this note");

        if self.show_delete_confirm {
            let confirm_btn = widget::button::destructive(fl!("delete-confirm-yes"))
                .on_press(Message::ConfirmDeleteNote);
            let cancel_btn =
                widget::button::standard(fl!("cancel")).on_press(Message::CancelDeleteNote);

            widget::dialog()
                .title(fl!("delete-confirm-title"))
                .body(fl!("delete-confirm-body", name = active_name))
                .primary_action(confirm_btn)
                .secondary_action(cancel_btn)
                .into()
        } else {
            self.opaque_popup_container(content_col).into()
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if self.is_resizing {
            cosmic::iced::event::listen_with(|event, _status, _window_id| match event {
                cosmic::iced::Event::Mouse(
                    cosmic::iced::mouse::Event::CursorMoved { position },
                ) => Some(Message::ResizeMove(position)),
                cosmic::iced::Event::Mouse(
                    cosmic::iced::mouse::Event::ButtonReleased(
                        cosmic::iced::mouse::Button::Left,
                    ),
                ) => Some(Message::ResizeEnd),
                _ => None,
            })
        } else {
            Subscription::none()
        }
    }
}

impl AppModel {
    /// Reloads the `TextEditor` content from the currently active note.
    fn reload_editor(&mut self) {
        let text = self
            .store
            .active()
            .map(|n| n.content.as_str())
            .unwrap_or("");
        self.editor_content = text_editor::Content::with_text(text);
    }

    /// Opaque popup container: guarantees 100% solid, non-transparent background.
    fn opaque_popup_container<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
    ) -> cosmic::widget::autosize::Autosize<'a, Message, cosmic::Theme, cosmic::Renderer> {
        let (vertical_align, horizontal_align) = match self.core.applet.anchor {
            PanelAnchor::Left => (Vertical::Center, Horizontal::Left),
            PanelAnchor::Right => (Vertical::Center, Horizontal::Right),
            PanelAnchor::Top => (Vertical::Top, Horizontal::Center),
            PanelAnchor::Bottom => (Vertical::Bottom, Horizontal::Center),
        };

        cosmic::widget::autosize::autosize(
            widget::container(
                widget::container(content)
                    .width(self.store.popup_width)
                    .height(self.store.popup_height)
                    .style(|theme| {
                        let cosmic = theme.cosmic();
                        let corners = cosmic.corner_radii;
                        // Force transparent: false so the popup background is 100% opaque
                        let bg = cosmic.background(false).base;
                        let mut bg_color: cosmic::iced::Color = bg.into();
                        bg_color.a = 1.0;
                        container::Style {
                            text_color: Some(cosmic.background(false).on.into()),
                            background: Some(bg_color.into()),
                            border: cosmic::iced::Border {
                                radius: corners.radius_m.into(),
                                width: 1.0,
                                color: cosmic.background(false).divider.into(),
                            },
                            shadow: cosmic::iced::Shadow {
                                color: cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                                offset: cosmic::iced::Vector::new(0.0, 4.0),
                                blur_radius: 16.0,
                            },
                            icon_color: Some(cosmic.background(false).on.into()),
                            snap: true,
                        }
                    }),
            )
            .height(cosmic::iced::Length::Shrink)
            .width(cosmic::iced::Length::Shrink)
            .align_x(horizontal_align)
            .align_y(vertical_align),
            cosmic::widget::Id::new("cosmic-applet-autosize"),
        )
        .limits(
            cosmic::iced::Limits::NONE
                .min_height(200.0)
                .min_width(260.0)
                .max_width(1200.0)
                .max_height(1200.0),
        )
    }
}
