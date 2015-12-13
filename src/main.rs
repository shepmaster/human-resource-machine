#[macro_use]
extern crate peresil;

use std::fs::File;
use std::io::prelude::*;

use peresil::{ParseMaster, StringPoint, Progress, Status, Recoverable};

#[derive(Debug, Copy, Clone)]
enum Error {
    ExpectedHeader,
    ExpectedInbox,
    ExpectedOutbox,
    ExpectedCopyFrom,
    ExpectedCopyTo,
    ExpectedBumpUp,
    ExpectedBumpDown,
    ExpectedAdd,
    ExpectedSub,
    ExpectedIndirectRegister,
    ExpectedIndirectRegisterEnd,
    ExpectedRegisterValue,
    ExpectedLabelDefinition,
    ExpectedLabelValue,
    ExpectedJump,
    ExpectedJumpIfZero,
    ExpectedJumpIfNegative,
    ExpectedWhiteSpace,
    ExpectedComment,
    ExpectedCommentId,
    ExpectedCommentDefinition,
    ExpectedCommentDefinitionData,
    ExpectedCommentDefinitionEnd,
    ExpectedColon,
}

impl Recoverable for Error {
    fn recoverable(&self) -> bool { true }
}

type ZPM<'a> = ParseMaster<StringPoint<'a>, Error>;
type ZPR<'a, T> = Progress<StringPoint<'a>, T, Error>;

struct Thing<'a> {
    point: StringPoint<'a>,
}

impl<'a> Thing<'a> {
    fn new(s: &str) -> Thing {
        Thing {
            point: StringPoint::new(s),
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Register {
    Direct(u8),
    Indirect(u8),
}

type Label<'a> = &'a str;
type CommentId<'a> = &'a str;
type CommentData<'a> = &'a str;

#[derive(Debug, Copy, Clone)]
enum Token<'a> {
    Header,
    Inbox,
    Outbox,
    CopyFrom(Register),
    CopyTo(Register),
    BumpUp(Register),
    BumpDown(Register), // name?
    Add(Register),
    Sub(Register),
    LabelDefinition(Label<'a>),
    Jump(Label<'a>),
    JumpIfZero(Label<'a>),
    JumpIfNegative(Label<'a>),
    Comment(CommentId<'a>),
    CommentDefinition(CommentId<'a>, CommentData<'a>),
    Whitespace(&'a str),
}

fn parse_header<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    pt.consume_literal("-- HUMAN RESOURCE MACHINE PROGRAM --")
        .map(|_| Token::Header)
        .map_err(|_| Error::ExpectedHeader)
}

fn parse_inbox<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    pt.consume_literal("INBOX")
        .map(|_| Token::Inbox)
        .map_err(|_| Error::ExpectedInbox)
}

fn parse_outbox<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    pt.consume_literal("OUTBOX")
        .map(|_| Token::Outbox)
        .map_err(|_| Error::ExpectedOutbox)
}

fn parse_copy_from<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_single_register_instruction(pm, pt, "COPYFROM", Token::CopyFrom, Error::ExpectedCopyFrom)
}

fn parse_copy_to<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_single_register_instruction(pm, pt, "COPYTO", Token::CopyTo, Error::ExpectedCopyTo)
}

fn parse_bump_up<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_single_register_instruction(pm, pt, "BUMPUP", Token::BumpUp, Error::ExpectedBumpUp)
}

fn parse_bump_down<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_single_register_instruction(pm, pt, "BUMPDOWN", Token::BumpDown, Error::ExpectedBumpDown)
}

fn parse_add<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_single_register_instruction(pm, pt, "ADD", Token::Add, Error::ExpectedAdd)
}

fn parse_sub<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_single_register_instruction(pm, pt, "SUB", Token::Sub, Error::ExpectedSub)
}

fn parse_single_register_instruction<'a, F>(
    pm: &mut ZPM<'a>,
    pt: StringPoint<'a>,
    instruction_name: &str,
    token_creator: F,
    error_kind: Error
)
    -> ZPR<'a, Token<'a>>
    where F: FnOnce(Register) -> Token<'a>
{
    let (pt, _) = try_parse!(pt.consume_literal(instruction_name).map_err(|_| error_kind));
    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};
    let (pt, reg) = try_parse!{parse_register(pm, pt)};

    Progress::success(pt, token_creator(reg))
}

