#[macro_use]
extern crate peresil;
extern crate rustc_serialize;
extern crate docopt;

mod parser;
mod compiler;
mod machine;
mod level;

use std::fs::File;
use std::io::prelude::*;

use parser::Parser;
use compiler::Program;
use machine::Machine;

use docopt::Docopt;

#[derive(Debug, Copy, Clone)]
pub enum Register {
    Direct(u8),
    Indirect(u8),
}

fn report_parsing_error(s: &str, offset: usize, errors: &[parser::Error]) {
    let upto = &s[..offset];
    let leading_nl = upto.rfind("\n").map(|x| x + 1).unwrap_or(0);
    let after = &s[offset..];
    let trailing_nl = after.find("\n").unwrap_or(after.len()) + offset;

    let line = &s[leading_nl..trailing_nl];
    let inner_offset = offset - leading_nl;

    println!("Error occured while parsing:");
    println!("{}", line);
    for _ in 0..inner_offset { print!(" ") }
    println!("^");
    println!("{:?}", errors);
}

const USAGE: &'static str = "
Human Resource Machine simulator.

Usage:
  human-resource-machine <level> <file>
";

#[derive(Debug, Clone, RustcDecodable)]
struct Args {
    arg_level: usize,
    arg_file: String,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let mut f = File::open(args.arg_file).expect("Could not open source file");

    let mut s = String::new();
    f.read_to_string(&mut s).expect("Could not read source file");

    let t = Parser::new(&s);

    let p = match Program::compile(t) {
        Ok(p) => p,
        Err(compiler::Error::ParserError((offset, errors))) => {
            report_parsing_error(&s, offset, &errors);
            return;
        },
        Err(e) =>  {
            println!("Error occurred while compiling: {:?}", e);
            return;
        },
    };
    let program_length = p.stats_len();

    let (input, registers, output) = match args.arg_level {
        1 => level::level_1(),
        2 => level::level_2(),
        3 => level::level_3(),
        35 => level::level_35(),
        36 => level::level_36(),
        37 => level::level_37(),
        38 => level::level_38(),
        _ => panic!("Unknown level {}", args.arg_level),
    };
    let mut m = Machine::new(p, input, registers);

    match m.run() {
        Ok(..) => {
            let actual_output = m.output();
            println!("Program completed");
            if actual_output == &output {
                let stats = m.stats();

                println!("Output matched!");
                println!("==========");
                println!("Instructions {}", program_length);
                println!("Runtime      {}", stats.runtime);
                println!("Memory Usage {}", stats.memory_usage);
            } else {
                println!("Output did not match");
                println!("Expected: {:?}", output);
                println!("Got:      {:?}", actual_output);
            }
        },
        Err(e) => {
            println!("Program failed");
            println!("{:?}", e);
        }
    }
}
