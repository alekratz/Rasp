extern crate argparse;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate ansi_term;
extern crate time;

mod lexer;
mod parser;
mod ast;
mod gatherer;
mod internal;
mod compiler;
mod util;

use lexer::Lexer;
use parser::Parser;
use compiler::Compiler;

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

fn parse_args() -> Result<Config, String> {
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
    Ok(config)
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
    // parse args
    let config_result = parse_args();
    if let &Err(ref err_str) = &config_result {
        exit_error(err_str);
    }
    let config = config_result.unwrap();
    // load file contents
    let read_result = util::read_file(&config.file);
    if let &Err(ref err) = &read_result {
        exit_error(format!("could not read {}: {}", config.file, err));
    }
    trace!("Load {}", &config.file);
    let source_text = read_result.unwrap();
    // lex
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

    let ast = parse_result.unwrap();
    // compile
    trace!("Compiling");
    let mut compiler = Compiler::new(ast);
    let compile_result = compiler.compile();
    if let Err(ref err_str) = compile_result {
        exit_error(err_str);
    }
    // save compiled file(?)
    // run(?)
    // shut down
    info!("OK");
    trace!("Clean exit");
}
