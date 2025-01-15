use futures::stream::SplitStream;
use futures::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};
use reqwest;
use tokio::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use tokio::time::{self, Duration};
use tokio::sync::Mutex;

use crate::event::{check_mention_for_me, check_word, update_vote};
use crate::game::{channel_exists, load_channel};
use crate::utility::{self, verbose_log_async};
use crate::spawn;

#[derive(Serialize, Deserialize)]
struct UrlResponse {
    url: String
}

const HARTBEAT_REQUEST: &str = "{\"op\":1, \"d\":null}";
type StreamLock = Arc<Mutex<futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>>;

pub async fn login_bot() {
    let config = &utility::CONFIG;
    let client = reqwest::Client::new();
    let gateway_url = "https://discordapp.com/api/gateway";

    let url_raw_response = client.get(gateway_url).send().await.unwrap();
    let url_response: UrlResponse = url_raw_response.json().await.unwrap();

    let ws_url = url_response.url;
    let identify = format!(
        r#"{{"op": 2, "d": {{"token": "{}", "properties": {{"os": "linux", "device": "device", "browser": "browser"}}, "intents": 1536}}}}"#,
        config.token
    );

    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect to gateway");
    let (write, read) = ws_stream.split();
    let write: StreamLock = Arc::new(Mutex::new(write));
    println!("Connected to gateway at {}", ws_url);

    {
        let mut write_stream = write.lock().await;
        verbose_log_async(format!("Sending identify: {}", identify).as_str()).await;
        write_stream.send(Message::text(identify)).await.expect("Failed to send identify");
    }

    spawn!(main_loop(write, read));
    
    spawn!(registry_for());

}

async fn main_loop(write: StreamLock, mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) {
    loop {
        let stream = read.next().await.expect("Failed to receive message from gateway");
        match stream {
            Ok(message) => match message {
                Message::Text(text) => {
                    let json = serde_json::from_str::<serde_json::Value>(&text).unwrap();
                    
                    let op = json["op"].as_u64().unwrap();

                    if op == 10 {
                        let heartbeat_interval = json["d"]["heartbeat_interval"].as_u64().unwrap();
                        let lock_clone = write.clone();
                        let mut interval = time::interval(Duration::from_millis(heartbeat_interval));

                        tokio::spawn(async move {
                            loop {
                                interval.tick().await;
                                
                                let mut write_stream = lock_clone.lock().await;
                                write_stream.send(Message::text(HARTBEAT_REQUEST)).await.unwrap();

                                verbose_log_async("Sent heartbeat").await;
                            }
                        });
                    } else if op == 0 {
                        event_handler(json).await;
                    }
                }
                Message::Close(Some(close_frame)) => {
                    println!("Gateway closed with code: {}, reason: {}", close_frame.code, close_frame.reason);
                    break;
                }
                _ => {}
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
}

async fn registry_for() {
    verbose_log_async("Registering channels...").await;
    let current_dir = std::env::current_dir().unwrap();
    verbose_log_async(format!("Current directory: {}", current_dir.display()).as_str()).await;

    let path_str = "./channels/";

    let path: &Path = Path::new(path_str);
    if path.exists() && path.is_dir() {
        let dir = match std::fs::read_dir(path) {
            Ok(dir) => dir,
            Err(e) => {
                println!("Failed to read channels directory: {}", e);
                return;
            }
        };

        for entry in dir {
            let channel = entry.unwrap().path();
            let file_name    = match channel.file_name().unwrap().to_str() {
                Some(file_name) => file_name,
                None => continue
            };

            if channel.is_dir() {
                spawn!(load_channel(file_name.to_string()));
            }
        }
    } else {
        verbose_log_async(format!("{} is not a directory or does not exist", path_str).as_str()).await;
    }
}

async fn event_handler(event: serde_json::Value) {
    let event_type = event["t"].as_str().unwrap_or("");
    match event_type {
        "MESSAGE_CREATE" => {
            if event["d"]["author"]["bot"] != true {
                if check_mention_for_me(&event).await.is_ok() { return; }

                verbose_log_async("Message received").await;
                let channel_id = match event["d"]["channel_id"].as_str() {
                    Some(channel_id) => channel_id,
                    None => return
                };

                if channel_exists(&channel_id).await == true {
                    verbose_log_async("Channel active").await;
                    let content = event["d"]["content"].as_str().unwrap();
                    spawn!(check_word(content.to_string(), channel_id.to_string()));
                }
            }
        }

        "MESSAGE_REACTION_ADD" => {
            println!("Reaction added");
            let _ = update_vote(&event["d"]).await;
        }

        _ => {
            println!("Unknown event type: {}", event_type);
        }
    }
        
}