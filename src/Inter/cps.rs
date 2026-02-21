use std::collections::HashMap;
use std::fs::{self, File};
use std::rc::Rc;
use std::cell::RefCell;

use crate::errortype::{CPSError, ErrorType};
use crate::Parser::ast::{BlockStmt, Expr, FileMode};


#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Integer,
    Real,
    String,
    Boolean,
    Char,
    Function,
    Array(ArrayType),
    // Record(String), 
    // Enum(String),
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


#[derive(Debug)]
pub struct CloneableFile(File);

impl Clone for CloneableFile {
    fn clone(&self) -> Self {
        CloneableFile(self.0.try_clone().expect("Failed to clone file handle"))
    }
}



#[derive(Clone, Debug)]
pub struct OpenFile {
    pub mode: FileMode,
    pub lines: Option<Vec<String>>, // buffer lines in read mode
    pub line_idx: usize,
    pub handle: Option<CloneableFile>, // file handle for write/append mode
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
    parent: Option<Rc<RefCell<Environment>>>,
    pub open_files: HashMap<String, OpenFile>, // track open files by variable name
}

impl Environment {
    pub fn new_global() -> Rc<RefCell<Self>> {
        let global = Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            parent: None,
            open_files: HashMap::new(),
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
            open_files: HashMap::new(),
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

    pub fn is_file_open_conflicting(&self, name: &str, requested_mode: &FileMode) -> bool {
        if let Some(open_file) = self.open_files.get(name) {
            return match (&open_file.mode, requested_mode) {
                (FileMode::Read, FileMode::Read) => true,   // can't open twice for read
                (FileMode::Write, FileMode::Append) => true, // conflicting
                (FileMode::Append, FileMode::Write) => true, // conflicting
                (FileMode::Write, FileMode::Write) => true,  // same mode, conflict
                (FileMode::Append, FileMode::Append) => true, // same mode, conflict
                _ => false,
            };
        }
        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().is_file_open_conflicting(name, requested_mode),
            None => false,
        }
    }

    pub fn closefile(&mut self, name: &str, mode: &FileMode) -> Result<(), CPSError> {
        if let Some(open_file) = self.open_files.get_mut(name) {
            if &open_file.mode != mode {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Cannot close file '{}': opened in {:?} mode but attempting to close in {:?} mode", name, open_file.mode, mode),
                    hint: None,
                    line: 0, column: 0, source: None,
                });
            }
            // flush before closing
            if let Some(handle) = &mut open_file.handle {
                use std::io::Write;
                handle.0.flush().map_err(|e| CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Failed to flush file '{}' before closing: {}", name, e),
                    hint: None,
                    line: 0, column: 0, source: None,
                })?;
            }
            self.open_files.remove(name);
            return Ok(());
        }
        match &mut self.parent {
            Some(parent_rc) => parent_rc.borrow_mut().closefile(name, mode),
            None => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot close file '{}': file is not open", name),
                hint: None,
                line: 0, column: 0, source: None,
            }),
        }
    }


    pub fn openfile(&mut self, name: &str, mode: &FileMode) -> Result<(), CPSError> {
        if self.is_file_open_conflicting(name, mode) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot open file '{}': file is already open", name),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            });
        }

        let file = match mode {
            FileMode::Write => {
                let f = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(name)
                    .map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to open file '{}': {}", name, e),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })?;
                CloneableFile(f)
            }
            FileMode::Append => {
                let f = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(name)
                    .map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to open file '{}': {}", name, e),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })?;
                CloneableFile(f)
            }
            FileMode::Read => {
                let f = fs::OpenOptions::new()
                    .read(true)
                    .open(name)
                    .map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to open file '{}': {}", name, e),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })?;
                CloneableFile(f)
            }
        };

        self.open_files.insert(name.to_string(), OpenFile { mode: mode.clone(), handle: Some(file), line_idx: 0, lines: None, });
        Ok(())
    }

    pub fn writefile(&mut self, filename: &str, value: &Value) -> Result<(), CPSError> {
        if let Some(open_file) = self.open_files.get_mut(filename) {
            if open_file.mode == FileMode::Read {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Cannot write to file '{}': file is opened in read mode", filename),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                });
            }

            if let Some(handle) = &mut open_file.handle {
                use std::io::Write;
                let output = match value {
                    Value::Integer(i) => i.to_string(),
                    Value::Real(f) => f.to_string(),
                    Value::String(s) => s.clone(),
                    Value::Boolean(b) => b.to_string(),
                    Value::Char(c) => c.to_string(),
                    _ => return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Unsupported value type for writing to file '{}'", filename),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    }),
                    // Value::Array { .. } => format!("{:?}", value), // simple debug output for arrays
                    // Value::Identifier(id) => id.clone(),
                    // Value::Function(_) => "<function>".to_string(),
                };
                handle.0.write_all(format!("{}\n", output).as_bytes()).map_err(|e| CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Failed to write to file '{}': {}", filename, e),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                })?;
                Ok(())
            } else {
                Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("File '{}' is not properly opened", filename),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                })
            }
        } else {
            match &mut self.parent {
                Some(parent_rc) => parent_rc.borrow_mut().writefile(filename, value),
                None => Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("File '{}' is not open", filename),
                    hint: None,
                    line: 0,
                    column: 0,
                    source: None,
                }),
            }
        }
    }

    pub fn readfile(&mut self, filename: &str) -> Result<String, CPSError> {
        if let Some(open_file) = self.open_files.get_mut(filename) {
            if open_file.mode != FileMode::Read {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Cannot read from file '{}': file is not opened in read mode", filename),
                    hint: None,
                    line: 0, column: 0, source: None,
                });
            }

            // buffer lines on first read
            if open_file.lines.is_none() {
                use std::io::Read;
                let mut contents = String::new();
                if let Some(handle) = &mut open_file.handle {
                    handle.0.read_to_string(&mut contents).map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to read file '{}': {}", filename, e),
                        hint: None,
                        line: 0, column: 0, source: None,
                    })?;
                }
                open_file.lines = Some(contents.lines().map(|l| l.to_string()).collect());
            }

            let lines = open_file.lines.as_ref().unwrap();
            if open_file.line_idx >= lines.len() {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Cannot read past end of file '{}'", filename),
                    hint: Some("Check EOF before reading".to_string()),
                    line: 0, column: 0, source: None,
                });
            }

            let line = lines[open_file.line_idx].clone();
            open_file.line_idx += 1;
            Ok(line)
        } else {
            match &mut self.parent {
                Some(parent_rc) => parent_rc.borrow_mut().readfile(filename),
                None => Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("File '{}' is not open", filename),
                    hint: None,
                    line: 0, column: 0, source: None,
                }),
            }
        }
    }

    pub fn is_eof(&self, filename: &str) -> Result<bool, CPSError> {
        if let Some(open_file) = self.open_files.get(filename) {
            if open_file.mode != FileMode::Read {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("EOF can only be used on files opened in READ mode, '{}' is in {:?} mode", filename, open_file.mode),
                    hint: None,
                    line: 0, column: 0, source: None,
                });
            }
            let is_eof = match &open_file.lines {
                Some(lines) => open_file.line_idx >= lines.len(),
                None => false, // not yet buffered, so not EOF
            };
            return Ok(is_eof);
        }
        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().is_eof(filename),
            None => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("File '{}' is not open", filename),
                hint: Some("Make sure to open the file before checking EOF".to_string()),
                line: 0, column: 0, source: None,
            }),
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
