# Khyept Language Support for VS Code

Syntax highlighting and basic editor support for the Khyept programming language.

Features:

- `.khyept` and `.kypt` language detection
- comments, strings, f-strings and interpolation
- string escapes: `\\n`, `\\t`, `\\r`, `\\b`, `\\f`, `\\v`, `\\a`, `\\0`, octal, hex and Unicode forms
- keywords, types, structures, constants, functions, numbers and operators
- `if`/`elif` and `switch`/`case`/`default`/`break` syntax highlighting
- bracket matching, auto-closing pairs and indentation
- snippets for functions, variables, `print` and `println`

## Compile and run

Open a `.khyept` or `.kypt` file and press `Ctrl+Shift+B`, or run `Khyept: Compile and Run` from the Command Palette. The extension runs `kyptc` in VS Code's integrated terminal, so `readln()` input can be entered there. The `kyptc` executable must be available on `PATH`.

## Install from the VSIX

Run `Extensions: Install from VSIX...` in VS Code and select the generated `khyept-language-support-0.2.0.vsix` file.
