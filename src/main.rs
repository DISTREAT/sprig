use anyhow::Result;
use clap::Parser;
use dialoguer::Password;
use std::env;
use std::path::PathBuf;

mod api;
mod blockchain;
mod network;
mod node;
mod wallet;

mod cli {
    use clap_derive::{Parser, Subcommand};

    #[derive(Parser)]
    #[command(author, version, about)]
    pub struct Arguments {
        #[command(subcommand)]
        pub subcommand: Command,
    }

    #[derive(Subcommand)]
    pub enum Command {
        /// Manage your wallet and broadcast transactions
        Wallet {
            #[arg(short, long)]
            wallet_path: String,

            #[command(subcommand)]
            subcommand: WalletCommand,
        },
        /// Serve your own node serving and syncing a local copy of the blockchain
        Node {
            #[arg(short, long)]
            listen_address: Option<String>,

            #[arg(short, long)]
            public_address: String,

            #[arg(short, long)]
            bootstrapping_nodes: Vec<String>,

            #[arg(short, long)]
            api_address: Option<String>,
        },
    }

    #[derive(Subcommand)]
    pub enum WalletCommand {
        /// Generate a new wallet
        Generate,
        /// Print hex-encoded secret key to stdout
        Dump,
        /// Print the public address of a wallet
        Address,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let log_level = match env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_owned())
        .as_str()
    {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        "trace" => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    };
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(log_level)
            .finish(),
    )?;

    let arguments = cli::Arguments::parse();

    match &arguments.subcommand {
        cli::Command::Wallet {
            wallet_path,
            subcommand,
        } => {
            let path = PathBuf::from(wallet_path);

            match &subcommand {
                cli::WalletCommand::Generate => {
                    let password = Password::new()
                        .with_prompt("New Password")
                        .with_confirmation("Confirm password", "Passwords mismatching")
                        .interact()?;

                    wallet::generate_wallet(&path, &password)?;
                }
                cli::WalletCommand::Dump => {
                    let password = Password::new().with_prompt("Password").interact()?;
                    let secret_key = wallet::dump_wallet(&path, &password)?;
                    let encoded = hex::encode(secret_key);

                    println!("{encoded}");
                }
                cli::WalletCommand::Address => {
                    let password = Password::new().with_prompt("Password").interact()?;
                    let secret_key = wallet::open_wallet(&path, &password)?;
                    let encoded = hex::encode(secret_key.verifying_key().as_bytes());

                    println!("{encoded}");
                }
            };
        }
        cli::Command::Node {
            public_address,
            listen_address,
            bootstrapping_nodes,
            api_address,
        } => {
            let config = node::NodeServerConfig {
                public_address: public_address.clone(),
                listen_address: listen_address.clone(),
                bootstrapping_nodes: bootstrapping_nodes.clone(),
                api_address: api_address
                    .clone()
                    .unwrap_or_else(|| String::from("0.0.0.0:3034")),
            };

            node::serve(config).await?;
        }
    }

    Ok(())
}
