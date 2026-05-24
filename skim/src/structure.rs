use std::fmt;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct With<const NAME: &'static str, T> {
    value: Option<T>,
}

impl<const NAME: &'static str, T> With<NAME, T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<const NAME: &'static str, T: fmt::Debug> fmt::Debug for With<NAME, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.value {
            Some(v) => write!(f, "{}: {:?}", NAME, v),
            None => write!(f, "{}: <taken>", NAME),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Structure<Fields>(pub Fields);

impl<Fields> Structure<Fields> {
    #[inline]
    pub fn take<const NAME: &'static str, Index>(
        &mut self,
    ) -> <Fields as TakeField<NAME, Index>>::Type
    where
        Fields: TakeField<NAME, Index>,
    {
        self.0.take_field()
    }
}

pub struct Idx<const N: usize>;

pub trait TakeField<const NAME: &'static str, Index> {
    type Type;
    fn take_field(&mut self) -> Self::Type;
}

macro_rules! impl_take_field {
    ($pos:tt; [$($before:ident),*]; [$($after:ident),*]) => {
        impl<const __N: &'static str, __V, $($before,)* $($after,)*>
            TakeField<__N, Idx<$pos>>
            for ($($before,)* With<__N, __V>, $($after,)*)
        {
            type Type = __V;

            #[inline]
            fn take_field(&mut self) -> __V {
                self.$pos.value
                    .take()
                    .unwrap_or_else(|| panic!("field \"{}\" has already been taken", __N))
            }
        }
    };
}

impl_take_field!(0; []; []);

impl_take_field!(0; []; [T1]);
impl_take_field!(1; [T0]; []);

impl_take_field!(0; []; [T1, T2]);
impl_take_field!(1; [T0]; [T2]);
impl_take_field!(2; [T0, T1]; []);

impl_take_field!(0; []; [T1, T2, T3]);
impl_take_field!(1; [T0]; [T2, T3]);
impl_take_field!(2; [T0, T1]; [T3]);
impl_take_field!(3; [T0, T1, T2]; []);

impl_take_field!(0; []; [T1, T2, T3, T4]);
impl_take_field!(1; [T0]; [T2, T3, T4]);
impl_take_field!(2; [T0, T1]; [T3, T4]);
impl_take_field!(3; [T0, T1, T2]; [T4]);
impl_take_field!(4; [T0, T1, T2, T3]; []);

impl_take_field!(0; []; [T1, T2, T3, T4, T5]);
impl_take_field!(1; [T0]; [T2, T3, T4, T5]);
impl_take_field!(2; [T0, T1]; [T3, T4, T5]);
impl_take_field!(3; [T0, T1, T2]; [T4, T5]);
impl_take_field!(4; [T0, T1, T2, T3]; [T5]);
impl_take_field!(5; [T0, T1, T2, T3, T4]; []);

impl_take_field!(0; []; [T1, T2, T3, T4, T5, T6]);
impl_take_field!(1; [T0]; [T2, T3, T4, T5, T6]);
impl_take_field!(2; [T0, T1]; [T3, T4, T5, T6]);
impl_take_field!(3; [T0, T1, T2]; [T4, T5, T6]);
impl_take_field!(4; [T0, T1, T2, T3]; [T5, T6]);
impl_take_field!(5; [T0, T1, T2, T3, T4]; [T6]);
impl_take_field!(6; [T0, T1, T2, T3, T4, T5]; []);

impl_take_field!(0; []; [T1, T2, T3, T4, T5, T6, T7]);
impl_take_field!(1; [T0]; [T2, T3, T4, T5, T6, T7]);
impl_take_field!(2; [T0, T1]; [T3, T4, T5, T6, T7]);
impl_take_field!(3; [T0, T1, T2]; [T4, T5, T6, T7]);
impl_take_field!(4; [T0, T1, T2, T3]; [T5, T6, T7]);
impl_take_field!(5; [T0, T1, T2, T3, T4]; [T6, T7]);
impl_take_field!(6; [T0, T1, T2, T3, T4, T5]; [T7]);
impl_take_field!(7; [T0, T1, T2, T3, T4, T5, T6]; []);

pub trait DebugField {
    fn debug_field(&self, builder: &mut fmt::DebugStruct<'_, '_>);
}

impl<const NAME: &'static str, T: fmt::Debug> DebugField for With<NAME, T> {
    fn debug_field(&self, builder: &mut fmt::DebugStruct<'_, '_>) {
        match &self.value {
            Some(v) => builder.field(NAME, v),
            None => builder.field(NAME, &"<taken>"),
        };
    }
}

pub trait DebugFields {
    fn debug_fields(&self, builder: &mut fmt::DebugStruct<'_, '_>);
}

impl DebugFields for () {
    fn debug_fields(&self, _builder: &mut fmt::DebugStruct<'_, '_>) {}
}

macro_rules! impl_debug_fields {
    ($($T:ident : $idx:tt),+) => {
        impl<$($T: DebugField),+> DebugFields for ($($T,)+) {
            fn debug_fields(&self, builder: &mut fmt::DebugStruct<'_, '_>) {
                $(self.$idx.debug_field(builder);)+
            }
        }
    };
}

impl_debug_fields!(T0: 0);
impl_debug_fields!(T0: 0, T1: 1);
impl_debug_fields!(T0: 0, T1: 1, T2: 2);
impl_debug_fields!(T0: 0, T1: 1, T2: 2, T3: 3);
impl_debug_fields!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4);
impl_debug_fields!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5);
impl_debug_fields!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6);
impl_debug_fields!(T0: 0, T1: 1, T2: 2, T3: 3, T4: 4, T5: 5, T6: 6, T7: 7);

impl<Fields: DebugFields> fmt::Debug for Structure<Fields> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("Structure");
        self.0.debug_fields(&mut builder);
        builder.finish()
    }
}

#[macro_export]
macro_rules! make_struct {
    ($($name:ident : $val:expr),* $(,)?) => {
        $crate::Structure(($($crate::With::<{ stringify!($name) }, _>::new($val),)*))
    };
}

#[macro_export]
macro_rules! struct_type {
    ($($name:ident : $ty:ty),* $(,)?) => {
        $crate::Structure<($($crate::With<{ stringify!($name) }, $ty>,)*)>
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_structure_debugs() {
        let s = Structure(());
        assert_eq!(format!("{s:?}"), "Structure");
    }

    #[test]
    fn take_field() {
        let mut s = make_struct!(x: 42_i32, y: true);
        assert_eq!(s.take::<"x", _>(), 42);
        assert!(s.take::<"y", _>());
    }
}
