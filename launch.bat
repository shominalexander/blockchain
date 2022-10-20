echo on

cargo build

pause

start "one" cargo run -- "one"

pause

start "two" cargo run -- "two"

pause

start "three" cargo run -- "three"

pause

start "four" cargo run -- "four"

pause

start "five" cargo run -- "five"

pause
