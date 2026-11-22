//! Checks whether the dotenv parser can serialize files byte for byte.
//!
//! This internal example is not installed by default (`cargo install` skips
//! examples unless `--example` is passed, and the crate has `publish = false`);
//! the Flatpak manifest must install only the `envy` binary. Reports include
//! basenames and diagnostic metadata only. They never include keys, values, raw lines, or other file contents.

use std::{
    env, fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

use envy::domain::{
    error::{DomainError, NodeKindTag},
    parser::parse,
};

fn main() -> ExitCode {
    let stdout = io::stdout();
    let mut output = stdout.lock();

    match run(&mut output).and_then(|status| {
        output.flush()?;
        Ok(status)
    }) {
        Ok(status) => ExitCode::from(status),
        Err(_) => ExitCode::from(2),
    }
}

fn run(output: &mut impl Write) -> io::Result<u8> {
    let mut paths = env::args_os().skip(1).peekable();
    if paths.peek().is_none() {
        writeln!(
            output,
            "Usage: cargo run --example roundtrip -- <file> [<file>...]"
        )?;
        return Ok(2);
    }

    let mut status = 0;
    for (index, path) in paths.enumerate() {
        if index > 0 {
            writeln!(output)?;
        }
        status = status.max(report_file(Path::new(&path), output)?);
    }
    Ok(status)
}

fn report_file(path: &Path, output: &mut impl Write) -> io::Result<u8> {
    let name = path
        .file_name()
        .map_or_else(|| "<no basename>".into(), |name| name.to_string_lossy());
    writeln!(output, "{}", name.escape_debug())?;

    // Refuse FIFOs and devices, which would block or grow without bound.
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            writeln!(output, "Not a regular file")?;
            return Ok(2);
        }
        Err(error) => {
            writeln!(output, "I/O error reading file ({:?})", error.kind())?;
            return Ok(2);
        }
    }

    let input = match fs::read(path) {
        Ok(input) => input,
        Err(error) => {
            writeln!(output, "I/O error reading file ({:?})", error.kind())?;
            return Ok(2);
        }
    };
    writeln!(output, "Bytes: {}", input.len())?;

    let document = match parse(input) {
        Ok(document) => document,
        Err(error) => {
            match error {
                DomainError::InvalidUtf8 { byte_offset } => {
                    writeln!(output, "Parse error: InvalidUtf8 at byte {byte_offset}")?;
                }
                DomainError::NotEditable { .. } => {
                    writeln!(output, "Parse error: NotEditable")?;
                }
                DomainError::NodeNotFound { .. } => {
                    writeln!(output, "Parse error: NodeNotFound")?;
                }
                DomainError::UnclosedQuote { line } => {
                    writeln!(output, "Parse error: UnclosedQuote at line {line}")?;
                }
                _ => writeln!(output, "Parse error")?,
            }
            return Ok(1);
        }
    };

    let bom = if document.has_bom() { "yes" } else { "no" };
    writeln!(output, "BOM: {bom}")?;

    let (mut assignment, mut comment, mut blank, mut raw) = (0, 0, 0, 0);
    for node in document.nodes() {
        match node.kind().tag() {
            NodeKindTag::Assignment => assignment += 1,
            NodeKindTag::Comment => comment += 1,
            NodeKindTag::Blank => blank += 1,
            NodeKindTag::Raw => raw += 1,
        }
    }
    writeln!(
        output,
        "Nodes: assignment={assignment}, comment={comment}, blank={blank}, raw={raw}"
    )?;
    if let Some(line) = document.unclosed_quote_line() {
        writeln!(output, "Unclosed quote at line {line}")?;
    }

    // parse() keeps the original bytes, so no copy of the input is needed.
    let original = document.source();
    let serialized = document.serialize();
    if serialized == original {
        writeln!(output, "OK")?;
        return Ok(0);
    }

    let offset = original
        .iter()
        .zip(&serialized)
        .position(|(expected, actual)| expected != actual)
        .unwrap_or_else(|| original.len().min(serialized.len()));
    writeln!(
        output,
        "MISMATCH at byte offset {offset} (expected len {}, actual len {})",
        original.len(),
        serialized.len()
    )?;
    Ok(1)
}
