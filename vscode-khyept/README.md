# Khyept Language Support for VS Code

Syntax highlighting and basic editor support for the Khyept programming language.

## Features

- `.khyept` and `.kypt` language detection
- comments (`///` line and `/* */` block) and strings, including every escape form:
  `\n` `\t` `\r` `\b` `\f` `\v` `\a` `\0`, octal, `\xhh`, `\uhhhh`, `\Uhhhhhhhh`
- f-strings with `{...}` interpolation, nesting-aware
- keywords, types, struct declarations, constants
- builtins (`len`, `size`, `int`, `float`, `string`, `bool`, `type`, `readln`,
  `print`, `println`, `open`) and file-handle methods (`read`/`write`/`writeln`/`close`)
- operators grouped by meaning: comparison, range (`...`, `<..`, `..<`),
  assignment, increment, arithmetic, bitwise, logical
- highlighting follows the language exactly: `0x` / `0b` literals and scientific
  notation are **not** highlighted because the lexer rejects them
- bracket matching, auto-closing pairs and indentation
- 15 snippets covering functions, variables, `:=` inference, structs, loops,
  `if`, printing, `readln`, lists and dicts

Snippets emit the modern syntax with the type annotation **after** the name
(`var n : int = 5`), which is the preferred form. The legacy prefix style
(`var int n = 5`) still parses, but new code should not use it.

## Compile and run

Open a `.khyept` or `.kypt` file and press `Ctrl+Shift+B`, or run
`Khyept: Compile and Run` from the Command Palette. The extension runs `kyptc` in
VS Code's integrated terminal, so `readln()` input can be entered there. The
`kyptc` executable must be available on `PATH`.

## Install from the VSIX

Run `Extensions: Install from VSIX...` in VS Code and select the generated
`khyept-language-support-0.3.0.vsix` file.

To rebuild the VSIX: copy `package.json`, `extension.js`,
`language-configuration.json`, `LICENSE`, `README.md`, `snippets/` and
`syntaxes/` into `.vsix-stage/extension/`, then zip `.vsix-stage` as a `.vsix`
(OPC format: the manifest at the archive root, assets under `extension/`).