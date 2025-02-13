//! This module contains helpful utility functions for dealing with HTTP(s) requests and responses.

use anyhow::{bail, Context, Result};
use http_body_util::Full;
use hyper::{body::Bytes, body::Incoming, Request, Response, Uri};

use crate::secmod::Secmod;

// Read at most `max_bytes` from body. Error if more bytes are sent.
pub async fn get_body(mut body: Incoming, max_bytes: usize) -> Result<Vec<u8>> {
    use http_body_util::BodyExt;
    let mut result = Vec::with_capacity(max_bytes);
    let mut pos = 0;

    while let Some(frame) = body.frame().await {
        let frame = frame?;
        if let Some(data) = frame.data_ref() {
            let remaining = max_bytes - pos;
            let ln = data.len();
            if ln > remaining {
                bail!("too many bytes sent in body");
            }
            result.extend_from_slice(data);
            pos += ln;
            assert!(pos <= max_bytes);
        }
    }
    Ok(result)
}

pub fn get_query_param<'a>(query: Option<&'a str>, param: &str) -> Option<&'a str> {
    let query = query?;
    query
        .split('&')
        .find(|p| p.starts_with(param) && p.as_bytes().get(param.len()) == Some(&b'='))
        .and_then(|p| p.split('=').nth(1))
}

pub fn encode_with_encoding(
    data: Vec<u8>,
    uri: &Uri,
) -> Result<Response<Full<hyper::body::Bytes>>> {
    let encoding = get_query_param(uri.query(), "encoding").unwrap_or("base64");
    let (encoded, encoding) = match encoding {
        "binary" => (data, "application/octet-stream"),
        "hex" => (hex::encode(data).into_bytes(), "text/plain"),
        _ => (
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data).into_bytes(),
            "text/plain",
        ),
    };
    Ok(Response::builder().header(hyper::header::CONTENT_TYPE, encoding).body(full(encoded))?)
}

pub fn full<T: Into<Bytes>>(chunk: T) -> Full<Bytes> {
    Full::new(chunk.into())
}

pub async fn make_request<SM: Secmod + 'static>(
    out_port: u32,
    request: Request<Full<Bytes>>,
) -> Result<Response<Incoming>> {
    let uri = request.uri().clone();
    let scheme = uri.scheme_str().context("missing scheme")?;
    let require_tls = match scheme {
        "http" => false,
        "https" => true,
        _ => bail!("unexpected scheme {}", scheme),
    };
    let host = uri.host().context("missing hostname")?.to_string();
    let authority = uri.authority().context("missing authority")?.clone();
    tracing::debug!("connecting to host port {} for authority {}", out_port, authority);
    let stream = SM::connect(out_port).await?;
    use hyper::client::conn::http2::Builder;
    let mut sender = if !require_tls {
        let io = hyper_util::rt::TokioIo::new(stream);
        let (sender, conn) = Builder::new(hyper_util::rt::TokioExecutor::new())
            .initial_connection_window_size(65535) // Default HTTP/2 value
            .initial_stream_window_size(65535) // Default HTTP/2 value
            .max_frame_size(16384) // Standard value
            .handshake(io)
            .await?;
        // Spawn a task to poll the connection, driving the HTTP state
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                tracing::error!("connection failed: {:?}", err);
            }
        });
        sender
    } else {
        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        // Wrap the stream with TLS
        let server_name = pki_types::ServerName::try_from(host)?;
        let tls_stream = connector.connect(server_name, stream).await?;
        let io = hyper_util::rt::TokioIo::new(tls_stream);
        let (sender, conn) = Builder::new(hyper_util::rt::TokioExecutor::new())
            .initial_connection_window_size(65535) // Default HTTP/2 value
            .initial_stream_window_size(65535) // Default HTTP/2 value
            .max_frame_size(16384) // Standard value
            .handshake(io)
            .await?;
        // Spawn a task to poll the connection, driving the HTTP state
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                tracing::error!("connection failed: {:?}", err);
            }
        });
        sender
    };

    tracing::debug!(
        "sending request - URI: {}, method: {}, version: {:?}, headers: {:#?}",
        request.uri(),
        request.method(),
        request.version(),
        request.headers()
    );
    // Await the response...
    let response = sender.send_request(request).await?;
    tracing::debug!("response status: {}", response.status());

    Ok(response)
}

pub fn error_response(
    status: hyper::StatusCode,
    message: String,
) -> Response<Full<hyper::body::Bytes>> {
    let error_json = serde_json::json!({
        "error": {
            "message": message,
            "status": status.as_u16()
        }
    });
    Response::builder()
        .status(status)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(full(error_json.to_string()))
        .unwrap()
}

pub async fn serve_http_connection<SM: Secmod, T, F, Fut>(
    io: hyper_util::rt::TokioIo<T>,
    service: F,
) -> Result<()>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    F: Fn(hyper::Request<hyper::body::Incoming>) -> Fut + Clone + Send + 'static,
    Fut: std::future::Future<
            Output = Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>>,
        > + Send,
{
    // Connection builder.
    let builder = hyper::server::conn::http1::Builder::new();
    let service_fn = |x| async {
        let ok = match service(x).await {
            Ok(response) => response,
            Err(err) => {
                tracing::error!("request processing error: {}", err);
                error_response(hyper::StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
        };
        Ok::<_, hyper::Error>(ok)
    };
    builder.serve_connection(io, hyper::service::service_fn(service_fn)).await?;
    Ok(())
}
