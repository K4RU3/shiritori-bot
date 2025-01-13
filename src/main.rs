mod utility;
mod commands;
mod gateway;
mod event;
mod game;

#[tokio::main]
async fn main() {
    match commands::register_commands().await {
        Ok(_) => println!("Commands registered successfully!"),
        Err(_) => println!("Error registering commands"),
    }

    let _ = gateway::login_bot().await;

    loop {}
}
