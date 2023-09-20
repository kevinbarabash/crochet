use escalier_ast::*;

use crate::parse_error::ParseError;
use crate::parser::*;
use crate::token::*;

impl<'a> Parser<'a> {
    fn parse_decl(&mut self) -> Result<Decl, ParseError> {
        let token = self.peek().unwrap_or(&EOF).clone();
        let start = token.span.start;

        let item = match &token.kind {
            TokenKind::Let => {
                let token = self.next().unwrap_or(EOF.clone()); // consumes 'let'

                let is_var = token.kind == TokenKind::Var;

                let pattern = self.parse_pattern()?;

                let type_ann = match self.peek().unwrap_or(&EOF).kind {
                    TokenKind::Colon => {
                        self.next().unwrap_or(EOF.clone());
                        Some(self.parse_type_ann()?)
                    }
                    _ => None,
                };

                let expr = match self.peek().unwrap_or(&EOF).kind {
                    TokenKind::Assign => {
                        self.next().unwrap_or(EOF.clone());
                        Some(self.parse_expr()?)
                    }
                    _ => None,
                };

                let span = Span {
                    start,
                    end: if let Some(expr) = &expr {
                        expr.get_span().end
                    } else if let Some(type_ann) = &type_ann {
                        type_ann.span.end
                    } else {
                        pattern.span.end
                    },
                };

                // TODO: check invariants in semantic analysis pass
                Decl {
                    kind: DeclKind::VarDecl(VarDecl {
                        is_declare: false, // TODO
                        is_var,
                        pattern,
                        expr,
                        type_ann,
                    }),
                    span,
                }
            }
            TokenKind::Type => {
                self.next(); // consumes 'type'

                let name = match self.next().unwrap_or(EOF.clone()).kind {
                    TokenKind::Identifier(name) => name,
                    _ => {
                        return Err(ParseError {
                            message: "expected identifier".to_string(),
                        })
                    }
                };

                let type_params = self.maybe_parse_type_params()?;

                assert_eq!(self.next().unwrap_or(EOF.clone()).kind, TokenKind::Assign);
                let type_ann = self.parse_type_ann()?;
                let span = merge_spans(&token.span, &type_ann.span);

                Decl {
                    kind: DeclKind::TypeDecl(TypeDecl {
                        name,
                        type_ann,
                        type_params,
                    }),
                    span,
                }
            }
            _ => {
                return Err(ParseError {
                    message: "expected module item".to_string(),
                })
            }
        };

        Ok(item)
    }

    fn parse_module_item(&mut self) -> Result<ModuleItem, ParseError> {
        // import
        // import {a, b} from "alpha";

        // export
        // 'export' folloed by a decl, there are no default exports
        // 'export' vs 'pub'?
        // the nice thing about 'pub' is you can make other things public like
        // members on structs or classes

        // pub let x = 5
        // pub fn foo() {}

        let token = self.peek().unwrap_or(&EOF).clone();
        let start = token.span;

        let item = match &token.kind {
            TokenKind::Export => {
                self.next(); // consumes 'export'

                let decl = self.parse_decl()?;
                let span = merge_spans(&start, &decl.span);

                ModuleItem {
                    kind: ModuleItemKind::Export(Export { decl }),
                    span,
                }
            }
            _ => {
                let decl = self.parse_decl()?;
                let span = decl.span;

                ModuleItem {
                    kind: ModuleItemKind::Decl(decl),
                    span,
                }
            }
        };

        Ok(item)
    }

    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let mut items = Vec::new();
        while self.peek().unwrap_or(&EOF).kind != TokenKind::Eof {
            // TODO: attach comments to AST nodes
            if let TokenKind::Comment(_) = &self.peek().unwrap_or(&EOF).kind {
                self.next(); // consumes the comment
                continue;
            }
            items.push(self.parse_module_item()?);
        }
        Ok(Module { items })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Vec<ModuleItem> {
        let mut parser = Parser::new(input);
        parser.parse_module().unwrap().items
    }

    #[test]
    fn single_statement() {
        let input = "let x = 5";
        let items = parse(input);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn parse_multiple_decls() {
        insta::assert_debug_snapshot!(parse(
            r#"
            type Point = {x: number, y: number}
            let p: Point = {x: 5, y: 10}
            "#
        ));
    }

    #[test]
    fn parse_exports() {
        insta::assert_debug_snapshot!(parse(
            r#"
            export type Point = {x: number, y: number}
            export let p: Point = {x: 5, y: 10}
            "#
        ));
    }
}
