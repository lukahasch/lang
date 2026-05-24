#[macro_export]
macro_rules! syntax {
    [] => {
        move |_ctx: &mut _| $crate::parser::Parsed::Ok($crate::Structure(()))
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
        $crate::step!(
            @ {$($pipe $parser $(=> $binding $(: $map)?)? ,)* {move |p| p.or($crate::syntax![$($tokens)*])} $next $(=> $next_binding $(: $next_map)?)?, }

        )
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
}
#[macro_export]
macro_rules! d {
    ($($tt:tt)*) => {
        stringify!($($tt)*)
    };
}

#[cfg(test)]
mod tests {
    #[derive(Debug, Clone, PartialEq)]
    enum Tok {
        A,
        B,
        C,
    }
}
