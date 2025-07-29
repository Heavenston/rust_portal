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

#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    DummyText(&'a str),

    IdentifierReplace(&'a str),

    If,
    IfDef,
    Elif,
    ElifDef,
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

    fn statement_next(&mut self) -> Token<'a> {
        assert!(self.is_in_statement);

        if self.chars.consume_eq("\n") {
            self.is_in_statement = false;
            return Token::EndOfStatement;
        }

        const SIMPLE_TOKENS: [(&str, Token); 15] = [
            ("&&", Token::DoubleAmpersand),
            ("||", Token::DoublePipe),
            ("<=", Token::Lte),
            ("<", Token::Lt),
            (">=", Token::Gte),
            (">", Token::Gt),
            ("!=", Token::BangEqual),
            ("==", Token::DoubleEqual),
            ("(", Token::ParenOpen),
            (")", Token::ParenClose),
            ("-", Token::Dash),
            ("*", Token::Star),
            ("/", Token::Slash),
            ("+", Token::Plus),
            ("!", Token::Bang),
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
                return Token::FloatLiteral(value);
            }
            else {
                let Ok(value) = num.parse::<i64>()
                else { panic!("Invalid number somewhere '{num}'") };
                return Token::IntLiteral(value);
            }
        }

        match self.take_identifier() {
            ""        => Token::Error(self.chars.next().expect("Not empty at this point")),
            "true"    => Token::True,
            "false"   => Token::False,
            "if"      => Token::If,
            "ifdef"   => Token::IfDef,
            "elif"    => Token::Elif,
            "elifdef" => Token::ElifDef,
            "endif"   => Token::Endif,
            "else"    => Token::Else,
            "define"  => Token::Define,
            ident => Token::Identifier(ident),
        }
    }

    fn no_statement_next(&mut self) -> ControlFlow<Token<'a>> {
        assert!(!self.is_in_statement);

        if self.chars.consume_eq(&self.cfg.line_statement_prefix) {
            self.is_in_statement = true;
            return ControlFlow::Continue(());
        }

        if self.chars.consume_eq(&self.cfg.replace_identifier_prefix) {
            let ident = self.take_identifier();
            if ident.is_empty() {
                return ControlFlow::Break(Token::Error(self.chars.next().expect("Not empty at this point")));
            }
            return ControlFlow::Break(Token::IdentifierReplace(ident));
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

        ControlFlow::Break(Token::DummyText(dummy_text))
    }
}

impl<'a, 'c> Iterator for Tokenizer<'a, 'c> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.is_in_statement {
                self.skip_whitespaces();
            }

            if self.chars.peek(0).is_none() {
                // emit a EndOfStatement for EOF too
                if self.is_in_statement {
                    self.is_in_statement = false;
                    break Some(Token::EndOfStatement);
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
                    ControlFlow::Continue(()) => (),
                    ControlFlow::Break(token) => break Some(token),
                }
            }
        }
    }
}

pub type Lexer<'a, 'c> = NPeek<Tokenizer<'a, 'c>>;
