use crate::services::Track;
use submarine::{Client, SubsonicError, auth::AuthBuilder, data::Info};

/// Login to subsonic server
pub fn login_subsonic(url: String, user: String, pass: String) -> Client {
    let auth = AuthBuilder::new(user, "1.16.1")
        .client_name("TuneTracker")
        .hashed(&pass);

    Client::new(&url, auth)
}

/// Fetch all songs from subsonic
pub async fn fetch_subsonic_songs(client: &Client) -> Vec<Track> {
    let mut all_songs: Vec<Track> = Vec::new();
    let mut offset = 0;

    loop {
        // Providing an empty search string returns all results
        let search_results = match client
            .search3("", None, None, None, None, None, Some(offset), Some(""))
            .await
        {
            Ok(r) => r.song,
            Err(e) => {
                println!("Error while searching for songs! {}", e);
                std::process::exit(1)
            }
        };

        if !search_results.is_empty() {
            offset += 20;

            search_results
                .into_iter()
                .filter_map(|s| s.try_into().ok())
                .for_each(|song| all_songs.push(song));
        } else {
            break;
        }
    }

    all_songs
}

/// Creates the playlist and adds the song ID's of matched tracks
pub async fn create_playlist(
    client: &Client,
    name: String,
    comment: String,
    tracks: Vec<Track>,
) -> Result<Info, SubsonicError> {
    let empty_vec: Vec<String> = Vec::new();
    let playlist_id = client.create_playlist(name, empty_vec).await?.base.id;

    client
        .update_playlist(
            playlist_id,
            Some(""),
            Some(comment),
            Some(false),
            tracks.into_iter().map(|t| t.id).collect(),
            vec![],
        )
        .await
}

/// Gets a single song from subsonic using an ID
pub async fn get_song(client: &Client, id: &str) -> Option<Track> {
    client.get_song(id).await.ok().and_then(|song| song.try_into().ok())
}