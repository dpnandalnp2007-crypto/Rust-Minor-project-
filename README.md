# Rust Minor Project (Intersnship)
# MINIVM : A Deterministic Stack-Based Virtual Machine

A stack-based bytecode virtual machine with its own assembler and
disassembler. One binary, three subcommands, sharing a single instruction
definition. The ISA is straight-line arithmetic only — no jumps, no loops.

> Coursework project. Built from a spec that says "implement exactly,
> change nothing".

---

## Build

```bash
cargo build --release
```

The binary is at `target/release/minivm`.

---

## Usage

```
minivm asm  <input.tasm> -o <output.tbc>
minivm run  <input.tbc> [--trace]
minivm dis  <input.tbc> [-o <output.tasm>]
```

| Subcommand | What it does                                            |
| ---------- | ------------------------------------------------------- |
| `asm`      | Compile `.tasm` text → `.tbc` bytecode. Single-pass.    |
| `run`      | Execute a `.tbc` file on the stack machine.             |
| `dis`      | Disassemble `.tbc` → `.tasm` text.                      |
| `run --trace` | Print `ip`, decoded instruction, and stack before every step (debugger for everything). |

Exit codes:

| Code | Meaning                              |
| ---- | ------------------------------------ |
| 0    | Program halted cleanly               |
| 1    | VM trap (any of the 5 trap classes)  |
| 2    | CLI usage error                      |

---

## ISA at a glance

| Byte  | Mnemonic | Operand | Effect                          |
| ----- | -------- | ------- | ------------------------------- |
| 0x01  | PUSH     | i64 LE  | Push `n`                        |
| 0x02  | POP      | —       | Discard top                     |
| 0x03  | DUP      | —       | Duplicate top                   |
| 0x04  | SWAP     | —       | Swap top two                    |
| 0x10  | ADD      | —       | Pop b, pop a, push a+b          |
| 0x11  | SUB      | —       | Pop b, pop a, push a-b          |
| 0x12  | MUL      | —       | Pop b, pop a, push a*b          |
| 0x13  | DIV      | —       | Pop b, pop a, push a/b (b≠0)    |
| 0x14  | MOD      | —       | Pop b, pop a, push a%b (b≠0)    |
| 0x15  | NEG      | —       | Pop a, push -a                  |
| 0x40  | LOAD     | u8 slot | Push global slot `s`            |
| 0x41  | STORE    | u8 slot | Pop into global slot `s`        |
| 0x60  | PRINT    | —       | Pop, print with newline         |
| 0xFF  | HALT     | —       | Stop                            |

- **Word size:** 64-bit signed integers (`i64`).
- **Operand stack:** max 1024 entries.
- **Globals:** 256 `i64` slots, zero-initialized.
- **Endianness:** little-endian for multi-byte operands.
- **`ip`:** only moves forward.

---

## File format (`.tbc`)

```
+--------+--------+--------+--------+--------+--- (code bytes) ---+
| M      | V      | M      | \0     | ver    | len (u32 LE)      |
+--------+--------+--------+--------+--------+-------------------+
  0x4D     0x56     0x4D     0x00    0x01
```

- Magic: `4D 56 4D 00` ("`MVM\0`")
- Version: `0x01`
- Code length: u32 little-endian
- Then: raw code bytes

If the magic is wrong, the version is unsupported, or the body is
shorter than the length header claims, `run` and `dis` refuse to
proceed with a clear error message.

---

## Trap classes (5)

Every trap is reported in this format:

```
trap at ip=0x<HEX>: <reason>
```

…and the process exits with code `1`.

| # | Class                 | Reason                                |
| - | --------------------- | ------------------------------------- |
| 1 | Stack underflow       | `<OP>` on empty stack (e.g. POP, SUB) |
| 1 | Stack overflow        | Pushing the 1025th value              |
| 2 | Division by zero      | `DIV` or `MOD` with `b == 0`          |
| 2 | Division overflow     | `i64::MIN / -1` or `i64::MIN % -1`    |
| 3 | Unknown opcode        | A byte not in the ISA                 |
| 4 | Truncated instruction | PUSH/LOAD/STORE operand cut off       |
| 5 | `ip` past end         | Program ran off the end without HALT  |

Notes on class 2: the spec only requires trapping `b == 0`. We also
trap `i64::MIN / -1` (which would otherwise wrap to `i64::MIN`) to
avoid silent wrap-around; this is a defensible superset of the spec.

