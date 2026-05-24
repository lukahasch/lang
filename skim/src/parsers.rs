use crate::parser::{Checkpoint, Context, Delimiter, ParseError, Parsed, Parser};

pub fn sequence<Iter, Input, Token, Error, LexerError, Span, Delim, State>(
    tokens: Iter,
) -> impl Parser<Input, Token, (), Error, LexerError, Span, Delim, State>
where
    Iter: IntoIterator<Item = Token>,
    Token: Clone + PartialEq,
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
    Error::Expected: From<Token>,
{
    let tokens: Vec<Token> = tokens.into_iter().collect();
    move |ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>| {
        let mut accumulated: Option<Error> = None;

        for expected_token in &tokens {
            let merge = |a: Error, b: Error| Error::merge(a, b);
            let with_acc = |acc: Option<Error>, e: Error| match acc {
                Some(a) => merge(a, e),
                None => e,
            };

            match ctx.next::<Error>() {
                Parsed::Ok(Some(t)) if &t == expected_token => {}
                Parsed::Ok(Some(t)) => {
                    let err = Error::expected_found(
                        expected_token.clone().into(),
                        t,
                        ctx.span(),
                        &ctx.state,
                    );
                    return Parsed::Err(with_acc(accumulated, err));
                }
                Parsed::Ok(None) => {
                    let err = Error::expected_found_eof(
                        expected_token.clone().into(),
                        ctx.span(),
                        &ctx.state,
                    );
                    return Parsed::Err(with_acc(accumulated, err));
                }
                Parsed::Err(e) => return Parsed::Err(with_acc(accumulated, e)),
                Parsed::Fatal(e) => return Parsed::Fatal(with_acc(accumulated, e)),
                Parsed::Recover(e, Some(t)) if &t == expected_token => {
                    accumulated = Some(with_acc(accumulated, e));
                }
                Parsed::Recover(e, Some(t)) => {
                    let err = Error::expected_found(
                        expected_token.clone().into(),
                        t,
                        ctx.span(),
                        &ctx.state,
                    );
                    return Parsed::Fatal(with_acc(accumulated, merge(e, err)));
                }
                Parsed::Recover(e, None) => {
                    let err = Error::expected_found_eof(
                        expected_token.clone().into(),
                        ctx.span(),
                        &ctx.state,
                    );
                    return Parsed::Fatal(with_acc(accumulated, merge(e, err)));
                }
            }
        }

        match accumulated {
            Some(err) => Parsed::Recover(err, ()),
            None => Parsed::Ok(()),
        }
    }
}

pub fn select<Input, Token, Output, Error, LexerError, Span, Delim, State>(
    expected: Error::Expected,
    func: impl Fn(Token) -> Option<Output>,
) -> impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>
where
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
    Error::Expected: Clone,
    Token: Clone,
{
    move |ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>| {
        let token = ctx.next::<Error>();
        let eof_error = || Error::expected_found_eof(expected.clone(), ctx.span(), &ctx.state);
        let found_other_error =
            |other| Error::expected_found(expected.clone(), other, ctx.span(), &ctx.state);
        match token.map(|opt| opt.map(|t| (t.clone(), func(t)))) {
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Ok(Some((_, Some(value)))) => Parsed::Ok(value),
            Parsed::Ok(Some((t, None))) => Parsed::Err(found_other_error(t)),
            Parsed::Ok(None) => Parsed::Err(eof_error()),
            Parsed::Recover(err, Some((_, Some(value)))) => Parsed::Recover(err, value),
            Parsed::Recover(err, Some((t, None))) => {
                Parsed::Fatal(Error::merge(found_other_error(t), err))
            }
            Parsed::Recover(err, None) => Parsed::Fatal(Error::merge(eof_error(), err)),
        }
    }
}

