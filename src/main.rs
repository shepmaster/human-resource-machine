#[macro_use]
extern crate peresil;

use std::fs::File;
use std::io::prelude::*;

use peresil::{ParseMaster, StringPoint, Progress, Status, Recoverable};

#[derive(Debug, Copy, Clone)]
enum Error {
    Lazy,
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
    s: &'a str,
    point: StringPoint<'a>,
}

impl<'a> Thing<'a> {
    fn new(s: &str) -> Thing {
        Thing {
            s: s,
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
    // negative?
    Comment(CommentId<'a>),
    CommentDefinition(CommentId<'a>, CommentData<'a>),
    Whitespace(&'a str),
}

enum State {
    Initial
}

fn parse_header<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    pt.consume_literal("-- HUMAN RESOURCE MACHINE PROGRAM --")
        .map(|_| Token::Header)
        .map_err(|_| Error::ExpectedHeader)
}

fn parse_inbox<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    pt.consume_literal("INBOX")
        .map(|_| Token::Inbox)
        .map_err(|_| Error::ExpectedInbox)
}

fn parse_outbox<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    pt.consume_literal("OUTBOX")
        .map(|_| Token::Outbox)
        .map_err(|_| Error::ExpectedOutbox)
}

fn parse_copy_from<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!{
        pt.consume_literal("COPYFROM")
            .map_err(|_| Error::ExpectedCopyFrom)
    };

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, reg) = try_parse!{parse_register(pm, pt)};

    Progress::success(pt, Token::CopyFrom(reg))
}

fn parse_copy_to<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!{
        pt.consume_literal("COPYTO")
            .map_err(|_| Error::ExpectedCopyTo)
    };

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, reg) = try_parse!{parse_register(pm, pt)};

    Progress::success(pt, Token::CopyTo(reg))
}

fn parse_bump_up<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!{
        pt.consume_literal("BUMPUP")
            .map_err(|_| Error::ExpectedBumpUp)
    };

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, reg) = try_parse!{parse_register(pm, pt)};

    Progress::success(pt, Token::BumpUp(reg))
}

// Single register stuff is very repeated
fn parse_bump_down<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!{
        pt.consume_literal("BUMPDOWN")
            .map_err(|_| Error::ExpectedBumpDown)
    };

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, reg) = try_parse!{parse_register(pm, pt)};

    Progress::success(pt, Token::BumpDown(reg))
}

fn parse_add<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!{
        pt.consume_literal("ADD")
            .map_err(|_| Error::ExpectedAdd)
    };

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, reg) = try_parse!{parse_register(pm, pt)};

    Progress::success(pt, Token::Add(reg))
}

fn parse_sub<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!{
        pt.consume_literal("SUB")
            .map_err(|_| Error::ExpectedSub)
    };

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, reg) = try_parse!{parse_register(pm, pt)};

    Progress::success(pt, Token::Sub(reg))
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

fn parse_register_value<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, u8> {
    let end = match pt.s.char_indices().skip_while(|&(i, c)| c.is_digit(10)).next() {
        Some((pos, _)) if pos == 0 => None,
        Some((pos, _)) => Some(pos),
        None => Some(pt.s.len()),
    };
    pt.consume_to(end)
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

fn parse_label_value<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
    let end = match pt.s.char_indices().skip_while(|&(i, c)| c >= 'a' && c <= 'z').next() {
        Some((pos, _)) if pos == 0 => None,
        Some((pos, _)) => Some(pos),
        None => Some(pt.s.len()),
    };

    pt.consume_to(end)
        .map_err(|_| Error::ExpectedLabelValue)
}

fn parse_jump<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!(
        pt.consume_literal("JUMP")
            .map_err(|_| Error::ExpectedJump)
    );

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, lab) = try_parse!{parse_label_value(pm, pt)};

    Progress::success(pt, Token::Jump(lab))
}

fn parse_jump_if_zero<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!(
        pt.consume_literal("JUMPZ")
            .map_err(|_| Error::ExpectedJumpIfZero)
    );

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, lab) = try_parse!{parse_label_value(pm, pt)};

    Progress::success(pt, Token::JumpIfZero(lab))
}

fn parse_jump_if_negative<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!(
        pt.consume_literal("JUMPN")
            .map_err(|_| Error::ExpectedJumpIfNegative)
    );

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, lab) = try_parse!{parse_label_value(pm, pt)};

    Progress::success(pt, Token::JumpIfNegative(lab))
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

fn parse_comment_id<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
// Duplicated logic - check and pull to peresil?
    let end = match pt.s.char_indices().skip_while(|&(i, c)| c.is_digit(10)).next() {
        Some((pos, _)) if pos == 0 => None,
        Some((pos, _)) => Some(pos),
        None => Some(pt.s.len()),
    };
    pt.consume_to(end)
        .map_err(|_| Error::ExpectedCommentId)
}

fn parse_comment_data<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
// Duplicated logic - check and pull to peresil?
    let end = match pt.s.char_indices().skip_while(|&(i, c)| c != ';').next() {
        Some((pos, _)) if pos == 0 => None,
        Some((pos, _)) => Some(pos),
        None => Some(pt.s.len()),
    };
    pt.consume_to(end)
        .map_err(|_| Error::ExpectedCommentDefinitionData)
}


fn parse_whitespace<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    // Duplicated logic - check and pull to peresil?
    let end = match pt.s.char_indices().skip_while(|&(i, c)| c.is_whitespace()).next() {
        Some((pos, _)) if pos == 0 => None,
        Some((pos, _)) => Some(pos),
        None => Some(pt.s.len()),
    };
    pt.consume_to(end)
        .map(Token::Whitespace)
        .map_err(|_| Error::ExpectedWhiteSpace)
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

fn main() {
    let mut f = File::open("36-alphabetizer-both.txt").expect("File?");

    let mut s = String::new();
    f.read_to_string(&mut s).expect("read");

    let mut t = Thing::new(&s);

    for t in t {
        if let Token::Whitespace(..) = t { continue }
        println!("{:?}", t);
    }
}
