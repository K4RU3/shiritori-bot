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