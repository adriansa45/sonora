use anyhow::{Context as _, Result};

#[tokio::test]
#[ignore = "reads the connected YouTube Music account"]
async fn youtube_home_is_personalized() -> Result<()> {
    use crate::MusicProvider as _;

    let session = crate::youtube::YouTubeProvider::new()
        .restore()
        .await?
        .context("YouTube Music is not signed in")?;
    let home = session.api.home().await?;
    let picks: Vec<&str> = home
        .quick_picks
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|track| track.name.as_str())
        .collect();
    println!("quick picks: {picks:?}");
    let again: Vec<&str> = home
        .listen_again
        .iter()
        .map(|track| track.name.as_str())
        .collect();
    println!("listen again: {again:?}");
    Ok(())
}
