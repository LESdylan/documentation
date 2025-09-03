use libft_docs::*;
use clap::Parser;
use anyhow::Result;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "../")]
    source: String,
    
    #[arg(short, long, default_value = "./output")]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("🔍 Scanning libft source at: {}", args.source);
    println!("📝 Generating docs to: {}", args.output);
    
    // Your documentation generation logic here
    
    Ok(())
}
