use futures::future;
use std::option::Option;
use reqwest::Error;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Serialize, Deserialize};
use tokio::fs;
use crate::utility::BotConfig;

#[derive(Serialize, Deserialize, Debug)]
struct Command {
    name: String,
    #[serde(rename = "type")]
    r#type: u8,
    description: String,
    options: Vec<CommandOption>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommandOption {
    name: String,
    description: String,
    #[serde(rename = "type")]
    r#type: u8,
    required: bool,
    choices: Option<Vec<CommandChoice>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommandChoice {
    name: String,
    value: String,
}

async fn read_commands_from_file(file_path: &str) -> Result<Vec<Command>, ()> {
    let file_content = fs::read_to_string(file_path).await.unwrap();
    match serde_json::from_str(&file_content) {
        Ok(commands) => Ok(commands),
        Err(_) => Err(()),
    }
}

#[tokio::main]
async fn register_command<'a>
    (   target_url: &'a str,
        original_client:&'a reqwest::Client,
        command: &'a Command,
        original_headers: &'a HeaderMap
    ) -> Result<&'a Command, &'a Command> {
    let client = original_client.clone();
    let headers = original_headers.clone();
    let post_body = serde_json::to_string(&command).unwrap();

    client
        .post(target_url)
        .headers(headers)
        .body(post_body)
        .send()
        .await
        .map_err(|_| Err(command))
        .and_then(|_| Ok(command))
}

pub async fn register_commands(config: &BotConfig) -> Result<(), ()> {
    let target_url = format!("{}/applications/{}/commands", config.base_api_url, config.app_id);

    match read_commands_from_file("commands.json").await {
        Ok(commands) => {
            let client = reqwest::Client::new();
            let mut headers = HeaderMap::new();

            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert("Authorization", HeaderValue::from_str(&format!("Bot {}", config.token)).unwrap());
            headers.insert("User-Agent", HeaderValue::from_str("DiscordBot(www.rikka-space.com,10)").unwrap());

            
            Ok(())
        }

        Err(_) => {
            println!("Error reading commands from file");
            Err(())
        }
    }
}




// test codes
#[tokio::main]
async fn main() -> Result<(), Error> {
    // HTTPS URL
    let url = "https://www.example.com";

    // GETリクエストを送信
    let response = reqwest::get(url).await?;

    // レスポンスのステータスコードを表示
    println!("Response Status: {}", response.status());

    // レスポンスボディを文字列として取得して表示
    let body = response.text().await?;
    println!("Response Body: {}", body);

    Ok(())
}