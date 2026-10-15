# Sticky Notes (`cosmic-applet-stickynotes`)

A virtual sticky notes panel applet for the [COSMIC™ desktop environment](https://system76.com/cosmic) (Pop!_OS).

---

## 📌 Overview & Usage

- **Panel Applet**: Sits neatly in your COSMIC panel or dock with a dedicated sticky note icon.
- **Quick Access**: Click to open a clean, 100% opaque popup note.
- **Multiple Notes**: Create, name, switch between, and delete notes via the top dropdown bar.
- **Color Customization**: Choose from 6 pale pastel background colors (Butter Yellow, Mint, Powder Blue, Lavender, Rose, Peach) with high-contrast text.
- **Auto-Save & Persistence**: Notes automatically save as you type (debounced) and persist across reboots in `~/.config/cosmic-applet-stickynotes/notes.json`.

---

## 🛠️ Installation

### Prerequisites

- [Rust toolchain](https://rustup.rs/) (edition 2024 / rustc 1.85+)
- [`just`](https://github.com/casey/just) command runner

### Option 1: Install for current user (Recommended, no `sudo` needed)

```bash
PREFIX=$HOME/.local just install
```

### Option 2: Install system-wide

```bash
cargo build --release
sudo ~/.cargo/bin/just install
```

### Adding to your panel

After installing, open **Settings → Desktop → Panel → Applets → Add Applet** and add **Sticky Notes**.

To update after rebuilding:
```bash
killall cosmic-applet-stickynotes
```

---

## 💻 Development

### Run in debug mode

```bash
just run
# or: cargo run
```

### Run tests

```bash
cargo test
```

### Linting

```bash
cargo clippy -- -D warnings
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
