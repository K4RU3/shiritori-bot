pub fn create_message(message: &str) -> String {
    format!(
        r#"{{"content":"{}", "tts": false}}"#,
        message
    )
}