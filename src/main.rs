use axum::{extract::Request, middleware, response::Response};
use lifetrace_cloud::{app, security, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::from_env();
    config.validate().map_err(|message| {
        eprintln!("[lifetrace-cloud] invalid configuration: {message}");
        message
    })?;
    security::validate_config(&config).map_err(|message| {
        eprintln!("[lifetrace-cloud] insecure production configuration: {message}");
        message
    })?;

    let state = lifetrace_cloud::AppState::new(config.clone());
    state.initialize().await?;

    let primary_listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let primary_address = primary_listener.local_addr().unwrap_or(config.bind_addr);

    let beecount_addr = std::env::var("BEECOUNT_BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:8869".to_owned())
        .parse::<std::net::SocketAddr>()?;
    let beecount_listener = tokio::net::TcpListener::bind(beecount_addr).await?;
    let beecount_address = beecount_listener.local_addr().unwrap_or(beecount_addr);

    println!(
        "[lifetrace-cloud] env={} storage=sqlite http=http://{} beecount=http://{}",
        config.environment, primary_address, beecount_address
    );

    let primary = axum::serve(
        primary_listener,
        app(state.clone()).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );
    let beecount = axum::serve(
        beecount_listener,
        app(state.clone())
            .layer(middleware::from_fn(rewrite_beecount_request))
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );
    let mail_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = lifetrace_cloud::workers::mail::run(mail_state).await {
            eprintln!("[lifetrace-cloud] mail worker stopped: {error}");
        }
    });

    tokio::spawn(async move {
        if let Err(error) = lifetrace_cloud::workers::execution::run(state).await {
            eprintln!("[lifetrace-cloud] execution worker stopped: {error}");
        }
    });

    tokio::select! {
        result = primary => result?,
        result = beecount => result?,
        _ = shutdown_signal() => {},
    }

    Ok(())
}

async fn rewrite_beecount_request(mut request: Request, next: middleware::Next) -> Response {
    let path = request.uri().path();
    let rewritten = if path == "/ready" {
        Some("/health/ready".to_owned())
    } else if path == "/ws" {
        Some("/api/v1/integrations/beecount/compat/ws".to_owned())
    } else {
        path.strip_prefix("/api/v1")
            .map(|suffix| format!("/api/v1/integrations/beecount/compat{suffix}"))
    };

    if let Some(mut target) = rewritten {
        if let Some(query) = request.uri().query() {
            target.push('?');
            target.push_str(query);
        }
        if let Ok(uri) = target.parse() {
            *request.uri_mut() = uri;
        }
    }

    next.run(request).await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    println!("[lifetrace-cloud] shutting down");
}
