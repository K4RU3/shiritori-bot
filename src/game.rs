use std::fs::{self, OpenOptions};
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use lazy_static::lazy_static;

use serde::{Serialize, Deserialize};

use crate::utility::verbose_log_async;

lazy_static! {
    pub static ref CHANNELS: Arc<RwLock<HashMap<String, Channel>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Channel {
    channel_id: String,
    users: VecDeque<String>,
    words: Option<BTreeSet<String>>
}

pub async fn register(channel_path: String) {
    let path_name = format!("{}/data.json", channel_path);

    let channel_data_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path_name.clone());

    match channel_data_file {
        Ok(mut file) => {
            let abs_path = fs::canonicalize(&channel_path).unwrap();
            let abs_path_string = abs_path.to_str().unwrap().to_string();
            verbose_log_async(&format!("Absolute path: {}", abs_path_string)).await;

            let channel: Channel = match serde_json::from_reader(&mut file) {
                Ok(channel) => channel,
                Err(e) => {
                    println!("Failed to read channel data: {}", path_name);
                    verbose_log_async(&format!("Failed to read channel data: {}", e)).await;
                    return;
                }
            };

            let mut channels = CHANNELS.write().await;

            channels.insert(channel_path.clone(), channel);

            println!("Registered channel: {}", channel_path);
        }

        Err(e) => {
            println!("Failed to open channel: {}", e);
            verbose_log_async(&format!("Failed to open channel: {}", e)).await;
        }
    }
}

//queue.make_contiguous().reverse();