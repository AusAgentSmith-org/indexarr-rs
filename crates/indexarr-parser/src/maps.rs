use once_cell::sync::Lazy;
use regex::Regex;

fn pat(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap()
}

// --- Resolution ---

pub static RESOLUTION_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\b4320p\b"), "4320p"),
    (pat(r"(?i)\b2160p\b"), "2160p"),
    (pat(r"(?i)\b4K\b"), "2160p"),
    (pat(r"(?i)\bUHD\b"), "2160p"),
    (pat(r"(?i)\b1440p\b"), "1440p"),
    (pat(r"(?i)\b1080p\b"), "1080p"),
    (pat(r"(?i)\bFHD\b"), "1080p"),
    (pat(r"(?i)\b720p\b"), "720p"),
    // Note: bare "HD" omitted — too ambiguous (matches HDR, HDTV fragments)
    (pat(r"(?i)\b576p\b"), "576p"),
    (pat(r"(?i)\b480p\b"), "480p"),
    // Note: bare "SD" omitted — too ambiguous (matches SDR)
    (pat(r"(?i)\b360p\b"), "360p"),
]);

// --- Codec ---

pub static CODEC_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\b(?:H[. ]?265|x265|HEVC)\b"), "H.265"),
    (pat(r"(?i)\b(?:H[. ]?264|x264|AVC)\b"), "H.264"),
    (pat(r"(?i)\bXviD\b"), "XviD"),
    (pat(r"(?i)\bDivX\b"), "DivX"),
    (pat(r"(?i)\bMPEG[- ]?2\b"), "MPEG-2"),
    (pat(r"(?i)\bVP9\b"), "VP9"),
    (pat(r"(?i)\bAV1\b"), "AV1"),
    (pat(r"(?i)\bVC[- ]?1\b"), "VC-1"),
]);

// --- Source ---

pub static SOURCE_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\b(?:BluRay|BDRip|BRRip|BD)\b"), "BluRay"),
    (pat(r"(?i)\b(?:WEB[- ]?DL|WEBDL)\b"), "WEB-DL"),
    (pat(r"(?i)\b(?:WEB[- ]?Rip|WEBRip)\b"), "WEBRip"),
    (pat(r"(?i)\b(?:HDTV|PDTV)\b"), "HDTV"),
    (pat(r"(?i)\bDVDRip\b"), "DVDRip"),
    (pat(r"(?i)\bDVD[- ]?R\b"), "DVD-R"),
    (pat(r"(?i)\b(?:DVDScr|SCR)\b"), "SCR"),
    (pat(r"(?i)\b(?:CAM|CAMRip|HDCAM)\b"), "CAM"),
    (pat(r"(?i)\b(?:TS|TeleSYNC|HDTS)\b"), "TS"),
    (pat(r"(?i)\b(?:TC|TeleCine)\b"), "TC"),
    (pat(r"(?i)\bR5\b"), "R5"),
    (pat(r"(?i)\bPPV\b"), "PPV"),
    (pat(r"(?i)\bSATRip\b"), "SATRip"),
]);

// --- Modifier ---

pub static MODIFIER_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\bREMUX\b"), "REMUX"),
    (pat(r"(?i)\bPROPER\b"), "PROPER"),
    (pat(r"(?i)\b(?:REPACK|RERIP)\b"), "REPACK"),
    (pat(r"(?i)\bEXTENDED\b"), "EXTENDED"),
    (pat(r"(?i)\bDIRECTOR'?S[. ]CUT\b"), "DIRECTORS.CUT"),
    (pat(r"(?i)\bUNRATED\b"), "UNRATED"),
    (pat(r"(?i)\bTHEATRICAL\b"), "THEATRICAL"),
    (pat(r"(?i)\bIMAX\b"), "IMAX"),
]);

// --- HDR (multi-value — order matters for priority) ---

pub static HDR_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\b(?:Dolby[. ]?Vision|DoVi|DV)\b"), "DV"),
    (pat(r"(?i)\bHDR10\+\b"), "HDR10+"),
    (pat(r"(?i)\bHDR10\b"), "HDR10"),
    (pat(r"(?i)\bHDR\b"), "HDR"),
    (pat(r"(?i)\bHLG\b"), "HLG"),
    (pat(r"(?i)\bSDR\b"), "SDR"),
]);

// --- Audio Codec (multi-value — order matters) ---

