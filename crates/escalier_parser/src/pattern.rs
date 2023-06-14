use crate::expr::Expr;
use crate::identifier::{BindingIdent, Ident};
use crate::literal::Literal;
use crate::source_location::SourceLocation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternKind {
    Ident(BindingIdent),
    Rest(RestPat),
    Object(ObjectPat),
    Tuple(TuplePat),
    Lit(LitPat),
    Is(IsPat),
    Wildcard,
    // This can't be used at the top level similar to rest
    // Assign(AssignPat),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    pub loc: SourceLocation,
    // pub span: Span,
    pub kind: PatternKind,
    // pub inferred_type: Option<Index>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LitPat {
    pub lit: Literal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IsPat {
    pub ident: BindingIdent,
    pub is_id: Ident,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestPat {
    pub arg: Box<Pattern>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuplePat {
    // The elements are optional to support sparse arrays.
    pub elems: Vec<Option<TuplePatElem>>,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuplePatElem {
    // TODO: add .span property
    pub pattern: Pattern,
    pub init: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectPat {
    pub props: Vec<ObjectPatProp>,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectPatProp {
    KeyValue(KeyValuePatProp),
    Shorthand(ShorthandPatProp),
    Rest(RestPat), // TODO: create a new RestPatProp that includes a span
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyValuePatProp {
    pub loc: SourceLocation,
    // pub span: Span,
    pub key: Ident,
    pub value: Box<Pattern>,
    pub init: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShorthandPatProp {
    pub loc: SourceLocation,
    // pub span: Span,
    pub ident: BindingIdent,
    pub init: Option<Box<Expr>>,
}