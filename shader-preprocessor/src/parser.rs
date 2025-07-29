use crate::lexer::*;

// 
// UTILS
// 

fn peek_if<'a, 'r>(lexer: &'r mut Lexer<'a, '_>, cond: impl FnOnce(&Token<'a>) -> bool) -> Result<Option<&'r Token<'a>>, ParserError<'a>> {
    Ok(match lexer.peek(0) {
        Some(t) if cond(&t) => Some(t),
        _ => None,
    })
}

macro_rules! peek_matches {
    ($lexer: expr, $pat: pat) => {
        peek_if($lexer, |_new_token_| matches!(_new_token_, $pat))
    };
}

#[expect(dead_code)]
fn peek_eq<'a, 'b>(lexer: &mut Lexer<'a, '_>, expected: &Token<'b>) -> Result<bool, ParserError<'a>> {
    Ok(peek_if(lexer, |t| t == expected)?.is_some())
}

fn expect_if<'a>(lexer: &mut Lexer<'a, '_>, cond: impl FnOnce(&Token<'a>) -> bool) -> Result<Token<'a>, ParserError<'a>> {
    match lexer.next() {
        Some(t) if cond(&t) => Ok(t),
        Some(t) => Err(ParserError::UnexpectedToken(t)),
        None => Err(ParserError::UnexpectedEof),
    }
}

macro_rules! expect_matches {
    ($lexer: expr, $pat: pat) => {
        expect_if($lexer, |_new_token_| matches!(_new_token_, $pat))
    };
}

fn expect_eq<'a, 'b>(lexer: &mut Lexer<'a, '_>, token: &Token<'b>) -> Result<Token<'a>, ParserError<'a>> {
    expect_if(lexer, |t| t == token)
}

fn next_if_then<'a, 'b, F, O>(lexer: &mut Lexer<'a, '_>, pred: F) -> Result<Option<O>, ParserError<'a>>
    where F: FnOnce(&Token<'a>) -> Option<O>,
{
    if let Some(output) = lexer.peek(0).and_then(pred) {
        Ok(Some(output))
    }
    else {
        Ok(None)
    }
}

fn next_if<'a>(lexer: &mut Lexer<'a, '_>, pred: impl FnOnce(&Token<'a>) -> bool) -> Result<Option<Token<'a>>, ParserError<'a>> {
    Ok(lexer.next_if(pred))
}

macro_rules! next_if_matches {
    ($lexer: expr, $pat: pat) => {
        next_if($lexer, |_new_token_| matches!(_new_token_, $pat))
    };
}

