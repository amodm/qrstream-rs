use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex as AsyncMutex;

use crate::{console, error::Result, QRSTREAM_MAGIC, QRSTREAM_VERSION};

lazy_static::lazy_static! {
    static ref CAM_DATA_TX: Arc<AsyncMutex<Option<Sender<String>>>> = <_>::default();
}

type HyperResult<T> = std::result::Result<T, hyper::Error>;

pub async fn get_content_from_camera() -> Result<Vec<u8>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(10);
    CAM_DATA_TX.lock().await.replace(tx);

    let make_service =
        make_service_fn(|_conn| async { Ok::<_, hyper::Error>(service_fn(handle_decode_get)) });

    let server = Server::bind(&addr).serve(make_service);
    let url = format!("http://{}/", server.local_addr());
    console::println(format!("Opening {url}"));
    webbrowser::open(&url)?;

    let content_data: Arc<Mutex<Option<String>>> = <_>::default();
    let graceful = server.with_graceful_shutdown(async {
        if let Some(data) = rx.recv().await {
            if let Ok(mut guard) = content_data.lock() {
                guard.replace(data);
            }
        }
    });

    if let Err(e) = graceful.await {
        console::println(format!("server error: {}", e));
    }
    let bytes = content_data.lock().unwrap().take().unwrap().into_bytes();
    Ok(bytes)
}

async fn handle_decode_get(request: Request<Body>) -> HyperResult<Response<Body>> {
    Ok(
        if request.uri() == "/" && request.method() == hyper::Method::GET {
            Response::new(Body::from(
                SERVER_INDEX_HTML
                    .replace("MAGIC_PREFIX", &format!("\"{QRSTREAM_MAGIC}\""))
                    .replace("CURRENT_VERSION", &format!("{QRSTREAM_VERSION}")),
            ))
        } else if request.uri() == "/data" && request.method() == hyper::Method::PUT {
            let body =
                String::from_utf8(hyper::body::to_bytes(request.into_body()).await?.to_vec())
                    .unwrap();
            _ = CAM_DATA_TX.lock().await.as_ref().unwrap().send(body).await;
            Response::builder()
                .status(hyper::StatusCode::OK)
                .body(Body::from(""))
                .unwrap()
        } else {
            Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(Body::from(""))
                .unwrap()
        },
    )
}

const SERVER_INDEX_HTML: &str = include_str!("../static/index.html");
