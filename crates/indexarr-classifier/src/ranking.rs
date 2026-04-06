use indexarr_parser::ParsedTorrent;

/// Compute quality score from parsed torrent attributes.
/// Higher is better. Range roughly -5000 to +15000.
pub fn compute_quality_score(parsed: &ParsedTorrent) -> i32 {
    let mut score: i32 = 0;

    // Resolution
    score += match parsed.resolution.as_deref() {
        Some("4320p") => 5000,
        Some("2160p") => 4000,
        Some("1440p") => 2500,
        Some("1080p") => 2000,
        Some("720p") => 1000,
        Some("576p") => 500,
        Some("480p") => 300,
        Some("360p") => 100,
        _ => 0,
    };

    // Source
    score += match parsed.video_source.as_deref() {
        Some("BluRay") => 2000,
        Some("WEB-DL") => 1500,
        Some("WEBRip") => 1000,
        Some("HDTV") => 500,
        Some("DVDRip") => -500,
        Some("DVD-R") => -500,
        Some("SCR" | "R5") => -3000,
        Some("TC") => -4000,
        Some("TS" | "CAM") => -5000,
        _ => 0,
    };

    // Modifier
    score += match parsed.modifier.as_deref() {
        Some("REMUX") => 3000,
        Some("IMAX") => 200,
        Some("EXTENDED" | "DIRECTORS.CUT") => 100,
        Some("PROPER" | "REPACK") => 50,
        Some("UNRATED") => 50,
        _ => 0,
    };

    // Codec
    score += match parsed.codec.as_deref() {
        Some("H.265" | "AV1") => 500,
        Some("H.264") => 300,
        Some("VP9") => 200,
        Some("VC-1") => 0,
        Some("MPEG-2") => -500,
        Some("XviD" | "DivX") => -1000,
        _ => 0,
    };

    // HDR (additive)
    for hdr in &parsed.hdr {
        score += match hdr.as_str() {
            "DV" => 1500,
            "HDR10+" => 1200,
            "HDR10" => 1000,
            "HDR" => 800,
            "HLG" => 600,
            "SDR" => 0,
            _ => 0,
        };
    }

    // Audio codecs (additive)
    for ac in &parsed.audio_codecs {
        score += match ac.as_str() {
            "DTS-HD MA" | "DTS:X" | "TrueHD" => 1000,
            "Atmos" => 800,
            "PCM" | "LPCM" | "DTS-HD" => 500,
            "FLAC" => 400,
            "DTS" => 300,
            "EAC3" => 300,
            "DD+" | "DD+ 5.1" => 250,
            "DD 7.1" => 250,
            "Opus" => 200,
            "DD 5.1" => 200,
            "AC3" | "DD" => 100,
            "AAC" => 50,
            "MP3" => -200,
            _ => 0,
        };
    }

    // Audio channels
    score += match parsed.audio_channels.as_deref() {
        Some("7.1") => 200,
        Some("5.1") => 100,
        Some("2.0") => 0,
        Some("1.0") => -100,
        _ => 0,
    };

    // Bit depth
    score += match parsed.bit_depth.as_deref() {
        Some("12bit") => 150,
        Some("10bit") => 100,
        Some("8bit") => 0,
        _ => 0,
    };

    // 3D penalty
    if parsed.is_3d {
        score -= 500;
    }

    // Remastered bonus
    if parsed.is_remastered {
        score += 100;
    }

    score
}
