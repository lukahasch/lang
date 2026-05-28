use std::sync::Arc;

use derive_more::Display;
use skim::{Checkpoint, Context, Delimiter, Parsed, Parser, Spanned, select};

use crate::{
    Expr, Tagged,
    error::{Diag, Expected},
    parser::lexer::{Lexer, Span, Token},
};

pub mod lexer;

#[derive(Clone)]
pub struct State;

#[derive(PartialEq, Clone, Debug, Display)]
pub enum Delim {}

impl Delimiter for Delim {
    fn opposite(&self) -> Self {
        todo!()
    }
}

impl Checkpoint for State {
    type Save = Self;

    fn checkpoint(&self) -> Self::Save {
        self.clone()
    }

    fn restore(&mut self, save: Self::Save) {
        *self = save;
    }
}

pub type Ctx<'a> = Context<Lexer<'a>, Token<'a>, (), Span, Delim, State>;
pub type E = Expr<Span, String>;

pub fn context(source: Arc<str>, content: &'_ str) -> Ctx<'_> {
    Context::new(
        Lexer::new(source.clone(), content),
        Span {
            source: source.clone(),
            range: 0..0,
        },
        State,
    )
}

pub fn parse(source: Arc<str>, content: &str) -> Parsed<E, Diag<'_>> {
    let lexer = Lexer::new(source.clone(), content);
    let mut ctx: Ctx = Context::new(
        lexer,
        Span {
            source: source.clone(),
            range: 0..0,
        },
        State,
    );
    variable.parse(&mut ctx)
}

fn variable<'a>(ctx: &mut Ctx<'a>) -> Parsed<E, Diag<'a>> {
    select(Expected::Variable, |token| match token {
        Token::Identifier(s) => Some(String::from(s)),
        _ => None,
    })
    .spanned()
    .map(|Spanned(ident, span)| E::Variable(Tagged(ident, span)))
    .parse(ctx)
}

#[cfg(test)]
#[allow(clippy::precedence)]
mod tests {
    use skim::{Parsed, Parser, just, select, sequence, syntax};

    use super::*;

    fn test_ctx(content: &'_ str) -> Ctx<'_> {
        context(Arc::from("test"), content)
    }

    #[test]
    fn basic_functionality() {
        let parsed: Parsed<(), String> = sequence([
            Token::Let,
            Token::Identifier("x"),
            Token::Equal,
            Token::Identifier("x"),
        ])
        .parse(&mut test_ctx("let x = x"));
        dbg!(&parsed);
        assert!(parsed.is_ok());
    }

    #[test]
    fn let_identifier() {
        let identifier = select("identifier".into(), |token| match token {
            Token::Identifier(x) => Some(x),
            _ => None,
        });
        let parsed: Parsed<_, String> =
            syntax![Token::Let ! <ident> (<ident> {identifier.as_ref()} | <ident> {just(Token::Let).map(|_| "")}) Token::Equal <expr> {identifier.as_ref()}]
                .parse(&mut test_ctx("let x = test"));
        let mut results = parsed.unwrap_ok();
        assert_eq!(results.take::<"ident", _>().take::<"ident", _>(), "x");
        assert_eq!(results.take::<"expr", _>(), "test");
    }
}
