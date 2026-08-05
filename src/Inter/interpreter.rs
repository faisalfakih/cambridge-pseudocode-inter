use std::cmp::Ordering;
use std::collections::HashMap;
// use std::fs::{self, File};
use std::{cell::RefCell, rc::Rc};

// Thread+channel web mode is not available on WASM.
#[cfg(not(target_arch = "wasm32"))]
use crate::Inter::web::{WebContext, WebEvent};

use crate::errortype::{CPSError, ErrorType};
use crate::Inter::cps::{ArrayType, Date, Environment, Function, Type, Value};
use crate::Lexer::lexer::TokenType;
use crate::Parser::ast::{Ast, BinaryExpr, BlockStmt, CaseCondition, Expr, FileMode, Stmt, PassingValue};
use crate::Parser::parser::ast_to_expr;

const BUILTIN_FUNCTIONS: &[&str] = &[
    "RIGHT",
    "LENGTH",
    "MID",
    "SUBSTRING",
    "LCASE",
    "UCASE",
    "INT",
    "RAND",
    "NUM_TO_STR",
    "STR_TO_NUM",
    "IS_NUM",
    "ASC",
    "CHR",
];

/// A file living in memory (used for file I/O in web / WASM modes, and as the "write log" for replay mode).
#[derive(Clone, Debug)]
pub struct VirtualFile {
    // Lines stored in the file (written lines, or pre-loaded content for reads).
    pub lines: Vec<String>,
    // Next line index to serve on `READFILE`.
    pub read_pos: usize,
    pub write_pos: usize,
    pub mode: FileMode,
    pub open: bool,
    // True if the program ever wrote or appended to this file.
    pub was_written: bool,
}

/// State shared between the interpreter and `StepInterpreter` during a replay
/// run. The interpreter reads from `inputs` and writes signals back via the
/// `ErrorType::StepOutput` / `StepNeedsInput` error variants.
#[derive(Debug)]
pub struct ReplayContext {
    pub inputs: Vec<String>,
    pub input_pos: usize,
    pub output_skip: usize,
    pub rand_log: Vec<Value>,
    pub rand_pos: usize,
    pub virtual_fs: Rc<RefCell<HashMap<String, VirtualFile>>>,
}

#[derive(Debug)]
pub struct Interpreter {
    current_env: Rc<RefCell<Environment>>,
    source: String,
    /// Thread+channel bridge for the server-side web mode (non-WASM only).
    #[cfg(not(target_arch = "wasm32"))]
    web_ctx: Option<WebContext>,
    /// Replay context for the step-based WASM mode.
    replay_ctx: Option<Rc<RefCell<ReplayContext>>>,
}

impl Interpreter {
    pub fn new(source: String) -> Self {
        Interpreter {
            current_env: Environment::new_global(),
            source,
            #[cfg(not(target_arch = "wasm32"))]
            web_ctx: None,
            replay_ctx: None,
        }
    }

