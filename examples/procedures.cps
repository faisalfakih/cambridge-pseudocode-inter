// A procedure is a named block of code you can run (call) from anywhere
// Procedures let you avoid writing the same code over and over
// You define a procedure once using PROCEDURE and close it with ENDPROCEDURE
// Parameters are inputs you pass into the procedure when calling it
// To run a procedure you use the CALL keyword followed by its name and arguments
// A procedure does NOT return a value - it just performs actions

// This procedure takes one STRING parameter called Message
// It prints a decorated version of it
PROCEDURE PrintLine(Message : STRING)
    OUTPUT "--- ", Message, " ---"
ENDPROCEDURE

// This procedure takes a name and age and prints a greeting
// It also calls PrintLine, showing that procedures can call other procedures
PROCEDURE Greet(Name : STRING, Age : INTEGER)
    // Call PrintLine and pass it the string "Greeting" as the argument
    CALL PrintLine("Greeting")
    OUTPUT "Hello, ", Name
    OUTPUT "You are ", Age, " years old"
ENDPROCEDURE

// Call the Greet procedure with two arguments
// "John" is passed as Name, and 18 is passed as Age
CALL Greet("John", 18)
