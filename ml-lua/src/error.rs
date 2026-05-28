use derive_more::Display;
use miette::Diagnostic;
use skim::{Green, ParseError, Red};
use thiserror::Error;

use crate::parser::{
    Delim, State,
    lexer::{Span, Token},
};

#[derive(Debug, Clone, PartialEq, Diagnostic, Error)]
pub enum Diag<'a> {
    #[error("forbidden character")]
    #[diagnostic(severity(Error))]
    ForbiddenCharacter(#[label] Span),
    #[error("Expected {expected} found {found}")]
    #[diagnostic(severity(Error))]
    ExpectedFound {
        expected: Green<Expected<'a>>,
        found: Red<Found<'a>>,
        #[label]
        span: Span,
    },
    #[error("Tried to close unopened Delimiter with ''")]
    #[diagnostic(severity(Error))]
    TriedToCloseUnopenedDelimiter { closer: Red<Delim>, span: Span },
    #[error("t")]
    TriedToCloseMismatchingDelimiter {
        matching_opener: Delim,
        matching_span: Span,
        current_opener: Delim,
        current_span: Span,
        closer: Delim,
        span: Span,
    },
    #[error("Did not close Delimiter {opener}")]
    #[diagnostic(severity(Error))]
    DidNotCloseDelimiter {
        opener: Red<Delim>,
        #[label("opened here")]
        span: Span,
    },

    #[error("Parsing has failed")]
    ParsingFailed(#[related] Vec<Diag<'a>>),
}

#[derive(Debug, Clone, PartialEq, Display)]
pub enum Expected<'a> {
    #[display("'{_0}'")]
    Token(Token<'a>),
    #[display("a variable")]
    Variable,
    #[display("end of file")]
    Eof,
}

impl<'a> Expected<'a> {
    fn niceness(&self) -> i32 {
        match self {
            Self::Variable => 1,
            _ => 0,
        }
    }

    fn choose(self, other: Self) -> Self {
        if self.niceness() > other.niceness() {
            self
        } else {
            other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Display)]
pub enum Found<'a> {
    #[display("'{_0}'")]
    Token(Token<'a>),
    #[display("end of file")]
    Eof,
}

impl<'a> ParseError<Token<'a>, (), Span, Delim, State> for Diag<'a> {
    type Expected = Expected<'a>;

    fn lexer_error(_: (), span: Span, _: &State) -> Self {
        Diag::ForbiddenCharacter(span)
    }
    fn expected_found(expected: Self::Expected, found: Token<'a>, span: Span, _: &State) -> Self {
        Diag::ExpectedFound {
            expected: Green(expected),
            found: Red(Found::Token(found)),
            span,
        }
    }
    fn expected_found_eof(expected: Self::Expected, span: Span, _: &State) -> Self {
        Diag::ExpectedFound {
            expected: Green(expected),
            found: Red(Found::Eof),
            span,
        }
    }
    fn expected_eof_found(found: Token<'a>, span: Span, _: &State) -> Self {
        Diag::ExpectedFound {
            expected: Green(Expected::Eof),
            found: Red(Found::Token(found)),
            span,
        }
    }
    fn tried_to_close_unopened_delimiter(closer: Delim, span: Span, _: &State) -> Self {
        Diag::TriedToCloseUnopenedDelimiter {
            closer: Red(closer),
            span,
        }
    }
    fn tried_to_close_mismatching_delimiter(
        (matching_span, matching_opener): (Span, Delim),
        (current_span, current_opener): (Span, Delim),
        (span, closer): (Span, Delim),
        _: &State,
    ) -> Self {
        Diag::TriedToCloseMismatchingDelimiter {
            matching_opener,
            matching_span,
            current_opener,
            current_span,
            closer,
            span,
        }
    }
    fn did_not_close_delimiter(opener: Delim, span: Span, _: Span, _: &State) -> Self {
        Diag::DidNotCloseDelimiter {
            opener: Red(opener),
            span,
        }
    }

    fn expected(self, e2: Self::Expected) -> Self {
        match self {
            Diag::ExpectedFound {
                expected: Green(expected),
                found,
                span,
            } => Diag::ExpectedFound {
                expected: Green(expected.choose(e2)),
                found,
                span,
            },
            diag => diag,
        }
    }
    fn merge(a: Self, b: Self) -> Self {
        match (a, b) {
            (Diag::ParsingFailed(mut a), Diag::ParsingFailed(b)) => Diag::ParsingFailed({
                a.extend(b);
                a
            }),
            (Diag::ParsingFailed(mut a), b) => Diag::ParsingFailed({
                a.push(b);
                a
            }),
            (b, Diag::ParsingFailed(mut a)) => Diag::ParsingFailed({
                a.push(b);
                a
            }),
            (a, b) => Diag::ParsingFailed(vec![a, b]),
        }
    }
}
