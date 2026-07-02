// isa.rs — the ONLY file that knows byte encodings.
// asm, dis, and vm all go through it. If you write 0x41 in two places,
// you have made a mistake.

use std::fmt;

// ---------- Opcodes (the frozen ISA) ----------
pub const OP_PUSH:  u8 = 0x01;
pub const OP_POP:   u8 = 0x02;
pub const OP_DUP:   u8 = 0x03;
pub const OP_SWAP:  u8 = 0x04;
pub const OP_ADD:   u8 = 0x10;
pub const OP_SUB:   u8 = 0x11;
pub const OP_MUL:   u8 = 0x12;
pub const OP_DIV:   u8 = 0x13;
pub const OP_MOD:   u8 = 0x14;
pub const OP_NEG:   u8 = 0x15;
pub const OP_LOAD:  u8 = 0x40;
pub const OP_STORE: u8 = 0x41;
pub const OP_PRINT: u8 = 0x60;
pub const OP_HALT:  u8 = 0xFF;

// ---------- File format ----------
pub const MAGIC: [u8; 4] = [0x4D, 0x56, 0x4D, 0x00]; // "MVM\0"
pub const VERSION: u8 = 0x01;
pub const HEADER_LEN: usize = 4 + 1 + 4; // magic + version + u32 length
pub const MAX_STACK: usize = 1024;
pub const NUM_SLOTS: usize = 256;

// ---------- Instruction set ----------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Push(i64),
    Pop,
    Dup,
    Swap,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Load(u8),
    Store(u8),
    Print,
    Halt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,
    UnknownOpcode(u8),
    BadSlotOperand,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "truncated instruction"),
            DecodeError::UnknownOpcode(b) => write!(f, "unknown opcode 0x{:02X}", b),
            DecodeError::BadSlotOperand => write!(f, "bad slot operand"),
        }
    }
}

/// Encode an Op to its raw bytecode bytes (little-endian for multi-byte operands).
pub fn encode(op: &Op) -> Vec<u8> {
    match op {
        Op::Push(n) => {
            let mut v = vec![OP_PUSH];
            v.extend_from_slice(&n.to_le_bytes());
            v
        }
        Op::Pop     => vec![OP_POP],
        Op::Dup     => vec![OP_DUP],
        Op::Swap    => vec![OP_SWAP],
        Op::Add     => vec![OP_ADD],
        Op::Sub     => vec![OP_SUB],
        Op::Mul     => vec![OP_MUL],
        Op::Div     => vec![OP_DIV],
        Op::Mod     => vec![OP_MOD],
        Op::Neg     => vec![OP_NEG],
        Op::Load(s) => vec![OP_LOAD, *s],
        Op::Store(s) => vec![OP_STORE, *s],
        Op::Print   => vec![OP_PRINT],
        Op::Halt    => vec![OP_HALT],
    }
}

/// Decode starting at `ip`. Returns the Op and the number of bytes consumed.
/// `ip` is the byte offset within `code`.
pub fn decode(code: &[u8], ip: usize) -> Result<(Op, usize), DecodeError> {
    if ip >= code.len() {
        return Err(DecodeError::Truncated);
    }
    let opc = code[ip];
    match opc {
        OP_PUSH => {
            if ip + 1 + 8 > code.len() {
                return Err(DecodeError::Truncated);
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&code[ip + 1..ip + 1 + 8]);
            Ok((Op::Push(i64::from_le_bytes(buf)), 9))
        }
        OP_POP      => Ok((Op::Pop, 1)),
        OP_DUP      => Ok((Op::Dup, 1)),
        OP_SWAP     => Ok((Op::Swap, 1)),
        OP_ADD      => Ok((Op::Add, 1)),
        OP_SUB      => Ok((Op::Sub, 1)),
        OP_MUL      => Ok((Op::Mul, 1)),
        OP_DIV      => Ok((Op::Div, 1)),
        OP_MOD      => Ok((Op::Mod, 1)),
        OP_NEG      => Ok((Op::Neg, 1)),
        OP_PRINT    => Ok((Op::Print, 1)),
        OP_HALT     => Ok((Op::Halt, 1)),
        OP_LOAD | OP_STORE => {
            if ip + 1 >= code.len() {
                return Err(DecodeError::Truncated);
            }
            let s = code[ip + 1];
            if opc == OP_LOAD { Ok((Op::Load(s), 2)) } else { Ok((Op::Store(s), 2)) }
        }
        _ => Err(DecodeError::UnknownOpcode(opc)),
    }
}

/// Disassemble a single Op back to its assembly text representation.
/// IMPORTANT: round-trip property — asm(parse(line)) == line.
/// We strip comments/extra whitespace on input; we emit canonical form on output.
pub fn disasm(op: &Op) -> String {
    match op {
        Op::Push(n)  => format!("PUSH {}", n),
        Op::Pop      => "POP".into(),
        Op::Dup      => "DUP".into(),
        Op::Swap     => "SWAP".into(),
        Op::Add      => "ADD".into(),
        Op::Sub      => "SUB".into(),
        Op::Mul      => "MUL".into(),
        Op::Div      => "DIV".into(),
        Op::Mod      => "MOD".into(),
        Op::Neg      => "NEG".into(),
        Op::Load(s)  => format!("LOAD {}", s),
        Op::Store(s) => format!("STORE {}", s),
        Op::Print    => "PRINT".into(),
        Op::Halt     => "HALT".into(),
    }
}

