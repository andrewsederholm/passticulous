# Passticulous

A self-hostable password and passphrase generator. One small, self-contained
Rust binary serves a web UI that generates everything **in your browser**, plus
an optional HTTP API for scripts and `curl`.

## Features

- **Passwords**: length 4–256, uppercase, lowercase, digits, symbols, and an
  option to exclude look-alike characters (`I l 1 | O 0 o`). Every enabled
  character type is guaranteed to appear at least once.
- **Passphrases**: 3–20 words from the [EFF large wordlist](https://www.eff.org/dice)
  (7,776 words, ~12.9 bits each), with a custom separator and optional
  capitalization.
- **Client-side generation**: the UI uses `crypto.getRandomValues`, so
  generated values never leave your browser.
- **Server API**: `GET /api/generate` for scripts, using the OS CSPRNG.
- **Strength meter**: an entropy estimate for the current settings.
- **One binary**: HTML, CSS, JS and the wordlist are embedded at compile time.
  No database, no state, no runtime files.
- **Container-ready**: multi-stage Dockerfile, distroless non-root runtime
  image, Compose file with a healthcheck.

## Quick start

```sh
git clone https://github.com/andrewsederholm/passticulous.git
cd passticulous
docker compose up -d
```

Then open <http://localhost:8080>.

To use a different host port:

```sh
PASSTICULOUS_PORT=9000 docker compose up -d
```

### Without Docker

Requires a recent stable Rust toolchain.

```sh
cargo run --release
# or
cargo build --release && ./target/release/passticulous
```

## Configuration

The server reads these environment variables:

| Variable    | Default   | Description                                         |
|-------------|-----------|-----------------------------------------------------|
| `PORT`      | `8080`    | TCP port to listen on.                              |
| `BIND_ADDR` | `0.0.0.0` | IP address to bind, e.g. `127.0.0.1` or `::`.       |

With Docker Compose, set `PASSTICULOUS_PORT` to change the **host** port. The
container always listens on 8080 internally. To expose the app only on
localhost (for example, behind a reverse proxy), change the port mapping in
`docker-compose.yml` to `"127.0.0.1:${PASSTICULOUS_PORT:-8080}:8080"`.

## API

`GET /api/generate` accepts these query parameters, all optional:

| Parameter             | Default    | Applies to | Notes                          |
|-----------------------|------------|------------|--------------------------------|
| `mode`                | `password` | –          | `password` or `passphrase`     |
| `count`               | `1`        | –          | 1–50                           |
| `format`              | `json`     | –          | `json` or `text` (one per line)|
| `length`              | `20`       | password   | 4–256                          |
| `uppercase`           | `true`     | password   |                                |
| `lowercase`           | `true`     | password   |                                |
| `digits`              | `true`     | password   |                                |
| `symbols`             | `true`     | password   |                                |
| `exclude_look_alikes` | `false`    | password   | removes `I l 1 \| O 0 o`       |
| `words`               | `6`        | passphrase | 3–20                           |
| `separator`           | `-`        | passphrase | up to 8 characters             |
| `capitalize`          | `false`    | passphrase |                                |

```sh
$ curl -s 'http://localhost:8080/api/generate?length=24&count=2'
{"mode":"password","entropy_bits":155.8,"passwords":["…","…"]}

$ curl -s 'http://localhost:8080/api/generate?mode=passphrase&words=5&format=text'
gravity-unsalted-overdue-shrank-comply
```

Invalid parameters return `400` with `{"error": "..."}`.

`GET /healthz` returns `200 ok`. The binary also has a `passticulous healthcheck`
subcommand that probes it, which the Docker healthcheck uses because the
runtime image has no shell or curl.

## Security notes

- **Randomness.** The server uses the operating system CSPRNG (`OsRng`). The
  browser uses `crypto.getRandomValues`. Both map random numbers to characters
  or words with **rejection sampling**, so every choice is exactly equally
  likely (no modulo bias). Passwords that miss a required character type are
  discarded and redrawn whole, which keeps the result uniform over all valid
  passwords.
- **Nothing is logged or stored.** The server keeps no state, writes nothing to
  disk, and does not log requests. It prints only startup and shutdown lines.
  The UI saves your *options* (not generated values) in `localStorage`.
- **Prefer the web UI.** It generates locally and never sends a password over
  the network. The API sends the password in the response. Over plain HTTP that
  response can be read on the network, so put the API behind TLS (e.g. a
  reverse proxy) if you use it anywhere but localhost.
- **No caching.** API responses send `Cache-Control: no-store`.
- **Hardened headers.** A strict Content-Security-Policy (no inline or
  third-party scripts), `X-Frame-Options: DENY`, `nosniff` and
  `Referrer-Policy: no-referrer`.
- **Clipboard.** Copied passwords stay on your clipboard until you replace
  them. Browsers only allow the Clipboard API on HTTPS or `localhost`; on plain
  HTTP the Copy button selects the text so you can copy it manually.
- **Container.** Runs as a non-root user on a distroless image, with a
  read-only root filesystem, all capabilities dropped and `no-new-privileges`.
- **Entropy figures** assume the attacker knows your settings. They are
  `length × log2(pool size)` for passwords and `words × log2(7776)` for
  passphrases. The "every type present" rule lowers this slightly, mostly at
  very short lengths.

## Development

```sh
cargo test          # unit tests for the generator, config and HTTP routes
cargo clippy --all-targets
cargo fmt
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

The bundled wordlist (`assets/wordlist.txt`) is the
[EFF Large Wordlist](https://www.eff.org/deeplinks/2016/07/new-wordlists-random-passphrases)
by the Electronic Frontier Foundation, licensed under
[CC BY 3.0 US](https://creativecommons.org/licenses/by/3.0/us/).
