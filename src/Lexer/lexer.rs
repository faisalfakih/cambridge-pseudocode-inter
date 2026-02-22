use crate::errortype::{CPSError, ErrorType};
use std::collections::HashMap;

// TODO: Add error when user types a keyword which hasn't been implemented yet.
// TODO: Fix issue where comment ends before new line

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenType {
    // KEYWORDS
    And,
    Append, 
    Array, 
    Boolean, 
    ByRef,
    ByVal,
    Call,
    Case,
    Of,
    Char,  
    Class,
    CloseFile,
    Constant, 
    Date,
    Declare, 
    Div, 
    Else,
    EndCase,
    EndClass, 
    EndFunction, 
    EndIf,
    EndProcedure,
    EndType,
    EndWhile,
    False,
    For,
    To,
    Function,
    GetRecord,
    If,
    Inherits,
    Input, 
    Integer,
    Mod,
    Next,
    New,
    Not,
    OpenFile,
    Or,
    Otherwise, 
    Output,
    Procedure,
    Private,
    Public,
    PutRecord,
    Random,
    Read,
    ReadFile,
    Real,
    Repeat,
    Return, 
    Returns,
    Seek,
    Step,
    String,
    Super,
    Then,
    True,
    Type,
    Until,
    While,
    Write,
    WriteFile,

    // IDENTIFIERS AND LITERALS
    Identifier,
    NumberLiteral,
    StringLiteral,
    CharLiteral,

    // OPERATORS
    Minus,
    Arrow,
    Asterisk,
    ForwardSlash,
    Comment,
    Plus,
    LessThan,
    LessEqual,
    NotEqual,
    Equal,
    GreaterThan,
    GreaterEqual,
    Caret,
    Ampersand,
    Colon,
    LParen,
    RParen,
    LSquare,
    RSquare,
    Comma,

    Eof,
}


#[derive(Debug, Clone)]
pub struct Token {
    pub lexeme: String,
    pub token_type: TokenType,
    pub line: usize,
    pub column: usize,
    error: Option<CPSError>,
}

impl Token {
    pub fn new(lexeme: String, token_type: TokenType, line: usize, column: usize) -> Self {
        Token {
            lexeme,
            token_type,
            line,
            column,
            error: None,
        }
    }

}

#[derive(Debug, Clone)]
pub struct Lexer {
    source: String,
    position: usize,
    line: usize,
    column: usize,
    token_map: HashMap<String, TokenType>,
}


