@echo off
setlocal enabledelayedexpansion

rem KEEP IN SYNC WITH HOLONIX
set RUST_LOG=lib3h=debug
set RUST_BACKTRACE=1
cargo test --target-dir c:\build\lib3h\target --verbose
cargo bench -p lib3h --target-dir c:\build\lib3h\target -j 1 -- --test-threads=1
