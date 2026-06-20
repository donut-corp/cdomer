// ============================================================
// CDOMER - Code Generator
// Transpila a AST (ja tipada) para codigo C (C11).
// ============================================================

use crate::ast::*;
use std::collections::HashMap;

pub struct CodeGen {
    out: String,
    indent: usize,
    struct_field_types: HashMap<String, HashMap<String, Type>>,
}

impl CodeGen {
    pub fn new() -> Self {
        CodeGen {
            out: String::new(),
            indent: 0,
            struct_field_types: HashMap::new(),
        }
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.out.push_str(s);
        self.out.push('\n');
    }

    pub fn generate(&mut self, program: &Program) -> String {
        for item in &program.items {
            if let TopLevel::Struct(s) = item {
                let mut m = HashMap::new();
                for f in &s.fields {
                    m.insert(f.name.clone(), f.ty.clone());
                }
                self.struct_field_types.insert(s.name.clone(), m);
            }
        }

        self.out.push_str("/* ============================================================\n");
        self.out.push_str(" * Codigo gerado automaticamente pelo compilador CDOMER\n");
        self.out.push_str(" * Nao edite este arquivo diretamente -- edite o .cdo original\n");
        self.out.push_str(" * ============================================================ */\n\n");
        self.out.push_str("#include <stdio.h>\n");
        self.out.push_str("#include <stdlib.h>\n");
        self.out.push_str("#include <string.h>\n");
        self.out.push_str("#include <stdbool.h>\n\n");

        self.out.push_str("typedef struct { long* data; long len; } cdomer_arr_int;\n");
        self.out.push_str("typedef struct { double* data; long len; } cdomer_arr_float;\n");
        self.out.push_str("typedef struct { bool* data; long len; } cdomer_arr_bool;\n");
        self.out.push_str("typedef struct { char** data; long len; } cdomer_arr_string;\n\n");

        for item in &program.items {
            if let TopLevel::Struct(s) = item {
                self.gen_struct(s);
            }
        }

        for item in &program.items {
            if let TopLevel::Fn(f) = item {
                self.gen_fn_prototype(f);
            }
        }
        self.out.push('\n');

        for item in &program.items {
            if let TopLevel::Fn(f) = item {
                self.gen_fn(f);
                self.out.push('\n');
            }
        }

        self.out.clone()
    }

