use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::rc::Rc;
use std::cell::RefCell;

use crate::errortype::{CPSError, ErrorType};
use crate::Parser::ast::{BlockStmt, Expr, FileMode, PassingValue};



#[derive(Clone, PartialEq)]
pub enum Type {
    Integer,
    Real,
    String,
    Boolean,
    Char,
    Function,
    Array(ArrayType),
    Date,
    Named(String),
    // Record(String), 
    Enum(String),
}

impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Integer => write!(f, "Integer"),
            Type::Real => write!(f, "Real"),
            Type::String => write!(f, "String"),
            Type::Char => write!(f, "Char"),
            Type::Function => write!(f, "Function"),
            Type::Date => write!(f, "Date"),
            Type::Enum(name) => write!(f, "{}", name),
            Type::Named(name) => write!(f, "{}", name),
            Type::Array(array_type) => {
                let lower = fmt_bound(&array_type.lower_bound);
                let upper = fmt_bound(&array_type.upper_bound);
                match &array_type.bounds_2d {
                    Some((col_lower, col_upper)) => write!(
                        f,
                        "Array[{}:{}, {}:{}] OF {:?}",
                        lower,
                        upper,
                        fmt_bound(col_lower),
                        fmt_bound(col_upper),
                        array_type.base_type
                    ),
                    None => write!(f, "Array[{}:{}] OF {:?}", lower, upper, array_type.base_type),
                }
            }
            Type::Boolean => write!(f, "Boolean")
        }
    }
}

fn fmt_bound(e: &Expr) -> String {
    match e {
        Expr::Literal(Value::Integer(n)) => n.to_string(),
        Expr::Literal(Value::Real(r)) if r.fract() == 0.0 => (*r as i64).to_string(),
        Expr::Literal(Value::Identifier(name)) => name.clone(),
        _ => "?".to_string(),
    }
}

#[derive(Clone, PartialEq)]
pub struct ArrayType {
    pub lower_bound: Box<Expr>,
    pub upper_bound: Box<Expr>,
    pub bounds_2d: Option<(Box<Expr>, Box<Expr>)>,
    pub base_type: Box<Type>,
}

// Runtime values (actual data)
#[derive(Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Real(f64),
    String(String),
    Boolean(bool),
    Char(char),
    Array { array: Vec<Value>, lower_bound: usize, bounds_2d: Option<(usize, usize)> }, 
    Identifier(String),
    Function(Function),
    Date(Date),
    // Record(HashMap<String, Value>),
    Enum { type_name: String, variant: Option<String> }, // may not contain a variant
    // Null,  
}

impl Value {
    pub fn type_of(&self) -> Result<Type, CPSError> {
        Ok(match self {
            Value::Integer(_)  => Type::Integer,
            Value::Real(_)     => Type::Real,
            Value::String(_)   => Type::String,
            Value::Boolean(_)  => Type::Boolean,
            Value::Char(_)     => Type::Char,
            Value::Date(_)     => Type::Date,
            Value::Function(_) => Type::Function,
            Value::Enum { type_name, .. } => Type::Enum(type_name.to_owned()),

            Value::Identifier(name) => {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!(
                        "Internal error: identifier '{}' reached type checking without being evaluated",
                        name
                    ),
                    hint: Some(
                        "Identifiers should be resolved to values before their type is queried."
                            .to_string(),
                    ),
                    line: 0, column: 0, source: None,
                });
            }

            Value::Array { array, lower_bound, bounds_2d } => {
                let lit = |n: usize| Box::new(Expr::Literal(Value::Integer(n as i64)));

                let base_type = array
                    .first()
                    .ok_or_else(|| CPSError {
                        error_type: ErrorType::Runtime,
                        message: "Cannot determine the base type of an empty array".to_string(),
                        hint: Some(
                            "Assign a value to at least one element before using the array \
                             somewhere its type is needed."
                                .to_string(),
                        ),
                        line: 0, column: 0, source: None,
                    })?
                    .type_of()?;

                let (upper_bound, cols) = match bounds_2d {
                    Some((col_lb, col_ub)) => {
                        if col_ub < col_lb {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!(
                                    "Array column bounds are inverted: [{}:{}]",
                                    col_lb, col_ub
                                ),
                                hint: Some(
                                    "The lower bound must not exceed the upper bound.".to_string(),
                                ),
                                line: 0, column: 0, source: None,
                            });
                        }

                        let col_count = col_ub - col_lb + 1; // >= 1, so the divide is safe
                        let row_count = array.len() / col_count;

                        if row_count == 0 {
                            return Err(CPSError {
                                error_type: ErrorType::Runtime,
                                message: format!(
                                    "2D array holds {} element(s), fewer than one full row of {}",
                                    array.len(),
                                    col_count
                                ),
                                hint: Some(
                                    "The element count should be a multiple of the column count."
                                        .to_string(),
                                ),
                                line: 0, column: 0, source: None,
                            });
                        }

                        (lower_bound + row_count - 1, Some((lit(*col_lb), lit(*col_ub))))
                    }
                    None => {
                        // the empty case already returned above, so len() >= 1 and this subtraction cannot underflow.
                        (lower_bound + array.len() - 1, None)
                    }
                };

