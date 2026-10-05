# Passticulous

(Password + Meticulous = Passticulous)

>[!NOTE]
>Hey there! First off: **I am NOT a developer.** I have zero coding background or formal dev experience, my actual day job is in IT and hobby is automotive. This entire project was built using AI (specifically Claude in the terminal). 
>
>While I make enough to live comfortably, paying for AI API tokens for this project isn't a top financial priority. Because token costs add up fast, please keep in mind that updates might be slow. Development moves at the pace of what I can reasonably spend.
>
>To put things in perspective: $20 worth of tokens got the project to where it is today (which is honestly way further than I expected!), but that $20 probably should have gone into my gas tank instead...
>
>### Supporting Development
>**Please know that you DO NOT need to donate! It is 100% your choice, and I genuinely appreciate you just using or checking out the project regardless.**
>
>If you do feel inclined to chip in, any support is hugely appreciated. 100% of any donations received will go directly toward purchasing tokens in the Claude console to keep development moving.
>
>✨ **[Buy Me A ~~Coffee~~ Token](https://www.buymeacoffee.com/andrewsederholm)**

> [!WARNING]
> This software was built using AI without manual security verification. It is provided "as is", without warranty of any kind. You host this project entirely at your own risk, and the author is not responsible for any security incidents, losses, or system compromises.

## What is Passticulous?

Passticulous (pronounced PASS-TICK-YOU-LIS) is a password generator for when you need to get a little too particular with your passwords. Customize how they're generated, tweak the options however you want, and be as "passticulous" as you'd like. It's self-hosted: one small Rust binary serves a web UI that generates everything in your browser, plus an optional HTTP API for scripts and `curl`.

## Features

- **Passwords**: length 4–256, uppercase, lowercase, digits, symbols, and an
  option to exclude look-alike characters (`I l 1 | O 0 o`). Every enabled
  character type is guaranteed to appear at least once.
- **Passphrases**: 3–20 words from the [EFF large wordlist](https://www.eff.org/dice)
  (7,776 words, ~12.9 bits each), with a custom separator and optional
  capitalization.
- **Patterns**: memorable passwords like `Giraffe3287#@!`. Pick a word type
  (animals, colors, foods, nature, space, periodic elements, or any EFF word),
  1–5 words and their case (Title, lower, UPPER), how many numbers (0–16) and
  symbols (0–16), exactly which symbols are allowed (or the same fixed symbols,
  like `#@!`, every time), the order of the three parts, and an optional
  separator.
- **Themes**: light, dark or follow the device, with every color adjustable
  and nine one-click presets. Set the default for your instance, and visitors
  can tweak it for themselves.
- **Client-side generation**: the UI uses `crypto.getRandomValues`, so
  generated values never leave your browser.
- **Server API**: `GET /api/generate` for scripts, using the OS CSPRNG.
- **Bookmarkable modes**: open `/#password`, `/#passphrase` or `/#pattern`
  directly.
- **Strength meter**: an entropy estimate for the current settings.
- **One binary**: HTML, CSS, JS and the wordlist are embedded at compile time.
  No database, no state, no runtime files.
- **Container-ready**: multi-stage Dockerfile, distroless non-root runtime
  image, Compose file with a healthcheck.

## Screenshots
<p align="center">
  <img src="https://github.com/user-attachments/assets/83447241-2f8c-4405-83c5-4735591fb0eb" width="30%" alt="image" />
  <img src="https://github.com/user-attachments/assets/68418ee1-81f9-4c9d-b47a-9734f1110a8a" width="30%" alt="image" />
  <img src="https://github.com/user-attachments/assets/9a7324c7-7b70-46b0-936d-5cc43ca83be5" width="30%" alt="image" />
</p>






## Quick start

```sh
git clone https://github.com/andrewsederholm/passticulous.git
cd passticulous
docker compose up -d
```

Then open <http://localhost:8080>.

To use a different host port, set it in a `.env` file so it survives
upgrades:

```sh
echo "PASSTICULOUS_PORT=9000" > .env
docker compose up -d
```

To update later, see [Upgrading](#upgrading).

### Without Docker

Requires a recent stable Rust toolchain.

```sh
cargo run --release
# or
cargo build --release && ./target/release/passticulous
```

## Upgrading

Passticulous stores nothing on the server, so there is no data to back up or
migrate. Your UI settings live in your browser and carry over automatically.

Before upgrading, skim the [release notes](https://github.com/andrewsederholm/passticulous/releases)
or [CHANGELOG.md](CHANGELOG.md). A new **major** version (e.g. 1.x → 2.0.0)
may need changes to your setup, and its notes will say what to do.

### Docker (latest version)

From the folder you cloned:

```sh
git pull
docker compose up -d --build
```

`--build` matters: without it, Compose keeps running the old image. The
container is replaced with the new version in a few seconds.

Check that it worked:

```sh
docker compose ps        # STATUS should show "(healthy)" after a few seconds
docker compose exec passticulous /usr/local/bin/passticulous --version
```

Old images are kept by Docker after each rebuild. To free the space:

```sh
docker image prune
```

### Docker (a specific version)

To install or stay on a particular release instead of the latest:

```sh
git fetch --tags
git checkout v1.5.0
docker compose up -d --build
```

Git will say you are in a "detached HEAD" state. That is normal when checking
out a release tag. To go back to following the latest version:

```sh
git checkout main
git pull
docker compose up -d --build
```

### Rolling back

If a new version causes a problem, check out the previous release and rebuild:

```sh
git checkout v1.4.0
docker compose up -d --build
```

### Keep local settings out of the way of updates

If you change the port, put it in a `.env` file next to `docker-compose.yml`
rather than editing the compose file:

```sh
echo "PASSTICULOUS_PORT=9000" > .env
```

Compose reads `.env` automatically and Git ignores it, so `git pull` never
conflicts with your settings. If you have already edited `docker-compose.yml`
and `git pull` refuses to run, save your changes with `git stash`, pull, then
`git stash pop` to reapply them.

### Without Docker

```sh
git pull
cargo build --release
```

Then restart `./target/release/passticulous`.

### Getting notified of new releases

On the GitHub repository page, click **Watch → Custom → Releases** to get an
email when a new version is published.

## Configuration

The server reads these environment variables:

| Variable    | Default   | Description                                         |
|-------------|-----------|-----------------------------------------------------|
| `PORT`      | `8080`    | TCP port to listen on.                              |
| `BIND_ADDR` | `0.0.0.0` | IP address to bind, e.g. `127.0.0.1` or `::`.       |
| `THEME`     | `system`  | Default mode for visitors: `system` (follow their device), `light` or `dark`. |
| `THEME_BG`, `THEME_SURFACE`, `THEME_TEXT`, `THEME_MUTED`, `THEME_BORDER`, `THEME_ACCENT` | built-in | Default colors as hex, e.g. `#2f9e44`: page background, panels, text, secondary text, borders, and the accent (buttons, tabs, sliders). Used in both light and dark mode. |

With Docker Compose, set `PASSTICULOUS_PORT` in a `.env` file to change the
**host** port. The container always listens on 8080 internally. To expose the
app only on localhost (for example, behind a reverse proxy), change the port
mapping in `docker-compose.yml` to `"127.0.0.1:${PASSTICULOUS_PORT:-8080}:8080"`.
This is a local edit to a tracked file, so see
[Upgrading](#keep-local-settings-out-of-the-way-of-updates) for how to pull
updates afterwards.

### Theme

The `THEME` settings set the default look for everyone who visits. With Docker
Compose, add them to `.env` and run `docker compose up -d` again:

```sh
THEME=dark
THEME_ACCENT=#2f9e44
```

Any color you leave out keeps the built-in light or dark value. Text on accent
buttons switches between black and white automatically to stay readable.

Visitors can override the mode and every color under **Appearance** at the
bottom of the page, or pick one of the ready-made presets (Ocean, Forest,
Fall, Rose, Sand, Midnight, Arctic, Grape or Graphite) and adjust from there. Their choices are saved only in their browser, and
**Reset to default** goes back to your settings.

## API

`GET /api/generate` accepts these query parameters, all optional:

| Parameter             | Default    | Applies to | Notes                          |
|-----------------------|------------|------------|--------------------------------|
| `mode`                | `password` | –          | `password`, `passphrase` or `pattern` |
| `count`               | `1`        | –          | 1–50                           |
| `format`              | `json`     | –          | `json` or `text` (one per line)|
| `length`              | `20`       | password   | 4–256                          |
| `uppercase`           | `true`     | password   |                                |
| `lowercase`           | `true`     | password   |                                |
| `digits`              | `true`     | password   |                                |
| `symbols`             | `true`     | password   |                                |
| `exclude_look_alikes` | `false`    | password   | removes `I l 1 \| O 0 o`       |
| `words`               | `6` / `1`  | passphrase / pattern | 3–20 / 1–5           |
| `separator`           | `-` / none | passphrase / pattern | up to 8 characters   |
| `capitalize`          | `false`    | passphrase |                                |
| `category`            | `animals`  | pattern    | `animals`, `colors`, `foods`, `nature`, `space`, `elements`, `any` |
| `case`                | `title`    | pattern    | `title`, `lower` or `upper`    |
| `digit_count`         | `4`        | pattern    | 0–16                           |
| `symbol_count`        | `3`        | pattern    | 0–16                           |
| `symbol_set`          | `!@#$%&*?` | pattern    | any ASCII symbols; URL-encode it (`#` is `%23`) |
| `fixed_symbols`       | none       | pattern    | 1–16 ASCII symbols used as-is every time; overrides `symbol_count` and `symbol_set` |
| `order`               | `word,digits,symbols` | pattern | each part exactly once |

```sh
$ curl -s 'http://localhost:8080/api/generate?length=24&count=2'
{"mode":"password","entropy_bits":155.8,"passwords":["…","…"]}

$ curl -s 'http://localhost:8080/api/generate?mode=passphrase&words=5&format=text'
gravity-unsalted-overdue-shrank-comply

$ curl -s 'http://localhost:8080/api/generate?mode=pattern&category=animals&digit_count=4&symbol_set=%23%40%21&format=text'
Giraffe3287#@!

$ curl -s 'http://localhost:8080/api/generate?mode=pattern&fixed_symbols=%23%40%21&format=text'
Otter5102#@!
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
  passphrases. Patterns add `log2` of the word list size per word, ~3.3 bits
  per number and `log2(allowed symbols)` per symbol (fixed symbols add
  nothing). A single themed word gives
  only ~7–8 bits, so a default pattern is around 30 bits ("Weak"). Patterns are
  meant to be easy to remember. Use more words, numbers or symbols, or the
  Password mode, where strength matters. The "every type present" rule lowers this slightly, mostly at
  very short lengths.

## Development

```sh
cargo test          # unit tests for the generator, config and HTTP routes
cargo clippy --all-targets
cargo fmt
```

## Versioning

Passticulous follows [Semantic Versioning](https://semver.org/). Releases are
tagged `vX.Y.Z` and every change is recorded in [CHANGELOG.md](CHANGELOG.md).
Run `passticulous --version` to see which version you have. See
[Upgrading](#upgrading) for how to update or pin a version.

- **Patch** (1.1.0 → 1.1.1): bug fixes only.
- **Minor** (1.1.0 → 1.2.0): new features; existing behavior is unchanged.
- **Major** (1.x → 2.0.0): breaking changes; read the release notes first.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

The bundled wordlist (`assets/wordlist.txt`) is the
[EFF Large Wordlist](https://www.eff.org/deeplinks/2016/07/new-wordlists-random-passphrases)
by the Electronic Frontier Foundation, licensed under
[CC BY 3.0 US](https://creativecommons.org/licenses/by/3.0/us/).
