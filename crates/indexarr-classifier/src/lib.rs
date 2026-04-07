mod ranking;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use indexarr_parser::ParsedTorrent;

pub use ranking::compute_quality_score;

/// Result of content classification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub content_type: String,
    pub confidence: f64,
    pub tags: Vec<String>,
    pub banned: bool,
    pub ban_reason: Option<String>,
    pub is_anime: bool,
    pub music_format: Option<String>,
    pub platform: Option<String>,
    pub has_subtitles: bool,
}

/// File info for classification.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size: i64,
    pub extension: Option<String>,
}

/// Classify a torrent based on parsed name and file list.
pub fn classify(parsed: &ParsedTorrent, files: &[FileInfo], name: &str) -> ClassificationResult {
    let mut result = ClassificationResult {
        content_type: "unknown".to_string(),
        confidence: 0.0,
        tags: Vec::new(),
        has_subtitles: parsed.has_subtitles,
        platform: parsed.platform.clone(),
        ..Default::default()
    };

    // Build tags from parsed attributes
    if let Some(ref res) = parsed.resolution {
        result.tags.push(format!("resolution:{res}"));
    }
    if let Some(ref codec) = parsed.codec {
        result.tags.push(format!("codec:{codec}"));
    }
    if let Some(ref src) = parsed.video_source {
        result.tags.push(format!("source:{src}"));
    }
    for hdr in &parsed.hdr {
        result.tags.push(format!("hdr:{hdr}"));
    }
    for ac in &parsed.audio_codecs {
        result.tags.push(format!("audio:{ac}"));
    }
    if let Some(ref ed) = parsed.edition {
        result.tags.push(format!("edition:{ed}"));
    }
    if let Some(ref bd) = parsed.bit_depth {
        result.tags.push(format!("bitdepth:{bd}"));
    }
    if let Some(ref net) = parsed.network {
        result.tags.push(format!("network:{net}"));
    }

    // 1. XXX detection (early exit)
    if detect_xxx(name) {
        result.content_type = "xxx".to_string();
        result.confidence = 0.95;
        return result;
    }

    // 2. File extension analysis
    let dominant_type = dominant_file_type(files);

    // 3. Anime detection
    result.is_anime = detect_anime(name, parsed.group.as_deref());

    // 4. Music format detection
    result.music_format = detect_music_format(files, name);

    // 5. Classification decision tree
    let ct = classify_content(parsed, &dominant_type, name, &result);
    result.content_type = ct.0;
    result.confidence = ct.1;

    result
}

fn detect_xxx(name: &str) -> bool {
    static RE_XXX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:xxx|porn|adult|18\+|nsfw|brazzers|mofos|bangbros|naughty[. ]america|reality[. ]kings|pornhub|xvideos|xhamster|hentai[. ](?:anime|manga))\b").unwrap()
    });
    RE_XXX.is_match(name)
}

fn detect_anime(name: &str, group: Option<&str>) -> bool {
    static ANIME_GROUPS: &[&str] = &[
        "SubsPlease",
        "Erai-raws",
        "HorribleSubs",
        "Judas",
        "Tsundere-Raws",
        "NC-Raws",
        "Moozzi2",
        "EMBER",
        "ASW",
        "Cerberus",
        "SUGOI",
        "YuiSubs",
        "MTBB",
        "LostYears",
        "Commie",
    ];
    if let Some(g) = group {
        if ANIME_GROUPS.iter().any(|ag| ag.eq_ignore_ascii_case(g)) {
            return true;
        }
    }
    // Bracketed fansub group pattern: [GroupName]
    static RE_FANSUB: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\[([^\]]+)\]").unwrap());
    RE_FANSUB.is_match(name)
}

