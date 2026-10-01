//! HTTP server: embedded web UI, JSON/text API and health endpoint.
//!
//! Generated passwords are never logged or stored. The server does not log
//! requests at all; the only output is the startup and shutdown messages.

use crate::config::{Config, Theme};
use crate::generator::{
    self, Generated, Part, PassphraseOptions, PasswordOptions, PatternOptions, WordCase,
    WordCategory,
};
use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

const INDEX_HTML: &str = include_str!("../static/index.html");
const STYLE_CSS: &str = include_str!("../static/style.css");
const APP_JS: &str = include_str!("../static/app.js");
const THEME_JS: &str = include_str!("../static/theme.js");

pub const MAX_COUNT: usize = 50;

const CONTENT_SECURITY_POLICY: &str = "default-src 'none'; script-src 'self'; \
    style-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; \
    form-action 'none'; frame-ancestors 'none'";

pub async fn run(config: Config) -> std::io::Result<()> {
    let addr = config.listen_addr();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!(
        "passticulous {} listening on http://{addr}",
        env!("CARGO_PKG_VERSION")
    );
    axum::serve(listener, app(&config.theme))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    println!("passticulous shut down");
    Ok(())
}

pub fn app(theme: &Theme) -> Router {
    let index_html = Bytes::from(render_index(theme));
    let theme_css = Bytes::from(render_theme_css(theme));
    Router::new()
        .route(
            "/",
            get(move || async move { asset("text/html; charset=utf-8", index_html) }),
        )
        .route(
            "/style.css",
            get(|| static_asset("text/css; charset=utf-8", STYLE_CSS)),
        )
        .route(
            "/theme.css",
            get(move || async move { asset("text/css; charset=utf-8", theme_css) }),
        )
        .route(
            "/app.js",
            get(|| static_asset("text/javascript; charset=utf-8", APP_JS)),
        )
        .route(
            "/theme.js",
            get(|| static_asset("text/javascript; charset=utf-8", THEME_JS)),
        )
        .route(
            "/wordlist.txt",
            get(|| static_asset("text/plain; charset=utf-8", generator::WORDLIST_RAW)),
        )
        .route("/wordlists/{file}", get(wordlist))
        .route("/healthz", get(healthz))
        .route("/api/generate", get(generate))
        .layer(middleware::from_fn(security_headers))
}

async fn static_asset(content_type: &'static str, body: &'static str) -> Response {
    asset(content_type, Bytes::from_static(body.as_bytes()))
}

fn asset(content_type: &'static str, body: Bytes) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
}

/// The page with the instance's default theme mode, e.g. `data-theme="dark"`.
fn render_index(theme: &Theme) -> String {
    INDEX_HTML.replacen(
        "<html lang=\"en\">",
        &format!("<html lang=\"en\" data-theme=\"{}\">", theme.mode.name()),
        1,
    )
}

/// The instance's custom colors as CSS variables. They are unlayered, so
/// they win over the built-in light and dark palettes in `style.css`.
fn render_theme_css(theme: &Theme) -> String {
    let mut css = String::from("/* Instance theme colors (THEME_* settings). */\n");
    if theme.colors.is_empty() {
        return css;
    }
    css.push_str(":root {\n");
    for (var, color) in &theme.colors {
        css.push_str(&format!("  --{var}: {color};\n"));
        if *var == "accent" {
            css.push_str(&format!("  --accent-text: {};\n", readable_text_on(color)));
        }
    }
    css.push_str("}\n");
    css
}

/// Black or white, whichever contrasts more with a `#rrggbb` background.
/// Mirrors `readableTextOn` in static/theme.js.
fn readable_text_on(hex: &str) -> &'static str {
    let channel = |i: usize| {
        let c = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0)) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
    // Contrast against white is 1.05 / (L + 0.05); against black (L + 0.05) / 0.05.
    if 1.05 / (luminance + 0.05) >= (luminance + 0.05) / 0.05 {
        "#ffffff"
    } else {
        "#000000"
    }
}

