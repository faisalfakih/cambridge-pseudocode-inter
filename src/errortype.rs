use std::error::Error;
use colored::Colorize;

use crate::Inter::cps::Value;


#[derive(Debug, Clone)]
pub enum ErrorType {
    Lexical,
    Syntax,
    Return(Value), // For return in functions (as a call, not an error)
    Runtime,
    // Internal signals for the step-based WASM interpreter.
    // These are thrown to unwind the call stack and are always caught by
    // StepInterpreter::step() before reaching any user-facing error path.
    StepOutput(String),
    StepNeedsInput(String),
}

#[derive(Debug, Clone)]
pub struct CPSError {
    pub error_type: ErrorType,
    pub message: String,
    pub hint : Option<String>,
    pub line: usize,
    pub column: usize,
    pub source: Option<String>,
}

impl Error for CPSError {}

impl CPSError {
    // attach the position of the statement the error has unwinded through, keeping whatever position was already recorded (the innermost one is the most specific).
    #[inline]
    pub fn locate(mut self, line: usize, column: usize, source: &str) -> Self {
        if matches!(self.error_type, ErrorType::Return(_) | ErrorType::StepOutput(_) | ErrorType::StepNeedsInput(_)) {
            return self;
        }
        if self.line == 0 && self.column == 0 {
            self.line = line;
            self.column = column;
        }
        if self.source.is_none() {
            self.source = Some(source.to_string());
        }
        self
    }
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorType::Lexical => write!(f, "Lexical"),
            ErrorType::Syntax => write!(f, "Syntax"),
            ErrorType::Runtime => write!(f, "Runtime"),
            ErrorType::Return(_) => write!(f, "Return"),
            ErrorType::StepOutput(_) | ErrorType::StepNeedsInput(_) => write!(f, "Internal"),
        }
    }
}
    


impl std::fmt::Display for CPSError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.line == 0 && self.column == 0 {
            let _ = writeln!(f, "{}: {}: {}",
                "ERROR".bright_red().bold(),
                format!("{:?} Error", self.error_type).bright_red(),
                self.message
            );
        } else {
            let _ = writeln!(f, "{}: {} at line {}, column {}: {}",
                "ERROR".bright_red().bold(),
                format!("{:?} Error", self.error_type).bright_red(),
                self.line,
                self.column,
                self.message
            );
        }
        if let Some(source) = &self.source {
            let lines: Vec<&str> = source.lines().collect();
            
            if self.line > 0 && self.line <= lines.len() {
                let error_line = lines[self.line - 1];
                let start = self.column.saturating_sub(1).min(error_line.len());
                let underline_length = (error_line.len() - start).max(1);

                writeln!(f, "{}", error_line)?;
                writeln!(f, "{}{}",
                    " ".repeat(start),
                    "^".repeat(underline_length).bright_red().bold()
                )?;
            }
        }
        
        if let Some(hint) = &self.hint {
            let _ = writeln!(f, "{}: {}", "HINT".bright_yellow().bold(), hint);
        } 

        write!(f, "\n{}",
            format!("Think this is a bug in the interpreter? Report it: {}",
                "https://github.com/faisalfakih/cambridge-pseudocode-inter/issues".bright_blue().underline()
            ).dimmed()
        )
    }
}
