use anyhow::{Context as _, Result, ensure};

use crate::MusicProvider as _;
use crate::youtube::YouTubeProvider;

#[tokio::test]
#[ignore = "reads the connected YouTube Music account"]
async fn youtube_oauth_can_read_the_library() -> Result<()> {
    let session = YouTubeProvider::new()
        .restore()
        .await?
        .context("YouTube Music is not signed in")?;

    let tracks = session.api.saved_tracks(1).await?;
    ensure!(
        !tracks.is_empty(),
        "the connected account returned no liked songs"
    );
    session.api.saved_albums(1).await?;
    session.api.saved_artists(1).await?;
    session.api.playlists(1).await?;
    session.api.home().await?;
    let search = session.api.search("Daft Punk").await?;
    ensure!(
        !search.is_empty(),
        "OAuth session broke public music search"
    );
    Ok(())
}
