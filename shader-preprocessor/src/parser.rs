use crate::lexer::*;

// 
// UTILS
// 

#[expect(dead_code)]
fn peek_if<'a, 'r>(lexer: &'r mut Lexer<'a, '_>, cond: impl FnOnce(TokenKind) -> bool) -> Result<Option<&'r Token<'a>>, ParserError<'a>> {
    Ok(match lexer.peek(0) {
        Some(t) if cond(t.kind()) => Some(t),
        _ => None,
    })
}

fn peek_one_of<'a, 'r>(lexer: &'r mut Lexer<'a, '_>, expected: &[TokenKind]) -> Result<Option<&'r Token<'a>>, ParserError<'a>> {
    Ok(match lexer.peek(0) {
        Some(t) if expected.contains(&t.kind()) => Some(t),
        _ => None,
    })
}

#[expect(dead_code)]
fn peek_eq<'a, 'b>(lexer: &mut Lexer<'a, '_>, expected: TokenKind) -> Result<bool, ParserError<'a>> {
    Ok(peek_one_of(lexer, &[expected])?.is_some())
}

#[expect(dead_code)]
fn expect_if<'a>(lexer: &mut Lexer<'a, '_>, cond: impl FnOnce(&Token<'a>) -> bool) -> Result<Token<'a>, ParserError<'a>> {
    match lexer.next() {
        Some(t) if cond(&t) => Ok(t),
        Some(t) => Err(ParserError::UnexpectedToken {
            got: t,
            expected: vec![],
        }),
        None => Err(ParserError::UnexpectedEof),
    }
}

fn expect_one_of<'a>(lexer: &mut Lexer<'a, '_>, expected: &[TokenKind]) -> Result<Token<'a>, ParserError<'a>> {
    match lexer.next() {
        Some(t) if expected.contains(&t.kind()) => Ok(t),
        Some(t) => Err(ParserError::UnexpectedToken {
            got: t,
            expected: expected.to_vec(),
        }),
        None => Err(ParserError::UnexpectedEof),
    }
}

fn expect_eq<'a, 'b>(lexer: &mut Lexer<'a, '_>, expected: TokenKind) -> Result<Token<'a>, ParserError<'a>> {
    expect_one_of(lexer, &[expected])
}

fn next_if_then<'a, 'b, F, O>(lexer: &mut Lexer<'a, '_>, pred: F) -> Result<Option<O>, ParserError<'a>>
    where F: FnOnce(&Token<'a>) -> Option<O>,
{
    Ok(lexer.peek(0).and_then(pred))
}

#[expect(dead_code)]
fn next_if<'a>(lexer: &mut Lexer<'a, '_>, pred: impl FnOnce(&Token<'a>) -> bool) -> Result<Option<Token<'a>>, ParserError<'a>> {
    Ok(lexer.next_if(pred))
}

fn next_if_one_of<'a>(lexer: &mut Lexer<'a, '_>, one_of: &[TokenKind]) -> Result<Option<(usize, Token<'a>)>, ParserError<'a>> {
    let Some(nt) = lexer.peek(0)
    else { return Ok(None); };
    let Some(idx) = one_of.iter().position(|p| p == &nt.kind())
    else { return Ok(None); };
    Ok(Some((idx, lexer.next().expect("Peeked"))))
}

fn next_if_eq<'a>(lexer: &mut Lexer<'a, '_>, eq: TokenKind) -> Result<bool, ParserError<'a>> {
    next_if_one_of(lexer, &[eq]).map(|p| p.is_some())
}

