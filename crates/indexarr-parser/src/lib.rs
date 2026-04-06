pub mod maps;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub use maps::EXTENSION_CATEGORIES;

/// Result of parsing a torrent name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedTorrent {
    pub title: String,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub seasons: Vec<i32>,
    pub episode: Option<i32>,
    pub episodes: Vec<i32>,
    pub episode_title: Option<String>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub video_source: Option<String>,
    pub modifier: Option<String>,
    pub hdr: Vec<String>,
    pub audio_codecs: Vec<String>,
    pub audio_channels: Option<String>,
    pub languages: Vec<String>,
    pub group: Option<String>,
    pub edition: Option<String>,
    pub bit_depth: Option<String>,
    pub network: Option<String>,
    pub is_3d: bool,
    pub is_proper: bool,
    pub is_repack: bool,
    pub is_complete: bool,
    pub is_dubbed: bool,
    pub is_remastered: bool,
    pub is_scene: bool,
    pub has_subtitles: bool,
    pub platform: Option<String>,
}

/// Parse a torrent name into structured metadata.
pub fn parse(name: &str) -> ParsedTorrent {
    let mut result = ParsedTorrent::default();
    let mut text = name.replace('_', " ");

    // Episode patterns (run first for clean title extraction)
    text = extract_episodes(&mut result, &text);

    // Year
    text = extract_year(&mut result, &text);

    // Resolution
    text = extract_first_match(&mut result.resolution, &text, &maps::RESOLUTION_PATTERNS);

    // Codec
    text = extract_first_match(&mut result.codec, &text, &maps::CODEC_PATTERNS);

    // Source
    text = extract_first_match(&mut result.video_source, &text, &maps::SOURCE_PATTERNS);

    // Modifier
    text = extract_first_match(&mut result.modifier, &text, &maps::MODIFIER_PATTERNS);

    // HDR (multi-value)
    text = extract_all_matches(&mut result.hdr, &text, &maps::HDR_PATTERNS);

    // Audio codecs (multi-value)
    text = extract_all_matches(&mut result.audio_codecs, &text, &maps::AUDIO_CODEC_PATTERNS);

    // Audio channels
    text = extract_first_match(&mut result.audio_channels, &text, &maps::AUDIO_CHANNEL_PATTERNS);

    // Languages (multi-value)
    text = extract_all_matches(&mut result.languages, &text, &maps::LANGUAGE_PATTERNS);

    // Bit depth
    static RE_BITDEPTH: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(12[- ]?bit|10[- ]?bit|8[- ]?bit)\b").unwrap());
    if let Some(m) = RE_BITDEPTH.find(&text) {
        let raw = m.as_str().to_lowercase().replace([' ', '-'], "");
        result.bit_depth = Some(raw);
        text = text[..m.start()].to_string() + &text[m.end()..];
    }

    // 3D
    static RE_3D: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\b3D\b").unwrap());
    result.is_3d = RE_3D.is_match(&text);

    // Flags
    result.is_proper = detect_flag(&text, r"(?i)\bPROPER\b");
    result.is_repack = detect_flag(&text, r"(?i)\b(?:REPACK|RERIP)\b");
    result.is_complete = detect_flag(&text, r"(?i)\b(?:COMPLETE|INTEGRAL)\b");
    result.is_dubbed = detect_flag(&text, r"(?i)\b(?:DUBBED|DUAL[. ]?AUDIO|MULTI)\b");
    result.is_remastered = detect_flag(&text, r"(?i)\b(?:REMASTERED|RESTORED|4K.REMASTER)\b");
    result.is_scene = detect_flag(&text, r"(?i)\b(?:SCENE|INTERNAL)\b");
    result.has_subtitles = detect_flag(&text, r"(?i)\b(?:SUBBED|SUBS?|SUBTITL|HARDSUB)\b");

    // Network
    text = extract_first_match(&mut result.network, &text, &maps::NETWORK_PATTERNS);

    // Edition
    static RE_EDITION: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(Director'?s[. ]Cut|Extended[. ](?:Cut|Edition)|Unrated|Theatrical[. ](?:Cut|Edition)|Ultimate[. ](?:Cut|Edition)|Criterion|IMAX[. ]Edition|Anniversary[. ]Edition|Collector'?s[. ]Edition|Special[. ]Edition|Deluxe[. ]Edition|Limited[. ]Edition)\b").unwrap()
    });
    if let Some(m) = RE_EDITION.find(&text) {
        result.edition = Some(m.as_str().to_string());
        text = text[..m.start()].to_string() + &text[m.end()..];
    }

    // Platform
    static RE_PLATFORM: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(PS5|PS4|PS3|PS2|PS[. ]Vita|PSP|Xbox[. ]Series|Xbox[. ]One|Xbox[. ]360|Switch|NSW|3DS|Wii[. ]?U?|PC|macOS|Mac|Linux|Android|iOS)\b").unwrap()
    });
    if let Some(m) = RE_PLATFORM.find(&text) {
        result.platform = Some(normalize_platform(m.as_str()));
        text = text[..m.start()].to_string() + &text[m.end()..];
    }

    // Release group
    extract_group(&mut result, &text);

    // Title (everything before first extracted attribute)
    extract_title(&mut result, name);

    result
}

