use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Arc;
use tokio::fs::{create_dir_all, metadata, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use lazy_static::lazy_static;

use serde::{Serialize, Deserialize};

use crate::utility::verbose_log_async;

lazy_static! {
    pub static ref CHANNELS: Arc<RwLock<HashMap<String, Channel>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Channel {
    channel_id: String,
    users: VecDeque<String>,
    words: Option<BTreeSet<String>>
}

pub async fn register(original_id: String) -> Result<(), i32> {
    verbose_log_async(format!("Registering channel {}", original_id).as_str()).await;

    let channel_path = format!("channels/{}", original_id);
    let path_name = format!("{}/data.json", &channel_path);

    if let Err(_) = create_dir_all(&channel_path).await {
        verbose_log_async(format!("Failed to create directory: {}", channel_path).as_str()).await;
        return Err(1);
    }

    let basic_channel = Channel {
        channel_id: original_id.clone(),
        users: VecDeque::new(),
        words: BTreeSet::new().into()
    };

    let file_result = File::create_new(&path_name).await;
    match file_result {
        Ok(mut file) => {
            
            let channel_data = serde_json::to_string(&basic_channel).unwrap();
            file.write_all(channel_data.as_bytes()).await.unwrap();
        }
        Err(_) => {
            verbose_log_async(format!("Failed to create file: {}", path_name).as_str()).await;
            return Err(1);
        },
    }

    let mut channels = CHANNELS.write().await;
    channels.insert(original_id, basic_channel);

    verbose_log_async(format!("Registered channels {:?}", channels.keys()).as_str()).await;

    Ok(())
}

pub async fn load_channel(channel_id: String) -> Result<(), i32> {
    verbose_log_async(format!("Loading channel {}", channel_id).as_str()).await;

    let channel_file_path = format!("channels/{}/data.json", channel_id);

    match metadata(&channel_file_path).await {
        Ok(_) => {
            let mut file = match File::open(&channel_file_path).await {
                Ok(file) => file,
                Err(_) => {
                    verbose_log_async(format!("Failed to open file: {}", channel_file_path).as_str()).await;
                    return Err(2);
                }
            };
            
            let mut content = String::new();
            match file.read_to_string(&mut content).await {
                Ok(_) => {
                    let channel: Channel = serde_json::from_str(&content).unwrap();
                    let mut channels = CHANNELS.write().await;
                    channels.insert(channel_id.clone(), channel.clone());
                    verbose_log_async(format!("Loaded channel {:?}", channel).as_str()).await;
                }
                Err(_) => {
                    verbose_log_async(format!("Failed to read file: {}", channel_file_path).as_str()).await;
                    return Err(1);
                }
            }
        },
        Err(_) => {
            verbose_log_async(format!("Channel file not found: {}", channel_id).as_str()).await;
            return Err(1);
        }
    }

    Ok(())
}

pub async fn _save_channel(channel_id: String) {

}

pub async fn _save_all_channels() {
    let mut channel_ids = Vec::<String>::new();

    {
        let channels = CHANNELS.read().await;
        for channel_id in channels.keys() {
            channel_ids.push(channel_id.clone());
        }
    }

    for channel_id in channel_ids {
        tokio::spawn(async move {
            _save_channel(channel_id).await;
        });
    }
}

pub async fn channel_exists(channel_id: &str) -> bool {
    let channels = CHANNELS.read().await;
    channels.contains_key(channel_id)
}

//queue.make_contiguous().reverse();