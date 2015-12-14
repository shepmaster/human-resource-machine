use std::iter::FromIterator;

use std::collections::BTreeMap;

use super::parser::Token;
use super::machine::{Instruction};

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

impl IntoIterator for Program {
    type Item = Instruction;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
