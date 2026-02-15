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
        .version("0.1.2")
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
        .get_matches();

    // get the file argument
    let filename = matches
        .get_one::<String>("file")
        .map(|s| s.as_str())
        .unwrap_or("main.cps");

    let verbose = matches.get_flag("verbose");

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
            if verbose {
                println!("\n=== AST ===");
                for a in &ast {
                    println!("{:#?}", a);
                }
            }
            ast
        }
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    // interpret
    let mut interpreter = Inter::interpreter::Interpreter::new();
    if let Err(e) = interpreter.interpret(ast) {
        eprintln!("{}", e);
        process::exit(1);
    }
}
