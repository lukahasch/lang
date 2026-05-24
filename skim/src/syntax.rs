#[macro_export]
macro_rules! syntax {
    [] => {
        { move |_ctx: &mut _| $crate::parser::Parsed::Ok($crate::Structure(())) }
    };

    [$($tokens:tt)+] => {
        $crate::step!(@{} $($tokens)* )
    };
}

#[macro_export]
macro_rules! step {
    (@inner $ctx:tt $name:tt $lit:literal $($tokens:tt)*) => {
        $crate::modifier!(@ $ctx { $crate::just($lit) } $name $($tokens)*)
    };
    (@inner $ctx:tt $name:tt $a:ident :: $b:ident $($tokens:tt)*) => {
        $crate::modifier!(@ $ctx { $crate::just($a :: $b) } $name $($tokens)*)
    };
    (@inner $ctx:tt $name:tt $lit:ident $($tokens:tt)*) => {
        $crate::modifier!(@ $ctx { $crate::just($lit) } $name $($tokens)*)
    };
    (@inner $ctx:tt $name:tt ($($tt:tt)*) $($tokens:tt)*) => {
        $crate::modifier!(@ $ctx { $crate::syntax!($($tt)*) } $name $($tokens)*)
    };
    (@inner $ctx:tt $name:tt {$expr:expr} $($tokens:tt)*) => {
        $crate::modifier!(@ $ctx { $expr } $name $($tokens)*)
    };
    (@inner $ctx:tt $name:tt $($tokens:tt)+) => {
        compile_error!(stringify!(Invalid Parser: $($tokens)+))
    };
    (@ $ctx:tt) => {
        $crate::finish!($ctx)
    };
    (@ $ctx:tt <$name:ident $( : { $($map:tt)* })?> $($tokens:tt)+) => {
        $crate::step!(@inner $ctx {=> $name $(: { $($map)*})?} $($tokens)*)
    };
    (@ $ctx:tt $($tokens:tt)+) => {
        $crate::step!(@inner $ctx {} $($tokens)*)
    };
}

#[macro_export]
macro_rules! modifier {
    (@ $ctx:tt $next:tt $name:tt * $($tokens:tt)*) => {
        $crate::append!(@ $ctx { $next.repeated() } $name $($tokens)*)
    };
    (@ $ctx:tt $next:tt $name:tt ? $($tokens:tt)*) => {
        $crate::append!(@ $ctx { $next.optional() } $name $($tokens)*)
    };
    (@ $ctx:tt $next:tt $name:tt $($tokens:tt)*) => {
        $crate::append!(@ $ctx $next $name $($tokens)*)
    };
}

#[macro_export]
macro_rules! append {
    (@ {$($pipe:tt $parser:expr $(=> $binding:ident $(: $map:expr)?)? ,)*} $next:tt {$(=> $next_binding:ident $(: $next_map:expr)?)?} ! $($tokens:tt)*) => {
        $crate::step!(
            @ {$($pipe $parser $(=> $binding $(: $map)?)? ,)* {move |p| p.cut()} $next $(=> $next_binding $(: $next_map)?)?, }
            $($tokens)*
        )
    };
    (@ {$($pipe:tt $parser:expr $(=> $binding:ident $(: $map:expr)?)? ,)*} $next:tt {$(=> $next_binding:ident $(: $next_map:expr)?)?} | $($tokens:tt)*) => {
        $crate::finish!(
            {$($pipe $parser $(=> $binding $(: $map)?)? ,)* {move |p| p} $next $(=> $next_binding $(: $next_map)?)?, }
        ).or($crate::syntax![$($tokens)*])
    };
    (@ {$($pipe:tt $parser:expr $(=> $binding:ident $(: $map:expr)?)? ,)*} $next:tt {$(=> $next_binding:ident $(: $next_map:expr)?)?} $($tokens:tt)*) => {
        $crate::step!(
            @ {$($pipe $parser $(=> $binding $(: $map)?)? ,)* {move |p| p} $next $(=> $next_binding $(: $next_map)?)?, }
            $($tokens)*
        )
    };
}

pub use tap::Pipe;

