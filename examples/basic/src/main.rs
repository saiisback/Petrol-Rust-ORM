use petrol_client::PetrolClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    let client = PetrolClient::new(&database_url).await?;
    client.ping().await?;
    println!("Connected to database ✅");
    Ok(())
}
