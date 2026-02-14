pub mod spotify;
pub mod subsonic;

use rspotify_model::FullTrack;
use submarine::data::Child;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: i32,
    pub track_number: u32,
    pub disc_number: u32,
    pub year: i32,
    pub id: String,
    pub isrc: Option<String>,
    pub musicbrainz_id: Option<String>,
    pub track_source: TrackSource,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum TrackSource {
    Subsonic,
    #[default]
    Spotify,
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
    // Compares different aspects of two songs and gives a rating based on how well they match
    // A song is considered a match if the rating is 70 or higher
    pub fn match_tracks(source: &Self, target: &Self) -> bool {
        let mut score = 0;

        // 1. Album name matching
        let album_name_match = Self::string_comparisons(&source.album, &target.album);
        score += album_name_match;

        // 2. International Standard Recording Code matching
        if let (Some(source_isrc), Some(target_isrc)) = (&source.isrc, &target.isrc) {
            if source_isrc == target_isrc {
                // Compilation (Greatest Hits, etc) albums may share the same ISRC for certain tracks
                // By checking if the album name is even slightly a match, this will eliminate most false positives
                if album_name_match > 0 {
                    return true;
                }
            }
        }

        // 3. Song name matching
        score += Self::string_comparisons(&source.title, &target.title);

        // 4. Year matching
        if source.year == target.year {
            score += 10
        }

        // 5. Artist name matching
        if source.artist.to_lowercase() == target.artist.to_lowercase() {
            score += 20
        }

        // 6. Account for 1-3 seconds of variation in track duration
        if ((source.duration - target.duration).abs()) <= 3 {
            // If the duration is an exact match, rate it higher
            if source.duration == target.duration {
                score += 20
            } else {
                score += 10;
            }
        }

        // 7. Track number
        // Spotify resets the track number for each disc, meaning the track number
        // is unreliable unless it's not a multi-disc album
        if source.track_source == TrackSource::Spotify
            && !source.disc_number > 1
            && source.track_number == target.track_number
        {
            score += 20
        }

        score >= 70
    }

    // Compare two strings and return a rating on how similar they are.
    fn string_comparisons(string1: &str, string2: &str) -> i32 {
        let source = string1.to_lowercase();
        let target = string2.to_lowercase();

        // An exact match should be rated highest
        if source == target {
            return 20;
        }

        // If either source or target contain a prefix/suffix (eg. (Remaster))
        if source.contains(&target) || target.contains(&source) {
            return 10;
        }

        // No match was found
        0
    }
}

