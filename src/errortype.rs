use std::error::Error;
use colored::Colorize;

use crate::Inter::cps::Value;


#[derive(Debug, Clone)]
pub enum ErrorType {
    Lexical, 
    Syntax,
    Return(Value), // For return in functions (as a call, not an error)
    Runtime,
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

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorType::Lexical => write!(f, "Lexical"),
            ErrorType::Syntax => write!(f, "Syntax"),
            ErrorType::Runtime => write!(f, "Runtime"),
            ErrorType::Return(_) => write!(f, "Return"),
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
                let remaining = error_line.len() - (self.column - 1);
                let underline_length = remaining.min(error_line.len());
                
                writeln!(f, "{}", error_line)?;
                writeln!(f, "{}{}",
                    " ".repeat(self.column - 1),
                    "^".repeat(underline_length).bright_red().bold()
                )?;
            }
        }
        
        if let Some(hint) = &self.hint {
            let _ = writeln!(f, "{}: {}", "HINT".bright_yellow().bold(), hint);
        } 

        write!(f, "\n{}",
            format!("Think this is a bug in the interpreter? Report it: {}",
                "https://github.com/faisalfakih/cambridge-psudocode-inter/issues".bright_blue().underline()
            ).dimmed()
        )
    }
}
