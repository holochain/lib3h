@echo off
setlocal enabledelayedexpansion

rem KEEP IN SYNC WITH HOLONIX
set RUST_BACKTRACE=full
cargo test --target-dir c:\build\lib3h\target --verbose