                Type::Array(ArrayType {
                    lower_bound: lit(*lower_bound),
                    upper_bound: lit(upper_bound),
                    bounds_2d: cols,
                    base_type: Box::new(base_type),
                })
            }
        })
    }


    fn days_in_month(month: u16, year: u16) -> u16 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
            2 => 28,
            _ => 0,
        }
    }

    pub fn validate_date(day: u16, month: u16, year: u16) -> Result<(), String> { // TODO: Refactor this into impl Date?
        const MONTH_NAMES: [&str; 12] = [
            "January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December",
        ];

        if month < 1 || month > 12 {
            return Err(format!("{:02} is not a valid month; months are 01 to 12", month));
        }

        let max_day = Value::days_in_month(month, year);
        if day < 1 || day > max_day {
            return Err(format!(
                    "{:02} is not a valid day; {} {} has {} days",
                    day, MONTH_NAMES[(month - 1) as usize], year, max_day
            ));
        }

        Ok(())
    }

}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "INTEGER({})", n),
            Value::Real(n) => write!(f, "REAL({:?})", n),
            Value::String(string_value) => write!(f, "STRING(\"{}\")", string_value),
            Value::Boolean(b) => {
                if *b {
                    write!(f, "BOOLEAN(TRUE)")
                } else {
                    write!(f, "BOOLEAN(FALSE)")
                }
            }
            Value::Char(c) => write!(f, "CHAR('{}')", c),
            Value::Date(date) => write!(f, "DATE({:02}/{:02}/{:04})", date.day, date.month, date.year),
            Value::Enum { type_name, variant } => {
                match variant {
                    Some(v) => write!(f, "{}({})", type_name, v),
                    None => write!(f, "{}(None)", type_name)
                }
            },
            Value::Array { array, lower_bound, bounds_2d } => {
                let base = match array.first() {
                    Some(Value::Integer(_)) => "Integer",
                    Some(Value::Real(_)) => "Real",
                    Some(Value::String(_)) => "String",
                    Some(Value::Boolean(_)) => "Boolean",
                    Some(Value::Char(_)) => "Char",
                    Some(Value::Date(_)) => "Date",
                    _ => "?",
                };
                match bounds_2d {
                    Some((col_lower, col_upper)) => {
                        let cols = col_upper - col_lower + 1;
                        let rows = if cols == 0 { 0 } else { array.len() / cols };
                        write!(
                            f,
                            "Array[{}:{}, {}:{}] OF {}",
                            lower_bound,
                            lower_bound + rows.saturating_sub(1),
                            col_lower,
                            col_upper,
                            base
                        )?;
                    }
                    None => {
                        write!(
                            f,
                            "Array[{}:{}] OF {}",
                            lower_bound,
                            lower_bound + array.len().saturating_sub(1),
                            base
                        )?;
                    }
                }
                if f.alternate() {
                    write!(f, " {:?}", array)?;
                }
                Ok(())
            }
            Value::Identifier(name) => write!(f, "{}", name),
            Value::Function(function) => {
                let params: Vec<String> = function
                    .parameters
                    .iter()
                    .map(|(param_name, param_type, passing_value)| {
                        let keyword = match passing_value {
                            PassingValue::ByVal => "BYVAL",
                            PassingValue::ByRef => "BYREF",
                        };
                        format!("{} {} : {:?}", keyword, param_name, param_type)
                    })
                .collect();

                match &function.return_type {
                    Some(return_type) => {
                        write!(f, "FUNCTION({}) RETURNS {:?}", params.join(", "), return_type)
                    }
                    None => write!(f, "PROCEDURE({})", params.join(", ")),
                }
            }
        }
    }
}


impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(n) => write!(f, "{}", n),
            Value::Real(n) => write!(f, "{}", n),
            Value::String(string_value) => write!(f, "{}", string_value),
            Value::Char(c) => write!(f, "{}", c),
            Value::Boolean(true) => write!(f, "TRUE"),
            Value::Boolean(false) => write!(f, "FALSE"),
            Value::Date(date) => write!(f, "{:02}/{:02}/{:04}", date.day, date.month, date.year),
            Value::Enum { variant, .. } => {
                    match variant {
                        Some(v) => write!(f, "{}", v),
                        None => write!(f, "None"),
                    }
            },
            Value::Array { .. } | Value::Function(_) | Value::Identifier(_) => {
                write!(f, "{:?}", self)
            }
        }
    }
}

impl Value {
    pub fn to_output_string(&self, context: &str) -> Result<String, CPSError> {
        match self {
            Value::Array { .. } => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot {} a whole array", context),
                hint: Some(
                    "Use a loop to handle each element, e.g. FOR i <- 1 TO 3 / OUTPUT A[i] / NEXT i"
                        .to_string(),
                ),
                line: 0, column: 0, source: None,
            }),
            Value::Function(_) => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot {} a procedure or function", context),
                hint: None,
                line: 0, column: 0, source: None,
            }),
            Value::Identifier(name) => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Undefined identifier: {}", name),
                hint: None,
                line: 0, column: 0, source: None,
            }),
            Value::Enum { type_name, variant: None } => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot {} a {} value before one has been assigned", context, type_name),
                hint: Some(format!("Assign one of {}'s values first.", type_name)),
                line: 0, column: 0, source: None,
            }),
            other => Ok(other.to_string()),
        }
    }

    /// Turns a raw INPUT line into a value of the declared type.
    /// `target` names the destination for the error message, e.g. "variable 'Age'".
    pub fn from_input(text: &str, ty: &Type, target: &str, environment: Rc<RefCell<Environment>>) -> Result<Value, CPSError> {
        match ty {
            Type::String => Ok(Value::String(text.to_string())),

            Type::Integer => text
                .parse::<i64>()
                .map(Value::Integer)
                .map_err(|_| input_error("an integer", target, text)),

            Type::Real => text
                .parse::<f64>()
                .map(Value::Real)
                .map_err(|_| input_error("a real number", target, text)),

            Type::Boolean => match text.to_lowercase().as_str() {
                "true" => Ok(Value::Boolean(true)),
                "false" => Ok(Value::Boolean(false)),
                _ => Err(input_error("'true' or 'false'", target, text)),
            },

            Type::Char => {
                if text.chars().count() != 1 {
                    return Err(input_error("a single character", target, text));
                }
                Ok(Value::Char(text.chars().next().unwrap()))
            }

            Type::Date => match Date::parse(text) {
                Ok(date) => Ok(Value::Date(date)),
                Err(reason) => Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("'{}' is not a valid date for {}: {}", text, target, reason),
                    hint: Some("Dates are written as dd/mm/yyyy.".to_owned()),
                    line: 0, column: 0, source: None,
                }),
            },

            Type::Enum(name) => {
                let is_valid_enum = environment.borrow().check_if_valid_enum_value(name, text);

                if is_valid_enum {
                    Ok(Value::Enum { type_name: name.into(), variant: Some(text.into()) })
                } else {
                    Err(input_error(format!("a valid variant of the enum {}", name).as_ref(), target, text))
                }
            }

            _ => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unsupported type for INPUT into {}: {:?}", target, ty),
                hint: None,
                line: 0, column: 0, source: None,
            }),
        }
    }
}

