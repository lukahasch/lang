#![feature(decl_macro)]

use std::{marker::PhantomData, sync::atomic::AtomicBool};

use ena::unify::{InPlaceUnificationTable, NoError, UnifyKey, UnifyValue};

#[derive(Debug)]
pub struct TypeID<T>(u32, PhantomData<T>);

#[allow(clippy::non_canonical_clone_impl)]
impl<T> Clone for TypeID<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}
impl<T> Copy for TypeID<T> {}

impl<T: Type> UnifyKey for TypeID<T> {
    type Value = Ty<T>;

    fn from_index(u: u32) -> Self {
        Self(u, PhantomData)
    }

    fn index(&self) -> u32 {
        self.0
    }

    fn tag() -> &'static str {
        "typy"
    }
}

impl<T> PartialEq for TypeID<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl<T> Eq for TypeID<T> {}
#[allow(clippy::non_canonical_partial_ord_impl)]
impl<T> PartialOrd for TypeID<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}
impl<T> Ord for TypeID<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

pub trait Type: Sized + std::fmt::Debug + Clone + PartialEq {
    type Error;
    type Reason;

    fn unify(
        &self,
        other: &Self,
        reason: &Self::Reason,
        typy: &mut Typy<Self>,
    ) -> Result<Self, Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ty<T> {
    Type(T),
    Unknown,
    Set(T),
}

impl<T: Type> UnifyValue for Ty<T> {
    type Error = NoError;

    fn unify_values(value1: &Self, value2: &Self) -> Result<Self, NoError> {
        Ok(match (value1, value2) {
            (Ty::Unknown, a) => a.clone(),
            (a, Ty::Unknown) => a.clone(),
            (Ty::Set(a), Ty::Type(_))
            | (Ty::Type(a), Ty::Set(_))
            | (Ty::Set(a), Ty::Set(_))
            | (Ty::Type(a), Ty::Type(_)) => Ty::Set(a.clone()),
        })
    }
}

pub trait Typed<T> {
    type Match;
    fn get_match(&self) -> &Self::Match;
    fn r#type(&self) -> TypeID<T>;
}

impl<T> Typed<T> for TypeID<T> {
    type Match = ();
    fn get_match(&self) -> &Self::Match {
        &()
    }
    fn r#type(&self) -> TypeID<T> {
        *self
    }
}

pub trait Visit {
    fn visit<C, E>(&self, c: &mut C, f: &impl Fn(&Self, &mut C) -> Result<(), E>) -> Result<(), E>;
}

pub trait Rule<V, T>
where
    V: Typed<T>,
    T: Type,
    Self: Sized,
{
    type Error;
    fn match_on(&self, value: &V, typy: &mut Typy<T>) -> Result<(), Self::Error>;
    fn combine(
        self,
        o: impl Rule<V, T, Error = Self::Error>,
    ) -> impl Rule<V, T, Error = Self::Error> {
        move |value: &V, typy: &mut _| {
            self.match_on(value, typy)?;
            o.match_on(value, typy)
        }
    }
}

impl<V, T, E, F> Rule<V, T> for F
where
    F: Fn(&V, &mut Typy<T>) -> Result<(), E>,
    V: Typed<T>,
    T: Type,
{
    type Error = E;
    fn match_on(&self, value: &V, typy: &mut Typy<T>) -> Result<(), Self::Error> {
        self(value, typy)
    }
}

pub macro rule(|$typy:pat_param| $pat:pat $(if $if:expr)? => |$this:ident $(: $ty:ty)?| $expr:expr) {{
    fn cast_rule<V, T, E>(r: impl Rule<V, T, Error = E>) -> impl Rule<V, T, Error = E>
    where
        V: Typed<T>,
        T: Type,
    {
        r
    }

    cast_rule(|$this $(: $ty)?, $typy: &mut Typy<_>| match &$this.get_match() {
        $pat $(if $if)? => $expr.map(|_| ()),
        _ => Ok(()),
    })
}}

#[derive(Clone)]
pub struct Typy<T: Type> {
    table: InPlaceUnificationTable<TypeID<T>>,
}

