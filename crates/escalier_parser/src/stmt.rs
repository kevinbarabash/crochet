use crate::expr::Expr;
use crate::pattern::Pattern;
use crate::span::Span;
use crate::type_ann::TypeAnn;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum StmtKind {
    Expr {
        expr: Expr,
    },
    Let {
        pattern: Pattern,
        expr: Expr,
        type_ann: Option<TypeAnn>,
    },
    Return {
        arg: Option<Expr>,
    },
    // TODO:
    // - explicit type annotations
    // - function decls: `fn foo() {}` desugars to `let foo = fn () {}`
    // - class decls: `class Foo {}` desugars to `let Foo = class {}`
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}