---

## Infix → Stack-code translation table

The classic pattern is: **postfix operands, then the operator**,
pushed on a stack that always holds partial results. Use a global
slot to remember a value if you need it more than once.

### arith.tasm — `(7 + 3) * (9 - 4) / 5`

| Step | Infix slice             | Stack after   | Code               |
| ---- | ----------------------- | ------------- | ------------------ |
| 1    | `7`                     | `[7]`         | `PUSH 7`           |
| 2    | `3`                     | `[7, 3]`      | `PUSH 3`           |
| 3    | `7 + 3`                 | `[10]`        | `ADD`              |
| 4    | `9`                     | `[10, 9]`     | `PUSH 9`           |
| 5    | `4`                     | `[10, 9, 4]`  | `PUSH 4`           |
| 6    | `9 - 4`                 | `[10, 5]`     | `SUB`              |
| 7    | `(7+3) * (9-4)`         | `[50]`        | `MUL`              |
| 8    | `5`                     | `[50, 5]`     | `PUSH 5`           |
| 9    | `… / 5`                 | `[10]`        | `DIV`              |
| 10   | (print)                 | `[]`          | `PRINT`            |

### horner.tasm — `3x³ + 2x² + 5x + 7` at `x = 11`  via Horner

`x` lives in slot 0. Horner: `((3*x + 2)*x + 5)*x + 7`.

| Step | Infix slice         | Stack after   | Code                |
| ---- | ------------------- | ------------- | ------------------- |
| 1    | put x in slot 0     | `[11]`        | `PUSH 11`           |
|      |                     | `[]`          | `STORE 0`           |
| 2    | `x`                 | `[11]`        | `LOAD 0`            |
| 3    | `3 * x`             | `[33]`        | `PUSH 3` `MUL`      |
| 4    | `+ 2`               | `[35]`        | `PUSH 2` `ADD`      |
| 5    | `* x`               | `[385]`       | `LOAD 0` `MUL`      |
| 6    | `+ 5`               | `[390]`       | `PUSH 5` `ADD`      |
| 7    | `* x`               | `[4290]`      | `LOAD 0` `MUL`      |
| 8    | `+ 7`               | `[4297]`      | `PUSH 7` `ADD`      |
| 9    | print               | `[]`          | `PRINT`             |

### celsius.tasm — `100 °C → °F`

`F = C * 9 / 5 + 32`.

| Step | Infix slice          | Stack after   | Code              |
| ---- | -------------------- | ------------- | ----------------- |
| 1    | `C = 100`            | `[100]`       | `PUSH 100`        |
| 2    | `* 9`                | `[900]`       | `PUSH 9` `MUL`    |
| 3    | `/ 5`                | `[180]`       | `PUSH 5` `DIV`    |
| 4    | `+ 32`               | `[212]`       | `PUSH 32` `ADD`   |
| 5    | print                | `[]`          | `PRINT`           |

### stackplay.tasm — `a² + b²` at `a=12, b=35`

Each input is pushed **exactly once** into a slot; later we `LOAD` it
twice for the squares. This forces the use of `LOAD`/`STORE` so the
spec constraint "each input pushed exactly once" is met.

| Step | Infix slice          | Stack after   | Code                |
| ---- | -------------------- | ------------- | ------------------- |
| 1    | put a=12 in slot 0   | `[]`          | `PUSH 12` `STORE 0` |
| 2    | put b=35 in slot 1   | `[]`          | `PUSH 35` `STORE 1` |
| 3    | `a` `a`              | `[12, 12]`    | `LOAD 0` `LOAD 0`   |
| 4    | `a²`                 | `[144]`       | `MUL`               |
| 5    | `b` `b`              | `[144, 35, 35]` | `LOAD 1` `LOAD 1` |
| 6    | `b²`                 | `[144, 1225]` | `MUL`               |
| 7    | `a² + b²`            | `[1369]`      | `ADD`               |
| 8    | print                | `[]`          | `PRINT`             |

### digits.tasm — digits of `9274` on four lines using only `DIV`/`MOD`

Strategy: at each step the stack holds the remaining number. We pop it
twice into `(quotient, divisor=10)`, do `DIV` and `MOD` separately,
print the digit. The program is **unrolled** so it works without
jumps.

For `9274`:

