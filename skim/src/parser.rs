use std::ops::Range;
use std::{fmt::Display, marker::PhantomData};

// --- Checkpoint ---

pub trait Checkpoint {
    type Save;
    fn checkpoint(&self) -> Self::Save;
    fn restore(&mut self, save: Self::Save);
}

pub struct CheckpointClone<T>(pub T);

impl<T: Clone> Checkpoint for CheckpointClone<T> {
    type Save = T;
    fn checkpoint(&self) -> T {
        self.0.clone()
    }
    fn restore(&mut self, save: T) {
        self.0 = save;
    }
}

impl Checkpoint for () {
    type Save = ();

    fn checkpoint(&self) -> Self::Save {}

    fn restore(&mut self, _: Self::Save) {}
}

// --- Parser trait ---

pub trait Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>
where
    Self: Sized,
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
{
    fn parse(
        &self,
        ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>,
    ) -> Parsed<Output, Error>;

    #[inline(always)]
    fn chain<LeftOutput, RightOutput, LeftParser, RightParser, ChainError>(
        left: LeftParser,
        right: RightParser,
    ) -> impl Parser<Input, Token, (LeftOutput, RightOutput), ChainError, LexerError, Span, Delim, State>
    where
        LeftParser: Parser<Input, Token, LeftOutput, ChainError, LexerError, Span, Delim, State>,
        RightParser: Parser<Input, Token, RightOutput, ChainError, LexerError, Span, Delim, State>,
        ChainError: ParseError<Token, LexerError, Span, Delim, State>,
    {
        move |ctx: &mut _| match left.parse(ctx) {
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Ok(ok) => match right.parse(ctx) {
                Parsed::Err(err) => Parsed::Err(err),
                Parsed::Fatal(err) => Parsed::Fatal(err),
                Parsed::Recover(err, value) => Parsed::Recover(err, (ok, value)),
                Parsed::Ok(value) => Parsed::Ok((ok, value)),
            },
            Parsed::Recover(err, ok) => match right.parse(ctx) {
                Parsed::Ok(value) => Parsed::Recover(err, (ok, value)),
                Parsed::Err(err2) | Parsed::Fatal(err2) => {
                    Parsed::Fatal(ChainError::merge(err, err2))
                }
                Parsed::Recover(err2, value) => {
                    Parsed::Recover(ChainError::merge(err, err2), (ok, value))
                }
            },
        }
    }

    #[inline(always)]
    fn map_parsed_ctx<MappedOutput, MappedError>(
        self,
        f: impl Fn(
            Parsed<Output, Error>,
            &mut Context<Input, Token, LexerError, Span, Delim, State>,
        ) -> Parsed<MappedOutput, MappedError>,
    ) -> impl Parser<Input, Token, MappedOutput, MappedError, LexerError, Span, Delim, State>
    where
        MappedError: ParseError<Token, LexerError, Span, Delim, State>,
    {
        struct Map<Inner, Func, Output, Error, Token, LexerError, Span, Delim, State>(
            Inner,
            Func,
            PhantomData<(Output, Error, Token, LexerError, Span, Delim, State)>,
        );

        impl<
            Input,
            Token,
            Output,
            Error,
            LexerError,
            Span,
            Delim,
            State,
            Inner,
            Func,
            MappedOutput,
            MappedError,
        > Parser<Input, Token, MappedOutput, MappedError, LexerError, Span, Delim, State>
            for Map<Inner, Func, Output, Error, Token, LexerError, Span, Delim, State>
        where
            Self: Sized,
            State: Checkpoint,
            Error: ParseError<Token, LexerError, Span, Delim, State>,
            MappedError: ParseError<Token, LexerError, Span, Delim, State>,
            Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
            Span: Clone,
            Inner: Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>,
            Func: Fn(
                Parsed<Output, Error>,
                &mut Context<Input, Token, LexerError, Span, Delim, State>,
            ) -> Parsed<MappedOutput, MappedError>,
            Delim: Delimiter,
        {
            #[inline(always)]
            fn parse(
                &self,
                ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>,
            ) -> Parsed<MappedOutput, MappedError> {
                self.1(self.0.parse(ctx), ctx)
            }

            #[inline(always)]
            fn chain<LeftOutput, RightOutput, LeftParser, RightParser, ChainError>(
                left: LeftParser,
                right: RightParser,
            ) -> impl Parser<
                Input,
                Token,
                (LeftOutput, RightOutput),
                ChainError,
                LexerError,
                Span,
                Delim,
                State,
            >
            where
                LeftParser:
                    Parser<Input, Token, LeftOutput, ChainError, LexerError, Span, Delim, State>,
                RightParser:
                    Parser<Input, Token, RightOutput, ChainError, LexerError, Span, Delim, State>,
                ChainError: ParseError<Token, LexerError, Span, Delim, State>,
            {
                Inner::chain(left, right)
            }
        }

        Map(self, f, PhantomData)
    }

    fn inspect<InspectReturn>(
        self,
        f: impl Fn(&Parsed<Output, Error>) -> InspectReturn,
    ) -> impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State> {
        self.map_parsed_ctx(move |parsed, _| {
            f(&parsed);
            parsed
        })
    }

    fn map<MappedOutput>(
        self,
        f: impl Fn(Output) -> MappedOutput,
    ) -> impl Parser<Input, Token, MappedOutput, Error, LexerError, Span, Delim, State> {
        self.map_parsed_ctx(move |parsed, _| match parsed {
            Parsed::Ok(ok) => Parsed::Ok(f(ok)),
            Parsed::Recover(err, ok) => Parsed::Recover(err, f(ok)),
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
        })
    }

    fn map_err<MappedError>(
        self,
        f: impl Fn(Error) -> MappedError,
    ) -> impl Parser<Input, Token, Output, MappedError, LexerError, Span, Delim, State>
    where
        MappedError: ParseError<Token, LexerError, Span, Delim, State>,
    {
        self.map_parsed_ctx(move |parsed, _| match parsed {
            Parsed::Ok(ok) => Parsed::Ok(ok),
            Parsed::Recover(err, ok) => Parsed::Recover(f(err), ok),
            Parsed::Err(err) => Parsed::Err(f(err)),
            Parsed::Fatal(err) => Parsed::Fatal(f(err)),
        })
    }

    fn expect(
        self,
        expected: Error::Expected,
    ) -> impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>
    where
        Error::Expected: Clone + 'static,
    {
        self.map_parsed_ctx(move |parsed, _| match parsed {
            Parsed::Ok(ok) => Parsed::Ok(ok),
            Parsed::Err(err) => Parsed::Err(err.expected(expected.clone())),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Recover(err, ok) => Parsed::Recover(err, ok),
        })
    }

    fn map_ctx<MappedOutput>(
        self,
        f: impl Fn(Output, &mut Context<Input, Token, LexerError, Span, Delim, State>) -> MappedOutput,
    ) -> impl Parser<Input, Token, MappedOutput, Error, LexerError, Span, Delim, State> {
        self.map_parsed_ctx(move |parsed, ctx| match parsed {
            Parsed::Ok(ok) => Parsed::Ok(f(ok, ctx)),
            Parsed::Recover(err, ok) => Parsed::Recover(err, f(ok, ctx)),
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
        })
    }

    fn try_map_ctx<MappedOutput>(
        self,
        f: impl Fn(
            Output,
            &mut Context<Input, Token, LexerError, Span, Delim, State>,
        ) -> Parsed<MappedOutput, Error>,
    ) -> impl Parser<Input, Token, MappedOutput, Error, LexerError, Span, Delim, State> {
        self.map_parsed_ctx(move |parsed, ctx| match parsed {
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
    fn and<OtherOutput>(
        self,
        other: impl Parser<Input, Token, OtherOutput, Error, LexerError, Span, Delim, State>,
    ) -> impl Parser<Input, Token, (Output, OtherOutput), Error, LexerError, Span, Delim, State>
    {
        Self::chain(self, other)
    }

    #[inline(always)]
    fn and_ignore<OtherOutput>(
        self,
        other: impl Parser<Input, Token, OtherOutput, Error, LexerError, Span, Delim, State>,
    ) -> impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State> {
        self.and(other).map(|(value, _)| value)
    }

    #[inline(always)]
    fn ignore_and<OtherOutput>(
        self,
        other: impl Parser<Input, Token, OtherOutput, Error, LexerError, Span, Delim, State>,
    ) -> impl Parser<Input, Token, OtherOutput, Error, LexerError, Span, Delim, State> {
        self.and(other).map(|(_, value)| value)
    }

    fn cut(self) -> impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State> {
        struct Cut<Inner>(Inner);

        impl<Input, Token, Output, Error, LexerError, Span, Delim, State, Inner>
            Parser<Input, Token, Output, Error, LexerError, Span, Delim, State> for Cut<Inner>
        where
            Self: Sized,
            State: Checkpoint,
            Error: ParseError<Token, LexerError, Span, Delim, State>,
            Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
            Span: Clone,
            Inner: Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>,
            Delim: Delimiter,
        {
            #[inline(always)]
            fn parse(
                &self,
                ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>,
            ) -> Parsed<Output, Error> {
                self.0.parse(ctx)
            }

            #[inline(always)]
            fn chain<LeftOutput, RightOutput, LeftParser, RightParser, ChainError>(
                left: LeftParser,
                right: RightParser,
            ) -> impl Parser<
                Input,
                Token,
                (LeftOutput, RightOutput),
                ChainError,
                LexerError,
                Span,
                Delim,
                State,
            >
            where
                LeftParser:
                    Parser<Input, Token, LeftOutput, ChainError, LexerError, Span, Delim, State>,
                RightParser:
                    Parser<Input, Token, RightOutput, ChainError, LexerError, Span, Delim, State>,
                ChainError: ParseError<Token, LexerError, Span, Delim, State>,
            {
                Cut(move |ctx: &mut _| match left.parse(ctx) {
                    Parsed::Err(err) => Parsed::Err(err),
                    Parsed::Fatal(err) => Parsed::Fatal(err),
                    Parsed::Ok(ok) => match right.parse(ctx) {
                        Parsed::Err(err) => Parsed::Fatal(err),
                        Parsed::Fatal(err) => Parsed::Fatal(err),
                        Parsed::Recover(err, value) => Parsed::Recover(err, (ok, value)),
                        Parsed::Ok(value) => Parsed::Ok((ok, value)),
                    },
                    Parsed::Recover(err, ok) => match right.parse(ctx) {
                        Parsed::Ok(value) => Parsed::Recover(err, (ok, value)),
                        Parsed::Err(err2) | Parsed::Fatal(err2) => {
                            Parsed::Fatal(ChainError::merge(err, err2))
                        }
                        Parsed::Recover(err2, value) => {
                            Parsed::Recover(ChainError::merge(err, err2), (ok, value))
                        }
                    },
                })
            }
        }

        Cut(self)
    }

    fn optional(
        self,
    ) -> impl Parser<Input, Token, Option<Output>, Error, LexerError, Span, Delim, State> {
        #[inline(always)]
        move |ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>| {
            let save = ctx.checkpoint();
            match self.parse(ctx) {
                Parsed::Ok(value) => Parsed::Ok(Some(value)),
                Parsed::Fatal(err) => Parsed::Fatal(err),
                Parsed::Recover(err, value) => Parsed::Recover(err, Some(value)),
                Parsed::Err(_) => {
                    ctx.restore(save);
                    Parsed::Ok(None)
                }
            }
        }
    }

    fn or(
        self,
        other: impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>,
    ) -> impl Parser<Input, Token, Output, Error, LexerError, Span, Delim, State> {
        #[inline(always)]
        move |ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>| {
            let save = ctx.checkpoint();
            match self.parse(ctx) {
                Parsed::Ok(value) => Parsed::Ok(value),
                Parsed::Fatal(err) => Parsed::Fatal(err),
                Parsed::Recover(err, value) => Parsed::Recover(err, value),
                Parsed::Err(_) => {
                    ctx.restore(save);
                    other.parse(ctx)
                }
            }
        }
    }

    fn as_ref(&self) -> Ref<'_, Self> {
        Ref(self)
    }

    fn repeated(
        self,
    ) -> impl Parser<Input, Token, Vec<Output>, Error, LexerError, Span, Delim, State> {
        #[inline(always)]
        move |ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>| {
            let mut accumulator: Result<Vec<Output>, (Error, Vec<Output>)> = Ok(Vec::new());
            loop {
                let save = ctx.checkpoint();
                accumulator = match (accumulator, self.parse(ctx)) {
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
                    (Ok(vec), Parsed::Err(_)) => {
                        ctx.restore(save);
                        return Parsed::Ok(vec);
                    }
                    (Err((err, vec)), Parsed::Err(_)) => {
                        ctx.restore(save);
                        return Parsed::Recover(err, vec);
                    }
                };
            }
        }
    }

    fn spanned(
        self,
    ) -> impl Parser<Input, Token, Spanned<Output, Span>, Error, LexerError, Span, Delim, State>
    where
        Span: Clone + Merge,
    {
        struct SpanCapture<Inner>(Inner);

        impl<Input, Token, Output, Error, LexerError, Span, Delim, State, Inner>
            Parser<Input, Token, Spanned<Output, Span>, Error, LexerError, Span, Delim, State>
            for SpanCapture<Inner>
        where
            Self: Sized,
            State: Checkpoint,
            Error: ParseError<Token, LexerError, Span, Delim, State>,
            Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
            Span: Clone + Merge,
            Inner: Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>,
            Delim: Delimiter,
        {
            #[inline(always)]
            fn parse(
                &self,
                ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>,
            ) -> Parsed<Spanned<Output, Span>, Error> {
                let before = {
                    let save = ctx.checkpoint();
                    let _ = ctx.next::<Error>();
                    let span = ctx.span();
                    ctx.restore(save);
                    span
                };
                let res = self.0.parse(ctx);
                let after = ctx.span();
                res.map(|v| Spanned(v, Span::merge(before, after)))
            }

            #[inline(always)]
            fn chain<LeftOutput, RightOutput, LeftParser, RightParser, ChainError>(
                left: LeftParser,
                right: RightParser,
            ) -> impl Parser<
                Input,
                Token,
                (LeftOutput, RightOutput),
                ChainError,
                LexerError,
                Span,
                Delim,
                State,
            >
            where
                LeftParser:
                    Parser<Input, Token, LeftOutput, ChainError, LexerError, Span, Delim, State>,
                RightParser:
                    Parser<Input, Token, RightOutput, ChainError, LexerError, Span, Delim, State>,
                ChainError: ParseError<Token, LexerError, Span, Delim, State>,
            {
                Inner::chain(left, right)
            }
        }

        SpanCapture(self)
    }
}

// --- Parsed ---

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed<Value, Error> {
    Ok(Value),
    Err(Error),
    Fatal(Error),
    Recover(Error, Value),
}

impl<Value, Error> Parsed<Value, Error> {
    #[inline(always)]
    pub fn map<Mapped>(self, f: impl FnOnce(Value) -> Mapped) -> Parsed<Mapped, Error> {
        match self {
            Parsed::Ok(value) => Parsed::Ok(f(value)),
            Parsed::Err(err) => Parsed::Err(err),
            Parsed::Fatal(err) => Parsed::Fatal(err),
            Parsed::Recover(err, value) => Parsed::Recover(err, f(value)),
        }
    }

    #[inline(always)]
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }

    #[inline(always)]
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    #[inline(always)]
    pub fn is_recover(&self) -> bool {
        matches!(self, Self::Recover(_, _))
    }

    #[inline(always)]
    pub fn unwrap_ok(self) -> Value
    where
        Error: std::fmt::Debug,
    {
        match self {
            Self::Ok(value) | Self::Recover(_, value) => value,
            Self::Err(err) => panic!("expected Ok, got Err({err:?})"),
            Self::Fatal(err) => panic!("expected Ok, got Fatal({err:?})"),
        }
    }
}

// --- Ref ---

pub struct Ref<'a, Inner>(&'a Inner);

