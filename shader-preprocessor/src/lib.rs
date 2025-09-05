#![feature(iterator_try_reduce)]

// let me do like in C++ mouhahaha
#![allow(irrefutable_let_patterns)]

#![forbid(unsafe_code)]

use std::collections::HashMap;

mod parser;
pub use parser::{ ParserError };
mod lexer;
mod eval;
pub use eval::{ Value, ValueType, EvalError };

#[derive(Debug, Clone)]
pub struct Config {
    pub line_statement_prefix: String,
    pub replace_identifier_prefix: String,
}

impl Config {
    
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_statement_prefix: "//!".to_string(),
            replace_identifier_prefix: "#".to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<'a> {
    #[error(transparent)]
    ParserError(ParserError<'a>),
    #[error(transparent)]
    EvalError(#[from] EvalError),
}

pub fn preprocess<'a, 'b>(cfg: &'a Config, defines: impl Into<HashMap<String, Value>>, shader_src: &'b str) -> Result<String, Error<'b>> {
    let ast = parser::parse(&mut lexer::NPeek::new(lexer::Tokenizer::new(cfg, shader_src))).map_err(Error::ParserError)?;
    let mut ctx = eval::EvalCtx {
        defines: defines.into(),
        ..Default::default()
    };
    Ok(eval::eval_statements(&mut ctx, &ast)?)
}
