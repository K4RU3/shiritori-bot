use crate::{game::{channel_exists, register}, utility::{generate_basic_message, generate_client, verbose_log_async, CONFIG}};

pub async fn check_mention_for_me(event: &serde_json::Value) {
    let mentions = event["d"]["mentions"].clone();
    let channel_id = event["d"]["channel_id"].as_str().unwrap();
    let mut message = "Internal Error".to_string();

    if channel_exists(channel_id).await {
        message = generate_basic_message("すでにこのチャンネルは登録されています。");
    }else if mentions.is_array() {
        for mention in mentions.as_array().unwrap() {
            let mention_username = mention["username"].as_str().unwrap();
            if mention_username == "NS Shiritori" {
                verbose_log_async("Mentioned NS Shiritori").await;

                match register(channel_id.to_string()).await {
                    Ok(_) => message = generate_basic_message("このチャンネルを登録しました。"),
                    Err(_) => message = generate_basic_message("チャンネルの登録に失敗しました。"),
                };
            }
        }
    }

    let client = generate_client();

    client.post(&format!("{}/channels/{}/messages", CONFIG.base_api_url, channel_id)).body(message).send().await.unwrap();
}