fn detect_music_format(files: &[FileInfo], name: &str) -> Option<String> {
    static LOSSLESS: &[&str] = &["flac", "alac", "wav", "ape", "pcm", "dsf", "dff"];
    static LOSSY: &[&str] = &["mp3", "aac", "ogg", "opus", "wma", "m4a"];

    // Check file extensions
    for f in files {
        if let Some(ref ext) = f.extension {
            let ext_l = ext.to_lowercase();
            if LOSSLESS.contains(&ext_l.as_str()) {
                return Some("lossless".to_string());
            }
            if LOSSY.contains(&ext_l.as_str()) {
                return Some("lossy".to_string());
            }
        }
    }

    // Check name for audio format hints
    let name_upper = name.to_uppercase();
    if name_upper.contains("FLAC") || name_upper.contains("LOSSLESS") || name_upper.contains("ALAC")
    {
        return Some("lossless".to_string());
    }
    if name_upper.contains("MP3")
        || name_upper.contains("320KBPS")
        || name_upper.contains("256KBPS")
        || name_upper.contains("V0")
    {
        return Some("lossy".to_string());
    }

    None
}

fn dominant_file_type(files: &[FileInfo]) -> Option<String> {
    if files.is_empty() {
        return None;
    }

    let mut type_sizes: std::collections::HashMap<&str, i64> = std::collections::HashMap::new();
    for f in files {
        if let Some(ref ext) = f.extension {
            if let Some(cat) = indexarr_parser::maps::extension_category(ext) {
                *type_sizes.entry(cat).or_insert(0) += f.size;
            }
        }
    }

    type_sizes
        .into_iter()
        .max_by_key(|(_, size)| *size)
        .map(|(cat, _)| cat.to_string())
}

fn classify_content(
    parsed: &ParsedTorrent,
    dominant_type: &Option<String>,
    name: &str,
    partial: &ClassificationResult,
) -> (String, f64) {
    // Game group detection
    static GAME_GROUPS: &[&str] = &[
        "CODEX",
        "PLAZA",
        "SKIDROW",
        "FitGirl",
        "DODI",
        "CPY",
        "HOODLUM",
        "EMPRESS",
        "RUNE",
        "GOG",
        "TiNYiSO",
        "TENOKE",
        "DARKSiDERS",
        "GOLDBERG",
        "SiMPLEX",
        "PROPHET",
        "RAZOR1911",
        "RELOADED",
    ];

    let group = parsed.group.as_deref().unwrap_or("");
    if GAME_GROUPS.iter().any(|g| g.eq_ignore_ascii_case(group)) {
        return ("game".to_string(), 0.9);
    }

    // By dominant file type
    if let Some(dt) = dominant_type {
        match dt.as_str() {
            "audio" => return ("music".to_string(), 0.85),
            "ebook" => return ("ebook".to_string(), 0.9),
            "comic" => return ("comic".to_string(), 0.9),
            _ => {}
        }
    }

    // Music detection: name pattern "Artist - Title" + music descriptors
    static RE_MUSIC: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)^[^/\n]{2,40}\s+-\s+[^/\n]{2,}").unwrap());
    static RE_MUSIC_DESC: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:remastered|deluxe[. ]edition|EP|Single|Album|Discography|FLAC|MP3|320kbps|V0)\b").unwrap()
    });
    if RE_MUSIC.is_match(name) && RE_MUSIC_DESC.is_match(name) {
        if partial.music_format.is_some() || dominant_type.as_deref() == Some("audio") {
            return ("music".to_string(), 0.8);
        }
    }

    // Audiobook
    static RE_AUDIOBOOK: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:audiobook|unabridged|narrated[. ]by|audible)\b").unwrap()
    });
    if RE_AUDIOBOOK.is_match(name) {
        return ("audiobook".to_string(), 0.9);
    }

    // Software / version number detection
    static RE_VERSION: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bv?\d+\.\d+(?:\.\d+)+\b").unwrap());
    static RE_SOFTWARE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:portable|installer|setup|crack|keygen|patch|serial|license|registration|activation|multilingual)\b").unwrap()
    });
    if RE_SOFTWARE.is_match(name)
        || (RE_VERSION.is_match(name) && dominant_type.as_deref() == Some("software"))
    {
        if parsed.platform.is_some()
            && !matches!(parsed.platform.as_deref(), Some("PC" | "macOS" | "Linux"))
        {
            // Console platform + software keywords → game
            return ("game".to_string(), 0.7);
        }
        return ("software".to_string(), 0.7);
    }

    // Game keywords
    static RE_GAME: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(?:v\d+\.\d+.*(?:GOG|Steam|Epic)|Update[. ]\d|DLC[. ]Pack|Build[. ]\d+|Early[. ]Access)\b").unwrap()
    });
    if RE_GAME.is_match(name) {
        return ("game".to_string(), 0.75);
    }

    // TV show (has season/episode or anime)
    if parsed.season.is_some() || partial.is_anime {
        return ("tv_show".to_string(), 0.85);
    }

    // Movie (has year + video attributes, no episode)
    if parsed.year.is_some()
        && (parsed.resolution.is_some() || parsed.codec.is_some() || parsed.video_source.is_some())
        && parsed.episode.is_none()
    {
        return ("movie".to_string(), 0.8);
    }

    // Video file dominant
    if dominant_type.as_deref() == Some("video") {
        if parsed.episode.is_some() {
            return ("tv_show".to_string(), 0.6);
        }
        return ("movie".to_string(), 0.5);
    }

    ("unknown".to_string(), 0.0)
}

