use crate::{errortype::{CPSError, ErrorType}, Inter::cps::Value};
use rand::random_range;

pub fn call_builtin(name: String, args: &[Value]) -> Result<Option<Value>, CPSError> {
    match name.as_str() {
        "RIGHT" => builtin_right(args),
        "LENGTH" => builtin_length(args),
        "MID" => builtin_mid(args),
        "LCASE" => builtin_lcase(args),
        "UCASE" => builtin_ucase(args),
        "INT" => builtin_int(args),
        "RAND" => builtin_rand(args),
        _ => {
            Err(CPSError {
                error_type: ErrorType::Runtime,
                message: format!("Unknown builtin function: {}", name),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            })
        } 
    }
}

// Helper functions for type extraction and conversion
fn expect_string(value: &Value, func: &str, pos: usize) -> Result<String, CPSError> {
    match value {
        Value::String(s) => Ok(s.clone()),
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} argument {} must be a string", func, pos),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        })
    }
}

fn expect_int(value: &Value, func: &str, pos: usize) -> Result<i64, CPSError> {
    match value {
        Value::Integer(i) => Ok(*i),
        Value::Real(r) if r.fract() == 0.0 => Ok(*r as i64),
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} argument {} must be an integer", func, pos),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        })
    }
}

// fn expect_real(value: &Value, func: &str, pos: usize) -> Result<f64, CPSError> {
//     match value {
//         Value::Real(r) => Ok(*r),
//         Value::Integer(i) => Ok(*i as f64),
//         _ => Err(CPSError {
//             error_type: ErrorType::Runtime,
//             message: format!("{} argument {} must be a real number", func, pos),
//             hint: None,
//             line: 0,
//             column: 0,
//             source: None,
//         })
//     }
// }

fn arg_count_error(func: &str, expected: usize, got: usize) -> CPSError {
    CPSError {
        error_type: ErrorType::Runtime,
        message: format!("{} expects exactly {} argument(s), got {}", func, expected, got),
        hint: None,
        line: 0,
        column: 0,
        source: None,
    }
}

// Builtin function implementations
fn builtin_right(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 2 {
        return Err(arg_count_error("RIGHT", 2, args.len()));
    }

    let string = expect_string(&args[0], "RIGHT", 1)?;
    let length = expect_int(&args[1], "RIGHT", 2)?;
    
    if length < 0 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: "RIGHT length must be non-negative".to_string(),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        });
    }

    let chars: Vec<char> = string.chars().collect();
    let length_usize = length as usize;
    let start = if length_usize > chars.len() { 
        0 
    } else { 
        chars.len() - length_usize 
    };
    let result = chars[start..].iter().collect::<String>();
    
    Ok(Some(Value::String(result)))
}

fn builtin_length(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("LENGTH", 1, args.len()));
    }

    let string = expect_string(&args[0], "LENGTH", 1)?;
    let length = string.chars().count() as i64;
    Ok(Some(Value::Integer(length)))
}

fn builtin_mid(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 3 {
        return Err(arg_count_error("MID", 3, args.len()));
    }

    let string = expect_string(&args[0], "MID", 1)?;
    let start = expect_int(&args[1], "MID", 2)?;
    let length = expect_int(&args[2], "MID", 3)?;
    
    if start < 1 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: "MID start position must be >= 1".to_string(),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        });
    }
    
    if length < 0 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: "MID length must be non-negative".to_string(),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        });
    }

    let chars: Vec<char> = string.chars().collect();
    let start_idx = (start - 1) as usize;
    
    if start_idx >= chars.len() {
        return Ok(Some(Value::String(String::new())));
    }
    
    let end = usize::min(start_idx + length as usize, chars.len());
    let result = chars[start_idx..end].iter().collect::<String>();
    Ok(Some(Value::String(result)))
}

fn builtin_lcase(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("LCASE", 1, args.len()));
    }

    let string = expect_string(&args[0], "LCASE", 1)?;
    let result = string.to_lowercase();
    Ok(Some(Value::String(result)))
}

fn builtin_ucase(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("UCASE", 1, args.len()));
    }

    let string = expect_string(&args[0], "UCASE", 1)?;
    let result = string.to_uppercase();
    Ok(Some(Value::String(result)))
}

// fn builtin_int(args: &[Value]) -> Result<Option<Value>, CPSError> { // <- this is the original in the cambridge pseudocode but it only accepts real numbers
//                                                                     // <- makes it really useless
//     if args.len() != 1 {
//         return Err(arg_count_error("INT", 1, args.len()));
//     }
//     let real = expect_real(&args[0], "INT", 1)?;
//     let result = real.floor() as i64;
//     Ok(Some(Value::Integer(result)))
// }


fn builtin_int(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("INT", 1, args.len()));
    }
    
    let real = match &args[0] {
        Value::Real(r) => *r,
        Value::Integer(i) => *i as f64,
        Value::String(s) => {
            s.trim().parse::<f64>().map_err(|_| CPSError {
                error_type: ErrorType::Runtime,
                message: format!("INT cannot convert string '{}' to a number", s),
                hint: Some("String must contain a valid number".to_string()),
                line: 0,
                column: 0,
                source: None,
            })?
        },
        _ => {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: "INT argument must be a real number, integer, or numeric string".to_string(),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            });
        }
    };
    
    let result = real.floor() as i64;
    Ok(Some(Value::Integer(result)))
}


fn builtin_rand(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("RAND", 1, args.len()));
    }

    let upper = expect_int(&args[0], "RAND", 1)?;
    
    if upper <= 0 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: "RAND argument must be a positive integer".to_string(),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        });
    }

    let result: f64 = random_range(0.0..upper as f64);
    Ok(Some(Value::Real(result)))
}
