mod utility;
mod commands;

#[tokio::main]
async fn main() {
    let config = utility::BotConfig::new();

    commands::register_commands(&config).await;

    println!("Hello, world!");
}
