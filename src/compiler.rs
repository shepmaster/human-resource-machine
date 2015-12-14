use std::collections::BTreeMap;

use super::parser::Token;
use super::machine::Instruction;

#[derive(Debug, Copy, Clone)]
pub enum Error<E> {
    ParserError(E),
    UndefinedLabel
}

impl<E> From<E> for Error<E> {
    fn from(e: E) -> Error<E> {
        Error::ParserError(e)
    }
}

#[derive(Debug, Clone)]
pub struct Program(Vec<Instruction>);

impl Program {
    pub fn compile<'a, I, E>(iterator: I) -> Result<Program, Error<E>>
        where I: IntoIterator<Item = Result<Token<'a>, E>>
    {
        // Find any parsing failures
        let tokens: Vec<_> = try!(iterator.into_iter().collect());

        // Remove values that don't change the behavior
        let without_junk: Vec<_> = tokens.into_iter().filter(|t| match *t {
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

        let unmap = |id| label_mapping.get(id).map(|&x| x).ok_or(Error::UndefinedLabel);

        // Make the instructions, resolving jump locations
        let i = without_junk.into_iter().map(|t| {
            let instr = match t {
                Token::Inbox => Instruction::Inbox,
                Token::Outbox => Instruction::Outbox,
                Token::CopyFrom(r) => Instruction::CopyFrom(r),
                Token::CopyTo(r) => Instruction::CopyTo(r),
                Token::BumpUp(r) => Instruction::BumpUp(r),
                Token::BumpDown(r) => Instruction::BumpDown(r),
                Token::Add(r) => Instruction::Add(r),
                Token::Sub(r) => Instruction::Sub(r),
                Token::LabelDefinition(..) => Instruction::NoOp,
                Token::Jump(id) => Instruction::Jump(try!(unmap(id))),
                Token::JumpIfZero(id) => Instruction::JumpIfZero(try!(unmap(id))),
                Token::JumpIfNegative(id) => Instruction::JumpIfNegative(try!(unmap(id))),
                _ => unreachable!(),
            };
            Ok(instr)
        });

        let instrs = try!(i.collect::<Result<_, Error<E>>>());
        Ok(Program(instrs))
    }

    pub fn stats_len(&self) -> usize {
        self.0.iter().filter(|i| i.counts_towards_stats()).count()
    }
}

impl IntoIterator for Program {
    type Item = Instruction;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
