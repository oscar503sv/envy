# Envy

**English** · [Español](README.es.md)

> *Your environments, beautifully organized.*

Envy is a local-first, GNOME-native editor for dotenv (`.env`) files. Open a file and change a variable in seconds, or keep the environment files of all your projects one click away, without opening an IDE and without Envy reformatting your file.

> **Status: pre-alpha, in planning.** There is no usable application yet. The product requirements, technical design and the v0.1.0 delivery plan are in [`docs/`](docs/). The first release, v0.1.0, is estimated for March 2027.

## Why Envy

Changing a single variable shouldn't require an IDE. General-purpose text editors don't give you a structured view of assignments, protect secrets on screen, organize files by project or warn you when another process (a script, a teammate's tool or an AI agent) changes the file you are editing.

Envy aims to do exactly that, and nothing more: it is **not** a remote secrets manager, an IDE or a deployment tool.

## Planned features (v0.1.0)

**Quick Edit**: `envy .env` from a terminal, or open any file from the file chooser.

- Structured view of assignments, with search by key.
- Edit unquoted, single-quoted, double-quoted and multiline values.
- Add and remove keys, paste blocks of `KEY=value` lines with a preview, copy a value or the whole document.
- Undo and redo.
- Secret-looking values (passwords, tokens, keys) hidden on screen until you reveal them.

**Workspace**: register local project folders and their service subfolders (`backend/`, `frontend/`…).

- See every `.env`, `.env.*` and `*.env` file of a project without browsing the file system.
- Environment labels (development, staging, production) shown as text, not only as color.
- Compare a file with its `.env.example` and add missing keys.
- Warnings for duplicated keys.

**File safety**, in both modes:

- **Lossless:** opening and saving without changes produces exactly the same bytes. Editing a value changes only that value, never comments, quoting, spacing, line endings or the BOM.
- **External changes:** if the file changes on disk and you have no local edits, Envy reloads it. If you do have local edits, Envy shows a conflict and never discards either version without your explicit decision.
- **Safe saving:** conditional replacement with post-write verification, file permissions preserved (e.g. `0600`), no silent overwrites and no backup copies of your secrets.
- **Symbolic links** are never replaced by regular files.

## Privacy

- Everything stays on your device: no accounts, no cloud, no sync, no telemetry.
- Variable values are never written to logs, error messages or Envy's own configuration. The project catalog stores only paths, names and labels.
- Envy never executes or expands anything: `$VAR`, `${VAR}` and `$(command)` are shown as literal text.
- Copying to the clipboard only happens when you ask for it. Keep in mind that the clipboard may retain secrets.

## Supported syntax

Envy supports a documented subset of dotenv:

- `KEY=value`, `KEY='value'`, `KEY="value"` and quoted values spanning several lines.
- An optional `export` prefix.
- Comment lines (`# …`) and trailing comments (` # …`, preceded by whitespace).
- Double quotes recognize `\\`, `\"`, `\n`, `\r` and `\t`. Single quotes are fully literal.

Anything outside the subset is shown read-only as "unsupported syntax" and preserved byte for byte. The full rules will live in `docs/SYNTAX.md`.

## Installation

Not available yet. v0.1.0 will be distributed as a Flatpak, with a submission to Flathub after the release. Native builds will also be supported.

## Building from source

Envy is in early development: the crate compiles, but there is no application window yet.

Stack:

- Rust 1.95 or newer, edition 2024 (`rust-toolchain.toml` pins the toolchain for `rustup`)
- GTK 4 and libadwaita 1.5 or newer (from the UI milestone on)
- Gio for file access, monitoring and saving
- Cargo with `build.rs` as the build system
- Flatpak with the GNOME 51 runtime

Build and check:

```sh
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Flatpak build instructions will be added together with the manifest.

## Documentation

- [Product requirements (PRD)](docs/PRD.md)
- [Technical design](docs/TECHNICAL_DESIGN.md)
- [v0.1.0 planning](docs/planning/): roadmap, sprint plan, backlog, milestones, risks, decisions and definition of done

The design and planning documents are written in Spanish.

## Contributing

Contributions are welcome once the foundation is in place. Read [CONTRIBUTING.md](CONTRIBUTING.md) first. **Never include real secrets** in issues, pull requests, screenshots or test fixtures.

## License

Envy is free software, released under the [GNU General Public License v3.0 or later](LICENSE) (`GPL-3.0-or-later`).
