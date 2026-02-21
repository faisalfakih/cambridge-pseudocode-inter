// An array is a collection of variables of the same type stored under one name
// You access individual elements using an index number in square brackets
// A 1D array is like a single row of boxes, each with a numbered position
// ARRAY[1:5] means the array has 5 elements, indexed from 1 to 5
// You must declare arrays just like regular variables

DECLARE Scores : ARRAY[1:5] OF INTEGER
DECLARE Idx : INTEGER
DECLARE Total : INTEGER
Total <- 0

// Fill each element of the array with a random number between 1 and 100
// RAND(100) returns a random REAL between 0.0 and 99.999...
// INT() removes the decimal part, giving 0 to 99
// Adding 1 shifts the range to 1 to 100
FOR Idx <- 1 TO 5
    Scores[Idx] <- INT(RAND(100)) + 1
NEXT Idx

// Read back each element and add it to the total
// Scores[Idx] accesses the element at position Idx
FOR Idx <- 1 TO 5
    OUTPUT "Score ", Idx, ": ", Scores[Idx]
    Total <- Total + Scores[Idx]
NEXT Idx

OUTPUT "Total: ", Total
