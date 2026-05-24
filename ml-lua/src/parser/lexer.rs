use std::{ops::Range, sync::Arc};

use derive_more::Display;
use logos::Logos;

#[derive(Debug, Clone, Copy, Display, Logos, PartialEq, Eq)]
#[logos(skip(" |\t|\n"))]
pub enum Token<'a> {
    #[regex("[a-zA-Z_][a-zA-Z_0-9]*")]
    #[display("{_0}")]
    Identifier(&'a str),
    #[token("let")]
    #[display("let")]
    Let,
    #[token("=")]
    #[display("=")]
    Equal,
}

impl<'a> From<Token<'a>> for String {
    fn from(value: Token<'a>) -> Self {
        format!("{value}")
    }
}

#[derive(Clone)]
pub struct Lexer<'a> {
    lexer: logos::SpannedIter<'a, Token<'a>>,
    source: Arc<str>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: Arc<str>, content: &'a str) -> Self {
        Self {
            lexer: logos::Lexer::new(content).spanned(),
            source,
        }
    }
}

#[derive(Clone, Display, PartialEq)]
#[display("{source}:{}..{}", range.start, range.end)]
pub struct Span {
    pub source: Arc<str>,
    pub range: Range<usize>,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = (Result<Token<'a>, ()>, Span);

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.next().map(|(v, range)| {
            (
                v,
                Span {
                    source: self.source.clone(),
                    range,
                },
            )
        })
    }
}