//
//
// 

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum BinOp {
    BooleanAnd,
    BooleanOr,

    Eq,
    Neq,

    Gt,
    Gte,
    Lt,
    Lte,

    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UnOp {
    Plus,
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct BinOpExpression {
    pub lhs: Box<Expression>,
    pub op: BinOp,
    pub rhs: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct UnOpExpression {
    pub op: UnOp,
    pub operand: Box<Expression>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Variable(String),
    IsDef(String),
    BinOp(BinOpExpression),
    UnOp(UnOpExpression),
    BoolLiteral(bool),
    IntLiteral(i64),
    FloatLiteral(f64),
}

#[derive(Debug, Clone)]
pub struct DefineStatement {
    pub variable_name: String,
    pub value: Expression,
}

impl DefineStatement {
    pub const FIRST_TOKENS: &[TokenKind] = &[TokenKind::Define];
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expression,
    pub then: Statements,
    pub r#else: Option<Statements>,
}

impl IfStatement {
    pub const FIRST_TOKENS: &[TokenKind] = &[TokenKind::If, TokenKind::IfDef];
}

#[derive(Debug, Clone)]
pub enum Statement {
    DummyText(String),
    ReplaceWithVariableValue(String),
    Define(DefineStatement),
    If(IfStatement),
}

impl Statement {
    pub const FIRST_TOKENS: &[TokenKind] = constcat::concat_slices!([TokenKind]:
        &[TokenKind::DummyText,
          TokenKind::IdentifierReplace],
        DefineStatement::FIRST_TOKENS,
        IfStatement::FIRST_TOKENS,
    );
}

#[derive(Debug, Clone)]
pub struct Statements(pub Vec<Statement>);

#[derive(Debug, thiserror::Error)]
pub enum ParserError<'a> {
    #[error("Unexpected token, got `{got:?}`, expected one of: {expected:?}")]
    UnexpectedToken {
        got: Token<'a>,
        expected: Vec<TokenKind>,
    },
    #[error("Unexpected eof")]
    UnexpectedEof,
}

impl<'a> ParserError<'a> {
    fn unexpected_next(lexer: &mut Lexer<'a, '_>) -> Self {
        match lexer.next() {
            Some(t) => Self::UnexpectedToken {
                got: t,
                expected: todo!(),
            },
            None => Self::UnexpectedEof,
        }
    }
}

fn push_statement(statements: &mut Vec<Statement>, nstmt: Statement) {
    if let (Some(Statement::DummyText(prev)), Statement::DummyText(new)) = (statements.last_mut(), &nstmt) {
        debug_assert!(!new.is_empty());
        prev.push_str(&new);
    }
    else {
        statements.push(nstmt);
    }
}

fn util_parse_binop<'a, const N: usize>(
    lexer: &mut Lexer<'a, '_>,
    prev: fn(&mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>>,
    ops: [(TokenKind, BinOp); N],
) -> Result<Expression, ParserError<'a>> {
    let mut expr = prev(lexer)?;
    while let Some((idx, _)) = next_if_one_of(lexer, &ops.map(|(kind, _)| kind))? {
        let op = ops[idx].1;
        let rhs = Box::new(prev(lexer)?);
        expr = Expression::BinOp(BinOpExpression { lhs: Box::new(expr), op, rhs });
    }
    Ok(expr)
}

// fn peek_is_expression_num(token: &Token) -> bool {
//     matches!(token, Token::ParenOpen | Token::IntLiteral(_) | Token::FloatLiteral(_) | Token::Identifier(_) | Token::True | Token::False)
// }
fn parse_expression_num<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    if next_if_eq(lexer, TokenKind::ParenOpen)? {
        let expr = parse_expression(lexer)?;
        expect_eq(lexer, TokenKind::ParenClose)?;
        return Ok(expr);
    }

    if next_if_eq(lexer, TokenKind::True)? {
        return Ok(Expression::BoolLiteral(true));
    }
    if next_if_eq(lexer, TokenKind::False)? {
        return Ok(Expression::BoolLiteral(false));
    }

    if let Some((_, Token{ variant: TokenVariant::Identifier(ident), .. })) = next_if_one_of(lexer, &[TokenKind::Identifier])? {
        return Ok(Expression::Variable(ident.to_string()));
    }

    if let Some((_, Token { variant: TokenVariant::IntLiteral(val), .. })) = next_if_one_of(lexer, &[TokenKind::IntLiteral])? {
        return Ok(Expression::IntLiteral(val));
    }

    if let Some((_, Token { variant: TokenVariant::FloatLiteral(val), .. })) = next_if_one_of(lexer, &[TokenKind::FloatLiteral])? {
        return Ok(Expression::FloatLiteral(val));
    }

    Err(ParserError::unexpected_next(lexer))
}

// fn peek_is_expression_unop(token: &Token) -> bool {
//     matches!(token, Token::Dash | Token::Plus) || peek_is_expression_num(token)
// }
fn parse_expression_unop<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    if let Some(op) = next_if_then(lexer, |t| Some(match t.kind() {
        TokenKind::Dash => UnOp::Neg,
        TokenKind::Plus => UnOp::Plus,
        TokenKind::Bang => UnOp::Not,
        _ => return None,
    }))? {
        let operand = Box::new(parse_expression_num(lexer)?);
        return Ok(Expression::UnOp(UnOpExpression {
            op,
            operand,
        }));
    }

    parse_expression_num(lexer)
}

// fn peek_is_expression_mul(token: &Token) -> bool {
//     peek_is_expression_unop(token)
// }
fn parse_expression_mul<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_unop, [
        (TokenKind::Star, BinOp::Mul),
        (TokenKind::Slash, BinOp::Div),
    ])
}

// fn peek_is_expression_add(token: &Token) -> bool {
//     peek_is_expression_mul(token)
// }
fn parse_expression_add<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_mul, [
        (TokenKind::Plus, BinOp::Add),
        (TokenKind::Dash, BinOp::Sub),
    ])
}

// fn peek_is_expression_comp(token: &Token) -> bool {
//     peek_is_expression_add(token)
// }
fn parse_expression_comp<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_add, [
        (TokenKind::Lt, BinOp::Lt),
        (TokenKind::Lte, BinOp::Lte),
        (TokenKind::Gt, BinOp::Gt),
        (TokenKind::Gte, BinOp::Gte),
    ])
}

