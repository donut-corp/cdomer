// ============================================================
// CDOMER - AST (Abstract Syntax Tree)
// Estruturas que representam o programa apos o parsing.
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Void,
    Array(Box<Type>),
    Struct(String),
    Unknown, // usado antes da inferencia de tipo resolver
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Array(t) => write!(f, "{}[]", t),
            Type::Struct(name) => write!(f, "{}", name),
            Type::Unknown => write!(f, "?"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StringLit(String),
    Ident(String),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        line: usize,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
        line: usize,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        line: usize,
    },
    Call {
        name: String,
        args: Vec<Expr>,
        line: usize,
    },
    ArrayLit {
        elements: Vec<Expr>,
        line: usize,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
        line: usize,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        line: usize,
    },
    StructLit {
        name: String,
        fields: Vec<(String, Expr)>,
        line: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        declared_type: Option<Type>,
        value: Expr,
        line: usize,
    },
    ExprStmt(Expr),
    Return {
        value: Option<Expr>,
        line: usize,
    },
    If {
        cond: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        line: usize,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    For {
        init: Box<Option<Stmt>>,
        cond: Option<Expr>,
        step: Option<Expr>,
        body: Vec<Stmt>,
        line: usize,
    },
    Print {
        args: Vec<Expr>,
        arg_types: Vec<Type>,
        line: usize,
    },
    Break(usize),
    Continue(usize),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Vec<Stmt>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum TopLevel {
    Fn(FnDecl),
    Struct(StructDecl),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<TopLevel>,
}
