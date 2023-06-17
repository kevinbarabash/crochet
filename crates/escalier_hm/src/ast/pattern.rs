use generational_arena::Index;

use crate::ast::common::{SourceLocation, Span};
use crate::ast::expr::Expr;
use crate::ast::ident::*;
use crate::ast::Lit;

// TODO: split this into separate patterns:
// - one for assignment (obj, ident, array, rest)
// - one for pattern matching/if let
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternKind {
    // TODO: use Ident instead of BindingIdent, there's no need to
    // have BindingIdent which simply wraps Ident
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
    pub span: Span,
    pub kind: PatternKind,
    pub inferred_type: Option<Index>,
}

impl Pattern {
    pub fn get_name(&self, index: &usize) -> String {
        match &self.kind {
            PatternKind::Ident(BindingIdent { name, .. }) => name.to_owned(),
            _ => format!("arg{index}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LitPat {
    pub lit: Lit,
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
    pub span: Span,
    pub key: Ident,
    pub value: Box<Pattern>,
    pub init: Option<Box<Expr>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShorthandPatProp {
    pub loc: SourceLocation,
    pub span: Span,
    pub ident: BindingIdent,
    pub init: Option<Box<Expr>>,
}

pub fn is_refutable(pat: &Pattern) -> bool {
    match &pat.kind {
        // irrefutable
        PatternKind::Ident(_) => false,
        PatternKind::Rest(_) => false,
        PatternKind::Wildcard => false,

        // refutable
        PatternKind::Lit(_) => true,
        PatternKind::Is(_) => true,

        // refutable if at least one sub-pattern is refutable
        PatternKind::Object(ObjectPat { props, .. }) => props.iter().any(|prop| match prop {
            ObjectPatProp::KeyValue(KeyValuePatProp { value, .. }) => is_refutable(value),
            ObjectPatProp::Shorthand(_) => false, // corresponds to {x} or {x = 5}
            ObjectPatProp::Rest(RestPat { arg, .. }) => is_refutable(arg),
        }),
        PatternKind::Tuple(TuplePat { elems, .. }) => {
            elems.iter().any(|elem| {
                match elem {
                    Some(elem) => is_refutable(&elem.pattern),
                    // FixMe: this should probably be true since it's equivalent
                    // to having an element with the value `undefined`
                    None => false,
                }
            })
        }
    }
}

pub fn is_irrefutable(pat: &Pattern) -> bool {
    !is_refutable(pat)
}

// #[derive(Visitor, Default)]
// #[visitor(BindingIdent(enter))]
// struct BindingCollector {
//     bindings: BTreeSet<String>,
// }

// impl BindingCollector {
//     fn enter_binding_ident(&mut self, binding: &BindingIdent) {
//         self.bindings.insert(binding.name.to_owned());
//     }
// }

// pub fn get_binding(pat: &Pattern) -> BTreeSet<String> {
//     let mut collector = BindingCollector::default();
//     pat.drive(&mut collector);
//     collector.bindings
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::common::DUMMY_LOC;

    fn ident(name: &str) -> Ident {
        Ident {
            loc: DUMMY_LOC,
            span: 0..0,
            name: name.to_owned(),
        }
    }

    fn binding_ident(name: &str) -> BindingIdent {
        BindingIdent {
            loc: DUMMY_LOC,
            span: 0..0,
            name: name.to_owned(),
            mutable: false,
        }
    }

    fn ident_pattern(name: &str) -> Pattern {
        let kind = PatternKind::Ident(BindingIdent {
            loc: DUMMY_LOC,
            name: name.to_owned(),
            mutable: false,
            span: 0..0,
        });
        Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        }
    }

    fn num_lit_pat(value: &str) -> Pattern {
        let kind = PatternKind::Lit(LitPat {
            lit: Lit::num(String::from(value), 0..0, DUMMY_LOC),
        });
        Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        }
    }

    #[test]
    fn ident_is_irrefutable() {
        let ident = ident_pattern("foo");
        assert!(is_irrefutable(&ident));
    }

    #[test]
    fn rest_is_irrefutable() {
        let ident = ident_pattern("foo");
        let kind = PatternKind::Rest(RestPat {
            arg: Box::from(ident),
        });
        let rest = Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        };
        assert!(is_irrefutable(&rest));
    }

    #[test]
    fn obj_with_all_irrefutable_props_is_irrefutable() {
        let kind = PatternKind::Object(ObjectPat {
            props: vec![ObjectPatProp::KeyValue(KeyValuePatProp {
                key: ident("foo"),
                value: Box::from(ident_pattern("foo")),
                init: None,
                loc: DUMMY_LOC,
                span: 0..0,
            })],
            optional: false,
        });
        let obj = Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        };
        assert!(is_irrefutable(&obj));
    }

    #[test]
    fn obj_with_one_refutable_prop_is_refutable() {
        let kind = PatternKind::Object(ObjectPat {
            props: vec![
                ObjectPatProp::KeyValue(KeyValuePatProp {
                    key: ident("foo"),
                    value: Box::from(ident_pattern("foo")),
                    init: None,
                    loc: DUMMY_LOC,
                    span: 0..0,
                }),
                ObjectPatProp::KeyValue(KeyValuePatProp {
                    key: ident("bar"),
                    value: Box::from(num_lit_pat("5")),
                    init: None,
                    loc: DUMMY_LOC,
                    span: 0..0,
                }),
            ],
            optional: false,
        });
        let obj = Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        };
        assert!(is_refutable(&obj));
    }

    #[test]
    fn array_with_all_irrefutable_elements_is_irrefutable() {
        let kind = PatternKind::Tuple(TuplePat {
            elems: vec![Some(TuplePatElem {
                pattern: ident_pattern("foo"),
                init: None,
            })],
            optional: false,
        });
        let array = Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        };
        assert!(is_irrefutable(&array));
    }

    #[test]
    fn array_with_one_refutable_prop_is_refutable() {
        let kind = PatternKind::Tuple(TuplePat {
            elems: vec![
                Some(TuplePatElem {
                    pattern: ident_pattern("foo"),
                    init: None,
                }),
                Some(TuplePatElem {
                    pattern: num_lit_pat("5"),
                    init: None,
                }),
            ],
            optional: false,
        });
        let array = Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        };
        assert!(is_refutable(&array));
    }

    #[test]
    fn literal_pattern_is_refutable() {
        assert!(is_refutable(&num_lit_pat("5")));
    }

    #[test]
    fn is_is_refutable() {
        let kind = PatternKind::Is(IsPat {
            ident: binding_ident("foo"),
            is_id: ident("string"),
        });
        let is_pat = Pattern {
            loc: DUMMY_LOC,
            span: 0..0,
            kind,
            inferred_type: None,
        };
        assert!(is_refutable(&is_pat));
    }
}