/// Serves a category word list, e.g. `/wordlists/animals.txt`.
async fn wordlist(Path(file): Path<String>) -> Response {
    match file.strip_suffix(".txt").and_then(WordCategory::from_name) {
        Some(category) => static_asset("text/plain; charset=utf-8", category.raw()).await,
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn healthz() -> &'static str {
    "ok"
}

async fn security_headers(request: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    for (name, value) in [
        (header::CONTENT_SECURITY_POLICY, CONTENT_SECURITY_POLICY),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (header::REFERRER_POLICY, "no-referrer"),
        (
            header::HeaderName::from_static("cross-origin-opener-policy"),
            "same-origin",
        ),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    response
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Mode {
    #[default]
    Password,
    Passphrase,
    Pattern,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Format {
    #[default]
    Json,
    Text,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct GenerateQuery {
    mode: Mode,
    count: usize,
    format: Format,
    // Password options
    length: usize,
    uppercase: bool,
    lowercase: bool,
    digits: bool,
    symbols: bool,
    exclude_look_alikes: bool,
    // Passphrase and pattern options (defaults differ by mode)
    words: Option<usize>,
    separator: Option<String>,
    // Passphrase options
    capitalize: bool,
    // Pattern options
    category: String,
    case: String,
    digit_count: usize,
    symbol_count: usize,
    symbol_set: String,
    fixed_symbols: Option<String>,
    order: String,
}

impl Default for GenerateQuery {
    fn default() -> Self {
        let pw = PasswordOptions::default();
        let pp = PassphraseOptions::default();
        let pat = PatternOptions::default();
        Self {
            mode: Mode::default(),
            count: 1,
            format: Format::default(),
            length: pw.length,
            uppercase: pw.uppercase,
            lowercase: pw.lowercase,
            digits: pw.digits,
            symbols: pw.symbols,
            exclude_look_alikes: pw.exclude_look_alikes,
            words: None,
            separator: None,
            capitalize: pp.capitalize,
            category: pat.category.name().to_string(),
            case: "title".to_string(),
            digit_count: pat.digits,
            symbol_count: pat.symbols,
            symbol_set: pat.symbol_set,
            fixed_symbols: pat.fixed_symbols,
            order: "word,digits,symbols".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct GenerateResponse {
    mode: Mode,
    entropy_bits: f64,
    passwords: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn error_response(message: String) -> Response {
    no_store((
        StatusCode::BAD_REQUEST,
        axum::Json(ErrorResponse { error: message }),
    ))
}

/// Marks a response as uncacheable so passwords never land in a cache.
fn no_store(response: impl IntoResponse) -> Response {
    let mut response = response.into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

enum Options {
    Password(PasswordOptions),
    Passphrase(PassphraseOptions),
    Pattern(PatternOptions),
}

impl Options {
    fn from_query(mode: Mode, q: &GenerateQuery) -> Result<Self, String> {
        Ok(match mode {
            Mode::Password => Self::Password(PasswordOptions {
                length: q.length,
                uppercase: q.uppercase,
                lowercase: q.lowercase,
                digits: q.digits,
                symbols: q.symbols,
                exclude_look_alikes: q.exclude_look_alikes,
            }),
            Mode::Passphrase => {
                let defaults = PassphraseOptions::default();
                Self::Passphrase(PassphraseOptions {
                    words: q.words.unwrap_or(defaults.words),
                    separator: q.separator.clone().unwrap_or(defaults.separator),
                    capitalize: q.capitalize,
                })
            }
            Mode::Pattern => {
                let defaults = PatternOptions::default();
                let category = WordCategory::from_name(&q.category).ok_or_else(|| {
                    let names: Vec<&str> = WordCategory::ALL.iter().map(|c| c.name()).collect();
                    format!("category must be one of: {}", names.join(", "))
                })?;
                let case = WordCase::from_name(&q.case)
                    .ok_or("case must be one of: title, lower, upper")?;
                let order = Part::parse_order(&q.order).ok_or(
                    "order must list word, digits and symbols once each, e.g. word,digits,symbols",
                )?;
                Self::Pattern(PatternOptions {
                    category,
                    words: q.words.unwrap_or(defaults.words),
                    case,
                    digits: q.digit_count,
                    symbols: q.symbol_count,
                    symbol_set: q.symbol_set.clone(),
                    fixed_symbols: q.fixed_symbols.clone(),
                    order,
                    separator: q.separator.clone().unwrap_or(defaults.separator),
                })
            }
        })
    }
}

async fn generate(query: Result<Query<GenerateQuery>, QueryRejection>) -> Response {
    let Query(q) = match query {
        Ok(q) => q,
        Err(rejection) => return error_response(rejection.body_text()),
    };
    if !(1..=MAX_COUNT).contains(&q.count) {
        return error_response(format!("count must be between 1 and {MAX_COUNT}"));
    }

    let options = match Options::from_query(q.mode, &q) {
        Ok(options) => options,
        Err(message) => return error_response(message),
    };

    let mut rng = OsRng;
    let results: Result<Vec<Generated>, _> = (0..q.count)
        .map(|_| match &options {
            Options::Password(o) => generator::generate_password(&mut rng, o),
            Options::Passphrase(o) => generator::generate_passphrase(&mut rng, o),
            Options::Pattern(o) => generator::generate_pattern(&mut rng, o),
        })
        .collect();
    let results = match results {
        Ok(results) => results,
        Err(err) => return error_response(err.to_string()),
    };

    let entropy_bits = results.first().map_or(0.0, |g| g.entropy_bits);
    let passwords: Vec<String> = results.into_iter().map(|g| g.value).collect();

    match q.format {
        Format::Json => no_store(axum::Json(GenerateResponse {
            mode: q.mode,
            entropy_bits: (entropy_bits * 10.0).round() / 10.0,
            passwords,
        })),
        Format::Text => {
            let mut body = passwords.join("\n");
            body.push('\n');
            no_store(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body))
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThemeMode;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get(uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
        get_with(&Theme::default(), uri).await
    }

    async fn get_with(theme: &Theme, uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
        let response = app(theme)
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, headers, String::from_utf8(body.to_vec()).unwrap())
    }

    fn json(body: &str) -> serde_json::Value {
        serde_json::from_str(body).unwrap()
    }

    #[tokio::test]
    async fn healthz_is_ok() {
        let (status, _, body) = get("/healthz").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn serves_ui_with_security_headers() {
        let (status, headers, body) = get("/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("Passticulous"));
        assert_eq!(
            headers[header::CONTENT_SECURITY_POLICY],
            CONTENT_SECURITY_POLICY
        );
        assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        assert!(body.contains("<html lang=\"en\" data-theme=\"system\">"));
        for path in [
            "/app.js",
            "/theme.js",
            "/style.css",
            "/theme.css",
            "/wordlist.txt",
            "/wordlists/animals.txt",
            "/wordlists/any.txt",
            "/wordlists/elements.txt",
        ] {
            assert_eq!(get(path).await.0, StatusCode::OK, "{path}");
        }
        assert_eq!(get("/wordlists/nope.txt").await.0, StatusCode::NOT_FOUND);
        assert_eq!(get("/wordlists/animals").await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn serves_instance_theme() {
        let (_, _, css) = get("/theme.css").await;
        assert!(!css.contains(":root"), "{css}");

        let theme = Theme {
            mode: ThemeMode::Dark,
            colors: vec![
                ("bg", "#101010".to_string()),
                ("accent", "#ffd43b".to_string()),
            ],
        };
        let (_, _, html) = get_with(&theme, "/").await;
        assert!(html.contains("<html lang=\"en\" data-theme=\"dark\">"));
        let (status, headers, css) = get_with(&theme, "/theme.css").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            headers[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/css")
        );
        assert!(css.contains("--bg: #101010;"), "{css}");
        assert!(css.contains("--accent: #ffd43b;"), "{css}");
        // Yellow is light, so text on accent buttons should be black.
        assert!(css.contains("--accent-text: #000000;"), "{css}");
    }

    #[test]
    fn readable_text_on_picks_the_higher_contrast() {
        assert_eq!(readable_text_on("#4f46e5"), "#ffffff");
        assert_eq!(readable_text_on("#000000"), "#ffffff");
        assert_eq!(readable_text_on("#ffffff"), "#000000");
        assert_eq!(readable_text_on("#ffd43b"), "#000000");
    }

    #[tokio::test]
    async fn generate_defaults_to_one_password() {
        let (status, headers, body) = get("/api/generate").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers[header::CACHE_CONTROL], "no-store");
        let v = json(&body);
        assert_eq!(v["mode"], "password");
        let passwords = v["passwords"].as_array().unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].as_str().unwrap().len(), 20);
        assert!(v["entropy_bits"].as_f64().unwrap() > 100.0);
    }

    #[tokio::test]
    async fn generate_respects_password_options() {
        let (status, _, body) =
            get("/api/generate?length=32&count=5&symbols=false&uppercase=false&exclude_look_alikes=true")
                .await;
        assert_eq!(status, StatusCode::OK);
        let v = json(&body);
        let passwords = v["passwords"].as_array().unwrap();
        assert_eq!(passwords.len(), 5);
        for pw in passwords {
            let pw = pw.as_str().unwrap();
            assert_eq!(pw.len(), 32);
            assert!(
                pw.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "{pw}"
            );
            assert!(
                !pw.chars().any(|c| generator::LOOK_ALIKES.contains(c)),
                "{pw}"
            );
        }
    }

    #[tokio::test]
    async fn generate_passphrase_as_text() {
        let (status, headers, body) =
            get("/api/generate?mode=passphrase&words=4&separator=_&format=text&count=3").await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            headers[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/plain")
        );
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 3);
        for line in lines {
            assert_eq!(line.split('_').count(), 4, "{line}");
        }
    }

    #[tokio::test]
    async fn generate_rejects_bad_input() {
        for uri in [
            "/api/generate?length=2",
            "/api/generate?length=abc",
            "/api/generate?count=0",
            "/api/generate?count=51",
            "/api/generate?mode=pin",
            "/api/generate?uppercase=false&lowercase=false&digits=false&symbols=false",
            "/api/generate?mode=passphrase&words=100",
            "/api/generate?mode=pattern&category=dinosaurs",
            "/api/generate?mode=pattern&case=sideways",
            "/api/generate?mode=pattern&order=word,word,digits",
            "/api/generate?mode=pattern&symbol_set=",
            "/api/generate?mode=pattern&symbol_set=%23a",
            "/api/generate?mode=pattern&digit_count=17",
            "/api/generate?mode=pattern&fixed_symbols=",
            "/api/generate?mode=pattern&fixed_symbols=%23a",
            "/api/generate?mode=pattern&words=6",
        ] {
            let (status, headers, body) = get(uri).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
            assert_eq!(headers[header::CACHE_CONTROL], "no-store", "{uri}");
            assert!(json(&body)["error"].is_string(), "{uri}");
        }
    }

    #[tokio::test]
    async fn generate_pattern_defaults() {
        let (status, _, body) = get("/api/generate?mode=pattern&count=20").await;
        assert_eq!(status, StatusCode::OK);
        let v = json(&body);
        assert_eq!(v["mode"], "pattern");
        for pw in v["passwords"].as_array().unwrap() {
            let pw = pw.as_str().unwrap();
            let word: String = pw.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            assert!(
                WordCategory::Animals
                    .words()
                    .contains(&word.to_lowercase().as_str()),
                "{pw}"
            );
            let rest = &pw[word.len()..];
            assert!(rest[..4].chars().all(|c| c.is_ascii_digit()), "{pw}");
            assert!(
                rest[4..]
                    .chars()
                    .all(|c| generator::DEFAULT_PATTERN_SYMBOLS.contains(c)),
                "{pw}"
            );
        }
    }

    #[tokio::test]
    async fn generate_pattern_with_options() {
        // symbol_set "#@!" is URL-encoded as %23%40%21.
        let (status, _, body) = get(
            "/api/generate?mode=pattern&category=space&words=2&case=upper\
             &digit_count=2&symbol_count=3&symbol_set=%23%40%21\
             &order=symbols,digits,word&separator=_&format=text",
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let segments: Vec<&str> = body.trim_end().split('_').collect();
        assert_eq!(segments.len(), 4, "{body}");
        assert!(segments[0].len() == 3 && segments[0].chars().all(|c| "#@!".contains(c)));
        assert!(segments[1].len() == 2 && segments[1].chars().all(|c| c.is_ascii_digit()));
        for word in &segments[2..] {
            assert!(
                WordCategory::Space
                    .words()
                    .contains(&word.to_lowercase().as_str()),
                "{body}"
            );
            assert_eq!(*word, word.to_uppercase());
        }
    }

    #[tokio::test]
    async fn generate_pattern_with_fixed_symbols() {
        // fixed_symbols "#@!" is URL-encoded as %23%40%21.
        let (status, _, body) =
            get("/api/generate?mode=pattern&count=10&fixed_symbols=%23%40%21").await;
        assert_eq!(status, StatusCode::OK);
        let v = json(&body);
        for pw in v["passwords"].as_array().unwrap() {
            let pw = pw.as_str().unwrap();
            assert!(pw.ends_with("#@!"), "{pw}");
            let digits = &pw[pw.len() - 7..pw.len() - 3];
            assert!(digits.chars().all(|c| c.is_ascii_digit()), "{pw}");
        }
    }
}
