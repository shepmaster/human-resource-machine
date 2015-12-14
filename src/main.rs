#[macro_use]
extern crate peresil;
extern crate rustc_serialize;
extern crate docopt;

mod parser;
mod compiler;
mod machine;

use std::fs::File;
use std::io::prelude::*;
use std::collections::BTreeMap;

use parser::Parser;
use compiler::Program;
use machine::{Input, Output, Registers, Tile, Machine};

use docopt::Docopt;

#[derive(Debug, Copy, Clone)]
pub enum Register {
    Direct(u8),
    Indirect(u8),
}

fn append_string(input: &mut Input, s: &str) {
    input.extend(s.chars().map(Tile::Letter));
}

fn append_zero_terminated_string(input: &mut Input, s: &str) {
    append_string(input, s);
    input.push(Tile::num(0));
}

type Level = (Input, Registers, Output);

// Copy inbox to outbox, losing duplicates
fn level_35() -> Level {
    let mut input = Vec::new();
    append_string(&mut input, "eabedebaeb");

    let mut registers = BTreeMap::new();
    registers.insert(14, Tile::num(0));

    let mut output = Vec::new();
    append_string(&mut output, "eabd");

    (input, registers, output)
}

// Given two zero-terminated words, output the word that is first in
// alphabetical order
fn level_36() -> Level {
    let mut input = Vec::new();
    append_zero_terminated_string(&mut input, "aab");
    append_zero_terminated_string(&mut input, "aaa");

    let mut registers = BTreeMap::new();
    registers.insert(23, Tile::num(0));
    registers.insert(24, Tile::num(10));

    let mut output = Vec::new();
    append_string(&mut output, "aaa");

    (input, registers, output)
}

fn level_37() -> Level {
    let input = [0, 23].iter().cloned().map(Tile::num).collect();

    let mut registers = BTreeMap::new();
    let z = [
        (0, 'e', 13),
        (3, 'c', 23),
        (10, 'p', 20),
        (13, 's', 3),
        (20, 'e', -1),
        (23, 'a', 10),
    ];
    for &(idx, c, v) in &z {
        registers.insert(idx, Tile::Letter(c));
        registers.insert(idx + 1, Tile::num(v));
    }

    let mut output = Vec::new();
    append_string(&mut output, "escapeape");

    (input, registers, output)
}

// Given numbers, output the digits of the numbers
fn level_38() -> Level {
    let input = [33, 505, 7, 979].iter().cloned().map(Tile::num).collect();

    let mut registers = BTreeMap::new();
    registers.insert(9, Tile::num(0));
    registers.insert(10, Tile::num(10));
    registers.insert(11, Tile::num(100));

    let output = [3, 3, 5, 0, 5, 7, 9, 7, 9].iter().cloned().map(Tile::num).collect();

    (input, registers, output)
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

    let (input, registers, output) = match args.arg_level {
        35 => level_35(),
        36 => level_36(),
        37 => level_37(),
        38 => level_38(),
        _ => panic!("Unknown level {}", args.arg_level),
    };
    let mut m = Machine::new(p, input, registers);

    match m.run() {
        Ok(..) => {
            let actual_output = m.into_output();
            println!("Program completed");
            if actual_output == output {
                println!("Output matched!");
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