    /// Create an interpreter that communicates with a web client via channels
    /// instead of using stdin/stdout.  Not available in WASM builds.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_web(source: String, ctx: WebContext) -> Self {
        Interpreter {
            current_env: Environment::new_global(),
            source,
            web_ctx: Some(ctx),
            replay_ctx: None,
        }
    }

    pub fn new_replay(source: String, ctx: Rc<RefCell<ReplayContext>>) -> Self {
        Interpreter {
            current_env: Environment::new_global(),
            source,
            #[cfg(not(target_arch = "wasm32"))]
            web_ctx: None,
            replay_ctx: Some(ctx),
        }
    }

    fn is_web_mode(&self) -> bool {
        if self.replay_ctx.is_some() {
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.web_ctx.is_some() {
            return true;
        }
        false
    }

    fn emit_output(&self, output: String) -> Result<(), CPSError> {
        if let Some(ref ctx_rc) = self.replay_ctx {
            let mut ctx = ctx_rc.borrow_mut();
            if ctx.output_skip > 0 {
                ctx.output_skip -= 1;
                return Ok(());
            }
            return Err(CPSError {
                error_type: ErrorType::StepOutput(output),
                message: String::new(),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref ctx) = self.web_ctx {
            ctx.event_tx.send(WebEvent::Output(output)).map_err(|_| CPSError {
                error_type: ErrorType::Runtime,
                message: "Web client disconnected".to_string(),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            })?;
            return Ok(());
        }
        println!("{}", output);
        Ok(())
    }

    fn read_input(&self, variable_name: &str) -> Result<String, CPSError> {
        if let Some(ref ctx_rc) = self.replay_ctx {
            let mut ctx = ctx_rc.borrow_mut();
            if ctx.input_pos < ctx.inputs.len() {
                let input = ctx.inputs[ctx.input_pos].clone();
                ctx.input_pos += 1;
                return Ok(input);
            } else {
                return Err(CPSError {
                    error_type: ErrorType::StepNeedsInput(variable_name.to_string()),
                    message: String::new(),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref ctx) = self.web_ctx {
            ctx.event_tx.send(WebEvent::NeedsInput {
                variable: variable_name.to_string(),
            }).map_err(|_| CPSError {
                error_type: ErrorType::Runtime,
                message: "Web client disconnected".to_string(),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            })?;
            return ctx.input_rx.recv().map_err(|_| CPSError {
                error_type: ErrorType::Runtime,
                message: "Web client disconnected while waiting for input".to_string(),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            });
        }
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        Ok(input.trim().to_string())
    }

    pub fn interpret(&mut self, ast_nodes: Vec<Ast>) -> Result<(), CPSError> {
        for node in ast_nodes.into_iter() {
            self.evaluate_ast(node)?;
        }
        Ok(())
    }

    pub fn interpret_slice(&mut self, ast_nodes: &[Ast]) -> Result<(), CPSError> {
        for node in ast_nodes {
            self.evaluate_ast_ref(node)?;
        }
        Ok(())
    }

    fn evaluate_ast_ref(&mut self, node: &Ast) -> Result<Value, CPSError> {
        match node {
            Ast::Expression(expr) => self.evaluate_expr(expr),
            Ast::Identifier(name) => {
                match self.current_env.borrow().get(name) {
                    Some(value) => Ok(value),
                    None => Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Undefined identifier: {}", name),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    }),
                }
            },
            Ast::Stmt(stmt) => {
                match self.evaluate_stmt(stmt) {
                    Ok(_) => Ok(Value::Boolean(true)),
                    Err(e) => Err(e),
                }
            },
        }
    }

    fn evaluate_ast(&mut self, node: Ast) -> Result<Value, CPSError> {
        match node {
            Ast::Expression(expr) => {
                self.evaluate_expr(&expr)
            },
            Ast::Identifier(name) => {
                match self.current_env
                    .borrow()
                    .get(&name) {
                        Some(value) => Ok(value),
                        None => Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Undefined identifier: {}", name),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        }),
                    }
            },
            Ast::Stmt(stmt) => {
                match self.evaluate_stmt(&stmt) {
                    Ok(_) => Ok(Value::Boolean(true)),
                    Err(e) => Err(e),
                }
            },
        }
        // Ok(Value::Boolean(true)) // Placeholder return value
    }

    fn evaluate_stmt(&mut self, statement: &crate::Parser::ast::Stmt) -> Result<(), CPSError> {
        // Stmts are returned as At to show where in the file the error has occured so that user errors are returned at nicely.
        // Unwrapped in a loop rather than in its own arm so a statement costs no extra stack frame.
        let mut statement = statement;
        let mut position = None;
        while let Stmt::At { line, column, inner } = statement {
            position = Some((*line, *column));
            statement = inner;
        }

        let result = match statement {
            Stmt::Output { target } => self.evaluate_output_stmt(target),
            Stmt::Decleration { identifier, type_ } => self.evaluate_declaration_stmt(identifier, type_),
            Stmt::Assignment { identifier, array_index, value } => self.evaluate_assignment_stmt(identifier, value, array_index),
            Stmt::Input { identifier } => self.evaluate_input_stmt(identifier),
            Stmt::If { condition, then_branch, else_branch } => self.evaluate_if_stmt(condition, then_branch, else_branch),
            Stmt::Case { identifier, cases, otherwise } => self.evaluate_case_stmt(identifier, cases, otherwise),
            Stmt::While { condition, body } => self.evaluate_while_stmt(condition, body),
            Stmt::Repeat { body, until } => self.evaluate_repeat_stmt(until, body),
            Stmt::Procedure { name, parameters, body } => self.evaluate_procedure(name, parameters, body),
            Stmt::Function { name, parameters, return_type, body } => self.evaluate_function(name, parameters, return_type.to_owned(), body),
            Stmt::Call { name, arguments } => self.evaluate_call(name, arguments).map(|_| ()),
            Stmt::Block(block) => block.statements.iter().try_for_each(|stmt| self.evaluate_stmt(stmt)),
            Stmt::For { identifier, start, end, body, step } => self.evaluate_for(identifier, start, end, body, step),
            Stmt::OpenFile { filename, mode  } => self.evaluate_open_file(filename, mode),
            Stmt::CloseFile { filename } => self.evaluate_close_file(filename),
            Stmt::WriteFile { filename, value } => self.evaluate_write_file(filename, value),
            Stmt::ReadFile { filename, target } => self.evaluate_read_file(filename, target),
            Stmt::Return { value } => match self.evaluate_expr(value) {
                Ok(val) => Err(CPSError {
                    error_type: ErrorType::Return(val),
                    message: String::new(),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
                Err(e) => Err(e),
            },
            Stmt::Constant { identifier, value } => self.evaluate_constant(identifier, value),
            Stmt::EnumDeclaration { identifier, variants } => self.evaluate_enum_declaration(identifier, variants),
            Stmt::At { .. } => unreachable!(),
        };

        match position {
            Some((line, column)) => result.map_err(|e| e.locate(line, column, &self.source)),
            None => result,
        }
    }

    fn evaluate_assignment_stmt(&mut self, identifier: &String, value: &Ast, array_index: &Option<(Box<Expr>, Option<Box<Expr>>)>) -> Result<(), CPSError> {
        let value_expression = match value {
            Ast::Expression(expr) => expr,
            Ast::Identifier(name) => {
                let v = self.current_env
                    .borrow()
                    .get(name)
                    .ok_or_else(|| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Undefined identifier: {}", name),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })?;
                let expr = ast_to_expr(Ast::Expression(Expr::Literal(v)))?;
                &expr.clone()

            }
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Invalid assignment value for '{}': {:?}", identifier, value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        };


        let mut val = self.evaluate_expr(value_expression)?;

        let expected_type = self.current_env.borrow_mut().get_type(identifier)?;

        let actual_type = self.find_actual_type(&val, identifier)?;


        let converted_val = match (&val, &expected_type, &actual_type) {
            (Value::Real(r), Type::Integer, Type::Real) =>  {
                if (*r).fract() == 0.0 {
                    Value::Integer(*r as i64) 
                } else {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!(
                            "Type mismatch: cannot assign {:?} to variable '{}' of type {:?}",
                            actual_type, identifier, expected_type
                        ),
                        hint: Some(format!(
                                "A REAL can only be assigned to an INTEGER when it has no fractional part. \
                                Use INT({}) if you want to truncate toward zero.", r
                        )),
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
            },

            (Value::Integer(i), Type::Real, Type::Integer) => Value::Real(*i as f64),

            _ if actual_type == expected_type => val.clone(),


            (_, Type::Array(arr_type), _) if array_index.is_some() => {
                let can_be_converted = check_if_type_can_be_converted(&val, &*arr_type.base_type);
                if actual_type == *arr_type.base_type || can_be_converted {
                    if can_be_converted {
                        val = convert_values_to_base_type(&val, &*arr_type.base_type)?;
                    }
                    val.clone()
                } else {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!(
                            "Type mismatch: cannot assign {:?} to array element of type {:?}",
                            actual_type, arr_type.base_type
                        ),
                        hint: Some("Array element type must match the array's base type".to_string()),
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
            }


            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!(
                        "Type mismatch: cannot assign {:?} to variable '{}' of type {:?}",
                        actual_type, identifier, expected_type
                    ),
                    hint: Some("The value type must match the declared variable type".to_string()),
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        };

        match array_index {
            Some(indices) => {
                let idx = &indices.0;
                let col = &indices.1;
                let index_value = self.evaluate_expr(&*idx)?;
                let column_value = match col {
                    Some(col_expr) => Some(self.evaluate_expr(&*col_expr)?),
                    None => None,
                };

                let index_int = match index_value {
                    Value::Integer(n) => n as isize,
                    Value::Real(r) => {
                        if r.fract() != 0.0 {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Array index must be an integer, got real number: {}", r),
                                hint: None,
                                line: 0,
                                column: 0,
                                source: None,
                            });
                        }
                        r as isize
                    }
                    _ => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Array index must be an integer, got: {:?}", index_value),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                };

                let column_int = match column_value {
                    Some(Value::Integer(n)) => Some(n as isize),
                    Some(Value::Real(r)) => {
                        if r.fract() != 0.0 {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Array column index must be an integer, got real number: {}", r),
                                hint: None,
                                line: 0,
                                column: 0,
                                source: None,
                            });
                        }
                        Some(r as isize)
                    }
                    Some(_) => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Array column index must be an integer, got: {:?}", column_value),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                    None => None,
                };

                let column_usize = match column_int {
                    Some(c) if c >= 0 => Some(c as usize),
                    Some(_) => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Array column index cannot be negative, got: {}", column_int.unwrap()),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                    None => None,
                };

                // now set the array element at index
                self.current_env
                    .borrow_mut()
                    .set_array_element(identifier, index_int as usize, column_usize, converted_val)
                    .map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to assign value to array element '{}[{}]': {}", identifier, index_int, e.message),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })
            }
            None => {
            self.current_env
                .borrow_mut()
                .set(identifier, converted_val)
                .map_err(|e| CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Failed to assign value to '{}': {}", identifier, e.message),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                })
            }
        }

        
    }

    fn find_actual_type(&self, val: &Value, identifier: &String) -> Result<Type, CPSError> {
        // Identifiers and functions are not assignable, so they keep their own messages
        match val {
            Value::Identifier(_) => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot assign unresolved identifier to '{}'", identifier),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
            Value::Function(_) => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot assign function to '{}'", identifier),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
            other => other.type_of().map_err(|mut e| {
                e.message = format!("{} (variable '{}')", e.message, identifier);
                e
            }),
        }
    }

    fn evaluate_for(&mut self, identifier: &String, start: &Expr, end: &Expr, body: &BlockStmt, step: &Expr) -> Result<(), CPSError> {
        let start_value = self.evaluate_expr(start)?;
        let end_value = self.evaluate_expr(end)?;
        let step_value = self.evaluate_expr(step)?;

        let start_int;
        let end_int;
        let step_int;

        match start_value {
            Value::Integer(n) => start_int = n,
            Value::Real(n) => {
                if n.fract() != 0.0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("FOR loop start value must be an integer, got real number: {}", n),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                start_int = n as i64;
            }
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("FOR loop start value must be an integer, got: {:?}", start_value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }
        match end_value {
            Value::Integer(n) => end_int = n,
            Value::Real(n) => {
                if n.fract() != 0.0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("FOR loop end value must be an integer, got real number: {}", n),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                end_int = n as i64;
            }
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("FOR loop end value must be an integer, got: {:?}", end_value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }
        match step_value {
            Value::Integer(n) => step_int = n,
            Value::Real(n) => {
                if n.fract() != 0.0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("FOR loop step value must evaluate to an integer number, got: {:?}", step_value),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                step_int = n as i64;
            },
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("FOR loop step value must evaluate to an integer number, got: {:?}", step_value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }

        // first declare the loop variable
        self.current_env
            .borrow_mut()
            .define(identifier.to_owned(), Value::Integer(start_int.to_owned()))?;

        // if step int is 0, panic
        if step_int == 0 {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: "The STEP in a for loop cannot be equal to 0.".to_string(),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            })
        }

        let mut i = start_int;
        while (step_int > 0 && i <= end_int) || (step_int < 0 && i >= end_int) {
            self.current_env
                .borrow_mut()
                .set(identifier, Value::Integer(i))
                .map_err(|e| CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Failed to assign value to loop variable '{}': {}", identifier, e.message),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                })?;

            for stmt in &body.statements {
                self.evaluate_stmt(stmt)?;
            }

            i += step_int;
        }

        Ok(())
    }

    fn evaluate_output_stmt(&mut self, target: &Expr) -> Result<(), CPSError> {
        let value = match self.evaluate_expr(target)? {
            Value::Identifier(id) => match self.current_env.borrow().get(&id) {
                Some(v) => v.to_output_string("output")?,
                None => {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Undefined identifier: {}", id),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
            },
            other => other.to_output_string("output")?,
        };
        self.emit_output(value)
    }

    fn evaluate_input_stmt(&mut self, identifier: &Box<Expr>) -> Result<(), CPSError> {
        // Determine the variable/array name for the NeedsInput event befor blocking so the web client knows what it is being asked for.
        let var_name = match identifier.as_ref() {
            Expr::Literal(Value::Identifier(iden)) => iden.clone(),
            Expr::ArrayAccess { name, .. } => name.clone(),
            _ => String::from("?"),
        };
        let input = self.read_input(&var_name)?;

        match identifier.as_ref() {
            Expr::Literal(name) => {
                match name {
                    Value::Identifier(ref iden) => {
                        // get the type of the identifier
                        
                        let type_ = self.current_env.borrow_mut().get_type(&iden)?;
                        let value = Value::from_input(&input, &type_, &format!("variable '{}'", iden), self.current_env.to_owned())?;

                        self.current_env.borrow_mut()
                            .set(iden, value)
                            .map_err(|e| CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Failed to assign input to '{}': {}", iden, e.message),
                                hint: None,
                                line: 0,
                                column: 0,
                                source: None,
                            })?;
                    }
                    _ => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("INPUT statement requires an identifier, got: {:?}", name),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                } 
                Ok(())
            } 
            Expr::ArrayAccess { name, index, col } => {
                let index_value = self.evaluate_expr(&index)?;

                let index_int = match index_value {
                    Value::Integer(n) => n as isize,
                    Value::Real(r) => {
                        if r.fract() != 0.0 {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Array index must be an integer, got real number: {}", r),
                                hint: None,
                                line: 0,
                                column: 0,
                                source: None,
                            });
                        }
                        r as isize
                    }
                    _ => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Array index must be an integer, got: {:?}", index_value),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                };
                let col_value = match col {
                    Some(col_expr) => Some(self.evaluate_expr(&col_expr)?),
                    None => None,
                };

                 let col_int = match col_value {
                    Some(Value::Integer(n)) => Some(n as isize),
                    Some(Value::Real(r)) => {
                        if r.fract() != 0.0 {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Array column index must be an integer, got real number: {}", r),
                                hint: None,
                                line: 0,
                                column: 0,
                                source: None,
                            });
                        }
                        Some(r as isize)
                    }
                    Some(_) => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Array column index must be an integer, got: {:?}", col_value),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                    None => None,
                };

                let col_usize = match col_int {
                    Some(c) if c >= 0 => Some(c as usize),
                    Some(_) => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Array column index cannot be negative, got: {}", col_int.unwrap()),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                    None => None,
                };

                let type_ = self.current_env.borrow_mut().get_type(&name)?;
                match type_ {
                    Type::Array(arr_type) => {
                        let target = format!("array element '{}[{}]'", name, index_int);
                        let value = Value::from_input(&input, &arr_type.base_type, &target, self.current_env.to_owned())?;

                        self.current_env.borrow_mut()
                            .set_array_element(&name, index_int as usize, col_usize, value)
                            .map_err(|e| CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Failed to assign input to array element '{}[{}]': {}", name, index_int, e.message),
                                hint: None,
                                line: 0,
                                column: 0,
                                source: None,
                            })?;
                    }
                    _ => {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Identifier '{}' is not an array for INPUT statement", name),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }
                }
                Ok(())


            }
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("INPUT statement requires an identifier, got: {:?}", identifier),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }
    }

    fn evaluate_if_stmt(&mut self, condition: &Expr, then_branch: &BlockStmt, else_brach: &Option<BlockStmt>) -> Result<(), CPSError> {
        let cond_value = self.evaluate_expr(condition)?;
        match cond_value {
            Value::Boolean(true) => {
                for stmt in &then_branch.statements {
                    self.evaluate_stmt(stmt)?;
                }
            }
            Value::Boolean(false) => {
                if let Some(else_block) = else_brach {
                    for stmt in &else_block.statements {
                        self.evaluate_stmt(stmt)?;
                    }
                }
            }
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Condition in IF statement did not evaluate to a boolean: {:?}", cond_value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }
        Ok(())
    }

    fn evaluate_case_stmt(
        &mut self, 
        identifier: &Box<Expr>, 
        cases: &Vec<(CaseCondition, BlockStmt)>, 
        otherwise: &Option<BlockStmt>
    ) -> Result<(), CPSError> {
        let mut comparitor_value = self.evaluate_expr(identifier)?;

        match comparitor_value {
            Value::Identifier(iden) => {
                comparitor_value = self.current_env.borrow().get(&iden).ok_or_else(|| CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Undefined identifier in CASE statement: {}", iden),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                })?;
            }
            _ => {}
        }

        for (condition, block) in cases {
            if self.matches_case_condition(&comparitor_value, condition)? {
                for stmt in &block.statements {
                    self.evaluate_stmt(stmt)?;
                }
                return Ok(());
            }
        }

        if let Some(otherwise_block) = otherwise {
            for stmt in &otherwise_block.statements {
                self.evaluate_stmt(stmt)?;
            }
        }

        Ok(())
    }

    fn matches_case_condition(&mut self, value: &Value, condition: &CaseCondition) -> Result<bool, CPSError> {
        match condition {
            CaseCondition::Single(case_expr) => {
                let case_value = self.evaluate_expr(case_expr)?;
                self.values_equal(value, &case_value)
            }
            CaseCondition::Range(start_expr, end_expr) => {
                let start = self.evaluate_expr(start_expr)?;
                let end = self.evaluate_expr(end_expr)?;
                self.value_in_range(value, &start, &end)
            }
        }
    }

    fn values_equal(&self, a: &Value, b: &Value) -> Result<bool, CPSError> {
        let (a_converted, b_converted) = convert_values_to_compatible_types(a, b);
        Ok(values_are_equal(&a_converted, &b_converted).unwrap_or(false))
    }

    fn value_in_range(&mut self, value: &Value, start: &Value, end: &Value) -> Result<bool, CPSError> {
        let (value_conv, start_conv) = convert_values_to_compatible_types(value, start);
        let (value_final, end_conv) = convert_values_to_compatible_types(&value_conv, end);

        match (
            compare_values(&value_final, &start_conv),
            compare_values(&value_final, &end_conv),
        ) {
            (Some(from_start), Some(to_end)) => {
                Ok(from_start != Ordering::Less && to_end != Ordering::Greater)
            }
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Invalid range comparison: {:?} TO {:?} for value {:?}", start, end, value),
                hint: Some("Range values must be compatible types".to_string()),
                line: 0,
                column: 0,
                source: None,
            })
        }
    }


    fn evaluate_while_stmt(&mut self, condition: &Expr, body: &BlockStmt) -> Result<(), CPSError> {
        loop {
            let cond_value = self.evaluate_expr(condition)?;
            match cond_value {
                Value::Boolean(true) => {
                    self.evaluate_stmt(&Stmt::Block(body.to_owned()))?;
                }
                Value::Boolean(false) => break,
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Condition in WHILE statement did not evaluate to a boolean: {:?}", cond_value),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
            }
        }
        Ok(())
    }

    fn evaluate_repeat_stmt(&mut self, condition: &Expr, body: &BlockStmt) -> Result<(), CPSError> {
        loop {
            self.evaluate_stmt(&Stmt::Block(body.to_owned()))?;
            let cond_value = self.evaluate_expr(condition)?;
            match cond_value {
                Value::Boolean(true) => break,
                Value::Boolean(false) => continue,
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Condition in REPEAT statement did not evaluate to a boolean: {:?}", cond_value),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
            }
        }
        Ok(())
    }

    fn evaluate_open_file(&mut self, filename: &Box<Expr>, mode: &FileMode) -> Result<(), CPSError> {
        if let Some(ctx_rc) = self.replay_ctx.clone() {
            let filename_value = self.evaluate_expr(filename)?;
            let filename_str = match filename_value {
                Value::String(s) => s,
                other => return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Filename in OPENFILE statement must evaluate to a string, got: {:?}", other),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            };
            let ctx = ctx_rc.borrow();
            let mut vfs = ctx.virtual_fs.borrow_mut();
            match mode {
                FileMode::Write => {
                    let entry = vfs.entry(filename_str).or_insert_with(|| VirtualFile {
                        lines: Vec::new(),
                        read_pos: 0,
                        write_pos: 0,
                        mode: FileMode::Write,
                        open: false,
                        was_written: false,
                    });
                    entry.lines.clear();
                    entry.mode = FileMode::Write;
                    entry.read_pos = 0;
                    entry.write_pos = 0;
                    entry.open = true;
                }
                FileMode::Append => {
                    let entry = vfs.entry(filename_str).or_insert_with(|| VirtualFile {
                        lines: Vec::new(),
                        read_pos: 0,
                        write_pos: 0,
                        mode: FileMode::Append,
                        open: false,
                        was_written: false,
                    });
                    entry.mode = FileMode::Append;
                    // Reset to 0 so that on replay the existing lines are
                    // consumed (skipped) by the write-pos < lines.len() check
                    // in evaluate_write_file, preventing duplicate appends.
                    entry.write_pos = 0;
                    entry.open = true;
                }
                FileMode::Read => {
                    let entry = vfs.get_mut(&filename_str).ok_or_else(|| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("File '{}' not found; it must be preloaded before opening for read", filename_str),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })?;
                    entry.mode = FileMode::Read;
                    entry.read_pos = 0;
                    entry.open = true;
                }
            }
            return Ok(());
        }
        let filename_value = self.evaluate_expr(filename)?;
        let filename_str;
        match &filename_value {
            Value::String(str) => filename_str = str,
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Filename in OPENFILE statement must evaluate to a string, got: {:?}", filename_value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }

        self.current_env.borrow_mut().openfile(&filename_str, mode)?;
        Ok(())

    }

    fn evaluate_close_file(&mut self, filename: &Box<Expr>) -> Result<(), CPSError> {
        if let Some(ctx_rc) = self.replay_ctx.clone() {
            let filename_value = self.evaluate_expr(filename)?;
            let filename_str = match filename_value {
                Value::String(s) => s,
                other => return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Filename in CLOSEFILE statement must evaluate to a string, got: {:?}", other),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            };
            let ctx = ctx_rc.borrow();
            let mut vfs = ctx.virtual_fs.borrow_mut();
            if let Some(vfile) = vfs.get_mut(&filename_str) {
                vfile.open = false;
                return Ok(());
            }
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot close file '{}': file is not open", filename_str),
                hint: None,
                line: 0, column: 0, source: None,
            });
        }
        let filename_value = self.evaluate_expr(filename)?;
        let filename_str;
        match &filename_value {
            Value::String(str) => filename_str = str,
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Filename in CLOSEFILE statement must evaluate to a string, got: {:?}", filename_value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }

        self.current_env.borrow_mut().closefile(&filename_str)
    }

    fn evaluate_write_file(&mut self, filename: &Box<Expr>, value: &Box<Expr>) -> Result<(), CPSError> {
        if let Some(ctx_rc) = self.replay_ctx.clone() {
            let filename_value = self.evaluate_expr(filename)?;
            let filename_str = match filename_value {
                Value::String(s) => s,
                other => return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Filename in WRITEFILE statement must evaluate to a string, got: {:?}", other),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            };
            let value_to_write = self.evaluate_expr(value)?;
            let line = value_to_write.to_output_string("write")?;
            let ctx = ctx_rc.borrow();
            let mut vfs = ctx.virtual_fs.borrow_mut();
            if let Some(vfile) = vfs.get_mut(&filename_str) {
                if vfile.mode == FileMode::Read {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Cannot write to file '{}': file is opened in read mode", filename_str),
                        hint: None,
                        line: 0, column: 0, source: None,
                    });
                }
                // if the write_pos < lines.len(), then this write is already captured in the log, so we just advance the cursor without writing
                vfile.was_written = true;
                if vfile.write_pos < vfile.lines.len() {
                    vfile.write_pos += 1;
                } else {
                    vfile.lines.push(line);
                    vfile.write_pos += 1;
                }
                return Ok(());
            }
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("File '{}' is not open", filename_str),
                hint: None,
                line: 0, column: 0, source: None,
            });
        }
        let filename_value = self.evaluate_expr(filename)?;
        let filename_str;
        match &filename_value {
            Value::String(str) => filename_str = str,
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Filename in WRITEFILE statement must evaluate to a string, got: {:?}", filename_value),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }

        let value_to_write = self.evaluate_expr(value)?;

        self.current_env.borrow_mut().writefile(&filename_str, &value_to_write)
    }

    /// Stores one line read from a file into the target, converted to the
    /// target's declared type the same way INPUT does.
    fn assign_read_line(&mut self, target: &Expr, line: String) -> Result<(), CPSError> {
        match target {
            Expr::Literal(Value::Identifier(iden)) => {
                let type_ = self.current_env.borrow_mut().get_type(iden)?;
                let value = Value::from_input(&line, &type_, &format!("variable '{}'", iden), self.current_env.to_owned())?;

                self.current_env
                    .borrow_mut()
                    .set(iden, value)
                    .map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to assign READFILE result to '{}': {}", iden, e.message),
                        hint: None,
                        line: 0, column: 0, source: None,
                    })
            }
            Expr::ArrayAccess { name, index, col } => {
                let index_value = self.evaluate_expr(index)?;
                let index_int = self.value_to_index(&index_value, name)?;
                if index_int < 0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Array index cannot be negative for '{}': {}", name, index_int),
                        hint: None,
                        line: 0, column: 0, source: None,
                    });
                }
                let col_int = match col {
                    Some(col_expr) => {
                        let col_value = self.evaluate_expr(col_expr)?;
                        let c =  self.value_to_index(&col_value, name)?;

                        if c < 0 {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Column index cannot be negative for '{}': {}", name, c),
                                hint: None,
                                line: 0, column: 0, source: None,
                            });
                        }
                        Some(c)


                    }
                    None => None,
                };

                let var_type = self.current_env.borrow_mut().get_type(name)?;
                let base_type = match var_type {
                    Type::Array(arr_type) => *arr_type.base_type,
                    _ => return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Identifier '{}' is not an array for READFILE statement", name),
                        hint: None,
                        line: 0, column: 0, source: None,
                    }),
                };

                let target_desc = format!("array element '{}[{}]'", name, index_int);
                let value = Value::from_input(&line, &base_type, &target_desc, self.current_env.to_owned())?;

                self.current_env
                    .borrow_mut()
                    .set_array_element(name, index_int as usize, col_int.map(|c| c as usize), value)
                    .map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to assign READFILE result to array element: {}", e.message),
                        hint: None,
                        line: 0, column: 0, source: None,
                    })
            }
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("READFILE target must be a variable or array element, got: {:?}", target),
                hint: None,
                line: 0, column: 0, source: None,
            }),
        }
    }

    fn evaluate_read_file(&mut self, filename: &Box<Expr>, target: &Box<Expr>) -> Result<(), CPSError> {
        if let Some(ctx_rc) = self.replay_ctx.clone() {
            let filename_value = self.evaluate_expr(filename)?;
            let filename_str = match filename_value {
                Value::String(s) => s,
                other => return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Filename in READFILE must be a string, got: {:?}", other),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            };
            let line = {
                let ctx = ctx_rc.borrow();
                let mut vfs = ctx.virtual_fs.borrow_mut();
                if let Some(vfile) = vfs.get_mut(&filename_str) {
                    if vfile.mode != FileMode::Read {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Cannot read from file '{}': file is not opened in read mode", filename_str),
                            hint: None,
                            line: 0, column: 0, source: None,
                        });
                    }
                    if vfile.read_pos >= vfile.lines.len() {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!("Cannot read past end of file '{}'", filename_str),
                            hint: Some("Check EOF before reading".to_string()),
                            line: 0, column: 0, source: None,
                        });
                    }
                    let line = vfile.lines[vfile.read_pos].clone();
                    vfile.read_pos += 1;
                    line
                } else {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("File '{}' is not open", filename_str),
                        hint: None,
                        line: 0, column: 0, source: None,
                    });
                }
            };
            return self.assign_read_line(target.as_ref(), line);
        }
        let filename_value = self.evaluate_expr(filename)?;
        let filename_str = match &filename_value {
            Value::String(s) => s.clone(),
            _ => return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Filename in READFILE must be a string, got: {:?}", filename_value),
                hint: None,
                line: 0, column: 0, source: None,
            }),
        };

        let line = self.current_env.borrow_mut().readfile(&filename_str)?;

        // assign the line to the target variable
        self.assign_read_line(target.as_ref(), line)
    }

    /// Evaluates an array bound expression to an integer.
    /// `what` names the bound in the error message, e.g. "lower".
    fn evaluate_bound(&mut self, expr: &Expr, what: &str) -> Result<i64, CPSError> {
        match self.evaluate_expr(expr)? {
            Value::Integer(n) => Ok(n),
            Value::Real(r) if r.fract() == 0.0 => Ok(r as i64),
            other => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Array {} bound must be a whole number, got {:?}", what, other),
                hint: Some("Array bounds must evaluate to INTEGER values.".to_string()),
                line: 0, column: 0, source: None,
            }),
        }
    }

    /// Turns a type name written in the source into the type it was declared as.
    fn resolve_type(&self, ty: &Type) -> Result<Type, CPSError> {
        Ok(match ty {
            Type::Named(name) => {
                let env = self.current_env.borrow();
                env.find_type_of_named_type(name)?
            }
            Type::Array(arr) => Type::Array(ArrayType {
                base_type: Box::new(self.resolve_type(&arr.base_type)?),
                ..arr.to_owned()
            }),
            other => other.to_owned(),
        })
    }

    fn resolve_parameters(&self, parameters: &Vec<(String, Type, PassingValue)>) -> Result<Vec<(String, Type, PassingValue)>, CPSError> {
        parameters
            .iter()
            .map(|(name, type_, passing_value)| Ok((name.to_owned(), self.resolve_type(type_)?, passing_value.to_owned())))
            .collect()
    }

    /// The value a variable of this type holds before anything is assigned to it.
    fn default_value(&mut self, ty: &Type) -> Result<Value, CPSError> {
        Ok(match ty { 
            // TODO: Set all variable default types to be NONE, for them to only have Some after being assigned something
            Type::Integer => Value::Integer(0),
            Type::Real    => Value::Real(0.0),
            Type::String  => Value::String(String::new()),
            Type::Boolean => Value::Boolean(false),
            Type::Char    => Value::Char('\0'),
            Type::Date    => Value::Date(Date { day: 1, month: 1, year: 1900 }),
            Type::Enum(type_name) => Value::Enum { type_name: type_name.to_owned(), variant: None },
            Type::Named(name) => {
                // find the actual type
                let type_ = self.current_env.borrow()
                    .find_type_of_named_type(name)?;

                return self.default_value(&type_)
            }
            // Type::Enum(Name) => todo!(),

            Type::Array(arr) => {
                let lower = self.evaluate_bound(&arr.lower_bound, "lower")?;
                let upper = self.evaluate_bound(&arr.upper_bound, "upper")?;

                if upper < lower {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Array upper bound {} cannot be less than lower bound {}", upper, lower),
                        hint: None, line: 0, column: 0, source: None,
                    });
                }
                if lower < 0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Array lower bound {} cannot be negative", lower),
                        hint: Some("Cambridge pseudocode arrays are usually indexed from 1.".to_string()),
                        line: 0, column: 0, source: None,
                    });
                }

                // let row_count = (upper - lower + 1) as usize;
                let row_count = match upper.checked_sub(lower)
                    .and_then(|num| num.checked_add(1))
                    .and_then(|num| usize::try_from(num).ok()) {
                        Some(n) => n,
                        None => 
                        {
                            return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Array size [{}:{}] is too large", lower, upper),
                        hint: None, line: 0, column: 0, source: None,
                    })}
                    };

                let bounds_2d = match &arr.bounds_2d {
                    Some((col_lb_expr, col_ub_expr)) => {
                        let col_lb = self.evaluate_bound(col_lb_expr, "column lower")?;
                        let col_ub = self.evaluate_bound(col_ub_expr, "column upper")?;

                        if col_ub < col_lb {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Column upper bound {} cannot be less than lower bound {}", col_ub, col_lb),
                                hint: None, line: 0, column: 0, source: None,
                            });
                        }
                        if col_lb < 0 {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Array column lower bound {} cannot be negative", col_lb),
                                hint: Some("Cambridge pseudocode arrays are usually indexed from 1.".to_string()),
                                line: 0, column: 0, source: None,
                            });
                        }
                        Some((col_lb as usize, col_ub as usize))
                    }
                    None => None,
                };

                const MAX_ELEMENTS: usize = 16_000_000;
                let total_length = match &bounds_2d {
                    Some((col_lb, col_ub)) => row_count
                        .checked_mul(col_ub - col_lb + 1)
                        .ok_or_else(|| CPSError {
                            error_type: ErrorType::Runtime,
                            message: "Array size is too large".to_string(),
                            hint: None, line: 0, column: 0, source: None,
                        })?,
                    None => row_count,
                };
                if total_length > MAX_ELEMENTS {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Array of {} elements exceeds the supported limit", total_length),
                        hint: Some("Reduce the array bounds.".to_string()),
                        line: 0, column: 0, source: None,
                    });
                }

                // let total_length = match &bounds_2d {
                //     Some((col_lb, col_ub)) => row_count * (col_ub - col_lb + 1),
                //     None => row_count,
                // };

                let element = self.default_value(&arr.base_type)?;

                Value::Array {
                    array: vec![element; total_length],
                    lower_bound: lower as usize,
                    bounds_2d,
                }
            }

            Type::Function => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: "Cannot declare a variable of type Function".to_string(),
                    hint: Some("Procedures and functions are declared with PROCEDURE/FUNCTION, not DECLARE.".to_string()),
                    line: 0, column: 0, source: None,
                });
            }
        })
    }



    fn evaluate_declaration_stmt(&mut self, identifier: &String, type_: &Type) -> Result<(), CPSError> {
        let inital_value = self.default_value(type_)?;

        self.current_env
            .borrow_mut()
            .define(identifier.to_owned(), inital_value)?;
        Ok(())
    }


    fn evaluate_enum_declaration(&mut self, identifier: &String, variants: &Vec<String>) -> Result<(), CPSError> {
        self.current_env.borrow_mut()
            .define_enum(identifier, variants)?;

        Ok(())
    }

    fn evaluate_constant(&mut self, identifier: &String, value: &Value) -> Result<(), CPSError> {
        self.current_env
            .borrow_mut()
            .declare_constant(&identifier, value)?;
        Ok(())
    }



    fn evaluate_procedure(&mut self, identifier: &String, parameters: &Vec<(String, Type, PassingValue)>, body: &BlockStmt) -> Result<(), CPSError> {
        // self.evaluate_declaration_stmt(identifier, &Type::Function)?;

        // first check if procedure is a builtin
        if BUILTIN_FUNCTIONS.contains(&identifier.as_str()) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot redefine builtin function as procedure: {}", identifier),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            });
        }

        let function_value = Value::Function(Function {
            parameters: self.resolve_parameters(parameters)?,
            body: body.to_owned(),
            return_type: None,
        });

        self.current_env
            .borrow_mut()
            .define(identifier.to_owned(), function_value)?;


        // self.evaluate_assignment_stmt(
        //     identifier, 
        //     &Ast::Expression(Expr::Literal(function_value))
        // )?;



        Ok(())
    }

    fn evaluate_call(&mut self, identifier: &String, arguments: &Vec<Expr>) -> Result<Value, CPSError> {
        if identifier == "RAND" {
            let rand_ctx = self.replay_ctx.as_ref().map(Rc::clone);
            if let Some(ctx_rc) = rand_ctx {
                // Always evaluate arguments first to preserve side effects.
                let arg_values: Result<Vec<Value>, CPSError> = arguments
                    .iter()
                    .map(|arg| self.evaluate_expr(arg))
                    .collect();
                let arg_values = arg_values?;

                let maybe_logged = {
                    let ctx = ctx_rc.borrow();
                    if ctx.rand_pos < ctx.rand_log.len() {
                        Some(ctx.rand_log[ctx.rand_pos].clone())
                    } else {
                        None
                    }
                };

                if let Some(val) = maybe_logged {
                    ctx_rc.borrow_mut().rand_pos += 1;
                    return Ok(val);
                }
                let value = match crate::Inter::builtins::call_builtin(identifier.clone(), &arg_values)? {
                    Some(v) => v,
                    None => Value::Boolean(false),
                };
                let mut ctx = ctx_rc.borrow_mut();
                ctx.rand_log.push(value.clone());
                ctx.rand_pos += 1;
                return Ok(value);
            }
        }

        if BUILTIN_FUNCTIONS.contains(&identifier.as_str()) {
            let arg_values: Result<Vec<Value>, CPSError> = arguments
                .iter()
                .map(|arg| self.evaluate_expr(arg))
                .collect();

            let arg_values = arg_values?;
            return match crate::Inter::builtins::call_builtin(identifier.clone(), &arg_values)? {
                Some(value) => Ok(value),
                None => Ok(Value::Boolean(false))
            };
        }


        let func_value = match self.current_env.borrow().get(identifier) {
            Some(Value::Function(func)) => func,
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Undefined function: {}", identifier),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        };

        if func_value.parameters.len() != arguments.len() {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Function '{}' expected {} arguments, got {}", identifier, func_value.parameters.len(), arguments.len()),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            });
        }

        let new_env = Environment::new_child(Rc::clone(&self.current_env));

        for (i, (param_name, param_type, passing_value)) in func_value.parameters.iter().enumerate() {
            match passing_value {
                PassingValue::ByVal => {
                    let arg_value = self.evaluate_expr(&arguments[i])?;

                    if !check_if_type_can_be_converted(&arg_value, param_type) {
                        return Err(CPSError {
                            error_type: ErrorType::Runtime,
                            message: format!(
                                "Type mismatch for parameter '{}': expected {:?}, got {:?}",
                                param_name, param_type, arg_value
                            ),
                            hint: None,
                            line: 0,
                            column: 0,
                            source: None,
                        });
                    }

                    let converted = convert_values_to_base_type(&arg_value, param_type).unwrap_or(arg_value);
                    new_env.borrow_mut().define(param_name.to_owned(), converted)?;
                }
                PassingValue::ByRef => {
                    let current_arg = &arguments[i];
                    match current_arg {
                        Expr::Literal(Value::Identifier(caller_variable_name)) => {
                            // prevent ByRef on constants.
                            if self.current_env.borrow().is_constant(caller_variable_name) {
                                return Err(CPSError {
                                    error_type: ErrorType::Runtime,
                                    message: format!(
                                        "Cannot pass constant '{}' to BYREF parameter '{}' of '{}'",
                                        caller_variable_name, param_name, identifier
                                    ),
                                    hint: Some("Constants cannot be modified. Pass a variable instead, or declare the parameter BYVAL.".to_string()),
                                    line: 0,
                                    column: 0,
                                    source: None,
                                });
                            }

                            // get the address 
                            let address = match self.current_env.borrow_mut().find_address_of_variable(caller_variable_name.to_owned()) {
                                Some(a) => {a},
                                None => {
                                    return Err(CPSError {
                                        error_type: ErrorType::Runtime,
                                        message: format!(
                                            "'{}' is not a variable and cannot be passed BYREF.",
                                            caller_variable_name
                                        ),
                                        hint: Some(
                                            "BYREF parameters must be defined variables, not a literal, expression, or function call."
                                            .to_string(),
                                        ),
                                        line: 0,
                                        column: 0,
                                        source: None,
                                    });

                                }
                            };

                            let arg_value = self.current_env.borrow().get(caller_variable_name).ok_or_else(|| CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Undefined variable '{}'", caller_variable_name),
                                hint: None,
                                line: 0, column: 0, source: None,
                            })?;
                            let actual_type = self.find_actual_type(&arg_value, caller_variable_name)?;

                            if !check_if_types_match_exactly(&actual_type, param_type) {
                                return Err(CPSError {
                                    error_type: ErrorType::Runtime,
                                    message: format!(
                                        "Type mismatch for BYREF parameter '{}' of '{}': expected {:?}, but '{}' is {:?}",
                                        param_name, identifier, param_type, caller_variable_name, actual_type
                                    ),
                                    hint: Some("A BYREF argument must have exactly the same type as the parameter, because they share storage".to_string()),
                                    line: 0, column: 0, source: None,
                                });
                            }

                            new_env.borrow_mut().set_variable_at_address(param_name.clone(), address)?;
                        }
                        _ => {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!(
                                    "Cannot pass argument {} to '{}' by reference: parameter '{}' is BYREF but the argument is not a variable",
                                    i + 1,
                                    identifier,
                                    param_name
                                ),
                                hint: Some("BYREF parameters must be given a plain variable name, not a literal, expression, or function call".to_string()),
                                line: 0,
                                column: 0,
                                source: None,
                            });
                        }
                    }
                }
            }
        }

        let previous_env = Rc::clone(&self.current_env);
        self.current_env = new_env;
        let mut return_value = None;
        for stmt in &func_value.body.statements {
            match self.evaluate_stmt(stmt) {
                Ok(_) => {}
                Err(CPSError { error_type: ErrorType::Return(val), .. }) => {
                    return_value = Some(val);
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        self.current_env = previous_env;

        match (&return_value, &func_value.return_type) {
            (Some(val), Some(expected_type)) => {
                if !check_if_type_can_be_converted(val, expected_type) {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!(
                            "Function '{}' return type mismatch: expected {:?}, got {:?}",
                            identifier, expected_type, val
                        ),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                Ok(val.clone())
            }
            (None, Some(expected_type)) => {
                Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!(
                        "Function '{}' must return a value of type {:?}",
                        identifier, expected_type
                    ),
                    hint: Some("Add a RETURN statement".to_string()),
                    line: 0,
                    column: 0,
                    source: None,
                })
            }
            (Some(_), None) => { // procedure return (error)
                Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Procedure '{}' should not return a value", identifier),
                    hint: Some("Use FUNCTION instead of PROCEDURE if you need to return a value".to_string()),
                    line: 0,
                    column: 0,
                    source: None,
                })
            }
            (None, None) => {
                Ok(Value::Boolean(false)) // procedures return "void"
            }
        }
    }


    fn evaluate_function(&mut self, identifier: &String, parameters: &Vec<(String, Type, PassingValue)>, return_type: Type, body: &BlockStmt) -> Result<(), CPSError> {
        // first check if function is a builtin
        if BUILTIN_FUNCTIONS.contains(&identifier.as_str()) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot redefine builtin function: {}", identifier),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            });
        }

        let function_value = Value::Function(Function {
            parameters: self.resolve_parameters(parameters)?,
            body: body.to_owned(),
            return_type: Some(self.resolve_type(&return_type)?),
        });

        self.current_env
            .borrow_mut()
            .define(identifier.to_owned(), function_value.clone())?;

        Ok(())
    }


    fn evaluate_expr(&mut self, expression: &Expr) -> Result<Value, CPSError> {
        match expression {
            Expr::Binary(expr) => self.evaluate_binary(expr),
            Expr::Literal(value) => self.evaluate_literal(value),
            Expr::Call { name, arguments } => self.evaluate_call(name, arguments),
            Expr::ArrayAccess { name, index, col } => self.evaluate_array_access(name, index, col),
            Expr::EOF { filename } => {
                if let Some(ctx_rc) = self.replay_ctx.clone() {
                    let ctx = ctx_rc.borrow();
                    let vfs = ctx.virtual_fs.borrow();
                    if let Some(vfile) = vfs.get(filename.as_str()) {
                        return Ok(Value::Boolean(vfile.read_pos >= vfile.lines.len()));
                    }
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("File '{}' is not open", filename),
                        hint: Some("Make sure to open the file before checking EOF".to_string()),
                        line: 0, column: 0, source: None,
                    });
                }
                let is_eof = self.current_env.borrow().is_eof(filename)?;
                Ok(Value::Boolean(is_eof))
            }
            // _ => {
            //     return Err(CPSError {
            //         error_type: ErrorType::Runtime,
            //         message: format!("Unsupported expression in interpreter: {:?}", expression),
            //         hint: None,
            //         line: 0,
            //         column: 0,
            //         source: None,
            //     });
            // }
        }
    }

    fn evaluate_array_access(&mut self, name: &String, index: &Box<Expr>, col: &Option<Box<Expr>>) -> Result<Value, CPSError> {
        let index_value = self.evaluate_expr(index)?;
        let index_int = self.value_to_index(&index_value, name)?;

        let col_int = match col {
            Some(col_expr) => {
                let col_value = self.evaluate_expr(col_expr)?;
                Some(self.value_to_index(&col_value, name)?)
            }
            None => None,
        };

        if index_int < 0 {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Array index cannot be negative for '{}': {}", name, index_int),
                hint: None,
                line: 0, column: 0, source: None,
            });
        }

        if let Some(c) = col_int {
            if c < 0 {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Column index cannot be negative for '{}': {}", name, c),
                    hint: None,
                    line: 0, column: 0, source: None,
                });
            }
        }

        self.current_env
            .borrow()
            .get_array_element(name, index_int as usize, col_int.map(|c| c as usize))
    }

    fn value_to_index(&self, value: &Value, name: &str) -> Result<isize, CPSError> {
        match value {
            Value::Integer(n) => Ok(*n as isize),
            Value::Real(r) => {
                if r.fract() != 0.0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Array index must be an integer, got: {}", r),
                        hint: None,
                        line: 0, column: 0, source: None,
                    });
                }
                Ok(*r as isize)
            }
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Array index must be an integer for '{}', got: {:?}", name, value),
                hint: None,
                line: 0, column: 0, source: None,
            }),
        }
    }


    fn evaluate_binary(&mut self, binary: &BinaryExpr) -> Result<Value, CPSError> {
        let mut left = self.evaluate_ast(*binary.left.clone())?;
        let mut right = self.evaluate_ast(*binary.right.clone())?;

        // if !check_if_value_can_be_converted(&left, &right) {
        //     return Err(CPSError {
        //         error_type: ErrorType::Runtime,
        //         message: format!("Incompatible types for binary operation: {:?} and {:?}", left, right),
        //         hint: None,
        //         line: 0,
        //         column: 0,
        //         source: None,
        //     });
        // }

        // convert them to compatible types
        (left, right) = convert_values_to_compatible_types(&left, &right); // allow comparison between int and real


        match binary.operator {
            TokenType::Plus => self.add(left, right),
            TokenType::Minus => self.subtract(left, right),
            TokenType::Asterisk => self.multiply(left, right),
            TokenType::ForwardSlash => self.divide(left, right),
            TokenType::Div => self.integer_divide(left, right),
            TokenType::Mod => self.modulo(left, right),
            TokenType::Caret => self.power(left, right),
            TokenType::Ampersand => self.concatenate(left, right),

            // Comparison operators
            TokenType::Equal => self.equal(left, right),
            TokenType::NotEqual => self.not_equal(left, right),
            TokenType::LessThan => self.less_than(left, right),
            TokenType::LessEqual => self.less_equal(left, right),
            TokenType::GreaterThan => self.greater_than(left, right),
            TokenType::GreaterEqual => self.greater_equal(left, right),

            // Logical operators
            TokenType::And => self.logical_and(left, right),
            TokenType::Or => self.logical_or(left, right),

            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported binary operator in interpreter: {:?}", binary.operator),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }

    }

    fn add(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l + r)),
            (Value::Real(l), Value::Real(r)) => Ok(Value::Real(l + r)),
            (Value::Integer(l), Value::Real(r)) => Ok(Value::Real(l as f64 + r)),
            (Value::Real(l), Value::Integer(r)) => Ok(Value::Real(l + r as f64)),
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for addition: {:?} + {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }

    fn subtract(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l - r)),
            (Value::Real(l), Value::Real(r)) => Ok(Value::Real(l - r)),
            (Value::Integer(l), Value::Real(r)) => Ok(Value::Real(l as f64 - r)),
            (Value::Real(l), Value::Integer(r)) => Ok(Value::Real(l - r as f64)),
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for subtraction: {:?} - {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }

    fn multiply(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l * r)),
            (Value::Real(l), Value::Real(r)) => Ok(Value::Real(l * r)),
            (Value::Integer(l), Value::Real(r)) => Ok(Value::Real(l as f64 * r)),
            (Value::Real(l), Value::Integer(r)) => Ok(Value::Real(l * r as f64)),
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for multiplication: {:?} * {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }

    fn divide(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        let is_zero = match &right {
            Value::Integer(n) => *n == 0,
            Value::Real(f) => *f == 0.0,
            _ => false,
        };

        if is_zero {
            return Err(
                CPSError {
                    error_type: ErrorType::Runtime,
                    message: "Division by zero".to_string(),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                }
            );
        }

        match (left.clone(), right.clone()) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Real(a as f64 / b as f64)),
            (Value::Real(a), Value::Real(b)) => Ok(Value::Real(a / b)),
            (Value::Integer(a), Value::Real(b)) => Ok(Value::Real(a as f64 / b)),
            (Value::Real(a), Value::Integer(b)) => Ok(Value::Real(a / b as f64)),
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for division: {:?} / {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }

    }

    fn integer_divide(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Integer(a), Value::Integer(b)) => {
                if b == 0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: "Division by zero".to_string(),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                Ok(Value::Integer(a / b))
            }
            (Value::Real(a), Value::Real(b)) => {
                if b == 0.0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: "Division by zero".to_string(),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                Ok(Value::Real((a / b).floor()))
            }
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for integer division: {:?} // {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }

    fn modulo(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Integer(a), Value::Integer(b)) => {
                if b == 0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: "Modulo by zero".to_string(),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                Ok(Value::Integer(a % b))
            },
            (Value::Real(a), Value::Real(b)) => {
                if b == 0.0 {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: "Modulo by zero".to_string(),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    });
                }
                Ok(Value::Real(a % b))
            },
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for modulo: {:?} % {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }

    fn power(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a.pow(b as u32))),
            (Value::Real(a), Value::Real(b)) => Ok(Value::Real(a.powf(b))),
            (Value::Integer(a), Value::Real(b)) => Ok(Value::Real((a as f64).powf(b))),
            (Value::Real(a), Value::Integer(b)) => Ok(Value::Real(a.powf(b as f64))),
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for power: {:?} ^ {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }

    fn concatenate(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        let left_str = left.to_output_string("concatenate")?;
        let right_str = right.to_output_string("concatenate")?;

        Ok(Value::String(format!("{}{}", left_str, right_str)))
    }



    fn equal(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        values_are_equal(&left, &right)
            .map(|o| Value::Boolean(o))
            .ok_or_else(|| comparison_error("=", &left, &right))
    }

    fn not_equal(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        values_are_equal(&left, &right)
            .map(|o| Value::Boolean(!o))
            .ok_or_else(|| comparison_error("<>", &left, &right))
    }



    fn less_than(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        compare_values(&left, &right)
            .map(|o| Value::Boolean(o == Ordering::Less))
            .ok_or_else(|| comparison_error("<", &left, &right))
    }

    fn less_equal(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        compare_values(&left, &right)
            .map(|o| Value::Boolean(o == Ordering::Less || o == Ordering::Equal))
            .ok_or_else(|| comparison_error("<=", &left, &right))
    }

    fn greater_than(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        compare_values(&left, &right)
            .map(|o| Value::Boolean(o == Ordering::Greater))
            .ok_or_else(|| comparison_error(">", &left, &right))
    }

    fn greater_equal(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        compare_values(&left, &right)
            .map(|o| Value::Boolean(o == Ordering::Greater || o == Ordering::Equal))
            .ok_or_else(|| comparison_error(">=", &left, &right))
    }

    fn logical_and(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(l && r)),
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for logical AND: {:?} AND {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }

    fn logical_or(&self, left: Value, right: Value) -> Result<Value, CPSError> {
        match (left.clone(), right.clone()) {
            (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(l || r)),
            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported types for logical OR: {:?} OR {:?}", left, right),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }





    fn evaluate_literal(&self, lit: &Value) -> Result<Value, CPSError> {
        match lit {
            Value::Integer(_) |
            Value::Real(_) |
            Value::String(_) |
            Value::Boolean(_) |
            Value::Array { .. } |
            Value::Date(_) |
            Value::Enum { .. } |
            Value::Char(_) => {},
            Value::Identifier(iden) => {
                match self.current_env 
                    .borrow()
                    .get(iden) {
                        Some(value) => return Ok(value),
                        None => {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!("Undefined identifier: {}", iden),
                                hint: None,
                                line: 0,
                                column: 0,
                                source: None,
                            });
                        }
                    }
            }
            _ => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Unsupported literal in interpreter: {:?}", lit),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
        }
        Ok(lit.clone())
    }



}

