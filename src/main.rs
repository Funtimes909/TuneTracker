mod services;

use std::io::Write;

use clap::{Parser, ValueEnum};
use futures::StreamExt;
use rspotify::prelude::BaseClient;
use rspotify_model::{PlayableItem, PlaylistId};
use services::{spotify, subsonic};
use submarine::Client;

use services::{Track, TrackSource, search, subsonic::get_song};

use crate::services::subsonic::add_songs_to_favorites;

#[derive(Parser)]
#[command(name = "TuneTracker")]
struct Args {
    #[clap(long, help = "Id of the playlist to import")]
    playlist: String,
    #[clap(long, default_value_t, value_enum, help = "Whether to add songs to a new playlist or add them to favorited songs")]
    destination: TrackDestination,
    #[clap(long, help = "Spotify client id")]
    client_id: String,
    #[clap(long, help = "Spotify client secret")]
    client_secret: String,
    #[clap(long, help = "URL of the subsonic server")]
    subsonic_url: String,
    #[clap(long, help = "Username of the user on the subsonic server")]
    subsonic_user: String,
    #[clap(long, help = "Password for the user account")]
    subsonic_password: String,
}

#[derive(Default, Clone, PartialEq, ValueEnum)]
enum TrackDestination {
    #[default]
    Playlist,
    Favorites,
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

    let spotify_playlist = match spotify_client.playlist(playlist_id, None, None).await {
        Ok(playlist) => playlist,
        Err(e) => panic!("{e}"),
    };

    println!("{BOLD}{GREEN}=== Importing Playlist ==={RESET}");
    println!("Name: {}", spotify_playlist.name);
    println!("Total Tracks: {}", spotify_playlist.tracks.total);

    let mut spotify_tracks = Vec::new();

    // Turn all spotify tracks into a Track type and add them to the collection
    for item in spotify_playlist.tracks.items {
        if let Some(PlayableItem::Track(track)) = item.track {
            // Turn source track into a Track
            if let Ok(track) = track.try_into() {
                spotify_tracks.push(track);
            }
        }
    }

    // Do a first pass to see how many tracks can be confidently matched.
    // It's important to keep the exact order of the playlist, including unmatched tracks
    // so that a later pass can use those unmatched tracks to prompt the user for input.
    let partially_matched_playlist: Vec<Track> = spotify_tracks
        .into_iter()
        .map(|track| search(track, &subsonic_tracks))
        .collect();

    let mut playlist: Vec<Track> =
        futures::stream::iter(partially_matched_playlist.into_iter().map(|track| track))
            .then(|track| {
                let client = &subsonic_client;
                async move {
                    // If the track source is spotify, it failed to match in the first pass
                    // prompt the user for input on how to handle the track.
                    // Returns the new track if it could be found and the same old track if not.
                    match track.track_source == TrackSource::Spotify {
                        true => {
                            // Separate each prompt slightly
                            println!();
                            prompt_user(&track, client).await
                        }
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
    let playlist_length = playlist.len();

    // Finally, add the songs to either a new playlist or the favorites
    if args.destination == TrackDestination::Favorites {
        match add_songs_to_favorites(&subsonic_client, playlist).await {
            Ok(_) => {
                println!();
                println!("{BOLD}{GREEN}=== Songs added! ==={RESET}");
                println!(
                    "{playlist_length}/{} Songs matched!",
                    spotify_playlist.tracks.total
                );
            }
            Err(e) => println!("Error adding songs to favorites! {e}"),
        }
    } else {
        match subsonic::create_playlist(
            &subsonic_client,
            spotify_playlist.name,
            spotify_playlist.description.unwrap_or(String::new()),
            playlist,
        )
        .await
        {
            Ok(_) => {
                println!();
                println!("{BOLD}{GREEN}=== Playlist created! ==={RESET}");
                println!(
                    "{playlist_length}/{} Songs matched!",
                    spotify_playlist.tracks.total
                );
            }
            Err(e) => println!("Error during playlist creation! {e}"),
        }
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
        return get_song(client, &id[..id.len() - 1]).await;
    }

    return None;
}
