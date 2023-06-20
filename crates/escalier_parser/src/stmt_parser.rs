use std::iter::Peekable;

use crate::expr_parser::parse_expr;
use crate::lexer::*;
use crate::pattern_parser::parse_pattern;
use crate::source_location::merge_locations;
use crate::source_location::*;
use crate::stmt::{Stmt, StmtKind};
use crate::token::{Token, TokenKind};
use crate::type_ann_parser::parse_type_ann;

const EOF: Token = Token {
    kind: TokenKind::Eof,
    loc: SourceLocation {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 0 },
    },
};

pub fn parse_stmt(lexer: &mut Peekable<Lexer>) -> Stmt {
    let token = lexer.peek().unwrap_or(&EOF).clone();

    match &token.kind {
        TokenKind::Let => {
            lexer.next().unwrap_or(EOF.clone());
            let pattern = parse_pattern(lexer);

            let type_ann = match lexer.peek().unwrap_or(&EOF).kind {
                TokenKind::Colon => {
                    lexer.next().unwrap_or(EOF.clone());
                    Some(parse_type_ann(lexer))
                }
                _ => None,
            };

            assert_eq!(lexer.next().unwrap_or(EOF.clone()).kind, TokenKind::Assign);
            let expr = parse_expr(lexer);
            assert_eq!(
                lexer.next().unwrap_or(EOF.clone()).kind,
                TokenKind::Semicolon
            );

            let loc = merge_locations(&token.loc, &expr.loc);
            Stmt {
                kind: StmtKind::Let {
                    pattern,
                    expr,
                    type_ann,
                },
                loc,
            }
        }
        TokenKind::Return => {
            lexer.next().unwrap_or(EOF.clone());
            let next = lexer.peek().unwrap_or(&EOF).clone();
            match next.kind {
                TokenKind::Semicolon => {
                    lexer.next().unwrap_or(EOF.clone());
                    Stmt {
                        kind: StmtKind::Return { arg: None },
                        loc: merge_locations(&token.loc, &next.loc),
                    }
                }
                _ => {
                    let arg = parse_expr(lexer);
                    assert_eq!(
                        lexer.next().unwrap_or(EOF.clone()).kind,
                        TokenKind::Semicolon
                    );

                    let loc = merge_locations(&next.loc, &arg.loc);
                    Stmt {
                        kind: StmtKind::Return { arg: Some(arg) },
                        loc,
                    }
                }
            }
        }
        _ => {
            let expr = parse_expr(lexer);
            assert_eq!(
                lexer.next().unwrap_or(EOF.clone()).kind,
                TokenKind::Semicolon
            );

            let loc = expr.loc.clone();
            Stmt {
                kind: StmtKind::Expr { expr },
                loc,
            }
        }
    }
}

pub fn parse_program(lexer: &mut Peekable<Lexer>) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    while lexer.peek().unwrap_or(&EOF).kind != TokenKind::Eof {
        stmts.push(parse_stmt(lexer));
    }
    stmts
}

pub fn parse(input: &str) -> Vec<Stmt> {
    let lexer = Lexer::new(input);
    parse_program(&mut lexer.peekable())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_statement() {
        let input = "let x = 5;";
        let stmts = parse(input);
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn single_variable_expression() {
        let input = "x;";
        let stmts = parse(input);
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn multiple_statements() {
        let input = r#"
        let x = 5;
        let y = 10;
        x + y;
        return; 
        "#;

        let stmts = parse(input);

        assert_eq!(stmts.len(), 4);
    }

    #[test]
    fn parse_let() {
        insta::assert_debug_snapshot!(parse(r#"let y = m*x + b;"#));
    }

    #[test]
    fn parse_let_with_type_annotation() {
        insta::assert_debug_snapshot!(parse(r#"let y: number = m*x + b;"#));
    }

    #[test]
    fn parse_let_with_destructuring() {
        insta::assert_debug_snapshot!(parse(r#"let {x, y} = point;"#));
    }

    #[test]
    fn parse_let_with_destructuring_and_type_annotation() {
        insta::assert_debug_snapshot!(parse(r#"let {x, y}: Point = point;"#));
    }

    // TODO: support assignment separate from let decls
    #[test]
    #[ignore]
    fn parse_assignment() {
        insta::assert_debug_snapshot!(parse(r#"y = m*x + b;"#));
    }

    #[test]
    fn parse_conditionals() {
        insta::assert_debug_snapshot!(parse("let max = if (x > y) { x; } else { y; };"));
        insta::assert_debug_snapshot!(parse("if (foo) { console.log(foo); };"));
    }

    #[test]
    fn parse_lambda() {
        insta::assert_debug_snapshot!(parse("let add = fn (x, y) => x + y;"));
        insta::assert_debug_snapshot!(parse("let add = fn (x) => fn (y) => x + y;"));
    }

    #[test]
    fn parse_let_destructuring() {
        insta::assert_debug_snapshot!(parse("let {x, y} = point;"));
        insta::assert_debug_snapshot!(parse("let {x: x1, y: y1} = p1;"));
        insta::assert_debug_snapshot!(parse("let [p1, p2] = line;"));
        insta::assert_debug_snapshot!(parse("let [head, ...tail] = polygon;"));
    }

    #[test]
    fn parse_let_fn_with_fn_type() {
        insta::assert_debug_snapshot!(parse(
            r#"let add: fn (a: number, b: number) => number = fn (a, b) => a + b;"#
        ));
    }
}
