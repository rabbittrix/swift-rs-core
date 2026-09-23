#[tokio::main]
async fn main() -> anyhow::Result<()> {
    swift_rs_gateway::serve().await
}
