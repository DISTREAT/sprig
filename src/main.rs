use anyhow::Result;
use clap::Parser;
use dialoguer::Password;
use std::path::PathBuf;

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
    }

    #[derive(Subcommand)]
    pub enum WalletCommand {
        /// Generate a new wallet
        Generate,
    }
}

fn main() -> Result<()> {
    let arguments = cli::Arguments::parse();

    match &arguments.subcommand {
        cli::Command::Wallet {
            wallet_path,
            subcommand,
        } => match &subcommand {
            cli::WalletCommand::Generate => {
                let path = PathBuf::from(wallet_path);
                let password = Password::new()
                    .with_prompt("New Password")
                    .with_confirmation("Confirm password", "Passwords mismatching")
                    .interact()
                    .unwrap();

                wallet::generate_wallet(&path, &password)?;
            }
        },
    }

    Ok(())
}