fn next_if_eq<'a, 'b>(lexer: &mut Lexer<'a, '_>, token: &Token<'b>) -> Result<bool, ParserError<'a>> {
    Ok(next_if(lexer, |t| t == token)?.is_some())
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
    fn peek_is_first(t: &Token) -> bool {
        matches!(t, Token::Define)
    }
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expression,
    pub then: Statements,
    pub r#else: Option<Statements>,
}

impl IfStatement {
    fn peek_is_first(t: &Token) -> bool {
        matches!(t, Token::If | Token::IfDef)
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    DummyText(String),
    ReplaceWithVariableValue(String),
    Define(DefineStatement),
    If(IfStatement),
}

impl Statement {
    fn peek_is_first(t: &Token) -> bool {
        matches!(t, Token::DummyText(_) | Token::IdentifierReplace(_)) ||
        DefineStatement::peek_is_first(t) ||
        IfStatement::peek_is_first(t)
    }
}

#[derive(Debug, Clone)]
pub struct Statements(pub Vec<Statement>);

#[derive(Debug, thiserror::Error)]
pub enum ParserError<'a> {
    #[error("Unexpected token `{0:?}`")]
    UnexpectedToken(Token<'a>),
    #[error("Unexpected eof")]
    UnexpectedEof,
}

impl<'a> ParserError<'a> {
    fn unexpected_next(lexer: &mut Lexer<'a, '_>) -> Self {
        match lexer.next() {
            Some(t) => Self::UnexpectedToken(t),
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

fn util_parse_binop<'a>(
    lexer: &mut Lexer<'a, '_>,
    prev: fn(&mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>>,
    mut matcher: impl FnMut(&Token<'a>) -> Option<BinOp>,
) -> Result<Expression, ParserError<'a>> {
    let mut expr = prev(lexer)?;
    while let Some(op) = next_if_then(lexer, &mut matcher)? {
        let rhs = Box::new(prev(lexer)?);
        expr = Expression::BinOp(BinOpExpression { lhs: Box::new(expr), op, rhs });
    }
    Ok(expr)
}

// fn peek_is_expression_num(token: &Token) -> bool {
//     matches!(token, Token::ParenOpen | Token::IntLiteral(_) | Token::FloatLiteral(_) | Token::Identifier(_) | Token::True | Token::False)
// }
fn parse_expression_num<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    if next_if_eq(lexer, &Token::ParenOpen)? {
        let expr = parse_expression(lexer)?;
        expect_eq(lexer, &Token::ParenClose)?;
        return Ok(expr);
    }

    if next_if_eq(lexer, &Token::True)? {
        return Ok(Expression::BoolLiteral(true));
    }
    if next_if_eq(lexer, &Token::False)? {
        return Ok(Expression::BoolLiteral(false));
    }

    if let Some(Token::Identifier(ident)) = next_if_matches!(lexer, Token::Identifier(_))? {
        return Ok(Expression::Variable(ident.to_string()));
    }

    if let Some(Token::IntLiteral(val)) = next_if_matches!(lexer, Token::IntLiteral(_))? {
        return Ok(Expression::IntLiteral(val));
    }

    if let Some(Token::FloatLiteral(val)) = next_if_matches!(lexer, Token::FloatLiteral(_))? {
        return Ok(Expression::FloatLiteral(val));
    }

    Err(ParserError::unexpected_next(lexer))
}

// fn peek_is_expression_unop(token: &Token) -> bool {
//     matches!(token, Token::Dash | Token::Plus) || peek_is_expression_num(token)
// }
fn parse_expression_unop<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    if let Some(op) = next_if_then(lexer, |t| Some(match t {
        Token::Dash => UnOp::Neg,
        Token::Plus => UnOp::Plus,
        Token::Bang => UnOp::Not,
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
    util_parse_binop(lexer, parse_expression_unop, |token| Some(match token {
        Token::Star => BinOp::Mul,
        Token::Slash => BinOp::Div,
        _ => return None,
    }))
}

// fn peek_is_expression_add(token: &Token) -> bool {
//     peek_is_expression_mul(token)
// }
fn parse_expression_add<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_mul, |token| Some(match token {
        Token::Plus => BinOp::Add,
        Token::Dash => BinOp::Sub,
        _ => return None,
    }))
}

// fn peek_is_expression_comp(token: &Token) -> bool {
//     peek_is_expression_add(token)
// }
fn parse_expression_comp<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_add, |token| Some(match token {
        Token::Lt => BinOp::Lt,
        Token::Lte => BinOp::Lte,
        Token::Gt => BinOp::Gt,
        Token::Gte => BinOp::Gte,
        _ => return None,
    }))
}

// fn peek_is_expression_eq_comp(token: &Token) -> bool {
//     peek_is_expression_comp(token)
// }
fn parse_expression_eq_comp<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_comp, |token| Some(match token {
        Token::DoubleEqual => BinOp::Eq,
        Token::BangEqual => BinOp::Neq,
        _ => return None,
    }))
}

// fn peek_is_expression_bool_and(token: &Token) -> bool {
//     peek_is_expression_eq_comp(token)
// }
fn parse_expression_bool_and<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_eq_comp, |token| Some(match token {
        Token::DoubleAmpersand => BinOp::BooleanAnd,
        _ => return None,
    }))
}