fn check_if_type_can_be_converted(value: &Value, target_type: &Type) -> bool {
    match (value, target_type) {
        (Value::Integer(_), Type::Integer) => true,
        (Value::Real(_), Type::Real) => true,
        (Value::String(_), Type::String) => true,
        (Value::Boolean(_), Type::Boolean) => true,
        (Value::Char(_), Type::Char) => true,
        (Value::Integer(_), Type::Real) => true,
        (Value::Date(_), Type::Date) => true,
        (Value::Enum { type_name: l1, .. }, Type::Enum(r1)) => l1 == r1,
        (Value::Array { array, lower_bound: _, bounds_2d: _ }, Type::Array(arr_type)) => {
            match arr_type {
                ArrayType { lower_bound: _, upper_bound: _, base_type , bounds_2d: _} => {
                    if array.is_empty() {
                        return true; // empty array can be converted
                    }
                    for element in array {
                        if !check_if_type_can_be_converted(element, *&base_type) {
                            return false;
                        }
                    }
                    true
                }
            }
        }
        (Value::Real(_), Type::Integer) => {
            if let Value::Real(f) = value {
                f.fract() == 0.0
            } else {
                false
            }
        },
        _ => false,
    }
}

fn check_if_types_match_exactly(actual: &Type, expected: &Type) -> bool {
    match (actual, expected) {
        (Type::Integer, Type::Integer) => true,
        (Type::Real,    Type::Real)    => true,
        (Type::String,  Type::String)  => true,
        (Type::Boolean, Type::Boolean) => true,
        (Type::Char,    Type::Char)    => true,
        (Type::Date,    Type::Date)    => true,
        (Type::Enum(l1), Type::Enum(r1)) => l1 == r1,
        (Type::Array(a), Type::Array(b)) => {
            a.bounds_2d.is_some() == b.bounds_2d.is_some()
                && check_if_types_match_exactly(&a.base_type, &b.base_type)
        }
        _ => false,
    }
}

