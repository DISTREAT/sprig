use anyhow::Result;
use clap::Parser;
use dialoguer::Password;
use std::path::PathBuf;

mod blockchain;
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
        },
    }

    #[derive(Subcommand)]
    pub enum WalletCommand {
        /// Generate a new wallet
        Generate,
        /// Print hex-encoded secret key to stdout
        Dump,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // TODO: https://github.com/libp2p/rust-libp2p/blob/master/examples/ping/src/main.rs
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
            };
        }
        cli::Command::Node {
            public_address,
            listen_address,
            bootstrapping_nodes,
        } => {
            node::server(
                public_address.to_owned(),
                listen_address.to_owned(),
                bootstrapping_nodes.to_owned(),
            )
            .await?
        }
    }

    Ok(())
}
