Debug-TUI
=========

Interactive XDebug step debugger for your terminal with vim-like key bindings.

![Screenshot](https://github.com/user-attachments/assets/9f938d2b-717b-4816-bb35-9f317f82a0a3)

- **Travel forwards**: step over, into and out.
- **Travel backwards**: it's not quite time travel, but you can revisit
  previous steps.
- **Vim-like motions**: Typing `100n` will repeat "step into" 100 times.

CLI options:

- `--log`: Debug log to file.
- `--listen`: Listen on an alternative address (defaults to `0.0.0.0:9003`).

Key bindings (prefix with number to repeat):

- `n`     next / step into
- `N`     step over
- `p`     previous (switches to history mode if in current mode)
- `o`     step out
- `j`     scroll down (with shift to scroll down 10)
- `k`     scroll up (with shift to scroll up 10)
- `tab`   switch pane
- `enter` toggle pane focus (full screen)
- `?`     Show help
