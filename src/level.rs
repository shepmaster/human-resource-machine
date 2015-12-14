use std::collections::BTreeMap;

use super::machine::{Input, Output, Registers, Tile};

pub type Level = (Input, Registers, Output);

// Copy inbox to outbox
pub fn level_1() -> Level {
    let input = from_numbers(&[1, 2, 3]);

    let registers = BTreeMap::new();

    let output = input.clone();

    (input, registers, output)
}

// Copy long inbox to outbox
pub fn level_2() -> Level {
    let input = from_string("initialize");

    let registers = BTreeMap::new();

    let output = input.clone();

    (input, registers, output)
}

// Copy from tiles to outbox
pub fn level_3() -> Level {
    let input = from_numbers(&[-99, -99, -99, -99]);

    let mut registers = BTreeMap::new();
    for (i, c) in "ujxgbe".chars().enumerate() {
        registers.insert(i as u8, Tile::Letter(c));
    }

    let output = from_string("bug");

    (input, registers, output)
}

// Swap pairs from the input
pub fn level_4() -> Level {
    let input = parse_mixed("6,4,-1,7,ih");

    let registers = BTreeMap::new();

    let output = parse_mixed("4,6,7,-1,hi");

    (input, registers, output)
}

// Copy inbox to outbox, losing duplicates
pub fn level_35() -> Level {
    let input = from_string("eabedebaeb");

    let mut registers = BTreeMap::new();
    registers.insert(14, Tile::num(0));

    let output = from_string("eabd");

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

    let output = from_string("aaa");

    (input, registers, output)
}

// There are pairs of letters and next pointers in the registers,
// starting at the input, follow the chain of registers until you get
// to -1. Output each letter on the way.
pub fn level_37() -> Level {
    let input = from_numbers(&[0, 23]);

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

    let output = from_string("escapeape");

    (input, registers, output)
}

// Given numbers, output the digits of the numbers
pub fn level_38() -> Level {
    let input = from_numbers(&[33, 505, 7, 979]);

    let mut registers = BTreeMap::new();
    registers.insert(9, Tile::num(0));
    registers.insert(10, Tile::num(10));
    registers.insert(11, Tile::num(100));

    let output = from_numbers(&[3, 3, 5, 0, 5, 7, 9, 7, 9]);

    (input, registers, output)
}

fn parse_mixed(s: &str) -> Input {
    let mut input = Vec::new();

    for part in s.split(",") {
        match part.parse() {
            Ok(n) => input.push(Tile::num(n)),
            Err(..) => append_string(&mut input, part),
        }
    }

    input
}

fn from_numbers(n: &[i16]) -> Input {
    let mut input = Vec::new();
    append_numbers(&mut input, n);
    input
}

fn append_numbers(input: &mut Input, n: &[i16]) {
    input.extend(n.iter().cloned().map(Tile::num))
}

fn from_string(s: &str) -> Input {
    let mut input = Vec::new();
    append_string(&mut input, s);
    input
}

fn append_string(input: &mut Input, s: &str) {
    input.extend(s.chars().map(Tile::Letter));
}

fn append_zero_terminated_string(input: &mut Input, s: &str) {
    append_string(input, s);
    input.push(Tile::num(0));
}
