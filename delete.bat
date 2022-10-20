echo delete Cargo.lock and target? (1 - yes, other - no)

set /p delete=

if [%delete%] == [1] (del Cargo.lock)
if [%delete%] == [1] (rmdir /s/q target)

pause