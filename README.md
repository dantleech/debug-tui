Debug-TUI
=========

Interactive [Xdebug](https://xdebug.org) step-debugging client your terminal.

![Image](https://github.com/user-attachments/assets/21627682-b2f1-4622-b67d-ff6cd32e4363)

- **Travel forwards**: step over, into and out.
- **Travel backwards**: it's not quite time travel, but you can revisit
  previous steps in _history mode_.
- **Jump the stack**: jump up and down the stack.
- **Vim-like motions**: Typing `100n` will repeat "step into" 100 times.
- **Inline values**: Show variable values inline with the source code.

## Installation

- Download the [latest release](https://github.com/dantleech/debug-tui/releases/latest)
- Compile it yourself `cargo build`

## CLI options

- `--log`: Debug log to file.
- `--listen`: Listen on an alternative address (defaults to `0.0.0.0:9003`).

## Key bindings

Prefix with number to repeat:

- `r`     run
- `n`     next / step into
- `N`     step over
- `p`     previous (switches to history mode if in current mode)
- `o`     step out
- `j`     down
- `J`     down 10
- `k`     up
- `K`     up 10
- `h`     left
- `H`     left 10
- `l`     right
- `L`     right 10
- `+`     increase context depth
- `-`     decrease context depth
- `tab`   switch pane
- `enter` toggle pane focus (full screen)
- `t`     rotate the theme
- `?`     Show help
- `f`     Filter (context pane) - use dot notation to filter on multiple levels.

## Setting Breakpoints

`debug-tui` has no mechanism for setting a breakpoint but you can use the
function `xdebug_break()` in your code:

```php
<?php

function my_function() {
    xdebug_break(); // break after this line
}
```