fn parse_register<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Register> {
    pm.alternate()
        .one(|pm| parse_register_indirect(pm, pt))
        .one(|pm| parse_register_value(pm, pt).map(Register::Direct))
        .finish()
}

fn parse_register_indirect<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Register> {
    let (pt, _) = try_parse!{
        pt.consume_literal("[").map_err(|_| Error::ExpectedIndirectRegister)
    };
    let (pt, reg) = try_parse!(parse_register_value(pm, pt));
    let (pt, _) = try_parse!{
        pt.consume_literal("]").map_err(|_| Error::ExpectedIndirectRegisterEnd)
    };

    Progress::success(pt, Register::Indirect(reg))
}

fn parse_register_value<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, u8> {
    string_point_consume_while(pt, |c| c.is_digit(10))
        .map(|v| v.parse().unwrap()) // unrecoverable error
        .map_err(|_| Error::ExpectedRegisterValue)
}

fn parse_label_definition<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, val) = try_parse!{
        parse_label_value(pm, pt)
            .map_err(|_| Error::ExpectedLabelDefinition)
    };
    let (pt, _) = try_parse!(pt.consume_literal(":").map_err(|_| Error::ExpectedColon));

    Progress::success(pt, Token::LabelDefinition(val))
}

fn parse_label_value<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
    string_point_consume_while(pt, |c| c >= 'a' && c <= 'z')
        .map_err(|_| Error::ExpectedLabelValue)
}

fn parse_jump<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_jump_instruction(pm, pt, "JUMP", Token::Jump, Error::ExpectedJump)
}

fn parse_jump_if_zero<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_jump_instruction(pm, pt, "JUMPZ", Token::JumpIfZero, Error::ExpectedJumpIfZero)
}

fn parse_jump_if_negative<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    parse_jump_instruction(pm, pt, "JUMPN", Token::JumpIfNegative, Error::ExpectedJumpIfNegative)
}

fn parse_jump_instruction<'a, F>(
    pm: &mut ZPM<'a>,
    pt: StringPoint<'a>,
    instruction_name: &str,
    token_creator: F,
    error_kind: Error
)
    -> ZPR<'a, Token<'a>>
    where F: FnOnce(Label<'a>) -> Token<'a>
{
    let (pt, _) = try_parse!(pt.consume_literal(instruction_name).map_err(|_| error_kind));
    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};
    let (pt, lab) = try_parse!{parse_label_value(pm, pt)};

    Progress::success(pt, token_creator(lab))
}

fn parse_comment<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!(
        pt.consume_literal("COMMENT")
            .map_err(|_| Error::ExpectedComment)
    );

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, id) = try_parse!{parse_comment_id(pm, pt)};

    Progress::success(pt, Token::Comment(id))
}

fn parse_comment_definition<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!(
        pt.consume_literal("DEFINE COMMENT")
            .map_err(|_| Error::ExpectedCommentDefinition)
    );

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, id) = try_parse!{parse_comment_id(pm, pt)};

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, data) = try_parse!{parse_comment_data(pm, pt)};

    let (pt, _) = try_parse!{
        pt.consume_literal(";")
            .map_err(|_| Error::ExpectedCommentDefinitionEnd)
    };

    Progress::success(pt, Token::CommentDefinition(id, data))
}

fn parse_comment_id<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
    string_point_consume_while(pt, |c| c.is_digit(10))
        .map_err(|_| Error::ExpectedCommentId)
}

fn parse_comment_data<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
    string_point_consume_while(pt, |c| c != ';')
        .map_err(|_| Error::ExpectedCommentDefinitionData)
}

fn parse_whitespace<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    string_point_consume_while(pt, char::is_whitespace)
        .map(Token::Whitespace)
        .map_err(|_| Error::ExpectedWhiteSpace)
}

