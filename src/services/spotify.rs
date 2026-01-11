use futures::TryStreamExt;
use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth, prelude::*, scopes};

/// Login to spotify and return the instance
pub async fn login_spotify(id: String, secret: String) -> AuthCodeSpotify {
    let credentials = Credentials::new(&id, &secret);

    let oauth = OAuth {
        redirect_uri: "http://127.0.0.1:8888/callback".to_string(),
        scopes: scopes!(
            "playlist-read-private",
            "playlist-read-collaborative"
            // "user-library-read"
        ),
        ..Default::default()
    };

    let config = Config {
        token_cached: true,
        token_refreshing: true,
        ..Default::default()
    };

    let spotify = AuthCodeSpotify::with_config(credentials, oauth, config);
    let url = spotify.get_authorize_url(false).unwrap();
    spotify.prompt_for_token(&url).await.unwrap();

    spotify
}

#[allow(unused)]
pub async fn list_playlists(client: &AuthCodeSpotify) {
    let mut stream = client.current_user_playlists();

    while let Ok(Some(playlist)) = stream.try_next().await {
        println!("{} {}", playlist.name, playlist.id);
    }
}