// fn peek_is_expression_bool_or(token: &Token) -> bool {
//     peek_is_expression_eq_comp(token)
// }
fn parse_expression_bool_or<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    util_parse_binop(lexer, parse_expression_bool_and, |token| Some(match token {
        Token::DoublePipe => BinOp::BooleanOr,
        _ => return None,
    }))
}

// fn peek_is_expression(token: &Token) -> bool {
//     peek_is_expression_bool_or(token)
// }
fn parse_expression<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Expression, ParserError<'a>> {
    parse_expression_bool_or(lexer)
}

fn parse_define_statement<'a>(lexer: &mut Lexer<'a, '_>) -> Result<DefineStatement, ParserError<'a>> {
    expect_eq(lexer, &Token::Define)?;
    let Token::Identifier(variable_name) = expect_matches!(lexer, Token::Identifier(_))?
    else { unreachable!() };
    let value = parse_expression(lexer)?;
    expect_eq(lexer, &Token::EndOfStatement)?;

    Ok(DefineStatement {
        variable_name: variable_name.into(),
        value,
    })
}

fn parse_if_statement<'a>(lexer: &mut Lexer<'a, '_>) -> Result<IfStatement, ParserError<'a>> {
    let if_type = expect_matches!(lexer, Token::If | Token::Elif | Token::IfDef | Token::ElifDef)?;
    let is_isdef = matches!(if_type, Token::IfDef | Token::ElifDef);
    let condition = if is_isdef {
        let Token::Identifier(variable_name) = expect_matches!(lexer, Token::Identifier(_))?
        else { unreachable!() };
        Expression::IsDef(variable_name.to_string())
    }
    else {
        parse_expression(lexer)?
    };
    expect_eq(lexer, &Token::EndOfStatement)?;
    let then = parse_statements(lexer)?;

    let r#else = if peek_matches!(lexer, Token::Elif | Token::ElifDef)?.is_some() {
        Some(parse_if_statement(lexer).map(|istmt| Statements(vec![Statement::If(istmt)]))?)
    }
    else if next_if_eq(lexer, &Token::Else)? {
        expect_eq(lexer, &Token::EndOfStatement)?;
        let r#else = parse_statements(lexer)?;
        expect_eq(lexer, &Token::Endif)?;
        expect_eq(lexer, &Token::EndOfStatement)?;
        Some(r#else)
    }
    else {
        expect_eq(lexer, &Token::Endif)?;
        expect_eq(lexer, &Token::EndOfStatement)?;
        None
    };

    Ok(IfStatement {
        condition,
        then,
        r#else,
    })
}

fn parse_statement<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Statement, ParserError<'a>> {
    match lexer.peek(0) {
        Some(t) if DefineStatement::peek_is_first(t) =>
            parse_define_statement(lexer).map(Statement::Define),
        Some(t) if IfStatement::peek_is_first(t) =>
            parse_if_statement(lexer).map(Statement::If),
        Some(Token::DummyText(_)) => {
            let Some(Token::DummyText(text)) = lexer.next()
            else { unreachable!() };
            Ok(Statement::DummyText(text.to_string()))
        }
        Some(Token::IdentifierReplace(_)) => {
            let Some(Token::IdentifierReplace(variable_name)) = lexer.next()
            else { unreachable!() };
            Ok(Statement::ReplaceWithVariableValue(variable_name.to_string()))
        }

        _ => Err(ParserError::unexpected_next(lexer))
    }
}

fn parse_statements<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Statements, ParserError<'a>> {
    let mut statements = Vec::<Statement>::new();
    while lexer.peek(0).is_some_and(Statement::peek_is_first) {
        let nstmt = parse_statement(lexer)?;
        push_statement(&mut statements, nstmt);
    }
    Ok(Statements(statements))
}

pub fn parse<'a>(lexer: &mut Lexer<'a, '_>) -> Result<Statements, ParserError<'a>> {
    let statements = parse_statements(lexer)?;
    if let Some(token) = lexer.next() {
        Err(ParserError::UnexpectedToken(token))
    }
    else {
        Ok(statements)
    }
}
