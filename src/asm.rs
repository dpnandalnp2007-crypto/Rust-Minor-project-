// asm.rs — single-pass assembler with line-numbered errors.

use std::fs;
use std::path::Path;

use crate::isa;

pub fn run(input: &Path, output: &Path) -> Result<(), String> {
    let src = fs::read_to_string(input)
        .map_err(|e| format!("cannot read {}: {}", input.display(), e))?;

    let mut code: Vec<u8> = Vec::new();
    let mut last_op: Option<isa::Op> = None;
    let mut non_empty_line_count = 0usize;

    for (lineno, raw) in src.lines().enumerate() {
        let lineno = lineno + 1; // 1-indexed
        let stripped = strip_comment(raw);
        if stripped.trim().is_empty() {
            continue;
        }
        non_empty_line_count += 1;

        let op = isa::parse_line(&stripped)
            .map_err(|e| format!("error: line {}: {}", lineno, e))?;

        // Defence in depth: PUSH overflow already caught in parse_line, but slots
        // are u8 so already safe.
        code.extend_from_slice(&isa::encode(&op));
        last_op = Some(op);
    }

    if non_empty_line_count == 0 {
        return Err("error: empty program (no instructions)".into());
    }

    if !matches!(last_op, Some(isa::Op::Halt)) {
        eprintln!("warning: program does not end in HALT");
    }

    let len: u32 = code.len().try_into()
        .map_err(|_| "code length exceeds u32")?;

    let mut out = Vec::with_capacity(isa::HEADER_LEN + code.len());
    out.extend_from_slice(&isa::MAGIC);
    out.push(isa::VERSION);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&code);

    fs::write(output, &out)
        .map_err(|e| format!("cannot write {}: {}", output.display(), e))?;
    Ok(())
}

fn strip_comment(line: &str) -> String {
    // Split on the FIRST ';' (semicolons inside strings are not a thing in this
    // language; we don't have string literals).
    match line.find(';') {
        Some(idx) => line[..idx].to_string(),
        None => line.to_string(),
    }
}
