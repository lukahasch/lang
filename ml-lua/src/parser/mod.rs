use std::sync::Arc;

use skim::Context;

use crate::parser::lexer::{Lexer, Span, Token};

pub mod lexer;

pub type Ctx<'a> = Context<Lexer<'a>, Token<'a>, (), Span, (), ()>;

pub fn context(source: Arc<str>, content: &'_ str) -> Ctx<'_> {
    Context::new(
        Lexer::new(source.clone(), content),
        Span {
            source: source.clone(),
            range: 0..0,
        },
        (),
    )
}

pub fn parse(source: Arc<str>, content: &str) {
    let lexer = Lexer::new(source.clone(), content);
    let _ctx: Ctx = Context::new(
        lexer,
        Span {
            source: source.clone(),
            range: 0..0,
        },
        (),
    );
}

#[cfg(tests)]
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
            syntax![Token::Let ! <ident> (<ident> {identifier} | <ident> {just(Token::Let).map(|_| "")}) Token::Equal <expr> {identifier}]
                .parse(&mut test_ctx("let x = test"));
        let mut results = parsed.unwrap_ok();
        assert_eq!(results.take::<"ident", _>().take::<"ident", _>(), "x");
        assert_eq!(results.take::<"expr", _>(), "test");
    }
}
