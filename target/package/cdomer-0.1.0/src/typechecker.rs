// ============================================================
// CDOMER - Type Checker
// Faz a verificacao estatica de tipos com inferencia para
// declaracoes `let` sem tipo explicito. Anota a AST resolvendo
// Type::Unknown sempre que possivel, e produz erros de tipo
// quando ha incompatibilidades.
// ============================================================

use crate::ast::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
    pub line: usize,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Erro de tipo [linha {}]: {}", self.line, self.message)
    }
}

#[derive(Clone)]
struct FnSig {
    params: Vec<Type>,
    return_type: Type,
}

#[derive(Clone)]
struct StructInfo {
    fields: Vec<(String, Type)>,
}

pub struct TypeChecker {
    functions: HashMap<String, FnSig>,
    structs: HashMap<String, StructInfo>,
    scopes: Vec<HashMap<String, Type>>,
    current_return_type: Type,
    loop_depth: usize,
}

type TResult<T> = Result<T, TypeError>;

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            functions: HashMap::new(),
            structs: HashMap::new(),
            scopes: vec![HashMap::new()],
            current_return_type: Type::Void,
            loop_depth: 0,
        }
    }

    fn err(&self, line: usize, msg: impl Into<String>) -> TypeError {
        TypeError { message: msg.into(), line }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare_var(&mut self, name: &str, ty: Type) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), ty);
    }

    fn lookup_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    pub fn check_program(&mut self, program: &mut Program) -> TResult<()> {
        // 1a passada: registra assinaturas de structs e funcoes (permite chamadas mutuamente recursivas / fora de ordem)
        for item in &program.items {
            match item {
                TopLevel::Struct(s) => {
                    let fields = s.fields.iter().map(|f| (f.name.clone(), f.ty.clone())).collect();
                    self.structs.insert(s.name.clone(), StructInfo { fields });
                }
                TopLevel::Fn(f) => {
                    let params = f.params.iter().map(|p| p.ty.clone()).collect();
                    self.functions.insert(f.name.clone(), FnSig { params, return_type: f.return_type.clone() });
                }
            }
        }

        // valida que struct nao referencia tipos de struct inexistentes
        for item in &program.items {
            if let TopLevel::Struct(s) = item {
                for field in &s.fields {
                    self.validate_type_exists(&field.ty, s.line)?;
                }
            }
        }

        // 2a passada: checa corpo de cada funcao
        for item in &mut program.items {
            if let TopLevel::Fn(f) = item {
                self.check_fn(f)?;
            }
        }

        if !self.functions.contains_key("main") {
            return Err(TypeError { message: "funcao 'main' nao encontrada (todo programa CDOMER precisa de uma fn main())".to_string(), line: 0 });
        }

        Ok(())
    }

    fn validate_type_exists(&self, ty: &Type, line: usize) -> TResult<()> {
        match ty {
            Type::Struct(name) => {
                if !self.structs.contains_key(name) {
                    return Err(self.err(line, format!("tipo struct '{}' nao declarado", name)));
                }
                Ok(())
            }
            Type::Array(inner) => self.validate_type_exists(inner, line),
            _ => Ok(()),
        }
    }

    fn check_fn(&mut self, f: &mut FnDecl) -> TResult<()> {
        self.push_scope();
        for p in &f.params {
            self.validate_type_exists(&p.ty, f.line)?;
            self.declare_var(&p.name, p.ty.clone());
        }
        self.current_return_type = f.return_type.clone();
        self.check_block(&mut f.body)?;
        self.pop_scope();
        Ok(())
    }

    fn check_block(&mut self, stmts: &mut Vec<Stmt>) -> TResult<()> {
        for stmt in stmts.iter_mut() {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &mut Stmt) -> TResult<()> {
        match stmt {
            Stmt::Let { name, declared_type, value, line } => {
                let inferred = self.check_expr(value)?;
                let final_type = match declared_type {
                    Some(dt) => {
                        self.validate_type_exists(dt, *line)?;
                        if !self.types_compatible(dt, &inferred) {
                            return Err(self.err(*line, format!(
                                "tipo declarado '{}' nao bate com o tipo do valor '{}' na variavel '{}'",
                                dt, inferred, name
                            )));
                        }
                        dt.clone()
                    }
                    None => inferred,
                };
                *declared_type = Some(final_type.clone());
                self.declare_var(name, final_type);
                Ok(())
            }
            Stmt::ExprStmt(e) => {
                self.check_expr(e)?;
                Ok(())
            }
            Stmt::Return { value, line } => {
                match (value, self.current_return_type.clone()) {
                    (None, Type::Void) => Ok(()),
                    (None, expected) => Err(self.err(*line, format!("funcao espera retornar '{}', mas 'return' nao retorna valor", expected))),
                    (Some(expr), expected) => {
                        let t = self.check_expr(expr)?;
                        if !self.types_compatible(&expected, &t) {
                            return Err(self.err(*line, format!("tipo de retorno incompativel: esperado '{}', encontrado '{}'", expected, t)));
                        }
                        Ok(())
                    }
                }
            }
            Stmt::If { cond, then_branch, else_branch, line } => {
                let ct = self.check_expr(cond)?;
                if ct != Type::Bool {
                    return Err(self.err(*line, format!("condicao do 'if' deve ser bool, encontrado '{}'", ct)));
                }
                self.push_scope();
                self.check_block(then_branch)?;
                self.pop_scope();
                if let Some(eb) = else_branch {
                    self.push_scope();
                    self.check_block(eb)?;
                    self.pop_scope();
                }
                Ok(())
            }
            Stmt::While { cond, body, line } => {
                let ct = self.check_expr(cond)?;
                if ct != Type::Bool {
                    return Err(self.err(*line, format!("condicao do 'while' deve ser bool, encontrado '{}'", ct)));
                }
                self.loop_depth += 1;
                self.push_scope();
                self.check_block(body)?;
                self.pop_scope();
                self.loop_depth -= 1;
                Ok(())
            }
            Stmt::For { init, cond, step, body, line } => {
                self.push_scope();
                if let Some(init_stmt) = init.as_mut() {
                    self.check_stmt(init_stmt)?;
                }
                if let Some(c) = cond {
                    let ct = self.check_expr(c)?;
                    if ct != Type::Bool {
                        return Err(self.err(*line, format!("condicao do 'for' deve ser bool, encontrado '{}'", ct)));
                    }
                }
                if let Some(s) = step {
                    self.check_expr(s)?;
                }
                self.loop_depth += 1;
                self.check_block(body)?;
                self.loop_depth -= 1;
                self.pop_scope();
                Ok(())
            }
            Stmt::Print { args, arg_types, .. } => {
                arg_types.clear();
                for a in args.iter_mut() {
                    let t = self.check_expr(a)?;
                    arg_types.push(t);
                }
                Ok(())
            }
            Stmt::Break(line) => {
                if self.loop_depth == 0 {
                    return Err(self.err(*line, "'break' usado fora de um loop"));
                }
                Ok(())
            }
            Stmt::Continue(line) => {
                if self.loop_depth == 0 {
                    return Err(self.err(*line, "'continue' usado fora de um loop"));
                }
                Ok(())
            }
            Stmt::Block(stmts) => {
                self.push_scope();
                self.check_block(stmts)?;
                self.pop_scope();
                Ok(())
            }
        }
    }

    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if expected == actual {
            return true;
        }
        // int literal pode "promover" para float
        if *expected == Type::Float && *actual == Type::Int {
            return true;
        }
        false
    }

    fn check_expr(&mut self, expr: &mut Expr) -> TResult<Type> {
        match expr {
            Expr::IntLit(_) => Ok(Type::Int),
            Expr::FloatLit(_) => Ok(Type::Float),
            Expr::BoolLit(_) => Ok(Type::Bool),
            Expr::StringLit(_) => Ok(Type::String),
            Expr::Ident(name) => self
                .lookup_var(name)
                .ok_or_else(|| self.err(0, format!("variavel '{}' nao declarada", name))),
            Expr::Unary { op, expr, line } => {
                let t = self.check_expr(expr)?;
                match op {
                    UnOp::Neg => {
                        if t != Type::Int && t != Type::Float {
                            return Err(self.err(*line, format!("operador unario '-' requer int ou float, encontrado '{}'", t)));
                        }
                        Ok(t)
                    }
                    UnOp::Not => {
                        if t != Type::Bool {
                            return Err(self.err(*line, format!("operador unario '!' requer bool, encontrado '{}'", t)));
                        }
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::Binary { op, left, right, line } => {
                let lt = self.check_expr(left)?;
                let rt = self.check_expr(right)?;
                self.check_binop(op, &lt, &rt, *line)
            }
            Expr::Assign { target, value, line } => {
                let tt = self.check_expr(target)?;
                let vt = self.check_expr(value)?;
                if !self.types_compatible(&tt, &vt) {
                    return Err(self.err(*line, format!("nao e possivel atribuir '{}' a variavel do tipo '{}'", vt, tt)));
                }
                Ok(tt)
            }
            Expr::Call { name, args, line } => {
                let line = *line;
                let sig = self
                    .functions
                    .get(name)
                    .cloned()
                    .ok_or_else(|| self.err(line, format!("funcao '{}' nao declarada", name)))?;
                if args.len() != sig.params.len() {
                    return Err(self.err(line, format!(
                        "funcao '{}' espera {} argumento(s), recebeu {}",
                        name, sig.params.len(), args.len()
                    )));
                }
                for (i, arg) in args.iter_mut().enumerate() {
                    let at = self.check_expr(arg)?;
                    if !self.types_compatible(&sig.params[i], &at) {
                        return Err(self.err(line, format!(
                            "argumento {} de '{}': esperado '{}', encontrado '{}'",
                            i + 1, name, sig.params[i], at
                        )));
                    }
                }
                Ok(sig.return_type)
            }
            Expr::ArrayLit { elements, line } => {
                if elements.is_empty() {
                    return Err(self.err(*line, "nao e possivel inferir o tipo de um array vazio; use anotacao de tipo"));
                }
                let first_t = self.check_expr(&mut elements[0])?;
                for el in elements.iter_mut().skip(1) {
                    let t = self.check_expr(el)?;
                    if t != first_t {
                        return Err(self.err(*line, format!("elementos do array com tipos diferentes: '{}' e '{}'", first_t, t)));
                    }
                }
                Ok(Type::Array(Box::new(first_t)))
            }
            Expr::Index { array, index, line } => {
                let at = self.check_expr(array)?;
                let it = self.check_expr(index)?;
                if it != Type::Int {
                    return Err(self.err(*line, format!("indice de array deve ser int, encontrado '{}'", it)));
                }
                match at {
                    Type::Array(inner) => Ok(*inner),
                    other => Err(self.err(*line, format!("nao e possivel indexar valor do tipo '{}'", other))),
                }
            }
            Expr::FieldAccess { object, field, line } => {
                let ot = self.check_expr(object)?;
                match ot {
                    Type::Struct(sname) => {
                        let info = self.structs.get(&sname).cloned().ok_or_else(|| {
                            self.err(*line, format!("struct '{}' nao declarada", sname))
                        })?;
                        info.fields
                            .iter()
                            .find(|(fname, _)| fname == field)
                            .map(|(_, ft)| ft.clone())
                            .ok_or_else(|| self.err(*line, format!("struct '{}' nao possui campo '{}'", sname, field)))
                    }
                    Type::Array(_) if field == "len" => Ok(Type::Int),
                    other => Err(self.err(*line, format!("acesso de campo '.{}' em tipo nao-struct '{}'", field, other))),
                }
            }
            Expr::StructLit { name, fields, line } => {
                let line = *line;
                let info = self
                    .structs
                    .get(name)
                    .cloned()
                    .ok_or_else(|| self.err(line, format!("struct '{}' nao declarada", name)))?;
                if fields.len() != info.fields.len() {
                    return Err(self.err(line, format!(
                        "struct '{}' espera {} campo(s), recebeu {}",
                        name, info.fields.len(), fields.len()
                    )));
                }
                for (fname, fval) in fields.iter_mut() {
                    let expected = info
                        .fields
                        .iter()
                        .find(|(n, _)| n == fname)
                        .map(|(_, t)| t.clone())
                        .ok_or_else(|| self.err(line, format!("struct '{}' nao possui campo '{}'", name, fname)))?;
                    let actual = self.check_expr(fval)?;
                    if !self.types_compatible(&expected, &actual) {
                        return Err(self.err(line, format!(
                            "campo '{}' de '{}': esperado '{}', encontrado '{}'",
                            fname, name, expected, actual
                        )));
                    }
                }
                Ok(Type::Struct(name.clone()))
            }
        }
    }

    fn check_binop(&self, op: &BinOp, lt: &Type, rt: &Type, line: usize) -> TResult<Type> {
        use BinOp::*;
        match op {
            Add | Sub | Mul | Div | Mod => {
                let numeric = |t: &Type| *t == Type::Int || *t == Type::Float;
                if !numeric(lt) || !numeric(rt) {
                    if *op == Add && *lt == Type::String && *rt == Type::String {
                        return Ok(Type::String);
                    }
                    return Err(self.err(line, format!("operador aritmetico requer numeros, encontrado '{}' e '{}'", lt, rt)));
                }
                if lt == rt {
                    Ok(lt.clone())
                } else {
                    // promocao int -> float
                    Ok(Type::Float)
                }
            }
            Eq | NotEq => {
                if lt != rt && !(self.types_compatible(lt, rt) || self.types_compatible(rt, lt)) {
                    return Err(self.err(line, format!("nao e possivel comparar '{}' com '{}'", lt, rt)));
                }
                Ok(Type::Bool)
            }
            Lt | Gt | LtEq | GtEq => {
                let numeric = |t: &Type| *t == Type::Int || *t == Type::Float;
                if !numeric(lt) || !numeric(rt) {
                    return Err(self.err(line, format!("operador de comparacao requer numeros, encontrado '{}' e '{}'", lt, rt)));
                }
                Ok(Type::Bool)
            }
            And | Or => {
                if *lt != Type::Bool || *rt != Type::Bool {
                    return Err(self.err(line, format!("operador logico requer bool, encontrado '{}' e '{}'", lt, rt)));
                }
                Ok(Type::Bool)
            }
        }
    }
}