static CHANGE: AtomicBool = AtomicBool::new(false);

impl<T: Type> Typy<T> {
    pub fn new() -> Self {
        Self {
            table: InPlaceUnificationTable::new(),
        }
    }

    pub fn root(&mut self, t: &impl Typed<T>) -> TypeID<T> {
        self.table.find(t.r#type())
    }

    pub fn apply<V, R>(&mut self, rule: R, v: &V) -> Result<(), R::Error>
    where
        R: Rule<V, T>,
        V: Typed<T>,
        V: Visit,
    {
        CHANGE.store(true, std::sync::atomic::Ordering::SeqCst);
        let visit = move |value: &V, typy: &mut Self| rule.match_on(value, typy);
        while CHANGE.load(std::sync::atomic::Ordering::SeqCst) {
            CHANGE.store(false, std::sync::atomic::Ordering::SeqCst);
            v.visit(self, &visit)?;
        }
        Ok(())
    }

    pub fn unknown(&mut self) -> TypeID<T> {
        self.table.new_key(Ty::Unknown)
    }

    pub fn known(&mut self, t: T) -> TypeID<T> {
        self.table.new_key(Ty::Type(t))
    }

    pub fn eq(
        &mut self,
        reason: T::Reason,
        a: &impl Typed<T>,
        b: &impl Typed<T>,
    ) -> Result<&mut Self, T::Error> {
        let a = self.table.find(a.r#type());
        let b = self.table.find(b.r#type());
        if a == b {
            return Ok(self);
        }
        let av = self.table.probe_value(a);
        let bv = self.table.probe_value(b);
        let value = match (av, bv) {
            (Ty::Unknown, a) => a,
            (a, Ty::Unknown) => a,
            (Ty::Set(a), Ty::Type(b))
            | (Ty::Type(a), Ty::Set(b))
            | (Ty::Set(a), Ty::Set(b))
            | (Ty::Type(a), Ty::Type(b)) => Ty::Set(a.unify(&b, &reason, self)?),
        };
        log::info!("union {:?} {:?}", a.r#type(), b.r#type());
        self.table.union(a, b);
        self.table.union_value(a, value);
        Ok(self)
    }

    pub fn set(
        &mut self,
        reason: T::Reason,
        var: &impl Typed<T>,
        value: T,
    ) -> Result<&mut Self, T::Error>
    where
        T: std::fmt::Debug,
    {
        let probe = self.table.probe_value(var.r#type());
        if probe == Ty::Type(value.clone()) {
            return Ok(self);
        }
        let value = match (probe, Ty::Type(value)) {
            (Ty::Unknown, a) => a,
            (a, Ty::Unknown) => a,
            (Ty::Set(a), Ty::Type(b))
            | (Ty::Type(a), Ty::Set(b))
            | (Ty::Set(a), Ty::Set(b))
            | (Ty::Type(a), Ty::Type(b)) => Ty::Set(a.unify(&b, &reason, self)?),
        };
        log::info!("union {:?} {value:?}", var.r#type());
        self.table.union_value(var.r#type(), value);
        Ok(self)
    }

    pub fn get(&mut self, var: &impl Typed<T>) -> Option<T> {
        match self.table.probe_value(var.r#type()) {
            Ty::Type(t) => Some(t),
            Ty::Set(t) => Some(t),
            Ty::Unknown => None,
        }
    }
}

impl<T: Type> Default for Typy<T> {
    fn default() -> Self {
        Self::new()
    }
}

/*
 * match self {
 *   Expr::Add(a, b) => {
 *      ctx.eq(a.vist(ctx), b.visit(ctx));
 *      self.r#type  4
 *  }
 * }
 *
 * rule!(|ctx| Spanned(Expr::Add(a, b), span) => ctx.at(span).eq(a, b))
 * rule!(|ctx| Spanned(Expr::Let { r#pattern, r#type, value }, span) => {ctx.at(span).teq(value, r#type)?; r#pattern.type_chk(r#type, ctx) })
 */
