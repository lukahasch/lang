use std::{ops::Range, sync::Arc};

use derive_more::Display;
use logos::Logos;
use miette::SourceSpan;
use skim::Merge;

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

#[derive(Clone, Debug, Display, PartialEq)]
#[display("{source}:{}..{}", range.start, range.end)]
pub struct Span {
    pub source: Arc<str>,
    pub range: Range<usize>,
}

impl Merge for Span {
    fn merge(a: Self, b: Self) -> Self {
        assert_eq!(a.source, b.source);
        Span {
            source: a.source,
            range: a.range.start.min(b.range.start)..a.range.end.max(b.range.end),
        }
    }
}

impl From<Span> for SourceSpan {
    fn from(value: Span) -> Self {
        value.range.into()
    }
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
