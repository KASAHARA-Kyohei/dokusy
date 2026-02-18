mod ast;
mod interpreter;
mod lexer;
mod parser;

use std::env;
use std::fs;
use std::process;

use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;

fn main() {
    if let Err(err) = run_cli() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn run_cli() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(usage());
    }

    match args[1].as_str() {
        "run" => {
            if args.len() != 3 {
                return Err("`dokusy run <file.dk>` の形式で指定してください".to_string());
            }
            run_file(&args[2])
        }
        "repl" => {
            println!("REPL は未実装です（M0では後回し）");
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn run_file(path: &str) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("failed to read `{}`: {}", path, e))?;

    let tokens = Lexer::new(&source)
        .tokenize()
        .map_err(|e| format!("{}", e))?;
    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|e| format!("{}", e))?;

    let mut interpreter = Interpreter::new();
    let result = interpreter
        .run_program(program)
        .map_err(|e| format!("{}", e))?;

    print!("{}", result.output);
    Ok(())
}

fn usage() -> String {
    [
        "Usage:",
        "  dokusy run <file.dk>   Execute a dokusy program",
        "  dokusy repl            Start REPL (M0: placeholder)",
    ]
    .join("\n")
}
