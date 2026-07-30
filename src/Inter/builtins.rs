use crate::{errortype::{CPSError, ErrorType}, Inter::cps::Value};
use rand::random_range;

pub fn call_builtin(name: String, args: &[Value]) -> Result<Option<Value>, CPSError> {
    match name.as_str() {
        "RIGHT" => builtin_right(args),
        "LENGTH" => builtin_length(args),
        "MID" => builtin_mid(args, "MID"),
        "SUBSTRING" => builtin_mid(args, "SUBSTRING"), // Alias for MID
        "LCASE" => builtin_lcase(args),
        "UCASE" => builtin_ucase(args),
        "INT" => builtin_int(args),
        "RAND" => builtin_rand(args),
        "NUM_TO_STR" => builtin_num_to_str(args),
        "STR_TO_NUM" => builtin_str_to_num(args),
        "IS_NUM" => builtin_is_num(args),
        "ASC" => builtin_asc(args),
        "CHR" => builtin_chr(args),
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
    match value { // allow char values as strings for now (may change later if asked to)
        Value::String(s) => Ok(s.clone()),
        Value::Char(s) => Ok(s.to_string().clone()),
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

fn expect_char(value: &Value, func: &str, pos: usize) -> Result<char, CPSError> {
    match value {
        Value::Char(s) => Ok(s.clone()),
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} argument {} must be a char", func, pos),
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

fn expect_real(value: &Value, func: &str, pos: usize) -> Result<f64, CPSError> {
    match value {
        Value::Real(r) => Ok(*r),
        Value::Integer(i) => Ok(*i as f64),
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} argument {} must be a real number", func, pos),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        })
    }
}


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

fn builtin_mid(args: &[Value], name: &str) -> Result<Option<Value>, CPSError> {
    if args.len() != 3 {
        return Err(arg_count_error(name, 3, args.len()));
    }
 
    let string = expect_string(&args[0], name, 1)?;
    let start = expect_int(&args[1], name, 2)?;
    let length = expect_int(&args[2], name, 3)?;
    
    if start < 1 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} start position must be >= 1", name),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        });
    }
    
    if length < 0 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} length must be non-negative", name),
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

fn builtin_num_to_str(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("NUM_TO_STR", 1, args.len()))
    }

    let val = &args[0];

    let num = expect_real(val, "NUM_TO_STR", 1)?;

    Ok(Some(Value::String(num.to_string())))
}

fn builtin_str_to_num(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("STR_TO_NUM", 1, args.len()))
    }

    let string = expect_string(&args[0], "STR_TO_NUM", 1)?;

    let num = match string.parse::<f64>() {
        Ok(n) => n,
        Err(_) => return Err(CPSError { error_type: ErrorType::Runtime, 
            message: "STR_TO_NUM argument must be a numeric string".to_string(), hint: None, line: 0, column: 0, source: None }),
    };
    
    Ok(Some(Value::Real(num)))

}

fn builtin_is_num(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("IS_NUM", 1, args.len()))
    }

    let num = expect_string(&args[0], "IS_NUM", 1)?;

    Ok(Some(Value::Boolean(num.parse::<f64>().is_ok())))

}

fn builtin_asc(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("ASC", 1, args.len()))
    }

    let c = expect_char(&args[0], "ASC", 1)?;

    Ok(Some(Value::Integer(c as u8 as i64)))
}

fn builtin_chr(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("CHR", 1, args.len()))
    }

    let number = expect_int(&args[0], "CHR", 1)?;

    if number < 0 || number > 255 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("CHR argument {} is out of range (must be 0 to 255)", number),
            hint: Some("Valid character codes are 0 to 255".to_string()),
            line: 0,
            column: 0,
            source: None,
        });
    }



    Ok(Some(Value::Char(number as u8 as char)))

}
