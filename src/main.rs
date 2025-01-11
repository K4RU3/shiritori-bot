mod utility;
mod commands;
mod gateway;
mod event;

#[tokio::main]
async fn main() {
    let config = utility::BotConfig::new();

    match commands::register_commands(&config).await {
        Ok(_) => println!("Commands registered successfully!"),
        Err(_) => println!("Error registering commands"),
    }

    let _ = gateway::login_bot(&config).await;
}
