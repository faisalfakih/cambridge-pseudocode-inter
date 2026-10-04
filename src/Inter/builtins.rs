use crate::{
    errortype::{CPSError, ErrorType},
    Inter::cps::{Date, Type, Value},
};
use rand::random_range;

pub const BUILTIN_IDENTIFIERS: [&str; 22] = [
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
    "LEFT",
    // dates
    "DAY",
    "MONTH",
    "YEAR",
    "DAYINDEX",
    "SETDATE",
    "TODAY",
    // alias for LCASE and UCASE
    "TO_LOWER",
    "TO_UPPER",
];

pub fn call_builtin(name: String, args: &[Value]) -> Result<Option<Value>, CPSError> {
    match name.as_str() {
        name if name == BUILTIN_IDENTIFIERS[0] => builtin_right(args),
        name if name == BUILTIN_IDENTIFIERS[1] => builtin_length(args),
        name if name == BUILTIN_IDENTIFIERS[2] => builtin_mid(args, BUILTIN_IDENTIFIERS[2]),
        name if name == BUILTIN_IDENTIFIERS[3] => builtin_mid(args, BUILTIN_IDENTIFIERS[3]), // "SUBSTRING" alias for MID
        name if name == BUILTIN_IDENTIFIERS[4] => builtin_lcase(args, BUILTIN_IDENTIFIERS[4]),
        name if name == BUILTIN_IDENTIFIERS[5] => builtin_ucase(args, BUILTIN_IDENTIFIERS[5]),
        name if name == BUILTIN_IDENTIFIERS[6] => builtin_int(args),
        name if name == BUILTIN_IDENTIFIERS[7] => builtin_rand(args),
        name if name == BUILTIN_IDENTIFIERS[8] => builtin_num_to_str(args),
        name if name == BUILTIN_IDENTIFIERS[9] => builtin_str_to_num(args),
        name if name == BUILTIN_IDENTIFIERS[10] => builtin_is_num(args),
        name if name == BUILTIN_IDENTIFIERS[11] => builtin_asc(args),
        name if name == BUILTIN_IDENTIFIERS[12] => builtin_chr(args),
        name if name == BUILTIN_IDENTIFIERS[13] => builtin_left(args),
        name if name == BUILTIN_IDENTIFIERS[14] => builtin_day(args),
        name if name == BUILTIN_IDENTIFIERS[15] => builtin_month(args),
        name if name == BUILTIN_IDENTIFIERS[16] => builtin_year(args),
        name if name == BUILTIN_IDENTIFIERS[17] => builtin_day_index(args),
        name if name == BUILTIN_IDENTIFIERS[18] => builtin_set_date(args),
        name if name == BUILTIN_IDENTIFIERS[19] => builtin_today(args),
        name if name == BUILTIN_IDENTIFIERS[20] => builtin_lcase(args, BUILTIN_IDENTIFIERS[20]), // "TO_LOWER" alias for LCASE
        name if name == BUILTIN_IDENTIFIERS[21] => builtin_ucase(args, BUILTIN_IDENTIFIERS[21]), // "TO_UPPER" alias for UCASE
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("Unknown builtin function: {}", name),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        }),
    }
}

// Helper functions for type extraction and conversion
fn expect_string(value: &Value, func: &str, pos: usize) -> Result<String, CPSError> {
    match value {
        // allow char values as strings for now (may change later if asked to)
        Value::String(s) => Ok(s.clone()),
        Value::Char(s) => Ok(s.to_string().clone()),
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} argument {} must be a string", func, pos),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        }),
    }
}

fn expect_string_or_char(
    value: &Value,
    func: &str,
    pos: usize,
) -> Result<(String, Type), CPSError> {
    match value {
        // allow char values as strings for now (may change later if asked to)
        Value::String(s) => Ok((s.clone(), Type::String)),
        Value::Char(s) => Ok((s.to_string().clone(), Type::Char)),
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} argument {} must be a string", func, pos),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        }),
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
        }),
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
        }),
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
        }),
    }
}

fn expect_date(value: &Value, func: &str, pos: usize) -> Result<Date, CPSError> {
    match value {
        Value::Date(d) => Ok(d.to_owned()),
        _ => Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("{} argument {} must be a date", func, pos),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        }),
    }
}

