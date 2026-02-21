// A WHILE loop repeats a block of code as long as a condition stays TRUE
// Unlike a FOR loop, you control when it stops yourself
// The condition is checked BEFORE each iteration
// If the condition is FALSE from the start, the body never runs at all
// ENDWHILE marks the end of the loop body

DECLARE Countdown : INTEGER
Countdown <- 10

// Keep looping as long as Countdown is greater than 0
// Once Countdown reaches 0, the condition becomes FALSE and the loop stops
WHILE Countdown > 0
    OUTPUT Countdown
    // We must update Countdown ourselves or the loop would run forever
    // This is called decrementing - reducing a value by 1 each time
    Countdown <- Countdown - 1
ENDWHILE

// This line runs after the loop has finished
OUTPUT "Blast off!"