// fn peek_is_expression_eq_comp(token: &Token) -> bool {
//     peek_is_expression_comp(token)
// }
fn parse_expression_eq_comp<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_comp, [
        (TokenKind::DoubleEqual, BinOp::Eq),
        (TokenKind::BangEqual, BinOp::Neq),
    ])
}

// fn peek_is_expression_bool_and(token: &Token) -> bool {
//     peek_is_expression_eq_comp(token)
// }
fn parse_expression_bool_and<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_eq_comp, [
        (TokenKind::DoubleAmpersand, BinOp::BooleanAnd),
    ])
}

// fn peek_is_expression_bool_or(token: &Token) -> bool {
//     peek_is_expression_eq_comp(token)
// }
fn parse_expression_bool_or<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_bool_and, [
        (TokenKind::DoublePipe, BinOp::BooleanOr),
    ])
}

// fn peek_is_expression(token: &Token) -> bool {
//     peek_is_expression_bool_or(token)
// }
fn parse_expression<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    parse_expression_bool_or(lexer)
}

fn parse_define_statement<'a>(lexer: &mut Lexer<'a, '_>) -> Result<DefineStatement, ParserError<'a>> {
    expect_eq(lexer, TokenKind::Define)?;
    let Token { variant: TokenVariant::Identifier(variable_name), .. } = expect_eq(lexer, TokenKind::Identifier)?
    else { unreachable!() };
    let value = parse_expression(lexer)?;
    expect_eq(lexer, TokenKind::EndOfStatement)?;

    Ok(DefineStatement {
        variable_name: variable_name.into(),
        value,
    })
}

fn parse_if_statement<'a>(lexer: &mut Lexer<'a, '_>) -> Result<IfStatement, ParserError<'a>> {
    let if_type = expect_one_of(lexer, &[TokenKind::If, TokenKind::Elif, TokenKind::IfDef, TokenKind::ElifDef])?;
    let is_isdef = matches!(if_type.kind(), TokenKind::IfDef | TokenKind::ElifDef);
    let condition = if is_isdef {
        let Token { variant: TokenVariant::Identifier(variable_name), .. } = expect_eq(lexer, TokenKind::Identifier)?
        else { unreachable!() };
        Expression::IsDef(variable_name.to_string())
    }
    else {
        parse_expression(lexer)?
    };
    expect_eq(lexer, TokenKind::EndOfStatement)?;
    let then = parse_statements(lexer)?;

    let r#else = if peek_one_of(lexer, &[TokenKind::Elif, TokenKind::ElifDef])?.is_some() {
        Some(parse_if_statement(lexer).map(|istmt| Statements(vec![Statement::If(istmt)]))?)
    }
    else if next_if_eq(lexer, TokenKind::Else)? {
        expect_eq(lexer, TokenKind::EndOfStatement)?;
        let r#else = parse_statements(lexer)?;
        expect_eq(lexer, TokenKind::Endif)?;
        expect_eq(lexer, TokenKind::EndOfStatement)?;
        Some(r#else)
    }
    else {
        expect_eq(lexer, TokenKind::Endif)?;
        expect_eq(lexer, TokenKind::EndOfStatement)?;
        None
    };

    Ok(IfStatement {
        condition,
        then,
        r#else,
    })
}

fn parse_statement<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Statement, ParserError<'a>> {
    match lexer.peek(0).map(|t| t.kind()) {
        Some(t) if DefineStatement::FIRST_TOKENS.contains(&t) =>
            parse_define_statement(lexer).map(Statement::Define),
        Some(t) if IfStatement::FIRST_TOKENS.contains(&t) =>
            parse_if_statement(lexer).map(Statement::If),
        Some(TokenKind::DummyText) => {
            let Some(Token { variant: TokenVariant::DummyText(text), .. }) = lexer.next()
            else { unreachable!() };
            Ok(Statement::DummyText(text.to_string()))
        }
        Some(TokenKind::IdentifierReplace) => {
            let Some(Token { variant: TokenVariant::IdentifierReplace(variable_name), .. }) = lexer.next()
            else { unreachable!() };
            Ok(Statement::ReplaceWithVariableValue(variable_name.to_string()))
        }

        _ => Err(ParserError::unexpected_next(lexer))
    }
}

fn parse_statements<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Statements, ParserError<'a>> {
    let mut statements = Vec::<Statement>::new();
    while lexer.peek(0).is_some_and(|t| Statement::FIRST_TOKENS.contains(&t.kind())) {
        let nstmt = parse_statement(lexer)?;
        push_statement(&mut statements, nstmt);
    }
    Ok(Statements(statements))
}

pub fn parse<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Statements, ParserError<'a>> {
    let statements = parse_statements(lexer)?;
    if let Some(token) = lexer.next() {
        Err(ParserError::UnexpectedToken {
            got: token,
            expected: vec![],
        })
    }
    else {
        Ok(statements)
    }
}
