use futures::{SinkExt, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Serialize, Deserialize};
use reqwest;
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio::time::{self, Duration};
use tokio::sync::Mutex;

use crate::event::create_message;
use crate::utility::{self, verbose_log_async, BotConfig};


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
        r#"{{"op": 2, "d": {{"token": "{}", "properties": {{"os": "linux", "device": "device", "browser": "browser"}}, "intents": 513}}}}"#,
        config.token
    );

    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect to gateway");
    let (write, mut read) = ws_stream.split();
    let write: Arc<Mutex<futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>> = Arc::new(Mutex::new(write));
    println!("Connected to gateway at {}", ws_url);

    {
        let mut write_stream = write.lock().await;
        write_stream.send(Message::text(identify)).await.expect("Failed to send identify");
    }


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

                                println!("Sent heartbeat");
                            }
                        });
                    } else if op == 0 {
                        event_handler(json, config).await;
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

async fn event_handler(event: serde_json::Value, config: &BotConfig) {
    match event["t"].as_str().unwrap() {
        "MESSAGE_CREATE" => {
            if event["d"]["author"]["bot"] != true {
                let username = event["d"]["author"]["username"].as_str().unwrap();
                let channel_id = event["d"]["channel_id"].as_str().unwrap();
                let recieved_message = event["d"]["content"].as_str().unwrap();
                let raw_message = format!("Test Message: {} -> {} at {}", username, recieved_message, channel_id);

                let message = create_message(&raw_message);

                let target_url = format!("{}/channels/{}/messages", config.base_api_url, channel_id);

                let client = reqwest::Client::new();
                let mut headers = HeaderMap::new();
                headers.insert("Content-Type", HeaderValue::from_str(&config.content_type).unwrap());
                headers.insert("Authorization", HeaderValue::from_str(&config.auth).unwrap());
                headers.insert("User-Agent", HeaderValue::from_str(&config.user_agent).unwrap());

                let result = client.post(&target_url).headers(headers).body(message).send().await;

                match result {
                    Ok(result) => {
                        let body = result.text().await.unwrap();
                        println!("Response body: {}", body);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
        }

        _ => {
            
        }
    }
}