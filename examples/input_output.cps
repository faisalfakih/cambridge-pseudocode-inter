// INPUT and OUTPUT are how your program communicates with the user
// OUTPUT displays text or values on the screen
// INPUT waits for the user to type something and stores it in a variable

DECLARE Name : STRING
DECLARE Age : INTEGER

// OUTPUT a prompt so the user knows what to type
OUTPUT "Enter your name:"

// INPUT reads whatever the user types and stores it in Name
INPUT Name

OUTPUT "Enter your age:"
INPUT Age

// You can OUTPUT multiple values on one line by separating them with commas
// The commas here concatinate (join) the strings
OUTPUT "Hello, ", Name, "! You are ", Age, " years old."
