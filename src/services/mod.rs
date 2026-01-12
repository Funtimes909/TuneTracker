pub mod spotify;
pub mod subsonic;

use rspotify_model::FullTrack;
use submarine::data::Child;

#[derive(Debug, PartialEq, Eq)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: i32,
    pub track_number: u32,
    pub disc_number: u32,
    pub year: i32,
    pub id: String,
}

#[derive(Debug)]
#[allow(unused)]
struct Match {
    title: bool,
    artist: bool,
    album: bool,
    duration: bool,
    track_number: bool,
    multi_disc_album: bool,
    year: bool,
}

#[allow(unused)]
impl Track {
    pub fn match_tracks(track1: &Self, track2: &Self) -> bool {
        let mut match_percent = 0;
        let mut track_matching = false;
        let mut album_matching = false;

        let multi_disc_album = track1.disc_number > 1;

        if track1.artist.to_lowercase() == track2.artist.to_lowercase() {
            match_percent += 20
        }

        if track1.title.to_lowercase() == track2.title.to_lowercase()
            || track1.title.contains(&track2.title)
        {
            track_matching = true;
            match_percent += 20
        }

        if track1.album.to_lowercase() == track2.album.to_lowercase()
            // If spotify album has a "(Remaster)" tag at the end of the album name
            // This will match that on navidrome
            || track1.album.contains(&track2.album)
        {
            album_matching = true;
            match_percent += 10
        }

        if track1.year == track2.year {
            match_percent += 10
        }

        // Account for 1-3 seconds of variation in track duration
        if ((track1.duration - track2.duration).abs()) <= 3 {
            // If the duration is an exact match. rate it higher
            if track1.duration == track2.duration {
                match_percent += 20
            } else {
                match_percent += 10;
            }
        }

        // Spotify resets the track number for each disc, meaning the track_number
        // is unreliable unless it's not a multi-disc album
        if track1.disc_number == 1 && track1.track_number == track2.track_number {
            match_percent += 20
        }

        // Debugging
        // let status = Match {
        //     title: track_matching,
        //     artist: track1.artist.to_lowercase() == track2.artist.to_lowercase(),
        //     album: album_matching,
        //     duration: ((track1.duration - track2.duration).abs()) <= 3,
        //     track_number: track1.track_number == track2.track_number,
        //     multi_disc_album: multi_disc_album,
        //     year: track1.year == track2.year,
        // };

        if match_percent >= 70 {
            // println!("Match status for {}", track1.title);
            // println!("{status:?}");
            // println!(
            //     "[{match_percent}] Found song with more than 70 match: {} by {} matches {} by {}",
            //     track1.title, track1.artist, track2.title, track2.artist
            // );
            true
        } else {
            false
        }
    }
}

// Find matches from a spotify track and a collection of tracks
pub fn search(track: Track, collection: &[Track]) -> Vec<String> {
    collection
        .iter()
        .filter(|t| Track::match_tracks(&track, t))
        .map(|t| t.id.clone())
        .collect()
}

/// Spotify
impl TryFrom<FullTrack> for Track {
    type Error = ();
    fn try_from(track: FullTrack) -> Result<Self, ()> {
        // Get release year
        let release_year: i32 = match track.album.release_date {
            Some(release_date) => match release_date.split_once('-') {
                Some((year, _)) => year.parse().unwrap_or(0),
                _ => Default::default(),
            },
            _ => 0,
        };

        Ok(Self {
            title: track.name,
            artist: track.artists.first().ok_or(())?.name.to_string(),
            album: track.album.name,
            duration: track.duration.as_seconds_f64() as i32,
            track_number: track.track_number,
            disc_number: track.disc_number as u32,
            year: release_year,
            // Spotify song id, needed for downloading the
            // song using external tools in the future
            id: track.id.ok_or(())?.to_string(),
        })
    }
}

/// Subsonic
impl TryFrom<Child> for Track {
    type Error = ();
    fn try_from(track: Child) -> Result<Self, ()> {
        Ok(Self {
            title: track.title,
            artist: track.artist.ok_or(())?,
            album: track.album.ok_or(())?,
            duration: track.duration.ok_or(())?,
            track_number: track.track.ok_or(())? as u32,
            disc_number: track.disc_number.unwrap_or(0) as u32,
            year: track.year.ok_or(())?,
            // Subsonic song ID, needed for adding
            // song to a playlist
            id: track.id,
        })
    }
}