impl Lexer {
    pub fn new(source: String) -> Self {
        let token_map: HashMap<String, TokenType> = [
            // Keywords A-C
            ("AND".to_string(), TokenType::And),
            ("APPEND".to_string(), TokenType::Append),
            ("ARRAY".to_string(), TokenType::Array),
            ("BOOLEAN".to_string(), TokenType::Boolean),
            ("BYREF".to_string(), TokenType::ByRef),
            ("BYVAL".to_string(), TokenType::ByVal),
            ("CALL".to_string(), TokenType::Call),
            ("CASE".to_string(), TokenType::Case),
            ("CHAR".to_string(), TokenType::Char),
            ("CLASS".to_string(), TokenType::Class),
            ("CLOSEFILE".to_string(), TokenType::CloseFile),
            ("CONSTANT".to_string(), TokenType::Constant),

            // Keywords D-F
            ("DATE".to_string(), TokenType::Date),
            ("DECLARE".to_string(), TokenType::Declare),
            ("DIV".to_string(), TokenType::Div),
            ("ELSE".to_string(), TokenType::Else),
            ("ENDCASE".to_string(), TokenType::EndCase),
            ("ENDCLASS".to_string(), TokenType::EndClass),
            ("ENDFUNCTION".to_string(), TokenType::EndFunction),
            ("ENDIF".to_string(), TokenType::EndIf),
            ("ENDPROCEDURE".to_string(), TokenType::EndProcedure),
            ("ENDTYPE".to_string(), TokenType::EndType),
            ("ENDWHILE".to_string(), TokenType::EndWhile),
            ("FALSE".to_string(), TokenType::False),
            ("FOR".to_string(), TokenType::For),
            ("FUNCTION".to_string(), TokenType::Function),

            // Keywords G-L
            ("GETRECORD".to_string(), TokenType::GetRecord),
            ("IF".to_string(), TokenType::If),
            ("INHERITS".to_string(), TokenType::Inherits),
            ("INPUT".to_string(), TokenType::Input),
            ("INTEGER".to_string(), TokenType::Integer),

            // Keywords M-O
            ("MOD".to_string(), TokenType::Mod),
            ("NEW".to_string(), TokenType::New),
            ("NEXT".to_string(), TokenType::Next),
            ("NOT".to_string(), TokenType::Not),
            ("OF".to_string(), TokenType::Of),
            ("OPENFILE".to_string(), TokenType::OpenFile),
            ("OR".to_string(), TokenType::Or),
            ("OTHERWISE".to_string(), TokenType::Otherwise),
            ("OUTPUT".to_string(), TokenType::Output),

            // Keywords P-R
            ("PRINT".to_string(), TokenType::Output), // alias for OUTPUT
            ("PRIVATE".to_string(), TokenType::Private),
            ("PROCEDURE".to_string(), TokenType::Procedure),
            ("PUBLIC".to_string(), TokenType::Public),
            ("PUTRECORD".to_string(), TokenType::PutRecord),
            ("RANDOM".to_string(), TokenType::Random),
            ("READ".to_string(), TokenType::Read),
            ("READFILE".to_string(), TokenType::ReadFile),
            ("REAL".to_string(), TokenType::Real),
            ("REPEAT".to_string(), TokenType::Repeat),
            ("RETURN".to_string(), TokenType::Return),
            ("RETURNS".to_string(), TokenType::Returns),

            // Keywords S-U
            ("SEEK".to_string(), TokenType::Seek),
            ("STEP".to_string(), TokenType::Step),
            ("STRING".to_string(), TokenType::String),
            ("SUPER".to_string(), TokenType::Super),
            ("THEN".to_string(), TokenType::Then),
            ("TO".to_string(), TokenType::To),
            ("TRUE".to_string(), TokenType::True),
            ("TYPE".to_string(), TokenType::Type),
            ("UNTIL".to_string(), TokenType::Until),

            // Keywords W
            ("WHILE".to_string(), TokenType::While),
            ("WRITE".to_string(), TokenType::Write),
            ("WRITEFILE".to_string(), TokenType::WriteFile),
        ].iter().cloned().collect();

        Lexer {
            source,
            position: 0,
            line: 1,
            column: 1,
            token_map,
        }
    }

