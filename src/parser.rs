use peresil::{ParseMaster, StringPoint, Progress, Status, Recoverable};

#[derive(Debug, Copy, Clone)]
pub enum Error {
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
    ExpectedRegisterLabelDefinition,
    ExpectedRegisterLabelId,
    ExpectedRegisterLabelDefinitionData,
    ExpectedRegisterLabelDefinitionEnd,
    ExpectedColon,
}

impl Recoverable for Error {
    fn recoverable(&self) -> bool { true }
}

type ZPM<'a> = ParseMaster<StringPoint<'a>, Error>;
type ZPR<'a, T> = Progress<StringPoint<'a>, T, Error>;

#[derive(Debug, Copy, Clone)]
pub struct Parser<'a> {
    point: StringPoint<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(s: &str) -> Parser {
        Parser {
            point: StringPoint::new(s),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Register {
    Direct(u8),
    Indirect(u8),
}

pub type Label<'a> = &'a str;
pub type CommentId<'a> = &'a str;
pub type CommentData<'a> = &'a str;

#[derive(Debug, Copy, Clone)]
pub enum Token<'a> {
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
    parse_single_register_instruction(pm, pt, "BUMPDN", Token::BumpDown, Error::ExpectedBumpDown)
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

fn parse_register_label_definition<'a>(pm: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, Token<'a>> {
    let (pt, _) = try_parse!(
        pt.consume_literal("DEFINE LABEL")
            .map_err(|_| Error::ExpectedRegisterLabelDefinition)
    );

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, id) = try_parse!{parse_register_label_id(pm, pt)};

    let (pt, _) = try_parse!{parse_whitespace(pm, pt)};

    let (pt, data) = try_parse!{parse_register_label_data(pm, pt)};

    let (pt, _) = try_parse!{
        pt.consume_literal(";")
            .map_err(|_| Error::ExpectedRegisterLabelDefinitionEnd)
    };

    Progress::success(pt, Token::CommentDefinition(id, data))
}

fn parse_register_label_id<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
    string_point_consume_while(pt, |c| c.is_digit(10))
        .map_err(|_| Error::ExpectedRegisterLabelId)
}

fn parse_register_label_data<'a>(_: &mut ZPM<'a>, pt: StringPoint<'a>) -> ZPR<'a, &'a str> {
    string_point_consume_while(pt, |c| c != ';')
        .map_err(|_| Error::ExpectedRegisterLabelDefinitionData)
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

impl<'a> Iterator for Parser<'a> {
    type Item = Result<Token<'a>, (usize, Vec<Error>)>;

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
            .one(|pm| parse_register_label_definition(pm, pt))
            .one(|pm| parse_whitespace(pm, pt))
            .finish();

        match pm.finish(tmp) {
            Progress { status: Status::Success(tok), point } => {
                self.point = point;
                Some(Ok(tok))
            },
            Progress { status: Status::Failure(e), point } => {
                Some(Err((point.offset, e)))
            }
        }
    }
}
