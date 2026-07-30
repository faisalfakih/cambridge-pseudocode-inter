# Cambridge Pseudocode Interpreter

![Stars](https://img.shields.io/github/stars/faisalfakih/cambridge-pseudocode-inter)
![Version](https://img.shields.io/github/v/release/faisalfakih/cambridge-pseudocode-inter)
![License](https://img.shields.io/github/license/faisalfakih/cambridge-pseudocode-inter)

A Rust-based interpreter for Cambridge International AS & A Level Computer Science (9618) pseudocode specification. This project implements a complete interpreter that parses and executes pseudocode in accordance with the Cambridge syllabus standards.

## Try it instantly

Use the interpreter directly in your browser with no installation required:

https://www.cambridge-pseudocode.com/

### Features
- Full online IDE for writing and running pseudocode
- Practice problems (both exam-style and LeetCode-style questions available)
- Built-in learning resources for the Cambridge pseudocode specification
  
##  Installation

### Linux/macOS
```bash
curl -s https://raw.githubusercontent.com/faisalfakih/cambridge-pseudocode-inter/main/install/install.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/faisalfakih/cambridge-pseudocode-inter/main/install/install.ps1 | iex
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
git clone https://github.com/faisalfakih/cambridge-pseudocode-inter
cd cambridge-pseudocode-inter
cargo build --release
./target/release/cps yourfile.cps
```


## Quick Start

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

## Architecture

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
   - Validates syntax according to the Cambridge specification

3. **Interpreter** (`Inter/interpreter.rs`)
   - Evaluates AST nodes
   - Manages runtime environment and variable storage
   - Performs type checking and conversion
   - Executes control flow structures

4. **Error Handling** (`errortype.rs`)
   - Comprehensive error reporting 
   - Helpful hints for common mistakes
   - Differentation for different error types such as runtime and syntax errors
   
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

- [Cambridge Pseudocode Guide for Teachers (PDF)](https://www.cambridgeinternational.org/Images/721401-2027-2029-pseudocode-guide.pdf)
- [Project Repository](https://github.com/faisalfakih/cambridge-pseudocode-inter)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## 📧 Contact

For questions, suggestions, or discussions about the project:
- Open an issue on [GitHub](https://github.com/faisalfakih/cambridge-pseudocode-inter/issues)
- Email: me@faisalfakih.com

---

**Note**: This interpreter is an educational tool and may not cover every edge case in the Cambridge specification. Always refer to official Cambridge resources for authoritative information on pseudocode syntax and semantics.
