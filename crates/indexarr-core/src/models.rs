use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// --- Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Movie,
    TvShow,
    Music,
    Ebook,
    Comic,
    Audiobook,
    Game,
    Software,
    Xxx,
    Unknown,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::TvShow => "tv_show",
            Self::Music => "music",
            Self::Ebook => "ebook",
            Self::Comic => "comic",
            Self::Audiobook => "audiobook",
            Self::Game => "game",
            Self::Software => "software",
            Self::Xxx => "xxx",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum VideoResolution {
    #[sqlx(rename = "360p")]
    #[serde(rename = "360p")]
    R360p,
    #[sqlx(rename = "480p")]
    #[serde(rename = "480p")]
    R480p,
    #[sqlx(rename = "576p")]
    #[serde(rename = "576p")]
    R576p,
    #[sqlx(rename = "720p")]
    #[serde(rename = "720p")]
    R720p,
    #[sqlx(rename = "1080p")]
    #[serde(rename = "1080p")]
    R1080p,
    #[sqlx(rename = "1440p")]
    #[serde(rename = "1440p")]
    R1440p,
    #[sqlx(rename = "2160p")]
    #[serde(rename = "2160p")]
    R2160p,
    #[sqlx(rename = "4320p")]
    #[serde(rename = "4320p")]
    R4320p,
}

impl std::fmt::Display for VideoResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::R360p => "360p",
            Self::R480p => "480p",
            Self::R576p => "576p",
            Self::R720p => "720p",
            Self::R1080p => "1080p",
            Self::R1440p => "1440p",
            Self::R2160p => "2160p",
            Self::R4320p => "4320p",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum VideoCodec {
    #[sqlx(rename = "H.264")]
    #[serde(rename = "H.264")]
    H264,
    #[sqlx(rename = "H.265")]
    #[serde(rename = "H.265")]
    H265,
    #[sqlx(rename = "XviD")]
    #[serde(rename = "XviD")]
    Xvid,
    #[sqlx(rename = "DivX")]
    #[serde(rename = "DivX")]
    Divx,
    #[sqlx(rename = "MPEG-2")]
    #[serde(rename = "MPEG-2")]
    Mpeg2,
    #[sqlx(rename = "VP9")]
    #[serde(rename = "VP9")]
    Vp9,
    #[sqlx(rename = "AV1")]
    #[serde(rename = "AV1")]
    Av1,
    #[sqlx(rename = "VC-1")]
    #[serde(rename = "VC-1")]
    Vc1,
}

impl std::fmt::Display for VideoCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::H264 => "H.264",
            Self::H265 => "H.265",
            Self::Xvid => "XviD",
            Self::Divx => "DivX",
            Self::Mpeg2 => "MPEG-2",
            Self::Vp9 => "VP9",
            Self::Av1 => "AV1",
            Self::Vc1 => "VC-1",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum VideoSource {
    BluRay,
    #[sqlx(rename = "WEB-DL")]
    #[serde(rename = "WEB-DL")]
    WebDl,
    WEBRip,
    HDTV,
    DVDRip,
    #[sqlx(rename = "DVD-R")]
    #[serde(rename = "DVD-R")]
    DvdR,
    CAM,
    TS,
    TC,
    SCR,
    R5,
    PPV,
    SATRip,
}

impl std::fmt::Display for VideoSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::BluRay => "BluRay",
            Self::WebDl => "WEB-DL",
            Self::WEBRip => "WEBRip",
            Self::HDTV => "HDTV",
            Self::DVDRip => "DVDRip",
            Self::DvdR => "DVD-R",
            Self::CAM => "CAM",
            Self::TS => "TS",
            Self::TC => "TC",
            Self::SCR => "SCR",
            Self::R5 => "R5",
            Self::PPV => "PPV",
            Self::SATRip => "SATRip",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum VideoModifier {
    REMUX,
    PROPER,
    REPACK,
    EXTENDED,
    #[sqlx(rename = "DIRECTORS.CUT")]
    #[serde(rename = "DIRECTORS.CUT")]
    DirectorsCut,
    UNRATED,
    THEATRICAL,
    IMAX,
}

impl std::fmt::Display for VideoModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::REMUX => "REMUX",
            Self::PROPER => "PROPER",
            Self::REPACK => "REPACK",
            Self::EXTENDED => "EXTENDED",
            Self::DirectorsCut => "DIRECTORS.CUT",
            Self::UNRATED => "UNRATED",
            Self::THEATRICAL => "THEATRICAL",
            Self::IMAX => "IMAX",
        };
        f.write_str(s)
    }
}

// --- Models ---

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Torrent {
    pub info_hash: String,
    pub name: Option<String>,
    pub size: Option<i64>,
    pub piece_length: Option<i32>,
    pub piece_count: Option<i32>,
    pub private: bool,
    pub source: String,
    pub discovered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolve_attempts: i32,
    pub no_peers: bool,
    pub announce_miss: i32,
    pub retry_after: Option<DateTime<Utc>>,
    pub observations: i32,
    pub priority: bool,
    pub seed_count: i32,
    pub peer_count: i32,
    pub scraped_at: Option<DateTime<Utc>>,
    pub trackers: Option<serde_json::Value>,
    pub announced_at: Option<DateTime<Utc>>,
    pub nfo: Option<String>,
    pub search_vector: Option<String>,
    pub epoch: i32,
    pub contributor_id: Option<String>,
    pub sync_sequence: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TorrentFile {
    pub id: i32,
    pub info_hash: String,
    pub path: String,
    pub size: i64,
    pub extension: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TorrentContent {
    pub info_hash: String,
    pub content_type: Option<String>,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub episode_title: Option<String>,
    pub group: Option<String>,
    pub language: Option<String>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub video_source: Option<String>,
    pub modifier: Option<String>,
    pub is_3d: bool,
    pub hdr: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<String>,
    pub edition: Option<String>,
    pub bit_depth: Option<String>,
    pub network: Option<String>,
    pub quality_score: Option<i32>,
    pub is_dubbed: bool,
    pub is_complete: bool,
    pub is_remastered: bool,
    pub is_scene: bool,
    pub is_proper: bool,
    pub is_repack: bool,
    pub platform: Option<String>,
    pub has_subtitles: bool,
    pub is_anime: bool,
    pub music_format: Option<String>,
    pub tmdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub tmdb_data: Option<serde_json::Value>,
    pub classified_at: Option<DateTime<Utc>>,
    pub classifier_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TorrentTag {
    pub id: i32,
    pub info_hash: String,
    pub tag: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TorrentComment {
    pub id: i32,
    pub info_hash: String,
    pub parent_id: Option<i32>,
    pub nickname: String,
    pub body: String,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TorrentVote {
    pub id: i32,
    pub info_hash: String,
    pub fingerprint: String,
    pub value: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NukeSuggestion {
    pub id: i32,
    pub info_hash: String,
    pub fingerprint: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub reviewed: bool,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SyncState {
    pub id: i32,
    pub peer_id: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sequence: i64,
    pub peer_url: Option<String>,
    pub first_seen: Option<DateTime<Utc>>,
    pub fail_count: i32,
    pub reputation: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContentBan {
    pub id: i32,
    pub pattern: String,
    pub ban_type: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TmdbCache {
    pub id: i32,
    pub tmdb_id: i32,
    pub media_type: String,
    pub data: serde_json::Value,
    pub fetched_at: DateTime<Utc>,
}
