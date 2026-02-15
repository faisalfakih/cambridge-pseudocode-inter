use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::errortype::CPSError;
use crate::Parser::ast::{BlockStmt, Expr, Stmt};


#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Integer,
    Real,
    String,
    Boolean,
    Char,
    Function,
    Array(ArrayType),
    Record(String), 
    Enum(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayType {
    pub lower_bound: Box<Expr>,
    pub upper_bound: Box<Expr>,
    pub bounds_2d: Option<(Box<Expr>, Box<Expr>)>,
    pub base_type: Box<Type>,
}

// Runtime values (actual data)
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Integer(i64),
    Real(f64),
    String(String),
    Boolean(bool),
    Char(char),
    Array { array: Vec<Value>, lower_bound: usize, bounds_2d: Option<(usize, usize)> }, 
    Identifier(String),
    Function(Function),
    // Record(HashMap<String, Value>),
    // Enum { type_name: String, variant: String },
    // Null,  
}

pub enum FunctionType {
    UserDefined(Function),
    Builtin(BuiltinFunction),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuiltinFunction {
    pub name: String,
    pub parameters: Vec<(String, Type)>,
    pub return_type: Option<Type>,
    pub implementation: fn(Vec<Value>) -> Result<Option<Value>, CPSError>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub parameters: Vec<(String, Type)>,
    pub return_type: Option<Type>,
    pub body: BlockStmt
}


#[derive(Debug, Clone)]
pub struct Environment {
    pub bindings: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Environment>>>
}

impl Environment {
    pub fn new_global() -> Rc<RefCell<Self>> {
        let global = Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            parent: None,
        }));
        
        // declare builtin functions here


        global
    }

    fn register_builtins(env: Rc<RefCell<Environment>>) {
        env.borrow_mut()
            .define("RIGHT".to_string(), Value::Function(Function {
                parameters: vec![
                    ("string".to_string(), Type::String),
                    ("length".to_string(), Type::Integer),
                ],
                return_type: Some(Type::String),
                body: BlockStmt { statements: vec![] },
            }));
    }

    pub fn new_child(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            parent: Some(parent),
        }))
    }


    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.bindings.get(name) {
            return Some(value.clone());
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().get(name),
            None => {
                None
            },
        }
    }

    pub fn set(&mut self, name: &str, value: Value) -> Result<(), CPSError> {
        if self.bindings.contains_key(name) {
            self.bindings.insert(name.to_string(), value);
            return Ok(());
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow_mut().set(name, value),
            None => Err(CPSError {
                error_type: crate::errortype::ErrorType::Runtime,
                message: format!("Undefined variable '{}'", name),
                hint: Some("Check if the variable is declared before use.".to_string()),
                line: 0,
                column: 0,
                source: None,
            }),
        }
    }


    pub fn set_array_element(&mut self, name: &str, index: usize, col: Option<usize>, value: Value) -> Result<(), CPSError> {
        if let Some(current_value) = self.bindings.get_mut(name) {
            match current_value {
                Value::Array { array, lower_bound, bounds_2d } => {
                    if index < *lower_bound {
                        return Err(CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Array index {} is below lower bound {} for '{}'", index, lower_bound, name),
                            hint: Some(format!("Valid indices start from {}", lower_bound)),
                            line: 0, column: 0, source: None,
                        });
                    }

                    let flat_index = if let Some((col_lb, col_ub)) = bounds_2d {
                        let col_idx = col.ok_or_else(|| CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Missing column index for 2D array '{}'", name),
                            hint: Some("2D arrays require both row and column indices".to_string()),
                            line: 0, column: 0, source: None,
                        })?;

                        let col_count = *col_ub - *col_lb + 1;
                        let row_offset = index - *lower_bound;
                        let col_offset = col_idx.checked_sub(*col_lb).ok_or_else(|| CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Column index {} is below lower bound {} for '{}'", col_idx, col_lb, name),
                            hint: Some(format!("Valid column indices start from {}", col_lb)),
                            line: 0, column: 0, source: None,
                        })?;

                        if col_offset >= col_count {
                            return Err(CPSError {
                                error_type: crate::errortype::ErrorType::Runtime,
                                message: format!("Column index {} is out of bounds for '{}'", col_idx, name),
                                hint: Some(format!("Valid column indices range from {} to {}", col_lb, col_ub)),
                                line: 0, column: 0, source: None,
                            });
                        }

                        row_offset * col_count + col_offset
                    } else {
                        index - *lower_bound
                    };

                    if flat_index >= array.len() {
                        return Err(CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Array index out of bounds for '{}'", name),
                            hint: None,
                            line: 0, column: 0, source: None,
                        });
                    }

                    array[flat_index] = value;
                    return Ok(());
                }
                _ => return Err(CPSError {
                    error_type: crate::errortype::ErrorType::Runtime,
                    message: format!("Variable '{}' is not an array", name),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            }
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow_mut().set_array_element(name, index, col, value),
            None => Err(CPSError {
                error_type: crate::errortype::ErrorType::Runtime,
                message: format!("Variable '{}' not found", name),
                hint: None,
                line: 0, column: 0, source: None,
            }),
        }
    }

    pub fn get_array_element(&self, name: &str, index: usize, col: Option<usize>) -> Result<Value, CPSError> {
        if let Some(current_value) = self.bindings.get(name) {
            match current_value {
                Value::Array { array, lower_bound, bounds_2d } => {
                    if index < *lower_bound {
                        return Err(CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Array index {} is below lower bound {} for '{}'", index, lower_bound, name),
                            hint: Some(format!("Valid indices start from {}", lower_bound)),
                            line: 0, column: 0, source: None,
                        });
                    }

                    let flat_index = if let Some((col_lb, col_ub)) = bounds_2d {
                        let col_idx = col.ok_or_else(|| CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Missing column index for 2D array '{}'", name),
                            hint: Some("2D arrays require both row and column indices".to_string()),
                            line: 0, column: 0, source: None,
                        })?;

                        let col_count = *col_ub - *col_lb + 1;
                        let row_offset = index - *lower_bound;
                        let col_offset = col_idx.checked_sub(*col_lb).ok_or_else(|| CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Column index {} is below lower bound {} for '{}'", col_idx, col_lb, name),
                            hint: Some(format!("Valid column indices start from {}", col_lb)),
                            line: 0, column: 0, source: None,
                        })?;

                        if col_offset >= col_count {
                            return Err(CPSError {
                                error_type: crate::errortype::ErrorType::Runtime,
                                message: format!("Column index {} is out of bounds for '{}'", col_idx, name),
                                hint: Some(format!("Valid column indices range from {} to {}", col_lb, col_ub)),
                                line: 0, column: 0, source: None,
                            });
                        }

                        row_offset * col_count + col_offset
                    } else {
                        index - *lower_bound
                    };

                    if flat_index >= array.len() {
                        return Err(CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Array index out of bounds for '{}'", name),
                            hint: None,
                            line: 0, column: 0, source: None,
                        });
                    }

                    Ok(array[flat_index].clone())
                }
                _ => Err(CPSError {
                    error_type: crate::errortype::ErrorType::Runtime,
                    message: format!("Variable '{}' is not an array", name),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            }
        } else {
            match &self.parent {
                Some(parent_rc) => parent_rc.borrow().get_array_element(name, index, col),
                None => Err(CPSError {
                    error_type: crate::errortype::ErrorType::Runtime,
                    message: format!("Variable '{}' not found", name),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            }
        }
    }

    pub fn get_type(&mut self, name: &str) -> Result<Type, CPSError> {
        if let Some(value) = self.bindings.get(name) {
            let var_type = match value {
                Value::Integer(_) => Type::Integer,
                Value::Real(_) => Type::Real,
                Value::String(_) => Type::String,
                Value::Boolean(_) => Type::Boolean,
                Value::Char(_) => Type::Char,
                Value::Array { array, lower_bound, bounds_2d } => {
                    let (upper_bound, bounds_2d_type) = if let Some((col_lb, col_ub)) = bounds_2d {
                        let col_count = col_ub - col_lb + 1;
                        let row_count = array.len() / col_count;
                        let row_ub = *lower_bound + row_count - 1;
                        (
                            row_ub as i64,
                            Some((
                                    Box::new(Expr::Literal(Value::Integer(*col_lb as i64))),
                                    Box::new(Expr::Literal(Value::Integer(*col_ub as i64))),
                            ))
                        )
                    } else {
                        ((array.len() + *lower_bound - 1) as i64, None)
                    };

                    let base_type = if let Some(first_elem) = array.first() {
                        match first_elem {
                            Value::Integer(_) => Type::Integer,
                            Value::Real(_) => Type::Real,
                            Value::String(_) => Type::String,
                            Value::Boolean(_) => Type::Boolean,
                            Value::Char(_) => Type::Char,
                            Value::Array { .. } => {
                                return Err(CPSError {
                                    error_type: crate::errortype::ErrorType::Runtime,
                                    message: format!("Nested arrays are not supported for '{}'", name),
                                    hint: None,
                                    line: 0, column: 0, source: None,
                                });
                            }
                            Value::Identifier(_) => Type::String,
                            Value::Function(_) => Type::Function,
                        }
                    } else {
                        return Err(CPSError {
                            error_type: crate::errortype::ErrorType::Runtime,
                            message: format!("Cannot determine base type of empty array '{}'", name),
                            hint: Some("Consider initializing the array with a default value.".to_string()),
                            line: 0, column: 0, source: None,
                        });
                    };

                    Type::Array(ArrayType {
                        lower_bound: Box::new(Expr::Literal(Value::Integer(*lower_bound as i64))),
                        upper_bound: Box::new(Expr::Literal(Value::Integer(upper_bound))),
                        bounds_2d: bounds_2d_type,
                        base_type: Box::new(base_type),
                    })
                },
                Value::Identifier(_) => Type::String,
                Value::Function(_) => Type::Function,
            };
            return Ok(var_type);
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow_mut().get_type(name),
            None => Err(CPSError {
                error_type: crate::errortype::ErrorType::Runtime,
                message: format!("Undefined variable '{}'", name),
                hint: Some("Check if the variable is declared before use.".to_string()),
                line: 0, column: 0, source: None,
            }),
        }
    }



    pub fn define(&mut self, name: String, value: Value) {
        self.bindings.insert(name, value);
    }

}
