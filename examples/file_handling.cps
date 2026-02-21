// Files let your program save data that persists after the program ends
// You must open a file before reading or writing, and close it when done
// OPENFILE opens a file in a given mode:
//   WRITE  - creates the file (or overwrites it) and lets you write to it
//   READ   - opens an existing file so you can read from it
//   APPEND - opens an existing file and adds new data after what is already there
// WRITEFILE writes a line of text into an open file
// READFILE reads the next line from an open file into a variable
// CLOSEFILE closes the file when you are done with it
// EOF(filename) returns TRUE when there are no more lines left to read

DECLARE LineOfText : STRING
DECLARE Name : STRING
DECLARE Score : INTEGER

Name <- "John"
Score <- 95

// Open the file in WRITE mode so we can save data to it
// If the file already exists its contents will be erased
OPENFILE "results.txt" FOR WRITE
    // Each WRITEFILE call writes one line to the file
    WRITEFILE "results.txt", Name
    WRITEFILE "results.txt", Score
CLOSEFILE "results.txt"

// Now open the same file in READ mode to read the data back
OPENFILE "results.txt" FOR READ
    // Keep reading lines until we reach the end of the file
    // EOF returns TRUE when there are no more lines to read
    WHILE NOT EOF("results.txt")
        // Read the next line and store it in LineOfText
        READFILE "results.txt", LineOfText
        OUTPUT LineOfText
    ENDWHILE
CLOSEFILE "results.txt"
