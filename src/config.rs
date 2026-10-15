// SPDX-License-Identifier: MIT

//! Data model and persistence for sticky notes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Predefined pale, aesthetic colors for virtual sticky notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NoteColor {
    #[default]
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Orange,
}

impl NoteColor {
    pub const ALL: [NoteColor; 6] = [
        NoteColor::Yellow,
        NoteColor::Green,
        NoteColor::Blue,
        NoteColor::Purple,
        NoteColor::Pink,
        NoteColor::Orange,
    ];

    /// Returns (background, border, text, placeholder) colors as iced::Color.
    pub fn palette(self) -> (cosmic::iced::Color, cosmic::iced::Color, cosmic::iced::Color, cosmic::iced::Color) {
        let text = cosmic::iced::Color::from_rgb(0.14, 0.13, 0.12);
        let placeholder = cosmic::iced::Color::from_rgba(0.14, 0.13, 0.12, 0.5);
        match self {
            NoteColor::Yellow => (
                cosmic::iced::Color::from_rgb(0.996, 0.953, 0.663), // #FEF3A9
                cosmic::iced::Color::from_rgb(0.91, 0.85, 0.52),
                text,
                placeholder,
            ),
            NoteColor::Green => (
                cosmic::iced::Color::from_rgb(0.831, 0.957, 0.867), // #D4F4DD
                cosmic::iced::Color::from_rgb(0.69, 0.86, 0.74),
                text,
                placeholder,
            ),
            NoteColor::Blue => (
                cosmic::iced::Color::from_rgb(0.827, 0.898, 1.0),   // #D3E5FF
                cosmic::iced::Color::from_rgb(0.68, 0.79, 0.93),
                text,
                placeholder,
            ),
            NoteColor::Purple => (
                cosmic::iced::Color::from_rgb(0.910, 0.871, 1.0),   // #E8DEFF
                cosmic::iced::Color::from_rgb(0.78, 0.72, 0.92),
                text,
                placeholder,
            ),
            NoteColor::Pink => (
                cosmic::iced::Color::from_rgb(0.992, 0.863, 0.902), // #FDDCE6
                cosmic::iced::Color::from_rgb(0.90, 0.72, 0.78),
                text,
                placeholder,
            ),
            NoteColor::Orange => (
                cosmic::iced::Color::from_rgb(1.0, 0.890, 0.808),   // #FFE3CE
                cosmic::iced::Color::from_rgb(0.91, 0.76, 0.65),
                text,
                placeholder,
            ),
        }
    }
}

/// A single sticky note.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StickyNote {
    /// Unique auto-incrementing ID used as a stable key.
    pub id: u64,
    /// User-visible name of the note.
    pub name: String,
    /// Full text content of the note.
    pub content: String,
    /// Color theme of this note.
    #[serde(default)]
    pub color: NoteColor,
}

impl StickyNote {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            content: String::new(),
            color: NoteColor::Yellow,
        }
    }
}

/// Persisted application state (serialised to/from JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesStore {
    /// All sticky notes.
    pub notes: Vec<StickyNote>,
    /// Index of the currently active note.
    pub active_index: usize,
    /// Monotonically increasing counter for assigning stable IDs.
    pub next_id: u64,
}

impl Default for NotesStore {
    fn default() -> Self {
        let first = StickyNote::new(1, "Note 1");
        Self {
            notes: vec![first],
            active_index: 0,
            next_id: 2,
        }
    }
}

impl NotesStore {
    /// Returns the path to the JSON storage file:
    /// `~/.config/cosmic-applet-stickynotes/notes.json`
    pub fn storage_path() -> PathBuf {
        let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config"));
        p.push("cosmic-applet-stickynotes");
        p.push("notes.json");
        p
    }

    /// Loads from disk, falling back to the default if the file is
    /// missing or the JSON is malformed.
    pub fn load() -> Self {
        let path = Self::storage_path();
        let Ok(data) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        let Ok(mut store) = serde_json::from_str::<Self>(&data) else {
            return Self::default();
        };
        if store.notes.is_empty() {
            return Self::default();
        }
        store.active_index = store.active_index.min(store.notes.len() - 1);
        store
    }

    /// Saves to disk, creating parent directories as needed.
    /// Failures are silently ignored (best-effort).
    pub fn save(&self) {
        let path = Self::storage_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Returns a reference to the currently active note, if any.
    pub fn active(&self) -> Option<&StickyNote> {
        self.notes.get(self.active_index)
    }

    /// Returns a mutable reference to the currently active note, if any.
    pub fn active_mut(&mut self) -> Option<&mut StickyNote> {
        self.notes.get_mut(self.active_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_store() {
        let store = NotesStore::default();
        assert_eq!(store.notes.len(), 1);
        assert_eq!(store.active_index, 0);
        assert_eq!(store.next_id, 2);
        assert_eq!(store.active().unwrap().name, "Note 1");
        assert_eq!(store.active().unwrap().content, "");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut store = NotesStore::default();
        store.notes.push(StickyNote {
            id: 2,
            name: "Todo List".into(),
            content: "- Buy milk\n- Code Rust".into(),
            color: NoteColor::Green,
        });
        store.active_index = 1;

        let json = serde_json::to_string(&store).expect("Serialization failed");
        let loaded: NotesStore = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(loaded.notes.len(), 2);
        assert_eq!(loaded.active_index, 1);
        assert_eq!(loaded.active().unwrap().name, "Todo List");
        assert_eq!(loaded.active().unwrap().content, "- Buy milk\n- Code Rust");
        assert_eq!(loaded.active().unwrap().color, NoteColor::Green);
    }

    #[test]
    fn test_active_mut_editing() {
        let mut store = NotesStore::default();
        if let Some(note) = store.active_mut() {
            note.content = "Updated content".to_string();
        }
        assert_eq!(store.active().unwrap().content, "Updated content");
    }
}
