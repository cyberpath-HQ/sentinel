use tracing::error;

mod cli;
mod commands;
mod logging;

#[tokio::main]
async fn main() {
    if let Err(e) = cli::run().await {
        error!("{}", e);
        std::process::exit(1);
    }
}
