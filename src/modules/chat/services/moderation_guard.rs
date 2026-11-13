use aho_corasick::AhoCorasick;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use rustrict::{Censor, Type};
use std::{num::NonZeroU32, time::Duration};

use crate::modules::chat::error::ChatError;

const MAX_MESSAGE_BYTES: usize = 4096;

const BLOCKED_ID_TERMS: &[&str] = &[
    "anjing",
    "bangsat",
    "kontol",
    "memek",
    "ngentot",
    "jancok",
    "bajingan",
    "keparat",
    "lonte",
    "perek",
    "slot gacor",
    "judi online",
    "togel online",
    "situs gacor",
    "maxwin88",
];

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
            ChatError::RateLimited(
                "Batas pengiriman tercapai (maksimal 8 pesan per 5 detik).".to_string(),
            )
        })
    }

    pub fn check_typing_quota(&self) -> Result<(), ChatError> {
        self.typing_limiter.check().map_err(|_| {
            ChatError::RateLimited("Indikator mengetik terlalu sering.".to_string())
        })
    }
}

/// Hybrid zero-alloc profanity and anti-spam engine (rustrict + Aho-Corasick).
pub struct ModerationGuard {
    id_spam_automaton: Option<AhoCorasick>,
}

impl Default for ModerationGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ModerationGuard {
    pub fn new() -> Self {
        let id_spam_automaton = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(BLOCKED_ID_TERMS)
            .ok();

        Self { id_spam_automaton }
    }

    pub fn check_text(
        &self,
        raw_text: &str,
        mode: ModerationMode,
    ) -> Result<String, ChatError> {
        if raw_text.len() > MAX_MESSAGE_BYTES {
            return Err(ChatError::InvalidFrame(
                "Panjang pesan melebihi batas maksimum 4 KB.".to_string(),
            ));
        }

        let trimmed = raw_text.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }

        let id_flagged = self
            .id_spam_automaton
            .as_ref()
            .map(|ac| ac.find(trimmed).is_some())
            .unwrap_or(false);
        let (en_censored, analysis) = Censor::from_str(trimmed)
            .with_censor_threshold(Type::PROFANE | Type::OFFENSIVE | Type::SEXUAL)
            .censor_and_analyze();

        let en_flagged = analysis.is(Type::PROFANE)
            || analysis.is(Type::OFFENSIVE)
            || analysis.is(Type::SEXUAL);

        if mode == ModerationMode::RejectMessage && (id_flagged || en_flagged) {
            return Err(ChatError::ProfanityRejected(
                "Pesan ditolak oleh filter moderasi komunitas Social Coffee.".to_string(),
            ));
        }

        let base = if en_flagged {
            en_censored
        } else {
            trimmed.to_string()
        };

        if !id_flagged {
            return Ok(base);
        }

        Ok(match &self.id_spam_automaton {
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
        let char_len = input[mat.start()..mat.end()].chars().count().max(3);
        for _ in 0..char_len {
            result.push('*');
        }
        last_idx = mat.end();
    }
    if last_idx < input.len() {
        result.push_str(&input[last_idx..]);
    }
    result
}
