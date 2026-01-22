mod services;

use std::io::Write;

use clap::{Parser, arg};
use futures::{StreamExt, TryStreamExt, pin_mut};
use rspotify::prelude::BaseClient;
use rspotify_model::{PlayableItem, PlaylistId};
use services::{spotify, subsonic};
use submarine::Client;

use crate::services::{Track, TrackSource, search, subsonic::get_song};

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

// Colors
pub const YELLOW: &str = "\x1b[33m";
pub const GREEN: &str = "\x1b[32m";
pub const BOLD: &str = "\x1b[1m";
pub const RESET: &str = "\x1b[0m";

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

    let playlist_id = match PlaylistId::from_id_or_uri(&args.playlist) {
        Ok(id) => id,
        Err(e) => {
            println!("Error converting playlist to ID {}", e);
            std::process::exit(1);
        }
    };

    let playlist_stream = spotify_client.playlist_items(playlist_id, None, None);
    let mut spotify_tracks = Vec::new();

    pin_mut!(playlist_stream);
    let mut track_index = 1;

    // Turn all spotify tracks into a Track type and add them to the collection
    while let Ok(Some(item)) = playlist_stream.try_next().await {
        if let Some(PlayableItem::Track(track)) = item.track {
            // Turn source track into a Track
            if let Ok(track) = track.try_into() {
                spotify_tracks.push((track_index, track));
                track_index += 1;
            }
        }
    }

    // Do a first pass to see how many tracks can be confidently matched.
    // It's important to keep the exact order of the playlist, including unmatched tracks
    // so that a later pass can use those unmatched tracks to prompt the user for input.
    let partially_matched_playlist: Vec<(i32, Track)> = spotify_tracks
        .into_iter()
        .map(|(i, track)| (i, search(track, &subsonic_tracks)))
        .collect();

    let mut playlist: Vec<Track> = futures::stream::iter(
        partially_matched_playlist
            .into_iter()
            .map(|(_, track)| track),
    )
    .then(|track| {
        let client = &subsonic_client;
        async move {

            // If the track source is spotify, it failed to match in the first pass
            // prompt the user for input on how to handle the track.
            // Returns the new track if it could be found and the same old track if not.
            match track.track_source == TrackSource::Spotify {
                true => prompt_user(&track, client).await,
                false => Some(track),
            }
        }
    })
    .flat_map(futures::stream::iter)
    .collect()
    .await;

    // Remove all remaining unmatched tracks. Navidrome specifically has an issue with keeping
    // song index in playlists if invalid ID's are provided in the playlist creation
    playlist.retain(|track| track.track_source == TrackSource::Subsonic);

    // Finally, create the playlist
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

/// Called for every track that failed to match. Asks the user how they want to proceed. Options include:
/// - Skip the track entirely, no track will be added to the created playlist.
/// - Enter ID, the user is prompted to enter the track id from the target platform manually.
/// - Download, currently unimplemented. Unsure of how to handle this at the moment.
///
/// Returns an Option<Track>, containing Some() if a track could be resolved and None if no track could be
/// resolved from subsonic.
async fn prompt_user(missing_track: &Track, client: &Client) -> Option<Track> {
    println!("{BOLD}{YELLOW}=== Missing track! ==={RESET}");
    println!(
        "Can't find '{}' by '{}'",
        missing_track.title, missing_track.artist
    );
    println!("What would you like to do?");
    print!("[{BOLD}{GREEN}S{RESET}]kip. Enter [{BOLD}I{RESET}]d: ");

    // Flush stdout so we can read from the same line as the available options
    let _ = std::io::stdout().flush();
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read stdin!");
    // Remove trailing new line and capatalize
    let input = &input[..input.len() - 1].to_uppercase();

    // Default option
    if input.is_empty() || input.eq("S") {
        return None;
    }

    // Prompt to input tacks subsonic id
    if input.eq("I") {
        let mut id = String::new();
        print!("Please enter the subsonic ID of the track: ");
        let _ = std::io::stdout().flush();
        std::io::stdin()
            .read_line(&mut id)
            .expect("Failed to read stdin!");

        // Check for that song on subsonic
        let song = get_song(client, &id[..id.len() - 1]).await;

        return song;
    }

    return None;
}