// Duplicated logic - check and pull to peresil?
fn string_point_consume_while<'a, F>(pt: StringPoint<'a>, predicate: F) -> Progress<StringPoint<'a>, &str, ()>
    where F: Fn(char) -> bool
{
    let end = match pt.s.char_indices().skip_while(|&(_, c)| predicate(c)).next() {
        Some((pos, _)) if pos == 0 => None,
        Some((pos, _)) => Some(pos),
        None => Some(pt.s.len()),
    };

    pt.consume_to(end)
}

impl<'a> Iterator for Thing<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let pt = self.point;

        if pt.s.is_empty() { return None }

        let mut pm = ParseMaster::new();

        let tmp = pm.alternate()
            .one(|pm| parse_header(pm, pt))
            .one(|pm| parse_inbox(pm, pt))
            .one(|pm| parse_outbox(pm, pt))
            .one(|pm| parse_copy_from(pm, pt))
            .one(|pm| parse_copy_to(pm, pt))
            .one(|pm| parse_bump_up(pm, pt))
            .one(|pm| parse_bump_down(pm, pt))
            .one(|pm| parse_add(pm, pt))
            .one(|pm| parse_sub(pm, pt))
            .one(|pm| parse_label_definition(pm, pt))
            .one(|pm| parse_jump(pm, pt)) // check ordering of jump (prefix-based?)
            .one(|pm| parse_jump_if_zero(pm, pt))
            .one(|pm| parse_jump_if_negative(pm, pt))
            .one(|pm| parse_comment(pm, pt))
            .one(|pm| parse_comment_definition(pm, pt))
            .one(|pm| parse_whitespace(pm, pt))
            .finish();

        match pm.finish(tmp) {
            Progress { status: Status::Success(tok), point } => {
                self.point = point;
                Some(tok)
            },
            Progress { status: Status::Failure(x), point } => {
                println!("Actually an error: {:?}, {:?}", x, point);
                None
            }
        }
    }
}

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
struct Program(Vec<Instruction>);

use std::iter::FromIterator;
use std::collections::BTreeMap;

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

#[derive(Debug, Copy, Clone)]
enum Tile {
    Number(i8), // what is the actual size here?
    Letter(char),
}

#[derive(Debug, Clone)]
enum StepError {
    EndOfProgram,
    Other(String),
}

impl StepError {
    fn e(s: &str) -> StepError {
        StepError::Other(s.into())
    }
}

type Input = Vec<Tile>;
type Output = Vec<Tile>;
type Registers = BTreeMap<u8, Tile>;

struct Machine {
    program: Program,
    input: Input,
    output: Output,
    pc: usize,
    accumulator: Option<Tile>,
    registers: Registers,
}

impl Machine {
    fn new(program: Program, mut input: Input, registers: Registers) -> Machine {
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

    fn step(&mut self) -> Result<(), StepError> {
        use Instruction::*;

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

    fn run(&mut self) -> Result<(), StepError> {
        loop {
            match self.step() {
                Ok(..) => continue,
                Err(StepError::EndOfProgram) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }
}

// two zero-term words; output first in alpha order
fn level_36() -> (Input, Registers) {
    let mut input = Vec::new();
    input.extend("aab".chars().map(Tile::Letter));
    input.push(Tile::Number(0));
    input.extend("aaa".chars().map(Tile::Letter));
    input.push(Tile::Number(0));

    let mut registers = BTreeMap::new();
    registers.insert(23, Tile::Number(0));
    registers.insert(24, Tile::Number(10));

    (input, registers)
}

fn main() {
    let mut f = File::open("36-alphabetizer-both.txt").expect("File?");

    let mut s = String::new();
    f.read_to_string(&mut s).expect("read");

    let t = Thing::new(&s);

    let p: Program = t.collect();

    let (input, registers) = level_36();
    let mut m = Machine::new(p, input, registers);

    match m.run() {
        Ok(..) => {
            println!("Program completed");
            println!("Output:");
            println!("{:?}", m.output);
        },
        Err(e) => {
            println!("Program failed");
            println!("{:?}", e);
        }
    }
}
