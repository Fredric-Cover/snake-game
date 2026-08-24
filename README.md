# Terminal Snake (Rust)

A feature-packed, terminal Snake game built in Rust using `crossterm`. It features full emoji support (`✴️`), custom terminal raw-mode handling, high-score persistence via `bincode`, and high-performance rendering.

---

## Features

- **Emoji & Character-Width Support:** Correctly handles wide-character rendering (like `✴️` for food or custom snake avatars) using proper display-width calculations.
- **High-Score Persistence:** Automatically saves and loads your top scores locally using `bincode` serialization.
- **Graceful Terminal Recovery:** Custom panic hook restores raw terminal mode, unhides the cursor, and cleans up the buffer on unexpected crashes or exits.

---

## Prerequisites

- **Rust:** Install via [rustup.rs](https://rustup.rs/) (Rust 2024 edition or later required, although if you edit Cargo.toml you can change that).
- **Terminal:** A modern terminal emulator with UTF-8 / Emoji support (e.g., Alacritty, iTerm2, Kitty, Ghostty).

---

## Installation & Running

1. **Clone the repository:**
   ```
   git clone https://github.com/Fredric-Cover/snake-game.git
   cd snake-game
   ```

3. **Run in release mode:**
   ```
   cargo run --release
   ```

---

## Controls

| Key | Action |
| :--- | :--- |
| `W` / `Up Arrow` | Move Up |
| `S` / `Down Arrow` | Move Down |
| `A` / `Left Arrow` | Move Left |
| `D` / `Right Arrow` | Move Right |
| `P` | Pause |
| `R` | Resume |
| `Q` | Quit Game |

---

## License

This project is open-source and available under the [MIT License](LICENSE).
