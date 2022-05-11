use itertools::join;
use std::fmt;

use super::literal::Literal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    Num,
    Bool,
    Str,
    Undefined,
    Null,
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Primitive::Num => write!(f, "number",),
            Primitive::Bool => write!(f, "boolean"),
            Primitive::Str => write!(f, "string"),
            Primitive::Null => write!(f, "null"),
            Primitive::Undefined => write!(f, "undefined"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TLam {
    pub args: Vec<Type>,
    pub ret: Box<Type>,
}

impl fmt::Display for TLam {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { args, ret } = self;
        write!(f, "({}) => {}", join(args, ", "), ret)
    }
}

#[derive(Clone, Debug)]
pub struct TProp {
    pub name: String,
    pub ty: Type,
}

impl fmt::Display for TProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { name, ty } = self;
        write!(f, "{name}: {ty}")
    }
}

#[derive(Clone, Debug)]
pub enum TypeKind {
    Var,
    Lam(TLam),
    Prim(Primitive),
    Lit(Literal),
    Union(Vec<Type>),
    Obj(Vec<TProp>),
    Alias {
        name: String,
        type_params: Vec<Type>,
    },
}

#[derive(Clone, Debug)]
pub struct Type {
    pub id: i32,
    pub frozen: bool,
    pub kind: TypeKind,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let id = self.id;
        match &self.kind {
            TypeKind::Var => {
                let chars: Vec<_> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
                    .chars()
                    .collect();
                let id = chars.get(id.to_owned() as usize).unwrap();
                write!(f, "{}", id)
            }
            TypeKind::Lam(tlam) => write!(f, "{}", tlam),
            TypeKind::Prim(prim) => write!(f, "{}", prim),
            TypeKind::Lit(lit) => write!(f, "{}", lit),
            TypeKind::Union(types) => write!(f, "{}", join(types, " | ")),
            TypeKind::Obj(props) => write!(f, "{{{}}}", join(props, ", ")),
            TypeKind::Alias { name, type_params } => {
                write!(f, "{name}<{}>", join(type_params, ", "))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Scheme {
    pub qualifiers: Vec<i32>,
    pub ty: Type,
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Scheme { qualifiers, ty } = self;
        let chars: Vec<_> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
            .chars()
            .collect();

        if qualifiers.is_empty() {
            write!(f, "{}", ty)
        } else {
            let mut quals = qualifiers.clone();
            quals.sort();
            write!(
                f,
                "<{}>{}",
                join(
                    quals.iter().map(|id| {
                        let id = chars.get(id.to_owned() as usize).unwrap();
                        format!("{id}")
                    }),
                    ", "
                ),
                ty
            )
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_fmt_literal() {
        assert_eq!(
            format!("{}", Literal::from("hello")),
            String::from("\"hello\"")
        );
        assert_eq!(
            format!("{}", Literal::Num(String::from("5.0"))),
            String::from("5.0")
        );
        assert_eq!(format!("{}", Literal::from(true)), String::from("true"));
    }
}