#[macro_export]
macro_rules! finish {
    ({$({ $($pipe:tt)* } $parser:expr ,)*}) => {{
        use $crate::Parser;
        use $crate::syntax::Pipe;

        { move |_: &mut _| $crate::parser::Parsed::Ok(()) }
        $(
            .and($parser)
            .pipe( $($pipe)* )
        )*
        .map(
            |$crate::tool!(
                {_} $crate::tool!(@identifiers{} {$({$parser})*} { A B C D E F })
            )|
            stringify!($crate::tool!(@identifiers{} {$({$parser})*} { {B} {C} {D} {E} {F} }))
        )
    }};
    ({$({ $($pipe:tt)* } $parser:expr $(=> $binding:ident $(: $map:expr)?)? ,)*}) => {{
        use $crate::Parser;
        use $crate::syntax::Pipe;

        { move |_: &mut _| $crate::parser::Parsed::Ok(()) }
        $(
            .and($parser $($( .map(|$binding| $map) )?)?)
            .pipe( $($pipe)* )
        )*
        .map(
            |$crate::tool!(
                {_} $(
                    {$crate::tool!(@first $($binding)? _)}
                )*
            )|
            $crate::make_struct!( $( $( $binding : $binding,)? )* )
        )
    }};
}

#[macro_export]
macro_rules! tool {
    (@unwrap { $($acc:tt)* }) => {
        $($acc)*
    };

    (@unwrap { $($acc:tt)* } { $($next:tt)* } $( { $($rest:tt)* } )*) => {
        $crate::tool!(@unwrap { ($($acc)*, $($next)*) } $( { $($rest)* } )*)
    };

    ({ $($first:tt)* } $( { $($rest:tt)* } )*) => {
        $crate::tool!(@unwrap { $($first)* } $( { $($rest)* } )*)
    };

    (@first $first:tt $($rest:tt)*) => {
        $first
    };

    (@identifiers {$($acc:tt)*} {$flen:tt } {$fi:tt} ) => {
        $($acc)* $fi
    };
    (@identifiers {$($acc:tt)*} {$flen:tt $($len:tt)*} {$fi:tt $($idents:tt)*}) => {
        $crate::tool!(@identifiers {$($acc)* $fi} {$flen $($len)*} {$($idents)*})
    };
}
#[macro_export]
macro_rules! d {
    ($($tt:tt)*) => {
        stringify!($($tt)*)
    };
}

#[cfg(test)]
mod tests {
    use derive_more::Display;

    use crate::{Context, Parsed, Parser};

    #[derive(Debug, Clone, PartialEq, Display)]
    enum Tok {
        A,
        B,
        C,
    }

    impl From<Tok> for String {
        fn from(value: Tok) -> Self {
            format!("{value}")
        }
    }

    #[allow(clippy::type_complexity)]
    fn ctx<T>(
        i: impl Iterator<Item = T> + Clone,
    ) -> Context<impl Iterator<Item = (Result<T, ()>, usize)> + Clone, T, (), usize, (), ()> {
        Context::new(i.enumerate().map(move |(i, v)| (Ok(v), i)), 0, ())
    }

    #[test]
    fn empty_syntax() {
        let v: Parsed<_, String> = syntax![].parse(&mut ctx("".chars()));
        assert!(v.is_ok())
    }

    #[test]
    fn single_token() {
        let v: Parsed<_, String> = syntax![Tok::A].parse(&mut ctx(std::iter::once(Tok::A)));
        assert!(v.is_ok());
    }

    #[test]
    fn token_sequence() {
        let v: Parsed<_, String> =
            syntax![Tok::A Tok::B Tok::C].parse(&mut ctx([Tok::A, Tok::B, Tok::C].into_iter()));
        assert!(v.is_ok());
    }

    #[test]
    fn token_sequence_error() {
        let v: Parsed<_, String> =
            syntax![Tok::A Tok::B Tok::C].parse(&mut ctx([Tok::A, Tok::C, Tok::C].into_iter()));
        assert!(v.is_err());
    }

    #[test]
    fn token_sequence_error_cut() {
        let v: Parsed<_, String> =
            syntax![Tok::A ! Tok::B Tok::C].parse(&mut ctx([Tok::A, Tok::C, Tok::C].into_iter()));
        assert!(v.is_fatal());
    }

    #[test]
    fn optional() {
        let v: Parsed<_, String> = syntax![Tok::A?].parse(&mut ctx([Tok::C].into_iter()));
        assert!(v.is_ok());
    }

    #[test]
    fn repeated() {
        let v: Parsed<_, String> =
            syntax![<items> Tok::A*].parse(&mut ctx([Tok::A, Tok::A, Tok::A].into_iter()));
        assert!(v.is_ok());
        assert_eq!(v.unwrap_ok().take::<"items", _>().len(), 3);
    }

    #[test]
    fn nested() {
        let v: Parsed<_, String> =
            syntax![<items> (Tok::A ! Tok::B)*]
                .parse(&mut ctx([Tok::A, Tok::A, Tok::A].into_iter()));
        assert!(v.is_fatal());
    }

    #[test]
    fn or() {
        let v: Parsed<_, String> =
            syntax![<items> (Tok::A | Tok::B)*]
                .parse(&mut ctx([Tok::A, Tok::B, Tok::A].into_iter()));
        assert!(v.is_ok());
    }
}
