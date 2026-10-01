//! HTTP server: embedded web UI, JSON/text API and health endpoint.
//!
//! Generated passwords are never logged or stored. The server does not log
//! requests at all; the only output is the startup and shutdown messages.

use crate::config::Config;
use crate::generator::{self, Generated, PassphraseOptions, PasswordOptions};
use axum::Router;
use axum::extract::Query;
use axum::extract::rejection::QueryRejection;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

const INDEX_HTML: &str = include_str!("../static/index.html");
const STYLE_CSS: &str = include_str!("../static/style.css");
const APP_JS: &str = include_str!("../static/app.js");

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
    axum::serve(listener, app())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    println!("passticulous shut down");
    Ok(())
}

pub fn app() -> Router {
    Router::new()
        .route(
            "/",
            get(|| static_asset("text/html; charset=utf-8", INDEX_HTML)),
        )
        .route(
            "/style.css",
            get(|| static_asset("text/css; charset=utf-8", STYLE_CSS)),
        )
        .route(
            "/app.js",
            get(|| static_asset("text/javascript; charset=utf-8", APP_JS)),
        )
        .route(
            "/wordlist.txt",
            get(|| static_asset("text/plain; charset=utf-8", generator::WORDLIST_RAW)),
        )
        .route("/healthz", get(healthz))
        .route("/api/generate", get(generate))
        .layer(middleware::from_fn(security_headers))
}

async fn static_asset(content_type: &'static str, body: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
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
    // Passphrase options
    words: usize,
    separator: String,
    capitalize: bool,
}

impl Default for GenerateQuery {
    fn default() -> Self {
        let pw = PasswordOptions::default();
        let pp = PassphraseOptions::default();
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
            words: pp.words,
            separator: pp.separator,
            capitalize: pp.capitalize,
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

async fn generate(query: Result<Query<GenerateQuery>, QueryRejection>) -> Response {
    let Query(q) = match query {
        Ok(q) => q,
        Err(rejection) => return error_response(rejection.body_text()),
    };
    if !(1..=MAX_COUNT).contains(&q.count) {
        return error_response(format!("count must be between 1 and {MAX_COUNT}"));
    }

    let password_opts = PasswordOptions {
        length: q.length,
        uppercase: q.uppercase,
        lowercase: q.lowercase,
        digits: q.digits,
        symbols: q.symbols,
        exclude_look_alikes: q.exclude_look_alikes,
    };
    let passphrase_opts = PassphraseOptions {
        words: q.words,
        separator: q.separator,
        capitalize: q.capitalize,
    };

    let mut rng = OsRng;
    let results: Result<Vec<Generated>, _> = (0..q.count)
        .map(|_| match q.mode {
            Mode::Password => generator::generate_password(&mut rng, &password_opts),
            Mode::Passphrase => generator::generate_passphrase(&mut rng, &passphrase_opts),
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
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get(uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
        let response = app()
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
        for path in ["/app.js", "/style.css", "/wordlist.txt"] {
            assert_eq!(get(path).await.0, StatusCode::OK, "{path}");
        }
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
        ] {
            let (status, headers, body) = get(uri).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
            assert_eq!(headers[header::CACHE_CONTROL], "no-store", "{uri}");
            assert!(json(&body)["error"].is_string(), "{uri}");
        }
    }
}
