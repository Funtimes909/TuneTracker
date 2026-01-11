use crate::services::Track;
use submarine::{Client, auth::AuthBuilder};

/// Login to subsonic server
pub async fn login_subsonic(url: String, user: String, pass: String) -> Client {
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
                println!("Error while searching for songs! {}", e.to_string());
                std::process::exit(1)
            }
        };

        if search_results.len() != 0 {
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
pub async fn create_playlist(client: &Client, name: String, songs: Vec<impl Into<String>>) {
    if let Err(e) = client.create_playlist(&name, songs).await {
        println!("Subsonic error while creating playlist! {}", e.to_string());
        std::process::exit(1)
    }
}
