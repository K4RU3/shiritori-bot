mod utility;
mod commands;
mod gateway;
mod event;
mod game;

#[macro_export]
macro_rules! spawn {
    ($task:expr) => {
        tokio::spawn($task);
    };
}

#[tokio::main]
async fn main() {
    utility::verbose_log_async("Starting NS Shiritori...").await;

    match commands::register_commands().await {
        Ok(_) => println!("Commands registered successfully!"),
        Err(_) => println!("Error registering commands"),
    }

    let _ = gateway::login_bot().await;

    loop {}
}