fn input_error(expected: &str, target: &str, got: &str) -> CPSError {
    CPSError {
        error_type: ErrorType::Runtime,
        message: format!("Expected {} for {}, got: {}", expected, target, got),
        hint: None,
        line: 0, column: 0, source: None,
    }
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
pub struct Function {
    pub parameters: Vec<(String, Type, PassingValue)>,
    pub return_type: Option<Type>,
    pub body: BlockStmt
}

#[derive(Clone, Debug, PartialEq)] 
pub struct Date {
    pub day: u16,
    pub month: u16,
    pub year: u16,
}

impl Date {
    pub fn parse(text: &str) -> Result<Date, String> {
        let parts: Vec<&str> = text.split('/').collect();
        if parts.len() != 3
            || parts[0].len() != 2 || parts[1].len() != 2 || parts[2].len() != 4
            || !parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
        {
            return Err("dates are written as dd/mm/yyyy".to_string());
        }
        let (day, month, year) = (parts[0].parse().unwrap(), parts[1].parse().unwrap(), parts[2].parse().unwrap());
        Value::validate_date(day, month, year)?;
        Ok(Date { day, month, year })
    }
}


#[derive(Debug, Clone)]
pub struct Environment {
    pub bindings: HashMap<String, usize>,
    parent: Option<Rc<RefCell<Environment>>>,
    pub open_files: HashMap<String, OpenFile>, // track open files by variable name
    constants: HashSet<String>, // track constant variable names
    pub heap: Rc<RefCell<HashMap<usize, Value>>>, // for use in pointers and reference types in the future 
    next_address: Rc<RefCell<usize>>, // simple counter to assign unique addresses for reference types
    types: HashMap<String, Type>, // defined custom types within the environment
    enum_variants: HashMap<String, Vec<String>>, // the values of each enumerated type, in the order they were declared
}

impl Environment {
    pub fn new_global() -> Rc<RefCell<Self>> {
        let global = Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            parent: None,
            open_files: HashMap::new(),
            constants: HashSet::new(),
            heap: Rc::new(RefCell::new(HashMap::new())),
            next_address: Rc::new(RefCell::new(0)),
            types: HashMap::new(),
            enum_variants: HashMap::new(),
        }));
        
        // declare builtin functions here


        global
    }

    // fn register_builtins(env: Rc<RefCell<Environment>>) {
    //     env.borrow_mut()
    //         .define("RIGHT".to_string(), Value::Function(Function {
    //             parameters: vec![
    //                 ("string".to_string(), Type::String),
    //                 ("length".to_string(), Type::Integer),
    //             ],
    //             return_type: Some(Type::String),
    //             body: BlockStmt { statements: vec![] },
    //         }));
    // }

    pub fn new_child(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            parent: Some(parent.clone()),
            open_files: HashMap::new(),
            constants: HashSet::new(),
            heap: Rc::clone(&parent.borrow().heap), // share heap with parent for reference types
            next_address: Rc::clone(&parent.borrow().next_address),
            types: HashMap::new(),
            enum_variants: HashMap::new(),
        }))
    }

    fn get_variable_at_address(&self, address: usize) -> Option<Value> {
        self.heap.borrow().get(&address).cloned()
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(address) = self.bindings.get(name) {
            return Some(self.get_variable_at_address(*address)?);
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().get(name),
            None => {
                None
            },
        }
    }

    pub fn set(&mut self, name: &str, value: Value) -> Result<(), CPSError> {
        if self.is_constant(name) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot modify constant '{}'", name),
                hint: None,
                line: 0, column: 0, source: None,
            });
        }

        if self.bindings.contains_key(name) {
            // variable exists in current scope, update it
            let address = self.bindings[name];
            self.heap.borrow_mut().insert(address, value);

            return Ok(());
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow_mut().set(name, value),
            None => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Undefined variable '{}'", name),
                hint: Some("Check if the variable is declared before use.".to_string()),
                line: 0, column: 0, source: None,
            }),
        }
    }



    pub fn set_array_element(&mut self, name: &str, index: usize, col: Option<usize>, value: Value) -> Result<(), CPSError> {
        if let Some(address) = self.bindings.get(name) {
            if let Some(current_value) = self.heap.borrow_mut().get_mut(address) {
                match current_value {
                    Value::Array { array, lower_bound, bounds_2d } => {
                        let lower_bound = *lower_bound;
                        let bounds_2d = *bounds_2d;

                        if index < lower_bound {
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

                            let col_count = col_ub - col_lb + 1;
                            let row_offset = index - lower_bound;
                            let col_offset = col_idx.checked_sub(col_lb).ok_or_else(|| CPSError {
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
                            index - lower_bound
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
        if let Some(address) = self.bindings.get(name) {
            let current_value = self.get_variable_at_address(*address).ok_or_else(|| CPSError {
                error_type: crate::errortype::ErrorType::Runtime,
                message: format!("Variable '{}' not found in memory", name),
                hint: None,
                line: 0, column: 0, source: None,
            })?;
            match current_value {
                Value::Array { array, lower_bound, bounds_2d } => {
                    if index < lower_bound {
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

                        let col_count = col_ub - col_lb + 1;
                        let row_offset = index - lower_bound;
                        let col_offset = col_idx.checked_sub(col_lb).ok_or_else(|| CPSError {
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
                        index - lower_bound
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


    pub fn closefile(&mut self, name: &str) -> Result<(), CPSError> {
        if let Some(open_file) = self.open_files.get_mut(name) {
            // if &open_file.mode != mode {
            //     return Err(CPSError {
            //         error_type: ErrorType::Runtime,
            //         message: format!("Cannot close file '{}': opened in {:?} mode but attempting to close in {:?} mode", name, open_file.mode, mode),
            //         hint: None,
            //         line: 0, column: 0, source: None,
            //     });
            // }
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
            Some(parent_rc) => parent_rc.borrow_mut().closefile(name),
            None => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot close file '{}': file is not open", name),
                hint: None,
                line: 0, column: 0, source: None,
            }),
        }
    }

    pub fn is_file_open_conflicting(&self, name: &str, requested_mode: &FileMode) -> bool {
        if let Some(_) = self.open_files.get(name) { // can't open the same file twice
            return true;
        }
        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().is_file_open_conflicting(name, requested_mode),
            None => false,
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
                use std::io::Read;

                let mut f = fs::OpenOptions::new()
                    .read(true)
                    .open(name)
                    .map_err(|e| CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!("Failed to open file '{}': {}", name, e),
                        hint: None,
                        line: 0, column: 0, source: None,
                    })?;

                let mut contents = String::new();
                f.read_to_string(&mut contents).map_err(|e| CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("Failed to read file '{}': {}", name, e),
                    hint: None,
                    line: 0, column: 0, source: None,
                })?;
                let lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();

                self.open_files.insert(name.to_string(), OpenFile {
                    mode: mode.clone(),
                    handle: None, 
                    line_idx: 0,
                    lines: Some(lines),
                });
                return Ok(());
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
                let output = value.to_output_string("write")?;
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

            let lines = match open_file.lines.as_ref() {
                Some(l) => l,
                None => return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("File '{}' has not been buffered for reading", filename),
                    hint: Some("File content should have been loaded on open".to_string()),
                    line: 0, column: 0, source: None,
                }),
            };
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
                None => return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("File '{}' has not been buffered", filename),
                    hint: Some("File content was not loaded on open".to_string()),
                    line: 0, column: 0, source: None,
                }),
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
        if let Some(address) = self.bindings.get(name) {
            let value = self.get_variable_at_address(*address).ok_or_else(|| CPSError {
                error_type: crate::errortype::ErrorType::Runtime,
                message: format!("Variable '{}' not found in memory", name),
                hint: None,
                line: 0, column: 0, source: None,
            })?;
            return value.type_of().map_err(|mut e| {
                e.message = format!("{} (variable '{}')", e.message, name);
                e
            });
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

    pub fn declare_constant(&mut self, name: &str, value: &Value) -> Result<(), CPSError> {
        if self.constants.contains(name) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Constant '{}' is already declared", name),
                hint: None,
                line: 0, column: 0, source: None,
            });
        }
        self.define(name.to_string(), value.clone())?;
        self.constants.insert(name.to_string());
        Ok(())
    }

    pub fn is_constant(&self, name: &str) -> bool {
        if self.constants.contains(name) {
            return true;
        }
        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().is_constant(name),
            None => false,
        }
    }

    pub fn define_enum(&mut self, name: &str, variants: &[String]) -> Result<(), CPSError> {
        if self.types.contains_key(name) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("The type '{}' is already defined", name),
                hint: None,
                line: 0, column: 0, source: None,
            })
        }

        // checked up front so a rejected declaration leaves nothing behind
        for (i, variant) in variants.iter().enumerate() {
            if variants[..i].contains(variant) {
                return Err(CPSError {
                    error_type: ErrorType::Runtime,
                    message: format!("'{}' is listed twice in the type '{}'", variant, name),
                    hint: Some("Every value of an enumerated type needs a name of its own.".to_owned()),
                    line: 0, column: 0, source: None,
                })
            }
        }

        for variant in variants.iter() {
            self.declare_constant(variant, &Value::Enum { type_name: name.to_owned(), variant: Some(variant.to_owned()) })?;
        }

        self.types.insert(name.to_owned(), Type::Enum(name.to_owned()));
        self.enum_variants.insert(name.to_owned(), variants.to_vec());

        Ok(())
    }


    pub fn check_if_valid_enum_value(&self, name: &str, variant: &str) -> bool {
        if let Some(variants) = self.enum_variants.get(name) {
            return variants.iter().any(|v| v == variant);
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().check_if_valid_enum_value(name, variant),
            None => false,
        }
    }

    pub fn find_type_of_named_type(&self, name: &String) -> Result<Type, CPSError> {
        if let Some(type_) = self.types.get(name) {
            return Ok(type_.to_owned());
        }

        match &self.parent {
            Some(parent_rc) => parent_rc.borrow().find_type_of_named_type(name),
            None => Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("The type '{}' has not been defined", name),
                hint: Some("Make sure you define types before you use them".to_owned()),
                line: 0, column: 0, source: None,
            }),
        }
    }



    pub fn define(&mut self, name: String, value: Value) -> Result<(), CPSError> {
        if self.constants.contains(&name) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot redefine constant '{}'", name),
                hint: None,
                line: 0, column: 0, source: None,
            });
        }

        let current_addr = *self.next_address.borrow();
        self.heap.borrow_mut().insert(current_addr, value.clone());
        self.bindings.insert(name, current_addr);

        // ignore this, pointer arithmatic in the pseudocode works differently (+1 to a mem address will bring u to the next value past the array)
        // match &value { 
        //     Value::Array { array, lower_bound: _, bounds_2d: _ } => {
        //         let arr_size = array.len();
        //         self.next_address += arr_size; // reserve contiguous addresses for the array
        //     }
        //     _ => self.next_address += 1, // reserve one address for non-array values
        // }

        *self.next_address.borrow_mut() += 1;




        Ok(())
    }

    pub fn set_variable_at_address(&mut self, name: String, address: usize) -> Result<(), CPSError> {
        if self.constants.contains(&name) {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Cannot redefine constant '{}'", name),
                hint: None,
                line: 0, column: 0, source: None,
            });
        }

        self.bindings.insert(name, address); // create variable at that specific address

        Ok(())
    }

    pub fn find_address_of_variable(&mut self, name: String) -> Option<usize> {
        let address = self.bindings.get(&name);

        match address {
            Some(addr) => {
                return Some(*addr);
            }
            None => {
                match &self.parent {
                    Some(parent_rc) => parent_rc.borrow_mut().find_address_of_variable(name),
                    None => {
                        None
                    },
                }
            }
        }
    }

}
