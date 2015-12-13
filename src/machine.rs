use std::iter::FromIterator;
use std::collections::BTreeMap;

use super::parser::{Token, Register};

type AbsoluteIndex = usize;

#[derive(Debug, Copy, Clone)]
enum Instruction {
    Inbox,
    Outbox,
    CopyFrom(Register),
    CopyTo(Register),
    BumpUp(Register),
    BumpDown(Register),
    Add(Register),
    Sub(Register),
    Jump(AbsoluteIndex),
    JumpIfZero(AbsoluteIndex),
    JumpIfNegative(AbsoluteIndex),
    NoOp,
}

#[derive(Debug, Clone)]
pub struct Program(Vec<Instruction>);

impl<'a> FromIterator<Token<'a>> for Program {
    fn from_iter<T>(iterator: T) -> Self
        where T: IntoIterator<Item = Token<'a>>
    {
        // Remove values that don't change the behavior
        let without_junk: Vec<_> = iterator.into_iter().filter(|t| match *t {
            Token::Header |
            Token::Comment(..) |
            Token::CommentDefinition(..) |
            Token::Whitespace(..) => false,
            _ => true,
        }).collect();

        // Find all the indexes of the labels
        let label_mapping = {
            let mut map = BTreeMap::new();

            for (i, t) in without_junk.iter().enumerate() {
                if let Token::LabelDefinition(id) = *t {
                    map.insert(id, i);
                }
            }

            map
        };

        let unmap = |id| *label_mapping.get(id).expect("Label is not defined");

        // Make the instructions, resolving jump locations
        let i = without_junk.into_iter().map(|t| match t {
            Token::Inbox => Instruction::Inbox,
            Token::Outbox => Instruction::Outbox,
            Token::CopyFrom(r) => Instruction::CopyFrom(r),
            Token::CopyTo(r) => Instruction::CopyTo(r),
            Token::BumpUp(r) => Instruction::BumpUp(r),
            Token::BumpDown(r) => Instruction::BumpDown(r),
            Token::Add(r) => Instruction::Add(r),
            Token::Sub(r) => Instruction::Sub(r),
            Token::LabelDefinition(..) => Instruction::NoOp,
            Token::Jump(id) => Instruction::Jump(unmap(id)),
            Token::JumpIfZero(id) => Instruction::JumpIfZero(unmap(id)),
            Token::JumpIfNegative(id) => Instruction::JumpIfNegative(unmap(id)),
            _ => unreachable!(),
        });

        Program(i.collect())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tile {
    Number(i8), // what is the actual size here?
    Letter(char),
}

#[derive(Debug, Clone)]
pub enum StepError {
    EndOfProgram,
    Other(String),
}

impl StepError {
    fn e(s: &str) -> StepError {
        StepError::Other(s.into())
    }
}

pub type Input = Vec<Tile>;
pub type Output = Vec<Tile>;
pub type Registers = BTreeMap<u8, Tile>;

pub struct Machine {
    program: Program,
    input: Input,
    output: Output,
    pc: usize,
    accumulator: Option<Tile>,
    registers: Registers,
}

impl Machine {
    pub fn new(program: Program, mut input: Input, registers: Registers) -> Machine {
        // We want to pop off the front, so flip it around for efficiency.
        input.reverse();

        Machine {
            program: program,
            input: input,
            output: Vec::new(),
            pc: 0,
            accumulator: None,
            registers: registers,
        }
    }

    fn deref_target(&self, r: Register) -> Result<u8, StepError> {
        match r {
            Register::Direct(r) => Ok(r),
            Register::Indirect(r) => match self.registers.get(&r) {
                None => Err(StepError::e("indirect through nil!")),
                Some(&Tile::Number(v)) if v < 0 => Err(StepError::e("indirect to negative!")),
                Some(&Tile::Number(v)) => Ok(v as u8),
                Some(&Tile::Letter(..)) => Err(StepError::e("indirect through letter")),
            },
        }
    }

    pub fn into_output(self) -> Output { self.output }

    pub fn step(&mut self) -> Result<(), StepError> {
        use self::Instruction::*;

        // println!("PC: {}", self.pc);
        // println!("Instr: {:?}", self.program.0[self.pc]);
        // println!("Acc: {:?}", self.accumulator);

        match self.program.0[self.pc] {
            Inbox => {
                match self.input.pop() {
                    Some(v) => self.accumulator = Some(v),
                    None => return Err(StepError::EndOfProgram),
                }
            },
            Outbox => {
                match self.accumulator {
                    Some(v) => self.output.push(v),
                    None => return Err(StepError::e("Can't output with nothing!")),
                }
            },
            CopyFrom(r) => {
                let r = try!(self.deref_target(r));
                let v = try!(self.registers.get(&r).ok_or(StepError::e("copy from nil")));
                self.accumulator = Some(*v);
            },
            CopyTo(r) => {
                match self.accumulator {
                    Some(v) => {
                        let r = try!(self.deref_target(r));
                        self.registers.insert(r, v);
                    },
                    None => return Err(StepError::e("nothing to copy to the tile")),
                }
            },
            BumpUp(r) => {
                let r = try!(self.deref_target(r));
                let v = match self.registers.get_mut(&r) {
                    None => return Err(StepError::e("can't bump nil")),
                    Some(&mut Tile::Number(ref mut v)) => {
                        *v = *v + 1;
                        *v
                    },
                    Some(&mut Tile::Letter(..)) => return Err(StepError::e("can't bump a letter"))
                };
                self.accumulator = Some(Tile::Number(v))
            },
            BumpDown(r) => {
                let r = try!(self.deref_target(r));
                let v = match self.registers.get_mut(&r) {
                    None => return Err(StepError::e("can't bump nil")),
                    Some(&mut Tile::Number(ref mut v)) => {
                        *v = *v - 1;
                        *v
                    },
                    Some(&mut Tile::Letter(..)) => return Err(StepError::e("can't bump a letter"))
                };
                self.accumulator = Some(Tile::Number(v))
            },
            Add(r) => {
                let r = try!(self.deref_target(r));
                let v = match (self.accumulator, self.registers.get(&r)) {
                    (None, _) => return Err(StepError::e("Cannot add with nil in hand")),
                    (_, None) => return Err(StepError::e("Cannot add to nil")),
                    (Some(Tile::Number(a)), Some(&Tile::Number(v))) => a + v,
                    _ => return Err(StepError::e("Cannot add with letters")),
                };
                self.accumulator = Some(Tile::Number(v));
            },
            Sub(r) => {
                let r = try!(self.deref_target(r));
                let v = match (self.accumulator, self.registers.get(&r)) {
                    (None, _) => return Err(StepError::e("Cannot sub with nil in hand")),
                    (_, None) => return Err(StepError::e("can't sub to nil")),
                    (Some(Tile::Number(a)), Some(&Tile::Number(v))) => a - v,
                    (Some(Tile::Letter(a)), Some(&Tile::Letter(v))) => a as i8 - v as i8,
                    _ => return Err(StepError::e("Cannot sub a letter and number")),
                };
                self.accumulator = Some(Tile::Number(v))
            },
            Jump(i) => {
                self.pc = i;
                return Ok(());
            },
            JumpIfZero(i) => {
                match self.accumulator {
                    None => return Err(StepError::e("cannot jump zero with nil in hand")),
                    Some(v) => match v {
                        Tile::Number(v) if v == 0 => {
                            self.pc = i;
                            return Ok(());
                        }
                        Tile::Number(..) |
                        Tile::Letter(..) => {}, // noop
                    }
                }
            },
            JumpIfNegative(i) => {
                match self.accumulator {
                    None => return Err(StepError::e("cannot jump neg with nil in hand")),
                    Some(v) => match v {
                        Tile::Number(v) if v < 0 => {
                            self.pc = i;
                            return Ok(());
                        }
                        Tile::Number(..) |
                        Tile::Letter(..) => {}, // noop
                    }
                }
            },
            NoOp => {},
        }

        self.pc += 1;

        if self.pc >= self.program.0.len() {
            Err(StepError::EndOfProgram)
        } else {
            Ok(())
        }
    }

    pub fn run(&mut self) -> Result<(), StepError> {
        loop {
            match self.step() {
                Ok(..) => continue,
                Err(StepError::EndOfProgram) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }
}
