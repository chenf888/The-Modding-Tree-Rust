@echo off
rem Build the Rust engine to WebAssembly and place it next to index.html.
cd /d "%~dp0"

rem Ensure the wasm target is present. `rustup target add` is idempotent on a
rem healthy install; on installs with a corrupted component manifest it fails
rem while the std files are actually fine, so the error is ignored and cargo
rem gets the final say.
rustup target add wasm32-unknown-unknown >nul 2>&1
cargo build --target wasm32-unknown-unknown --release
if errorlevel 1 goto :fail
copy /y target\wasm32-unknown-unknown\release\tmt_engine.wasm engine.wasm >nul
if errorlevel 1 goto :fail
echo engine.wasm updated
goto :eof

:fail
echo BUILD FAILED
exit /b 1
