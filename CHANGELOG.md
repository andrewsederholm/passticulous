# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.10.0] - 2026-10-05

### Added

- **Exact word length** for patterns: tick "Exactly this length" under the
  word length slider to only use words of exactly that many letters. Some
  lengths have no words in some lists (Space has none of 11 or 12 letters),
  which shows a clear message. Also available in the API as `word_length`,
  which overrides `min_word_length`.

### Changed

- The pattern "Minimum word length" slider is now called "Word length", since
  it also sets the exact length.

## [1.9.0] - 2026-10-05

### Added

- **Minimum word length** for patterns: only use words with at least this
  many letters. The slider runs from the chosen word type's shortest word to
  its longest, and the page shows how many words are long enough. Fewer words
  means a weaker password, and the strength meter accounts for it. Also
  available in the API as `min_word_length`.

## [1.8.0] - 2026-10-05

### Added

- **Simple** mode for temporary passwords that are easy to read out to a new
  hire, like `Giraffe4821!`: an easy title-case word, numbers, then one symbol
  (`! @ # $ ?`), at least 12 characters by default (10–16). Use a random word
  or your own, like `Welcome` or your organization's name. Also available in
  the API as `mode=simple` with `min_length` and `word`.

## [1.7.0] - 2026-10-05

### Added

- The running version is shown at the bottom of the page, linking to its
  release notes on GitHub.

## [1.6.0] - 2026-10-05

### Changed

- New main wordlist for passphrases and the pattern "Any word" type: 14,014
  common English words of 6–10 letters (~13.8 bits per word), replacing the EFF
  large wordlist (7,776 words, ~12.9 bits). Passphrases with the same number of
  words are slightly stronger, and also longer. Offensive and sensitive words
  have been removed.

## [1.5.0] - 2026-10-01

### Added

- Theme presets under **Appearance**: Ocean, Forest, Fall, Rose and Sand
  (light), and Midnight, Arctic, Grape and Graphite (dark). One click sets the
  mode and all six colors, and you can still adjust any color afterwards.

## [1.4.0] - 2026-10-01

### Added

- "Periodic elements" word type for patterns, with all 118 element names
  (e.g. `Rhodium8754#@!`). Use `category=elements` in the API.

## [1.3.0] - 2026-10-01

### Added

- Themes. Set the default mode (`THEME=system|light|dark`) and any of the
  colors (`THEME_BG`, `THEME_SURFACE`, `THEME_TEXT`, `THEME_MUTED`,
  `THEME_BORDER`, `THEME_ACCENT`) for your instance. Visitors can override the
  mode and colors under **Appearance**; their choices are saved in their
  browser only.

## [1.2.0] - 2026-10-01

### Added

- Pattern mode can use the same symbols every time, in the order you type them
  (for example `#@!`), instead of random ones. Turn on "Use the same symbols
  every time" in the UI, or pass `fixed_symbols` to `/api/generate`.

## [1.1.0] - 2026-10-01

### Added

- Pattern mode for memorable passwords like `Giraffe3287#@!`: choose a word
  category (animals, colors, foods, nature, space or any word), number of
  words, word case, how many numbers and symbols, exactly which symbols are
  allowed, the order of the parts, and a separator. Available in the UI and
  via `/api/generate?mode=pattern`.
- Themed word lists, served at `/wordlists/{category}.txt`.
- The selected mode is kept in the URL (`/#pattern`) so it can be bookmarked.

## [1.0.0] - 2026-10-01

First release.

### Added

- Password generation with configurable length (4–256), uppercase, lowercase,
  digits, symbols, and an option to exclude look-alike characters. Every
  enabled character type is guaranteed to appear.
- Passphrase mode using the EFF large wordlist (3–20 words), with a custom
  separator and optional capitalization.
- Single-page web UI that generates in the browser with
  `crypto.getRandomValues`, with a strength meter and copy button.
- `GET /api/generate` for scripts (JSON or plain text, up to 50 at a time),
  using the OS CSPRNG with rejection sampling.
- `GET /healthz` endpoint and a `passticulous healthcheck` subcommand.
- Configuration via the `PORT` and `BIND_ADDR` environment variables.
- Multi-stage Dockerfile (distroless, non-root) and a `docker-compose.yml`
  with a configurable port, restart policy and healthcheck.

[Unreleased]: https://github.com/andrewsederholm/passticulous/compare/v1.10.0...HEAD
[1.10.0]: https://github.com/andrewsederholm/passticulous/compare/v1.9.0...v1.10.0
[1.9.0]: https://github.com/andrewsederholm/passticulous/compare/v1.8.0...v1.9.0
[1.8.0]: https://github.com/andrewsederholm/passticulous/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/andrewsederholm/passticulous/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/andrewsederholm/passticulous/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/andrewsederholm/passticulous/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/andrewsederholm/passticulous/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/andrewsederholm/passticulous/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/andrewsederholm/passticulous/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/andrewsederholm/passticulous/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/andrewsederholm/passticulous/releases/tag/v1.0.0
