use anyhow::Result;
use clap::{Parser, Subcommand};

use aptosbb::AptosBB;
use aptosbb::pentest::run_pentest;

#[derive(Parser)]
#[clap(name = "aptosbb")]
#[clap(about = "Aptos Bug Bounty pentesting tool", long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Default, /// Use default mainnet connection (rate limited)
    Api,     /// Use API key (https://geomi.dev/) from APTOSBB_KEY environment variable for higher rate limits
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Default => {
            println!("🚀 Starting AptosBB in default mode (rate limited)...");
            println!("⚠️  Using anonymous connection - may hit rate limits");
            
            let aptosbb = AptosBB::from_mainnet_latest().await?;
            println!("✅ Connected to mainnet successfully!");
            
            // Run pentest with remote state
            println!("\n🧪 Running pentest with remote mainnet state...\n");
            run_pentest(aptosbb)?;
            
            println!("\n✅ Complete!");
        }
        
        Commands::Api => {
            println!("🚀 Starting AptosBB in API mode...");
            
            // Get API key from environment variable
            let api_key = std::env::var("APTOSBB_KEY")
                .map_err(|_| anyhow::anyhow!("APTOSBB_KEY environment variable not found. Please set it with your API key."))?;
            
            if api_key.is_empty() {
                return Err(anyhow::anyhow!("APTOSBB_KEY environment variable is empty. Please set it with your API key."));
            }
            
            println!("✅ Using API key from APTOSBB_KEY environment variable");
            
            let aptosbb = AptosBB::from_mainnet_latest_with_api_key(&api_key).await?;
            println!("✅ Connected to mainnet successfully!");
            
            // Run pentest with remote state
            println!("\n🧪 Running pentest with remote mainnet state...\n");
            run_pentest(aptosbb)?;
            
            println!("\n✅ Complete!");
        }
    }
    
    Ok(())
}