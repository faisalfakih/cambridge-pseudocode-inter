// DATE stores a calendar date - a day, a month and a year together
// A date literal is written as dd/mm/yyyy with no spaces around the slashes
// The day and month must always be two digits and the year four digits,
// so 05/03/2026 is a date but 5/3/2026 is read as ordinary division
// Dates are checked when the program is read, so an impossible date such as
// 31/02/2026 is reported as an error before the program runs

DECLARE Today : DATE
DECLARE Deadline : DATE
DECLARE Earliest : DATE

// A date declared but not yet assigned starts at 01/01/1900
OUTPUT "An unassigned date starts as ", Today

Today <- 05/03/2026
Deadline <- 17/10/2026

// Dates are always displayed as dd/mm/yyyy, padded with zeros
OUTPUT "Today is ", Today
OUTPUT "The deadline is ", Deadline

// Use & to join a date onto a string
OUTPUT "Reminder: report due " & Deadline

// Dates can be compared with all six relational operators
// Comparing them tests which date is earlier in the calendar,
// so the year is considered first, then the month, then the day
IF Today < Deadline THEN
    OUTPUT "There is still time before the deadline"
ELSE
    OUTPUT "The deadline has passed"
ENDIF

IF Today = Today THEN
    OUTPUT "A date always equals itself"
ENDIF

IF Today <> Deadline THEN
    OUTPUT "These two dates are different"
ENDIF

// A constant can hold a date literal
CONSTANT Epoch = 01/01/1970

IF Today > Epoch THEN
    OUTPUT "Today comes after ", Epoch
ENDIF


// A function can take dates as parameters and return a date
FUNCTION EarlierOf(BYVAL First : DATE, BYVAL Second : DATE) RETURNS DATE
    IF First <= Second THEN
        RETURN First
    ELSE
        RETURN Second
    ENDIF
ENDFUNCTION

OUTPUT "The earlier of the two is ", EarlierOf(Today, Deadline)


// Arrays can hold dates just like any other type
DECLARE Milestones : ARRAY[1:4] OF DATE
DECLARE Index : INTEGER

Milestones[1] <- 12/06/2026
Milestones[2] <- 28/02/2026
Milestones[3] <- 01/11/2026
Milestones[4] <- 09/09/2026

// Because dates can be compared, they can be searched and sorted
// This loop finds the earliest date in the array
Earliest <- Milestones[1]
FOR Index <- 2 TO 4
    IF Milestones[Index] < Earliest THEN
        Earliest <- Milestones[Index]
    ENDIF
NEXT Index

OUTPUT "The earliest milestone is ", Earliest


// CASE works with dates, both for an exact match and for a range
CASE OF Deadline
    01/01/2026 : OUTPUT "The deadline is New Year's Day"
    01/01/2026 TO 30/06/2026 : OUTPUT "The deadline is in the first half of 2026"
    01/07/2026 TO 31/12/2026 : OUTPUT "The deadline is in the second half of 2026"
    OTHERWISE : OUTPUT "The deadline is not in 2026"
ENDCASE


// 2029 is not a leap year, so February has only 28 days that year.
// Writing 29/02/2029 here would stop the program with an error.
// 2028 is a leap year, so this date is accepted
DECLARE LeapDay : DATE
LeapDay <- 29/02/2028
OUTPUT "The next leap day is ", LeapDay