fn extract_episodes(result: &mut ParsedTorrent, text: &str) -> String {
    let mut t = text.to_string();

    // S01E02-E10 or S01E02-10 (range — requires hyphen separator)
    static RE_EP_RANGE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bS(\d{1,4})E(\d{1,4})-E?(\d{1,4})\b").unwrap());
    if let Some(caps) = RE_EP_RANGE.captures(&t) {
        let s: i32 = caps[1].parse().unwrap_or(0);
        let e1: i32 = caps[2].parse().unwrap_or(0);
        let e2: i32 = caps[3].parse().unwrap_or(0);
        result.season = Some(s);
        result.seasons = vec![s];
        result.episodes = if e2 > e1 { (e1..=e2).collect() } else { vec![e1, e2] };
        result.episode = Some(e1);
        let m = caps.get(0).unwrap();
        t = t[..m.start()].to_string() + &t[m.end()..];
        return t;
    }

    // S01E02E03E04 (multi-episode)
    static RE_EP_MULTI: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bS(\d{1,4})((?:E\d{1,4}){2,})\b").unwrap());
    if let Some(caps) = RE_EP_MULTI.captures(&t) {
        let s: i32 = caps[1].parse().unwrap_or(0);
        result.season = Some(s);
        result.seasons = vec![s];
        static RE_ENUM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)E(\d{1,4})").unwrap());
        result.episodes = RE_ENUM.captures_iter(&caps[2])
            .filter_map(|c| c[1].parse::<i32>().ok())
            .collect();
        result.episode = result.episodes.first().copied();
        let m = caps.get(0).unwrap();
        t = t[..m.start()].to_string() + &t[m.end()..];
        return t;
    }

    // S01-S03 (season range)
    static RE_SEASON_RANGE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bS(\d{1,4})[- ]?S(\d{1,4})\b").unwrap());
    if let Some(caps) = RE_SEASON_RANGE.captures(&t) {
        let s1: i32 = caps[1].parse().unwrap_or(0);
        let s2: i32 = caps[2].parse().unwrap_or(0);
        result.season = Some(s1);
        result.seasons = (s1..=s2).collect();
        let m = caps.get(0).unwrap();
        t = t[..m.start()].to_string() + &t[m.end()..];
        return t;
    }

    // S01E02 (standard)
    static RE_SE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bS(\d{1,4})E(\d{1,4})\b").unwrap());
    if let Some(caps) = RE_SE.captures(&t) {
        result.season = Some(caps[1].parse().unwrap_or(0));
        result.episode = Some(caps[2].parse().unwrap_or(0));
        result.seasons = vec![result.season.unwrap()];
        result.episodes = vec![result.episode.unwrap()];
        let m = caps.get(0).unwrap();
        t = t[..m.start()].to_string() + &t[m.end()..];
        return t;
    }

    // S01 (season only — without episode)
    // Can't use lookahead with regex crate, so we check manually
    static RE_SEASON_ONLY: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\bS(\d{1,4})\b").unwrap());
    if let Some(caps) = RE_SEASON_ONLY.captures(&t) {
        let m = caps.get(0).unwrap();
        // Manual negative lookahead: ensure no 'E' follows
        let after = &t[m.end()..];
        if !after.starts_with('E') && !after.starts_with('e') {
            result.season = Some(caps[1].parse().unwrap_or(0));
            result.seasons = vec![result.season.unwrap()];
            t = t[..m.start()].to_string() + &t[m.end()..];
            return t;
        }
    }

    // 1x02 (archive format)
    static RE_ARCHIVE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(\d{1,2})x(\d{2,3})\b").unwrap());
    if let Some(caps) = RE_ARCHIVE.captures(&t) {
        result.season = Some(caps[1].parse().unwrap_or(0));
        result.episode = Some(caps[2].parse().unwrap_or(0));
        result.seasons = vec![result.season.unwrap()];
        result.episodes = vec![result.episode.unwrap()];
        let m = caps.get(0).unwrap();
        t = t[..m.start()].to_string() + &t[m.end()..];
        return t;
    }

    // Season N Episode M
    static RE_LONG: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\bSeason[. ](\d{1,4})[. ]Episode[. ](\d{1,4})\b").unwrap()
    });
    if let Some(caps) = RE_LONG.captures(&t) {
        result.season = Some(caps[1].parse().unwrap_or(0));
        result.episode = Some(caps[2].parse().unwrap_or(0));
        result.seasons = vec![result.season.unwrap()];
        result.episodes = vec![result.episode.unwrap()];
        let m = caps.get(0).unwrap();
        t = t[..m.start()].to_string() + &t[m.end()..];
        return t;
    }

    // Part N
    static RE_PART: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?i)\b(?:Part|Pt)[. ]?(\d{1,3})\b").unwrap());
    if let Some(caps) = RE_PART.captures(&t) {
        result.episode = Some(caps[1].parse().unwrap_or(0));
        result.episodes = vec![result.episode.unwrap()];
        let m = caps.get(0).unwrap();
        t = t[..m.start()].to_string() + &t[m.end()..];
        return t;
    }

    t
}

