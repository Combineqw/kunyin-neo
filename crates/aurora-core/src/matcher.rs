//! Local music matching, fuzzy scoring, candidate selection, and text normalization.
//!
//! Provides high-performance text normalisation, artist relationship matching,
//! and candidate scoring for batch-importing and enriching thousands of tracks
//! without GC pressure or V8 regex overhead.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchTags {
    pub title: String,
    pub artist: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredCandidate {
    pub item: serde_json::Value,
    pub score: f64,
    pub title_matched: bool,
    pub artist_matched: bool,
    pub duration_diff_ms: i64,
}

/// Normalizes music title or artist text.
///
/// Equivalent to JavaScript:
/// `value.normalize('NFKC').toLocaleLowerCase().replace(/\b(?:feat|ft)\.?\b/gu, '').replace(/[^\p{L}\p{N}]+/gu, '')`
pub fn normalize_music_text(input: &str) -> String {
    // 1. Full-width ASCII normalization to half-width, lowercase
    let mut normalized = String::with_capacity(input.len());
    for c in input.chars() {
        let mapped = match c {
            '\u{3000}' => ' ',
            '\u{FF01}'..='\u{FF5E}' => {
                char::from_u32(c as u32 - 0xFEE0).unwrap_or(c)
            }
            other => other,
        };
        for lc in mapped.to_lowercase() {
            normalized.push(lc);
        }
    }

    // 2. Strip \b(?:feat|ft)\.?\b
    let bytes = normalized.as_bytes();
    let len = bytes.len();
    let mut filtered = String::with_capacity(len);
    let mut i = 0;
    while i < len {
        let is_word_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        let left_bound = i == 0 || !is_word_char(bytes[i - 1]);
        if left_bound {
            let is_feat = i + 4 <= len && &bytes[i..i + 4] == b"feat";
            let is_ft = i + 2 <= len && &bytes[i..i + 2] == b"ft";
            if is_feat || is_ft {
                let word_end = if is_feat { i + 4 } else { i + 2 };
                let has_dot = word_end < len && bytes[word_end] == b'.';
                let after_pos = if has_dot { word_end + 1 } else { word_end };
                let right_bound = after_pos >= len || !is_word_char(bytes[after_pos]);
                if right_bound {
                    i = after_pos;
                    continue;
                }
            }
        }
        let ch = normalized[i..].chars().next().unwrap_or(' ');
        filtered.push(ch);
        i += ch.len_utf8();
    }

    // 3. Keep only alphanumeric (\p{L} and \p{N})
    let mut result = String::with_capacity(filtered.len());
    for c in filtered.chars() {
        if c.is_alphanumeric() {
            result.push(c);
        }
    }
    result
}

/// Checks whether two artist strings match (equality or containment).
pub fn artists_match(left: &str, right: &str) -> bool {
    let a = normalize_music_text(left);
    let b = normalize_music_text(right);
    !a.is_empty() && !b.is_empty() && (a == b || a.contains(&b) || b.contains(&a))
}

/// Selects a single unambiguous candidate match from a candidate list.
///
/// Requires exact normalized title, artist match, and duration tolerance within 5000ms.
/// Returns None if 0 matches or if multiple ambiguous matches occur.
pub fn select_enrichment_candidate(
    tags: &MatchTags,
    duration: u64,
    candidates: &[serde_json::Value],
) -> Option<serde_json::Value> {
    let title = normalize_music_text(&tags.title);
    if title.is_empty() || normalize_music_text(&tags.artist).is_empty() {
        return None;
    }

    let mut matches = Vec::new();
    for candidate in candidates {
        let item_type = candidate.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if item_type == "local" {
            continue;
        }

        let cand_title = candidate.get("title").and_then(|v| v.as_str()).unwrap_or("");
        if normalize_music_text(cand_title) != title {
            continue;
        }

        let cand_artist = candidate.get("artist").and_then(|v| v.as_str()).unwrap_or("");
        if !artists_match(&tags.artist, cand_artist) {
            continue;
        }

        let cand_duration = candidate.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);
        if duration > 0 && cand_duration > 0 {
            let diff = (duration as i64 - cand_duration as i64).abs();
            if diff > 5000 {
                continue;
            }
        }

        matches.push(candidate.clone());
    }

    if matches.len() == 1 {
        Some(matches.remove(0))
    } else {
        None
    }
}

