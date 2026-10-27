use frame_agent_connectors::runtime::{AgentRuntime, RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AgentRuntime::new(RuntimeConfig::default()).await?;
    println!("{}", runtime.capabilities());
    runtime.dispose().await?;
    Ok(())
}
