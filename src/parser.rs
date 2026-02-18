use std::fmt;

use crate::ast::{
    BinaryOp, Expr, ExprKind, Function, Param, Program, Span, Stmt, StmtKind, TypeName, UnaryOp,
};
use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl ParserError {
    fn new(message: impl Into<String>, token: &Token) -> Self {
        Self {
            message: message.into(),
            line: token.line,
            column: token.column,
        }
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[parser] {} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for ParserError {}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParserError> {
        let mut functions = Vec::new();
        while !self.is_eof() {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, ParserError> {
        let fn_tok = self.expect_keyword_fn()?;
        let (name, _) = self.expect_identifier("expected function name")?;

        self.expect_lparen("expected `(` after function name")?;
        let mut params = Vec::new();
        if !matches!(self.current().kind, TokenKind::RParen) {
            loop {
                let (param_name, param_span) =
                    self.expect_identifier("expected parameter name")?;
                self.expect_colon("expected `:` after parameter name")?;
                let param_ty = self.parse_type_name()?;
                params.push(Param {
                    name: param_name,
                    ty: param_ty,
                    span: param_span,
                });

                if matches!(self.current().kind, TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.expect_rparen("expected `)` after parameters")?;

        let return_type = if matches!(self.current().kind, TokenKind::Arrow) {
            self.advance();
            self.parse_type_name()?
        } else {
            TypeName::Unit
        };

        let body = self.parse_block_stmt()?;

        Ok(Function {
            name,
            params,
            return_type,
            body,
            span: Span::new(fn_tok.line, fn_tok.column),
        })
    }

    fn parse_type_name(&mut self) -> Result<TypeName, ParserError> {
        match &self.current().kind {
            TokenKind::TypeI64 => {
                self.advance();
                Ok(TypeName::I64)
            }
            TokenKind::TypeBool => {
                self.advance();
                Ok(TypeName::Bool)
            }
            TokenKind::TypeString => {
                self.advance();
                Ok(TypeName::String)
            }
            TokenKind::LParen => {
                let start = self.advance();
                self.expect_rparen("expected `)` in unit type")
                    .map_err(|_| ParserError::new("expected `()` as unit type", &start))?;
                Ok(TypeName::Unit)
            }
            _ => Err(ParserError::new(
                "expected type name (`i64`, `bool`, `string`, or `()`)",
                self.current(),
            )),
        }
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParserError> {
        match &self.current().kind {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::LBrace => self.parse_block_stmt(),
            TokenKind::Identifier(_) if self.peek_is_assign() => self.parse_assign_stmt(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_block_stmt(&mut self) -> Result<Stmt, ParserError> {
        let start = self.expect_lbrace("expected `{`")?;
        let mut statements = Vec::new();

        while !matches!(self.current().kind, TokenKind::RBrace) {
            if self.is_eof() {
                return Err(ParserError::new("unterminated block", self.current()));
            }
            statements.push(self.parse_stmt()?);
        }

        self.expect_rbrace("expected `}`")?;

        Ok(Stmt {
            kind: StmtKind::Block(statements),
            span: Span::new(start.line, start.column),
        })
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParserError> {
        let start = self.advance();
        let mutable = if matches!(self.current().kind, TokenKind::Mut) {
            self.advance();
            true
        } else {
            false
        };

        let (name, _) = self.expect_identifier("expected variable name after `let`")?;
        self.expect_assign("expected `=` in let declaration")?;
        let value = self.parse_expression()?;
        self.expect_semicolon("expected `;` after let declaration")?;

        Ok(Stmt {
            kind: StmtKind::Let {
                name,
                mutable,
                value,
            },
            span: Span::new(start.line, start.column),
        })
    }

    fn parse_assign_stmt(&mut self) -> Result<Stmt, ParserError> {
        let (name, span) = self.expect_identifier("expected assignment target")?;
        self.expect_assign("expected `=` in assignment")?;
        let value = self.parse_expression()?;
        self.expect_semicolon("expected `;` after assignment")?;

        Ok(Stmt {
            kind: StmtKind::Assign { name, value },
            span,
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParserError> {
        let if_tok = self.advance();
        let cond = self.parse_expression()?;
        let then_branch = self.parse_block_stmt()?;

        let else_branch = if matches!(self.current().kind, TokenKind::Else) {
            self.advance();
            let branch = if matches!(self.current().kind, TokenKind::If) {
                self.parse_if_stmt()?
            } else {
                self.parse_block_stmt()?
            };
            Some(Box::new(branch))
        } else {
            None
        };

        Ok(Stmt {
            kind: StmtKind::If {
                cond,
                then_branch: Box::new(then_branch),
                else_branch,
            },
            span: Span::new(if_tok.line, if_tok.column),
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParserError> {
        let while_tok = self.advance();
        let cond = self.parse_expression()?;
        let body = self.parse_block_stmt()?;

        Ok(Stmt {
            kind: StmtKind::While {
                cond,
                body: Box::new(body),
            },
            span: Span::new(while_tok.line, while_tok.column),
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParserError> {
        let ret_tok = self.advance();
        let expr = self.parse_expression()?;
        self.expect_semicolon("expected `;` after return")?;

        Ok(Stmt {
            kind: StmtKind::Return(expr),
            span: Span::new(ret_tok.line, ret_tok.column),
        })
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, ParserError> {
        let expr = self.parse_expression()?;
        self.expect_semicolon("expected `;` after expression")?;

        Ok(Stmt {
            span: expr.span,
            kind: StmtKind::ExprStmt(expr),
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParserError> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_logical_and()?;

        while matches!(self.current().kind, TokenKind::OrOr) {
            let op_tok = self.advance();
            let right = self.parse_logical_and()?;
            expr = Expr {
                span: expr.span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Or,
                    right: Box::new(right),
                },
            };
            if matches!(op_tok.kind, TokenKind::Eof) {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_equality()?;

        while matches!(self.current().kind, TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_equality()?;
            expr = Expr {
                span: expr.span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::And,
                    right: Box::new(right),
                },
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = match self.current().kind {
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::NotEq => BinaryOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            expr = Expr {
                span: expr.span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_term()?;

        loop {
            let op = match self.current().kind {
                TokenKind::Less => BinaryOp::Lt,
                TokenKind::LessEq => BinaryOp::Le,
                TokenKind::Greater => BinaryOp::Gt,
                TokenKind::GreaterEq => BinaryOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            expr = Expr {
                span: expr.span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_factor()?;

        loop {
            let op = match self.current().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            expr = Expr {
                span: expr.span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = match self.current().kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            expr = Expr {
                span: expr.span,
                kind: ExprKind::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                },
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParserError> {
        match self.current().kind {
            TokenKind::Bang => {
                let tok = self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr {
                    span: Span::new(tok.line, tok.column),
                    kind: ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                })
            }
            TokenKind::Minus => {
                let tok = self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr {
                    span: Span::new(tok.line, tok.column),
                    kind: ExprKind::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(expr),
                    },
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> Result<Expr, ParserError> {
        let mut expr = self.parse_primary()?;

        while matches!(self.current().kind, TokenKind::LParen) {
            let call_span = expr.span;
            self.advance(); // (
            let mut args = Vec::new();
            if !matches!(self.current().kind, TokenKind::RParen) {
                loop {
                    args.push(self.parse_expression()?);
                    if matches!(self.current().kind, TokenKind::Comma) {
                        self.advance();
                        continue;
                    }
                    break;
                }
            }
            self.expect_rparen("expected `)` after arguments")?;

            let name = match expr.kind {
                ExprKind::Var(name) => name,
                _ => {
                    return Err(ParserError {
                        message: "only function names can be called".to_string(),
                        line: call_span.line,
                        column: call_span.column,
                    })
                }
            };

            expr = Expr {
                span: call_span,
                kind: ExprKind::Call { name, args },
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParserError> {
        let token = self.advance();
        match token.kind {
            TokenKind::IntLiteral(value) => Ok(Expr {
                kind: ExprKind::Int(value),
                span: Span::new(token.line, token.column),
            }),
            TokenKind::StringLiteral(value) => Ok(Expr {
                kind: ExprKind::String(value),
                span: Span::new(token.line, token.column),
            }),
            TokenKind::True => Ok(Expr {
                kind: ExprKind::Bool(true),
                span: Span::new(token.line, token.column),
            }),
            TokenKind::False => Ok(Expr {
                kind: ExprKind::Bool(false),
                span: Span::new(token.line, token.column),
            }),
            TokenKind::Identifier(name) => Ok(Expr {
                kind: ExprKind::Var(name),
                span: Span::new(token.line, token.column),
            }),
            TokenKind::LParen => {
                let expr = self.parse_expression()?;
                self.expect_rparen("expected `)` after expression")?;
                Ok(expr)
            }
            _ => Err(ParserError::new("expected expression", &token)),
        }
    }

    fn expect_keyword_fn(&mut self) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::Fn) {
            Ok(self.advance())
        } else {
            Err(ParserError::new("expected `fn`", self.current()))
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<(String, Span), ParserError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => Ok((name, Span::new(token.line, token.column))),
            _ => Err(ParserError::new(message, &token)),
        }
    }

    fn expect_lparen(&mut self, message: &str) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::LParen) {
            Ok(self.advance())
        } else {
            Err(ParserError::new(message, self.current()))
        }
    }

    fn expect_rparen(&mut self, message: &str) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::RParen) {
            Ok(self.advance())
        } else {
            Err(ParserError::new(message, self.current()))
        }
    }

    fn expect_lbrace(&mut self, message: &str) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::LBrace) {
            Ok(self.advance())
        } else {
            Err(ParserError::new(message, self.current()))
        }
    }

    fn expect_rbrace(&mut self, message: &str) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::RBrace) {
            Ok(self.advance())
        } else {
            Err(ParserError::new(message, self.current()))
        }
    }

    fn expect_colon(&mut self, message: &str) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::Colon) {
            Ok(self.advance())
        } else {
            Err(ParserError::new(message, self.current()))
        }
    }

    fn expect_assign(&mut self, message: &str) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::Assign) {
            Ok(self.advance())
        } else {
            Err(ParserError::new(message, self.current()))
        }
    }

    fn expect_semicolon(&mut self, message: &str) -> Result<Token, ParserError> {
        if matches!(self.current().kind, TokenKind::Semicolon) {
            Ok(self.advance())
        } else {
            Err(ParserError::new(message, self.current()))
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn is_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }

    fn peek_is_assign(&self) -> bool {
        matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::Assign)
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{BinaryOp, ExprKind, StmtKind};
    use crate::lexer::Lexer;

    use super::Parser;

    fn parse(input: &str) -> crate::ast::Program {
        let tokens = Lexer::new(input).tokenize().expect("lex failed");
        Parser::new(tokens).parse_program().expect("parse failed")
    }

    #[test]
    fn parses_fn_and_return() {
        let program = parse("fn main() -> i64 { return 1; }");
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "main");
    }

    #[test]
    fn parses_operator_precedence() {
        let program = parse("fn main() -> i64 { return 1 + 2 * 3; }");
        let body = &program.functions[0].body;
        let StmtKind::Block(stmts) = &body.kind else {
            panic!("expected block");
        };
        let StmtKind::Return(expr) = &stmts[0].kind else {
            panic!("expected return");
        };
        let ExprKind::Binary { left: _, op, right } = &expr.kind else {
            panic!("expected binary");
        };
        assert_eq!(*op, BinaryOp::Add);
        let ExprKind::Binary { op: rhs_op, .. } = &right.kind else {
            panic!("expected rhs binary");
        };
        assert_eq!(*rhs_op, BinaryOp::Mul);
    }

    #[test]
    fn parses_if_else() {
        let program = parse(
            "fn main() -> i64 { if true { return 1; } else { return 2; } }",
        );
        let StmtKind::Block(stmts) = &program.functions[0].body.kind else {
            panic!("expected block");
        };
        assert!(matches!(stmts[0].kind, StmtKind::If { .. }));
    }
}
