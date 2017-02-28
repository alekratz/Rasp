// error_chain is known to recurse deeply
#![recursion_limit = "1024"]

extern crate argparse;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate ansi_term;
extern crate time;
#[macro_use]
extern crate error_chain;
extern crate libc;
#[macro_use]
extern crate lazy_static;

mod lexer;
mod parser;
mod ast;
mod gatherer;
mod internal;
mod preprocessor;
mod util;
mod vm;
mod bytecode;
mod errors {
    // error_chain setup
    error_chain! { }
}
mod builtins;

use lexer::Lexer;
use parser::Parser;
use preprocessor::Preprocessor;
use internal::{FunTable,TypeTable};

use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter, LogLevel};
use argparse::{ArgumentParser, Store, StoreTrue};
use ansi_term::{Style, Colour};

use std::env;
use std::process;
use std::fmt::Display;

struct Config {
    file: String,       // file to compile
    compile_only: bool, // compile only; don't run
    run_only: bool,     // run only; don't compile
}

impl Config {
    pub fn new() -> Config {
        Config {
            file: String::new(),
            compile_only: false,
            run_only: false,
        }
    }
}

fn parse_args() -> Config {
    let mut config = Config::new();
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("The rasp interpreter");
        ap.refer(&mut config.file)
            .add_argument("file", Store, "file to run");
        ap.refer(&mut config.compile_only)
            .add_option(&["-c", "--compile-only"], StoreTrue, "only compile; don't run");
        ap.refer(&mut config.run_only)
            .add_option(&["-r", "--run-only"], StoreTrue, "only run; don't compile");
        //ap.refer(&mut config.verbose)
        //    .add_option(&["-v", "--verbose"], StoreTrue, "verbose output");
        ap.parse_args_or_exit();
    }
    config
}

fn exit_error<T: Display>(err_str: T) {
    error!("Error: {}", err_str);
    trace!("Exiting with error");
    process::exit(1);
}

fn main() {
    // init logger
    {
        let logger_format = |record: &LogRecord| {
            let now_spec = time::get_time();
            let now = now_spec.sec as f64 + (now_spec.nsec as f64 / 1000000000.0);
            // TODO : source of the log message
            let color = match record.level() {
                LogLevel::Error => Colour::Red.bold(),
                LogLevel::Warn => Style::new().fg(Colour::Yellow),
                LogLevel::Info => Style::new().fg(Colour::White),
                _ => Colour::White.dimmed(),
            };
            format!("{}", color.paint(format!("{time:.2} [{level:07}] {msg}", 
                                              time=now, level=record.level(), msg=record.args())))
        };
        let mut builder = LogBuilder::new();
        builder.format(logger_format)
               .filter(None, LogLevelFilter::Warn);
        if let Ok(env_var) = env::var("RUST_LOG") {
            builder.parse(env_var.as_str());
        }
        builder.init().unwrap();
    }
    trace!("Starting up");
    trace!("Parsing args");
    // parse args; this automatically exits on failure
    let config = parse_args();

    // load file contents
    let read_result = util::read_file(&config.file);
    if let &Err(ref err) = &read_result {
        exit_error(format!("could not read {}: {}", config.file, err));
    }
    trace!("Load {}", &config.file);

    // lex
    let source_text = read_result.unwrap();
    trace!("Creating lexer");
    let lexer = Lexer::new(&source_text);

    // parse
    trace!("Creating parser");
    let mut parser = Parser::new(lexer);
    trace!("Making AST");
    let parse_result = parser.parse();
    if let Err(ref err_str) = parse_result {
        exit_error(err_str);
    }
    let mut ast = parse_result.unwrap();
    let mut fun_table = FunTable::new(Vec::new());
    let mut type_table = TypeTable::new(Vec::new());

    // Preprocess 
    {
        trace!("Preprocessing");
        let mut preprocessor = Preprocessor::new(&config.file, &mut ast, &mut fun_table, &mut type_table);
        let compile_result = preprocessor.preprocess();
        if let Err(ref err_chain) = compile_result {
            error!("Compile error. Halting.");
            error!("Error details:");
            error!("Caused by {}", err_chain.iter()
                   .nth(0)
                   .unwrap());
            for err in err_chain.iter().skip(1) {
                error!("    caused by {}", err);
            }
            exit_error("Compilation failed");
        }
    }
    // Make bytecode
    let bytecode = {
        let mut to_bytecode = bytecode::ToBytecode::new(&mut fun_table, &mut type_table);
        match to_bytecode.to_bytecode(&ast) {
            Ok(codez) => codez,
            Err(err_chain) => {
                error!("Compile error. Halting.");
                error!("Error details:");
                error!("Caused by {}", err_chain.iter()
                       .nth(0)
                       .unwrap());
                for err in err_chain.iter().skip(1) {
                    error!("    caused by {}", err);
                }
                exit_error("Compilation failed");
                unreachable!()
            }
        }
    };

    // bytecode debug
    debug!("Here comes the bytecode");
    for b in &bytecode {
        debug!("{:?}", b);
    }

    // save compiled file(?)
    // run(?)
    let mut vma = vm::VM::new(fun_table, type_table);
    match vma.run(&bytecode) {
        Ok(()) => info!("OK"),
        Err(err_chain) => {
            error!("Runtime error. Halting.");
            error!("Error details:");
            error!("Caused by {}", err_chain.iter()
                   .nth(0)
                   .unwrap());
            for err in err_chain.iter().skip(1) {
                error!("    caused by {}", err);
            }
            exit_error("Compilation failed");
            unreachable!()
        }
    }
    // shut down
    trace!("Clean exit");
}
