# Cambridge Pseudocode Interpreter

![Stars](https://img.shields.io/github/stars/faisalfakih/cambridge-psudocode-inter)
![Version](https://img.shields.io/github/v/release/faisalfakih/cambridge-psudocode-inter)
![License](https://img.shields.io/github/license/faisalfakih/cambridge-psudocode-inter)

A Rust-based interpreter for Cambridge International AS & A Level Computer Science (9618) pseudocode specification. This project implements a complete interpreter capable of parsing and executing pseudocode according to the Cambridge syllabus standards.

## 📦 Installation

### Linux/macOS
```bash
curl -s https://raw.githubusercontent.com/faisalfakih/cambridge-psudocode-inter/main/install/install.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/faisalfakih/cambridge-psudocode-inter/main/install/install.ps1 | iex
```

After installation, the `cps` command will be available globally:

```bash
cps --version
cps --help
```

### To update CPS
Simply run the install script again — it will always pull the latest release.

### Build from source
```bash
git clone https://github.com/faisalfakih/cambridge-psudocode-inter
cd cambridge-psudocode-inter
cargo build --release
./target/release/cps yourfile.cps
```


## 🚀 Quick Start

### Using the Interpreter

Create a file named `main.cps`:

```pseudocode
DECLARE name : STRING

OUTPUT "Enter your name: "
INPUT name
OUTPUT "Hello, " & name & "!"
```

#### Run it:

```bash
cps main.cps  # Replace main.cps with the name of the file you made
```

OR 


```bash
cps  # This only works if the name of the file is main.cps
```

### Command Line Options

```bash
# Run a pseudocode file
cps main.cps

# Show verbose output (tokens and AST)
cps main.cps --verbose
cps main.cps -v

# Show help
cps --help

# Show version
cps --version
```

## Project Overview

This interpreter is designed to help students and educators work with Cambridge 9618 pseudocode by providing a fully functional execution environment. It handles the complete pseudocode syntax including variable declarations, control structures, expressions, and I/O operations.

## Features

### Supported Language Features

- **Data Types**
  - `INTEGER` - Whole numbers
  - `REAL` - Floating-point numbers
  - `STRING` - Text data
  - `CHAR` - Single characters
  - `BOOLEAN` - True/False values
  - `ARRAY` - One-dimensional arrays (2D arrays in progress)
  
- **Variable Operations**
  - `DECLARE` statements with type annotations
  - Assignment using `<-` operator
  - Proper variable scoping
  
- **Control Structures**
  - `IF...THEN...ELSE...ENDIF` conditionals
  - `CASE...OF...ENDCASE` statements with ranges
  - `WHILE...ENDWHILE` loops
  - `REPEAT...UNTIL` loops
  - `FOR...TO...NEXT` loops with STEP support
  - `FUNCTIONS` and `PROCEDURES` with return values and parameters
  
- **Operators**
  - Arithmetic: `+`, `-`, `*`, `/`, `DIV`, `MOD`, `^` (power)
  - Comparison: `=`, `<>`, `<`, `<=`, `>`, `>=`
  - Logical: `AND`, `OR`, `NOT`
  - String concatenation: `&`
  
- **I/O Operations**
  - `OUTPUT` - Display values to console
  - `INPUT` - Read user input
  
- **Built-in Functions**
  - `RIGHT(string, length)` - Extract rightmost characters
  - `LENGTH(string)` - Get string length
  - `MID(string, start, length)` - Extract substring (1-indexed)
  - `LCASE(string)` - Convert to lowercase
  - `UCASE(string)` - Convert to uppercase
  - `INT(value)` - Convert to integer
  - `RAND(max)` - Generate random number [0, max)

## 💡 Motivation

You can't learn to ride a skateboard by watching someone else ride one. This concept applies not only to skateboarding but to almost every field that has ever existed. For this very reason, the best way to learn programming is through hands-on practice—writing code, testing it, debugging it, and iterating. However, for students studying Cambridge Computer Science, the lack of a reliable, freely available pseudocode interpreter has been a significant barrier. Without a way to execute their pseudocode, learners are forced to manually trace through logic or rely on expensive proprietary tools, which limits experimentation and slows down the learning process. This project aims to remove that barrier by providing an accessible, open-source interpreter that empowers students to learn through doing—running their code, seeing immediate results, and developing a deeper intuition for programming concepts.

As a student working with the Cambridge International curriculum myself, I experienced this gap firsthand. This interpreter was born from both personal need and a desire to give back to the community:

- **Immediate feedback** - Run and test pseudocode without manual execution
- **Learning by doing** - Enable experimentation and iterative development
- **Community resource** - Provide a free tool for fellow students and educators worldwide
- **Skill demonstration** - Showcase compiler theory, type systems, and Rust proficiency

## 🏗️ Architecture

The interpreter follows a classic three-stage architecture:

```
Source Code → Lexer → Parser → Interpreter → Output
```

### Components

1. **Lexer** (`Lexer/lexer.rs`)
   - Tokenizes source code into meaningful tokens
   - Handles keywords, operators, literals, and identifiers

2. **Parser** (`Parser/parser.rs`, `Parser/ast.rs`)
   - Implements Pratt parsing for expressions
   - Builds Abstract Syntax Tree (AST)
   - Validates syntax according to Cambridge specification

3. **Interpreter** (`Inter/interpreter.rs`)
   - Evaluates AST nodes
   - Manages runtime environment and variable storage
   - Performs type checking and conversion
   - Executes control flow structures

4. **Error Handling** (`errortype.rs`)
   - Comprehensive error reporting with line/column information
   - Helpful hints for common mistakes
   - Runtime and syntax error differentiation

## 📚 Example Programs

### Hello World

```pseudocode
OUTPUT "Hello, World!"
```

### Variable Operations

```pseudocode
DECLARE x : INTEGER
DECLARE y : INTEGER
DECLARE result : INTEGER

x <- 10
y <- 20
result <- x + y

OUTPUT "Sum: " & result
```

### Loops and Conditionals

```pseudocode
DECLARE i : INTEGER

FOR i <- 1 TO 10
    IF i MOD 2 = 0 THEN
        OUTPUT i & " is even"
    ELSE
        OUTPUT i & " is odd"
    ENDIF
NEXT i
```

### Arrays

```pseudocode
DECLARE numbers : ARRAY[1:5] OF INTEGER
DECLARE i : INTEGER

FOR i <- 1 TO 5
    numbers[i] <- i * 10
    OUTPUT numbers[i]
NEXT i
```

### Functions

```pseudocode
FUNCTION Square(n : INTEGER) RETURNS INTEGER
    RETURN n * n
ENDFUNCTION

DECLARE result : INTEGER
result <- Square(5)
OUTPUT "5 squared is " & result
```

## Language Syntax

### Variable Declaration
```pseudocode
DECLARE variableName : TYPE
```

### Assignment
```pseudocode
variableName <- expression
```

### Conditional Statements
```pseudocode
IF condition THEN
    // statements
ELSE
    // statements
ENDIF
```

### Loops
```pseudocode
// While loop
WHILE condition
    // statements
ENDWHILE

// Repeat-Until loop
REPEAT
    // statements
UNTIL condition

// For loop
FOR counter <- start TO end
    // statements
NEXT counter

// For loop with step
FOR counter <- start TO end STEP increment
    // statements
NEXT counter
```

### Case Statements
```pseudocode
CASE OF variable
    value1 : // statements
    value2 TO value3 : // statements
    OTHERWISE : // statements
ENDCASE
```

### Input/Output
```pseudocode
INPUT variableName
OUTPUT expression
```

## Technical Details

### Type System

The interpreter implements a strict type system with runtime type checking:

- All numeric literals are initially parsed as `Real`
- Type conversion occurs automatically during:
  - Variable assignment (Real → Integer or Integer → Real based on declared type)
  - Input operations (string input converted to declared variable type)
  - Arithmetic operations (mixed Integer/Real operations promote to Real)

### Expression Evaluation

- Uses Pratt parsing algorithm for operator precedence
- Supports nested expressions with parentheses
- Handles operator associativity (left/right)
- Precedence levels:
  - 30: `^` (power)
  - 20: `*`, `/`, `DIV`, `MOD`
  - 10: `+`, `-`
  - 8: `&` (concatenation)
  - 5: Comparison operators
  - 3: `AND`
  - 2: `OR`

### Environment Management

- Hierarchical environment structure supporting nested scopes
- Parent-child relationship for scope inheritance
- Variable lookup traverses scope chain
- Type information stored alongside values


## Current Status

### ✅ Implemented
- Complete lexer with all Cambridge pseudocode tokens
- Full expression parser with operator precedence
- Statement parsing (declarations, assignments, control structures)
- Runtime interpreter with type system
- Variable environment with scoping
- I/O operations
- All basic data types
- Arrays (1D)
- Functions and procedures
- All loop types (FOR, WHILE, REPEAT)
- CASE statements with ranges
- 2D array support
- File I/O operations

### 🚧 In Progress
- User-defined types

### 📋 Planned
- Advanced A-Level features

## Contributing

Contributions are welcome! This project is being developed as part of learning compiler/interpreter design and helping students with Cambridge Computer Science.

### Areas for Contribution
- Additional language features
- Test cases and examples
- Documentation improvements
- Bug fixes and optimizations
- Educational resources

## Resources

- [Cambridge Pseudocode Guide for Teachers (PDF)](https://www.cambridgeinternational.org/Images/697401-2026-pseudocode-guide-for-teachers.pdf)
- [Project Repository](https://github.com/faisalfakih/cambridge-pseudocode-inter)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built as a learning project for understanding interpreter design
- Inspired by the need for accessible Cambridge pseudocode tools
- Thanks to the Rust community for excellent documentation and libraries

## 📧 Contact

For questions, suggestions, or discussions about the project:
- Open an issue on [GitHub](https://github.com/faisalfakih/cambridge-pseudocode-inter/issues)
- Email: me@faisalfakih.com

---

**Note**: This interpreter is an educational tool and may not cover every edge case in the Cambridge specification. Always refer to official Cambridge resources for authoritative information on pseudocode syntax and semantics.