    fn c_type(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "long".to_string(),
            Type::Float => "double".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "char*".to_string(),
            Type::Void => "void".to_string(),
            Type::Struct(name) => format!("struct {}", name),
            Type::Array(inner) => match **inner {
                Type::Int => "cdomer_arr_int".to_string(),
                Type::Float => "cdomer_arr_float".to_string(),
                Type::Bool => "cdomer_arr_bool".to_string(),
                Type::String => "cdomer_arr_string".to_string(),
                _ => "void*".to_string(),
            },
            Type::Unknown => "void*".to_string(),
        }
    }

    fn gen_struct(&mut self, s: &StructDecl) {
        self.writeln(&format!("struct {} {{", s.name));
        self.indent += 1;
        for field in &s.fields {
            self.writeln(&format!("{} {};", self.c_type(&field.ty), field.name));
        }
        self.indent -= 1;
        self.writeln("};\n");
    }

    fn gen_fn_prototype(&mut self, f: &FnDecl) {
        let params: Vec<String> = f.params.iter().map(|p| format!("{} {}", self.c_type(&p.ty), p.name)).collect();
        let cname = if f.name == "main" { "cdomer_main".to_string() } else { f.name.clone() };
        self.writeln(&format!("{} {}({});", self.c_type(&f.return_type), cname, params.join(", ")));
    }

    fn gen_fn(&mut self, f: &FnDecl) {
        let params: Vec<String> = f.params.iter().map(|p| format!("{} {}", self.c_type(&p.ty), p.name)).collect();
        let cname = if f.name == "main" { "cdomer_main".to_string() } else { f.name.clone() };
        self.writeln(&format!("{} {}({}) {{", self.c_type(&f.return_type), cname, params.join(", ")));
        self.indent += 1;
        for stmt in &f.body {
            self.gen_stmt(stmt);
        }
        self.indent -= 1;
        self.writeln("}");
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, declared_type, value, .. } => {
                let ty = declared_type.clone().unwrap_or(Type::Unknown);
                self.write_indent();
                self.out.push_str(&format!("{} {} = ", self.c_type(&ty), name));
                self.gen_expr(value);
                self.out.push_str(";\n");
            }
            Stmt::ExprStmt(e) => {
                self.write_indent();
                self.gen_expr(e);
                self.out.push_str(";\n");
            }
            Stmt::Return { value, .. } => {
                self.write_indent();
                match value {
                    Some(e) => {
                        self.out.push_str("return ");
                        self.gen_expr(e);
                        self.out.push_str(";\n");
                    }
                    None => self.out.push_str("return;\n"),
                }
            }
            Stmt::If { cond, then_branch, else_branch, .. } => {
                self.write_indent();
                self.out.push_str("if (");
                self.gen_expr(cond);
                self.out.push_str(") {\n");
                self.indent += 1;
                for s in then_branch {
                    self.gen_stmt(s);
                }
                self.indent -= 1;
                self.write_indent();
                self.out.push('}');
                if let Some(eb) = else_branch {
                    self.out.push_str(" else {\n");
                    self.indent += 1;
                    for s in eb {
                        self.gen_stmt(s);
                    }
                    self.indent -= 1;
                    self.write_indent();
                    self.out.push_str("}\n");
                } else {
                    self.out.push('\n');
                }
            }
            Stmt::While { cond, body, .. } => {
                self.write_indent();
                self.out.push_str("while (");
                self.gen_expr(cond);
                self.out.push_str(") {\n");
                self.indent += 1;
                for s in body {
                    self.gen_stmt(s);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            Stmt::For { init, cond, step, body, .. } => {
                self.write_indent();
                self.out.push_str("for (");
                if let Some(init_stmt) = init.as_ref() {
                    self.gen_stmt_inline(init_stmt);
                } else {
                    self.out.push(';');
                }
                self.out.push(' ');
                if let Some(c) = cond {
                    self.gen_expr(c);
                }
                self.out.push_str("; ");
                if let Some(s) = step {
                    self.gen_expr(s);
                }
                self.out.push_str(") {\n");
                self.indent += 1;
                for s in body {
                    self.gen_stmt(s);
                }
                self.indent -= 1;
                self.writeln("}");
            }
            Stmt::Print { args, arg_types, .. } => {
                self.gen_print(args, arg_types);
            }
            Stmt::Break(_) => self.writeln("break;"),
            Stmt::Continue(_) => self.writeln("continue;"),
            Stmt::Block(stmts) => {
                self.writeln("{");
                self.indent += 1;
                for s in stmts {
                    self.gen_stmt(s);
                }
                self.indent -= 1;
                self.writeln("}");
            }
        }
    }

    fn gen_stmt_inline(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, declared_type, value, .. } => {
                let ty = declared_type.clone().unwrap_or(Type::Unknown);
                self.out.push_str(&format!("{} {} = ", self.c_type(&ty), name));
                self.gen_expr(value);
                self.out.push(';');
            }
            Stmt::ExprStmt(e) => {
                self.gen_expr(e);
                self.out.push(';');
            }
            _ => self.out.push(';'),
        }
    }

    fn gen_print(&mut self, args: &[Expr], arg_types: &[Type]) {
        self.write_indent();
        let mut fmt = String::new();
        let mut c_args: Vec<String> = Vec::new();
        for (a, t) in args.iter().zip(arg_types.iter()) {
            let (spec, code) = self.format_spec_and_code(a, t);
            fmt.push_str(&spec);
            fmt.push(' ');
            c_args.push(code);
        }
        let fmt = fmt.trim_end().to_string();
        self.out.push_str(&format!("printf(\"{}\\n\"", fmt));
        for c in c_args {
            self.out.push_str(", ");
            self.out.push_str(&c);
        }
        self.out.push_str(");\n");
    }

    fn format_spec_and_code(&mut self, expr: &Expr, ty: &Type) -> (String, String) {
        let code = self.expr_to_string(expr);
        match ty {
            Type::Int => ("%ld".to_string(), code),
            Type::Float => ("%g".to_string(), code),
            Type::String => ("%s".to_string(), code),
            Type::Bool => ("%s".to_string(), format!("(({}) ? \"true\" : \"false\")", code)),
            Type::Struct(name) => ("%s".to_string(), format!("\"<struct {}>\"", name)),
            Type::Array(_) => ("%s".to_string(), format!("\"<array>\"")),
            Type::Void | Type::Unknown => ("%s".to_string(), "\"<void>\"".to_string()),
        }
    }

    fn expr_to_string(&mut self, expr: &Expr) -> String {
        let saved = std::mem::take(&mut self.out);
        self.gen_expr(expr);
        std::mem::replace(&mut self.out, saved)
    }

    fn gen_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLit(n) => self.out.push_str(&format!("{}L", n)),
            Expr::FloatLit(n) => self.out.push_str(&format!("{}", n)),
            Expr::BoolLit(b) => self.out.push_str(if *b { "true" } else { "false" }),
            Expr::StringLit(s) => self.out.push_str(&format!("\"{}\"", escape_c_string(s))),
            Expr::Ident(name) => self.out.push_str(name),
            Expr::Unary { op, expr, .. } => {
                match op {
                    UnOp::Neg => self.out.push('-'),
                    UnOp::Not => self.out.push('!'),
                }
                self.out.push('(');
                self.gen_expr(expr);
                self.out.push(')');
            }
            Expr::Binary { op, left, right, .. } => {
                self.out.push('(');
                self.gen_expr(left);
                self.out.push_str(&format!(" {} ", binop_to_c(op)));
                self.gen_expr(right);
                self.out.push(')');
            }
            Expr::Assign { target, value, .. } => {
                self.gen_expr(target);
                self.out.push_str(" = ");
                self.gen_expr(value);
            }
            Expr::Call { name, args, .. } => {
                self.out.push_str(name);
                self.out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.gen_expr(a);
                }
                self.out.push(')');
            }
            Expr::ArrayLit { elements, .. } => {
                let elem_ty = self.guess_array_elem_type(elements);
                let c_elem = self.c_type(&elem_ty);
                let arr_ty = self.c_type(&Type::Array(Box::new(elem_ty)));
                self.out.push_str(&format!("({}){{ .data = ({}[]){{ ", arr_ty, c_elem));
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.gen_expr(e);
                }
                self.out.push_str(&format!(" }}, .len = {} }}", elements.len()));
            }
            Expr::Index { array, index, .. } => {
                self.gen_expr(array);
                self.out.push_str(".data[");
                self.gen_expr(index);
                self.out.push(']');
            }
            Expr::FieldAccess { object, field, .. } => {
                self.gen_expr(object);
                self.out.push('.');
                self.out.push_str(field);
            }
            Expr::StructLit { name, fields, .. } => {
                self.out.push_str(&format!("(struct {}){{ ", name));
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.out.push_str(&format!(".{} = ", fname));
                    self.gen_expr(fval);
                }
                self.out.push_str(" }");
            }
        }
    }

    fn guess_array_elem_type(&self, elements: &[Expr]) -> Type {
        match elements.first() {
            Some(Expr::IntLit(_)) => Type::Int,
            Some(Expr::FloatLit(_)) => Type::Float,
            Some(Expr::BoolLit(_)) => Type::Bool,
            Some(Expr::StringLit(_)) => Type::String,
            _ => Type::Int,
        }
    }
}

fn binop_to_c(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

fn escape_c_string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out
}

/// Gera o `main` real do C que chama `cdomer_main`.
pub fn gen_c_main_wrapper() -> String {
    "int main(void) {\n    cdomer_main();\n    return 0;\n}\n".to_string()
}