/// Check a torrent name against ban rules.
pub fn check_ban(name: &str, info_hash: &str, bans: &[BanRule]) -> Option<String> {
    for ban in bans {
        if !ban.active {
            continue;
        }
        match ban.ban_type.as_str() {
            "info_hash" => {
                if ban.pattern.eq_ignore_ascii_case(info_hash) {
                    return Some(ban.reason.clone().unwrap_or_default());
                }
            }
            "keyword" => {
                if name.to_lowercase().contains(&ban.pattern.to_lowercase()) {
                    return Some(ban.reason.clone().unwrap_or_default());
                }
            }
            "regex" => {
                if let Ok(re) = Regex::new(&ban.pattern) {
                    if re.is_match(name) {
                        return Some(ban.reason.clone().unwrap_or_default());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BanRule {
    pub pattern: String,
    pub ban_type: String,
    pub reason: Option<String>,
    pub active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_parsed(name: &str) -> ParsedTorrent {
        indexarr_parser::parse(name)
    }

    #[test]
    fn test_movie_classification() {
        let p = make_parsed("The.Matrix.1999.1080p.BluRay.x265-GROUP");
        let r = classify(&p, &[], "The.Matrix.1999.1080p.BluRay.x265-GROUP");
        assert_eq!(r.content_type, "movie");
    }

    #[test]
    fn test_tv_classification() {
        let p = make_parsed("Breaking.Bad.S05E16.720p.BluRay.x264-DEMAND");
        let r = classify(&p, &[], "Breaking.Bad.S05E16.720p.BluRay.x264-DEMAND");
        assert_eq!(r.content_type, "tv_show");
    }

    #[test]
    fn test_xxx_detection() {
        let p = make_parsed("Some.XXX.Movie.2023");
        let r = classify(&p, &[], "Some.XXX.Movie.2023");
        assert_eq!(r.content_type, "xxx");
    }

    #[test]
    fn test_game_group() {
        let p = make_parsed("Cyberpunk.2077.v2.1-CODEX");
        let r = classify(&p, &[], "Cyberpunk.2077.v2.1-CODEX");
        assert_eq!(r.content_type, "game");
    }

    #[test]
    fn test_quality_score() {
        let p = make_parsed("Movie.2023.2160p.BluRay.REMUX.DV.HDR10.TrueHD.Atmos.7.1-FraMeSToR");
        let score = compute_quality_score(&p);
        assert!(
            score > 10000,
            "4K REMUX DV score should be >10000, got {score}"
        );
    }
}