    // helper functions 
    fn peek(&self, n: usize) -> Option<char> {
        self.source.chars().nth(self.position + n)
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, CPSError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.get_next_token()?;
            if token.token_type == TokenType::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn get_next_token(&mut self) -> Result<Token, CPSError> {

        // tokenize symbols and literals first
        // iterate through source code character by character
        for _ in self.source.chars() {
            // skip whitespace
            while let Some(c) = self.peek(0) {
                if c.is_whitespace() {
                    if c == '\n' {
                        self.line += 1;
                        self.column = 1;
                    } else {
                        self.column += 1;
                    }
                    self.position += 1;
                } else {
                    break;
                }
            }

            // Check for end of file
            let c = match self.peek(0) {
                Some(ch) => ch,
                None => return Ok(Token::new("".to_string(), TokenType::Eof, self.line, self.column)),
            };

            // check for symbols
            match c {
                '+' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("+".to_string(), TokenType::Plus, self.line, self.column - 1));
                },
                '-' => {
                    if let Some(next_char) = self.peek(1) {
                        if next_char.is_digit(10) {
                            return self.handle_negative_number_literal(); // <- check if its a negative num
                        }
                    }
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("-".to_string(), TokenType::Minus, self.line, self.column - 1));

                },
                '*' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("*".to_string(), TokenType::Asterisk, self.line, self.column - 1));
                },
                '=' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("=".to_string(), TokenType::Equal, self.line, self.column - 1));
                },
                '/' => {
                    if self.peek(1) == Some('/') {
                        while let Some(next_char) = self.peek(1) { // skip until end of commend
                            if next_char == '\n' {
                                break;
                            }
                            self.position += 1;
                            self.column += 1;
                        }
                        self.position += 1;
                        self.column += 1; // move past the newline
                        continue; // skip adding a token for the comment
                    }
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("/".to_string(), TokenType::ForwardSlash, self.line, self.column - 1));
                },
                '<' => {
                    let next_char = self.peek(1);
                    if next_char == Some('-') {
                        self.position += 2;
                        self.column += 2;
                        return Ok(Token::new("<-".to_string(), TokenType::Arrow, self.line, self.column - 2))
                    }
                    else if next_char == Some('=') {
                        self.position += 2;
                        self.column += 2;
                        return Ok(Token::new("<=".to_string(), TokenType::LessEqual, self.line, self.column - 2));
                    } else if next_char == Some('>') {
                        self.position += 2;
                        self.column += 2;
                        return Ok(Token::new("<>".to_string(), TokenType::NotEqual, self.line, self.column - 2));
                    } else {
                        self.position += 1;
                        self.column += 1;
                        return Ok(Token::new("<".to_string(), TokenType::LessThan, self.line, self.column - 1));
                    }
                }
                '>' => {
                    let next_char = self.peek(1);
                    if next_char == Some('=') {
                        self.position += 2;
                        self.column += 2;
                        return Ok(Token::new(">=".to_string(), TokenType::GreaterEqual, self.line, self.column - 2));
                    } else {
                        self.position += 1;
                        self.column += 1;
                        return Ok(Token::new(">".to_string(), TokenType::GreaterThan, self.line, self.column - 1));
                    }
                },
                '^' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("^".to_string(), TokenType::Caret, self.line, self.column - 1));
                },
                '&' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("&".to_string(), TokenType::Ampersand, self.line, self.column - 1));
                },
                '"' => {
                    return self.handle_string_literal();
                },
                '\'' => {
                    return self.handle_char_literal();
                }
                ':' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new(":".to_string(), TokenType::Colon, self.line, self.column - 1));
                },
                '(' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("(".to_string(), TokenType::LParen, self.line, self.column - 1));
                },
                ')' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new(")".to_string(), TokenType::RParen, self.line, self.column - 1));
                },
                ',' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new(",".to_string(), TokenType::Comma, self.line, self.column - 1));
                },
                '[' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("[".to_string(), TokenType::LSquare, self.line, self.column - 1));
                },
                ']' => {
                    self.position += 1;
                    self.column += 1;
                    return Ok(Token::new("]".to_string(), TokenType::RSquare, self.line, self.column - 1));
                },
                _ if c.is_alphabetic() || c == '_' => return self.handle_identifier(),
                _ if c.is_digit(10) =>  return self.handle_number_literal(),
                _ => return Err(CPSError {
                    error_type: ErrorType::Lexical,
                    message: format!("Unexpected character: '{}'", c),
                    hint: None,
                    line: self.line,
                    column: self.column,
                    source: Some(self.source.clone()),
                }),
            }
        }

        Ok(Token::new("".to_string(), TokenType::Eof, self.line, self.column))
    }

    fn handle_string_literal(&mut self) -> Result<Token, CPSError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut lexeme = String::new();

        // consume the opening quote
        self.position += 1;
        self.column += 1;

        while let Some(c) = self.peek(0) {
            if c == '"' {
                // closing quote found
                self.position += 1;
                self.column += 1;
                return Ok(Token::new(lexeme, TokenType::StringLiteral, start_line, start_column));
            } else {
                lexeme.push(c);
                self.position += 1;
                self.column += 1;
            }
        }

        // if we reach here, the string literal was not closed
        Err(CPSError {
            error_type: ErrorType::Lexical,
            message: "Your string was not closed!".to_string(),
            hint: Some("Make sure to close your string with a \"".to_string()),
            line: start_line,
            column: start_column,
            source: Some(self.source.clone()),
        })
    }

    fn handle_char_literal(&mut self) -> Result<Token, CPSError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut lexeme = String::new();

        // consume the opening quote
        self.position += 1;
        self.column += 1;

        while let Some(c) = self.peek(0) {
            if c == '\'' {
                // closing quote found
                self.position += 1;
                self.column += 1;
                // check if lexeme is a single character or 0 character (empty char literal)
                if lexeme.chars().count() > 1 {
                    return Err(CPSError {
                        error_type: ErrorType::Lexical,
                        message: "Character literals must be a single character".to_string(),
                        hint: None,
                        line: start_line,
                        column: start_column,
                        source: Some(self.source.clone()),
                    });
                }
                return Ok(Token::new(lexeme, TokenType::CharLiteral, start_line, start_column));
            } else {
                lexeme.push(c);
                self.position += 1;
                self.column += 1;
            }
        }

        // if we reach here, the char literal was not closed
        Err(CPSError {
            error_type: ErrorType::Lexical,
            message: "Your character literal was not closed!".to_string(),
            hint: Some("Make sure to close your character literal with a '".to_string()),
            line: start_line,
            column: start_column,
            source: Some(self.source.clone()),
        })
    }

    fn handle_identifier(&mut self) -> Result<Token, CPSError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut lexeme = String::new();

        while let Some(c) = self.peek(0) {
            if c.is_alphanumeric() || c == '_' {
                lexeme.push(c);
                self.position += 1;
                self.column += 1;
            } else {
                break;
            }
        }

        let token_type = match self.token_map.get(lexeme.as_str()) {
            Some(t) => t.clone(),
            None => TokenType::Identifier,
        };

        Ok(Token::new(lexeme, token_type, start_line, start_column))
    }  

    fn handle_number_literal(&mut self) -> Result<Token, CPSError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut lexeme = String::new();

        while let Some(c) = self.peek(0) {
            if c.is_digit(10) || c == '.' {
                lexeme.push(c);
                self.position += 1;
                self.column += 1;
            } else if c == '_' {
                // ignore underscores in number literals
                self.position += 1;
                self.column += 1;
            } else if c.is_alphabetic() {
                return Err(CPSError {
                    error_type: ErrorType::Lexical,
                    message: "Invalid character in number literal".to_string(),
                    hint: Some("Number literals can only contain digits and decimal points.".to_string()),
                    line: self.line,
                    column: self.column,
                    source: Some(self.source.clone()),
                });
            }
            else {
                break;
            }
        }

        Ok(Token::new(lexeme, TokenType::NumberLiteral, start_line, start_column))
    }

    fn handle_negative_number_literal(&mut self) -> Result<Token, CPSError> {
        let start_line = self.line;
        let start_column = self.column;
        let mut lexeme = String::new();

        lexeme.push('-');
        self.position += 1;
        self.column += 1;

        while let Some(c) = self.peek(0) {
            if c.is_digit(10) || c == '.' {
                lexeme.push(c);
                self.position += 1;
                self.column += 1;
            } else if c == '_' {
                self.position += 1;
                self.column += 1;
            } else if c.is_alphabetic() {
                return Err(CPSError {
                    error_type: ErrorType::Lexical,
                    message: "Invalid character in number literal".to_string(),
                    hint: Some("Number literals can only contain digits and decimal points.".to_string()),
                    line: self.line,
                    column: self.column,
                    source: Some(self.source.clone()),
                });
            } else {
                break;
            }
        }

        Ok(Token::new(lexeme, TokenType::NumberLiteral, start_line, start_column))
    }


}
