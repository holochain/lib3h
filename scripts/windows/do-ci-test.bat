@echo off
setlocal enabledelayedexpansion

rem KEEP IN SYNC WITH HOLONIX
set nightly-date=nightly-2019-07-14
rustup toolchain install --no-self-update !nightly-date!
rustup default !nightly-date!

cargo test --verbose
