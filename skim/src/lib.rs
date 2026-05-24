#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(unsized_const_params)]

pub mod error;
pub mod parser;
pub mod parsers;
pub mod structure;
pub mod syntax;

pub use parser::*;
pub use parsers::*;
pub use structure::*;

use owo_colors::OwoColorize;
use std::fmt::Display;

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
