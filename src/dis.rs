// dis.rs — disassembler. asm -> dis -> asm must be byte-identical.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::isa;

pub fn run(input: &Path, output: Option<&Path>) -> Result<(), String> {
    let bytes = fs::read(input)
        .map_err(|e| format!("cannot read {}: {}", input.display(), e))?;

    let code = isa::parse_header(&bytes)
        .map_err(|e| format!("{}: {}", input.display(), e))?;

    let mut out = String::new();
    let mut ip: usize = 0;
    while ip < code.len() {
        match isa::decode(code, ip) {
            Ok((op, consumed)) => {
                out.push_str(&isa::disasm(&op));
                out.push('\n');
                ip += consumed;
            }
            Err(isa::DecodeError::Truncated) => {
                return Err(format!("truncated instruction at ip=0x{:04X}", ip));
            }
            Err(isa::DecodeError::UnknownOpcode(b)) => {
                return Err(format!("unknown opcode 0x{:02X} at ip=0x{:04X}", b, ip));
            }
            Err(isa::DecodeError::BadSlotOperand) => {
                return Err(format!("bad slot operand at ip=0x{:04X}", ip));
            }
        }
    }

    match output {
        Some(p) => {
            let mut f = fs::File::create(p)
                .map_err(|e| format!("cannot create {}: {}", p.display(), e))?;
            f.write_all(out.as_bytes())
                .map_err(|e| format!("cannot write {}: {}", p.display(), e))?;
        }
        None => print!("{}", out),
    }
    Ok(())
}