/// Computes normalized Levenshtein similarity between two strings (0.0 ..= 1.0).
pub fn levenshtein_similarity(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();
    if m == 0 && n == 0 {
        return 1.0;
    }
    if m == 0 || n == 0 {
        return 0.0;
    }
    let mut dp = vec![0usize; n + 1];
    for j in 0..=n {
        dp[j] = j;
    }
    for i in 1..=m {
        let mut prev = dp[0];
        dp[0] = i;
        for j in 1..=n {
            let temp = dp[j];
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[j] = (dp[j] + 1).min(dp[j - 1] + 1).min(prev + cost);
            prev = temp;
        }
    }
    let dist = dp[n];
    let max_len = m.max(n);
    1.0 - (dist as f64 / max_len as f64)
}

/// Computes a comprehensive match score between a local track and an online candidate.
///
/// Returns 0.0 to 1.0.
pub fn score_music_match(
    tag_title: &str,
    tag_artist: &str,
    tag_duration: u64,
    candidate_title: &str,
    candidate_artist: &str,
    candidate_duration: u64,
) -> f64 {
    let norm_tag_title = normalize_music_text(tag_title);
    let norm_cand_title = normalize_music_text(candidate_title);

    if norm_tag_title.is_empty() || norm_cand_title.is_empty() {
        return 0.0;
    }

    let mut score = 0.0;

    // Title score (max 0.6)
    if norm_tag_title == norm_cand_title {
        score += 0.6;
    } else if norm_tag_title.contains(&norm_cand_title) || norm_cand_title.contains(&norm_tag_title) {
        let ratio = (norm_tag_title.len().min(norm_cand_title.len()) as f64)
            / (norm_tag_title.len().max(norm_cand_title.len()) as f64);
        score += 0.3 + 0.25 * ratio;
    } else {
        let sim = levenshtein_similarity(&norm_tag_title, &norm_cand_title);
        score += 0.5 * sim;
    }

    // Artist score (max 0.3)
    let norm_tag_artist = normalize_music_text(tag_artist);
    let norm_cand_artist = normalize_music_text(candidate_artist);
    if !norm_tag_artist.is_empty() && !norm_cand_artist.is_empty() {
        if norm_tag_artist == norm_cand_artist {
            score += 0.3;
        } else if norm_tag_artist.contains(&norm_cand_artist) || norm_cand_artist.contains(&norm_tag_artist) {
            score += 0.2;
        } else {
            let sim = levenshtein_similarity(&norm_tag_artist, &norm_cand_artist);
            score += 0.2 * sim;
        }
    }

    // Duration score (max 0.1)
    if tag_duration > 0 && candidate_duration > 0 {
        let diff = (tag_duration as i64 - candidate_duration as i64).abs();
        if diff <= 2000 {
            score += 0.1;
        } else if diff <= 5000 {
            score += 0.05;
        } else if diff > 15000 {
            score -= 0.15;
        }
    } else {
        score += 0.05;
    }

    score.clamp(0.0, 1.0)
}

