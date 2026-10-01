mod config;
mod generator;
mod server;

use std::process::ExitCode;

fn main() -> ExitCode {
    let config = match config::Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("passticulous: configuration error: {err}");
            return ExitCode::FAILURE;
        }
    };

    match std::env::args().nth(1).as_deref() {
        None | Some("serve") => serve(config),
        Some("healthcheck") => healthcheck(&config),
        Some("--version" | "-V") => {
            println!("passticulous {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("passticulous: unknown command '{other}'");
            eprintln!("usage: passticulous [serve|healthcheck|--version]");
            ExitCode::from(2)
        }
    }
}

fn serve(config: config::Config) -> ExitCode {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to start tokio runtime");
    match runtime.block_on(server::run(config)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("passticulous: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Probes `/healthz` on the local server. Used by the Docker healthcheck so
/// the runtime image does not need curl or wget.
fn healthcheck(config: &config::Config) -> ExitCode {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = config.probe_addr();
    let timeout = Duration::from_secs(3);
    let result = (|| -> std::io::Result<String> {
        let mut stream = TcpStream::connect_timeout(&addr, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        stream.write_all(b"GET /healthz HTTP/1.0\r\nHost: localhost\r\n\r\n")?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        Ok(response)
    })();

    match result {
        Ok(response)
            if response.starts_with("HTTP/1.0 200") || response.starts_with("HTTP/1.1 200") =>
        {
            ExitCode::SUCCESS
        }
        Ok(response) => {
            eprintln!("unhealthy: {}", response.lines().next().unwrap_or(""));
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("unhealthy: {err}");
            ExitCode::FAILURE
        }
    }
}
