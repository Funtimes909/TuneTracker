mod services;

use clap::{Parser, arg};
use futures::{TryStreamExt, pin_mut};
use rspotify::prelude::BaseClient;
use rspotify_model::{PlayableItem, PlaylistId};
use services::{spotify, subsonic};

use crate::services::{Track, TrackSource, search};

#[derive(Parser)]
#[command(name = "TuneTracker")]
struct Args {
    #[arg(long)]
    playlist: String,

    #[arg(long)]
    playlist_name: String,

    #[arg(long)]
    playlist_description: String,

    #[arg(long)]
    client_id: String,

    #[arg(long)]
    client_secret: String,

    #[arg(long)]
    subsonic_url: String,

    #[arg(long)]
    subsonic_user: String,

    #[arg(long)]
    subsonic_password: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let spotify_client = spotify::login_spotify(args.client_id, args.client_secret).await;

    let subsonic_client = subsonic::login_subsonic(
        args.subsonic_url,
        args.subsonic_user,
        args.subsonic_password,
    );

    let subsonic_tracks = subsonic::fetch_subsonic_songs(&subsonic_client).await;

    let id = match PlaylistId::from_id_or_uri(&args.playlist) {
        Ok(id) => id,
        Err(e) => {
            println!("Error converting playlist to ID {}", e);
            std::process::exit(1);
        }
    };

    let stream = spotify_client.playlist_items(id, None, None);
    let mut spotify_tracks: Vec<Track> = Vec::new();
    pin_mut!(stream);

    while let Ok(Some(item)) = stream.try_next().await {
        if let Some(PlayableItem::Track(track)) = item.track
            && let Ok(track) = track.try_into()
        {
            spotify_tracks.push(track);
        }
    }

    let playlist = spotify_tracks
        .into_iter()
        .map(|t| search(t, &subsonic_tracks))
        .filter(|t| t.source == TrackSource::Subsonic)
        .collect();

    match subsonic::create_playlist(
        &subsonic_client,
        args.playlist_name,
        args.playlist_description,
        playlist,
    )
    .await
    {
        Ok(_) => println!("Playlist created!"),
        Err(e) => println!("Error during playlist creation! {e}"),
    }
}
