use std::env;
use reqwest::header::HeaderValue;
use reqwest::Client;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

pub struct BotConfig {
    pub base_api_url: String,
    pub token: String,
    pub app_id: String,
    pub user_agent: String,
    pub content_type: String,
    pub auth: String,
    pub msg_dist_threshold: f64,
    pub vote_count: u8,
}

impl BotConfig {
    pub fn new() -> Self {
        let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is not set");
        let app_id = std::env::var("DISCORD_APP_ID").expect("DISCORD_APP_ID is not set");
        let threshold = match std::env::var("MSG_DIST_THRESHOLD") {
            Ok(val) => val,
            Err(_) => {
                println!("MSG_DIST_THRESHOLD is not set, defaulting to 0.3");
                "0.3".to_string()
            }
        };
        let vote_count = match std::env::var("VOTE_COUNT") {
            Ok(val) => val,
            Err(_) => {
                println!("VOTE_COUNT is not set, defaulting to 3");
                "3".to_string()
            }
        };
        
        Self {
            base_api_url: String::from("https://discord.com/api/v10"),
            token: token.clone(),
            app_id: app_id.clone(),
            user_agent: String::from("DiscordBot(www.rikka-space.com, 10)"),
            content_type: String::from("application/json"),
            auth: format!("Bot {}", token),
            msg_dist_threshold: threshold.parse().unwrap_or(0.3),
            vote_count: vote_count.parse().unwrap_or(3),
        }
    }
}

lazy_static::lazy_static! { pub static ref CONFIG: BotConfig = BotConfig::new(); } // Globaly

pub async fn get_word_valid(word: String) -> bool {
    let base_url = "https://api.dictionaryapi.dev/api/v2/entries/en/";
    let target_url = format!("{}{}", base_url, word);

    let response = reqwest::get(&target_url).await;

    let response = match response {
        Ok(res) => res,
        Err(_) => return false,
    };

    let body = response.text().await;

    let body = match body {
        Ok(text) => text,
        Err(_) => return false,
    };

    if body.chars().next() == Some('[') {
        return true;
    }

    false
}

lazy_static::lazy_static! {
    static ref VERBOSE_LOGGING_ENABLED: Mutex<Option<bool>> = Mutex::new(None);
}

pub async fn verbose_log_async(message: &str) {
    let mut verbose_enabled = VERBOSE_LOGGING_ENABLED.lock().await;
    
    if verbose_enabled.is_none() {
        let verbose = env::var("verbose").unwrap_or_else(|_| String::new());
        *verbose_enabled = Some(verbose == "true" || verbose == "1");
    }

    if (*verbose_enabled).unwrap_or_else(|| false) {
        let now = chrono::offset::Local::now();
        let time_string = now.format("%H:%M:%S").to_string();
        let log_message = format!("{} - {}", time_string, message);
        println!("{}", log_message);

        let path = "log.txt";
        match fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(log_message.as_bytes()).await {
                    eprintln!("Failed to write to log file: {}", e);
                }
                if let Err(e) = file.write_all(b"\n").await {
                    eprintln!("Failed to write newline to log file: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to open log file: {}", e),
        }
    }
}

pub fn generate_client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", HeaderValue::from_str(&CONFIG.user_agent).unwrap());
    headers.insert("Authorization", HeaderValue::from_str(&CONFIG.auth).unwrap());
    headers.insert("Content-Type", HeaderValue::from_str(&CONFIG.content_type).unwrap());
    Client::builder().default_headers(headers).build().unwrap()
}

pub fn generate_basic_message(message: &str) -> String {
    format!(r#"{{"content":"{}", "tts": false}}"#, message)
}