fn extract_year(result: &mut ParsedTorrent, text: &str) -> String {
    static RE_YEAR: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?:^|[\s.(])((?:19|20)\d{2})(?:[\s.)\-]|$)").unwrap());
    if let Some(caps) = RE_YEAR.captures(text) {
        if let Ok(y) = caps[1].parse::<i32>() {
            if (1920..=2030).contains(&y) {
                result.year = Some(y);
                let m = caps.get(1).unwrap();
                return text[..m.start()].to_string() + &text[m.end()..];
            }
        }
    }
    text.to_string()
}

fn extract_title(result: &mut ParsedTorrent, original: &str) {
    static RE_TITLE_STOP: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)(?:(?:19|20)\d{2}|S\d{1,4}(?:E\d{1,4})?|\d{3,4}p|(?:H[. ]?26[45]|x26[45]|HEVC|AVC|XviD)|(?:BluRay|WEB[- ]?DL|WEBRip|HDTV|DVDRip|CAM)|(?:REMUX|PROPER|REPACK))").unwrap()
    });

    let text = original.replace('_', " ");
    let end_pos = RE_TITLE_STOP.find(&text).map(|m| m.start()).unwrap_or(text.len());
    let title = text[..end_pos]
        .replace('.', " ")
        .replace('-', " ")
        .trim()
        .trim_end_matches(" -")
        .trim_end_matches('-')
        .trim()
        .to_string();

    result.title = if title.is_empty() { original.to_string() } else { title };
}

fn extract_group(result: &mut ParsedTorrent, text: &str) {
    static RE_GROUP: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"-([A-Za-z0-9_]{2,20})(?:\[.*\])?$").unwrap());
    static KNOWN_TAGS: &[&str] = &[
        "720p", "1080p", "2160p", "4320p", "BluRay", "WEB", "HDTV", "DVDRip",
        "REMUX", "PROPER", "REPACK", "EXTENDED", "UNRATED", "IMAX",
        "H264", "H265", "x264", "x265", "HEVC", "AVC", "XviD", "AV1",
        "DTS", "AAC", "FLAC", "AC3", "Atmos", "TrueHD", "DD",
        "HDR", "HDR10", "DV", "DoVi", "HLG", "SDR",
    ];
    if let Some(caps) = RE_GROUP.captures(text) {
        let group = caps[1].to_string();
        if !KNOWN_TAGS.iter().any(|t| t.eq_ignore_ascii_case(&group)) {
            result.group = Some(group);
        }
    }
}

