# Contributing to Envy

**English** · [Español](CONTRIBUTING.es.md)

Thanks for your interest in Envy! The project is in an early stage: the requirements, technical design and v0.1.0 plan exist (see [`docs/`](docs/)), and the code base is being built sprint by sprint. Until the foundation is in place, the most useful contributions are feedback on the design documents and well-described bug reports or ideas.

## Ground rules

- **Never share real secrets.** Do not paste real `.env` contents, tokens, passwords or private keys in issues, pull requests, logs, screenshots or test fixtures. Use obviously fake values such as `EXAMPLE_TOKEN_123`.
- Be respectful. Envy follows the [GNOME Code of Conduct](https://conduct.gnome.org/).
- Keep the scope in mind. Envy is a local editor for dotenv files. Features such as cloud sync, remote secret providers, running commands or variable expansion are explicit non-goals (see PRD §2.2).

## Reporting bugs

Open an issue with:

- What you did, what you expected and what happened instead.
- Your distribution, GNOME version and how you installed Envy (Flatpak or native).
- A **synthetic** minimal `.env` that reproduces the problem, if the bug involves parsing or saving. Describe line endings, BOM, quotes and permissions when they matter.

If Envy lost or corrupted data, say so in the title. These bugs have the highest priority.

## Reporting security issues

Do **not** open a public issue for vulnerabilities, for example secret leaks in logs, unsafe file handling or symlink attacks. Report them privately through [GitHub security advisories](https://github.com/oscar503sv/envy/security/advisories/new).

## Proposing changes

1. For anything beyond a small fix, open an issue first so the approach can be agreed before you write code.
2. Fork the repository and create a branch from `main` (for example `feat/paste-preview` or `fix/crlf-insert`).
3. Keep each pull request focused on one change and reference the issue or planning task it addresses (for example `ENV-T016`).
4. Make sure every check below passes before requesting a review.

## Development guidelines

### Code

- Code, identifiers and comments are written in English.
- Format with `cargo fmt` and keep `cargo clippy --all-targets -- -D warnings` clean.
- Write comments only to explain *why*, not *what*.
- Follow the architecture in [`docs/TECHNICAL_DESIGN.md`](docs/TECHNICAL_DESIGN.md). The domain core (parser, document model, edit commands) must not depend on GTK.
- Do not add a dependency without discussing it first. Check whether something equivalent already exists in the project or in the standard library.

### File safety and privacy

These rules are non-negotiable, and reviews check them explicitly:

- Unmodified bytes must survive a round trip exactly. Never normalize line endings, trim whitespace or rebuild the file from a map of keys.
- Never write variable values to logs, error messages, `Debug` output, panics or configuration files.
- Never execute or expand file contents.
- Never report a save as successful unless it has been verified, and never overwrite an externally modified file silently.

### Tests

- Every bug fix comes with a test that reproduces it.
- Parser and editing changes need fixture-based tests, including CRLF, mixed line endings, BOM, missing trailing newline, multiline values and unsupported syntax. Property-based tests are welcome.
- File-system behavior is tested against real temporary files, not mocks.
- Run the whole suite with `cargo test` before opening a pull request. GTK tests that need a display must be reported as skipped when no display is available, never as passed.

### Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(parser): support export prefix
fix(storage): keep file mode after save
docs: describe CLI usage under Flatpak
```

Common types are `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci` and `chore`. Write the subject in the imperative mood and in English.

### User interface

- Follow the [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/) and use libadwaita widgets.
- Every action must be usable with the keyboard alone, with visible focus.
- Never convey information through color alone, and give icon-only buttons accessible names.
- Mark user-visible strings for translation.

## License

By contributing, you agree that your contributions are licensed under the [GNU General Public License v3.0 or later](LICENSE), the same license as the project.
