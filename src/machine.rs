use std::collections::BTreeMap;

use super::Register;

type AbsoluteIndex = usize;

#[derive(Debug, Copy, Clone)]
pub enum Instruction {
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

// Clamped at [-999, 999]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct NumberValue(i16);

impl NumberValue {
    fn from_char(c: char) -> Result<NumberValue, Error> {
        NumberValue::clamp(c as i16)
    }

    fn clamp(v: i16) -> Result<NumberValue, Error> {
        if v > 999 {
            Err(Error::Overflow)
        } else if v < -999 {
            Err(Error::Underflow)
        } else {
            Ok(NumberValue(v))
        }
    }

    fn add(self, other: NumberValue) -> Result<NumberValue, Error> {
        NumberValue::clamp(self.0 + other.0)
    }

    fn sub(self, other: NumberValue) -> Result<NumberValue, Error> {
        NumberValue::clamp(self.0 - other.0)
    }

    fn is_zero(self) -> bool { self.0 == 0 }
    fn is_negative(self) -> bool { self.0 < 0 }

    fn increment(self) -> Result<NumberValue, Error> {
        NumberValue::clamp(self.0 + 1)
    }

    fn decrement(self) -> Result<NumberValue, Error> {
        NumberValue::clamp(self.0 - 1)
    }

    fn into_u8(self) -> u8 {
        self.0 as u8 // Should have error of some kind?
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Tile {
    Number(NumberValue),
    Letter(char),
}

impl Tile {
    pub fn num(i: i16) -> Tile {
        Tile::Number(NumberValue::clamp(i).unwrap())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Error {
    EndOfProgram,
    IndirectThroughNil,
    IndirectThroughNegative,
    IndirectThroughLetter,
    OutputNil,
    CopyFromNil,
    CopyToNil,
    BumpNil,
    BumpLetter,
    AddWithNil,
    AddToNil,
    AddWithLetter,
    SubFromNil,
    SubWithNil,
    SubCrossTypes,
    JumpZeroNil,
    JumpNegativeNil,
    Underflow,
    Overflow,
}

pub type Input = Vec<Tile>;
pub type Output = Vec<Tile>;
pub type Registers = BTreeMap<u8, Tile>;

#[derive(Debug, Clone)]
pub struct Machine {
    program: Vec<Instruction>,
    input: Input,
    output: Output,
    pc: usize,
    accumulator: Option<Tile>,
    registers: Registers,
}

impl Machine {
    pub fn new<I>(program: I, mut input: Input, registers: Registers) -> Machine
        where I: IntoIterator<Item = Instruction>
    {
        // We want to pop off the front, so flip it around for efficiency.
        input.reverse();

        Machine {
            program: program.into_iter().collect(),
            input: input,
            output: Vec::new(),
            pc: 0,
            accumulator: None,
            registers: registers,
        }
    }

    fn deref_target(&self, r: Register) -> Result<u8, Error> {
        match r {
            Register::Direct(r) => Ok(r),
            Register::Indirect(r) => match self.registers.get(&r) {
                None => Err(Error::IndirectThroughNil),
                Some(&Tile::Number(v)) if v.is_negative() => Err(Error::IndirectThroughNegative),
                Some(&Tile::Number(v)) => Ok(v.into_u8()), // Should have max # registers?
                Some(&Tile::Letter(..)) => Err(Error::IndirectThroughLetter),
            },
        }
    }

    pub fn into_output(self) -> Output { self.output }

    pub fn step(&mut self) -> Result<(), Error> {
        use self::Instruction::*;

        // println!("PC: {}", self.pc);
        // println!("Instr: {:?}", self.program[self.pc]);
        // println!("Acc: {:?}", self.accumulator);

        match self.program[self.pc] {
            Inbox => {
                match self.input.pop() {
                    Some(v) => self.accumulator = Some(v),
                    None => return Err(Error::EndOfProgram),
                }
            },
            Outbox => {
                match self.accumulator {
                    Some(v) => self.output.push(v),
                    None => return Err(Error::OutputNil),
                }
            },
            CopyFrom(r) => {
                let r = try!(self.deref_target(r));
                let v = try!(self.registers.get(&r).ok_or(Error::CopyFromNil));
                self.accumulator = Some(*v);
            },
            CopyTo(r) => {
                match self.accumulator {
                    Some(v) => {
                        let r = try!(self.deref_target(r));
                        self.registers.insert(r, v);
                    },
                    None => return Err(Error::CopyToNil),
                }
            },
            BumpUp(r) => {
                let r = try!(self.deref_target(r));
                let v = match self.registers.get_mut(&r) {
                    None => return Err(Error::BumpNil),
                    Some(&mut Tile::Number(ref mut v)) => {
                        *v = try!(v.increment());
                        *v
                    },
                    Some(&mut Tile::Letter(..)) => return Err(Error::BumpLetter)
                };
                self.accumulator = Some(Tile::Number(v))
            },
            BumpDown(r) => {
                let r = try!(self.deref_target(r));
                let v = match self.registers.get_mut(&r) {
                    None => return Err(Error::BumpNil),
                    Some(&mut Tile::Number(ref mut v)) => {
                        *v = try!(v.decrement());
                        *v
                    },
                    Some(&mut Tile::Letter(..)) => return Err(Error::BumpLetter)
                };
                self.accumulator = Some(Tile::Number(v))
            },
            Add(r) => {
                let r = try!(self.deref_target(r));
                let v = match (self.accumulator, self.registers.get(&r)) {
                    (None, _) => return Err(Error::AddToNil),
                    (_, None) => return Err(Error::AddWithNil),
                    (Some(Tile::Number(a)), Some(&Tile::Number(v))) => try!(a.add(v)),
                    _ => return Err(Error::AddWithLetter),
                };
                self.accumulator = Some(Tile::Number(v));
            },
            Sub(r) => {
                let r = try!(self.deref_target(r));
                let v = match (self.accumulator, self.registers.get(&r)) {
                    (None, _) => return Err(Error::SubFromNil),
                    (_, None) => return Err(Error::SubWithNil),
                    (Some(Tile::Number(a)), Some(&Tile::Number(v))) => try!(a.sub(v)),
                    (Some(Tile::Letter(a)), Some(&Tile::Letter(v))) => {
                        let a = try!(NumberValue::from_char(a));
                        let v = try!(NumberValue::from_char(v));
                        try!(a.sub(v))
                    },
                    _ => return Err(Error::SubCrossTypes),
                };
                self.accumulator = Some(Tile::Number(v))
            },
            Jump(i) => {
                self.pc = i;
                return Ok(());
            },
            JumpIfZero(i) => {
                match self.accumulator {
                    None => return Err(Error::JumpZeroNil),
                    Some(v) => match v {
                        Tile::Number(v) if v.is_zero() => {
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
                    None => return Err(Error::JumpNegativeNil),
                    Some(v) => match v {
                        Tile::Number(v) if v.is_negative() => {
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

        if self.pc >= self.program.len() {
            Err(Error::EndOfProgram)
        } else {
            Ok(())
        }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        loop {
            match self.step() {
                Ok(..) => continue,
                Err(Error::EndOfProgram) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }
}
