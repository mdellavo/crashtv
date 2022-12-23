// #![deny(warnings)]

extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use warp::ws::WebSocket;
use warp::Filter;

pub mod game;
use game::GameArea;

const AREA_SIZE: u32 = 1000;

type AreaRef = Arc<Mutex<GameArea>>;

async fn user_connected(websocket: WebSocket, area: AreaRef) {

    area.lock().unwrap().


    user_disconnected(websocket).await;
}

async fn user_disconnected(websocket: WebSocket) {
    let rv = websocket.close().await;

    if let Err(e) = rv {
        log::error!("error closing socket: {:?}", e);
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let area = Arc::new(Mutex::new(GameArea::new(AREA_SIZE)));
    area.lock().unwrap().populate(500);

    let route = warp::path("ws")
        .and(warp::addr::remote())
        .and(warp::ws())
        .and(warp::any().map(move || area.clone()))
        .map(move |addr: Option<SocketAddr>, ws: warp::ws::Ws, area: AreaRef| {
            if let Some(addr) = addr {
                log::info!("{:?} connected", addr);
            }
            ws.on_upgrade(move |websocket| {
                async {
                    user_connected(websocket, area).await;
                }
            });
            ""
        });

    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
}
