# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build          # Build the project
cargo run            # Process inputs/*.txt -> outputs/story.txt
cargo test           # Run all 63 unit tests
cargo test lexer     # Run only lexer tests
cargo test parser    # Run only parser tests
cargo test renderer  # Run only renderer tests
cargo test <name>    # Run a single test by name, e.g. cargo test parse_multi_sticker
```

## Architecture

A 3-stage pipeline that compiles Arknights game story scripts (tagged narrative format) into a single plain Markdown file.

**Pipeline**: `inputs/*.txt` → Lexer → Parser → Renderer → `outputs/story.txt`

- **Lexer** (`src/lexer.rs`): Tokenizes each line into `Tag { name, raw_inner }` and `Text` tokens. Quote-aware bracket matching ensures `]` inside quoted strings doesn't break tags. `extract_param()` pulls named values from tag internals.

- **Parser** (`src/parser.rs`): Converts token streams into semantic `Event`s: `Narration`, `Dialogue { speaker, text }`, `Subtitle`, `Sticker`. Handles multi-sticker accumulation by id, strips XML/XHTML tags (`<i>`, `<color>`), and expands `\n` escapes. All unrecognized tags are silently dropped.

- **Renderer** (`src/renderer.rs`): Converts events to Markdown using only two features: double line breaks between paragraphs, and `> ` blockquotes for subtitles/stickers. Dialogue uses full-width colon (：U+FF1A).

- **Main** (`src/main.rs`): Reads all `.txt` files from `inputs/` sorted by filename, runs each through the pipeline, and concatenates into `outputs/story.txt`.

## Key Design Decisions

- Zero external dependencies — all parsing is hand-written.
- Tag names are lowercased at lex time so the parser never needs case-insensitive comparisons.
- Parameters are extracted on-demand via `extract_param()` rather than eagerly parsed into a map, since most tag types are ignored entirely.
- Tests are inline `#[cfg(test)]` modules in each source file (no `tests/` directory).