pub static AUDIO_CODEC_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\bDTS[- ]?HD[. ]?MA\b"), "DTS-HD MA"),
    (pat(r"(?i)\bDTS[- ]?X\b"), "DTS:X"),
    (pat(r"(?i)\bDTS[- ]?HD\b"), "DTS-HD"),
    (pat(r"(?i)\bTrueHD\b"), "TrueHD"),
    (pat(r"(?i)\bAtmos\b"), "Atmos"),
    (pat(r"(?i)\b(?:DD[P+][. ]?5[. ]1|DDP5[. ]1|E-?AC-?3[. ]?5[. ]1)\b"), "DD+ 5.1"),
    (pat(r"(?i)\b(?:DD[. ]?7[. ]1)\b"), "DD 7.1"),
    (pat(r"(?i)\b(?:DDP|DD[P+]|Dolby[. ]Digital[. ]Plus)\b"), "DD+"),
    (pat(r"(?i)\bDTS\b"), "DTS"),
    (pat(r"(?i)\b(?:E-?AC-?3|EAC3)\b"), "EAC3"),
    (pat(r"(?i)\b(?:AC-?3|AC3)\b"), "AC3"),
    (pat(r"(?i)\b(?:DD[. ]?5[. ]1)\b"), "DD 5.1"),
    (pat(r"(?i)\b(?:DD|Dolby[. ]Digital)\b"), "DD"),
    (pat(r"(?i)\bAAC\b"), "AAC"),
    (pat(r"(?i)\bFLAC\b"), "FLAC"),
    (pat(r"(?i)\bMP3\b"), "MP3"),
    (pat(r"(?i)\bOpus\b"), "Opus"),
    (pat(r"(?i)\bPCM\b"), "PCM"),
    (pat(r"(?i)\bLPCM\b"), "LPCM"),
]);

// --- Audio Channels ---

pub static AUDIO_CHANNEL_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\b7[. ]1\b"), "7.1"),
    (pat(r"(?i)\b5[. ]1\b"), "5.1"),
    (pat(r"(?i)\b(?:2[. ]0|stereo)\b"), "2.0"),
    (pat(r"(?i)\bmono\b"), "1.0"),
]);

// --- Language ---

pub static LANGUAGE_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\b(?:English|ENG)\b"), "English"),
    (pat(r"(?i)\b(?:French|Fran[çc]ais|FRE|FRA)\b"), "French"),
    (pat(r"(?i)\b(?:Spanish|Espa[ñn]ol|SPA|ESP)\b"), "Spanish"),
    (pat(r"(?i)\b(?:German|Deutsch|GER|DEU)\b"), "German"),
    (pat(r"(?i)\b(?:Italian|Italiano|ITA)\b"), "Italian"),
    (pat(r"(?i)\b(?:Portuguese|Portugu[eê]s|POR)\b"), "Portuguese"),
    (pat(r"(?i)\b(?:Russian|Русский|RUS)\b"), "Russian"),
    (pat(r"(?i)\b(?:Japanese|日本語|JPN|JAP)\b"), "Japanese"),
    (pat(r"(?i)\b(?:Chinese|中文|CHI|ZHO)\b"), "Chinese"),
    (pat(r"(?i)\b(?:Korean|한국어|KOR)\b"), "Korean"),
    (pat(r"(?i)\b(?:Arabic|العربية|ARA)\b"), "Arabic"),
    (pat(r"(?i)\b(?:Hindi|हिन्दी|HIN)\b"), "Hindi"),
    (pat(r"(?i)\b(?:Turkish|Türkçe|TUR)\b"), "Turkish"),
    (pat(r"(?i)\b(?:Polish|Polski|POL)\b"), "Polish"),
    (pat(r"(?i)\b(?:Dutch|Nederlands|NLD|DUT)\b"), "Dutch"),
    (pat(r"(?i)\b(?:Swedish|Svenska|SWE)\b"), "Swedish"),
    (pat(r"(?i)\b(?:Norwegian|Norsk|NOR)\b"), "Norwegian"),
    (pat(r"(?i)\b(?:Danish|Dansk|DAN)\b"), "Danish"),
    (pat(r"(?i)\b(?:Finnish|Suomi|FIN)\b"), "Finnish"),
    (pat(r"(?i)\b(?:Czech|Český|CZE|CES)\b"), "Czech"),
    (pat(r"(?i)\b(?:Hungarian|Magyar|HUN)\b"), "Hungarian"),
    (pat(r"(?i)\b(?:Romanian|Română|ROU|RUM)\b"), "Romanian"),
    (pat(r"(?i)\b(?:Greek|Ελληνικά|GRE|ELL)\b"), "Greek"),
    (pat(r"(?i)\b(?:Hebrew|עברית|HEB)\b"), "Hebrew"),
    (pat(r"(?i)\b(?:Thai|ไทย|THA)\b"), "Thai"),
    (pat(r"(?i)\b(?:Vietnamese|Tiếng Việt|VIE)\b"), "Vietnamese"),
    (pat(r"(?i)\b(?:Indonesian|Bahasa|IND)\b"), "Indonesian"),
    (pat(r"(?i)\b(?:Malay|MAL|MSA)\b"), "Malay"),
    (pat(r"(?i)\b(?:Ukrainian|Українська|UKR)\b"), "Ukrainian"),
    (pat(r"(?i)\b(?:Bulgarian|Български|BUL)\b"), "Bulgarian"),
    (pat(r"(?i)\b(?:Croatian|Hrvatski|HRV)\b"), "Croatian"),
    (pat(r"(?i)\b(?:Serbian|Српски|SRP)\b"), "Serbian"),
    (pat(r"(?i)\b(?:Slovak|Slovenský|SLO|SLK)\b"), "Slovak"),
    (pat(r"(?i)\b(?:Slovenian|Slovenski|SLV)\b"), "Slovenian"),
    (pat(r"(?i)\b(?:Estonian|Eesti|EST)\b"), "Estonian"),
    (pat(r"(?i)\b(?:Latvian|Latviešu|LAV)\b"), "Latvian"),
    (pat(r"(?i)\b(?:Lithuanian|Lietuvių|LIT)\b"), "Lithuanian"),
    (pat(r"(?i)\b(?:Catalan|Català|CAT)\b"), "Catalan"),
    (pat(r"(?i)\b(?:Icelandic|Íslenska|ICE|ISL)\b"), "Icelandic"),
    (pat(r"(?i)\bMULTi\b"), "Multi"),
]);

