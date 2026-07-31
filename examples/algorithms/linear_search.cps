// ===========
// DECLARATION
// ===========

// Declare an array called Numbers that holds 10 INTEGER values, indexed 1 to 10
DECLARE Numbers : ARRAY[1 : 10] OF INTEGER

// Target is the value the user wants to search for in the array
DECLARE Target : INTEGER

// Store the user input as a string
DECLARE UserInput : STRING

// Found is a BOOLEAN flag used to track whether the target has been found
// It starts as FALSE and is set to TRUE if a match is found
DECLARE Found : BOOLEAN

// FoundAt stores the index of the element that matched the target
// It is only meaningful if Found is TRUE
DECLARE FoundAt : INTEGER

// Idx is the loop counter used to step through each element of the array
DECLARE Idx : INTEGER

// ==============
// INITIALISATION
// ==============

// Found starts as FALSE since the target has not been found yet
Found <- FALSE

// FoundAt starts at 0 as a default value since no match has been found yet
FoundAt <- 0
 
// Initialize the target to 0
Target <- 0 

// Fill the array with random integers between 1 and 100
// RAND(100) returns a REAL in the range 0 to 99.999...
// INT() truncates it to an integer in the range 0 to 99
// Adding 1 shifts the range to 1 to 100
FOR Idx <- 1 TO 10
    Numbers[Idx] <- INT(RAND(100)) + 1
NEXT Idx

// Ask the user to enter the value they want to search for
OUTPUT "Enter the number to search for:"
INPUT UserInput

Target <- INT(STR_TO_NUM(UserInput)) // Convert the user input to an integer and store in target

// =============
// LINEAR SEARCH
// =============

// Step through each element in the array one by one from index 1 to 10
// Linear search checks every element in order until a match is found or the array ends
FOR Idx <- 1 TO 10

    // Compare the current element to the target value
    IF Numbers[Idx] = Target THEN

        // A match has been found
        // Record the index where the match was found
        FoundAt <- Idx

        // Set Found to TRUE so the result can be checked after the loop
        Found <- TRUE

    ENDIF

NEXT Idx

// ======
// OUTPUT
// ======

// Check whether the target was found anywhere in the array
IF Found THEN
    // The target was found, output the index where it was located
    OUTPUT Target, " was found at index ", FoundAt
ELSE
    // The target was not found in the array
    OUTPUT Target, " was not found in the array"
ENDIF
