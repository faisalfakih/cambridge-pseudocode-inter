// ===========
// DECLARATION
// ===========

// Declare an array called Numbers that holds 10 INTEGER values, indexed 1 to 10
DECLARE Numbers : ARRAY[1 : 10] OF INTEGER

// Sorted is a BOOLEAN flag used to track whether the array is fully sorted
// If no swaps occur in a pass, Sorted remains TRUE and the WHILE loop exits early
DECLARE Sorted : BOOLEAN

// Last stores the index of the last element to be compared in the inner FOR loop
// It shrinks after each pass since the largest unsorted element bubbles to the end
DECLARE Last : INTEGER

// Temp is used to temporarily hold a value during a swap
// Without it, one of the two values being swapped would be overwritten and lost
DECLARE Temp : INTEGER

// Idx is the loop counter used in the fill and output FOR loops
DECLARE Idx : INTEGER

// j is the loop counter used in the inner bubble sort FOR loop
DECLARE j : INTEGER

// ==============
// INITIALISATION
// ==============

// Set Last to 9 (one less than the array length)
// The inner loop compares Numbers[j] with Numbers[j+1]
// So j only needs to go up to index 9, since j+1 would be index 10 (the last element)
// Going further would cause an out-of-bounds access
Last <- 9

// Sorted starts as FALSE to ensure the WHILE loop runs at least once
Sorted <- FALSE

// Temp starts at 0 as a default initial value before any swaps occur
Temp <- 0

// Fill the array with random integers between 1 and 100
// RAND(100) returns a REAL in the range 0 to 99.999...
// INT() truncates it to an integer in the range 0 to 99
// Adding 1 shifts the range to 1 to 100
FOR Idx <- 1 TO 10
    Numbers[Idx] <- INT(RAND(100)) + 1
NEXT Idx

// ===========
// BUBBLE SORT
// ===========

// The WHILE loop repeats passes over the array until no swaps are made
// Each pass guarantees the largest unsorted element reaches its correct position
WHILE NOT Sorted

    // Assume the array is sorted at the start of each pass
    // If any swap is made during this pass, Sorted will be set to FALSE
    // and the loop will run again
    Sorted <- TRUE

    // Compare each adjacent pair from index 1 up to Last
    // After each pass, the element at index Last+1 is in its final sorted position
    // so Last shrinks by 1 at the end of each pass to avoid unnecessary comparisons
    FOR j <- 1 TO Last

        // If the left element is greater than the right, they are in the wrong order
        IF Numbers[j] > Numbers[j + 1] THEN

            // Swap Numbers[j] and Numbers[j+1] using Temp as a temporary holder
            // Step 1: Save Numbers[j] so it is not lost when overwritten
            Temp <- Numbers[j]
            // Step 2: Overwrite Numbers[j] with the smaller value from the right
            Numbers[j] <- Numbers[j + 1]
            // Step 3: Place the saved larger value into the right position
            Numbers[j + 1] <- Temp

            // A swap occurred, so the array is not fully sorted yet
            // Set Sorted to FALSE so the WHILE loop runs another pass
            Sorted <- FALSE

        ENDIF
    NEXT j

    // Shrink Last by 1 after each pass
    // The element now at index Last+1 is the largest of the remaining unsorted elements
    // and is already in its correct final position, so it never needs to be compared again
    Last <- Last - 1

ENDWHILE

// Output the sorted array in order from index 1 to 10
FOR Idx <- 1 TO 10
    OUTPUT Numbers[Idx]
NEXT Idx
