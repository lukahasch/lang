use owo_colors::OwoColorize;
use std::ops::Range;
use std::{fmt::Display, marker::PhantomData};

pub trait Parser<I, Item, Output, Error, LE, S, Delim, Ctx>
where
    Self: Sized,
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
{
    fn parse(&self, ctx: &mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<Output, Error>;

    #[inline(always)]
    fn chain<O1, O2, A, B, E>(a: A, b: B) -> impl Parser<I, Item, (O1, O2), E, LE, S, Delim, Ctx>
    where
        A: Parser<I, Item, O1, E, LE, S, Delim, Ctx>,
        B: Parser<I, Item, O2, E, LE, S, Delim, Ctx>,
        E: ParseError<Item, LE, S, Delim, Ctx>,
    {
        move |ctx: &mut _| match a.parse(ctx) {
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Ok(ok) => match b.parse(ctx) {
                Parsed::Err(err) => Parsed::Err(err),
                Parsed::Fatal(err) => Parsed::Fatal(err),
                Parsed::Recover(err, value) => Parsed::Recover(err, (ok, value)),
                Parsed::Ok(value) => Parsed::Ok((ok, value)),
            },
            Parsed::Recover(err, ok) => match b.parse(ctx) {
                Parsed::Ok(value) => Parsed::Recover(err, (ok, value)),
                Parsed::Err(err2) | Parsed::Fatal(err2) => Parsed::Fatal(E::merge(err, err2)),
                Parsed::Recover(err2, value) => Parsed::Recover(E::merge(err, err2), (ok, value)),
            },
        }
    }

    #[inline(always)]
    fn map_raw_ctx<T, E>(
        self,
        f: impl Fn(Parsed<Output, Error>, &mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<T, E>,
    ) -> impl Parser<I, Item, T, E, LE, S, Delim, Ctx>
    where
        E: ParseError<Item, LE, S, Delim, Ctx>,
    {
        struct Map<P, F, Output, Error, Item, LE, S, Delim, Ctx>(
            P,
            F,
            PhantomData<(Output, Error, Item, LE, S, Delim, Ctx)>,
        );

        impl<I, Item, Output, Error, LE, S, Delim, Ctx, P, F, T, E>
            Parser<I, Item, T, E, LE, S, Delim, Ctx>
            for Map<P, F, Output, Error, Item, LE, S, Delim, Ctx>
        where
            Self: Sized,
            Ctx: Clone,
            Error: ParseError<Item, LE, S, Delim, Ctx>,
            E: ParseError<Item, LE, S, Delim, Ctx>,
            I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
            S: Clone,
            P: Parser<I, Item, Output, Error, LE, S, Delim, Ctx>,
            F: Fn(Parsed<Output, Error>, &mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<T, E>,
            Delim: Delimiter,
        {
            #[inline(always)]
            fn parse(&self, ctx: &mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<T, E> {
                self.1(self.0.parse(ctx), ctx)
            }

            #[inline(always)]
            fn chain<O1, O2, A, B, EE>(
                a: A,
                b: B,
            ) -> impl Parser<I, Item, (O1, O2), EE, LE, S, Delim, Ctx>
            where
                A: Parser<I, Item, O1, EE, LE, S, Delim, Ctx>,
                B: Parser<I, Item, O2, EE, LE, S, Delim, Ctx>,
                EE: ParseError<Item, LE, S, Delim, Ctx>,
            {
                P::chain(a, b)
            }
        }

        Map(self, f, PhantomData)
    }

    fn inspect<T>(
        self,
        f: impl Fn(&Parsed<Output, Error>) -> T,
    ) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx> {
        self.map_raw_ctx(move |o, _| {
            f(&o);
            o
        })
    }

    fn map<O>(self, f: impl Fn(Output) -> O) -> impl Parser<I, Item, O, Error, LE, S, Delim, Ctx> {
        self.map_raw_ctx(move |o, _| match o {
            Parsed::Ok(ok) => Parsed::Ok(f(ok)),
            Parsed::Recover(err, ok) => Parsed::Recover(err, f(ok)),
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
        })
    }

    fn map_err<E>(
        self,
        f: impl Fn(Error) -> E,
    ) -> impl Parser<I, Item, Output, E, LE, S, Delim, Ctx>
    where
        E: ParseError<Item, LE, S, Delim, Ctx>,
    {
        self.map_raw_ctx(move |o, _| match o {
            Parsed::Ok(ok) => Parsed::Ok(ok),
            Parsed::Recover(err, ok) => Parsed::Recover(f(err), ok),
            Parsed::Err(err) => Parsed::Err(f(err)),
            Parsed::Fatal(err) => Parsed::Fatal(f(err)),
        })
    }

    fn expect(
        self,
        expect: Error::Expected,
    ) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx>
    where
        Error::Expected: Clone + 'static,
    {
        self.map_raw_ctx(move |err, _| match err {
            Parsed::Ok(ok) => Parsed::Ok(ok),
            Parsed::Err(err) => Parsed::Err(err.expected(expect.clone())),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Recover(err, ok) => Parsed::Recover(err, ok),
        })
    }

    fn map_ctx<O>(
        self,
        f: impl Fn(Output, &mut Context<I, Item, LE, S, Delim, Ctx>) -> O,
    ) -> impl Parser<I, Item, O, Error, LE, S, Delim, Ctx> {
        self.map_raw_ctx(move |o, ctx| match o {
            Parsed::Ok(ok) => Parsed::Ok(f(ok, ctx)),
            Parsed::Recover(err, ok) => Parsed::Recover(err, f(ok, ctx)),
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
        })
    }

    fn try_map_ctx<O>(
        self,
        f: impl Fn(Output, &mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<O, Error>,
    ) -> impl Parser<I, Item, O, Error, LE, S, Delim, Ctx> {
        self.map_raw_ctx(move |o, ctx| match o {
            Parsed::Ok(ok) => f(ok, ctx),
            Parsed::Recover(err, ok) => match f(ok, ctx) {
                Parsed::Err(err2) => Parsed::Fatal(Error::merge(err, err2)),
                Parsed::Fatal(err2) => Parsed::Fatal(Error::merge(err, err2)),
                Parsed::Ok(ok) => Parsed::Ok(ok),
                Parsed::Recover(err2, _) => Parsed::Fatal(Error::merge(err, err2)),
            },
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
        })
    }

    #[inline(always)]
    fn and<O>(
        self,
        p: impl Parser<I, Item, O, Error, LE, S, Delim, Ctx>,
    ) -> impl Parser<I, Item, (Output, O), Error, LE, S, Delim, Ctx> {
        Self::chain(self, p)
    }

    #[inline(always)]
    fn and_ignore<O>(
        self,
        p: impl Parser<I, Item, O, Error, LE, S, Delim, Ctx>,
    ) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx> {
        self.and(p).map(|(value, _)| value)
    }

    #[inline(always)]
    fn ignore_and<O>(
        self,
        p: impl Parser<I, Item, O, Error, LE, S, Delim, Ctx>,
    ) -> impl Parser<I, Item, O, Error, LE, S, Delim, Ctx> {
        self.and(p).map(|(_, value)| value)
    }

    fn cut(self) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx> {
        struct Cut<P>(P);

        impl<I, Item, Output, Error, LE, S, Delim, Ctx, P>
            Parser<I, Item, Output, Error, LE, S, Delim, Ctx> for Cut<P>
        where
            Self: Sized,
            Ctx: Clone,
            Error: ParseError<Item, LE, S, Delim, Ctx>,
            I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
            S: Clone,
            P: Parser<I, Item, Output, Error, LE, S, Delim, Ctx>,
            Delim: Delimiter,
        {
            #[inline(always)]
            fn parse(
                &self,
                ctx: &mut Context<I, Item, LE, S, Delim, Ctx>,
            ) -> Parsed<Output, Error> {
                self.0.parse(ctx)
            }

            #[inline(always)]
            fn chain<O1, O2, A, B, E>(
                a: A,
                b: B,
            ) -> impl Parser<I, Item, (O1, O2), E, LE, S, Delim, Ctx>
            where
                A: Parser<I, Item, O1, E, LE, S, Delim, Ctx>,
                B: Parser<I, Item, O2, E, LE, S, Delim, Ctx>,
                E: ParseError<Item, LE, S, Delim, Ctx>,
            {
                Cut(move |ctx: &mut _| match a.parse(ctx) {
                    Parsed::Err(err) => Parsed::Err(err),
                    Parsed::Fatal(err) => Parsed::Fatal(err),
                    Parsed::Ok(ok) => match b.parse(ctx) {
                        Parsed::Err(err) => Parsed::Fatal(err),
                        Parsed::Fatal(err) => Parsed::Fatal(err),
                        Parsed::Recover(err, value) => Parsed::Recover(err, (ok, value)),
                        Parsed::Ok(value) => Parsed::Ok((ok, value)),
                    },
                    Parsed::Recover(err, ok) => match b.parse(ctx) {
                        Parsed::Ok(value) => Parsed::Recover(err, (ok, value)),
                        Parsed::Err(err2) | Parsed::Fatal(err2) => {
                            Parsed::Fatal(E::merge(err, err2))
                        }
                        Parsed::Recover(err2, value) => {
                            Parsed::Recover(E::merge(err, err2), (ok, value))
                        }
                    },
                })
            }
        }

        Cut(self)
    }

    fn optional(self) -> impl Parser<I, Item, Option<Output>, Error, LE, S, Delim, Ctx> {
        #[inline(always)]
        move |ctx: &mut Context<I, Item, LE, S, Delim, Ctx>| {
            let clone = ctx.breakpoint();
            let parsed = self.parse(ctx);
            match parsed {
                Parsed::Ok(value) => Parsed::Ok(Some(value)),
                Parsed::Fatal(err) => Parsed::Fatal(err),
                Parsed::Recover(err, value) => Parsed::Recover(err, Some(value)),
                Parsed::Err(_) => {
                    *ctx = clone;
                    Parsed::Ok(None)
                }
            }
        }
    }

    fn or(
        self,
        p: impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx>,
    ) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx> {
        #[inline(always)]
        move |ctx: &mut Context<I, Item, LE, S, Delim, Ctx>| {
            let clone = ctx.breakpoint();
            let parsed = self.parse(ctx);
            match parsed {
                Parsed::Ok(value) => Parsed::Ok(value),
                Parsed::Fatal(err) => Parsed::Fatal(err),
                Parsed::Recover(err, value) => Parsed::Recover(err, value),
                Parsed::Err(_) => {
                    *ctx = clone;
                    p.parse(ctx)
                }
            }
        }
    }

    fn as_ref(&self) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx> {
        Borrow(self)
    }

    /// fix
    fn repeated(self) -> impl Parser<I, Item, Vec<Output>, Error, LE, S, Delim, Ctx> {
        #[inline(always)]
        move |ctx: &mut Context<I, Item, LE, S, Delim, Ctx>| {
            let mut state: Result<Vec<Output>, (Error, Vec<Output>)> = Ok(Vec::new());
            loop {
                let clone = ctx.breakpoint();
                state = match (state, self.parse(ctx)) {
                    (Ok(mut vec), Parsed::Ok(next)) => {
                        vec.push(next);
                        Ok(vec)
                    }
                    (Err((err, mut vec)), Parsed::Ok(next)) => {
                        vec.push(next);
                        Err((err, vec))
                    }
                    (Ok(mut vec), Parsed::Recover(err, next)) => {
                        vec.push(next);
                        Err((err, vec))
                    }
                    (Err((err, mut vec)), Parsed::Recover(err2, next)) => {
                        vec.push(next);
                        Err((Error::merge(err, err2), vec))
                    }
                    (_, Parsed::Fatal(err)) => return Parsed::Fatal(err),
                    (Ok(vec), Parsed::Err(_)) if !vec.is_empty() => {
                        *ctx = clone;
                        return Parsed::Ok(vec);
                    }
                    (_, Parsed::Err(err)) => return Parsed::Err(err),
                };
            }
        }
    }

    fn spanned(self) -> impl Parser<I, Item, Spanned<Output, S>, Error, LE, S, Delim, Ctx>
    where
        S: Clone + Merge,
    {
        struct Span<P>(P);

        impl<I, Item, Output, Error, LE, S, Delim, Ctx, P>
            Parser<I, Item, Spanned<Output, S>, Error, LE, S, Delim, Ctx> for Span<P>
        where
            Self: Sized,
            Ctx: Clone,
            Error: ParseError<Item, LE, S, Delim, Ctx>,
            I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
            S: Clone + Merge,
            P: Parser<I, Item, Output, Error, LE, S, Delim, Ctx>,
            Delim: Delimiter,
        {
            #[inline(always)]
            fn parse(
                &self,
                ctx: &mut Context<I, Item, LE, S, Delim, Ctx>,
            ) -> Parsed<Spanned<Output, S>, Error> {
                let before = {
                    let mut clone = ctx.breakpoint();
                    _ = clone.next::<Error>();
                    clone.span()
                };
                let res = self.0.parse(ctx);
                let after = ctx.span();
                res.map(|v| Spanned(v, S::merge(before, after)))
            }

            #[inline(always)]
            fn chain<O1, O2, A, B, EE>(
                a: A,
                b: B,
            ) -> impl Parser<I, Item, (O1, O2), EE, LE, S, Delim, Ctx>
            where
                A: Parser<I, Item, O1, EE, LE, S, Delim, Ctx>,
                B: Parser<I, Item, O2, EE, LE, S, Delim, Ctx>,
                EE: ParseError<Item, LE, S, Delim, Ctx>,
            {
                P::chain(a, b)
            }
        }

        Span(self)
    }
}

impl<T> Merge for Range<T> {
    fn merge(a: Self, b: Self) -> Self {
        a.start..b.end
    }
}

pub trait Merge {
    fn merge(a: Self, b: Self) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T, S>(pub T, pub S);

impl<T: std::fmt::Display, S> std::fmt::Display for Spanned<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Borrow<'a, P>(&'a P);

impl<'a, I, Item, Output, Error, LE, S, Delim, Ctx, P>
    Parser<I, Item, Output, Error, LE, S, Delim, Ctx> for Borrow<'a, P>
where
    Self: Sized,
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
    P: Parser<I, Item, Output, Error, LE, S, Delim, Ctx>,
{
    fn parse(&self, ctx: &mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<Output, Error> {
        self.0.parse(ctx)
    }

    fn chain<O1, O2, A, B, E>(a: A, b: B) -> impl Parser<I, Item, (O1, O2), E, LE, S, Delim, Ctx>
    where
        A: Parser<I, Item, O1, E, LE, S, Delim, Ctx>,
        B: Parser<I, Item, O2, E, LE, S, Delim, Ctx>,
        E: ParseError<Item, LE, S, Delim, Ctx>,
    {
        P::chain(a, b)
    }
}

pub trait Delimiter: PartialEq + Clone {
    fn opposite(&self) -> Self;
}

impl Delimiter for () {
    fn opposite(&self) -> Self {}
}

pub trait ParseError<Item, LE, S, Delim, Ctx> {
    type Expected;

    fn lexer_error(lexer_error: LE, span: S, ctx: &Ctx) -> Self;
    fn expected_found(expected: Self::Expected, found: Item, span: S, ctx: &Ctx) -> Self;
    fn expected_found_eof(expected: Self::Expected, span: S, ctx: &Ctx) -> Self;
    fn expected_eof_found(found: Item, span: S, ctx: &Ctx) -> Self;
    fn tried_to_close_unopened_delimiter(closer: Delim, span: S, ctx: &Ctx) -> Self;
    fn tried_to_close_mismatching_delimiter(
        matching_open: (S, Delim),
        current_open: (S, Delim),
        close: (S, Delim),
        ctx: &Ctx,
    ) -> Self;
    fn did_not_close_delimiter(opener: Delim, span: S, eof: S, ctx: &Ctx) -> Self;

    fn expected(self, expected: Self::Expected) -> Self;
    fn merge(a: Self, b: Self) -> Self;
}

impl<I, Item, Output, Error, LE, S, Delim, Ctx, F> Parser<I, Item, Output, Error, LE, S, Delim, Ctx>
    for F
where
    F: Fn(&mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<Output, Error>,
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
{
    #[inline(always)]
    fn parse(&self, ctx: &mut Context<I, Item, LE, S, Delim, Ctx>) -> Parsed<Output, Error> {
        self(ctx)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed<T, E> {
    Ok(T),
    Err(E),
    Fatal(E),
    Recover(E, T),
}

impl<T, E> Parsed<T, E> {
    #[inline(always)]
    pub fn map<U, F>(self, f: F) -> Parsed<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Parsed::Ok(value) => Parsed::Ok(f(value)),
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Recover(err, value) => Parsed::Recover(err, f(value)),
        }
    }
}

#[derive(Debug)]
pub struct Context<I, Item, LE, S, Delim, Ctx>
where
    I: Iterator<Item = (Result<Item, LE>, S)>,
{
    iter: I,
    span: S,
    stack: Vec<(S, Delim)>,
    pub ctx: Ctx,
}

impl<I, Item, LE, S, Delim, Ctx> Context<I, Item, LE, S, Delim, Ctx>
where
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
    Ctx: Clone,
{
    pub fn new(iter: I, span: S, ctx: Ctx) -> Self {
        Self {
            iter,
            span,
            stack: Vec::new(),
            ctx,
        }
    }

    #[allow(clippy::should_implement_trait)]
    #[inline(always)]
    pub fn next<E>(&mut self) -> Parsed<Option<Item>, E>
    where
        E: ParseError<Item, LE, S, Delim, Ctx>,
    {
        match self.iter.next() {
            None => Parsed::Ok(None),
            Some((Ok(item), span)) => {
                self.span = span;
                Parsed::Ok(Some(item))
            }
            Some((Err(e), span)) => {
                self.span = span.clone();
                Parsed::Fatal(E::lexer_error(e, span, &self.ctx))
            }
        }
    }

    pub fn open_delimiter(&mut self, opener: Delim) {
        let span = self.span();
        self.stack.push((span, opener))
    }

    pub fn close_delimiter<E>(&mut self, closer: Delim) -> Result<(), E>
    where
        E: ParseError<Item, LE, S, Delim, Ctx>,
    {
        if let Some((opener_span, opener)) = self.stack.last() {
            if opener.opposite() == closer {
                self.stack.pop();
                return Ok(());
            }
            for matching in self.stack.iter().rev() {
                if matching.1.opposite() == closer {
                    return Err(E::tried_to_close_mismatching_delimiter(
                        matching.clone(),
                        (opener_span.clone(), opener.clone()),
                        (self.span(), closer),
                        &self.ctx,
                    ));
                }
            }
        }
        Err(E::tried_to_close_unopened_delimiter(
            closer,
            self.span(),
            &self.ctx,
        ))
    }

    #[inline(always)]
    pub fn span(&self) -> S {
        self.span.clone()
    }

    fn breakpoint(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            span: self.span.clone(),
            stack: self.stack.clone(),
            ctx: self.ctx.clone(),
        }
    }

    pub fn take_ctx(self) -> Ctx {
        self.ctx
    }
}

pub fn select<I, Item, Output, Error, LE, S, Delim, Ctx>(
    expected: Error::Expected,
    func: impl Fn(Item) -> Option<Output>,
) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx>
where
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
    Error::Expected: Clone,
    Item: Clone,
{
    move |ctx: &mut Context<I, Item, LE, S, Delim, Ctx>| {
        let token = ctx.next::<Error>();
        let eof_error = || Error::expected_found_eof(expected.clone(), ctx.span(), &ctx.ctx);
        let found_other_error =
            |other| Error::expected_found(expected.clone(), other, ctx.span(), &ctx.ctx);
        match token.map(|opt| opt.map(|item| (item.clone(), func(item)))) {
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Ok(Some((_, Some(value)))) => Parsed::Ok(value),
            Parsed::Ok(Some((item, None))) => Parsed::Err(found_other_error(item)),
            Parsed::Ok(None) => Parsed::Err(eof_error()),
            Parsed::Recover(err, Some((_, Some(value)))) => Parsed::Recover(err, value),
            Parsed::Recover(err, Some((item, None))) => {
                Parsed::Fatal(Error::merge(found_other_error(item), err))
            }
            Parsed::Recover(err, None) => Parsed::Fatal(Error::merge(eof_error(), err)),
        }
    }
}

pub fn select_ctx<I, Item, Output, Error, LE, S, Delim, Ctx>(
    expected: Error::Expected,
    func: impl Fn(Item, &mut Context<I, Item, LE, S, Delim, Ctx>) -> Option<Output>,
) -> impl Parser<I, Item, Output, Error, LE, S, Delim, Ctx>
where
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
    Error::Expected: Clone,
    Item: Clone,
{
    move |ctx: &mut Context<I, Item, LE, S, Delim, Ctx>| {
        let token = ctx
            .next::<Error>()
            .map(|opt| opt.map(|item| (item.clone(), func(item, ctx))));
        let eof_error = || Error::expected_found_eof(expected.clone(), ctx.span(), &ctx.ctx);
        let found_other_error =
            |other| Error::expected_found(expected.clone(), other, ctx.span(), &ctx.ctx);
        match token {
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Ok(Some((_, Some(value)))) => Parsed::Ok(value),
            Parsed::Ok(Some((item, None))) => Parsed::Err(found_other_error(item)),
            Parsed::Ok(None) => Parsed::Err(eof_error()),
            Parsed::Recover(err, Some((_, Some(value)))) => Parsed::Recover(err, value),
            Parsed::Recover(err, Some((item, None))) => {
                Parsed::Fatal(Error::merge(found_other_error(item), err))
            }
            Parsed::Recover(err, None) => Parsed::Fatal(Error::merge(eof_error(), err)),
        }
    }
}

#[inline(always)]
pub fn just<I, Item, Error, LE, S, Delim, Ctx>(
    item: Item,
) -> impl Parser<I, Item, Item, Error, LE, S, Delim, Ctx>
where
    Item: Clone + PartialEq,
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
    Error::Expected: From<Item>,
{
    #[inline(always)]
    move |ctx: &mut Context<I, Item, LE, S, Delim, Ctx>| match ctx.next::<Error>() {
        Parsed::Ok(Some(i)) if i == item => Parsed::Ok(i),
        Parsed::Ok(Some(i)) => Parsed::Err(Error::expected_found(
            item.clone().into(),
            i,
            ctx.span(),
            &ctx.ctx,
        )),
        Parsed::Ok(None) => Parsed::Err(Error::expected_found_eof(
            item.clone().into(),
            ctx.span(),
            &ctx.ctx,
        )),
        Parsed::Err(e) => Parsed::Err(e),
        Parsed::Fatal(e) => Parsed::Fatal(e),
        Parsed::Recover(ee, Some(t)) if t == item => Parsed::Recover(ee, t),
        Parsed::Recover(ee, Some(t)) => Parsed::Err(Error::merge(
            Error::expected_found(item.clone().into(), t, ctx.span(), &ctx.ctx),
            ee,
        )),
        Parsed::Recover(ee, None) => Parsed::Fatal(Error::merge(
            Error::expected_found_eof(item.clone().into(), ctx.span(), &ctx.ctx),
            ee,
        )),
    }
}

pub fn eof<I, Item, Error, LE, S, Delim, Ctx>(
    ctx: &mut Context<I, Item, LE, S, Delim, Ctx>,
) -> Parsed<(), Error>
where
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter,
{
    match ctx.next() {
        Parsed::Err(err) => return Parsed::Err(err),
        Parsed::Fatal(err) => return Parsed::Fatal(err),
        Parsed::Recover(err, _) => return Parsed::Fatal(err),
        Parsed::Ok(Some(item)) => {
            return Parsed::Err(Error::expected_eof_found(item, ctx.span(), &ctx.ctx));
        }
        Parsed::Ok(None) => {}
    };
    let eof = ctx.span().clone();
    let ctx_ctx = &ctx.ctx;
    match ctx
        .stack
        .drain(0..)
        .map(|(span, open)| Error::did_not_close_delimiter(open, span, eof.clone(), ctx_ctx))
        .reduce(Error::merge)
    {
        Some(err) => Parsed::Err(err),
        None => Parsed::Ok(()),
    }
}

macro_rules! impl_color_wrapper {
    ($name:ident, $method:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name<T: Display>(pub T);

        impl<T: Display> std::fmt::Display for $name<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0.$method())
            }
        }
    };
}

impl_color_wrapper!(Red, red);
impl_color_wrapper!(Blue, blue);
impl_color_wrapper!(Green, green);

pub fn open_delimiter<I, Item, Error, LE, S, Delim, Ctx>(
    delim: Delim,
) -> impl Parser<I, Item, Delim, Error, LE, S, Delim, Ctx>
where
    Item: PartialEq + Clone,
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter + Clone + 'static,
    Item: From<Delim>,
    Error::Expected: From<Delim> + From<Item>,
    <Error as ParseError<Item, LE, S, Delim, Ctx>>::Expected: Clone + 'static,
{
    let expected: Error::Expected = delim.clone().into();
    just(delim.clone().into())
        .map_err(move |e: Error| e.expected(expected.clone()))
        .map_ctx(move |_, ctx| {
            ctx.open_delimiter(delim.clone());
            delim.clone()
        })
}

pub fn close_delimiter<I, Item, Error, LE, S, Delim, Ctx>(
    delim: Delim,
) -> impl Parser<I, Item, Delim, Error, LE, S, Delim, Ctx>
where
    Item: PartialEq + Clone,
    Ctx: Clone,
    Error: ParseError<Item, LE, S, Delim, Ctx>,
    I: Iterator<Item = (Result<Item, LE>, S)> + Clone,
    S: Clone,
    Delim: Delimiter + Clone + TryFrom<Item>,
    Item: From<Delim>,
    Error::Expected: From<Delim> + From<Item> + Clone,
{
    select(delim.clone().into(), move |token| {
        if token == delim.clone().into() {
            Some(delim.clone())
        } else {
            Delim::try_from(token).ok()
        }
    })
    .try_map_ctx(
        move |delim, ctx| match ctx.close_delimiter::<Error>(delim.clone()) {
            Ok(_) => Parsed::Ok(delim.clone()),
            Err(e) => Parsed::Err(e),
        },
    )
}

//pub fn r#if() {
//    let (cond, (then, r#else)) = syntax!(If ! expr Then expr ?{Else expr});
//}
//
//pub fn func() {
//    syntax!(Fn ! pattern Arrow expr);
//    syntax!(For ! OpenParen *Comma{identifier ?{ Colon ! *Comma{expr} } } ?{Comma} CloseParen);
//    syntax!(Def ! ?{ OpenBrace ! *Comma{expr} ?{Comma} CloseBrace } identifier *{pattern} Equal expr Semicolon);
//}
