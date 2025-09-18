# Rustdown Changelog

## Recent Improvements (September 2025)

### 🎯 Major Enhancements

#### Table Rendering Improvements
- **Cell trimming**: Automatically trims whitespace from table cells for cleaner display
- **Better alignment**: Improved column width calculation and padding for consistent table formatting
- **Inline formatting preservation**: Table cells now properly preserve **bold**, *italic*, and `code` formatting
- **Minimum column widths**: Ensures tables remain readable even with short content

#### Nested List Formatting & CommonMark Compatibility
- **Improved bullet selection**: Now uses CommonMark-compatible markers:
  - Top level: `-` (dash)
  - Second level: `*` (asterisk) 
  - Third level and beyond: `+` (plus)
- **Proper nesting indentation**: Consistent 2-space indentation per nesting level
- **Ordered list handling**: Improved numbering for nested ordered lists

#### Code Block Display Options
- **Preserve fences mode**: New environment variable `RUSTDOWN_PRESERVE_FENCES` to show original fenced code blocks
  - When enabled: Shows `\`\`\`lang` and `\`\`\`` around code blocks
  - When disabled: Shows language label `[lang]` with indented code (default behavior)
- **Exact content preservation**: Code blocks maintain original formatting and indentation

#### Task List Support
- **Visual checkboxes**: Renders `- [x]` as ☑ and `- [ ]` as ☐
- **Nested task lists**: Proper handling of task lists within nested list structures
- **Color coding**: Completed tasks shown in green, incomplete in white

### 🧪 Testing Infrastructure
- **Comprehensive test suite**: Added 9+ test cases covering all major markdown features
- **Reference file testing**: Tests against actual markdown files to prevent regressions
- **Feature-specific tests**: Individual tests for lists, tables, code blocks, and formatting
- **ANSI handling**: Tests properly account for color codes in terminal output

### 🔧 Technical Improvements
- **Library module**: Created `lib.rs` for better code organization and testability
- **Buffer-based rendering**: Improved output handling with `termcolor::Buffer`
- **State management**: Enhanced tracking of rendering context and formatting states
- **Error handling**: Better error propagation and handling throughout the rendering pipeline

### 📝 Code Quality
- **Modular design**: Separated concerns between rendering logic and terminal output
- **Documentation**: Added comprehensive inline documentation and examples
- **Test coverage**: High test coverage to ensure reliability and prevent regressions

## Example Output

```bash
# Basic usage
cargo run README.md

# Enable fence preservation mode
RUSTDOWN_PRESERVE_FENCES=1 cargo run document.md

# Run tests
cargo test
```

## Commit Messages Summary

Recent development focused on:
1. `feat: improve table rendering with cell trimming and better alignment`
2. `feat: implement CommonMark-compatible list markers and nesting`
3. `feat: add code fence preservation mode via environment variable`
4. `feat: enhance task list support with visual checkboxes`
5. `test: add comprehensive test suite for markdown rendering`
6. `refactor: create library module for better code organization`
7. `docs: add changelog and improve code documentation`

These improvements significantly enhance the terminal markdown rendering experience while maintaining backward compatibility and adding new functionality for power users.
