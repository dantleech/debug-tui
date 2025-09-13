CHANGELOG
=========

main
----

### Features

- Add support for arrow key scrolling
- Show historic inline values
- Introduce eval feature

### Improvements

- Improve scrolling performance on large files and contexts

### Bug fixes

- Fix position of closing braces in context view
- Do not show "undefined" variables
- Ensure context depth is in range of 1-9
- Fix display of multibyte values #27

0.1.1
-----

- Fix scroll limitation in source view #41

0.1.0
-----

- Improve notification display
- Do not accept connections while an existing session is running
- Do not immediately switch back to listen mode when server disconnects.
- Filter properties with dot notation in context pane (`f`)
- Stack traversal - select stack and inspect stack frames in current mode.
- Fixed light theme.

0.0.4
-----

- Fix out-of-bounds rendering issue

0.0.3
-----

- Horizontal scrolling
- Improved property rendering
- Introduced themes (including solarized and solarized dark)
- Show value of variables on current line
- Support `extended_properties` #5


0.0.2
-----

- Fix property value decoding for ints, floats and bools and generally anything that is not base64 #19
