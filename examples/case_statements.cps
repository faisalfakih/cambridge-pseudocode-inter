// CASE statements are a cleaner alternative to many nested IF statements
// They check a single variable against a list of possible values
// The program runs the block next to the first value that matches
// OTHERWISE acts like a final ELSE - it runs if nothing else matched
// Every CASE statement must be closed with ENDCASE

DECLARE Day : INTEGER
Day <- 3

CASE OF Day
    // If Day = 1, output Monday
    1 : OUTPUT "Monday"
    // If Day = 2, output Tuesday
    2 : OUTPUT "Tuesday"
    // Day is 3, so this line will run
    3 : OUTPUT "Wednesday"
    4 : OUTPUT "Thursday"
    5 : OUTPUT "Friday"
    6 : OUTPUT "Saturday"
    7 : OUTPUT "Sunday"
    // If Day was none of the above values, this runs instead
    OTHERWISE : OUTPUT "Invalid day"
ENDCASE
