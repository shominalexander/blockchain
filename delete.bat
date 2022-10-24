echo off

cls

echo delete Cargo.lock file and target folder? ( y - yes, other - no )

set /p delete=

if [%delete%] == [y] (
 del Cargo.lock

 rmdir /s/q target

 pause
)
