// ============================================================
// CDOMER - Lexer
// Converte o codigo-fonte (texto) em uma sequencia de Tokens.
// ============================================================

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literais
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    Ident(String),

    // Palavras-chave
    Let,
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    Struct,
    True,
    False,
    Break,
    Continue,
    Print,
    TypeInt,
    TypeFloat,
    TypeBool,
    TypeString,
    TypeVoid,

    // Operadores
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Assign,     // =
    Eq,         // ==
    NotEq,      // !=
    Lt,         // <
    Gt,         // >
    LtEq,       // <=
    GtEq,       // >=
    And,        // &&
    Or,         // ||
    Not,        // !
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    Arrow,      // ->
    Dot,        // .
    Ampersand,  // &

    // Pontuacao
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Comma,      // ,
    Colon,      // :
    Semicolon,  // ;

    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Erro lexico [{}:{}]: {}", self.line, self.col, self.message)
    }
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if let Some(ch) = c {
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    self.advance();
                    self.advance();
                    loop {
                        match self.peek() {
                            None => break,
                            Some('*') if self.peek_at(1) == Some('/') => {
                                self.advance();
                                self.advance();
                                break;
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_number(&mut self) -> Result<TokenKind, LexError> {
        let start = self.pos;
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else if c == '.' && !is_float && self.peek_at(1).map_or(false, |c2| c2.is_ascii_digit()) {
                is_float = true;
                self.advance();
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        if is_float {
            text.parse::<f64>()
                .map(TokenKind::FloatLit)
                .map_err(|_| LexError { message: format!("numero float invalido: {}", text), line: self.line, col: self.col })
        } else {
            text.parse::<i64>()
                .map(TokenKind::IntLit)
                .map_err(|_| LexError { message: format!("numero inteiro invalido: {}", text), line: self.line, col: self.col })
        }
    }

    fn read_ident(&mut self) -> TokenKind {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        match text.as_str() {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "struct" => TokenKind::Struct,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "print" => TokenKind::Print,
            "int" => TokenKind::TypeInt,
            "float" => TokenKind::TypeFloat,
            "bool" => TokenKind::TypeBool,
            "string" => TokenKind::TypeString,
            "void" => TokenKind::TypeVoid,
            _ => TokenKind::Ident(text),
        }
    }

    fn read_string(&mut self) -> Result<TokenKind, LexError> {
        self.advance(); // consome a aspas inicial
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        message: "string nao terminada".to_string(),
                        line: self.line,
                        col: self.col,
                    })
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('0') => s.push('\0'),
                        Some(other) => s.push(other),
                        None => {
                            return Err(LexError {
                                message: "string nao terminada apos escape".to_string(),
                                line: self.line,
                                col: self.col,
                            })
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        Ok(TokenKind::StringLit(s))
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let line = self.line;
            let col = self.col;
            let c = match self.peek() {
                None => {
                    tokens.push(Token { kind: TokenKind::Eof, line, col });
                    break;
                }
                Some(c) => c,
            };

            let kind = if c.is_ascii_digit() {
                self.read_number()?
            } else if c.is_alphabetic() || c == '_' {
                self.read_ident()
            } else if c == '"' {
                self.read_string()?
            } else {
                self.advance();
                match c {
                    '+' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::PlusEq
                        } else {
                            TokenKind::Plus
                        }
                    }
                    '-' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::MinusEq
                        } else if self.peek() == Some('>') {
                            self.advance();
                            TokenKind::Arrow
                        } else {
                            TokenKind::Minus
                        }
                    }
                    '*' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::StarEq
                        } else {
                            TokenKind::Star
                        }
                    }
                    '/' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::SlashEq
                        } else {
                            TokenKind::Slash
                        }
                    }
                    '%' => TokenKind::Percent,
                    '=' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::Eq
                        } else {
                            TokenKind::Assign
                        }
                    }
                    '!' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::NotEq
                        } else {
                            TokenKind::Not
                        }
                    }
                    '<' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::LtEq
                        } else {
                            TokenKind::Lt
                        }
                    }
                    '>' => {
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::GtEq
                        } else {
                            TokenKind::Gt
                        }
                    }
                    '&' => {
                        if self.peek() == Some('&') {
                            self.advance();
                            TokenKind::And
                        } else {
                            TokenKind::Ampersand
                        }
                    }
                    '|' => {
                        if self.peek() == Some('|') {
                            self.advance();
                            TokenKind::Or
                        } else {
                            return Err(LexError {
                                message: "caractere inesperado: '|' (use '||' para OR logico)".to_string(),
                                line,
                                col,
                            });
                        }
                    }
                    '(' => TokenKind::LParen,
                    ')' => TokenKind::RParen,
                    '{' => TokenKind::LBrace,
                    '}' => TokenKind::RBrace,
                    '[' => TokenKind::LBracket,
                    ']' => TokenKind::RBracket,
                    ',' => TokenKind::Comma,
                    ':' => TokenKind::Colon,
                    ';' => TokenKind::Semicolon,
                    '.' => TokenKind::Dot,
                    other => {
                        return Err(LexError {
                            message: format!("caractere inesperado: '{}'", other),
                            line,
                            col,
                        })
                    }
                }
            };

            tokens.push(Token { kind, line, col });
        }
        Ok(tokens)
    }
}