impl<'a, Input, Token, Output, Error, LexerError, Span, Delim, State, Inner>
    Parser<Input, Token, Output, Error, LexerError, Span, Delim, State> for Ref<'a, Inner>
where
    Self: Sized,
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
    Inner: Parser<Input, Token, Output, Error, LexerError, Span, Delim, State>,
{
    fn parse(
        &self,
        ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>,
    ) -> Parsed<Output, Error> {
        self.0.parse(ctx)
    }

    fn chain<LeftOutput, RightOutput, LeftParser, RightParser, ChainError>(
        left: LeftParser,
        right: RightParser,
    ) -> impl Parser<Input, Token, (LeftOutput, RightOutput), ChainError, LexerError, Span, Delim, State>
    where
        LeftParser: Parser<Input, Token, LeftOutput, ChainError, LexerError, Span, Delim, State>,
        RightParser: Parser<Input, Token, RightOutput, ChainError, LexerError, Span, Delim, State>,
        ChainError: ParseError<Token, LexerError, Span, Delim, State>,
    {
        Inner::chain(left, right)
    }
}

// --- Closure impl ---

impl<Input, Token, Output, Error, LexerError, Span, Delim, State, Func>
    Parser<Input, Token, Output, Error, LexerError, Span, Delim, State> for Func
