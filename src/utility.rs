pub struct BotConfig {
    pub base_api_url: String,
    pub token: String,
    pub app_id: String,
    pub user_agent: String,
    pub content_type: String,
    pub auth: String,
}

impl BotConfig {
    pub fn new() -> Self {
        let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is not set");
        let app_id = std::env::var("DISCORD_APP_ID").expect("DISCORD_APP_ID is not set");
        Self {
            base_api_url: String::from("https://discord.com/api/v10"),
            token: token.clone(),
            app_id: app_id.clone(),
            user_agent: String::from("User-Agent: DiscordBot(www.rikka-space.com, 10)"),
            content_type: String::from("Content-Type: application/json"),
            auth: format!("Authorization: Bot {}", token),
        }
    }
}

pub async fn get_word_valid(word: &str) -> bool {
    let base_url = "https://api.dictionaryapi.dev/api/v2/entries/en/";
    let target_url = base_url.to_owned() + word;

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