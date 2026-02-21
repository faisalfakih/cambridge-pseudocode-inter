// A function is like a procedure but it also returns a value back to the caller
// You define a function with FUNCTION and must state what type it returns using RETURNS
// Inside the function, RETURN sends a value back and exits the function immediately
// You do NOT use CALL for functions - instead you use them directly in expressions
// Close every function with ENDFUNCTION

// This function takes two integers and returns whichever is larger
FUNCTION Max(A : INTEGER, B : INTEGER) RETURNS INTEGER
    IF A > B THEN
        RETURN A  // Send A back to whoever called this function
    ELSE
        RETURN B  // Send B back instead
    ENDIF
ENDFUNCTION

// This function checks whether a number is even and returns a BOOLEAN
// A number is even if the remainder when divided by 2 is 0
// MOD gives the remainder of integer division
FUNCTION IsEven(N : INTEGER) RETURNS BOOLEAN
    IF N MOD 2 = 0 THEN
        RETURN TRUE
    ELSE
        RETURN FALSE
    ENDIF
ENDFUNCTION

DECLARE Result : INTEGER
DECLARE Check : BOOLEAN

// Call Max and store the returned value in Result
// The function runs, computes the answer, and the result replaces the call
Result <- Max(42, 17)

// Call IsEven with Result and store the returned BOOLEAN in Check
Check <- IsEven(Result)

OUTPUT "The larger number is: ", Result

// Use the BOOLEAN variable Check in an IF condition
IF Check THEN
    OUTPUT "It is even"
ELSE
    OUTPUT "It is odd"
ENDIF