fn arg_count_error(func: &str, expected: usize, got: usize) -> CPSError {
    CPSError {
        error_type: ErrorType::Runtime,
        message: format!(
            "{} expects exactly {} argument(s), got {}",
            func, expected, got
        ),
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

fn builtin_left(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 2 {
        return Err(arg_count_error("LEFT", 2, args.len()));
    }

    let string = expect_string(&args[0], "LEFT", 1)?;
    let length = expect_int(&args[1], "LEFT", 2)?;

    if length < 0 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: "LEFT length must be non-negative".to_string(),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        });
    }

    let chars: Vec<char> = string.chars().collect();
    let length_usize = length as usize;
    let end = length_usize.min(chars.len());

    let result = chars[0..end].iter().collect::<String>();

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

fn builtin_lcase(args: &[Value], name: &str) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error(name, 1, args.len()));
    }

    let (s, type_) = expect_string_or_char(&args[0], name, 1)?;
    let result = s.to_lowercase();
    match type_ {
        Type::Char => {
            let mut chars = result.chars();
            let ch_val = match (chars.next(), chars.next()) {
                (Some(c), None) => c,
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!(
                            "{} result must contain exactly one character for a CHAR argument",
                            name
                        ),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })
                }
            };
            Ok(Some(Value::Char(ch_val)))
        }
        Type::String => Ok(Some(Value::String(result))),
        _ => unreachable!("impossible to get here"),
    }
}

fn builtin_ucase(args: &[Value], name: &str) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error(name, 1, args.len()));
    }

    let (s, type_) = expect_string_or_char(&args[0], name, 1)?;
    let result = s.to_uppercase();
    match type_ {
        Type::Char => {
            let mut chars = result.chars();
            let ch_val = match (chars.next(), chars.next()) {
                (Some(c), None) => c,
                _ => {
                    return Err(CPSError {
                        error_type: ErrorType::Runtime,
                        message: format!(
                            "{} result must contain exactly one character for a CHAR argument",
                            name
                        ),
                        hint: None,
                        line: 0,
                        column: 0,
                        source: None,
                    })
                }
            };
            Ok(Some(Value::Char(ch_val)))
        }
        Type::String => Ok(Some(Value::String(result))),
        _ => unreachable!("impossible to get here"),
    }
}

fn builtin_int(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("INT", 1, args.len()));
    }

    if let Value::String(s) = &args[0] {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("INT cannot be applied to the string '{}'", s),
            hint: Some(
                "Use STR_TO_NUM to convert a string first, e.g. INT(STR_TO_NUM(X))".to_string(),
            ),
            line: 0,
            column: 0,
            source: None,
        });
    }

    let real = expect_real(&args[0], "INT", 1)?;
    let truncated = real.trunc();
    if !truncated.is_finite() {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("INT cannot convert {} to an INTEGER", real),
            hint: Some("The value is infinite or not a number".to_string()),
            line: 0,
            column: 0,
            source: None,
        });
    }
    if truncated < i64::MIN as f64 || truncated > i64::MAX as f64 {
        return Err(CPSError {
            error_type: ErrorType::Runtime,
            message: format!("INT argument {} is outside the range of an INTEGER", real),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        });
    }

    Ok(Some(Value::Integer(truncated as i64)))
}

// REMOVE THIS NOW AS STRING TO INT IS USELESS AFTER THE IMPLEMENTATION OF THE STR_TO_NUM FUNC
// fn builtin_int(args: &[Value]) -> Result<Option<Value>, CPSError> {
//     if args.len() != 1 {
//         return Err(arg_count_error("INT", 1, args.len()));
//     }
//
//     let real = match &args[0] {
//         Value::Real(r) => *r,
//         Value::Integer(i) => *i as f64,
//         Value::String(s) => {
//             s.trim().parse::<f64>().map_err(|_| CPSError {
//                 error_type: ErrorType::Runtime,
//                 message: format!("INT cannot convert string '{}' to a number", s),
//                 hint: Some("String must contain a valid number".to_string()),
//                 line: 0,
//                 column: 0,
//                 source: None,
//             })?
//         },
//         _ => {
//             return Err(CPSError {
//                 error_type: ErrorType::Runtime,
//                 message: "INT argument must be a real number, integer, or numeric string".to_string(),
//                 hint: None,
//                 line: 0,
//                 column: 0,
//                 source: None,
//             });
//         }
//     };
//
//     let result = real.floor() as i64;
//     Ok(Some(Value::Integer(result)))
// }

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
        return Err(arg_count_error("NUM_TO_STR", 1, args.len()));
    }

    let val = &args[0];

    let num = expect_real(val, "NUM_TO_STR", 1)?;

    Ok(Some(Value::String(num.to_string())))
}

