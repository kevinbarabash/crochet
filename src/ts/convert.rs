use super::super::syntax::{BindingIdent, Expr};
use super::super::types::{Scheme, TLam, Type};
use super::ast::{Param, TsQualifiedType, TsType};

pub fn extend_scheme(scheme: &Scheme, expr: Option<&Expr>) -> TsQualifiedType {
    TsQualifiedType {
        ty: extend_type(&scheme.ty, expr),
        type_params: scheme.qualifiers.clone(),
    }
}

// TODO: update this to accept a Scheme instead of Type
pub fn extend_type(ty: &Type, expr: Option<&Expr>) -> TsType {
    match ty {
        Type::Var(tvar) => TsType::Var(tvar.to_owned()),
        Type::Prim(prim) => TsType::Prim(prim.to_owned()),
        Type::Lit(lit) => TsType::Lit(lit.to_owned()),
        // This is used to copy the names of args from the expression
        // over to the lambda's type.
        Type::Lam(TLam { args, ret }) => {
            match expr {
                // TODO: handle is_async
                Some(Expr::Lam {
                    args: expr_args, ..
                }) => {
                    if args.len() != expr_args.len() {
                        panic!("number of args don't match")
                    } else {
                        let params: Vec<_> = args
                            .iter()
                            .zip(expr_args)
                            .map(|(arg, (binding, _span))| {
                                let name = match binding {
                                    BindingIdent::Ident { name } => name,
                                    BindingIdent::Rest { name } => name,
                                };
                                Param {
                                    name: name.to_owned(),
                                    ty: extend_type(arg, None),
                                }
                            })
                            .collect();

                        TsType::Func {
                            params,
                            ret: Box::new(extend_type(&ret, None)),
                        }
                    }
                },
                // Fix nodes are assumed to wrap a lambda where the body of
                // the lambda is recursive function.
                Some(Expr::Fix { expr }) => {
                    match expr.as_ref() {
                        (Expr::Lam {body, ..}, _) => {
                            extend_type(ty, Some(&body.0))
                        },
                        _ => panic!("mismatch")
                    }
                },
                None => {
                    let params: Vec<_> = args
                        .iter()
                        .enumerate()
                        .map(|(i, arg)| Param {
                            name: format!("arg{}", i),
                            ty: extend_type(arg, None),
                        })
                        .collect();
                    TsType::Func {
                        params,
                        ret: Box::new(extend_type(&ret, None)),
                    }
                },
                _ => panic!("mismatch"),
            }
        }
    }
}
