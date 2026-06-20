// ============================================================
// CDOMER - Parser
// Parser de descida recursiva com precedencia de operadores.
// Converte a sequencia de Tokens em uma AST (Program).
// ============================================================

use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use std::fmt;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Erro de sintaxe [{}:{}]: {}", self.line, self.col, self.message)
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

type PResult<T> = Result<T, ParseError>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_line(&self) -> usize {
        self.tokens[self.pos].line
    }

    fn peek_col(&self) -> usize {
        self.tokens[self.pos].col
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn expect(&mut self, kind: TokenKind) -> PResult<Token> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("esperava {:?}, encontrou {:?}", kind, self.peek()),
                line: self.peek_line(),
                col: self.peek_col(),
            })
        }
    }

    // ---------------- Programa / top-level ----------------

    pub fn parse_program(&mut self) -> PResult<Program> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            match self.peek() {
                TokenKind::Fn => items.push(TopLevel::Fn(self.parse_fn_decl()?)),
                TokenKind::Struct => items.push(TopLevel::Struct(self.parse_struct_decl()?)),
                other => {
                    return Err(ParseError {
                        message: format!("esperava 'fn' ou 'struct' no topo do arquivo, encontrou {:?}", other),
                        line: self.peek_line(),
                        col: self.peek_col(),
                    })
                }
            }
        }
        Ok(Program { items })
    }

    fn parse_type(&mut self) -> PResult<Type> {
        let base = match self.peek().clone() {
            TokenKind::TypeInt => { self.advance(); Type::Int }
            TokenKind::TypeFloat => { self.advance(); Type::Float }
            TokenKind::TypeBool => { self.advance(); Type::Bool }
            TokenKind::TypeString => { self.advance(); Type::String }
            TokenKind::TypeVoid => { self.advance(); Type::Void }
            TokenKind::Ident(name) => { self.advance(); Type::Struct(name) }
            other => {
                return Err(ParseError {
                    message: format!("tipo invalido: {:?}", other),
                    line: self.peek_line(),
                    col: self.peek_col(),
                })
            }
        };
        // suporte a array: int[]
        if self.check(&TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket)?;
            return Ok(Type::Array(Box::new(base)));
        }
        Ok(base)
    }

    fn parse_fn_decl(&mut self) -> PResult<FnDecl> {
        let line = self.peek_line();
        self.expect(TokenKind::Fn)?;
        let name = self.parse_ident_name()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let pname = self.parse_ident_name()?;
                self.expect(TokenKind::Colon)?;
                let ptype = self.parse_type()?;
                params.push(Param { name: pname, ty: ptype });
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            self.parse_type()?
        } else {
            Type::Void
        };

        let body = self.parse_block()?;

        Ok(FnDecl { name, params, return_type, body, line })
    }

    fn parse_struct_decl(&mut self) -> PResult<StructDecl> {
        let line = self.peek_line();
        self.expect(TokenKind::Struct)?;
        let name = self.parse_ident_name()?;
        self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            let fname = self.parse_ident_name()?;
            self.expect(TokenKind::Colon)?;
            let ftype = self.parse_type()?;
            fields.push(StructField { name: fname, ty: ftype });
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(StructDecl { name, fields, line })
    }

    fn parse_ident_name(&mut self) -> PResult<String> {
        match self.peek().clone() {
            TokenKind::Ident(s) => {
                self.advance();
                Ok(s)
            }
            other => Err(ParseError {
                message: format!("esperava identificador, encontrou {:?}", other),
                line: self.peek_line(),
                col: self.peek_col(),
            }),
        }
    }

    // ---------------- Statements ----------------

    fn parse_block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.peek().clone() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Print => self.parse_print_stmt(),
            TokenKind::Break => {
                let line = self.peek_line();
                self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Break(line))
            }
            TokenKind::Continue => {
                let line = self.peek_line();
                self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Continue(line))
            }
            TokenKind::LBrace => {
                let stmts = self.parse_block()?;
                Ok(Stmt::Block(stmts))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_let_stmt(&mut self) -> PResult<Stmt> {
        let line = self.peek_line();
        self.expect(TokenKind::Let)?;
        let name = self.parse_ident_name()?;
        let declared_type = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Assign)?;
        let value = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Let { name, declared_type, value, line })
    }

    fn parse_return_stmt(&mut self) -> PResult<Stmt> {
        let line = self.peek_line();
        self.expect(TokenKind::Return)?;
        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Return { value, line })
    }

    fn parse_if_stmt(&mut self) -> PResult<Stmt> {
        let line = self.peek_line();
        self.expect(TokenKind::If)?;
        self.expect(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        let then_branch = self.parse_block()?;
        let else_branch = if self.check(&TokenKind::Else) {
            self.advance();
            if self.check(&TokenKind::If) {
                let nested = self.parse_if_stmt()?;
                Some(vec![nested])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };
        Ok(Stmt::If { cond, then_branch, else_branch, line })
    }

    fn parse_while_stmt(&mut self) -> PResult<Stmt> {
        let line = self.peek_line();
        self.expect(TokenKind::While)?;
        self.expect(TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::RParen)?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body, line })
    }

    fn parse_for_stmt(&mut self) -> PResult<Stmt> {
        let line = self.peek_line();
        self.expect(TokenKind::For)?;
        self.expect(TokenKind::LParen)?;

        let init = if self.check(&TokenKind::Semicolon) {
            None
        } else if self.check(&TokenKind::Let) {
            Some(self.parse_let_stmt()?)
        } else {
            let e = self.parse_expr()?;
            self.expect(TokenKind::Semicolon)?;
            Some(Stmt::ExprStmt(e))
        };
        if init.is_none() {
            self.expect(TokenKind::Semicolon)?;
        }

        let cond = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(TokenKind::Semicolon)?;

        let step = if self.check(&TokenKind::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(TokenKind::RParen)?;

        let body = self.parse_block()?;

        Ok(Stmt::For { init: Box::new(init), cond, step, body, line })
    }

    fn parse_print_stmt(&mut self) -> PResult<Stmt> {
        let line = self.peek_line();
        self.expect(TokenKind::Print)?;
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Print { args, arg_types: Vec::new(), line })
    }

    // ---------------- Expressions (precedencia) ----------------

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> PResult<Expr> {
        let expr = self.parse_or()?;
        if self.check(&TokenKind::Assign) {
            let line = self.peek_line();
            self.advance();
            let value = self.parse_assignment()?;
            return Ok(Expr::Assign { target: Box::new(expr), value: Box::new(value), line });
        }
        let compound_op = match self.peek() {
            TokenKind::PlusEq => Some(BinOp::Add),
            TokenKind::MinusEq => Some(BinOp::Sub),
            TokenKind::StarEq => Some(BinOp::Mul),
            TokenKind::SlashEq => Some(BinOp::Div),
            _ => None,
        };
        if let Some(op) = compound_op {
            let line = self.peek_line();
            self.advance();
            let rhs = self.parse_assignment()?;
            let combined = Expr::Binary {
                op,
                left: Box::new(expr.clone()),
                right: Box::new(rhs),
                line,
            };
            return Ok(Expr::Assign { target: Box::new(expr), value: Box::new(combined), line });
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut left = self.parse_and()?;
        while self.check(&TokenKind::Or) {
            let line = self.peek_line();
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary { op: BinOp::Or, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut left = self.parse_equality()?;
        while self.check(&TokenKind::And) {
            let line = self.peek_line();
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary { op: BinOp::And, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> PResult<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::NotEq => BinOp::NotEq,
                _ => break,
            };
            let line = self.peek_line();
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> PResult<Expr> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::LtEq,
                TokenKind::GtEq => BinOp::GtEq,
                _ => break,
            };
            let line = self.peek_line();
            self.advance();
            let right = self.parse_term()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> PResult<Expr> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let line = self.peek_line();
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> PResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            let line = self.peek_line();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        match self.peek() {
            TokenKind::Minus => {
                let line = self.peek_line();
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnOp::Neg, expr: Box::new(expr), line })
            }
            TokenKind::Not => {
                let line = self.peek_line();
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(expr), line })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                TokenKind::LBracket => {
                    let line = self.peek_line();
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    expr = Expr::Index { array: Box::new(expr), index: Box::new(index), line };
                }
                TokenKind::Dot => {
                    let line = self.peek_line();
                    self.advance();
                    let field = self.parse_ident_name()?;
                    expr = Expr::FieldAccess { object: Box::new(expr), field, line };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let line = self.peek_line();
        match self.peek().clone() {
            TokenKind::IntLit(n) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            TokenKind::FloatLit(n) => {
                self.advance();
                Ok(Expr::FloatLit(n))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::BoolLit(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::BoolLit(false))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elements.push(self.parse_expr()?);
                        if self.check(&TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::ArrayLit { elements, line })
            }
            TokenKind::Ident(name) => {
                self.advance();
                if self.check(&TokenKind::LBrace) {
                    self.advance();
                    let mut fields = Vec::new();
                    if !self.check(&TokenKind::RBrace) {
                        loop {
                            let fname = self.parse_ident_name()?;
                            self.expect(TokenKind::Colon)?;
                            let fval = self.parse_expr()?;
                            fields.push((fname, fval));
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                    return Ok(Expr::StructLit { name, fields, line });
                }
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    return Ok(Expr::Call { name, args, line });
                }
                Ok(Expr::Ident(name))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(ParseError {
                message: format!("expressao invalida, encontrou {:?}", other),
                line: self.peek_line(),
                col: self.peek_col(),
            }),
        }
    }
}
