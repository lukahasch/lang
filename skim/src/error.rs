use crate::parser::ParseError;
use std::fmt::{Debug, Display};

impl<Token, LexerError, Span, Delim, State> ParseError<Token, LexerError, Span, Delim, State>
    for String
where
    Token: Display,
    LexerError: Debug,
    Span: Display,
    Delim: Debug,
{
    type Expected = String;

    fn lexer_error(lexer_error: LexerError, span: Span, _state: &State) -> Self {
        format!("lexer error at {span}: {lexer_error:?}")
    }

    fn expected_found(expected: String, found: Token, span: Span, _state: &State) -> Self {
        format!("expected {expected}, found '{found}' at {span}")
    }

    fn expected_found_eof(expected: String, span: Span, _state: &State) -> Self {
        format!("expected {expected}, found end of input at {span}")
    }

    fn expected_eof_found(found: Token, span: Span, _state: &State) -> Self {
        format!("expected end of input, found '{found}' at {span}")
    }

    fn tried_to_close_unopened_delimiter(closer: Delim, span: Span, _state: &State) -> Self {
        format!("tried to close '{closer:?}' at {span} but no delimiter is open")
    }

    fn tried_to_close_mismatching_delimiter(
        matching_open: (Span, Delim),
        current_open: (Span, Delim),
        close: (Span, Delim),
        _state: &State,
    ) -> Self {
        format!(
            "'{closer:?}' at {close_span} closes '{matching:?}' from {matching_span}, \
             but '{current:?}' opened at {current_span} is still unclosed",
            closer = close.1,
            close_span = close.0,
            matching = matching_open.1,
            matching_span = matching_open.0,
            current = current_open.1,
            current_span = current_open.0,
        )
    }

    fn did_not_close_delimiter(opener: Delim, span: Span, eof: Span, _state: &State) -> Self {
        format!("'{opener:?}' opened at {span} was never closed (end of input at {eof})")
    }

    fn expected(self, expected: String) -> Self {
        format!("{self} (expected: {expected})")
    }

    fn merge(a: Self, b: Self) -> Self {
        format!("{a}\n{b}")
    }
}
