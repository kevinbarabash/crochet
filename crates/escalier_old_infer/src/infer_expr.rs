use derive_visitor::{DriveMut, VisitorMut};
use im::hashmap::HashMap;

use escalier_old_ast::types::{
    self as types, Provenance, TCallable, TFnParam, TKeyword, TLam, TLit, TObjElem, TObject, TPat,
    TPropKey, TRef, TVar, Type, TypeKind,
};
use escalier_old_ast::values::*;

use crate::infer_pattern::PatternUsage;
use crate::substitutable::{Subst, Substitutable};
use crate::type_error::TypeError;
use crate::{util::*, Diagnostic};

use crate::checker::{Checker, Report, ScopeKind};

impl Checker {
    pub fn infer_expr(
        &mut self,
        expr: &mut Expr,
        is_lvalue: bool,
    ) -> Result<(Subst, Type), Vec<TypeError>> {
        self.push_report();
        let result = match &mut expr.kind {
            ExprKind::App(App {
                lam,
                args,
                type_args,
            }) => {
                let mut ss: Vec<Subst> = vec![];
                let mut arg_types: Vec<Type> = vec![];

                let (s1, lam_type) = self.infer_expr(lam, false)?;
                ss.push(s1);

                for arg in args {
                    let (arg_s, mut arg_t) = self.infer_expr(&mut arg.expr, false)?;
                    ss.push(arg_s);
                    if arg.spread.is_some() {
                        match &mut arg_t.kind {
                            TypeKind::Tuple(types) => arg_types.append(types),
                            _ => arg_types
                                .push(self.from_type_kind(TypeKind::Rest(Box::from(arg_t)))),
                        }
                    } else {
                        arg_types.push(arg_t);
                    }
                }

                let ret_type = self.fresh_var(None);
                let type_args = match type_args {
                    Some(type_args) => {
                        let tuples = type_args
                            .iter_mut()
                            .map(|type_arg| self.infer_type_ann(type_arg, &mut None))
                            .collect::<Result<Vec<_>, _>>()?;
                        let (mut subs, types): (Vec<_>, Vec<_>) = tuples.iter().cloned().unzip();
                        ss.append(&mut subs);
                        Some(types)
                    }
                    None => None,
                };

                // Are we missing an `apply()` call here?
                // Maybe, I could see us needing an apply to handle generic functions properly
                // s3       <- unify (apply s2 t1) (TArr t2 tv)
                let mut call_type = self.from_type_kind(TypeKind::App(types::TApp {
                    args: arg_types,
                    ret: Box::from(ret_type.clone()),
                    type_args,
                }));
                call_type.provenance =
                    Some(Box::from(Provenance::Expr(Box::from(expr.to_owned()))));

                let s3 = self.unify(&call_type, &lam_type)?;

                ss.push(s3);

                let s = compose_many_subs(&ss, self);
                let t = ret_type.apply(&s, self);

                // return (s3 `compose` s2 `compose` s1, apply s3 tv)
                Ok((s, t))
            }
            ExprKind::New(New {
                expr,
                args,
                type_args,
            }) => {
                let mut ss: Vec<Subst> = vec![];
                let mut arg_types: Vec<Type> = vec![];

                let (s1, t) = self.infer_expr(expr, false)?;
                ss.push(s1);
                let t = self.get_obj_type(&t)?;

                for arg in args {
                    let (arg_s, mut arg_t) = self.infer_expr(&mut arg.expr, false)?;
                    ss.push(arg_s);
                    if arg.spread.is_some() {
                        match &mut arg_t.kind {
                            TypeKind::Tuple(types) => arg_types.append(types),
                            _ => arg_types
                                .push(self.from_type_kind(TypeKind::Rest(Box::from(arg_t)))),
                        }
                    } else {
                        arg_types.push(arg_t);
                    }
                }

                let mut results: Vec<(Subst, Type, Report)> = vec![];
                // TODO: Try to unify this with the case in `unify()`
                // which tries to unify `App` and `Obj`.
                if let TypeKind::Object(TObject {
                    elems,
                    is_interface: _,
                }) = t.kind
                {
                    for elem in elems {
                        self.push_report();
                        if let TObjElem::Constructor(callable) = &elem {
                            let TCallable {
                                type_params,
                                params,
                                ret,
                            } = callable;

                            // TODO: Check to make sure that we're passing type args
                            // if and only if we need to.  In some cases it's okay
                            // not to pass type args, e.g. new Array(1, 2, 3);
                            let type_param_map: HashMap<String, Type> = match type_params {
                                Some(type_params) => {
                                    if let Some(type_args) = type_args {
                                        let mut type_param_map = HashMap::new();
                                        for (type_param, type_arg) in
                                            type_params.iter().zip(type_args)
                                        {
                                            let (s, t) =
                                                self.infer_type_ann(type_arg, &mut None)?;
                                            ss.push(s);
                                            type_param_map.insert(type_param.name.to_string(), t);
                                        }
                                        type_param_map
                                    } else {
                                        self.get_type_param_map(type_params)
                                    }
                                }
                                None => HashMap::new(),
                            };

                            let lam_type = self.from_type_kind(TypeKind::Lam(TLam {
                                params: params.to_owned(),
                                ret: ret.to_owned(),
                                type_params: None,
                            }));

                            let mut lam_type = replace_aliases_rec(&lam_type, &type_param_map);

                            // let t = generalize(&Env::default(), &lam_type);
                            // let mut lam_type = ctx.instantiate(&t);
                            lam_type.provenance =
                                Some(Box::from(Provenance::TObjElem(Box::from(elem.to_owned()))));

                            let ret_type = self.fresh_var(None);
                            let call_type = self.from_type_kind(TypeKind::App(types::TApp {
                                args: arg_types.clone(),
                                ret: Box::from(ret_type.clone()),
                                type_args: None,
                            }));

                            if let Ok(s3) = self.unify(&call_type, &lam_type) {
                                ss.push(s3);

                                let s = compose_many_subs(&ss.clone(), self);
                                let mut t = ret_type.apply(&s, self);
                                // NOTE: This is only necessary for TypeScript constructors
                                // since they return mutable instances by definition.
                                // We allow immutable objects to be converted to mutable
                                // ones when a new-expression is being assigned to an l-value.
                                // If no type annotation is specified for the l-value, we
                                // want it to be inferred as immutable.
                                t.mutable = false;

                                // return (s3 `compose` s2 `compose` s1, apply s3 tv)
                                results.push((s, t, self.current_report.clone()));
                                self.current_report = vec![]; // reset report
                            }
                        }
                        self.pop_report(); // current_report should be empty when doing this
                    }
                }

                // Sorts the results based on number of free type variables in
                // ascending order.
                results.sort_by(|a, b| {
                    let a_len = a.1.ftv().len();
                    let b_len = b.1.ftv().len();
                    a_len.cmp(&b_len)
                });

                // Pick the result with the lowest number of of free type variables
                match results.get(0) {
                    Some((s, t, report)) => {
                        // NOTE: This is a bit hacky.  We do this because we only
                        // want to report recoverable errors from the result that
                        // we're using.
                        self.push_report();
                        self.current_report = report.to_owned();
                        self.pop_report();
                        Ok((s.to_owned(), t.to_owned()))
                    }
                    None => {
                        // TODO: update this to communicate that we couldn't find a
                        // valid constructor for the given arguments
                        Err(vec![TypeError::Unspecified])
                    }
                }
            }
            ExprKind::Fix(Fix { expr, .. }) => {
                let (s1, t) = self.infer_expr(expr, false)?;
                let tv = self.fresh_var(None);
                let param = TFnParam {
                    pat: TPat::Ident(types::BindingIdent {
                        name: String::from("fix_param"),
                        mutable: false,
                    }),
                    t: tv.clone(),
                    optional: false,
                };
                let lam_t = self.from_type_kind(TypeKind::Lam(types::TLam {
                    params: vec![param],
                    ret: Box::from(tv),
                    type_params: None,
                }));
                let s2 = self.unify(&lam_t, &t)?;

                let s = compose_subs(&s2, &s1, self);
                // This leaves the function param names intact and returns a TLam
                // instead of a TApp.
                let t = match t.kind {
                    TypeKind::Lam(types::TLam { ret, .. }) => Ok(ret.as_ref().to_owned()),
                    _ => Err(vec![TypeError::InvalidFix]),
                }?;

                Ok((s, t))
            }
            ExprKind::Ident(Ident { name, .. }) => {
                let s = Subst::default();
                let t = self.lookup_value(name)?;

                Ok((s, t))
            }
            ExprKind::IfElse(IfElse {
                cond,
                consequent,
                alternate,
                ..
            }) => match alternate {
                Some(alternate) => {
                    match &mut cond.kind {
                        ExprKind::LetExpr(LetExpr { pat, expr, .. }) => {
                            // TODO: warn if the pattern isn't refutable
                            let init = self.infer_expr(expr, false)?;
                            self.push_scope(ScopeKind::Inherit);
                            let s1 = self.infer_pattern_and_init(
                                pat,
                                &mut None,
                                &init,
                                &PatternUsage::Match,
                            )?;

                            let (s2, t2) = self.infer_block(consequent)?;

                            self.pop_scope();

                            let s = compose_subs(&s2, &s1, self);

                            let (s3, t3) = self.infer_block(alternate)?;

                            let s = compose_subs(&s3, &s, self);
                            let t = union_types(&t2, &t3, self);

                            Ok((s, t))
                        }
                        _ => {
                            let (s1, t1) = self.infer_expr(cond, false)?;
                            let (s2, t2) = self.infer_block(consequent)?;
                            let (s3, t3) = self.infer_block(alternate)?;
                            let boolean = self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean));
                            let s4 = self.unify(&t1, &boolean)?;

                            let s = compose_many_subs(&[s1, s2, s3, s4], self);
                            let t = union_types(&t2, &t3, self);

                            Ok((s, t))
                        }
                    }
                }
                None => match &mut cond.kind {
                    ExprKind::LetExpr(LetExpr { pat, expr, .. }) => {
                        let init = self.infer_expr(expr, false)?;
                        self.push_scope(ScopeKind::Inherit);
                        let s1 = self.infer_pattern_and_init(
                            pat,
                            &mut None,
                            &init,
                            &PatternUsage::Match,
                        )?;

                        let (s2, t2) = self.infer_block(consequent)?;

                        self.pop_scope();

                        let s = compose_subs(&s2, &s1, self);

                        let undefined = self.from_type_kind(TypeKind::Keyword(TKeyword::Undefined));
                        let t = union_types(&t2, &undefined, self);

                        Ok((s, t))
                    }
                    _ => {
                        let (s1, t1) = self.infer_expr(cond, false)?;
                        let (s2, t2) = self.infer_block(consequent)?;
                        let boolean = self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean));
                        let s3 = self.unify(&t1, &boolean)?;

                        let s = compose_many_subs(&[s1, s2, s3], self);

                        let undefined = self.from_type_kind(TypeKind::Keyword(TKeyword::Undefined));
                        let t = union_types(&t2, &undefined, self);

                        Ok((s, t))
                    }
                },
            },
            ExprKind::JSXElement(JSXElement {
                name,
                attrs,
                children: _,
                ..
            }) => {
                let first_char = name.chars().next().unwrap();
                // JSXElement's starting with an uppercase char are user defined.
                if first_char.is_uppercase() {
                    let t = self.lookup_value(name)?;
                    match &t.kind {
                        TypeKind::Lam(_) => {
                            let mut ss: Vec<_> = vec![];
                            let mut elems: Vec<_> = vec![];
                            for attr in attrs {
                                let (s, t) = match &mut attr.value {
                                    JSXAttrValue::Lit(lit) => {
                                        let kind = ExprKind::Lit(lit.to_owned());
                                        let mut expr = Expr {
                                            loc: lit.loc(),
                                            span: lit.span(),
                                            kind,
                                            inferred_type: None,
                                        };
                                        self.infer_expr(&mut expr, false)?
                                    }
                                    JSXAttrValue::JSXExprContainer(JSXExprContainer {
                                        expr,
                                        ..
                                    }) => self.infer_expr(expr, false)?,
                                };
                                ss.push(s);

                                let prop = types::TProp {
                                    name: TPropKey::StringKey(attr.ident.name.to_owned()),
                                    optional: false,
                                    mutable: false,
                                    t,
                                };
                                elems.push(types::TObjElem::Prop(prop));
                            }

                            let ret_type = self.from_type_kind(TypeKind::Ref(types::TRef {
                                name: String::from("JSXElement"),
                                type_args: None,
                            }));

                            let arg_type = self.from_type_kind(TypeKind::Object(TObject {
                                elems,
                                is_interface: false,
                            }));
                            let call_type = self.from_type_kind(TypeKind::App(types::TApp {
                                args: vec![arg_type],
                                ret: Box::from(ret_type.clone()),
                                type_args: None,
                            }));

                            let s1 = compose_many_subs(&ss, self);
                            let s2 = self.unify(&call_type, &t)?;

                            let s = compose_subs(&s2, &s1, self);
                            let t = ret_type;

                            return Ok((s, t));
                        }
                        _ => return Err(vec![TypeError::InvalidComponent]),
                    }
                }

                let s = Subst::default();
                // TODO: check props on JSXInstrinsics
                let t = self.from_type_kind(TypeKind::Ref(types::TRef {
                    name: String::from("JSXElement"),
                    type_args: None,
                }));

                Ok((s, t))
            }
            ExprKind::Lambda(Lambda {
                params,
                body,
                is_async,
                return_type: rt_type_ann,
                type_params,
                ..
            }) => {
                self.push_scope(ScopeKind::Inherit);
                self.current_scope.is_async = is_async.to_owned();

                let type_params_map: HashMap<String, Type> = match type_params {
                    Some(type_params) => type_params
                        .iter_mut()
                        .map(|type_param| {
                            let tv = match &mut type_param.constraint {
                                Some(type_ann) => {
                                    // TODO: push `s` on to `ss`
                                    let (_s, t) = self.infer_type_ann(type_ann, &mut None)?;
                                    self.fresh_var(Some(Box::from(t)))
                                }
                                None => self.fresh_var(None),
                            };
                            self.insert_type(type_param.name.name.clone(), tv.clone());
                            Ok((type_param.name.name.to_owned(), tv))
                        })
                        .collect::<Result<HashMap<String, Type>, Vec<TypeError>>>()?,
                    None => HashMap::default(),
                };

                let params: Result<Vec<(Subst, TFnParam)>, Vec<TypeError>> = params
                    .iter_mut()
                    .map(|e_param| self.infer_fn_param(e_param, &type_params_map))
                    .collect();
                let (mut ss, t_params): (Vec<_>, Vec<_>) = params?.iter().cloned().unzip();

                let (body_s, mut body_t) = self.infer_block_or_expr(body)?;
                ss.push(body_s);

                self.pop_scope();

                if *is_async && !is_promise(&body_t) {
                    body_t = self.from_type_kind(TypeKind::Ref(types::TRef {
                        name: String::from("Promise"),
                        type_args: Some(vec![body_t]),
                    }))
                }

                if let Some(rt_type_ann) = rt_type_ann {
                    let (ret_s, ret_t) =
                        self.infer_type_ann_with_params(rt_type_ann, &type_params_map)?;
                    ss.push(ret_s);
                    ss.push(self.unify(&body_t, &ret_t)?);
                }

                let t = self.from_type_kind(TypeKind::Lam(types::TLam {
                    params: t_params,
                    ret: Box::from(body_t.clone()),
                    type_params: None,
                }));

                let s = compose_many_subs(&ss, self);
                let t = t.apply(&s, self);

                // TODO: Update the inferred_type on each param to equal the
                // corresponding type from t_params.

                Ok((s, t))
            }
            ExprKind::Assign(assign) => {
                // TODO:
                // - if left is an identifier look it up to see if it exists in the context
                // - if it does, check if its mutable or not
                if let ExprKind::Ident(id) = &assign.left.kind {
                    let name = &id.name;
                    let binding = self.lookup_binding(name)?;
                    if !binding.mutable {
                        return Err(vec![TypeError::NonMutableBindingAssignment(Box::from(
                            assign.to_owned(),
                        ))]);
                    }
                }

                // This is similar to infer let, but without the type annotation and
                // with pat being an expression instead of a pattern.
                let (rs, rt) = self.infer_expr(&mut assign.right, false)?;
                // TODO: figure out how to get the type of a setter
                let (ls, lt) = self.infer_expr(&mut assign.left, true)?;

                if assign.op != AssignOp::Eq {
                    todo!("handle update assignment operators");
                }

                let s = self.unify(&rt, &lt)?;

                let s = compose_many_subs(&[rs, ls, s], self);
                let t = rt; // This is JavaScript's behavior

                Ok((s, t))
            }
            ExprKind::LetExpr(_) => {
                panic!("Unexpected LetExpr.  All LetExprs should be handled by IfElse arm.")
            }
            ExprKind::Lit(lit) => {
                let s = Subst::new();
                let t = self.from_lit(lit.to_owned());

                Ok((s, t))
            }
            ExprKind::Keyword(keyword) => {
                let s = Subst::new();
                let t = self.from_keyword(keyword.to_owned());

                Ok((s, t))
            }
            ExprKind::BinaryExpr(BinaryExpr {
                op, left, right, ..
            }) => {
                // TODO: check what `op` is and handle comparison operators
                // differently from arithmetic operators
                // TODO: if both are literals, compute the result at compile
                // time and set the result to be appropriate number literal.
                let (s1, t1) = self.infer_expr(left, false)?;
                let (s2, t2) = self.infer_expr(right, false)?;
                let number = self.from_type_kind(TypeKind::Keyword(TKeyword::Number));
                let s3 = match self.unify(&t1, &number) {
                    Ok(s) => s,
                    Err(reasons) => {
                        self.current_report.push(Diagnostic {
                            code: 1,
                            message: format!("{t1} is not a number"),
                            reasons,
                        });
                        Subst::default()
                    }
                };
                let number = self.from_type_kind(TypeKind::Keyword(TKeyword::Number));
                let s4 = match self.unify(&t2, &number) {
                    Ok(s) => s,
                    Err(reasons) => {
                        self.current_report.push(Diagnostic {
                            code: 1,
                            message: format!("{t2} is not a number"),
                            reasons,
                        });
                        Subst::default()
                    }
                };

                let s = compose_many_subs(&[s1, s2, s3, s4], self);

                let t = match (&t1.kind, &t2.kind) {
                    (TypeKind::Lit(TLit::Num(n1)), TypeKind::Lit(TLit::Num(n2))) => {
                        let n1 = match n1.parse::<f64>() {
                            Ok(value) => Ok(value),
                            Err(_) => Err(vec![TypeError::Unspecified]), // Parse the number during parsing
                        }?;
                        let n2 = match n2.parse::<f64>() {
                            Ok(value) => Ok(value),
                            Err(_) => Err(vec![TypeError::Unspecified]), // Parse the number during parsing
                        }?;
                        match op {
                            BinOp::Add => {
                                self.from_type_kind(TypeKind::Lit(TLit::Num((n1 + n2).to_string())))
                            }
                            BinOp::Sub => {
                                self.from_type_kind(TypeKind::Lit(TLit::Num((n1 - n2).to_string())))
                            }
                            BinOp::Mul => {
                                self.from_type_kind(TypeKind::Lit(TLit::Num((n1 * n2).to_string())))
                            }
                            BinOp::Div => {
                                self.from_type_kind(TypeKind::Lit(TLit::Num((n1 / n2).to_string())))
                            }
                            BinOp::EqEq => self.from_type_kind(TypeKind::Lit(TLit::Bool(n1 == n2))),
                            BinOp::NotEq => {
                                self.from_type_kind(TypeKind::Lit(TLit::Bool(n1 != n2)))
                            }
                            BinOp::Gt => self.from_type_kind(TypeKind::Lit(TLit::Bool(n1 > n2))),
                            BinOp::GtEq => self.from_type_kind(TypeKind::Lit(TLit::Bool(n1 >= n2))),
                            BinOp::Lt => self.from_type_kind(TypeKind::Lit(TLit::Bool(n1 < n2))),
                            BinOp::LtEq => self.from_type_kind(TypeKind::Lit(TLit::Bool(n1 <= n2))),
                        }
                    }
                    _ => match op {
                        BinOp::Add => self.from_type_kind(TypeKind::Keyword(TKeyword::Number)),
                        BinOp::Sub => self.from_type_kind(TypeKind::Keyword(TKeyword::Number)),
                        BinOp::Mul => self.from_type_kind(TypeKind::Keyword(TKeyword::Number)),
                        BinOp::Div => self.from_type_kind(TypeKind::Keyword(TKeyword::Number)),
                        BinOp::EqEq => self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean)),
                        BinOp::NotEq => self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean)),
                        BinOp::Gt => self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean)),
                        BinOp::GtEq => self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean)),
                        BinOp::Lt => self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean)),
                        BinOp::LtEq => self.from_type_kind(TypeKind::Keyword(TKeyword::Boolean)),
                    },
                };

                eprintln!("t = {t}");

                Ok((s, t))
            }
            ExprKind::UnaryExpr(UnaryExpr { op, arg, .. }) => {
                let (s1, t1) = self.infer_expr(arg, false)?;
                let number = self.from_type_kind(TypeKind::Keyword(TKeyword::Number));
                let s2 = self.unify(&t1, &number)?;

                let s = compose_many_subs(&[s1, s2], self);
                let t = match op {
                    UnaryOp::Minus => self.from_type_kind(TypeKind::Keyword(TKeyword::Number)),
                };

                Ok((s, t))
            }
            ExprKind::Obj(Obj { props, .. }) => {
                let mut ss: Vec<Subst> = vec![];
                let mut elems: Vec<types::TObjElem> = vec![];
                let mut spread_types: Vec<_> = vec![];
                for p in props {
                    match p {
                        PropOrSpread::Prop(p) => {
                            match p.as_mut() {
                                Prop::Shorthand(Ident { name, .. }) => {
                                    let t = self.lookup_value(name)?;
                                    elems.push(types::TObjElem::Prop(types::TProp {
                                        name: TPropKey::StringKey(name.to_owned()),
                                        optional: false,
                                        mutable: false,
                                        t,
                                    }));
                                }
                                Prop::KeyValue(KeyValueProp { key, value, .. }) => {
                                    let (s, t) = self.infer_expr(value, false)?;
                                    ss.push(s);
                                    // TODO: check if the inferred type is T | undefined and use that
                                    // determine the value of optional
                                    elems.push(types::TObjElem::Prop(types::TProp {
                                        name: TPropKey::StringKey(key.name.to_owned()),
                                        optional: false,
                                        mutable: false,
                                        t,
                                    }));
                                }
                            }
                        }
                        PropOrSpread::Spread(SpreadElement { expr, .. }) => {
                            let (s, t) = self.infer_expr(expr, false)?;
                            ss.push(s);
                            spread_types.push(t);
                        }
                    }
                }

                let s = compose_many_subs(&ss, self);
                let t = if spread_types.is_empty() {
                    self.from_type_kind(TypeKind::Object(TObject {
                        elems,
                        is_interface: false,
                    }))
                } else {
                    let mut all_types = spread_types;
                    all_types.push(self.from_type_kind(TypeKind::Object(TObject {
                        elems,
                        is_interface: false,
                    })));
                    simplify_intersection(&all_types, self)
                };

                Ok((s, t))
            }
            ExprKind::Await(Await { expr, .. }) => {
                if !self.current_scope.is_async {
                    return Err(vec![TypeError::AwaitOutsideOfAsync]);
                }

                let (s1, t1) = self.infer_expr(expr, false)?;
                let inner_t = self.fresh_var(None);
                let promise_t = self.from_type_kind(TypeKind::Ref(types::TRef {
                    name: String::from("Promise"),
                    type_args: Some(vec![inner_t.clone()]),
                }));

                let s2 = self.unify(&t1, &promise_t)?;
                let s = compose_subs(&s2, &s1, self);

                Ok((s, inner_t))
            }
            ExprKind::Tuple(Tuple { elems, .. }) => {
                let mut ss: Vec<Subst> = vec![];
                let mut ts: Vec<Type> = vec![];

                for elem in elems {
                    let expr = elem.expr.as_mut();
                    match elem.spread {
                        Some(_) => {
                            let (s, mut t) = self.infer_expr(expr, false)?;
                            ss.push(s);
                            match &mut t.kind {
                                TypeKind::Tuple(types) => ts.append(types),
                                _ => {
                                    return Err(vec![TypeError::TupleSpreadOutsideTuple]);
                                }
                            }
                        }
                        None => {
                            let (s, t) = self.infer_expr(expr, false)?;
                            ss.push(s);
                            ts.push(t);
                        }
                    }
                }

                let s = compose_many_subs(&ss, self);
                let t = self.from_type_kind(TypeKind::Tuple(ts));

                Ok((s, t))
            }
            ExprKind::Member(Member { obj, prop, .. }) => {
                let (obj_s, mut obj_t) = self.infer_expr(obj, false)?;
                let (prop_s, prop_t) = self.infer_property_type(&mut obj_t, prop, is_lvalue)?;

                let s = compose_subs(&prop_s, &obj_s, self);
                let t = prop_t;

                Ok((s, t))
            }
            ExprKind::Empty => {
                let t = self.from_type_kind(TypeKind::Keyword(TKeyword::Undefined));
                let s = Subst::default();

                Ok((s, t))
            }
            ExprKind::TemplateLiteral(TemplateLiteral {
                exprs, quasis: _, ..
            }) => {
                let t = self.from_type_kind(TypeKind::Keyword(TKeyword::String));
                let result: Result<Vec<(Subst, Type)>, Vec<TypeError>> = exprs
                    .iter_mut()
                    .map(|expr| self.infer_expr(expr, false))
                    .collect();
                // We ignore the types of expressions if there are any because any expression
                // in JavaScript has a string representation.
                let (ss, _): (Vec<_>, Vec<_>) = result?.iter().cloned().unzip();
                let s = compose_many_subs(&ss, self);

                Ok((s, t))
            }
            ExprKind::TaggedTemplateLiteral(_) => {
                // TODO: treat this like a call/application
                // NOTE: requires:
                // - arrays
                // - rest params
                todo!()
            }
            ExprKind::Match(Match { expr, arms, .. }) => {
                // TODO: warn if the pattern isn't refutable
                let mut ss: Vec<Subst> = vec![];
                let mut ts: Vec<Type> = vec![];
                for arm in arms {
                    let init = self.infer_expr(expr, false)?;
                    self.push_scope(ScopeKind::Inherit);
                    let s1 = self.infer_pattern_and_init(
                        &mut arm.pattern,
                        &mut None,
                        &init,
                        &PatternUsage::Match,
                    )?;

                    let (s2, t2) = self.infer_block(&mut arm.body)?;

                    self.pop_scope();

                    let s = compose_subs(&s2, &s1, self);
                    let t = t2;

                    ss.push(s);
                    ts.push(t);
                }

                let s = compose_many_subs(&ss, self);
                let t = union_many_types(&ts, self);

                Ok((s, t))
            }
            ExprKind::Regex(Regex { pattern, flags }) => {
                let s = Subst::default();
                let regex_str_t =
                    self.from_type_kind(TypeKind::Lit(types::TLit::Str(pattern.to_owned())));
                let regex_flag_t =
                    self.from_type_kind(TypeKind::Lit(types::TLit::Str(match flags {
                        Some(flags) => flags.to_owned(),
                        None => "".to_string(),
                    })));
                let t = self.from_type_kind(TypeKind::Ref(TRef {
                    name: "RegExp".to_string(),
                    type_args: Some(vec![regex_str_t, regex_flag_t]),
                }));

                //     TypeKind::Regex(TRegex {
                //     pattern: pattern.to_owned(),
                //     flags: flags.to_owned(),
                // }));

                Ok((s, t))
            }
            // This is only need for classes that are expressions.  Allowing this
            // seems like a bad idea.
            ExprKind::Class(_) => todo!(),
            ExprKind::DoExpr(DoExpr { body }) => self.infer_block(body),
        };

        let (s, mut t) = result?;

        self.apply(&s);

        expr.inferred_type = Some(t.clone());
        t.provenance = Some(Box::from(Provenance::from(expr)));

        self.pop_report();

        Ok((s, t))
    }

    fn infer_property_type(
        &mut self,
        obj_t: &mut Type,
        prop: &mut MemberProp,
        is_lvalue: bool,
    ) -> Result<(Subst, Type), Vec<TypeError>> {
        // TODO: figure out when we have to copy .mutable from `obj_t` to the `t`
        // being returned.
        match &mut obj_t.kind {
            TypeKind::Var(TVar { constraint, .. }) => match constraint {
                Some(constraint) => self.infer_property_type(constraint, prop, is_lvalue),
                None => Err(vec![TypeError::PossiblyNotAnObject(Box::from(
                    obj_t.to_owned(),
                ))]),
            },
            TypeKind::Object(obj) => self.get_prop_value(obj, prop, is_lvalue, obj_t.mutable),
            TypeKind::Ref(_) => {
                let mut t = self.get_obj_type(obj_t)?;
                t.mutable = obj_t.mutable;
                self.infer_property_type(&mut t, prop, is_lvalue)
            }
            TypeKind::Lit(_) => {
                let mut t = self.get_obj_type(obj_t)?;
                self.infer_property_type(&mut t, prop, is_lvalue)
            }
            TypeKind::Keyword(_) => {
                let mut t = self.get_obj_type(obj_t)?;
                self.infer_property_type(&mut t, prop, is_lvalue)
            }
            TypeKind::Array(type_param) => {
                let type_param = type_param.clone();

                let mut t = self.get_obj_type(obj_t)?;
                t.mutable = obj_t.mutable;

                let (s, mut t) = self.infer_property_type(&mut t, prop, is_lvalue)?;

                // Replaces `this` with `mut <type_param>[]`
                let mut rep_t = self.from_type_kind(TypeKind::Array(type_param));
                rep_t.mutable = true;
                replace_this(&mut t, &rep_t);

                Ok((s, t))
            }
            TypeKind::Tuple(elem_types) => {
                // QUESTION: Why don't we need to call `replace_this` here as well?

                // If `prop` is a number literal then look up the index entry, if
                // not, treat it the same as a regular property look up on Array.
                match prop {
                    // TODO: lookup methods on Array.prototype
                    MemberProp::Ident(Ident { name, .. }) => {
                        if name == "length" {
                            let t = self.from_type_kind(TypeKind::Lit(types::TLit::Num(
                                elem_types.len().to_string(),
                            )));
                            let s = Subst::default();
                            return Ok((s, t));
                        }

                        let scheme = self.lookup_scheme("Array")?;

                        let mut type_param_map: HashMap<String, Type> = HashMap::new();
                        let type_param =
                            self.from_type_kind(TypeKind::Union(elem_types.to_owned()));
                        if let Some(type_params) = scheme.type_params {
                            type_param_map.insert(type_params[0].name.to_owned(), type_param);
                        }

                        let mut t = replace_aliases_rec(&scheme.t, &type_param_map);
                        t.mutable = obj_t.mutable;

                        self.infer_property_type(&mut t, prop, is_lvalue)
                    }
                    MemberProp::Computed(ComputedPropName { expr, .. }) => {
                        let (prop_s, prop_t) = self.infer_expr(expr, false)?;

                        match &prop_t.kind {
                            TypeKind::Keyword(TKeyword::Number) => {
                                // TODO: remove duplicate types
                                let mut elem_types = elem_types.to_owned();
                                elem_types.push(
                                    self.from_type_kind(TypeKind::Keyword(TKeyword::Undefined)),
                                );
                                let t = self.from_type_kind(TypeKind::Union(elem_types));
                                Ok((prop_s, t))
                            }
                            TypeKind::Lit(types::TLit::Num(index)) => {
                                let index: usize = index.parse().unwrap();
                                match elem_types.get(index) {
                                    Some(t) => Ok((prop_s, t.to_owned())),
                                    None => Err(vec![TypeError::IndexOutOfBounds(
                                        Box::from(obj_t.to_owned()),
                                        Box::from(prop_t.to_owned()),
                                    )]),
                                }
                            }
                            _ => Err(vec![TypeError::InvalidIndex(
                                Box::from(obj_t.to_owned()),
                                Box::from(prop_t.to_owned()),
                            )]),
                        }
                    }
                }
            }
            TypeKind::Intersection(types) => {
                for t in types {
                    let result = self.infer_property_type(t, prop, is_lvalue);
                    if result.is_ok() {
                        return result;
                    }
                }
                Err(vec![TypeError::Unspecified]) // TODO
            }
            _ => {
                todo!("Unhandled {obj_t:?} in infer_property_type")
            }
        }
    }

    fn get_prop_value(
        &mut self,
        obj: &TObject,
        prop: &mut MemberProp,
        is_lvalue: bool,
        obj_is_mutable: bool,
    ) -> Result<(Subst, Type), Vec<TypeError>> {
        let elems = &obj.elems;

        match prop {
            MemberProp::Ident(Ident { name, .. }) => {
                for elem in elems {
                    match elem {
                        types::TObjElem::Prop(prop) => {
                            if prop.name == TPropKey::StringKey(name.to_owned()) {
                                if is_lvalue && !obj_is_mutable {
                                    return Err(vec![TypeError::ObjectIsNotMutable]);
                                }

                                if is_lvalue && !prop.mutable {
                                    return Err(vec![TypeError::PropertyIsNotMutable]);
                                }

                                let t = get_property_type(prop, self);
                                return Ok((Subst::default(), t));
                            }
                        }
                        TObjElem::Getter(getter) if !is_lvalue => {
                            if getter.name == TPropKey::StringKey(name.to_owned()) {
                                return Ok((Subst::default(), getter.ret.as_ref().to_owned()));
                            }
                        }
                        TObjElem::Setter(setter) if is_lvalue && obj_is_mutable => {
                            if setter.name == TPropKey::StringKey(name.to_owned()) {
                                return Ok((Subst::default(), setter.param.t.to_owned()));
                            }
                        }
                        TObjElem::Method(method) if !is_lvalue => {
                            if method.is_mutating && !obj_is_mutable {
                                return Err(vec![TypeError::MissingKey(name.to_owned())]);
                            }

                            if method.name == TPropKey::StringKey(name.to_owned()) {
                                let t = self.from_type_kind(TypeKind::Lam(types::TLam {
                                    params: method.params.to_owned(),
                                    ret: method.ret.to_owned(),
                                    type_params: method.type_params.to_owned(),
                                }));
                                return Ok((Subst::default(), t));
                            }
                        }
                        _ => (),
                    }
                }

                Err(vec![TypeError::MissingKey(name.to_owned())])
            }
            MemberProp::Computed(ComputedPropName { expr, .. }) => {
                let (prop_s, prop_t) = self.infer_expr(expr, false)?;

                let prop_t_clone = prop_t.clone();
                let prop_s_clone = prop_s.clone();

                let result = match &prop_t.kind {
                    TypeKind::Keyword(TKeyword::String) => {
                        let mut value_types: Vec<Type> = elems
                            .iter()
                            .filter_map(|elem| match elem {
                                // TODO: include index types in the future
                                types::TObjElem::Index(_) => todo!(),
                                types::TObjElem::Prop(prop) => {
                                    // TODO: handle generic object properties
                                    Some(prop.t.to_owned())
                                }
                                types::TObjElem::Getter(_) => todo!(),
                                types::TObjElem::Method(_) => todo!(),
                                _ => None,
                            })
                            .collect();

                        // We can't tell if the property is in the object or not because the
                        // key is a string whose exact value is unknown at compile time.
                        value_types
                            .push(self.from_type_kind(TypeKind::Keyword(TKeyword::Undefined)));
                        let t = self.from_type_kind(TypeKind::Union(value_types));

                        Ok((prop_s, t))
                    }
                    // TODO: handle index types where the computed property key is
                    // a number instead of string.
                    TypeKind::Lit(types::TLit::Str(key)) => {
                        let prop = elems.iter().find_map(|elem| match elem {
                            // TODO: include index types in the future
                            types::TObjElem::Index(_) => None,
                            types::TObjElem::Prop(prop) => {
                                if prop.name == TPropKey::StringKey(key.to_owned()) {
                                    Some(prop)
                                } else {
                                    None
                                }
                            }
                            types::TObjElem::Getter(_) => todo!(),
                            types::TObjElem::Method(_) => todo!(),
                            _ => None,
                        });

                        match prop {
                            Some(prop) => {
                                // TODO: handle generic object properties
                                Ok((Subst::default(), prop.t.to_owned()))
                            }
                            None => Err(TypeError::MissingKey(key.to_owned())),
                        }
                    }
                    _ => Err(TypeError::InvalidKey(Box::from(prop_t.to_owned()))),
                };

                match result {
                    Ok((s, t)) => Ok((s, t)),
                    Err(err) => {
                        let indexers: Vec<_> = elems
                            .iter()
                            .filter_map(|elem| match elem {
                                TObjElem::Index(indexer) => Some(indexer),
                                _ => None,
                            })
                            .collect();

                        if indexers.is_empty() {
                            Err(vec![err])
                        } else {
                            for indexer in indexers {
                                let key_clone = indexer.key.t.clone();
                                let result = self.unify(&prop_t_clone, &key_clone);
                                if result.is_ok() {
                                    let key_s = result?;
                                    let s = compose_subs(&key_s, &prop_s_clone, self);
                                    // TODO: handle generic indexers
                                    // NOTE: Since access any indexer could result in an `undefined`
                                    // we include `| undefined` in the return type here.
                                    let undefined =
                                        self.from_type_kind(TypeKind::Keyword(TKeyword::Undefined));
                                    let t = union_types(&indexer.t, &undefined, self);
                                    return Ok((s, t));
                                }
                            }
                            Err(vec![TypeError::InvalidKey(Box::from(prop_t))])
                        }
                    }
                }
            }
        }
    }
}

fn is_promise(t: &Type) -> bool {
    matches!(&t, Type {kind: TypeKind::Ref(types::TRef { name, .. }), ..} if name == "Promise")
}

#[derive(VisitorMut)]
#[visitor(Type(enter))]
struct ReplaceVisitor {
    rep: Type,
}

impl ReplaceVisitor {
    fn new(t: &Type) -> Self {
        ReplaceVisitor { rep: t.to_owned() }
    }
    fn enter_type(&mut self, t: &mut Type) {
        if let TypeKind::This = t.kind {
            t.kind = self.rep.kind.to_owned();
            t.mutable = self.rep.mutable;
            // TODO: set t.provenance to the original type's kind
        }
    }
}

fn replace_this(t: &mut Type, rep: &Type) {
    let mut rep_visitor = ReplaceVisitor::new(rep);
    t.drive_mut(&mut rep_visitor);
}