| Iter | Before         | `N / 10`      | `N % 10`      | Print line |
| ---- | -------------- | ------------- | ------------- | ---------- |
| 1    | `[9274]`       | `[927]`       | `[4]`         | `4`        |
| 2    | `[9274]` again → `927` | `[92]` | `[7]`     | `7`        |
| 3    | `[927]` → `92` | `[9]`         | `[2]`         | `2`        |
| 4    | `[92]` → `9`   | `[0]`         | `[9]`         | `9`        |

Result: `4` / `7` / `2` / `9` on four lines (the order matches the
spec because the spec just says "the digits of 9274 on four lines").

---

## Acceptance tests

Run them all:

```bash
./scripts/run_tests.sh
```

(`scripts/run_tests.sh` assembles each `.tasm`, runs it, compares
output to expected, then round-trips through `dis` and confirms
byte-identity.)

| File                     | Expected output      |
| ------------------------ | -------------------- |
| `tests/arith.tasm`       | `10`                 |
| `tests/horner.tasm`      | `4297`               |
| `tests/celsius.tasm`     | `212`                |
| `tests/stackplay.tasm`   | `1369`               |
| `tests/digits.tasm`      | `4\n7\n2\n9`         |

Trap tests (`tests/traps/`) — each must exit non-zero and report
the correct trap class and `ip`:

| File                          | Trap class          | Expected trap                                |
| ----------------------------- | ------------------- | -------------------------------------------- |
| `traps/underflow.tasm`        | Stack underflow     | `trap at ip=0x0000: stack underflow (POP on empty stack)` |
| `traps/overflow.tasm`         | Stack overflow      | `trap at ip=0x2400: stack overflow (stack full at 1024)` |
| `traps/divzero.tasm`          | Division by zero    | `trap at ip=0x0012: division by zero (DIV)`  |
| `traps/modzero.tasm`          | Division by zero    | `trap at ip=0x0012: division by zero (MOD)`  |
| `traps/modzero.tasm`          | Division by zero    | `trap at ip=0x0012: division by zero (MOD)`  |
| `traps/unknown.tbc`           | Unknown opcode      | `trap at ip=0x0000: unknown opcode 0xEE`     |
| `traps/truncated.tbc`         | Truncated instr.    | `trap at ip=0x0000: truncated instruction`   |
| `traps/ippastend.tasm`        | `ip` past end       | `trap at ip=0x000A: ip past end without HALT` |

`unknown.tbc` and `truncated.tbc` are hand-crafted binaries — those
trap classes cannot be expressed in the `.tasm` surface language.

---

## Repository layout

```
minivm/
├── Cargo.toml
├── README.md               ← you are here
├── src/
│   ├── main.rs             ← CLI dispatcher
│   ├── isa.rs              ← THE ONLY file that knows byte encodings
│   ├── asm.rs              ← single-pass assembler (line-numbered errors)
│   ├── vm.rs               ← stack machine + trap reporting
│   └── dis.rs              ← disassembler (round-trip identical)
├── tests/
│   ├── arith.tasm
│   ├── horner.tasm
│   ├── celsius.tasm
│   ├── stackplay.tasm
│   ├── digits.tasm
│   └── traps/
│       ├── underflow.tasm
│       ├── overflow.tasm
│       ├── divzero.tasm
│       ├── modzero.tasm
│       ├── modzero.tasm
│       ├── ippastend.tasm
│       ├── unknown.tbc
│       └── truncated.tbc
└── scripts/
    └── run_tests.sh        ← runs every acceptance + trap test
```

---

## What I learned while building this

- The whole project pivots on a tiny `isa.rs`. Once `encode`/`decode`
  are right, the assembler and disassembler are two sides of the same
  coin. Write the round-trip test for `encode`/`decode` *first* — it
  catches a dozen bugs at once and everything else falls into place.
- `--trace` is more than a debug flag: it's your debugger for the
  rest of the project. Build it on day one. (The spec actually says
  this in the stretch-goal hint.)
- The "stack overflow" trap test is a fun edge case: 1025 `PUSH 1`
  instructions, the 1025th one fails, `ip` is `1025 * 9 = 9216 = 0x2400`
  — and that's exactly what we get. The ip arithmetic is part of the
  spec.
- "asm → dis → asm byte-identical" forces a clean canonical text
  format. No comments on the way back. Mnemonics upper-cased. One
  space between mnemonic and operand. Trailing newline. Once the
  format is fixed, the round-trip property is automatic.
