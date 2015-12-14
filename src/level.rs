use std::collections::BTreeMap;

use super::machine::{Input, Output, Registers, Tile};

pub type Level = (Input, Registers, Output);

// Copy inbox to outbox, losing duplicates
pub fn level_35() -> Level {
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
pub fn level_36() -> Level {
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

// There are pairs of letters and next pointers in the registers,
// starting at the input, follow the chain of registers until you get
// to -1. Output each letter on the way.
pub fn level_37() -> Level {
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
pub fn level_38() -> Level {
    let input = [33, 505, 7, 979].iter().cloned().map(Tile::num).collect();

    let mut registers = BTreeMap::new();
    registers.insert(9, Tile::num(0));
    registers.insert(10, Tile::num(10));
    registers.insert(11, Tile::num(100));

    let output = [3, 3, 5, 0, 5, 7, 9, 7, 9].iter().cloned().map(Tile::num).collect();

    (input, registers, output)
}

fn append_string(input: &mut Input, s: &str) {
    input.extend(s.chars().map(Tile::Letter));
}

fn append_zero_terminated_string(input: &mut Input, s: &str) {
    append_string(input, s);
    input.push(Tile::num(0));
}