// --- Network / Streaming Service ---

pub static NETWORK_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| vec![
    (pat(r"(?i)\bAMZN\b"), "AMZN"),
    (pat(r"(?i)\bATVP\b"), "ATVP"),
    (pat(r"(?i)\bDSNP\b"), "DSNP"),
    (pat(r"(?i)\bHMAX\b"), "HMAX"),
    (pat(r"(?i)\bPCOK\b"), "PCOK"),
    (pat(r"(?i)\bPMTP\b"), "PMTP"),
    (pat(r"(?i)\bNF\b"), "NF"),
    (pat(r"(?i)\bHULU\b"), "HULU"),
    (pat(r"(?i)\bHBO\b"), "HBO"),
    (pat(r"(?i)\bCBS\b"), "CBS"),
    (pat(r"(?i)\bNBC\b"), "NBC"),
    (pat(r"(?i)\bAMC\b"), "AMC"),
    (pat(r"(?i)\bPBS\b"), "PBS"),
    (pat(r"(?i)\bNICK\b"), "NICK"),
    (pat(r"(?i)\bSTAN\b"), "STAN"),
    (pat(r"(?i)\bCRAV\b"), "CRAV"),
    (pat(r"(?i)\bCR\b"), "CR"),
]);

// --- File Extension Categories ---

pub static EXTENSION_CATEGORIES: &[(&str, &[&str])] = &[
    ("video", &["mkv", "avi", "mp4", "m4v", "wmv", "ts", "mov", "flv", "webm", "mpg", "mpeg", "m2ts", "vob", "ogv", "divx", "rmvb"]),
    ("audio", &["mp3", "flac", "aac", "ogg", "wma", "wav", "m4a", "opus", "ape", "alac", "dsf", "dff"]),
    ("ebook", &["epub", "mobi", "azw3", "pdf", "djvu", "fb2", "lit"]),
    ("comic", &["cbr", "cbz", "cb7"]),
    ("subtitle", &["srt", "sub", "ass", "ssa", "vtt", "idx"]),
    ("software", &["exe", "msi", "dmg", "iso", "img", "apk", "deb", "rpm"]),
    ("archive", &["zip", "rar", "7z", "tar", "gz", "bz2", "xz"]),
    ("image", &["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "svg"]),
    ("nfo", &["nfo", "txt"]),
];

/// Get the category for a file extension.
pub fn extension_category(ext: &str) -> Option<&'static str> {
    let ext_lower = ext.to_lowercase();
    let ext_clean = ext_lower.trim_start_matches('.');
    for (category, extensions) in EXTENSION_CATEGORIES {
        if extensions.contains(&ext_clean) {
            return Some(category);
        }
    }
    None
}
