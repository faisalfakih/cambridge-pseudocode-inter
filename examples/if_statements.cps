// IF statements let your program make decisions
// The program checks a condition - if it is TRUE, it runs the THEN block
// If the condition is FALSE, it runs the ELSE block instead (if one exists)
// Every IF statement must be closed with ENDIF

DECLARE Score : INTEGER
Score <- 75

// A simple IF with an ELSE
// Checks whether Score is greater than or equal to 50
IF Score >= 50 THEN
    OUTPUT "You passed"
ELSE
    // This runs only if the condition above was FALSE
    OUTPUT "You failed"
ENDIF

// IF statements can be nested inside each other
// This lets you check multiple conditions in sequence
// The program checks each condition from top to bottom and stops at the first TRUE one
IF Score >= 90 THEN
    OUTPUT "Grade: A"
ELSE
    // If we reach here, Score was less than 90
    IF Score >= 75 THEN
        OUTPUT "Grade: B"
    ELSE
        // If we reach here, Score was less than 75
        IF Score >= 50 THEN
            OUTPUT "Grade: C"
        ELSE
            // If we reach here, Score was less than 50
            OUTPUT "Grade: F"
        ENDIF
    ENDIF
ENDIF