// Takes a single source track and a slice of target tracks and compares the source against
// each item of the slice. Returning the first track with the highest match, or the source
// track if no match is found.
//
// The source playlist must be recreated 1:1 even if the track doesn't match because the subsonic
// api doesn't support adding tracks at a specific index. so the songs must be added all at once
// and iterated through, adding the matches and prompting for user input for the missing songs
pub fn search(source_track: Track, collection: &[Track]) -> Track {
    for target_track in collection {
        if Track::match_tracks(&source_track, target_track) {
            return target_track.clone();
        }
    }

    source_track
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
            id: track.id.ok_or(())?.to_string(),
            isrc: track.external_ids.get("isrc").cloned(),
            musicbrainz_id: None,
            track_source: TrackSource::Spotify,
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
            id: track.id,
            isrc: track.isrc.first().cloned(),
            musicbrainz_id: track.music_brainz_id,
            track_source: TrackSource::Subsonic,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matching() {
        // Using example songs from spotify
        let spotify_songs = vec![
            Track {
                title: String::from("St. Tristan's Sword - Rough Mix"),
                artist: String::from("Led Zeppelin"),
                album: String::from("Coda (Deluxe Edition)"),
                duration: 341,
                track_number: 3,
                disc_number: 3,
                year: 1982,
                id: String::from("xxx"),
                isrc: Some(String::from("USAT21500101")),
                musicbrainz_id: None,
                track_source: TrackSource::Spotify,
            },
            Track {
                title: String::from("The Court Of The Crimson King"),
                artist: String::from("King Crimson"),
                album: String::from(
                    "In The Court Of The Crimson King (Expanded & Remastered Original Album Mix)",
                ),
                duration: 602,
                track_number: 5,
                disc_number: 1,
                year: 1969,
                id: String::from("xxx"),
                isrc: Some(String::from("GBCTX1400804")),
                musicbrainz_id: None,
                track_source: TrackSource::Spotify,
            },
            Track {
                title: String::from("Pictures Of A City"),
                artist: String::from("King Crimson"),
                album: String::from("In The Wake Of Poseidon"),
                duration: 481,
                track_number: 2,
                disc_number: 1,
                year: 1970,
                id: String::from("xxx"),
                isrc: Some(String::from("GBCTX1500265")),
                musicbrainz_id: None,
                track_source: TrackSource::Spotify,
            },
            Track {
                title: String::from("The Wanton Song - Remaster"),
                artist: String::from("Led Zeppelin"),
                album: String::from("Physical Graffiti (Remaster)"),
                duration: 248,
                track_number: 6,
                disc_number: 2,
                year: 1975,
                id: String::from("xxx"),
                isrc: Some(String::from("USAT21300975")),
                musicbrainz_id: None,
                track_source: TrackSource::Spotify,
            },
            Track {
                title: String::from("The Sky Is Fallin'"),
                artist: String::from("Queens of the Stone Age"),
                album: String::from("Songs For The Deaf"),
                duration: 376,
                track_number: 5,
                disc_number: 1,
                year: 2002,
                id: String::from("xxx"),
                isrc: Some(String::from("USIR10211296")),
                musicbrainz_id: None,
                track_source: TrackSource::Spotify,
            },
            Track {
                title: String::from("Street Spirit (Fade Out)"),
                artist: String::from("Radiohead"),
                album: String::from("The Bends"),
                duration: 253,
                track_number: 12,
                disc_number: 1,
                year: 1995,
                id: String::from("xxx"),
                isrc: Some(String::from("GBAYE9400061")),
                musicbrainz_id: None,
                track_source: TrackSource::Spotify,
            },
        ];

        // The same songs information from my personal navidrome instance
        // Tagged automatically using beets autotagger. https://github.com/beetbox/beets
        let subsonic_songs = vec![
            Track {
                title: String::from("The Sky Is Fallin'"),
                artist: String::from("Queens of the Stone Age"),
                album: String::from("Songs For The Deaf"),
                duration: 375,
                track_number: 6,
                disc_number: 1,
                year: 2002,
                id: String::from("xxx"),
                isrc: Some(String::from("USIR10211296")),
                musicbrainz_id: None,
                track_source: TrackSource::Subsonic,
            },
            Track {
                title: String::from("Pictures of a City (including 42nd at Treadmill)"),
                artist: String::from("King Crimson"),
                album: String::from("In the Wake of Poseidon"),
                duration: 482,
                track_number: 2,
                disc_number: 1,
                // This albums release date is incorrect. Subsonic/navidrome only returns the year
                // the song was remastered, not the year it was originally released.
                year: 2011,
                id: String::from("xxx"),
                isrc: Some(String::from("GBCTX9900221")),
                musicbrainz_id: None,
                track_source: TrackSource::Subsonic,
            },
            Track {
                title: String::from("The Wanton Song"),
                artist: String::from("Led Zeppelin"),
                album: String::from("Physical Graffiti"),
                duration: 249,
                track_number: 12,
                disc_number: 2,
                // Another incorrectly tagged album release year
                year: 1995,
                id: String::from("xxx"),
                isrc: Some(String::from("USAT21300975")),
                musicbrainz_id: None,
                track_source: TrackSource::Subsonic,
            },
            Track {
                title: String::from("Street Spirit"),
                artist: String::from("Radiohead"),
                album: String::from("The Bends"),
                duration: 254,
                track_number: 12,
                disc_number: 1,
                year: 1994,
                id: String::from("xxx"),
                isrc: Some(String::from("GBAYE9400061")),
                musicbrainz_id: None,
                track_source: TrackSource::Subsonic,
            },
            Track {
                title: String::from("The Court of the Crimson King"),
                artist: String::from("King Crimson"),
                album: String::from("In the Court of the Crimson King"),
                duration: 567,
                track_number: 5,
                disc_number: 1,
                year: 2019,
                id: String::from("xxx"),
                isrc: Some(String::from("B07X13ZHG9")),
                musicbrainz_id: None,
                track_source: TrackSource::Subsonic,
            },
            Track {
                title: String::from("St. Tristan’s Sword (rough mix)"),
                artist: String::from("Led Zeppelin"),
                album: String::from("Coda"),
                duration: 341,
                track_number: 19,
                disc_number: 3,
                year: 2015,
                id: String::from("xxx"),
                isrc: None,
                musicbrainz_id: None,
                track_source: TrackSource::Subsonic,
            },
        ];

        let matches: Vec<Track> = spotify_songs
            .into_iter()
            .map(|t| search(t, &subsonic_songs))
            .filter(|t| t.track_source == TrackSource::Subsonic)
            .collect();

        assert_eq!(matches.len(), 6)
    }
}
