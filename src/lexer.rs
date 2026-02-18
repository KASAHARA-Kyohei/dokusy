use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    IntLiteral(i64),
    StringLiteral(String),

    Let,
    Mut,
    Fn,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Match,
    TypeI64,
    TypeBool,
    TypeString,

    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Arrow,

    Assign,
    EqEq,
    NotEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    AndAnd,
    OrOr,
    Plus,
    Minus,
    Star,
    Slash,
    Bang,

    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl LexerError {
    fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[lexer] {} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for LexerError {}

pub struct Lexer<'a> {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    _source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            _source: source,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = Vec::new();

        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.advance();
                continue;
            }

            if c == '/' {
                if self.peek_char() == Some('/') {
                    self.skip_comment();
                    continue;
                }
            }

            let line = self.line;
            let column = self.column;

            let token = match c {
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '{' => {
                    self.advance();
                    TokenKind::LBrace
                }
                '}' => {
                    self.advance();
                    TokenKind::RBrace
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                ';' => {
                    self.advance();
                    TokenKind::Semicolon
                }
                ':' => {
                    if self.peek_char() == Some(':') {
                        return Err(LexerError::new(
                            "`::` is not supported in dokusy",
                            line,
                            column,
                        ));
                    }
                    self.advance();
                    TokenKind::Colon
                }
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '-' => {
                    if self.peek_char() == Some('>') {
                        self.advance();
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        self.advance();
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
                }
                '!' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        self.advance();
                        TokenKind::Bang
                    }
                }
                '=' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.advance();
                        TokenKind::EqEq
                    } else {
                        self.advance();
                        TokenKind::Assign
                    }
                }
                '<' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.advance();
                        TokenKind::LessEq
                    } else {
                        self.advance();
                        TokenKind::Less
                    }
                }
                '>' => {
                    if self.peek_char() == Some('=') {
                        self.advance();
                        self.advance();
                        TokenKind::GreaterEq
                    } else {
                        self.advance();
                        TokenKind::Greater
                    }
                }
                '&' => {
                    if self.peek_char() == Some('&') {
                        self.advance();
                        self.advance();
                        TokenKind::AndAnd
                    } else {
                        return Err(LexerError::new(
                            "`&` is not supported in dokusy",
                            line,
                            column,
                        ));
                    }
                }
                '|' => {
                    if self.peek_char() == Some('|') {
                        self.advance();
                        self.advance();
                        TokenKind::OrOr
                    } else {
                        return Err(LexerError::new("unexpected `|`", line, column));
                    }
                }
                '"' => {
                    let value = self.lex_string()?;
                    TokenKind::StringLiteral(value)
                }
                d if d.is_ascii_digit() => {
                    let value = self.lex_number()?;
                    TokenKind::IntLiteral(value)
                }
                ident_start if is_identifier_start(ident_start) => {
                    let ident = self.lex_identifier();
                    keyword_or_identifier(&ident)
                }
                _ => {
                    return Err(LexerError::new(
                        format!("unexpected character `{}`", c),
                        line,
                        column,
                    ));
                }
            };

            tokens.push(Token {
                kind: token,
                line,
                column,
            });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            column: self.column,
        });

        Ok(tokens)
    }

    fn current_char(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.index + 1).copied()
    }

    fn advance(&mut self) {
        if let Some(c) = self.current_char() {
            self.index += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.current_char() {
            self.advance();
            if c == '\n' {
                break;
            }
        }
    }

    fn lex_identifier(&mut self) -> String {
        let mut ident = String::new();
        while let Some(c) = self.current_char() {
            if is_identifier_continue(c) {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }
        ident
    }

    fn lex_number(&mut self) -> Result<i64, LexerError> {
        let start_line = self.line;
        let start_col = self.column;
        let mut value = String::new();
        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        value.parse::<i64>().map_err(|_| {
            LexerError::new(
                format!("invalid i64 literal `{}`", value),
                start_line,
                start_col,
            )
        })
    }

    fn lex_string(&mut self) -> Result<String, LexerError> {
        let start_line = self.line;
        let start_col = self.column;

        // opening quote
        self.advance();

        let mut value = String::new();
        while let Some(c) = self.current_char() {
            match c {
                '"' => {
                    self.advance();
                    return Ok(value);
                }
                '\\' => {
                    self.advance();
                    let escaped = self.current_char().ok_or_else(|| {
                        LexerError::new("unterminated string literal", start_line, start_col)
                    })?;
                    match escaped {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        'n' => value.push('\n'),
                        _ => {
                            return Err(LexerError::new(
                                format!("unsupported escape `\\{}`", escaped),
                                self.line,
                                self.column,
                            ))
                        }
                    }
                    self.advance();
                }
                '\n' => {
                    return Err(LexerError::new(
                        "unterminated string literal",
                        start_line,
                        start_col,
                    ));
                }
                other => {
                    value.push(other);
                    self.advance();
                }
            }
        }

        Err(LexerError::new(
            "unterminated string literal",
            start_line,
            start_col,
        ))
    }
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn keyword_or_identifier(value: &str) -> TokenKind {
    match value {
        "let" => TokenKind::Let,
        "mut" => TokenKind::Mut,
        "fn" => TokenKind::Fn,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "return" => TokenKind::Return,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "match" => TokenKind::Match,
        "i64" => TokenKind::TypeI64,
        "bool" => TokenKind::TypeBool,
        "string" => TokenKind::TypeString,
        _ => TokenKind::Identifier(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, TokenKind};

    #[test]
    fn tokenizes_let_and_math() {
        let src = "let mut x = 1 + 2; // comment\n";
        let tokens = Lexer::new(src).tokenize().expect("lex failed");
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Let,
                TokenKind::Mut,
                TokenKind::Identifier("x".to_string()),
                TokenKind::Assign,
                TokenKind::IntLiteral(1),
                TokenKind::Plus,
                TokenKind::IntLiteral(2),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tokenizes_string_escape() {
        let src = "let s = \"a\\n\\\"b\\\\\";";
        let tokens = Lexer::new(src).tokenize().expect("lex failed");
        assert_eq!(tokens[3].kind, TokenKind::StringLiteral("a\n\"b\\".to_string()));
    }

    #[test]
    fn rejects_ampersand() {
        let src = "let x = &1;";
        let err = Lexer::new(src).tokenize().expect_err("must fail");
        assert!(err.message.contains("`&` is not supported"));
    }
}
