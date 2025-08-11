use std::str::CharIndices;

use utils::itertools::Itertools;

#[derive(Debug, Clone)]
pub struct StrCharIter<'a> {
    str: &'a str,
    indices: CharIndices<'a>,
    line: usize,
    column: usize,
}

impl<'a> StrCharIter<'a> {
    pub fn new(str: &'a str) -> Self {
        Self {
            str,
            indices: str.char_indices(),
            line: 1,
            column: 0,
        }
    }

    pub fn str(&self) -> &'a str {
        self.str
    }

    pub fn peek_idx(&self) -> usize {
        self.indices.clone().nth(0)
            .map(|(idx, _)| idx)
            .unwrap_or(self.str.len())
    }

    pub fn peek_line(&self) -> usize {
        self.line
    }

    pub fn peek_column(&self) -> usize {
        self.column
    }

    pub fn peek(&self, n: usize) -> Option<char> {
        self.indices.clone().nth(n).map(|(_, char)| char)
    }

    pub fn rest(&self) -> &'a str {
        &self.str[self.peek_idx()..]
    }

    pub fn next_if(&mut self, pred: impl FnOnce(char) -> bool) -> Option<char> {
        if pred(self.peek(0)?) {
            self.next()
        }
        else {
            None
        }
    }

    pub fn consumed_slice(&mut self, fun: impl FnOnce(&mut Self)) -> &'a str {
        let start = self.peek_idx();
        fun(self);
        let end = self.peek_idx();
        &self.str[start..end]
    }

    pub fn consume_while(&mut self, mut pred: impl FnMut(char) -> bool) -> &'a str {
        self.consumed_slice(|this| {
            while this.next_if(&mut pred).is_some() { }
        })
    }

    pub fn peek_eq(&self, matching: &str) -> bool {
        self.rest().starts_with(matching)
    }

    pub fn consume_eq(&mut self, matching: &str) -> bool {
        if self.peek_eq(matching) {
            self.dropping(matching.chars().count());
            true
        }
        else {
            false
        }
    }
}

impl<'a> Iterator for StrCharIter<'a> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        let char = self.indices.next().map(|(_, char)| char)?;
        if char == '\n' {
            self.line += 1;
            self.column = 0;
        }
        else {
            self.column += 1;
        }
        Some(char)
    }
}
