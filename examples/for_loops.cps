// A FOR loop repeats a block of code a fixed number of times
// You give it a variable (the loop counter), a start value, and an end value
// The counter automatically increases by 1 each time through the loop
// NEXT marks the end of the loop body and triggers the next iteration
// The loop stops after the counter passes the end value

DECLARE Idx : INTEGER
DECLARE Total : INTEGER
Total <- 0

// Idx starts at 1 and goes up to 10
// Each time through the loop, Idx increases by 1 automatically
// So this loop runs exactly 10 times (Idx = 1, 2, 3, ..., 10)
FOR Idx <- 1 TO 10
    // Add the current value of Idx to Total
    // On the first pass Total = 0 + 1 = 1
    // On the second pass Total = 1 + 2 = 3, and so on
    Total <- Total + Idx
NEXT Idx

// After the loop, Total holds 1+2+3+4+5+6+7+8+9+10 = 55
OUTPUT "Sum of 1 to 10 is: ", Total
