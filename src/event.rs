use regex::Regex;

use crate::{game::{channel_exists, register}, utility::{generate_basic_message, generate_client, verbose_log_async, CONFIG}};

pub async fn check_mention_for_me(event: &serde_json::Value) -> Result<(), ()> {
    let mut through_flag = true;
    let mut send_flag = false;
    let mentions = event["d"]["mentions"].clone();
    let channel_id = event["d"]["channel_id"].as_str().unwrap();
    let mut message = "Internal Error".to_string();

    if mentions.is_array() && mentions.as_array().unwrap().iter()
        .any(|mention| mention["username"].as_str().unwrap()== "NS Shiritori") {
            if channel_exists(channel_id).await == false {
                if register(channel_id.to_string()).await.is_ok() {
                    message = "チャンネルの登録が完了しました。".to_string();
                } else {
                    message = "登録が失敗しました。".to_string();
                }
            } else {
                message = "このチャンネルは既に登録されています。".to_string();
            }
            send_flag = true;
            through_flag = false;
    }

    let client = generate_client();

    if send_flag {
        let send_message = generate_basic_message(message.as_str());
        client.post(&format!("{}/channels/{}/messages", CONFIG.base_api_url, channel_id)).body(send_message).send().await.unwrap();
    }

    if through_flag {
        Err(())
    } else {
        Ok(())
    }
}

pub async fn check_word(word: String) {
    let reg = Regex::new(r"^[a-zA-Z][a-zA-Z\s\-]*[a-zA-Z]$").unwrap();

    if reg.is_match(&word) {
        verbose_log_async(format!("Valid word: {}", word).as_str()).await;
        let space_reg = Regex::new(r"\s+").unwrap();
        let mut replaced = word.replace("-", " ");
        replaced = replaced.to_lowercase();
        replaced = space_reg.replace_all(&replaced, " ").to_string();

        verbose_log_async(format!("Replaced word: {}", replaced).as_str()).await;
    }
}