fn extract_first_match(target: &mut Option<String>, text: &str, patterns: &[(Regex, &str)]) -> String {
    let mut t = text.to_string();
    for (re, value) in patterns {
        if let Some(m) = re.find(&t) {
            *target = Some(value.to_string());
            t = t[..m.start()].to_string() + &t[m.end()..];
            return t;
        }
    }
    t
}

fn extract_all_matches(target: &mut Vec<String>, text: &str, patterns: &[(Regex, &str)]) -> String {
    let mut t = text.to_string();
    for (re, value) in patterns {
        if let Some(m) = re.find(&t) {
            let v = value.to_string();
            if !target.contains(&v) {
                target.push(v);
            }
            t = t[..m.start()].to_string() + &t[m.end()..];
        }
    }
    t
}

fn detect_flag(text: &str, pattern: &str) -> bool {
    Regex::new(pattern).map(|re| re.is_match(text)).unwrap_or(false)
}

fn normalize_platform(raw: &str) -> String {
    let lower = raw.to_lowercase().replace(['.', ' '], "");
    match lower.as_str() {
        "ps5" => "PS5", "ps4" => "PS4", "ps3" => "PS3", "ps2" => "PS2",
        "psvita" => "PS Vita", "psp" => "PSP",
        "xboxseries" => "Xbox Series", "xboxone" => "Xbox One", "xbox360" => "Xbox 360",
        "switch" | "nsw" => "Switch", "3ds" => "3DS",
        "wiiu" => "Wii U", "wii" => "Wii",
        "pc" => "PC", "macos" | "mac" => "macOS",
        "linux" => "Linux", "android" => "Android", "ios" => "iOS",
        _ => return raw.to_string(),
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_basic() {
        let p = parse("The.Matrix.1999.1080p.BluRay.x265-GROUP");
        assert_eq!(p.title, "The Matrix");
        assert_eq!(p.year, Some(1999));
        assert_eq!(p.resolution.as_deref(), Some("1080p"));
        assert_eq!(p.video_source.as_deref(), Some("BluRay"));
        assert_eq!(p.codec.as_deref(), Some("H.265"));
        assert_eq!(p.group.as_deref(), Some("GROUP"));
    }

    #[test]
    fn test_tv_episode() {
        let p = parse("Breaking.Bad.S05E16.720p.BluRay.x264-DEMAND");
        assert_eq!(p.title, "Breaking Bad");
        assert_eq!(p.season, Some(5));
        assert_eq!(p.episode, Some(16));
        assert_eq!(p.resolution.as_deref(), Some("720p"));
    }

    #[test]
    fn test_multi_episode() {
        let p = parse("Show.S01E01E02E03.1080p.WEB-DL");
        assert_eq!(p.season, Some(1));
        assert_eq!(p.episodes, vec![1, 2, 3]);
    }

    #[test]
    fn test_episode_range() {
        let p = parse("Show.S02E05-E10.720p.HDTV");
        assert_eq!(p.season, Some(2));
        assert_eq!(p.episodes, vec![5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_4k_remux_hdr() {
        let p = parse("Movie.2023.2160p.UHD.BluRay.REMUX.DV.HDR10.TrueHD.Atmos.7.1-FraMeSToR");
        assert_eq!(p.resolution.as_deref(), Some("2160p"));
        assert_eq!(p.video_source.as_deref(), Some("BluRay"));
        assert_eq!(p.modifier.as_deref(), Some("REMUX"));
        assert!(p.hdr.contains(&"DV".to_string()));
        assert!(p.hdr.contains(&"HDR10".to_string()));
        assert!(p.audio_codecs.contains(&"TrueHD".to_string()));
        assert!(p.audio_codecs.contains(&"Atmos".to_string()));
        assert_eq!(p.audio_channels.as_deref(), Some("7.1"));
    }
}
