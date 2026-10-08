use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use switchbot_exporter::{
    config::{self, Config},
    server,
    switchbot::Switchbot,
};

#[derive(Parser)]
#[command(version, about, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List SwitchBot devices as JSON.
    Devices,
    /// Show a device's complete status response as JSON.
    DeviceStatus { device_id: String },
    /// Print Prometheus metrics.
    Metrics,
    /// Serve Prometheus metrics on 0.0.0.0.
    Exporter,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let directory = std::env::current_dir().context("Could not find working directory")?;
    if let Command::Exporter = cli.command {
        let port = config::server_port(&directory)?;
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        eprintln!("SwitchBot exporter listening on {}", listener.local_addr()?);
        axum::serve(listener, server::router(directory, None))
            .with_graceful_shutdown(server::shutdown_signal())
            .await?;
        return Ok(());
    }
    let mut client = Switchbot::new(Config::load(&directory)?)?;
    match cli.command {
        Command::Devices => println!("{}", serde_json::to_string(&client.fetch_devices().await?)?),
        Command::DeviceStatus { device_id } => {
            println!("{}", client.fetch_device_status(&device_id).await?)
        }
        Command::Metrics => println!("{}", client.fetch_metrics().await?.render()),
        Command::Exporter => unreachable!(),
    }
    Ok(())
}
