@echo off
setlocal enabledelayedexpansion

rem KEEP IN SYNC WITH HOLONIX
cargo test --release -p lib3h --target-dir c:\build\lib3h\target --verbose