fn builtin_str_to_num(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("STR_TO_NUM", 1, args.len()));
    }

    let string = expect_string(&args[0], "STR_TO_NUM", 1)?;

    let num = match string.parse::<f64>() {
        Ok(n) => n,
        Err(_) => {
            return Err(CPSError {
                error_type: ErrorType::Runtime,
                message: "STR_TO_NUM argument must be a numeric string".to_string(),
                hint: None,
                line: 0,
                column: 0,
                source: None,
            })
        }
    };

    Ok(Some(Value::Real(num)))
}

fn builtin_is_num(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("IS_NUM", 1, args.len()));
    }

    let num = expect_string(&args[0], "IS_NUM", 1)?;

    Ok(Some(Value::Boolean(num.parse::<f64>().is_ok())))
}

fn builtin_asc(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("ASC", 1, args.len()));
    }

    let c = expect_char(&args[0], "ASC", 1)?;

    Ok(Some(Value::Integer(c as u8 as i64)))
}

fn builtin_chr(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("CHR", 1, args.len()));
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

fn builtin_day(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("DAY", 1, args.len()));
    }

    let date = expect_date(&args[0], "DAY", 1)?;
    Ok(Some(Value::Integer(date.day as i64)))
}

fn builtin_month(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("MONTH", 1, args.len()));
    }

    let date = expect_date(&args[0], "MONTH", 1)?;
    Ok(Some(Value::Integer(date.month as i64)))
}

fn builtin_year(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("YEAR", 1, args.len()));
    }

    let date = expect_date(&args[0], "YEAR", 1)?;
    Ok(Some(Value::Integer(date.year as i64)))
}

/// sakomotos algo to get an index for the day of the week where 1 = saturday, 2 = monday etc
fn day_of_week(day: u16, month: u16, year: u16) -> i64 {
    const OFFSETS: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];

    let d = day as i64;
    let m = month as i64;
    let mut y = year as i64;

    if m < 3 {
        y -= 1;
    }

    let weekday = (y + y.div_euclid(4) - y.div_euclid(100)
        + y.div_euclid(400)
        + OFFSETS[(m - 1) as usize]
        + d)
        .rem_euclid(7);

    weekday + 1
}

fn builtin_day_index(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 1 {
        return Err(arg_count_error("DAYINDEX", 1, args.len()));
    }

    let date = expect_date(&args[0], "DAYINDEX", 1)?;
    Ok(Some(Value::Integer(day_of_week(
        date.day, date.month, date.year,
    ))))
}

fn builtin_set_date(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if args.len() != 3 {
        return Err(arg_count_error("SETDATE", 3, args.len()));
    }

    let day = expect_int(&args[0], "SETDATE", 1)?;
    let month = expect_int(&args[1], "SETDATE", 2)?;
    let year = expect_int(&args[2], "SETDATE", 3)?;

    let date = Date::parse(&format!("{:02}/{:02}/{:04}", day, month, year)).map_err(|reason| {
        CPSError {
            error_type: ErrorType::Runtime,
            message: format!("SETDATE: {}", reason),
            hint: None,
            line: 0,
            column: 0,
            source: None,
        }
    })?;

    Ok(Some(Value::Date(date)))
}

fn builtin_today(args: &[Value]) -> Result<Option<Value>, CPSError> {
    if !args.is_empty() {
        return Err(arg_count_error("TODAY", 0, args.len()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    let today = chrono::Local::now().format("%d/%m/%Y").to_string();

    #[cfg(target_arch = "wasm32")]
    let today = {
        let now = js_sys::Date::new_0();
        format!(
            "{:02}/{:02}/{:04}",
            now.get_date(),
            now.get_month() + 1,
            now.get_full_year()
        )
    };

    let date = Date::parse(&today).map_err(|reason| CPSError {
        error_type: ErrorType::Runtime,
        message: format!("TODAY: {}", reason),
        hint: None,
        line: 0,
        column: 0,
        source: None,
    })?;

    Ok(Some(Value::Date(date)))
}
