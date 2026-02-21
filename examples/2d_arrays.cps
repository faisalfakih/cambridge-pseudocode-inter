// A 2D array is like a grid or table with rows and columns
// You need two index values to access an element: [row, column]
// ARRAY[1:3, 1:3] declares a 3x3 grid (3 rows, 3 columns = 9 elements total)
// Nested FOR loops are the standard way to work through every cell in a 2D array

DECLARE Grid : ARRAY[1:3, 1:3] OF INTEGER
DECLARE Row : INTEGER
DECLARE Col : INTEGER

// The outer loop steps through each row (1, 2, 3)
FOR Row <- 1 TO 3
    // The inner loop steps through each column (1, 2, 3) for the current row
    // So for Row=1 we visit [1,1], [1,2], [1,3]
    // Then for Row=2 we visit [2,1], [2,2], [2,3], and so on
    FOR Col <- 1 TO 3
        // Store the product of the row and column number in each cell
        // This creates a simple multiplication table pattern
        Grid[Row, Col] <- Row * Col
    NEXT Col
NEXT Row

// Now read the grid back out using the same nested loop structure
FOR Row <- 1 TO 3
    FOR Col <- 1 TO 3
        OUTPUT "Grid[", Row, ",", Col, "] = ", Grid[Row, Col]
    NEXT Col
NEXT Row