/// Evaluates and ranks a list of online candidates against local tags.
///
/// Returns candidates sorted descending by match score.
pub fn rank_enrichment_candidates(
    tags: &MatchTags,
    duration: u64,
    candidates: &[serde_json::Value],
) -> Vec<ScoredCandidate> {
    let norm_title = normalize_music_text(&tags.title);
    let mut scored = Vec::new();

    for candidate in candidates {
        let item_type = candidate.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if item_type == "local" {
            continue;
        }

        let cand_title = candidate.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let cand_artist = candidate.get("artist").and_then(|v| v.as_str()).unwrap_or("");
        let cand_duration = candidate.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);

        let title_matched = normalize_music_text(cand_title) == norm_title;
        let artist_matched = artists_match(&tags.artist, cand_artist);
        let duration_diff_ms = if duration > 0 && cand_duration > 0 {
            (duration as i64 - cand_duration as i64).abs()
        } else {
            0
        };

        let score = score_music_match(
            &tags.title,
            &tags.artist,
            duration,
            cand_title,
            cand_artist,
            cand_duration,
        );

        scored.push(ScoredCandidate {
            item: candidate.clone(),
            score,
            title_matched,
            artist_matched,
            duration_diff_ms,
        });
    }

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_music_text() {
        assert_eq!(
            normalize_music_text("Jay Chou feat. JJ Lin"),
            "jaychoujjlin"
        );
        assert_eq!(
            normalize_music_text("Taylor Swift ft. Ice Spice"),
            "taylorswifticespice"
        );
        assert_eq!(
            normalize_music_text("陈奕迅 - 富士山下"),
            "陈奕迅富士山下"
        );
        assert_eq!(
            normalize_music_text("feature film"),
            "featurefilm"
        );
        assert_eq!(
            normalize_music_text("（全角） ＡＢＣ １２３"),
            "全角abc123"
        );
    }

    #[test]
    fn test_artists_match() {
        assert!(artists_match("周杰伦", "周杰伦"));
        assert!(artists_match("周杰伦 feat. 蔡依林", "周杰伦"));
        assert!(artists_match("林俊杰", "林俊杰 / 阿信"));
        assert!(!artists_match("周杰伦", "陈奕迅"));
        assert!(!artists_match("", "周杰伦"));
    }

    #[test]
    fn test_select_enrichment_candidate_unambiguous() {
        let tags = MatchTags {
            title: "晴天".to_string(),
            artist: "周杰伦".to_string(),
        };
        let candidates = vec![
            json!({
                "type": "wy",
                "id": 1001,
                "title": "晴天",
                "artist": "周杰伦",
                "duration": 269000
            }),
            json!({
                "type": "wy",
                "id": 1002,
                "title": "晴天 (Live)",
                "artist": "周杰伦",
                "duration": 270000
            }),
        ];

        let selected = select_enrichment_candidate(&tags, 269500, &candidates);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().get("id").unwrap().as_u64(), Some(1001));
    }

    #[test]
    fn test_select_enrichment_candidate_ambiguous_returns_none() {
        let tags = MatchTags {
            title: "晴天".to_string(),
            artist: "周杰伦".to_string(),
        };
        let candidates = vec![
            json!({
                "type": "wy",
                "id": 1001,
                "title": "晴天",
                "artist": "周杰伦",
                "duration": 269000
            }),
            json!({
                "type": "qq",
                "id": 1003,
                "title": "晴天",
                "artist": "周杰伦",
                "duration": 269200
            }),
        ];

        // Two identical matches -> ambiguous -> returns None
        let selected = select_enrichment_candidate(&tags, 269000, &candidates);
        assert!(selected.is_none());
    }

    #[test]
    fn test_score_and_rank() {
        let tags = MatchTags {
            title: "七里香".to_string(),
            artist: "周杰伦".to_string(),
        };
        let candidates = vec![
            json!({
                "type": "wy",
                "id": 1,
                "title": "七里香",
                "artist": "周杰伦",
                "duration": 300000
            }),
            json!({
                "type": "kw",
                "id": 2,
                "title": "七里香 (Live)",
                "artist": "周杰伦",
                "duration": 310000
            }),
            json!({
                "type": "kg",
                "id": 3,
                "title": "稻香",
                "artist": "周杰伦",
                "duration": 240000
            }),
        ];

        let ranked = rank_enrichment_candidates(&tags, 300100, &candidates);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].item.get("id").unwrap().as_u64(), Some(1));
        assert!(ranked[0].score > 0.9);
        assert!(ranked[0].score > ranked[1].score);
        assert!(ranked[1].score > ranked[2].score);
    }
}
