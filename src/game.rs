use std::fs::OpenOptions;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::io::Write;
use std::sync::{Arc, RwLock};
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

pub async fn register(channnel_id: &str) {
    let channel_data_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!("channels/{}/data.json", channnel_id));

    match channel_data_file {
        Ok(mut file) => {
            let channel = match serde_json::from_reader(&mut file) {
                Ok(channel) => channel,
                Err(e) => {
                    println!("Failed to read channel data: {}", channnel_id);
                    verbose_log_async(&format!("Failed to read channel data: {}", e)).await;
                    return;
                }
            };

            let mut channels = CHANNELS.write().unwrap();
        }

        Err(e) => {
            println!("Failed to open channel: {}", e);
            verbose_log_async(&format!("Failed to open channel: {}", e)).await;
        }
    }
}

pub async fn save_channel(channel_id: &str) {
    let channel = {
        let channels = CHANNELS.read().unwrap();
        match channels.get(channel_id) {
            Some(channel) => channel.clone(),
            None => {
                println!("Failed to save channel: {}", channel_id);
                verbose_log_async(&format!("Failed to save channel: {}", channel_id)).await;
                return;
            }
        }
    };

    let channel_data_json = serde_json::to_string(&channel).unwrap();

    let channel_data_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!("channels/{}/data.json", channel_id));

    match channel_data_file {
        Ok(mut file) => {
            match file.write_all(channel_data_json.as_bytes()) {
                Ok(_) => {
                    println!("Saved channel: {}", channel_id);
                    verbose_log_async(&format!("Saved channel: {}", channel_id)).await;
                }
                Err(e) => {
                    println!("Failed to save channel: {}", e);
                    verbose_log_async(&format!("Failed to save channel: {}", e)).await;
                }
            }
        }
        Err(e) => {
            println!("Failed to open channel: {}", e);
            verbose_log_async(&format!("Failed to open channel: {}", e)).await;
        }
    }
}

pub async fn check_word_in_channel(channel_id: &str, target_word: &str) -> bool {
    let channels = CHANNELS.read().unwrap();
    if let Some(channel) = channels.get(channel_id) {
        if let Some(words) = &channel.words {
            return words.contains(target_word);
        }
    }
    false
}

//queue.make_contiguous().reverse();