//! It was NOT useful to implement it myself and is certainly much slower than something like `logos` but i like it hehe

mod str_iter;
use str_iter::*;
mod npeek;
pub use npeek::*;

use crate::Config;

use std::ops::ControlFlow;

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenLocation {
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, kinded::Kinded)]
#[kinded(kind = TokenKind)]
pub enum TokenVariant<'a> {
    DummyText(&'a str),
    IdentifierReplace(&'a str),

    If,
    IfDef,
    IfNDef,
    Elif,
    ElifDef,
    ElifNDef,
    Else,
    Endif,

    DoubleAmpersand,
    DoublePipe,
    Gt,
    Gte,
    Lt,
    Lte,

    Define,

    ParenOpen,
    ParenClose,

    Dash,
    Star,
    Slash,
    Plus,

    Bang,

    DoubleEqual,
    BangEqual,

    True,
    False,

    IntLiteral(i64),
    FloatLiteral(f64),

    Identifier(&'a str),

    EndOfStatement,

    Error(char),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token<'a> {
    pub variant: TokenVariant<'a>,
    pub text: &'a str,
    pub location: TokenLocation,
}

impl<'a> Token<'a> {
    pub fn kind(&self) -> TokenKind {
        self.variant.kind()
    }
}

#[derive(Debug, Clone)]
pub struct Tokenizer<'a, 'c> {
    chars: StrCharIter<'a>,
    pub cfg: &'c Config,

    is_in_statement: bool,
}

impl<'a, 'c> Tokenizer<'a, 'c> {
    pub fn new(cfg: &'c Config, text: &'a str) -> Self {
        Self {
            chars: StrCharIter::new(text),
            cfg,

            is_in_statement: false,
        }
    }

    pub fn location(&self) -> TokenLocation {
        TokenLocation {
            line: self.chars.peek_line(),
            column: self.chars.peek_column(),
            index: self.chars.peek_idx(),
        }
    }
    
    fn skip_whitespaces(&mut self) {
        debug_assert!(self.is_in_statement, "Should only keep whitespaces in statements");
        self.chars.consume_while(|c| c != '\n' && c.is_whitespace());
    }

    fn take_identifier(&mut self) -> &'a str {
        self.chars.consume_while(is_identifier_char)
    }

    fn take_number(&mut self) -> &'a str {
        // 5. -> take 5.
        // .5 -> take .5
        // .a -> take nothing
        self.chars.consumed_slice(|chars| {
            let first_part = chars.consume_while(|c| c.is_ascii_digit());
            if chars.peek(0) != Some('.') { return }
            if !first_part.is_empty() || chars.peek(1).is_some_and(|c| c.is_ascii_digit()) {
                chars.next(); // consume the .
                chars.consume_while(|c| c.is_ascii_digit());
            }
        })
    }

    fn statement_next(&mut self) -> TokenVariant<'a> {
        assert!(self.is_in_statement);

        if self.chars.consume_eq("\n") {
            self.is_in_statement = false;
            return TokenVariant::EndOfStatement;
        }

        const SIMPLE_TOKENS: [(&str, TokenVariant); 15] = [
            ("&&", TokenVariant::DoubleAmpersand),
            ("||", TokenVariant::DoublePipe),
            ("<=", TokenVariant::Lte),
            ("<", TokenVariant::Lt),
            (">=", TokenVariant::Gte),
            (">", TokenVariant::Gt),
            ("!=", TokenVariant::BangEqual),
            ("==", TokenVariant::DoubleEqual),
            ("(", TokenVariant::ParenOpen),
            (")", TokenVariant::ParenClose),
            ("-", TokenVariant::Dash),
            ("*", TokenVariant::Star),
            ("/", TokenVariant::Slash),
            ("+", TokenVariant::Plus),
            ("!", TokenVariant::Bang),
        ];

        for (str, token) in SIMPLE_TOKENS {
            if self.chars.consume_eq(str) {
                return token;
            }
        }

        // number token
        if let num = self.take_number() && !num.is_empty() {
            if num.contains('.') {
                let Ok(value) = num.parse::<f64>()
                else { panic!("Invalid number somewhere '{num}'") };
                return TokenVariant::FloatLiteral(value);
            }
            else {
                let Ok(value) = num.parse::<i64>()
                else { panic!("Invalid number somewhere '{num}'") };
                return TokenVariant::IntLiteral(value);
            }
        }

        match self.take_identifier() {
            ""        => TokenVariant::Error(self.chars.next().expect("Not empty at this point")),
            "true"    => TokenVariant::True,
            "false"   => TokenVariant::False,
            "if"      => TokenVariant::If,
            "ifdef"   => TokenVariant::IfDef,
            "ifndef"   => TokenVariant::IfNDef,
            "elif"    => TokenVariant::Elif,
            "elifdef" => TokenVariant::ElifDef,
            "elifndef" => TokenVariant::ElifNDef,
            "endif"   => TokenVariant::Endif,
            "else"    => TokenVariant::Else,
            "define"  => TokenVariant::Define,
            ident => TokenVariant::Identifier(ident),
        }
    }

    fn no_statement_next(&mut self) -> ControlFlow<TokenVariant<'a>> {
        assert!(!self.is_in_statement);

        if self.chars.consume_eq(&self.cfg.line_statement_prefix) {
            self.is_in_statement = true;
            return ControlFlow::Continue(());
        }

        if self.chars.consume_eq(&self.cfg.replace_identifier_prefix) {
            let ident = self.take_identifier();
            if ident.is_empty() {
                return ControlFlow::Break(TokenVariant::Error(self.chars.next().expect("Not empty at this point")));
            }
            return ControlFlow::Break(TokenVariant::IdentifierReplace(ident));
        }

        let dummy_text = self.chars.consumed_slice(|chars| {
            while !chars.peek_eq(&self.cfg.line_statement_prefix) &&
                  !chars.peek_eq(&self.cfg.replace_identifier_prefix) &&
                  !chars.peek(0).is_none()
            {
                chars.next();
            }
        });

        debug_assert!(!dummy_text.is_empty());

        ControlFlow::Break(TokenVariant::DummyText(dummy_text))
    }
}

impl<'a, 'c> Iterator for Tokenizer<'a, 'c> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut location;
        let variant = loop {
            if self.is_in_statement {
                self.skip_whitespaces();
            }

            location = self.location();

            if self.chars.peek(0).is_none() {
                // emit a EndOfStatement for EOF too
                if self.is_in_statement {
                    self.is_in_statement = false;
                    break Some(TokenVariant::EndOfStatement);
                }
                else {
                    break None;
                }

            }

            if self.is_in_statement {
                break Some(self.statement_next());
            }
            else {
                match self.no_statement_next() {
                    ControlFlow::Continue(()) => continue,
                    ControlFlow::Break(token) => break Some(token),
                }
            }
        }?;

        Some(Token {
            variant,
            text: &self.chars.str()[location.index..self.chars.peek_idx()],
            location,
        })
    }
}

pub type Lexer<'a, 'c> = NPeek<Tokenizer<'a, 'c>>;