pub fn select_ctx<Input, Token, Output, Error, LexerError, Span, Delim, State>(
    expected: Error::Expected,
    func: impl Fn(Token, &mut Context<Input, Token, LexerError, Span, Delim, State>) -> Option<Output>,
) -> impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>
where
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
    Error::Expected: Clone,
    Token: Clone,
{
    move |ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>| {
        let token = ctx
            .next::<Error>()
            .map(|opt| opt.map(|t| (t.clone(), func(t, ctx))));
        let eof_error = || Error::expected_found_eof(expected.clone(), ctx.span(), &ctx.state);
        let found_other_error =
            |other| Error::expected_found(expected.clone(), other, ctx.span(), &ctx.state);
        match token {
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Ok(Some((_, Some(value)))) => Parsed::Ok(value),
            Parsed::Ok(Some((t, None))) => Parsed::Err(found_other_error(t)),
            Parsed::Ok(None) => Parsed::Err(eof_error()),
            Parsed::Recover(err, Some((_, Some(value)))) => Parsed::Recover(err, value),
            Parsed::Recover(err, Some((t, None))) => {
                Parsed::Fatal(Error::merge(found_other_error(t), err))
            }
            Parsed::Recover(err, None) => Parsed::Fatal(Error::merge(eof_error(), err)),
        }
    }
}

#[inline(always)]
pub fn just<Input, Token, Error, LexerError, Span, Delim, State>(
    token: Token,
) -> impl Parser<Input, Token, Token, Error, LexerError, Span, Delim, State>
where
    Token: Clone + PartialEq,
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
    Error::Expected: From<Token>,
{
    #[inline(always)]
    move |ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>| match ctx.next::<Error>()
    {
        Parsed::Ok(Some(t)) if t == token => Parsed::Ok(t),
        Parsed::Ok(Some(t)) => Parsed::Err(Error::expected_found(
            token.clone().into(),
            t,
            ctx.span(),
            &ctx.state,
        )),
        Parsed::Ok(None) => Parsed::Err(Error::expected_found_eof(
            token.clone().into(),
            ctx.span(),
            &ctx.state,
        )),
        Parsed::Err(e) => Parsed::Err(e),
        Parsed::Fatal(e) => Parsed::Fatal(e),
        Parsed::Recover(ee, Some(t)) if t == token => Parsed::Recover(ee, t),
        Parsed::Recover(ee, Some(t)) => Parsed::Err(Error::merge(
            Error::expected_found(token.clone().into(), t, ctx.span(), &ctx.state),
            ee,
        )),
        Parsed::Recover(ee, None) => Parsed::Fatal(Error::merge(
            Error::expected_found_eof(token.clone().into(), ctx.span(), &ctx.state),
            ee,
        )),
    }
}

pub fn eof<Input, Token, Error, LexerError, Span, Delim, State>(
    ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>,
) -> Parsed<(), Error>
where
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
{
    match ctx.next() {
        Parsed::Err(err) => return Parsed::Err(err),
        Parsed::Fatal(err) => return Parsed::Fatal(err),
        Parsed::Recover(err, _) => return Parsed::Fatal(err),
        Parsed::Ok(Some(token)) => {
            return Parsed::Err(Error::expected_eof_found(token, ctx.span(), &ctx.state));
        }
        Parsed::Ok(None) => {}
    };
    match ctx.drain_delimiter_errors::<Error>() {
        Some(err) => Parsed::Err(err),
        None => Parsed::Ok(()),
    }
}

pub fn open_delimiter<Input, Token, Error, LexerError, Span, Delim, State>(
    delim: Delim,
) -> impl Parser<Input, Token, Delim, Error, LexerError, Span, Delim, State>
where
    Token: PartialEq + Clone,
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter + Clone + 'static,
    Token: From<Delim>,
    Error::Expected: From<Delim> + From<Token>,
    <Error as ParseError<Token, LexerError, Span, Delim, State>>::Expected: Clone + 'static,
{
    let expected: Error::Expected = delim.clone().into();
    just(delim.clone().into())
        .map_err(move |e: Error| e.expected(expected.clone()))
        .map_ctx(move |_, ctx| {
            ctx.push_delimiter(delim.clone());
            delim.clone()
        })
}

pub fn close_delimiter<Input, Token, Error, LexerError, Span, Delim, State>(
    delim: Delim,
) -> impl Parser<Input, Token, Delim, Error, LexerError, Span, Delim, State>
where
    Token: PartialEq + Clone,
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter + Clone + TryFrom<Token>,
    Token: From<Delim>,
    Error::Expected: From<Delim> + From<Token> + Clone,
{
    select(delim.clone().into(), move |t| {
        if t == delim.clone().into() {
            Some(delim.clone())
        } else {
            Delim::try_from(t).ok()
        }
    })
    .try_map_ctx(
        move |delim, ctx| match ctx.pop_delimiter::<Error>(delim.clone()) {
            Ok(_) => Parsed::Ok(delim.clone()),
            Err(e) => Parsed::Err(e),
        },
    )
}
