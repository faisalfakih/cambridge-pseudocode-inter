#![cfg(not(target_arch = "wasm32"))]

mod Lexer;
mod Parser;
mod Inter;
mod errortype;

use std::fs;
use std::process;
use clap::{Arg, Command};
use crate::Parser::parser;


fn main() {
    let matches = Command::new("cps")
        .author("Faisal Fakih")
        .version("0.1.15")
        .about("Interpreter for the Cambridge Pseudocode Language")
        .arg(
            Arg::new("file")
                .value_name("FILE")
                .help("Path to the .cps file to execute")
                .index(1)
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(Arg::new("parse")
            .short('p')
            .long("parse")
            .help("Only parse the file and print the AST")
            .action(clap::ArgAction::SetTrue)
        )
        .arg(Arg::new("time")
            .short('t')
            .long("time")
            .help("Print how long the program took to run")
            .action(clap::ArgAction::SetTrue)
        )
        .get_matches();


    // get the file argument
    let filename = matches
        .get_one::<String>("file")
        .map(|s| s.as_str())
        .unwrap_or("main.cps");

    let verbose = matches.get_flag("verbose");
    let parse_only = matches.get_flag("parse");
    let show_time = matches.get_flag("time");

    let start = std::time::Instant::now();

    // read from file
    let contents = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    // tokenize
    let lexer = Lexer::lexer::Lexer::new(contents.clone());
    let tokens = match lexer.tokenize() {
        Ok(tok) => {
            if verbose {
                println!("=== Tokens ===");
                for t in &tok {
                    println!("{:?}", t);
                }
            }
            tok
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // parse
    let mut parser_instance = parser::Parser::new(tokens, contents.clone());
    let ast = match parser_instance.parse_statements() {
        Ok(ast) => {
            if verbose || parse_only {
                println!("\n=== AST ===");
                for a in &ast {
                    a.print_prefix();
                }
            }
            ast
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }; 

    if parse_only {
        process::exit(0);
    }

    // interpret
    let mut interpreter = Inter::interpreter::Interpreter::new();
    if let Err(e) = interpreter.interpret(ast) {
        eprintln!("{}", e);
        if show_time {
            let elapsed = start.elapsed();
            eprintln!("Execution time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
        }
        process::exit(1);
    }

    if show_time {
        let elapsed = start.elapsed();
        eprintln!("Execution time: {:.3}ms", elapsed.as_secs_f64() * 1000.0);
    }
}
