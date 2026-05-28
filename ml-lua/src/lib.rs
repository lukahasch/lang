use derive_more::Display;

pub mod error;
pub mod parser;

#[derive(Debug, Clone, PartialEq, Display)]
#[display("{_0} <<{_1}>>")]
pub struct Tagged<V, Tag>(V, Tag);

#[derive(Debug, Clone, PartialEq, Display)]
pub enum Expr<Tag, V> {
    Variable(Tagged<V, Tag>),
    #[display("({_0} + {_1})")]
    Add(Box<Tagged<Self, Tag>>, Box<Tagged<Self, Tag>>),
}