// fn check_if_value_can_be_converted(value1: &Value, value2: &Value) -> bool {
//     match (value1, value2) {
//         (Value::Integer(_), Value::Integer(_)) => true,
//         (Value::Real(_), Value::Real(_)) => true,
//         (Value::String(_), Value::String(_)) => true,
//         (Value::Boolean(_), Value::Boolean(_)) => true,
//         (Value::Char(_), Value::Char(_)) => true,
//         
//         (Value::Integer(_), Value::Real(_)) => true,
//         (Value::Real(_), Value::Integer(_)) => true,
//         
//         _ => false,
//     }
// }

fn convert_values_to_compatible_types(value1: &Value, value2: &Value) -> (Value, Value) {
    match (value1, value2) {
        (Value::Integer(i), Value::Real(r)) => 
        {
            (Value::Real(*i as f64), Value::Real(*r))
        },
        (Value::Real(r), Value::Integer(i)) => {
            (Value::Real(*r), Value::Real(*i as f64))
        }
        _ => {
            (value1.clone(), value2.clone())
        },
    }
}


fn convert_values_to_base_type(value: &Value, target_type: &Type) -> Result<Value, CPSError> {
    match (value, target_type) {
        (Value::Integer(i), Type::Real) => Ok(Value::Real(*i as f64)),
        (Value::Real(r), Type::Integer) => {
            if r.fract() != 0.0 {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Cannot convert real number with fractional part to integer: {}", r),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }
            Ok(Value::Integer(*r as i64))
        },
        _ => Ok(value.clone()),
    }
}

fn comparison_error(op: &str, left: &Value, right: &Value) -> CPSError {
    let type_name = |v: &Value| match v.type_of() {
        Ok(t) => format!("{:?}", t).to_uppercase(),
        Err(_) => "UNKNOWN".to_string(),
    };


    let (lt, rt) = (type_name(left), type_name(right));


    let message = if lt == rt {
        format!("Cannot compare two {} values using '{}'", lt, op)
    } else {
        format!("Cannot compare {} with {} using '{}'", lt, rt, op)
    };

    let hint = if matches!((left, right), (Value::Real(l), Value::Real(r)) if l.is_nan() || r.is_nan()) {
        Some("A value that is not a number cannot be ordered".to_string())
    } else if lt == rt {
        Some(format!("{} values can only be compared with = and <>", lt))
    } else {
        Some("Both sides of a comparison must be the same type".to_string())
    };

    CPSError { error_type: ErrorType::Runtime, message, hint, line: 0, column: 0, source: None }
}

fn compare_values(left: &Value, right: &Value) -> Option<Ordering> {
    match (left, right) {
        (Value::Integer(l), Value::Integer(r)) => Some(l.cmp(r)),
        (Value::Real(l), Value::Real(r))       => l.partial_cmp(r),
        (Value::String(l), Value::String(r))   => Some(l.cmp(r)),
        (Value::Char(l), Value::Char(r))       => Some(l.cmp(r)),
        (Value::Date(l), Value::Date(r))       => {
            Some((l.year, l.month, l.day).cmp(&(r.year, r.month, r.day)))
        }
        _ => None,
    }
}

fn values_are_equal(left: &Value, right: &Value) -> Option<bool> {
    match (left, right) {
        (Value::Integer(l), Value::Integer(r)) => Some(l == r),
        (Value::Real(l), Value::Real(r))       => Some(l == r),
        (Value::String(l), Value::String(r))   => Some(l == r),
        (Value::Boolean(l), Value::Boolean(r)) => Some(l == r),
        (Value::Char(l), Value::Char(r))       => Some(l == r),
        (Value::Date(l), Value::Date(r))       => Some(l == r),
        (Value::Enum { type_name: l1, variant: l2 }, Value::Enum { type_name: r1, variant: r2 }) if l1 == r1 => Some(l2 == r2),
        (Value::Array { array: l1, lower_bound: l2, bounds_2d: l3 }, Value::Array { array: r1, lower_bound: r2, bounds_2d: r3 }) => Some(l1 == r1 && l2 == r2 && l3 == r3),
        _ => None,
    }
}
