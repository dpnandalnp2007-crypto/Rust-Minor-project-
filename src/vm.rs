// vm.rs — stack machine that executes frozen ISA bytecode.

use std::fmt::Write;
use std::process;

use crate::isa;

pub struct Vm {
    stack: Vec<i64>,
    globals: [i64; isa::NUM_SLOTS],
    trace: bool,
}

#[derive(Debug)]
pub enum Trap {
    StackUnderflow { ip: usize, what: &'static str },
    StackOverflow  { ip: usize },
    DivByZero      { ip: usize, what: &'static str }, // "DIV" or "MOD"
    DivOverflow    { ip: usize, what: &'static str }, // i64::MIN / -1, etc.
    UnknownOpcode  { ip: usize, opc: u8 },
    TruncatedInstr { ip: usize },
    IpPastEnd      { ip: usize },
}

impl Trap {
    pub fn report(&self) -> String {
        match self {
            Trap::StackUnderflow { ip, what } =>
                format!("trap at ip=0x{:04X}: stack underflow ({} on empty stack)", ip, what),
            Trap::StackOverflow  { ip } =>
                format!("trap at ip=0x{:04X}: stack overflow (stack full at {})", ip, isa::MAX_STACK),
            Trap::DivByZero { ip, what } =>
                format!("trap at ip=0x{:04X}: division by zero ({})", ip, what),
            Trap::DivOverflow { ip, what } =>
                format!("trap at ip=0x{:04X}: division overflow ({})", ip, what),
            Trap::UnknownOpcode { ip, opc } =>
                format!("trap at ip=0x{:04X}: unknown opcode 0x{:02X}", ip, opc),
            Trap::TruncatedInstr { ip } =>
                format!("trap at ip=0x{:04X}: truncated instruction", ip),
            Trap::IpPastEnd { ip } =>
                format!("trap at ip=0x{:04X}: ip past end without HALT", ip),
        }
    }
}

impl Vm {
    pub fn new(trace: bool) -> Self {
        Self {
            stack: Vec::with_capacity(isa::MAX_STACK),
            globals: [0i64; isa::NUM_SLOTS],
            trace,
        }
    }

    fn push(&mut self, v: i64, ip: usize) -> Result<(), Trap> {
        if self.stack.len() >= isa::MAX_STACK {
            return Err(Trap::StackOverflow { ip });
        }
        self.stack.push(v);
        Ok(())
    }

    fn pop(&mut self, what: &'static str, ip: usize) -> Result<i64, Trap> {
        match self.stack.pop() {
            Some(v) => Ok(v),
            None => Err(Trap::StackUnderflow { ip, what }),
        }
    }

    fn trace_line(&self, ip: usize, op: &isa::Op) {
        // ip=<hex>  <op>  stack=[a, b, c]
        let mut stack_str = String::new();
        write!(stack_str, "[").unwrap();
        for (i, v) in self.stack.iter().enumerate() {
            if i > 0 { write!(stack_str, ", ").unwrap(); }
            write!(stack_str, "{}", v).unwrap();
        }
        write!(stack_str, "]").unwrap();
        eprintln!("ip=0x{:04X}  {:<12} stack={}", ip, isa::disasm(op), stack_str);
    }

    pub fn run(&mut self, code: &[u8]) -> Result<(), Trap> {
        let mut ip: usize = 0;
        // We do NOT strictly require HALT — we detect ip-past-end as a trap —
        // but if we see HALT we stop cleanly. The spec says "warn otherwise" in
        // the assembler, not "trap" in the VM.
        loop {
            if ip >= code.len() {
                return Err(Trap::IpPastEnd { ip });
            }
            // Decode
            let (op, consumed) = match isa::decode(code, ip) {
                Ok(v) => v,
                Err(isa::DecodeError::Truncated) =>
                    return Err(Trap::TruncatedInstr { ip }),
                Err(isa::DecodeError::UnknownOpcode(b)) =>
                    return Err(Trap::UnknownOpcode { ip, opc: b }),
                Err(isa::DecodeError::BadSlotOperand) =>
                    return Err(Trap::TruncatedInstr { ip }), // same idea
            };

            if self.trace {
                self.trace_line(ip, &op);
            }

            // Execute
            match op {
                isa::Op::Push(n) => self.push(n, ip)?,
                isa::Op::Pop => { let _ = self.pop("POP", ip)?; }
                isa::Op::Dup => {
                    let top = *self.stack.last().ok_or(Trap::StackUnderflow { ip, what: "DUP" })?;
                    self.push(top, ip)?;
                }
                isa::Op::Swap => {
                    if self.stack.len() < 2 {
                        return Err(Trap::StackUnderflow { ip, what: "SWAP" });
                    }
                    let n = self.stack.len();
                    self.stack.swap(n - 1, n - 2);
                }
                isa::Op::Add => {
                    let b = self.pop("ADD", ip)?;
                    let a = self.pop("ADD", ip)?;
                    self.push(a.wrapping_add(b), ip)?;
                }
                isa::Op::Sub => {
                    let b = self.pop("SUB", ip)?;
                    let a = self.pop("SUB", ip)?;
                    self.push(a.wrapping_sub(b), ip)?;
                }
                isa::Op::Mul => {
                    let b = self.pop("MUL", ip)?;
                    let a = self.pop("MUL", ip)?;
                    self.push(a.wrapping_mul(b), ip)?;
                }
                isa::Op::Div => {
                    let b = self.pop("DIV", ip)?;
                    let a = self.pop("DIV", ip)?;
                    if b == 0 {
                        return Err(Trap::DivByZero { ip, what: "DIV" });
                    }
                    // Detect i64::MIN / -1 (would otherwise wrap to i64::MIN)
                    if a == i64::MIN && b == -1 {
                        return Err(Trap::DivOverflow { ip, what: "DIV" });
                    }
                    self.push(a.wrapping_div(b), ip)?;
                }
                isa::Op::Mod => {
                    let b = self.pop("MOD", ip)?;
                    let a = self.pop("MOD", ip)?;
                    if b == 0 {
                        return Err(Trap::DivByZero { ip, what: "MOD" });
                    }
                    // In Rust `i64::MIN % -1` panics in debug, returns 0 in release.
                    // We trap it explicitly to be safe.
                    if a == i64::MIN && b == -1 {
                        return Err(Trap::DivOverflow { ip, what: "MOD" });
                    }
                    self.push(a.wrapping_rem(b), ip)?;
                }
                isa::Op::Neg => {
                    let a = self.pop("NEG", ip)?;
                    self.push(a.wrapping_neg(), ip)?;
                }
                isa::Op::Load(s) => {
                    let v = self.globals[s as usize];
                    self.push(v, ip)?;
                }
                isa::Op::Store(s) => {
                    let v = self.pop("STORE", ip)?;
                    self.globals[s as usize] = v;
                }
                isa::Op::Print => {
                    let v = self.pop("PRINT", ip)?;
                    println!("{}", v);
                }
                isa::Op::Halt => return Ok(()),
            }

            ip += consumed;
        }
    }
}

pub fn execute(code: &[u8], trace: bool) -> Result<(), String> {
    let mut vm = Vm::new(trace);
    match vm.run(code) {
        Ok(()) => Ok(()),
        Err(trap) => {
            eprintln!("{}", trap.report());
            process::exit(1);
        }
    }
}
