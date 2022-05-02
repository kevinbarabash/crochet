use itertools::Itertools;
use std::fmt;

use super::super::literal::Literal;
use super::super::types::{Primitive, TVar};

pub struct TsQualifiedType {
    pub ty: TsType,
    pub type_params: Vec<i32>,
}

#[derive(Debug)]
pub enum TsType {
    Prim(Primitive),
    Var(TVar),
    Lit(Literal),
    Func {
        params: Vec<Param>,
        ret: Box<TsType>,
    },
}

impl fmt::Display for TsQualifiedType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.type_params.len() > 0 {
            let type_params = self.type_params.iter().join(", ");
            write!(f, "<{type_params}>{}", self.ty)
        } else {
            write!(f, "{}", self.ty)
        }
    }
}

impl fmt::Display for TsType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TsType::Var(tv) => write!(f, "{}", tv),
            TsType::Prim(prim) => write!(f, "{}", prim),
            TsType::Lit(lit) => write!(f, "{}", lit),
            TsType::Func { params, ret } => {
                let params = params
                    .iter()
                    // TODO: use write! to format the params more directly instead of
                    // using the intermediary format!
                    .map(|Param { name, ty }| format!("{name}: {ty}").to_owned())
                    .join(", ");
                write!(f, "({params}) => {ret}")
            }
        }
    }
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub ty: TsType,
}
