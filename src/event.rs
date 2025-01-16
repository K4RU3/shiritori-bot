use std::{collections::HashSet, future::Future, pin::Pin};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{game::{channel_exists, contains_word, find_levenstein_distance, find_piece_equals, register, CHANNELS}, utility::{generate_basic_message, generate_client, get_word_valid, verbose_log_async, CONFIG}};

lazy_static! {
    static ref VOTES: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));
}

const VALID_VOTE: &str = "👍";
const INVALID_VOTE: &str = "👎";

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    #[serde(rename = "type")]
    r#type: u8,
    id: String,
    channel_id: String,
    content: String,
    mentions: Vec<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reactions: Option<Vec<Reaction>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Reaction {
    count: u8,
    count_details: CountDetails,
    me: bool,
    me_burst: bool,
    emoji: Emoji
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

#[derive(Serialize, Deserialize, Debug)]
struct UpdateReaction {
    emoji: Emoji,
    channel_id: String,
    message_id: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Emoji {
    name: String,
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

        manage_exsist_word(channel_id.clone(), replaced.clone()).await;
        manage_find_word(channel_id.clone(), replaced.clone()).await;
        manage_like_word(channel_id.clone(), replaced.clone()).await;
        manage_valid_vote(channel_id.clone(), replaced.clone()).await;
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
                    next_message = format!("{} が dictionary api で見つかりました。", word);
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
                    find_levenstein_distance(channel_id.clone(), word.clone(), CONFIG.msg_dist_threshold)
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

async fn manage_valid_vote(channel_id: String, word: String) {
    let client = generate_client();
    let message = generate_basic_message(format!("「{}」 の有効投票を開始します。", word).as_str());

    let res = match client.post(format!("{}/channels/{}/messages", CONFIG.base_api_url, channel_id)).body(message).send().await {
        Ok(res) => res,
        Err(_) => return,
    };

    let text = res.text().await.unwrap_or("".to_string());

    let json: Message = match serde_json::from_str(text.as_str()) {
        Ok(json) => json,
        Err(_) => {
            verbose_log_async("Failed to parse message at vote").await;
            return
        }
    };
    let msg_id = json.id.clone();
    
    {
        let mut vote_lock = VOTES.write().await;
        vote_lock.insert(msg_id);
    }

    // up %F0%9F%91%8D%EF%B8%8F
    // down %F0%9F%91%8E%EF%B8%8F
    send_vote(client.clone(), channel_id.clone(), json.id.clone(), VALID_VOTE.to_string()).await;
    send_vote(client, channel_id, json.id.clone(), INVALID_VOTE.to_string()).await;
}

async fn send_vote(client: Client, channel_id: String, message_id: String, vote: String) {
    let _ = client.put(format!(r#"{}/channels/{}/messages/{}/reactions/{}/@me"#, CONFIG.base_api_url, channel_id, message_id, vote)).send().await;
}

async fn send_and_patch<F>(channel_id: String, first_message: String, gen_second_message: F) where F: FnOnce(Message) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + 'static, {
    let client = generate_client();
    let first_message_raw = generate_basic_message(first_message.as_str());
    let res = match client.post(format!("{}/channels/{}/messages", CONFIG.base_api_url, channel_id)).body(first_message_raw).send().await {
        Ok(res) => res,
        Err(_) => return,
    };
    
    tokio::spawn(async move {
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
        };
    });
}

pub async fn update_vote(d: &serde_json::Value) {
    let data: UpdateReaction = match serde_json::from_value(d.clone()) {
        Ok(data) => data,
        Err(_) => {
            verbose_log_async(format!("Failed to parse message: {}", d).as_str()).await;
            return;
        }
    };

    if !(data.emoji.name == VALID_VOTE || data.emoji.name == INVALID_VOTE) { 
        verbose_log_async("Reaction emoji is not vote emoji").await;
        return;
    }

    let target_reaction = data.emoji.name;

    let message;
    {
        let votes = VOTES.write().await;

        if !votes.contains(data.message_id.as_str()) {
            verbose_log_async("Message is not vote message").await;
            return;
        }

        let client = generate_client();
        match client.get(format!("{}/channels/{}/messages/{}", CONFIG.base_api_url, data.channel_id, data.message_id)).send().await {
            Ok(res) => {
                let text = res.text().await.unwrap_or("".to_string());
                message = match serde_json::from_str::<Message>(text.as_str()) {
                    Ok(message) => message,
                    Err(err) => {
                        verbose_log_async("Failed to parse message").await;
                        verbose_log_async(err.to_string().as_str()).await;
                        return;
                    }
                };
            },
            Err(_) => {
                verbose_log_async("Failed to get message").await;
                return;
            }
        };
    }

    let reactions = match message.reactions {
        Some(reactions) => reactions,
        None => {
            verbose_log_async("reactions is none").await;
            return;
        }
    };

    let target_reaction = reactions.iter().find(|x| x.emoji.name == target_reaction);

    let match_reaction = match target_reaction {
        Some(reaction) => reaction,
        None => {
            verbose_log_async("target reaction is none").await;
            return;
        }
    };

    if match_reaction.count >= CONFIG.vote_count {
        verbose_log_async("Vote count is over").await;

        {
            let mut votes = VOTES.write().await;
            votes.remove(data.message_id.as_str());
        }

        let client = generate_client();
        let _ = client.delete(format!("{}/channels/{}/messages/{}/reactions", CONFIG.base_api_url, data.channel_id, data.message_id)).send().await;

        let new_message;
        if match_reaction.emoji.name == VALID_VOTE {
            new_message = "可決されました。この単語を使用リストに追加します。";
        } else {
            new_message = "否決されました。";
        }

        let new_raw_message = generate_basic_message(new_message);

        match client.patch(format!("{}/channels/{}/messages/{}", CONFIG.base_api_url, data.channel_id, data.message_id)).body(new_raw_message).send().await {
            Ok(res) => verbose_log_async(format!("Message edit: {}", res.status()).as_str()).await,
            Err(e) => verbose_log_async(format!("Failed to send message: {}", e).as_str()).await,
        }

        let word_regex = regex::Regex::new(r"^「([a-zA-Z][a-zA-Z\s\-]*[a-zA-Z])」.*$").unwrap();
        let word = match word_regex.captures(message.content.as_str()) {
            Some(captures) => captures.get(1).unwrap().as_str(),
            None => return
        };

        if match_reaction.emoji.name == VALID_VOTE {
            {
                let mut channels = CHANNELS.write().await;
                let channel = channels.get_mut(&data.channel_id).unwrap();
                channel.words.as_mut().unwrap().insert(word.to_string());
            }
        }
    }
}