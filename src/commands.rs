use futures::future;
use std::option::Option;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Serialize, Deserialize};
use tokio::fs;
use crate::utility::{self, BotConfig};
use crate::utility::{verbose_log_async, verbose_log_sync};

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    errors: Option<ErrorDetails>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetails {
    #[serde(flatten)]
    _fields: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct Command {
    name: String,
    #[serde(rename = "type")]
    r#type: u8,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<CommandOption>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CommandOption {
    name: String,
    description: String,
    #[serde(rename = "type")]
    r#type: u8,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    choices: Option<Vec<CommandChoice>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_value: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_value: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_length: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_length: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autocomplete: Option<bool>,
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
        Err(e) => {
            let error_message = format!("Error reading commands.json file: {}", e.to_string());
            if let Err(log_error) = verbose_log_sync(&error_message) {
                eprintln!("Failed to write to log: {}", log_error);
            }

            Err(())
        }
    }
}

async fn register_command(
    target_url: String,
    client: reqwest::Client,
    command_json: String,
    headers: HeaderMap,
) -> Result<(), ()> {
    let result = client
        .post(&target_url)
        .headers(headers)
        .body(command_json.clone())
        .send()
        .await;

    verbose_log_async(format!("Sending request to {} with body {}", target_url, command_json).as_str()).await;

    match result {
        Ok(result) => {
            let body = result.text().await.unwrap();
            verbose_log_async(format!("Response body: {}", body).as_str()).await;

            if let Ok(parsed_error) = serde_json::from_str::<ErrorResponse>(&body) {
                let log_message = format!(
                    "Register command error {}: {}\n{:?}\n",
                    parsed_error.code,
                    parsed_error.message,
                    parsed_error.errors
                );

                verbose_log_async(&log_message).await;
                return Err(());
            }

            Ok(())
        },

        Err(e) => {
            let log_message = format!(
                "Error sending request to {} with body {}: {}\n",
                target_url,
                command_json,
                e.to_string()
            );

            verbose_log_async(&log_message).await;
            Err(())
        }
    }
}

pub async fn register_commands() -> Result<(), ()> {
    let config = &utility::CONFIG;
    let target_url = format!("{}/applications/{}/commands", config.base_api_url, config.app_id);

    match read_commands_from_file("commands.json").await {
        Ok(commands) => {
            let client = reqwest::Client::new();
            let mut headers = HeaderMap::new();

            headers.insert(CONTENT_TYPE, HeaderValue::from_str(&config.content_type).unwrap());
            headers.insert("Authorization", HeaderValue::from_str(&config.auth).unwrap());
            headers.insert("User-Agent", HeaderValue::from_str(&config.user_agent).unwrap());

            let futures: Vec<_> = commands.iter().map(|command| {
                let target_url = target_url.clone();
                let client = client.clone();
                let command_json = serde_json::to_string(command).unwrap();
                let headers = headers.clone();
                register_command(target_url, client, command_json, headers)
            }).collect();

            let results = future::join_all(futures).await;

            for i in 0..commands.len() {
                match results[i] {
                    Ok(_) => println!("Loaded command: {}", commands[i].name),
                    Err(_) => println!("Failed to load command: {}", commands[i].name),
                }
            }

            Ok(())
        }

        Err(_) => {
            println!("Error reading commands from file");
            Err(())
        }
    }
}