where
    Func: Fn(&mut Context<Input, Token, LexerError, Span, Delim, State>) -> Parsed<Output, Error>,
    State: Checkpoint,
    Error: ParseError<Token, LexerError, Span, Delim, State>,
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
{
    #[inline(always)]
    fn parse(
        &self,
        ctx: &mut Context<Input, Token, LexerError, Span, Delim, State>,
    ) -> Parsed<Output, Error> {
        self(ctx)
    }
}

// --- Support traits ---

pub trait Delimiter: PartialEq + Clone {
    fn opposite(&self) -> Self;
}

impl Delimiter for () {
    fn opposite(&self) -> Self {}
}

pub trait ParseError<Token, LexerError, Span, Delim, State> {
    type Expected;

    fn lexer_error(lexer_error: LexerError, span: Span, state: &State) -> Self;
    fn expected_found(expected: Self::Expected, found: Token, span: Span, state: &State) -> Self;
    fn expected_found_eof(expected: Self::Expected, span: Span, state: &State) -> Self;
    fn expected_eof_found(found: Token, span: Span, state: &State) -> Self;
    fn tried_to_close_unopened_delimiter(closer: Delim, span: Span, state: &State) -> Self;
    fn tried_to_close_mismatching_delimiter(
        matching_open: (Span, Delim),
        current_open: (Span, Delim),
        close: (Span, Delim),
        state: &State,
    ) -> Self;
    fn did_not_close_delimiter(opener: Delim, span: Span, eof: Span, state: &State) -> Self;

