use std::collections::HashMap;
use crate::Inter::cps::{ArrayType, CustomTypeKind, Type, Value};
use crate::Lexer::{lexer::Token, lexer::TokenType};
use crate::errortype::{CPSError, ErrorType};
use crate::Parser::ast::{BinaryExpr, BlockStmt, CaseCondition, Expr, FileMode, TypeDefinition};
use super::ast::{Ast, Operator, Position, Associativity, Precedence, Stmt};


#[derive(Debug, Clone)]
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    operators: HashMap<TokenType, Operator>,
    source: String,
    scope: u32, // track the current scope level to prevent invalid terminators in the global scope
}

impl Parser {
    pub fn new(tokens: Vec<Token>, source: String) -> Self {
        let operators: HashMap<TokenType, Operator> = [
            (TokenType::Plus, Operator { precedence: 10, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::Minus, Operator { precedence: 10, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::Asterisk, Operator { precedence: 20, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::ForwardSlash, Operator { precedence: 20, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::Mod, Operator { precedence: 20, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::Div, Operator { precedence: 20, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::Caret, Operator { precedence: 30, position: Position::Infix, associativity: Associativity::Right }), 
            // comparison operators
            (TokenType::Equal, Operator { precedence: 5, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::NotEqual, Operator { precedence: 5, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::LessThan, Operator { precedence: 5, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::LessEqual, Operator { precedence: 5, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::GreaterThan, Operator { precedence: 5, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::GreaterEqual, Operator { precedence: 5, position: Position::Infix, associativity: Associativity::Left }),
            // logical operators
            (TokenType::Or, Operator { precedence: 2, position: Position::Infix, associativity: Associativity::Left }),
            (TokenType::And, Operator { precedence: 3, position: Position::Infix, associativity: Associativity::Left }),
            // string concatination
            (TokenType::Ampersand, Operator { precedence: 8, position: Position::Infix, associativity: Associativity::Left})
        ].iter().cloned().collect();

        Parser { tokens, position: 0, operators, source, scope: 0 }
    }


    // helper functions
    fn peek(&self, n: usize) -> Token {
        if let Some(token) = self.tokens.get(self.position + n) {
            token.clone()
        } else {
            Token::new("".to_string(), TokenType::Eof, 0, 0)
        }
    }

    fn peek_result(&self, n: usize) -> Result<Token, CPSError> {
        if let Some(token) = self.tokens.get(self.position + n) {
            Ok(token.clone())
        } else {
            Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Unexpected end of input".to_string(),
                hint: Some("Expected more tokens".to_string()),
                line: 0,
                column: 0,
                source: Some(self.source.clone()),
            })
        }
    }

    fn advance(&mut self) -> Token {
        let peek = self.peek(0);
        self.position += 1;
        peek
    }



    fn is_terminator(&self, token_type: TokenType) -> bool {
        !matches!(token_type, 
            TokenType::Plus |
            TokenType::Minus |
            TokenType::Asterisk |
            TokenType::ForwardSlash |
            TokenType::Caret
            | TokenType::And | TokenType::Or | TokenType::Ampersand
            | TokenType::GreaterThan | TokenType::GreaterEqual
            | TokenType::Mod | TokenType::Div
            | TokenType::Equal | TokenType::NotEqual | TokenType::LessThan | TokenType::LessEqual
            | TokenType::Identifier | TokenType::NumberLiteral | TokenType::StringLiteral | TokenType::CharLiteral
            | TokenType::Until
            // | TokenType::True | TokenType::False
            | TokenType::LParen | TokenType::Eof
        )
    }

    fn is_operator(&self, token_type: &TokenType) -> bool {
        self.operators.contains_key(&token_type)
    }

    // parsing logic
    pub fn parse_top_expr(&mut self) -> Result<Ast, CPSError> {
        self.parse_expr(0)
    }

    fn parse_primary(&mut self) -> Result<Ast, CPSError> {
        let token = self.peek(0);

        match token.token_type {
            TokenType::NumberLiteral => {
                self.advance();
                let num = token.lexeme.parse::<f64>()
                    .map_err(|_| CPSError {
                        error_type: ErrorType::Syntax,
                        message: format!("Invalid number literal: '{}'", token.lexeme),
                        hint: None,
                        line: token.line,
                        column: token.column,
                        source: Some(self.source.clone()),
                    })?;
                Ok(Ast::Expression(Expr::Literal(Value::Real(num))))
            }
            TokenType::StringLiteral => {
                self.advance();
                let string_value = token.lexeme.trim_matches('"').to_string(); // TODO: should be able to remove the trim
                Ok(Ast::Expression(Expr::Literal(Value::String(string_value))))
            }

            TokenType::Not => {
                self.advance(); 
                let operand = self.parse_primary()?; 

                let true_literal = Ast::Expression(Expr::Literal(Value::Boolean(true))); // convert to TRUE <> x

                return Ok(Ast::Expression(Expr::Binary(BinaryExpr {
                    left: Box::new(true_literal),
                    operator: TokenType::NotEqual,
                    right: Box::new(operand),
                })));
            }


            TokenType::True => {
                self.advance();
                Ok(Ast::Expression(Expr::Literal(Value::Boolean(true))))
            }

            TokenType::False => {
                self.advance();
                Ok(Ast::Expression(Expr::Literal(Value::Boolean(false))))
            }

            TokenType::CharLiteral => {
                self.advance();
                let ch = token.lexeme.chars().next().unwrap_or('\0');
                Ok(Ast::Expression(Expr::Literal(Value::Char(ch))))
            }

            TokenType::Identifier => {
                let name = token.lexeme.clone();
                self.advance();
                if name.to_uppercase() == "EOF" && self.peek(0).token_type == TokenType::LParen {
                    // parse EOF("filename")
                    self.advance(); // consume '('
                    let filename_expr = self.parse_expr(0)?;
                    let filename_str = match ast_to_expr(filename_expr)? {
                        Expr::Literal(Value::String(s)) => s,
                        Expr::Literal(Value::Identifier(id)) => id,
                        _ => return Err(CPSError {
                            error_type: ErrorType::Syntax,
                            message: "EOF argument must be a filename".to_string(),
                            hint: None,
                            line: token.line,
                            column: token.column,
                            source: Some(self.source.clone()),
                        }),
                    };
                    let close = self.advance();
                    if close.token_type != TokenType::RParen {
                        return Err(CPSError {
                            error_type: ErrorType::Syntax,
                            message: "Expected ')' after EOF argument".to_string(),
                            hint: None,
                            line: close.line,
                            column: close.column,
                            source: Some(self.source.clone()),
                        });
                    }
                    return Ok(Ast::Expression(Expr::EOF { filename: filename_str }));
                } else if self.peek(0).token_type == TokenType::LParen {
                    self.parse_function_call_expr(name)
                } else if self.peek(0).token_type == TokenType::LSquare {
                    self.parse_array_access_expr(name)
                } else {
                    Ok(Ast::Identifier(name))
                }
            }

            TokenType::LParen => {
                self.advance();
                let expr = self.parse_expr(0)?;
                if self.peek(0).token_type != TokenType::RParen {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected ')'".to_string(),
                        hint: Some("Mismatched parentheses".to_string()),
                        line: self.peek(0).line,
                        column: self.peek(0).column,
                        source: Some(self.source.clone()),
                    });
                }
                self.advance();
                Ok(expr)
            }
            TokenType::Eof => {
                Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Unexpected end of input".to_string(),
                    hint: Some("Expected an expression".to_string()),
                    line: token.line,
                    column: token.column,
                    source: Some(self.source.clone()),
                })
            }
            _ => {
                Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: format!("Unexpected token: {:?}", token.token_type),
                    hint: Some("Expected a number, identifier, or '('".to_string()),
                    line: token.line,
                    column: token.column,
                    source: Some(self.source.clone()),
                })
            }
        }
    }

    fn parse_expr(&mut self, min_precedence: Precedence) -> Result<Ast, CPSError> {
        let mut lhs = self.parse_primary()?;

        loop {
            let current = self.peek(0);

            if self.is_terminator(current.token_type.clone()) || current.token_type == TokenType::Eof {
                break;
            }

            if !self.is_operator(&current.token_type) {
                break;
            }

            let operator = match self.operators.get(&current.token_type) {
                Some(op) => op.clone(),
                None => break,
            };

            if operator.precedence < min_precedence {
                break;
            }

            let op_token = current.token_type.clone();
            self.advance();

            let next_min_prec = match operator.associativity {
                Associativity::Left => operator.precedence + 1,
                Associativity::Right => operator.precedence,
            };

            let rhs = self.parse_expr(next_min_prec)?;

            lhs = Ast::Expression(Expr::Binary(BinaryExpr {
                left: Box::new(lhs),
                operator: op_token,
                right: Box::new(rhs),
            }));
        }

        Ok(lhs)
    }

    fn parse_function_call_expr(&mut self, name: String) -> Result<Ast, CPSError> {
        let mut args = vec![];

        // consume '('
        let open_paren = self.advance();
        if open_paren.token_type != TokenType::LParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected '(' after function name".to_string(),
                hint: Some("Function arguments must be enclosed in parentheses".to_string()),
                line: open_paren.line,
                column: open_paren.column,
                source: Some(self.source.clone()),
            });
        }

        while self.peek(0).token_type != TokenType::RParen {
            let expr = self.parse_expr(0)?;
            args.push(ast_to_expr(expr)?);

            if self.peek(0).token_type == TokenType::Comma {
                self.advance(); // consume comma
            } else {
                break;
            }
        }

        // consume ')'
        let close_paren = self.advance();
        if close_paren.token_type != TokenType::RParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ')' after function arguments".to_string(),
                hint: Some("Function arguments must be enclosed in parentheses".to_string()),
                line: close_paren.line,
                column: close_paren.column,
                source: Some(self.source.clone()),
            });
        }


        Ok(Ast::Expression(Expr::Call {
            name,
            arguments: args,
        }))

    }

    fn parse_array_access_expr(&mut self, name: String) -> Result<Ast, CPSError> {
        // consume '['
        let open_square = self.advance();
        if open_square.token_type != TokenType::LSquare {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected '[' after array name".to_string(),
                hint: Some("Array indices must be enclosed in brackets".to_string()),
                line: open_square.line,
                column: open_square.column,
                source: Some(self.source.clone()),
            });
        }

        let index_expr = self.parse_expr(0)?;
        let index = ast_to_expr(index_expr)?;

        let mut col = None;

        // consume ']'
        let close_square = self.advance();
        if close_square.token_type == TokenType::Comma { 
            // multi dimesnional array, check for more ub
            let col_expr = self.parse_expr(0)?;
            col = Some(ast_to_expr(col_expr)?);
            // expect close bracket

            let close_square = self.advance();
            if close_square.token_type != TokenType::RSquare {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected ']' after array index".to_string(),
                    hint: Some("Array indices must be enclosed in brackets".to_string()),
                    line: close_square.line,
                    column: close_square.column,
                    source: Some(self.source.clone()),
                });
            }
        }
        else if close_square.token_type != TokenType::RSquare {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ']' after array index".to_string(),
                hint: Some("Array indices must be enclosed in brackets".to_string()),
                line: close_square.line,
                column: close_square.column,
                source: Some(self.source.clone()),
            });
        }

        Ok(Ast::Expression(Expr::ArrayAccess {
            name: name,
            index: Box::new(index),
            col: col.map(Box::new), 
        }))
    }

    pub fn parse_statements(&mut self) -> Result<Vec<Ast>, CPSError> {
        let mut ast = vec![]; 
        loop {
            let token = self.peek(0);
            let statement = self.parse_statement(token);

            match statement {
                Ok(stmt) => ast.push(stmt),
                Err((e, should_break)) => {
                    if should_break {
                        break;
                    }
                    return Err(e);
                }
            }
        }
        Ok(ast)
    }

    fn parse_statement(&mut self, token: Token) -> Result<Ast, (CPSError, bool)> {
        match token.token_type {
            TokenType::If => self.parse_if_statement().map_err(|e| (e, false)),
            TokenType::Case => self.parse_case_statement().map_err(|e| (e, false)),
            TokenType::While => self.parse_while_statement().map_err(|e| (e, false)),
            TokenType::Repeat => self.parse_repeat_statement().map_err(|e| (e, false)),
            TokenType::Output => self.parse_output().map_err(|e| (e, false)),
            TokenType::Input => self.parse_input().map_err(|e| (e, false)),
            TokenType::Declare => self.parse_declaration().map_err(|e| (e, false)),
            TokenType::Constant => self.parse_constant_declaration().map_err(|e| (e, false)),
            TokenType::Identifier => self.parse_assignment().map_err(|e| (e, false)),
            TokenType::For => self.parse_for_statement().map_err(|e| (e, false)),
            TokenType::Procedure => self.parse_procedure().map_err(|e| (e, false)),
            TokenType::Function => self.parse_function().map_err(|e| (e, false)),
            TokenType::Return => self.parse_return().map_err(|e| (e, false)),
            TokenType::Call => self.parse_call().map_err(|e| (e, false)),
            TokenType::OpenFile => self.parse_open_file().map_err(|e| (e, false)),
            TokenType::CloseFile => self.parse_close_file().map_err(|e| (e, false)),
            TokenType::WriteFile => self.parse_writefile().map_err(|e| (e, false)),
            TokenType::ReadFile => self.parse_readfile().map_err(|e| (e, false)),
            TokenType::Type => self.parse_type_declaration().map_err(|e| (e, false)),
            // terminate if it leaves the scope
            TokenType::Eof | TokenType::EndIf | TokenType::EndCase |   
                TokenType::Else | TokenType::Next | TokenType::Until | TokenType::EndClass | 
                TokenType::EndWhile | TokenType::EndFunction | TokenType::EndProcedure  => {
                    if self.scope == 0 && token.token_type != TokenType::Eof {
                        return Err((CPSError {
                            error_type: ErrorType::Syntax,
                            message: format!("Unexpected token: {}" , token.lexeme),
                            hint: Some("You are trying to terminate a code block in the global scope.".to_string()),
                            line: token.line,
                            column: token.column,
                            source: Some(self.source.clone()),
                        }, false));
                    }
                    Err((CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Loop terminator".to_string(),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    }, true))
            }
            _ => {
                Err((CPSError { 
                    error_type: ErrorType::Syntax,
                    message: format!("Unexpected token in statement: {:?}", token.token_type),
                    hint: Some("Expected a valid statement".to_string()),
                    line: token.line,
                    column: token.column,
                    source: Some(self.source.clone()),
                }, false))
            }
        }
    }

    fn parse_output(&mut self) -> Result<Ast, CPSError> {
        let output_token = self.advance(); // consume 'output'

        let first = match self.parse_expr(0)? {
            Ast::Expression(e) => e,
            Ast::Identifier(id) => Expr::Literal(Value::Identifier(id)),
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: format!("Expected an expression after {}", output_token.lexeme),
                    hint: Some(format!("{} must be followed by a valid expression", output_token.lexeme)),
                    line: output_token.line,
                    column: output_token.column,
                    source: Some(self.source.clone()),
                });
            }
        };
        
        // Treat ',' as concatinators in output statements
        if self.peek(0).token_type != TokenType::Comma {
            return Ok(Ast::Stmt(Stmt::Output { target: first }));
        }

        let mut combined = first; 
        while self.peek(0).token_type == TokenType::Comma {
            self.advance(); // consume ','
            let next = match self.parse_expr(0)? {
                Ast::Expression(e) => e,
                Ast::Identifier(id) => Expr::Literal(Value::Identifier(id)),
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: format!("Expected an expression after ',' in {}", output_token.lexeme),
                        hint: Some("Each value separated by ',' must be a valid expression".to_string()),
                        line: output_token.line,
                        column: output_token.column,
                        source: Some(self.source.clone()),
                    });
                }
            };
            combined = Expr::Binary(BinaryExpr {
                left: Box::new(Ast::Expression(combined)),
                operator: TokenType::Ampersand,
                right: Box::new(Ast::Expression(next)),
            });
        }

        Ok(Ast::Stmt(Stmt::Output { target: combined }))
    }



    fn parse_input(&mut self) -> Result<Ast, CPSError> {
        let input_token = self.advance(); // consume 'input'
        let identifier_token = self.advance();
        if identifier_token.token_type != TokenType::Identifier {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: format!("Expected an identifier after {}", input_token.lexeme),
                hint: Some(format!("{} must be followed by a variable name", input_token.lexeme)),
                line: identifier_token.line,
                column: identifier_token.column,
                source: Some(self.source.clone()),
            });
        }

        if self.peek(0).token_type == TokenType::LSquare {
            // array access
            let array_expr = ast_to_expr(self.parse_array_access_expr(identifier_token.clone().lexeme)?)?;
            return Ok(Ast::Stmt(Stmt::Input { identifier: Box::new(array_expr) }));

        }
        Ok(Ast::Stmt(Stmt::Input { identifier: Box::new(Expr::Literal(Value::Identifier(identifier_token.lexeme))) }))
    }

    fn parse_if_statement(&mut self) -> Result<Ast, CPSError> {
        let if_token = self.advance(); // consume 'if'
        let condition = self.parse_expr(0)?;

        self.scope += 1; // enter the scope of the if statement
        
        // consume the 'THEN'
        let then = self.peek(0);
        if then.token_type != TokenType::Then {
            let line = if then.line == 0 { 0 } else { then.line - 1 };
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'THEN' after if condition".to_string(),
                hint: Some("IF statements must have a THEN clause".to_string()),
                line,
                column: then.column,
                source: Some(self.source.clone()),
            });
        }

        // consume 'then'
        self.advance();
        let then_branch = self.parse_statements()?;
        let else_branch = if self.peek(0).token_type == TokenType::Else {
            self.advance(); // consume 'else'
            Some(self.parse_statements()?)
        } else {
            None
        };

        // consume endif
        let end_token = self.peek(0);
        if end_token.token_type != TokenType::EndIf {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'ENDIF' after if statement".to_string(),
                hint: Some("IF statements must be closed with ENDIF".to_string()),
                line: end_token.line,
                column: end_token.column,
                source: Some(self.source.clone()),
            });
        }
        self.advance(); // consume 'endif'

        // Convert condition to Expr
        let condition_expr = match condition {
            Ast::Expression(e) => e,
            Ast::Identifier(id) => Expr::Literal(Value::Identifier(id)),
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Invalid condition in IF statement".to_string(),
                    hint: Some("Condition must be a valid expression".to_string()),
                    line: if_token.line,
                    column: if_token.column,
                    source: Some(self.source.clone()),
                });
            }
        };

        let then_statements: Result<Vec<Stmt>, CPSError> = then_branch.into_iter().map(|a| match a {
            Ast::Stmt(s) => Ok(s),
            _ => Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected statement in then branch".to_string(),
                hint: Some("IF statement body must contain valid statements".to_string()),
                line: if_token.line,
                column: if_token.column,
                source: Some(self.source.clone()),
            }),
        }).collect();

        let else_statements: Option<Result<Vec<Stmt>, CPSError>> = else_branch.map(|branch| {
            branch.into_iter().map(|a| match a {
                Ast::Stmt(s) => Ok(s),
                _ => Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected statement in else branch".to_string(),
                    hint: Some("ELSE statement body must contain valid statements".to_string()),
                    line: if_token.line,
                    column: if_token.column,
                    source: Some(self.source.clone()),
                }),
            }).collect()
        });

        self.scope -= 1; // decrease scope after parsing if statement

        Ok(Ast::Stmt(Stmt::If {
            condition: Box::new(condition_expr),
            then_branch: BlockStmt { 
                statements: then_statements?
            },
            else_branch: match else_statements {
                Some(result) => Some(BlockStmt { 
                    statements: result?
                }),
                None => None,
            },
        }))
    }

    fn parse_case_statement(&mut self) -> Result<Ast, CPSError> {
        let _ = self.advance(); // consume 'case'
        let of_token = self.advance();
        if of_token.token_type != TokenType::Of {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'OF' after CASE".to_string(),
                hint: Some("CASE statements must have an OF clause".to_string()),
                line: of_token.line,
                column: of_token.column,
                source: Some(self.source.clone()),
            });
        }

        let identifier = self.parse_expr(0)?;
        let identifier_expr = ast_to_expr(identifier)?;

        self.scope += 1; // enter the scope of the case statement

        let mut cases: Vec<(CaseCondition, BlockStmt)> = Vec::new();
        let mut otherwise: Option<BlockStmt> = None;

        loop {
            let token = self.peek(0);

            if token.token_type == TokenType::EndCase {
                self.advance(); // consume 'endcase'
                self.scope -= 1; // decrease scope after parsing case statement
                break;
            }

            // Check for OTHERWISE clause
            if token.token_type == TokenType::Otherwise {
                self.advance(); // consume 'otherwise'
                let colon_token = self.advance();
                if colon_token.token_type != TokenType::Colon {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected ':' after OTHERWISE".to_string(),
                        hint: Some("OTHERWISE clause must be followed by a colon".to_string()),
                        line: colon_token.line,
                        column: colon_token.column,
                        source: Some(self.source.clone()),
                    });
                }
                let statements = self.parse_case_body()?;
                otherwise = Some(BlockStmt { statements });
                continue;
            }

            // Parse case option
            let option = self.parse_expr(0)?;
            let option_expr = ast_to_expr(option)?;

            // Check if it's a range (value1 TO value2)
            let case_condition = if self.peek(0).token_type == TokenType::To {
                self.advance(); // consume 'to'
                let end_value = self.parse_expr(0)?;
                let end_expr = ast_to_expr(end_value)?;
                CaseCondition::Range(option_expr, end_expr)
            } else {
                CaseCondition::Single(option_expr)
            };

            // Expect colon
            let colon_token = self.advance();
            if colon_token.token_type != TokenType::Colon {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected ':' after case option".to_string(),
                    hint: Some("CASE options must be followed by a colon".to_string()),
                    line: colon_token.line,
                    column: colon_token.column,
                    source: Some(self.source.clone()),
                });
            }

            let statements = self.parse_case_body()?;
            cases.push((case_condition, BlockStmt { statements }));
        }

        Ok(Ast::Stmt(Stmt::Case {
            identifier: Box::new(identifier_expr),
            cases,
            otherwise,
        }))
    }

    fn parse_case_body(&mut self) -> Result<Vec<Stmt>, CPSError> {
        let mut statements = Vec::new();

        loop {
            let token = self.peek(0);

            if self.is_case_terminator(&token.token_type) {
                break;
            }

            let statement = self.parse_statement(token.clone());

            match statement {
                Ok(Ast::Stmt(s)) => statements.push(s),
                Ok(_) => {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected statement in case body".to_string(),
                        hint: Some("CASE body must contain valid statements".to_string()),
                        line: token.line,
                        column: token.column,
                        source: Some(self.source.clone()),
                    });
                }
                Err((e, should_break)) => {
                    if should_break {
                        break;
                    }
                    return Err(e);
                }
            };

            if self.is_case_terminator(&self.peek(0).token_type) {
                break;
            }
        }

        Ok(statements)
    }

    fn is_case_terminator(&self, token_type: &TokenType) -> bool {
        match token_type {
            TokenType::EndCase | TokenType::Otherwise | TokenType::Eof => true,
            TokenType::NumberLiteral | TokenType::StringLiteral | 
                TokenType::CharLiteral | TokenType::Identifier | 
                TokenType::True | TokenType::False => { 
                    self.could_be_case_label()
                }
            _ => false,
        }
    }

    fn could_be_case_label(&self) -> bool {
        match self.peek(0).token_type {
            TokenType::NumberLiteral | TokenType::StringLiteral | 
                TokenType::CharLiteral | TokenType::Identifier | 
                TokenType::True | TokenType::False => {  
                    match self.peek(1).token_type {
                        TokenType::Colon => true,
                        TokenType::To => true,
                        _ => false,
                    }
                }
            _ => false,
        }
    }




    fn parse_while_statement(&mut self) -> Result<Ast, CPSError> {
        let while_token = self.advance(); // consume 'while'
        let condition = self.parse_expr(0)?;

        self.scope += 1; // enter the scope of the while loop

        let body_statements = self.parse_statements()?;

        // consume endwhile
        let end_token = self.peek(0);
        if end_token.token_type != TokenType::EndWhile {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'ENDWHILE' after while statement".to_string(),
                hint: Some("WHILE statements must be closed with ENDWHILE".to_string()),
                line: end_token.line,
                column: end_token.column,
                source: Some(self.source.clone()),
            });
        }
        self.advance(); // consume 'endwhile'
        self.scope -= 1; // decrease scope after parsing while statement

        // Convert condition to Expr
        let condition_expr = match condition {
            Ast::Expression(e) => e,
            Ast::Identifier(id) => Expr::Literal(Value::Identifier(id)),
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Invalid condition in WHILE statement".to_string(),
                    hint: Some("Condition must be a valid expression".to_string()),
                    line: while_token.line,
                    column: while_token.column,
                    source: Some(self.source.clone()),
                });
            }
        };

        let body_statements: Result<Vec<Stmt>, CPSError> = body_statements.into_iter().map(|a| match a {
            Ast::Stmt(s) => Ok(s),
            _ => Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected statement in while body".to_string(),
                hint: Some("WHILE statement body must contain valid statements".to_string()),
                line: while_token.line,
                column: while_token.column,
                source: Some(self.source.clone()),
            }),
        }).collect();

        Ok(Ast::Stmt(Stmt::While {
            condition: Box::new(condition_expr),
            body: BlockStmt {
                statements: body_statements?,
            },
        }))
    }

    fn parse_repeat_statement(&mut self) -> Result<Ast, CPSError> {
        let repeat_token = self.advance(); // consume 'repeat'
        self.scope += 1;
        // parse block statements after
        let stmts = self.parse_statements()?;

        let until_token = self.advance();
        if until_token.token_type != TokenType::Until {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'UNTIL' after REPEAT block".to_string(),
                hint: Some("REPEAT statements must end with UNTIL".to_string()),
                line: until_token.line,
                column: until_token.column,
                source: Some(self.source.clone()),
            });
        }
        self.scope -= 1; 

        let condition = self.parse_expr(0)?;
        let body_statements: Result<Vec<Stmt>, CPSError> = stmts.into_iter().map(|a| match a {
            Ast::Stmt(s) => Ok(s),
            _ => Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected statement in while body".to_string(),
                hint: Some("WHILE statement body must contain valid statements".to_string()),
                line: repeat_token.line,
                column: repeat_token.column,
                source: Some(self.source.clone()),
            }),
        }).collect();




        Ok(Ast::Stmt(Stmt::Repeat {
            body: BlockStmt {
                statements: body_statements?,
            },
            until: Box::new(match condition {
                Ast::Expression(e) => e,
                Ast::Identifier(id) => Expr::Literal(Value::Identifier(id)),
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Invalid condition in REPEAT statement".to_string(),
                        hint: Some("Condition must be a valid expression".to_string()),
                        line: until_token.line,
                        column: until_token.column,
                        source: Some(self.source.clone()),
                    });
                }
            }),
        }))

    }

    fn parse_for_statement(&mut self) -> Result<Ast, CPSError> {
        let for_token = self.advance(); // consume 'for'
        let identifier_token = self.advance();
        if identifier_token.token_type != TokenType::Identifier {
            return Err(CPSError{
                error_type: ErrorType::Syntax,
                message: "Expected an identifier after 'FOR'".to_string(),
                hint: Some("FOR loops must specify a loop variable".to_string()),
                line: identifier_token.line,
                column: identifier_token.column,
                source: Some(self.source.clone()),
            })
        }

        // consume arrow
        let arrow_token = self.advance();
        if arrow_token.token_type != TokenType::Arrow {
            return Err(CPSError{
                error_type: ErrorType::Syntax,
                message: "Expected '<-' after loop identifier".to_string(),
                hint: Some("FOR loops must use '<-' to specify the start value".to_string()),
                line: arrow_token.line,
                column: arrow_token.column,
                source: Some(self.source.clone()),
            })
        }

        let start_expr = match self.parse_expr(0)? {
            Ast::Expression(e) => e,
            Ast::Identifier(id) => Expr::Literal(Value::Identifier(id)),
            _ => {
                return Err(CPSError{
                    error_type: ErrorType::Syntax,
                    message: "Invalid start value in FOR loop".to_string(),
                    hint: Some("Start value must be a valid expression".to_string()),
                    line: for_token.line,
                    column: for_token.column,
                    source: Some(self.source.clone()),
                })
            }
        };

        let to_token = self.advance();
        if to_token.token_type != TokenType::To {
            return Err(CPSError{
                error_type: ErrorType::Syntax,
                message: "Expected 'TO' after start value".to_string(),
                hint: Some("FOR loops must use 'TO' to specify the end value".to_string()),
                line: to_token.line,
                column: to_token.column,
                source: Some(self.source.clone()),
            })
        }

        let end_expr = match self.parse_expr(0)? {
            Ast::Expression(e) => e,
            Ast::Identifier(id) => Expr::Literal(Value::Identifier(id)),
            _ => {
                return Err(CPSError{
                    error_type: ErrorType::Syntax,
                    message: "Invalid end value in FOR loop".to_string(),
                    hint: Some("End value must be a valid expression".to_string()),
                    line: for_token.line,
                    column: for_token.column,
                    source: Some(self.source.clone()),
                })
            }
        };

        self.scope += 1; // enter the scope of the for loop

        let body_statements = self.parse_statements()?;

        let body_statements: Result<Vec<Stmt>, CPSError> = body_statements.into_iter().map(|a| match a {
            Ast::Stmt(s) => Ok(s),
            _ => Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected statement in while body".to_string(),
                hint: Some("WHILE statement body must contain valid statements".to_string()),
                line: for_token.line,
                column: for_token.column,
                source: Some(self.source.clone()),
            }),
        }).collect();

        let body_statements = body_statements?;

        // expect NEXT ident 
        let next_token = self.advance();
        if next_token.token_type != TokenType::Next {
            return Err(CPSError{
                error_type: ErrorType::Syntax,
                message: "Expected 'NEXT' after FOR loop body".to_string(),
                hint: Some("FOR loops must be closed with NEXT".to_string()),
                line: next_token.line,
                column: next_token.column,
                source: Some(self.source.clone()),
            })
        }

        self.scope -= 1; // decrease scope after parsing for loop

        let iden = self.advance(); // consume identifier after NEXT
        if iden.token_type != TokenType::Identifier {
            return Err(CPSError{
                error_type: ErrorType::Syntax,
                message: "Expected identifier after 'NEXT'".to_string(),
                hint: Some("FOR loops must specify the loop variable after NEXT".to_string()),
                line: iden.line,
                column: iden.column,
                source: Some(self.source.clone()),
            })
        }


        Ok(Ast::Stmt(Stmt::For {
            identifier: identifier_token.lexeme,
            start: Box::new(start_expr), 
            end: Box::new(end_expr),
            body: BlockStmt { statements: body_statements }, 
        }))
    }

    fn parse_declaration(&mut self) -> Result<Ast, CPSError> {
        self.advance();
        // expect identifier 
        let identifier = self.advance();
        if identifier.token_type != TokenType::Identifier {
            return Err(CPSError { error_type: ErrorType::Syntax, 
                message: "Expected an identifier after declare".to_string(), hint: Some("Make sure to write a variable name after DECLARE".to_string()), 
                line: identifier.line, column: identifier.column, source: Some(self.source.clone()) });
        }

        // consume arrow
        let colon = self.advance();
        if colon.token_type != TokenType::Colon {
            return Err(CPSError { error_type: ErrorType::Syntax, 
                message: "Expected a colon after the identifier".to_string(), hint: Some("Make sure to use a colon after the variable name and before the data type".to_string()), 
                line: identifier.line, column: identifier.column, source: Some(self.source.clone()) });
        }

        let type_token = self.advance();

        let data_type = match type_token.token_type {
            TokenType::Integer => Type::Integer,
            TokenType::Real => Type::Real,
            TokenType::String => Type::String,
            TokenType::Char => Type::Char,
            TokenType::Boolean => Type::Boolean, 
            TokenType::Array => self.parse_array_type()?,
            TokenType::Identifier => Type::CustomType(CustomTypeKind::Named(type_token.lexeme.clone())),
            _ => {
                return Err(CPSError { error_type: ErrorType::Syntax,
                    message: format!("'{}' is not a valid data type", type_token.lexeme),
                    hint: Some("Built-in types are INTEGER, REAL, STRING, BOOLEAN, CHAR. For a custom type, make sure it is defined with TYPE before use.".to_string()),
                    line: type_token.line, column: type_token.column, source: Some(self.source.clone())
                });
            }
        };

        Ok(Ast::Stmt(Stmt::Decleration { identifier: identifier.lexeme, type_: data_type }))
    }

    fn parse_constant_declaration(&mut self) -> Result<Ast, CPSError> {
        self.advance();
        // expect identifier 
        let identifier = self.advance();
        if identifier.token_type != TokenType::Identifier {
            return Err(CPSError { error_type: ErrorType::Syntax, 
                message: "Expected an identifier after constant".to_string(), hint: Some("Make sure to write a name after CONSTANT".to_string()), 
                line: identifier.line, column: identifier.column, source: Some(self.source.clone()) });
        }

        // consume equals
        let equals = self.advance();
        if equals.token_type != TokenType::Equal {
            return Err(CPSError { error_type: ErrorType::Syntax, 
                message: "Expected '=' after the identifier".to_string(), hint: Some("Make sure to use '=' after the constant name".to_string()), 
                line: identifier.line, column: identifier.column, source: Some(self.source.clone()) });
        }

        let value_expr = self.parse_expr(0)?;
        let value = self.ast_to_value(value_expr);

        match value {
            Ok(v) => Ok(Ast::Stmt(Stmt::Constant { identifier: identifier.lexeme, value: v })),
            Err(_) => Err(CPSError { error_type: ErrorType::Syntax, 
                message: "Expected a literal value for constant declaration".to_string(), hint: Some("Only literals can be used as the value of a constant. A variable, another constant or an expression must never be used.".to_string()), 
                line: identifier.line, column: identifier.column, source: Some(self.source.clone()) }),
        }

    }

    fn ast_to_value(&self, ast: Ast) -> Result<Value, CPSError> {
        match ast {
            Ast::Expression(e) => match e {
                Expr::Literal(v) => Ok(v),
                _ => Err(CPSError { error_type: ErrorType::Syntax, 
                    message: "Expected a literal value".to_string(), hint: None, 
                    line: 0, column: 0, source: Some(self.source.clone()) }),
            },
            _ => Err(CPSError { error_type: ErrorType::Syntax, 
                message: "Expected a literal value".to_string(), hint: None, 
                line: 0, column: 0, source: Some(self.source.clone()) }),
        }
    }

    fn parse_assignment(&mut self) -> Result<Ast, CPSError> {
        let identifier = self.advance();

        let assign_token = self.peek(0);
        let array_index = None;
        if assign_token.token_type != TokenType::Arrow && assign_token.token_type != TokenType::LSquare {
            return Err(CPSError { error_type: ErrorType::Syntax, 
                message: "Expected '<-' in assignment".to_string(), hint: None, 
                line: assign_token.line, column: assign_token.column, source: Some(self.source.clone()) });
        } 
        if assign_token.token_type == TokenType::LSquare {
            // array access
            let array_access = self.parse_array_access_expr(identifier.lexeme.clone())?;
            if let Ast::Expression(Expr::ArrayAccess { name: _, index, col }) = array_access {
                // consume '<-'
                let assign_token = self.advance();
                if assign_token.token_type != TokenType::Arrow {
                    return Err(CPSError { error_type: ErrorType::Syntax, 
                        message: "Expected '<-' in assignment".to_string(), hint: None, 
                        line: assign_token.line, column: assign_token.column, source: Some(self.source.clone()) });
                }

                let value = self.parse_expr(0)?;

                return Ok(Ast::Stmt(Stmt::Assignment { identifier: identifier.lexeme, array_index: Some((Box::new(*index), col)), value: Box::new(value) }));
            }         
        }

        self.advance();

        let value = self.parse_expr(0)?;

        Ok(Ast::Stmt(Stmt::Assignment { identifier: identifier.lexeme, array_index, value: Box::new(value) }))
    }

    fn parse_procedure(&mut self) -> Result<Ast, CPSError> {
        let procedure_token = self.advance(); // consume 'procedure'
        let identifier_token = self.advance();
        if identifier_token.token_type != TokenType::Identifier {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected an identifier after 'PROCEDURE'".to_string(),
                hint: Some("PROCEDURE must be followed by a valid name".to_string()),
                line: identifier_token.line,
                column: identifier_token.column,
                source: Some(self.source.clone()),
            });
        }

        let open_paren = self.advance();
        if open_paren.token_type != TokenType::LParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected '(' after procedure name".to_string(),
                hint: Some("PROCEDURE parameters must be enclosed in parentheses".to_string()),
                line: open_paren.line,
                column: open_paren.column,
                source: Some(self.source.clone()),
            });
        }

        let mut parameters: Vec<(String, Type)> = Vec::new();

        while self.peek(0).token_type != TokenType::RParen {
            // go through the identifier : type pairs
            let param_identifier = self.advance();
            if param_identifier.token_type != TokenType::Identifier {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected parameter name in procedure declaration".to_string(),
                    hint: Some("Procedure parameters must have valid names".to_string()),
                    line: param_identifier.line,
                    column: param_identifier.column,
                    source: Some(self.source.clone()),
                });
            }

            let colon_token = self.advance();
            if colon_token.token_type != TokenType::Colon {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected ':' after parameter name".to_string(),
                    hint: Some("Parameter name must be followed by its type".to_string()),
                    line: colon_token.line,
                    column: colon_token.column,
                    source: Some(self.source.clone()),
                });
            }

            let type_token = self.advance();
            let param_type = match type_token.token_type {
                TokenType::Integer => Type::Integer,
                TokenType::Real => Type::Real,
                TokenType::String => Type::String,
                TokenType::Char => Type::Char,
                TokenType::Boolean => Type::Boolean,
                TokenType::Array => self.parse_array_type()?,
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected a valid data type for parameter".to_string(),
                        hint: None,
                        line: type_token.line,
                        column: type_token.column,
                        source: Some(self.source.clone()),
                    });
                }
            };
            parameters.push((param_identifier.lexeme, param_type));

            // check for comma after 
            if self.peek(0).token_type == TokenType::Comma {
                self.advance(); // consume comma
            } else {
                break;
            }

        }

        let close_paren = self.advance();
        if close_paren.token_type != TokenType::RParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ')' after procedure parameters".to_string(),
                hint: Some("PROCEDURE parameters must be enclosed in parentheses".to_string()),
                line: close_paren.line,
                column: close_paren.column,
                source: Some(self.source.clone()),
            });
        }

        self.scope += 1;

        let body_statements = self.parse_statements()?;
        let body_statements: Result<Vec<Stmt>, CPSError> = body_statements.into_iter().map(|a| match a {
            Ast::Stmt(s) => Ok(s),
            _ => Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected statement in procedure body".to_string(),
                hint: Some("PROCEDURE body must contain valid statements".to_string()),
                line: procedure_token.line,
                column: procedure_token.column,
                source: Some(self.source.clone()),
            }),
        }).collect();
        // consume endprocedure
        let end_token = self.advance();
        if end_token.token_type != TokenType::EndProcedure {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'ENDPROCEDURE' after procedure body".to_string(),
                hint: Some("PROCEDURE must be closed with ENDPROCEDURE".to_string()),
                line: end_token.line,
                column: end_token.column,
                source: Some(self.source.clone()),
            });
        }
        self.scope -= 1;

        Ok(Ast::Stmt(Stmt::Procedure {
            name: identifier_token.lexeme,
            parameters,
            body: BlockStmt {
                statements: body_statements?,
            },
        }) )
    }

    fn parse_function(&mut self) -> Result<Ast, CPSError> {
        let function_token = self.advance(); // consume 'function'
        let identifier_token = self.advance();
        if identifier_token.token_type != TokenType::Identifier {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected an identifier after 'FUNCTION'".to_string(),
                hint: Some("FUNCTION must be followed by a valid name".to_string()),
                line: identifier_token.line,
                column: identifier_token.column,
                source: Some(self.source.clone()),
            });
        }

        let open_paren = self.advance();
        if open_paren.token_type != TokenType::LParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected '(' after function name".to_string(),
                hint: Some("FUNCTION parameters must be enclosed in parentheses".to_string()),
                line: open_paren.line,
                column: open_paren.column,
                source: Some(self.source.clone()),
            });
        }

        let mut parameters: Vec<(String, Type)> = Vec::new();

        while self.peek(0).token_type != TokenType::RParen {
            // go through the identifier : type pairs
            let param_identifier = self.advance();
            if param_identifier.token_type != TokenType::Identifier {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected parameter name in function declaration".to_string(),
                    hint: Some("Function parameters must have valid names".to_string()),
                    line: param_identifier.line,
                    column: param_identifier.column,
                    source: Some(self.source.clone()),
                });
            }

            let colon_token = self.advance();
            if colon_token.token_type != TokenType::Colon {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected ':' after parameter name".to_string(),
                    hint: Some("Parameter name must be followed by its type".to_string()),
                    line: colon_token.line,
                    column: colon_token.column,
                    source: Some(self.source.clone()),
                });
            }

            let type_token = self.advance();
            let param_type = match type_token.token_type {
                TokenType::Integer => Type::Integer,
                TokenType::Real => Type::Real,
                TokenType::String => Type::String,
                TokenType::Char => Type::Char,
                TokenType::Boolean => Type::Boolean,
                TokenType::Array => self.parse_array_type()?,
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected a valid data type for parameter".to_string(),
                        hint: None,
                        line: type_token.line,
                        column: type_token.column,
                        source: Some(self.source.clone()),
                    });
                }
            };
            parameters.push((param_identifier.lexeme, param_type));
            // check for comma after 
            if self.peek(0).token_type == TokenType::Comma {
                self.advance(); // consume comma
            } else {
                break;
            }
        }

        let close_paren = self.advance();
        if close_paren.token_type != TokenType::RParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ')' after function parameters".to_string(),
                hint: Some("FUNCTION parameters must be enclosed in parentheses".to_string()),
                line: close_paren.line,
                column: close_paren.column,
                source: Some(self.source.clone()),
            });
        }

        let returns = self.advance();
        if returns.token_type != TokenType::Returns {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'RETURNS' after function parameters".to_string(),
                hint: Some("FUNCTION must specify a return type using RETURNS".to_string()),
                line: returns.line,
                column: returns.column,
                source: Some(self.source.clone()),
            });
        }

        let return_type_token = self.advance();
        let return_type = match return_type_token.token_type {
            TokenType::Integer => Type::Integer,
            TokenType::Real => Type::Real,
            TokenType::String => Type::String,
            TokenType::Char => Type::Char,
            TokenType::Boolean => Type::Boolean,
            TokenType::Array => self.parse_array_type()?,
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected a valid return data type for function".to_string(),
                    hint: None,
                    line: return_type_token.line,
                    column: return_type_token.column,
                    source: Some(self.source.clone()),
                });
            }
        };

        self.scope += 1;

        let body_statements = self.parse_statements()?;
        let body_statements: Result<Vec<Stmt>, CPSError> = body_statements.into_iter().map(|a| match a {
            Ast::Stmt(s) => Ok(s),
            _ => Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected statement in procedure body".to_string(),
                hint: Some("PROCEDURE body must contain valid statements".to_string()),
                line: function_token.line,
                column: function_token.column,
                source: Some(self.source.clone()),
            }),
        }).collect();
        // consume endprocedure
        let end_token = self.advance();
        if end_token.token_type != TokenType::EndFunction {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'ENDFUNCTION' after procedure body".to_string(),
                hint: Some("PROCEDURE must be closed with ENDPROCEDURE".to_string()),
                line: end_token.line,
                column: end_token.column,
                source: Some(self.source.clone()),
            });
        }

        self.scope -= 1;

        Ok(Ast::Stmt(Stmt::Function {
            name: identifier_token.lexeme,
            parameters,
            body: BlockStmt {
                statements: body_statements?,
            },
            return_type,
        }) )
    }

    fn parse_return(&mut self) -> Result<Ast, CPSError> {
        let return_token = self.advance(); // consume 'return'
        let expr = self.parse_expr(0)?;

        let expr = ast_to_expr(expr).map_err(|_| CPSError {
            error_type: ErrorType::Syntax,
            message: "Expected an expression after RETURN".to_string(),
            hint: Some("RETURN must be followed by a valid expression".to_string()),
            line: return_token.line,
            column: return_token.column,
            source: Some(self.source.clone()),
        })?;

        Ok(Ast::Stmt(Stmt::Return { value: Box::new(expr) }))
    }


    fn parse_call(&mut self) -> Result<Ast, CPSError> {
        self.advance(); // consume 'call'
        let identifier_token = self.advance();
        if identifier_token.token_type != TokenType::Identifier {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected an identifier after 'CALL'".to_string(),
                hint: Some("CALL must be followed by a valid function or procedure name".to_string()),
                line: identifier_token.line,
                column: identifier_token.column,
                source: Some(self.source.clone()),
            })
        }
        
        // find arguments
        let open_bracket = self.advance();
        if open_bracket.token_type != TokenType::LParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected '(' after function/procedure name".to_string(),
                hint: Some("CALL arguments must be enclosed in parentheses".to_string()),
                line: open_bracket.line,
                column: open_bracket.column,
                source: Some(self.source.clone()),
            });
        }

        let mut arguments: Vec<Expr> = Vec::new();
        while self.peek(0).token_type != TokenType::RParen {
            let arg = self.parse_expr(0)?;
            arguments.push(ast_to_expr(arg)?);

            // check for comma after 
            if self.peek(0).token_type == TokenType::Comma {
                self.advance(); // consume comma
            } else {
                break;
            }
        }
        let close_bracket = self.advance();
        if close_bracket.token_type != TokenType::RParen {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ')' after function/procedure arguments".to_string(),
                hint: Some("CALL arguments must be enclosed in parentheses".to_string()),
                line: close_bracket.line,
                column: close_bracket.column,
                source: Some(self.source.clone()),
            });
        }

        Ok(Ast::Stmt(Stmt::Call { name: identifier_token.lexeme, arguments }) )
    }

    fn parse_array_type(&mut self) -> Result<Type, CPSError> {
        let open_square = self.advance();
        if open_square.token_type != TokenType::LSquare {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected '[' after 'ARRAY'".to_string(),
                hint: Some("Array type must specify dimensions using brackets".to_string()),
                line: open_square.line,
                column: open_square.column,
                source: Some(self.source.clone()),
            });
        }

        let lower_bound = self.parse_expr(0);
        let colon_token = self.advance();
        if colon_token.token_type != TokenType::Colon {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ':' in array dimension declaration".to_string(),
                hint: Some("Array dimensions must be specified as [lower:upper]".to_string()),
                line: colon_token.line,
                column: colon_token.column,
                source: Some(self.source.clone()),
            });
        }
        let upper_bound = self.parse_expr(0);
        let close_square = self.advance();
        let mut bounds_2d = None;
        if close_square.token_type == TokenType::Comma {
            // [lower:upper, lower:upper]
            // parsing a 2d array
            let lower_bound_2 = self.parse_expr(0)?;
            let colon_token_2 = self.advance();
            if colon_token_2.token_type != TokenType::Colon {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected ':' in array dimension declaration".to_string(),
                    hint: Some("Array dimensions must be specified as [lower:upper]".to_string()),
                    line: colon_token_2.line,
                    column: colon_token_2.column,
                    source: Some(self.source.clone()),
                });
            }
            let upper_bound_2 = self.parse_expr(0)?;
            let close_square_2 = self.advance();
            if close_square_2.token_type != TokenType::RSquare {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected ']' after array dimension declaration".to_string(),
                    hint: Some("Array type must specify dimensions using brackets".to_string()),
                    line: close_square_2.line,
                    column: close_square_2.column,
                    source: Some(self.source.clone()),
                });
            }

            bounds_2d = Some((Box::new(ast_to_expr(lower_bound_2)?), Box::new(ast_to_expr(upper_bound_2)?)));
        }
        else if close_square.token_type != TokenType::RSquare {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ']' after array dimension declaration".to_string(),
                hint: Some("Array type must specify dimensions using brackets".to_string()),
                line: close_square.line,
                column: close_square.column,
                source: Some(self.source.clone()),
            });
        }
        let of_type_token = self.advance();
        if of_type_token.token_type != TokenType::Of {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'OF' after array dimension declaration".to_string(),
                hint: Some("Array type must specify the base type using 'OF'".to_string()),
                line: of_type_token.line,
                column: of_type_token.column,
                source: Some(self.source.clone()),
            });
        }
        let base_type_token = self.advance();
        let base_type = match base_type_token.token_type {
            TokenType::Integer => Type::Integer,
            TokenType::Real => Type::Real,
            TokenType::String => Type::String,
            TokenType::Char => Type::Char,
            TokenType::Boolean => Type::Boolean,
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected a valid data type for array base type".to_string(),
                    hint: None,
                    line: base_type_token.line,
                    column: base_type_token.column,
                    source: Some(self.source.clone()),
                });
            }
        };
        Ok(Type::Array(ArrayType {
            lower_bound: Box::new(ast_to_expr(lower_bound?)?),
            upper_bound: Box::new(ast_to_expr(upper_bound?)?),
            bounds_2d,
            base_type: Box::new(base_type),
        }))
    }

    fn parse_close_file(&mut self) -> Result<Ast, CPSError> {
        let _ = self.advance();
        let filename_expr = self.parse_expr(0)?;
        return Ok(Ast::Stmt(Stmt::CloseFile { filename: Box::new(ast_to_expr(filename_expr)?) }) );
    }

    fn parse_open_file(&mut self) -> Result<Ast, CPSError> {
        let _ = self.advance(); // consume 'open'
        let filename_expr = self.parse_expr(0)?;
        let for_token = self.advance();
        if for_token.token_type != TokenType::For {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected 'FOR' after filename in OPEN statement".to_string(),
                hint: Some("OPEN statement must specify the mode using 'FOR'".to_string()),
                line: for_token.line,
                column: for_token.column,
                source: Some(self.source.clone()),
            });
        }

        let mode_token = self.advance();
        let mode;
        match mode_token.token_type {
            TokenType::Read => mode = FileMode::Read,
            TokenType::Write => mode = FileMode::Write, 
            TokenType::Append => mode = FileMode::Append,
            _ => return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected a valid file mode (READ, WRITE, APPEND) in OPEN statement".to_string(),
                hint: None,
                line: mode_token.line,
                column: mode_token.column,
                source: Some(self.source.clone()),
            }),
        };

        return Ok(Ast::Stmt(Stmt::OpenFile {
            filename: Box::new(ast_to_expr(filename_expr)?),
            mode,
        }) );
    }

    fn parse_writefile(&mut self) -> Result<Ast, CPSError> {
        let _ = self.advance();
        let filename_expr = self.parse_expr(0)?;
        let comma = self.advance();
        if comma.token_type != TokenType::Comma {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ',' after filename in WRITEFILE statement".to_string(),
                hint: Some("WRITEFILE statement must specify the content expression after the filename, separated by a comma".to_string()),
                line: comma.line,
                column: comma.column,
                source: Some(self.source.clone()),
            });
        }
        let expr = self.parse_expr(0)?;

        return Ok(Ast::Stmt(Stmt::WriteFile {
            filename: Box::new(ast_to_expr(filename_expr)?),
            value: Box::new(ast_to_expr(expr)?),
        }) );
    }

    fn parse_readfile(&mut self) -> Result<Ast, CPSError> {
        let _ = self.advance();
        let filename_expr = self.parse_expr(0)?;
        let comma = self.advance();
        if comma.token_type != TokenType::Comma {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected ',' after filename in READFILE statement".to_string(),
                hint: Some("READFILE statement must specify the content expression after the filename, separated by a comma".to_string()),
                line: comma.line,
                column: comma.column,
                source: Some(self.source.clone()),
            });
        }
        let target_expr = self.parse_expr(0)?;
        return Ok(Ast::Stmt(Stmt::ReadFile {
            filename: Box::new(ast_to_expr(filename_expr)?),
            target: Box::new(ast_to_expr(target_expr)?),
        }));
    }


    fn parse_type_declaration(&mut self) -> Result<Ast, CPSError> {
        let _ = self.advance(); // consume 'type'

        let identifier_token = self.advance();
        if identifier_token.token_type != TokenType::Identifier {
            return Err(CPSError {
                error_type: ErrorType::Syntax,
                message: "Expected an identifier after 'TYPE'".to_string(),
                hint: Some("TYPE must be followed by a valid name".to_string()),
                line: identifier_token.line,
                column: identifier_token.column,
                source: Some(self.source.clone()),
            });
        }

        let next_token = self.advance();
        // if the next token is a '=', it's either a enumerated, pointer or set type declaration. 
        // otherwise, it's a record type declaration

        if next_token.token_type == TokenType::Equal {
            // enumerated, pointer or set type declaration
            
            // if the next token is a '(', it's an enumerated type declaration
            let next_token = self.advance();
            match next_token.token_type {
                TokenType::LParen => {
                    // enumerated token type

                    let mut enum_values: Vec<String> = Vec::new();
                    while self.peek(0).token_type != TokenType::RParen {
                        let enum_value_token = self.advance();
                        if enum_value_token.token_type != TokenType::Identifier {
                            return Err(CPSError {
                                error_type: ErrorType::Syntax,
                                message: "Expected an identifier in enumerated type declaration".to_string(),
                                hint: Some("Enumerated type declarations must specify their values as identifiers enclosed in parentheses".to_string()),
                                line: enum_value_token.line,
                                column: enum_value_token.column,
                                source: Some(self.source.clone()),
                            });
                        }

                        enum_values.push(enum_value_token.lexeme);

                        // check for comma after each value
                        if self.peek(0).token_type == TokenType::Comma {
                            self.advance(); // consume comma
                        } else {
                            break;
                        }
                    }

                    return Ok(Ast::Stmt(Stmt::TypeDef {
                        type_definition: TypeDefinition::Enumerated { name: identifier_token.lexeme, values: enum_values },
                    }) );
                }
                TokenType::Caret => {
                    // pointer type declaration

                    let type_ = self.advance();
                    match type_.token_type {
                        TokenType::Integer => {
                            return Ok(Ast::Stmt(Stmt::TypeDef {
                                type_definition: TypeDefinition::Pointer { name: identifier_token.lexeme, points_to: Box::new(Type::Integer) },
                            }) );
                        }
                        TokenType::Real => {
                            return Ok(Ast::Stmt(Stmt::TypeDef {
                                type_definition: TypeDefinition::Pointer { name: identifier_token.lexeme, points_to: Box::new(Type::Real) },
                            }) );
                        }
                        TokenType::String => {
                            return Ok(Ast::Stmt(Stmt::TypeDef {
                                type_definition: TypeDefinition::Pointer { name: identifier_token.lexeme, points_to: Box::new(Type::String) },
                            }) );
                        }
                        TokenType::Char => {
                            return Ok(Ast::Stmt(Stmt::TypeDef {
                                type_definition: TypeDefinition::Pointer { name: identifier_token.lexeme, points_to: Box::new(Type::Char) },
                            }) );
                        }
                        TokenType::Boolean => {
                            return Ok(Ast::Stmt(Stmt::TypeDef {
                                type_definition: TypeDefinition::Pointer { name: identifier_token.lexeme, points_to: Box::new(Type::Boolean) },
                            }) );
                        }
                        _ => {
                            return Err(CPSError {
                                error_type: ErrorType::Syntax,
                                message: "Expected a valid data type after '^' in pointer type declaration".to_string(),
                                hint: None,
                                line: type_.line,
                                column: type_.column,
                                source: Some(self.source.clone()),
                            });
                        }
                    }
                }
                TokenType::Set => {
                    // set type declaration

                    let of_token = self.advance();
                    if of_token.token_type != TokenType::Of {
                        return Err(CPSError {
                            error_type: ErrorType::Syntax,
                            message: "Expected 'OF' after 'SET' in set type declaration".to_string(),
                            hint: Some("SET type declaration must specify the base type using 'OF'".to_string()),
                            line: of_token.line,
                            column: of_token.column,
                            source: Some(self.source.clone()),
                        });
                    }

                    let base_type_token = self.advance();
                    let base_type = match base_type_token.token_type {
                        TokenType::Integer => Type::Integer,
                        TokenType::Real => Type::Real,
                        TokenType::String => Type::String,
                        TokenType::Char => Type::Char,
                        TokenType::Boolean => Type::Boolean,
                        _ => {
                            return Err(CPSError {
                                error_type: ErrorType::Syntax,
                                message: "Expected a valid data type for set base type".to_string(),
                                hint: None,
                                line: base_type_token.line,
                                column: base_type_token.column,
                                source: Some(self.source.clone()),
                            });
                        }
                    };

                    return Ok(Ast::Stmt(Stmt::TypeDef {
                        type_definition: TypeDefinition::Set { name: identifier_token.lexeme, base_type: Box::new(base_type) },
                    }) );
                }
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected '(', '^' or 'SET' after '=' in type declaration".to_string(),
                        hint: Some("After the '=' in a type declaration, you must specify the type definition, which can be an enumerated type (starting with '('), a pointer type (starting with '^') or a set type (starting with 'SET')".to_string()),
                        line: next_token.line,
                        column: next_token.column,
                        source: Some(self.source.clone()),
                    });
                }
            }

        } else {
            // record type declaration
            let mut fields: Vec<(String, Box<Type>)> = Vec::new();

            while self.peek(0).token_type != TokenType::EndType {
                // consume declare 
                let declare_token = self.advance();
                if declare_token.token_type != TokenType::Declare {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected 'DECLARE' in record type declaration".to_string(),
                        hint: Some("Record type declarations must specify their fields using 'DECLARE'".to_string()),
                        line: declare_token.line,
                        column: declare_token.column,
                        source: Some(self.source.clone()),
                    });
                }

                let field_identifier_token = self.advance();
                if field_identifier_token.token_type != TokenType::Identifier {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected an identifier for field name in record type declaration".to_string(),
                        hint: Some("Each field in a record type declaration must have a valid name".to_string()),
                        line: field_identifier_token.line,
                        column: field_identifier_token.column,
                        source: Some(self.source.clone()),
                    });
                }

                let colon_token = self.advance();
                if colon_token.token_type != TokenType::Colon {
                    return Err(CPSError {
                        error_type: ErrorType::Syntax,
                        message: "Expected ':' after field name in record type declaration".to_string(),
                        hint: Some("Each field declaration in a record type must have the format 'fieldName: fieldType'".to_string()),
                        line: colon_token.line,
                        column: colon_token.column,
                        source: Some(self.source.clone()),
                    });
                }

                let field_type_token = self.advance();
                let field_type = match field_type_token.token_type {
                    TokenType::Integer => Box::new(Type::Integer),
                    TokenType::Real => Box::new(Type::Real),
                    TokenType::String => Box::new(Type::String),
                    TokenType::Char => Box::new(Type::Char),
                    TokenType::Boolean => Box::new(Type::Boolean),
                    TokenType::Array => Box::new(self.parse_array_type()?),
                    _ => {
                        return Err(CPSError {
                            error_type: ErrorType::Syntax,
                            message: "Expected a valid data type for field type in record type declaration".to_string(),
                            hint: None,
                            line: field_type_token.line,
                            column: field_type_token.column,
                            source: Some(self.source.clone()),
                        });
                    }
                };

                fields.push((field_identifier_token.lexeme, field_type) );
            }

            let end_type_token = self.advance();
            if end_type_token.token_type != TokenType::EndType {
                return Err(CPSError {
                    error_type: ErrorType::Syntax,
                    message: "Expected 'ENDTYPE' after record type declaration".to_string(),
                    hint: Some("Record type declarations must be closed with 'ENDTYPE'".to_string()),
                    line: end_type_token.line,
                    column: end_type_token.column,
                    source: Some(self.source.clone()),
                });
            }

            return Ok(Ast::Stmt(Stmt::TypeDef {
                type_definition: TypeDefinition::Record { name: identifier_token.lexeme, fields },
             }) );

        }
    }
}


pub fn ast_to_expr(ast: Ast) -> Result<Expr, CPSError> {
    match ast {
        Ast::Expression(e) => Ok(e),
        Ast::Identifier(id) => Ok(Expr::Literal(Value::Identifier(id))),
        _ => Err(CPSError {
            error_type: ErrorType::Syntax,
            message: "Expected an expression".to_string(),
            hint: Some("Make sure to provide a valid expression".to_string()),
            line: 0,
            column: 0,
            source: None, // TOOD: <- fix this
        }),
    }
}
