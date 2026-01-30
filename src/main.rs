mod chunk;
mod compiler;
mod debug;
mod native;
mod scanner;
mod value;
mod vm;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;
use vm::{InterpretResult, VM};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut vm = VM::new();

    match args.len() {
        1 => repl(&mut vm),
        2 => run_file(&mut vm, &args[1]),
        _ => {
            eprintln!("Usage: rlox [path]");
            process::exit(64);
        }
    }
}

fn repl(vm: &mut VM) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) | Err(_) => {
                println!();
                break;
            }
            Ok(_) => {
                vm.interpret(&line);
            }
        }
    }
}

fn run_file(vm: &mut VM, path: &str) {
    let source = fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("Could not open file \"{}\": {}", path, err);
        process::exit(74);
    });

    match vm.interpret(&source) {
        InterpretResult::Ok => {}
        InterpretResult::CompileError => process::exit(65),
        InterpretResult::RuntimeError => process::exit(70),
    }
}
