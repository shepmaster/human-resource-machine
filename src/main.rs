#[macro_use]
extern crate peresil;

mod parser;
mod machine;

use std::fs::File;
use std::io::prelude::*;
use std::collections::BTreeMap;

use parser::{Thing};
use machine::{Input, Output, Registers, Tile, Program, Machine};

fn append_string(input: &mut Input, s: &str) {
    input.extend(s.chars().map(Tile::Letter));
}

fn append_zero_terminated_string(input: &mut Input, s: &str) {
    append_string(input, s);
    input.push(Tile::Number(0));
}

// Given two zero-terminated words, output the word that is first in
// alphabetical order
fn level_36() -> (Input, Registers, Output) {
    let mut input = Vec::new();
    append_zero_terminated_string(&mut input, "aab");
    append_zero_terminated_string(&mut input, "aaa");

    let mut registers = BTreeMap::new();
    registers.insert(23, Tile::Number(0));
    registers.insert(24, Tile::Number(10));

    let mut output = Vec::new();
    append_string(&mut output, "aaa");

    (input, registers, output)
}

fn main() {
    let fname = ::std::env::args().nth(1).expect("filename");
    let mut f = File::open(fname).expect("File?");

    let mut s = String::new();
    f.read_to_string(&mut s).expect("read");

    let t = Thing::new(&s);

    let p: Program = t.collect();

    let (input, registers, output) = level_36();
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
