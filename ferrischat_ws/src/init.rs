#![allow(clippy::module_name_repetitions)]
use crate::handle_connection::handle_ws_connection;
use crate::redis_handler::redis_event_handler;
use crate::{SUB_TO_ME, USERID_CONNECTION_MAP};
use dashmap::DashMap;
use ferrischat_redis::get_pubsub;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::oneshot::channel;
use tokio_rustls::server::TlsStream;

/// Initialize the `WebSocket` by starting all services it depends on.
///
/// # Panics
/// This function panics if it is called more than once.
pub async fn init_ws() {
    // plop the DashMap into the UserId connection map first thing
    USERID_CONNECTION_MAP
        .set(DashMap::new())
        .unwrap_or_else(|_| panic!("don't call `preload_ws()` more than once"));

    // allow up to 250 new subscriptions to be processed
    let (tx, rx) = tokio::sync::mpsc::channel(250);

    SUB_TO_ME
        .set(tx)
        .expect("don't call `preload_ws()` more than once");

    tokio::spawn(redis_event_handler(
        get_pubsub()
            .await
            .expect("failed to open pubsub connection"),
        rx,
    ));
}

#[allow(clippy::missing_panics_doc)]
/// Initialize the `WebSocket` server.
/// `init_ws` MUST be called before this, otherwise panics may occur due to missing dependencies.
pub async fn init_ws_server<T: tokio::net::ToSocketAddrs + Send>(addr: T) {
    enum DieOrResult<T> {
        Die,
        Result(tokio::io::Result<T>),
    }

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind to address");

    let (end_tx, mut end_rx) = channel();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl+c");
        end_tx
            .send(())
            .expect("failed to send message to listeners");
    });

    let cfg = ferrischat_config::GLOBAL_CONFIG
        .get()
        .expect("config not loaded! call load_config before websocket setup");
    let ferrischat_config::TlsConfig {
        private_key_file,
        certificate_file,
    } = &cfg.tls;

    let certs = tokio_rustls::rustls::internal::pemfile::certs(&mut std::io::BufReader::new(
        std::fs::File::open(certificate_file).expect("failed to open cert file"),
    ))
    .expect("failed to parse cert file");
    let privkeys =
        tokio_rustls::rustls::internal::pemfile::pkcs8_private_keys(&mut std::io::BufReader::new(
            std::fs::File::open(private_key_file).expect("failed to open privkey file"),
        ))
        .expect("failed to parse privkey file");
    let mut tls_config =
        tokio_rustls::rustls::ServerConfig::new(tokio_rustls::rustls::NoClientAuth::new());
    tls_config
        .set_single_cert(certs, privkeys.get(0).expect("no privkeys found").clone())
        .expect("privkey invalid");

    let tls_acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls_config));
    tokio::spawn(async move {
        loop {
            let res = tokio::select! {
                stream_addr = listener.accept() => {DieOrResult::Result(stream_addr)}
                _ = &mut end_rx => {DieOrResult::Die}
            };

            match res {
                DieOrResult::Die => break,
                DieOrResult::Result(r) => match r {
                    Ok((stream, addr)) => {
                        let tls_stream: TlsStream<TcpStream> =
                            match tls_acceptor.accept(stream).await {
                                Ok(s) => s,
                                Err(_) => continue,
                            };
                        tokio::spawn(handle_ws_connection(tls_stream, addr));
                    }
                    Err(e) => eprintln!("failed to accept WS conn: {}", e),
                },
            }
        }
    });
}
