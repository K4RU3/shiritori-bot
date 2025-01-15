use std::{future::Future, pin::Pin};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{game::{channel_exists, contains_word, register, find_levenstein_distance, find_piece_equals}, spawn, utility::{generate_basic_message, generate_client, get_word_valid, verbose_log_async, CONFIG}};

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    #[serde(rename = "type")]
    r#type: u8,
    id: String,
    channel_id: String,
    content: String,
    mentions: Vec<User>,

    #[serde(skip_serializing_if = "Option::is_none")]
    reactions: Option<Reaction>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Reaction {
    count: u8,
    count_details: Vec<CountDetails>,
    me: bool,
    me_burst: bool,
    //emoji:
    burst_colors: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug)]
struct CountDetails {
    burst: u8,
    normal: u8
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: String,
    username: String,
    global_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bot: Option<bool>,
    permission_type: i32,
}

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

pub async fn check_word(word: String, channel_id: String) {
    let reg = Regex::new(r"^[a-zA-Z][a-zA-Z\s\-]*[a-zA-Z]$").unwrap();

    if reg.is_match(&word) {
        verbose_log_async(format!("Valid word: {}", word).as_str()).await;
        let space_reg = Regex::new(r"\s+").unwrap();
        let mut replaced = word.replace("-", " ");
        replaced = replaced.to_lowercase();
        replaced = space_reg.replace_all(&replaced, " ").to_string();

        spawn!(manage_exsist_word(channel_id.clone(), replaced.clone()));
        spawn!(manage_find_word(channel_id.clone(), replaced.clone()));
        spawn!(manage_like_word(channel_id.clone(), replaced.clone()));
        spawn!(manage_valid_vote(channel_id.clone(), replaced.clone()));
    }
}

async fn manage_exsist_word(channel_id: String, word: String) {
    let gen_after = {
        let channel_id = channel_id.clone();
        let word = word.clone();
        move |_message: Message| {
            Box::pin(async move {
                let contains = contains_word(channel_id.clone(), word.clone()).await;
                let next_message;
                if contains {
                    next_message = format!("{} は既に使用されています。", word);
                } else {
                    next_message = format!("{} と完全一致する単語は使用されていません。", word);
                }

                generate_basic_message(next_message.as_str())
            }) as Pin<Box<dyn Future<Output = String> + Send>>
        }
    };

    send_and_patch(channel_id, format!("{} を検索中...", word), gen_after).await;
}



async fn manage_find_word(channel_id: String, word: String) {
    let gen_after = {
        let word = word.clone();
        move |_message: Message| {
            Box::pin(async move {
                let exists = get_word_valid(word.clone()).await;
                let next_message;
                if exists {
                    next_message = format!("{} が見つかりました。", word);
                } else {
                    next_message = format!("{} は dictionary apiでは見つかりませんでした。", word);
                }

                generate_basic_message(next_message.as_str())
            }) as Pin<Box<dyn Future<Output = String> + Send>>
        }
    };

    send_and_patch(channel_id, format!("{} を dictionary api で検索中...", word), gen_after).await;
}

async fn manage_like_word(channel_id: String, word: String) {
    let gen_after = {
        let channel_id = channel_id.clone();
        let word = word.clone();
        move |_message: Message| {
            Box::pin(async move {
                let (piece, distance): (Option<Vec<String>>, Option<Vec<String>>) = tokio::join!(
                    find_piece_equals(channel_id.clone(), word.clone()),
                    find_levenstein_distance(channel_id.clone(), word.clone(), 0.3)
                );

                let mut result = Vec::<String>::new();
                if let Some(piece) = piece {
                    result = piece;
                }
                if let Some(dist) = distance {
                    result.extend(dist);
                }

                let next_message;
                if result.is_empty() {
                    next_message = format!("{} に近似する単語は使用されていません。", word);
                } else {
                    let joined_result = result.join("\n");
                    next_message = format!("{} に近い単語\\n{}\\nが見つかりました。", word, joined_result);
                }

                generate_basic_message(next_message.as_str())
            }) as Pin<Box<dyn Future<Output = String> + Send>>
        }
    };

    send_and_patch(channel_id, format!("{} を使用単語から検索中...", word), gen_after).await;
}

async fn manage_valid_vote(_channel_id: String, _word: String) {

}

async fn send_and_patch<F>(channel_id: String, first_message: String, gen_second_message: F) where F: FnOnce(Message) -> Pin<Box<dyn Future<Output = String> + Send>>, {
    let client = generate_client();
    let first_message_raw = generate_basic_message(first_message.as_str());
    let res = match client.post(format!("{}/channels/{}/messages", CONFIG.base_api_url, channel_id)).body(first_message_raw).send().await {
        Ok(res) => res,
        Err(_) => return,
    };
    
    let json: Message = match res.json().await {
        Ok(json) => json,
        Err(_) => {
            verbose_log_async("Failed to parse message").await;
            return
        }
    };

    let message_id = json.id.clone();

    let second_message = gen_second_message(json).await;

    match client.patch(format!("{}/channels/{}/messages/{}", CONFIG.base_api_url, channel_id, message_id)).body(second_message).send().await {
        Ok(res) => verbose_log_async(format!("Message edit: {}", res.status()).as_str()).await,
        Err(e) => verbose_log_async(format!("Failed to send message: {}", e).as_str()).await,
    }
}