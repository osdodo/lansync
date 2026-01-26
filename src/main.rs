use futures_util::{SinkExt, StreamExt};
use qrcode::{QrCode, render::unicode};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use warp::{Filter, http::Response};

const HTML: &str = include_str!("../static/index.html");
const CSS: &str = include_str!("../static/style.css");
const JS: &str = include_str!("../static/app.js");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ip = local_ip_address::local_ip().unwrap_or("127.0.0.1".parse()?);
    let url = format!("http://{}:8080", ip);

    println!(
        "\nOpen in browser: {}\n\nScan QR code with mobile device:\n{}\n",
        url,
        QrCode::new(&url)?.render::<unicode::Dense1x2>().build()
    );
    ctrlc::set_handler(|| std::process::exit(0))?;

    tokio::runtime::Runtime::new()?.block_on(run_server(url, 8080))
}

async fn run_server(_url: String, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(String::new()));
    let (tx, _) = broadcast::channel(16);

    let state_filter = warp::any().map(move || Arc::clone(&state));
    let tx_filter = warp::any().map({
        let tx = tx.clone();
        move || tx.clone()
    });
    let rx_filter = warp::any().map({
        let tx = tx.clone();
        move || tx.subscribe()
    });

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(state_filter)
        .and(tx_filter)
        .and(rx_filter)
        .and_then(|ws: warp::ws::Ws, state, tx, rx| async move {
            Ok::<_, warp::Rejection>(ws.on_upgrade(move |socket| handle(socket, state, tx, rx)))
        });

    let index = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(HTML)
    });

    let style = warp::path("static").and(warp::path("style.css")).map(|| {
        Response::builder()
            .header("content-type", "text/css; charset=utf-8")
            .body(CSS)
    });

    let script = warp::path("static").and(warp::path("app.js")).map(|| {
        Response::builder()
            .header("content-type", "application/javascript; charset=utf-8")
            .body(JS)
    });

    let routes = ws.or(index).or(style).or(script);
    warp::serve(routes)
        .run(format!("0.0.0.0:{}", port).parse::<std::net::SocketAddr>()?)
        .await;

    Ok(())
}

async fn handle(
    socket: warp::ws::WebSocket,
    state: Arc<Mutex<String>>,
    tx: broadcast::Sender<String>,
    mut rx: broadcast::Receiver<String>,
) {
    let (mut sink, mut stream) = socket.split();

    tokio::spawn(async move {
        let initial = state.lock().await.clone();
        let _ = sink.send(warp::ws::Message::text(&initial)).await;

        tokio::select! {
            _ = async {
                while let Ok(msg) = rx.recv().await {
                    if sink.send(warp::ws::Message::text(&msg)).await.is_err() {
                        break;
                    }
                }
            } => {}
            _ = async {
                while let Some(msg) = stream.next().await {
                    if let Ok(msg) = msg
                        && let Ok(text) = msg.to_str() {
                            *state.lock().await = text.to_string();
                            let _ = tx.send(text.to_string());
                        }
                }
            } => {}
        }
    });
}
