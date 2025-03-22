# Changelog

## [0.2.0] - 2025-03-22
### Added
- Support for `Ctrl+A` (move to start of line) and `Ctrl+E` (move to end of line).
- Ability to delete the next character with `EOF` (Ctrl+D).
- Improved cursor handling for more accurate text navigation.

### Fixed
- Fixed all bugs related to the prompt string (`$PS1`).
- Fixed `Backspace` behavior in the middle of the input.
- Made `EOF` only exit when the input line is empty.
- Fixed parameter expansion. Only simple expansions are supported at the
present moment.

## [0.1.0] - 2024-09-28
- Initial release.
