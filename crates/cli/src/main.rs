mod cli;
mod commands;
mod logging;

#[tokio::main]
async fn main() -> std::io::Result<()> { cli::run().await }
