# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/andrewsederholm/passticulous/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/andrewsederholm/passticulous/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/andrewsederholm/passticulous/releases/tag/v1.0.0