    fn expected(self, expected: Self::Expected) -> Self;
    fn merge(a: Self, b: Self) -> Self;
}

pub trait Merge {
    fn merge(a: Self, b: Self) -> Self;
}

impl<T> Merge for Range<T> {
    fn merge(a: Self, b: Self) -> Self {
        a.start..b.end
    }
}

// --- Spanned ---

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spanned<Value, Span>(pub Value, pub Span);

impl<Value: Display, Span> Display for Spanned<Value, Span> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// --- Context ---

#[derive(Debug)]
pub struct Context<Input, Token, LexerError, Span, Delim, State>
where
    Input: Iterator<Item = (Result<Token, LexerError>, Span)>,
{
    iter: Input,
    span: Span,
    pub(crate) stack: Vec<(Span, Delim)>,
    pub state: State,
}

// --- ContextSave & Checkpoint impl ---

pub struct ContextSave<Input, Span, Delim, StateSave> {
    iter: Input,
    span: Span,
    stack: Vec<(Span, Delim)>,
    state: StateSave,
}

impl<Input, Token, LexerError, Span, Delim, State> Checkpoint
    for Context<Input, Token, LexerError, Span, Delim, State>
where
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
    State: Checkpoint,
{
    type Save = ContextSave<Input, Span, Delim, State::Save>;

    fn checkpoint(&self) -> Self::Save {
        ContextSave {
            iter: self.iter.clone(),
            span: self.span.clone(),
            stack: self.stack.clone(),
            state: self.state.checkpoint(),
        }
    }

    fn restore(&mut self, save: Self::Save) {
        self.iter = save.iter;
        self.span = save.span;
        self.stack = save.stack;
        self.state.restore(save.state);
    }
}