/// Parse a single assembly instruction (no comments/whitespace).
/// Used by both asm and dis. Returns Err with a string description on failure.
pub fn parse_line(line: &str) -> Result<Op, String> {
    let line = line.trim();
    if line.is_empty() {
        return Err("empty line".into());
    }
    let mut parts = line.splitn(2, char::is_whitespace);
    let mnemonic = parts.next().unwrap().to_ascii_uppercase();
    let operand = parts.next().map(|s| s.trim());

    match mnemonic.as_str() {
        "PUSH" => {
            let n = operand.ok_or("PUSH requires an i64 operand")?
                .parse::<i64>().map_err(|e| format!("invalid PUSH operand: {}", e))?;
            Ok(Op::Push(n))
        }
        "POP"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Pop),
        "DUP"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Dup),
        "SWAP"     => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Swap),
        "ADD"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Add),
        "SUB"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Sub),
        "MUL"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Mul),
        "DIV"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Div),
        "MOD"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Mod),
        "NEG"      => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Neg),
        "PRINT"    => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Print),
        "HALT"     => require_no_operand(mnemonic.as_str(), operand).map(|_| Op::Halt),
        "LOAD" => {
            let s = parse_slot(operand)?;
            Ok(Op::Load(s))
        }
        "STORE" => {
            let s = parse_slot(operand)?;
            Ok(Op::Store(s))
        }
        other => Err(format!("unknown mnemonic '{}'", other)),
    }
}

fn require_no_operand(name: &str, operand: Option<&str>) -> Result<(), String> {
    match operand {
        None => Ok(()),
        Some("") => Ok(()),
        Some(s) => Err(format!("{} takes no operand, got '{}'", name, s)),
    }
}

fn parse_slot(operand: Option<&str>) -> Result<u8, String> {
    let s = operand.ok_or("slot operand required")?.trim();
    let n: i64 = s.parse().map_err(|e| format!("invalid slot '{}': {}", s, e))?;
    if n < 0 || n > 255 {
        return Err(format!("slot {} out of range [0, 255]", n));
    }
    Ok(n as u8)
}

/// Validate a file header and return the code slice that follows.
pub fn parse_header(bytes: &[u8]) -> Result<&[u8], String> {
    if bytes.len() < HEADER_LEN {
        return Err("file too short for header".into());
    }
    if &bytes[0..4] != MAGIC {
        return Err(format!(
            "bad magic: expected {:02X?} got {:02X?}",
            MAGIC,
            &bytes[0..4]
        ));
    }
    if bytes[4] != VERSION {
        return Err(format!("unsupported version 0x{:02X}", bytes[4]));
    }
    let len_bytes: [u8; 4] = bytes[5..9].try_into().unwrap();
    let len = u32::from_le_bytes(len_bytes) as usize;
    let expected_total = HEADER_LEN + len;
    if bytes.len() < expected_total {
        return Err(format!(
            "truncated body: header says {} bytes of code but file has {}",
            len,
            bytes.len() - HEADER_LEN
        ));
    }
    Ok(&bytes[HEADER_LEN..expected_total])
}

// ---------- Tests ----------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encode_decode() {
        let ops = vec![
            Op::Push(0),
            Op::Push(-1),
            Op::Push(i64::MIN),
            Op::Push(i64::MAX),
            Op::Pop,
            Op::Dup,
            Op::Swap,
            Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Neg,
            Op::Load(0),
            Op::Load(255),
            Op::Store(128),
            Op::Print,
            Op::Halt,
        ];
        for op in &ops {
            let bytes = encode(op);
            let (back, consumed) = decode(&bytes, 0).unwrap();
            assert_eq!(*op, back);
            assert_eq!(consumed, bytes.len());
        }
    }

    #[test]
    fn truncated_push() {
        // Only 5 bytes after opcode -> need 8
        let bytes = vec![OP_PUSH, 1, 2, 3, 4, 5];
        assert!(matches!(decode(&bytes, 0), Err(DecodeError::Truncated)));
    }

    #[test]
    fn unknown_opcode() {
        let bytes = vec![0xEE];
        assert!(matches!(decode(&bytes, 0), Err(DecodeError::UnknownOpcode(0xEE))));
    }

    #[test]
    fn roundtrip_asm_text() {
        let cases = [
            "PUSH 7", "PUSH -42", "PUSH 0",
            "POP", "DUP", "SWAP",
            "ADD", "SUB", "MUL", "DIV", "MOD", "NEG",
            "LOAD 0", "LOAD 255",
            "STORE 0", "STORE 128",
            "PRINT", "HALT",
        ];
        for line in &cases {
            let op = parse_line(line).unwrap();
            let back = disasm(&op);
            // Re-parse to confirm canonical form
            let op2 = parse_line(&back).unwrap();
            assert_eq!(op, op2, "roundtrip failed for {}", line);
        }
    }
}
