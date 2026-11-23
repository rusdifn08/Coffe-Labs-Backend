use aho_corasick::AhoCorasick;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use rustrict::{Censor, Type};
use std::{num::NonZeroU32, time::Duration};

use crate::modules::chat::{
    error::ChatError,
    services::{
        moderation_normalizer::contains_multilingual_profanity,
        profanity_lexicon::SEVERE_SUBSTRING_TERMS,
    },
};

const MAX_MESSAGE_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationMode {
    MaskWithAsterisks,
    RejectMessage,
}

type LocalBucket = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Per-connection in-socket token bucket enforcing anti-flooding quotas.
pub struct SocketRateLimiter {
    msg_limiter: LocalBucket,
    typing_limiter: LocalBucket,
}

impl Default for SocketRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketRateLimiter {
    pub fn new() -> Self {
        let msg_burst = NonZeroU32::new(8).unwrap_or(NonZeroU32::MIN);
        let typing_burst = NonZeroU32::new(2).unwrap_or(NonZeroU32::MIN);
        let msg_quota = Quota::with_period(Duration::from_millis(625))
            .unwrap_or_else(|| Quota::per_second(msg_burst))
            .allow_burst(msg_burst);
        let typing_quota = Quota::with_period(Duration::from_millis(1500))
            .unwrap_or_else(|| Quota::per_second(typing_burst))
            .allow_burst(typing_burst);
        Self {
            msg_limiter: RateLimiter::direct(msg_quota),
            typing_limiter: RateLimiter::direct(typing_quota),
        }
    }

    pub fn check_message_quota(&self) -> Result<(), ChatError> {
        self.msg_limiter.check().map_err(|_| {
            ChatError::RateLimited("Batas pengiriman tercapai (maksimal 8 pesan per 5 detik).".into())
        })
    }

    pub fn check_typing_quota(&self) -> Result<(), ChatError> {
        self.typing_limiter.check().map_err(|_| {
            ChatError::RateLimited("Indikator mengetik terlalu sering.".into())
        })
    }
}

/// Mesin moderasi multibahasa hybrid (Normalizer Anti-Bypass + Aho-Corasick + rustrict).
pub struct ModerationGuard {
    automaton: Option<AhoCorasick>,
}

impl Default for ModerationGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ModerationGuard {
    pub fn new() -> Self {
        let automaton = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(SEVERE_SUBSTRING_TERMS)
            .ok();
        Self { automaton }
    }

    pub fn check_text(&self, raw_text: &str, mode: ModerationMode) -> Result<String, ChatError> {
        if raw_text.len() > MAX_MESSAGE_BYTES {
            return Err(ChatError::InvalidFrame(
                "Panjang pesan melebihi batas maksimum 4 KB.".into(),
            ));
        }

        let trimmed = raw_text.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }

        let multi_flagged = contains_multilingual_profanity(trimmed);
        let (en_censored, analysis) = Censor::from_str(trimmed)
            .with_censor_threshold(Type::PROFANE & Type::SEVERE)
            .censor_and_analyze();
        let en_flagged = analysis.is(Type::PROFANE & Type::SEVERE);

        if mode == ModerationMode::RejectMessage && (multi_flagged || en_flagged) {
            return Err(ChatError::ProfanityRejected(
                "Konten mengandung kata kasar atau tidak pantas dan diblokir oleh sistem moderasi Coffee Labs.".into(),
            ));
        }

        let base = if en_flagged { en_censored } else { trimmed.to_string() };
        if !multi_flagged {
            return Ok(base);
        }
        Ok(match &self.automaton {
            Some(ac) => mask_aho_matches(&base, ac),
            None => base,
        })
    }
}

fn mask_aho_matches(input: &str, ac: &AhoCorasick) -> String {
    let mut result = String::with_capacity(input.len());
    let mut last_idx = 0usize;
    for mat in ac.find_iter(input) {
        if mat.start() > last_idx {
            result.push_str(&input[last_idx..mat.start()]);
        }
        for _ in 0..input[mat.start()..mat.end()].chars().count().max(3) {
            result.push('*');
        }
        last_idx = mat.end();
    }
    if last_idx < input.len() {
        result.push_str(&input[last_idx..]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_multilingual_and_obfuscated_profanity() {
        let g = ModerationGuard::new();
        for bad in [
            "bangsat lu", "kontol", "fuck", "f.u.c.k", "f u c k", "k0nt0l", "b4ngs4t",
            "f*ck", "putangina", "blyat", "씨발", "操你妈", "شرموطة", "madarchod",
            "kurwa", "vaffanculo", "scheisse", "scheiiiiisse", "jancok tenan", "slot gacor maxwin88",
        ] {
            assert!(g.check_text(bad, ModerationMode::RejectMessage).is_err(), "Harusnya diblokir: {bad}");
        }
    }

    #[test]
    fn allows_clean_coffee_discussions_without_false_positives() {
        let g = ModerationGuard::new();
        for ok in [
            "V60 Kintamani classic pour over 15g/240ml suhu 92C",
            "Proses memekarkan bloom 35 detik lalu analisis TDS 1.38%",
            "Rasa passion fruit manis, cocok dan suka banget!",
            "Masuk akal, kasus ekstraksi bitter bisa diatasi di grind size",
        ] {
            assert!(g.check_text(ok, ModerationMode::RejectMessage).is_ok(), "Harusnya lolos: {ok}");
        }
    }
}
