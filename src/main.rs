// main.rs — CLI dispatcher for minivm.

use std::path::PathBuf;
use std::process;

mod asm;
mod dis;
mod isa;
mod vm;

const USAGE: &str = "\
minivm — stack-based bytecode VM

USAGE:
    minivm asm  <input.tasm> -o <output.tbc>
    minivm run  <input.tbc> [--trace]
    minivm dis  <input.tbc> [-o <output.tasm>]
";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprint!("{}", USAGE);
        process::exit(2);
    }
    let sub = args[1].as_str();
    let rest = &args[2..];

    let result: Result<(), String> = match sub {
        "asm" => cmd_asm(rest),
        "run" => cmd_run(rest),
        "dis" => cmd_dis(rest),
        "--help" | "-h" | "help" => {
            print!("{}", USAGE);
            Ok(())
        }
        other => Err(format!("unknown subcommand '{}'\n{}", other, USAGE)),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn cmd_asm(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("asm: missing input file".into());
    }
    let input = PathBuf::from(&args[0]);
    let mut output: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                output = Some(PathBuf::from(
                    args.get(i).ok_or("asm: -o requires a path")?,
                ));
            }
            other => return Err(format!("asm: unknown argument '{}'", other)),
        }
        i += 1;
    }
    let out = output.ok_or("asm: -o <output.tbc> required")?;
    asm::run(&input, &out)
}

fn cmd_run(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("run: missing input file".into());
    }
    let input = PathBuf::from(&args[0]);
    let trace = args.iter().any(|a| a == "--trace");
    let bytes = std::fs::read(&input)
        .map_err(|e| format!("run: cannot read {}: {}", input.display(), e))?;
    let code = isa::parse_header(&bytes)
        .map_err(|e| format!("run: {}: {}", input.display(), e))?;
    vm::execute(code, trace)
}

fn cmd_dis(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("dis: missing input file".into());
    }
    let input = PathBuf::from(&args[0]);
    let mut output: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                output = Some(PathBuf::from(
                    args.get(i).ok_or("dis: -o requires a path")?,
                ));
            }
            other => return Err(format!("dis: unknown argument '{}'", other)),
        }
        i += 1;
    }
    dis::run(&input, output.as_deref())
}