impl<Input, Token, LexerError, Span, Delim, State>
    Context<Input, Token, LexerError, Span, Delim, State>
where
    Input: Iterator<Item = (Result<Token, LexerError>, Span)> + Clone,
    Span: Clone,
    Delim: Delimiter,
    State: Checkpoint,
{
    pub fn new(iter: Input, span: Span, state: State) -> Self {
        Self {
            iter,
            span,
            stack: Vec::new(),
            state,
        }
    }

    #[allow(clippy::should_implement_trait)]
    #[inline(always)]
    pub fn next<Error>(&mut self) -> Parsed<Option<Token>, Error>
    where
        Error: ParseError<Token, LexerError, Span, Delim, State>,
    {
        match self.iter.next() {
            None => Parsed::Ok(None),
            Some((Ok(token), span)) => {
                self.span = span;
                Parsed::Ok(Some(token))
            }
            Some((Err(e), span)) => {
                self.span = span.clone();
                Parsed::Fatal(Error::lexer_error(e, span, &self.state))
            }
        }
    }

    pub fn push_delimiter(&mut self, opener: Delim) {
        let span = self.span();
        self.stack.push((span, opener))
    }

    pub fn pop_delimiter<Error>(&mut self, closer: Delim) -> Result<(), Error>
    where
        Error: ParseError<Token, LexerError, Span, Delim, State>,
    {
        if let Some((opener_span, opener)) = self.stack.last() {
            if opener.opposite() == closer {
                self.stack.pop();
                return Ok(());
            }
            for matching in self.stack.iter().rev() {
                if matching.1.opposite() == closer {
                    return Err(Error::tried_to_close_mismatching_delimiter(
                        matching.clone(),
                        (opener_span.clone(), opener.clone()),
                        (self.span(), closer),
                        &self.state,
                    ));
                }
            }
        }
        Err(Error::tried_to_close_unopened_delimiter(
            closer,
            self.span(),
            &self.state,
        ))
    }

    /// Drains any unclosed delimiters from the stack and merges them into a
    /// single error, returning `None` if the stack is empty.
    pub fn drain_delimiter_errors<Error>(&mut self) -> Option<Error>
    where
        Error: ParseError<Token, LexerError, Span, Delim, State>,
    {
        let eof = self.span.clone();
        let entries: Vec<(Span, Delim)> = self.stack.drain(..).collect();
        entries
            .into_iter()
            .map(|(span, open)| {
                Error::did_not_close_delimiter(open, span, eof.clone(), &self.state)
            })
            .reduce(Error::merge)
    }

    #[inline(always)]
    pub fn span(&self) -> Span {
        self.span.clone()
    }

    pub fn take_state(self) -> State {
        self.state
    }
}
