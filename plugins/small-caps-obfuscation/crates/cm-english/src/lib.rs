//! Canonical ASCII conversion + English dictionary word ratio.

#![forbid(unsafe_code)]

use cm_alphabet::{map_char, CanonicalStats};
use cm_core::{Action, Checker, CheckVerdict, PLUGIN_MODULE_ID};
use cm_normalize::{is_ascii, nfkc_into};
use std::cell::RefCell;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/english_words.rs"));

thread_local! {
    static WORK: RefCell<WorkBuffers> = RefCell::new(WorkBuffers::new());
}

fn early_exit_trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("CM_TRACE_EARLY_EXIT")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "True" | "yes"))
            .unwrap_or(false)
    })
}

fn trace_early_exit(message: impl AsRef<str>) {
    if early_exit_trace_enabled() {
        eprintln!("[cm-english:early-exit] {}", message.as_ref());
    }
}

struct WorkBuffers {
    normalized: String,
    canonical: String,
    obfuscated_mask: Vec<bool>,
    token: String,
    alpha: String,
}

impl WorkBuffers {
    fn new() -> Self {
        Self {
            normalized: String::new(),
            canonical: String::new(),
            obfuscated_mask: Vec::new(),
            token: String::new(),
            alpha: String::new(),
        }
    }
}

struct DecideOutcome {
    action: Action,
    stats: CanonicalStats,
    words: WordStats,
    canonical: String,
}

/// Tunables for English detection after canonicalization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnglishConfig {
    /// Block when obfuscated words / total words `>=` this (default 0.75).
    pub block_obfuscation_threshold: f32,
    /// Block when English hits / obfuscated words `>=` this (default 0.75).
    pub block_english_ratio: f32,
    /// Flag when obfuscation ratio crosses this (below block).
    pub flag_obfuscation_threshold: f32,
    /// Minimum token count before English ratio can trigger block/flag.
    pub min_words: u32,
    pub force_action: Option<Action>,
}

impl Default for EnglishConfig {
    fn default() -> Self {
        Self {
            block_obfuscation_threshold: 0.75,
            block_english_ratio: 0.75,
            flag_obfuscation_threshold: 0.25,
            min_words: 3,
            force_action: None,
        }
    }
}

impl EnglishConfig {
    pub fn with_block_obfuscation_threshold(mut self, t: f32) -> Self {
        self.block_obfuscation_threshold = t.clamp(0.0, 1.0);
        self
    }

    pub fn with_block_english_ratio(mut self, t: f32) -> Self {
        self.block_english_ratio = t.clamp(0.0, 1.0);
        self
    }

    pub fn with_force_action(mut self, action: Action) -> Self {
        self.force_action = Some(action);
        self
    }
}

/// Per-word scoring tallies on canonical ASCII text.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WordStats {
    /// Scored word tokens (length >= 2, not excluded).
    pub total_word_count: u32,
    /// Words that contain at least one obfuscated character.
    pub obfuscated_word_count: u32,
    /// Obfuscated words that match the English dictionary.
    pub english_word_count: u32,
}

impl WordStats {
    /// Obfuscated words / all scored words.
    #[inline]
    pub fn obfuscation_ratio(&self) -> f32 {
        if self.total_word_count == 0 {
            0.0
        } else {
            self.obfuscated_word_count as f32 / self.total_word_count as f32
        }
    }

    /// English among obfuscated words only: `english / obfuscated_words`.
    #[inline]
    pub fn english_ratio(&self) -> f32 {
        if self.obfuscated_word_count == 0 {
            0.0
        } else {
            self.english_word_count as f32 / self.obfuscated_word_count as f32
        }
    }
}

/// Full result: conversion stats + English word stats + canonical text.
#[derive(Debug, Clone, PartialEq)]
pub struct EnglishResult {
    pub action: Action,
    /// Obfuscated words / total scored words.
    pub obfuscation_ratio: f32,
    /// Mapping quality: converted / suspicious.
    pub conversion_ratio: f32,
    pub converted: u32,
    pub suspicious: u32,
    pub relevant: u32,
    /// English hits / obfuscated words (100% if every obfuscated word is English).
    pub english_ratio: f32,
    pub total_word_count: u32,
    pub obfuscated_word_count: u32,
    pub english_word_count: u32,
    pub canonical: String,
}

#[inline]
fn is_apostrophe(ch: char) -> bool {
    ch == '\'' || ch == '\u{2019}'
}

/// Letter or apostrophe inside a token. `ascii_only` selects canonical scan vs suffix forecast scan.
#[inline]
fn is_token_letter(ch: char, ascii_only: bool) -> bool {
    (if ascii_only {
        ch.is_ascii_alphabetic()
    } else {
        ch.is_alphabetic()
    }) || is_apostrophe(ch)
}

/// Apostrophes stay inside a token so `you've` / `you're` are not split into fragments.
#[inline]
fn is_word_char(ch: char) -> bool {
    is_token_letter(ch, true)
}

/// Normalized-text counterpart to [`is_word_char`] for suffix forecast tokenization.
#[inline]
fn is_suffix_scan_char(ch: char) -> bool {
    is_token_letter(ch, false)
}

/// Usernames / names excluded from English scoring entirely.
#[inline]
fn is_scoring_excluded(alpha: &str) -> bool {
    matches!(alpha, "deematrix" | "hassan")
}

/// Letters-only form of a token (`you've` → `youve`).
fn alpha_only_into(token: &str, buf: &mut String) {
    buf.clear();
    for c in token.chars() {
        if c.is_ascii_alphabetic() {
            buf.push(c.to_ascii_lowercase());
        }
    }
}

/// Dictionary hit, including common contractions whose base word is in the list.
fn token_is_english(alpha: &str) -> bool {
    if alpha.len() < 2 {
        return false;
    }
    if WORDS.contains(alpha) {
        return true;
    }
    for suffix in ["ve", "re", "ll", "d", "m", "t", "s"] {
        if let Some(base) = alpha.strip_suffix(suffix) {
            if !base.is_empty() && WORDS.contains(base) {
                return true;
            }
        }
    }
    false
}

fn token_is_scorable(alpha: &str) -> bool {
    alpha.len() >= 2 && !is_scoring_excluded(alpha)
}

fn record_word_token(
    stats: &mut WordStats,
    token: &str,
    from_obfuscation: bool,
    alpha: &mut String,
) {
    alpha_only_into(token, alpha);
    if !token_is_scorable(alpha) {
        return;
    }
    stats.total_word_count += 1;
    if !from_obfuscation {
        return;
    }
    stats.obfuscated_word_count += 1;
    if token_is_english(alpha) {
        stats.english_word_count += 1;
    }
}

fn push_word_char(token: &mut String, ch: char) {
    token.push(if ch.is_ascii_alphabetic() {
        ch.to_ascii_lowercase()
    } else {
        ch
    });
}

/// Cheap suffix scan: ASCII-only letter words are ruled plain; everything else is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SuffixForecast {
    /// Scored suffix words whose letters are all ASCII — cannot be obfuscated.
    ascii_plain_words: u32,
    /// Scored suffix words with at least one non-ASCII letter — worst-case obfuscated.
    unknown_words: u32,
}

impl SuffixForecast {
    fn dilution_words(self) -> u32 {
        self.ascii_plain_words + self.unknown_words
    }
}

/// True when every letter in the token is ASCII (digits/apostrophes allowed).
fn is_ascii_plain_word(token: &str) -> bool {
    let mut letters = 0u32;
    for ch in token.chars() {
        if ch.is_ascii_alphabetic() {
            letters += 1;
        } else if ch.is_alphabetic() {
            return false;
        }
    }
    letters >= 2
}

/// `Some(true)` = ASCII-plain, `Some(false)` = unknown (may be obfuscated), `None` = not scored.
fn classify_suffix_word(token: &str) -> Option<bool> {
    if is_ascii_plain_word(token) {
        return Some(true);
    }
    let letters = token.chars().filter(|ch| ch.is_alphabetic()).count();
    if letters >= 2 {
        Some(false)
    } else {
        None
    }
}

fn collect_token_kinds(text: &str) -> Vec<Option<bool>> {
    let mut kinds = Vec::new();
    let mut token = String::new();
    for ch in text.chars() {
        if is_suffix_scan_char(ch) {
            token.push(ch);
        } else if !token.is_empty() {
            kinds.push(classify_suffix_word(&token));
            token.clear();
        }
    }
    if !token.is_empty() {
        kinds.push(classify_suffix_word(&token));
    }
    kinds
}

#[cfg(test)]
fn forecast_from_kinds(kinds: &[Option<bool>]) -> SuffixForecast {
    let mut forecast = SuffixForecast::default();
    for kind in kinds {
        match kind {
            Some(true) => forecast.ascii_plain_words += 1,
            Some(false) => forecast.unknown_words += 1,
            None => {}
        }
    }
    forecast
}

/// `forecasts[k]` = suffix stats after word boundary `#k` (aligned with `decide_core` token breaks).
fn precompute_suffix_forecasts(text: &str) -> Vec<SuffixForecast> {
    let kinds = collect_token_kinds(text);
    let n = kinds.len();
    let mut forecasts = vec![SuffixForecast::default(); n + 1];
    let mut acc = SuffixForecast::default();
    for i in (0..n).rev() {
        forecasts[i + 1] = acc;
        match kinds[i] {
            Some(true) => acc.ascii_plain_words += 1,
            Some(false) => acc.unknown_words += 1,
            None => {}
        }
    }
    forecasts
}

fn worst_case_obfuscation_ratio(words: &WordStats, max_future_words: u32) -> f32 {
    let total = words.total_word_count + max_future_words;
    if total == 0 {
        0.0
    } else {
        words.obfuscated_word_count as f32 / total as f32
    }
}

fn worst_case_english_ratio(words: &WordStats, max_future_obfuscated_words: u32) -> f32 {
    let obf = words.obfuscated_word_count + max_future_obfuscated_words;
    if obf == 0 {
        0.0
    } else {
        words.english_word_count as f32 / obf as f32
    }
}

fn should_block(words: &WordStats, config: &EnglishConfig) -> bool {
    if words.obfuscated_word_count == 0 {
        return false;
    }

    words.obfuscated_word_count >= config.min_words
        && words.obfuscation_ratio() >= config.block_obfuscation_threshold
        && words.english_ratio() >= config.block_english_ratio
}

/// Early block is safe only when both ratios would still clear thresholds in the worst case.
fn is_safe_with_forecast(words: &WordStats, config: &EnglishConfig, forecast: SuffixForecast) -> bool {
    worst_case_obfuscation_ratio(words, forecast.dilution_words())
        >= config.block_obfuscation_threshold
        && worst_case_english_ratio(words, forecast.unknown_words) >= config.block_english_ratio
}

fn block_gate_reasons(words: &WordStats, config: &EnglishConfig) -> String {
    let mut reasons: Vec<String> = Vec::new();
    if words.obfuscated_word_count == 0 {
        reasons.push("no obfuscated words".to_string());
    }
    if words.obfuscated_word_count < config.min_words {
        reasons.push(format!(
            "obfuscated_words {} < min {}",
            words.obfuscated_word_count,
            config.min_words
        ));
    }
    if words.obfuscation_ratio() < config.block_obfuscation_threshold {
        reasons.push(format!(
            "obfuscation {:.3} < {:.3}",
            words.obfuscation_ratio(),
            config.block_obfuscation_threshold
        ));
    }
    if words.english_ratio() < config.block_english_ratio {
        reasons.push(format!(
            "english {:.3} < {:.3}",
            words.english_ratio(),
            config.block_english_ratio
        ));
    }
    if reasons.is_empty() {
        "unknown".to_string()
    } else {
        reasons.join("; ")
    }
}

fn early_exit_safe(
    words: &WordStats,
    config: &EnglishConfig,
    forecast: SuffixForecast,
    trace_suffix: Option<&str>,
    word_boundary: Option<u32>,
) -> bool {
    if early_exit_trace_enabled() {
        let label = word_boundary
            .map(|n| format!("word_boundary #{n}"))
            .unwrap_or_else(|| "final token".to_string());
        let suffix = trace_suffix.unwrap_or("");
        trace_early_exit(format!("--- {label} ---"));
        trace_early_exit(format!(
            "  prefix (live): total={} obfuscated={} english_hits={} obfuscation={:.3} english_ratio={:.3}",
            words.total_word_count,
            words.obfuscated_word_count,
            words.english_word_count,
            words.obfuscation_ratio(),
            words.english_ratio()
        ));
        let preview = suffix.chars().take(48).collect::<String>();
        let preview = if suffix.chars().count() > 48 {
            format!("{preview}...")
        } else {
            preview
        };
        trace_early_exit(format!(
            "  suffix: chars={} ascii_plain={} unknown={} preview=\"{}\"",
            suffix.chars().count(),
            forecast.ascii_plain_words,
            forecast.unknown_words,
            preview.replace('\n', "\\n")
        ));
    }

    if !should_block(words, config) {
        if early_exit_trace_enabled() {
            trace_early_exit(format!(
                "  should_block=false ({}) → keep scanning",
                block_gate_reasons(words, config)
            ));
        }
        return false;
    }

    let worst_obfuscation = worst_case_obfuscation_ratio(words, forecast.dilution_words());
    let worst_english = worst_case_english_ratio(words, forecast.unknown_words);
    let safe = is_safe_with_forecast(words, config, forecast);

    if early_exit_trace_enabled() {
        trace_early_exit("  should_block=true on prefix (live ratios — not used for exit safety)");
        trace_early_exit(format!(
            "  worst_obfuscation={:.3} (obfuscated / (total + ascii_plain + unknown = {} + {} + {})) threshold={:.3}",
            worst_obfuscation,
            words.total_word_count,
            forecast.ascii_plain_words,
            forecast.unknown_words,
            config.block_obfuscation_threshold
        ));
        trace_early_exit(format!(
            "  worst_english={:.3} (english / (obfuscated + unknown = {} + {})) threshold={:.3}",
            worst_english,
            words.obfuscated_word_count,
            forecast.unknown_words,
            config.block_english_ratio
        ));
        trace_early_exit(if safe {
            "  → safe early exit (both worst-case ratios still block)"
        } else {
            "  → NOT safe → keep scanning"
        });
    }

    safe
}

struct FinishTokenCtx<'a> {
    words: &'a mut WordStats,
    token: &'a mut String,
    token_from_obfuscation: bool,
    alpha: &'a mut String,
    config: &'a EnglishConfig,
    allow_early_block: bool,
    suffix_forecasts: Option<&'a [SuffixForecast]>,
    trace_suffix: Option<&'a str>,
    word_boundary: Option<u32>,
}

fn finish_token(ctx: &mut FinishTokenCtx<'_>) -> Option<Action> {
    if ctx.token.is_empty() {
        return None;
    }
    if early_exit_trace_enabled() && ctx.allow_early_block {
        trace_early_exit(format!(
            "finished token {:?} (obfuscated={})",
            ctx.token,
            ctx.token_from_obfuscation
        ));
    }
    record_word_token(
        ctx.words,
        ctx.token,
        ctx.token_from_obfuscation,
        ctx.alpha,
    );
    ctx.token.clear();

    if !ctx.allow_early_block || !should_block(ctx.words, ctx.config) {
        return None;
    }

    let forecast = match (ctx.suffix_forecasts, ctx.word_boundary) {
        (Some(table), Some(n)) => table.get(n as usize).copied().unwrap_or_default(),
        _ => SuffixForecast::default(),
    };

    if early_exit_safe(
        ctx.words,
        ctx.config,
        forecast,
        ctx.trace_suffix,
        ctx.word_boundary,
    ) {
        if early_exit_trace_enabled() {
            trace_early_exit(format!(
                ">>> EARLY EXIT: Block at {label}",
                label = ctx
                    .word_boundary
                    .map(|n| format!("word boundary #{n}"))
                    .unwrap_or_else(|| "final token".to_string())
            ));
        }
        return Some(Action::Block);
    }
    None
}

/// Count English dictionary hits among **obfuscated** word tokens only.
pub fn analyze_words(canonical: &str, obfuscated_mask: &[bool]) -> WordStats {
    let mut stats = WordStats::default();
    let mut token = String::new();
    let mut alpha = String::new();
    let mut token_from_obfuscation = false;
    let mut idx = 0usize;

    for ch in canonical.chars() {
        if is_word_char(ch) {
            push_word_char(&mut token, ch);
            if obfuscated_mask.get(idx) == Some(&true) {
                token_from_obfuscation = true;
            }
        } else if !token.is_empty() {
            record_word_token(&mut stats, &token, token_from_obfuscation, &mut alpha);
            token.clear();
            token_from_obfuscation = false;
        }
        idx += 1;
    }

    if !token.is_empty() {
        record_word_token(&mut stats, &token, token_from_obfuscation, &mut alpha);
    }

    stats
}

fn action_from_stats(words: &WordStats, config: &EnglishConfig) -> Action {
    if should_block(words, config) {
        return Action::Block;
    }

    if words.obfuscated_word_count == 0 {
        return Action::Pass;
    }

    let obfuscation = words.obfuscation_ratio();
    let english = words.english_ratio();
    let enough_words = words.obfuscated_word_count >= config.min_words;

    if enough_words
        && obfuscation >= config.flag_obfuscation_threshold
        && english < config.block_english_ratio
    {
        return Action::Flag;
    }
    if enough_words
        && english >= config.block_english_ratio
        && obfuscation < config.block_obfuscation_threshold
    {
        return Action::Flag;
    }

    Action::Pass
}

fn trace_decide_start(config: &EnglishConfig, input: &str, materialize: bool) {
    if !early_exit_trace_enabled() {
        return;
    }
    trace_early_exit("=== decide_core start ===");
    trace_early_exit(format!(
        "  input: chars={} ascii={} materialize={}",
        input.chars().count(),
        is_ascii(input),
        materialize
    ));
    trace_early_exit(format!(
        "  thresholds: obfuscation>={:.3} english>={:.3} min_words={}",
        config.block_obfuscation_threshold,
        config.block_english_ratio,
        config.min_words
    ));
    trace_early_exit(format!("  early_exit_enabled={}", !materialize));
}

fn ascii_fast_path_outcome(input: &str, materialize: bool) -> DecideOutcome {
    if early_exit_trace_enabled() {
        trace_early_exit("ASCII fast path → Pass (no scan)");
    }
    DecideOutcome {
        action: Action::Pass,
        stats: CanonicalStats::default(),
        words: WordStats::default(),
        canonical: if materialize {
            input.to_string()
        } else {
            String::new()
        },
    }
}

fn prepare_materialize_buffers(work: &mut WorkBuffers, materialize: bool, normalized_len: usize) {
    if !materialize {
        return;
    }
    work.canonical.clear();
    work.obfuscated_mask.clear();
    work.canonical.reserve(normalized_len);
    work.obfuscated_mask.reserve(normalized_len);
}

fn trace_decide_end(action: Action, stats: &CanonicalStats, words: &WordStats) {
    if !early_exit_trace_enabled() {
        return;
    }
    trace_early_exit(format!(
        "=== decide_core end: {:?} (full scan, no early exit) ===",
        action
    ));
    trace_early_exit(format!(
        "  final conversion: suspicious={} relevant={} conversion={:.3}",
        stats.suspicious,
        stats.relevant,
        stats.conversion_ratio()
    ));
    trace_early_exit(format!(
        "  final words: total={} obfuscated={} english_hits={} obfuscation={:.3} english_ratio={:.3}",
        words.total_word_count,
        words.obfuscated_word_count,
        words.english_word_count,
        words.obfuscation_ratio(),
        words.english_ratio()
    ));
}

fn decide_core(
    config: &EnglishConfig,
    input: &str,
    materialize: bool,
    work: &mut WorkBuffers,
) -> DecideOutcome {
    trace_decide_start(config, input, materialize);

    if is_ascii(input) {
        return ascii_fast_path_outcome(input, materialize);
    }

    nfkc_into(input, &mut work.normalized);
    let allow_early_block = !materialize;
    let suffix_forecasts = if allow_early_block {
        precompute_suffix_forecasts(&work.normalized)
    } else {
        Vec::new()
    };
    let forecast_table = if allow_early_block {
        Some(suffix_forecasts.as_slice())
    } else {
        None
    };

    prepare_materialize_buffers(work, materialize, work.normalized.len());

    let mut stats = CanonicalStats::default();
    let mut words = WordStats::default();
    let mut token_from_obfuscation = false;
    let mut word_boundary = 0u32;
    work.token.clear();
    let trace = early_exit_trace_enabled();

    for (start, ch) in work.normalized.char_indices() {
        let (mapped, obfuscated) = map_char(ch, &mut stats);
        let next_byte = start + ch.len_utf8();

        if materialize {
            work.canonical.push(mapped);
            work.obfuscated_mask.push(obfuscated);
        }

        if is_word_char(mapped) {
            push_word_char(&mut work.token, mapped);
            if obfuscated {
                token_from_obfuscation = true;
            }
        } else if !work.token.is_empty() {
            word_boundary += 1;
            let trace_suffix = if trace {
                Some(work.normalized[next_byte..].as_ref())
            } else {
                None
            };
            let mut finish_ctx = FinishTokenCtx {
                words: &mut words,
                token: &mut work.token,
                token_from_obfuscation,
                alpha: &mut work.alpha,
                config,
                allow_early_block,
                suffix_forecasts: forecast_table,
                trace_suffix,
                word_boundary: Some(word_boundary),
            };
            if finish_token(&mut finish_ctx).is_some() {
                return DecideOutcome {
                    action: Action::Block,
                    stats,
                    words,
                    canonical: String::new(),
                };
            }
            token_from_obfuscation = false;
        }
    }

    if !work.token.is_empty() {
        let mut finish_ctx = FinishTokenCtx {
            words: &mut words,
            token: &mut work.token,
            token_from_obfuscation,
            alpha: &mut work.alpha,
            config,
            allow_early_block,
            suffix_forecasts: forecast_table,
            trace_suffix: None,
            word_boundary: None,
        };
        if finish_token(&mut finish_ctx).is_some() {
            return DecideOutcome {
                action: Action::Block,
                stats,
                words,
                canonical: String::new(),
            };
        }
    }

    let action = action_from_stats(&words, config);
    trace_decide_end(action, &stats, &words);
    DecideOutcome {
        action,
        stats,
        words,
        canonical: if materialize {
            work.canonical.clone()
        } else {
            String::new()
        },
    }
}

/// English checker: obfuscated text → canonical ASCII → dictionary word ratio.
#[derive(Debug, Clone)]
pub struct EnglishChecker {
    config: EnglishConfig,
}

impl EnglishChecker {
    pub fn new(config: EnglishConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &EnglishConfig {
        &self.config
    }

    /// Hot path: action only, no canonical string materialized.
    pub fn decide(&self, input: &str) -> Action {
        if let Some(action) = self.config.force_action {
            return action;
        }
        WORK.with(|work| decide_core(&self.config, input, false, &mut work.borrow_mut()).action)
    }

    pub fn evaluate(&self, input: &str) -> EnglishResult {
        if let Some(action) = self.config.force_action {
            return EnglishResult {
                action,
                obfuscation_ratio: 0.0,
                conversion_ratio: 0.0,
                converted: 0,
                suspicious: 0,
                relevant: 0,
                english_ratio: 0.0,
                total_word_count: 0,
                obfuscated_word_count: 0,
                english_word_count: 0,
                canonical: String::new(),
            };
        }

        let outcome =
            WORK.with(|work| decide_core(&self.config, input, true, &mut work.borrow_mut()));

        EnglishResult {
            action: outcome.action,
            obfuscation_ratio: outcome.words.obfuscation_ratio(),
            conversion_ratio: outcome.stats.conversion_ratio(),
            converted: outcome.stats.converted,
            suspicious: outcome.stats.suspicious,
            relevant: outcome.stats.relevant,
            english_ratio: outcome.words.english_ratio(),
            total_word_count: outcome.words.total_word_count,
            obfuscated_word_count: outcome.words.obfuscated_word_count,
            english_word_count: outcome.words.english_word_count,
            canonical: outcome.canonical,
        }
    }
}

impl Default for EnglishChecker {
    fn default() -> Self {
        Self::new(EnglishConfig::default())
    }
}

impl Checker for EnglishChecker {
    fn name(&self) -> &'static str {
        PLUGIN_MODULE_ID
    }

    fn is_good(&self, input: &str) -> bool {
        self.decide(input).is_good()
    }

    fn check(&self, input: &str) -> CheckVerdict {
        CheckVerdict::from_bool(self.is_good(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::Action;

    fn would_safe_early_block(words: &WordStats, suffix: &str) -> bool {
        let config = EnglishConfig::default();
        should_block(words, &config)
            && is_safe_with_forecast(
                words,
                &config,
                forecast_from_kinds(&collect_token_kinds(suffix)),
            )
    }

    #[test]
    fn decide_matches_evaluate_action() {
        let checker = EnglishChecker::default();
        for input in [
            "hello stream",
            "hellᴏ",
            "ʏᴏ ʙʀᴏ ᴊᴜꜱᴛ ᴡᴀɴᴛᴇᴅ ᴛᴏ ꜱʜᴏᴡ",
        ] {
            assert_eq!(checker.decide(input), checker.evaluate(input).action);
        }
    }

    #[test]
    fn flag_counts_as_good_for_is_good() {
        let checker = EnglishChecker::default();
        let input = "ʏᴏ ʙʀᴏ hello";
        let action = checker.decide(input);
        if action == Action::Flag {
            assert!(checker.is_good(input));
        }
    }

    #[test]
    fn obfuscation_without_english_does_not_block() {
        let input = "ᴛᴡᴢ ᴢᴢᴢ ᴢᴢᴢ ᴢᴢᴢ";
        assert_ne!(EnglishChecker::default().evaluate(input).action, Action::Block);
    }

    #[test]
    fn early_block_exits_when_suffix_stays_obfuscated() {
        let prefix = "ʏᴏ ʙʀᴏ ᴊᴜꜱᴛ ᴡᴀɴᴛᴇᴅ ᴛᴏ ꜱʜᴏᴡ ";
        let suffix = "ʙʀᴏ ᴊᴜꜱᴛ ᴡᴀɴᴛᴇᴅ ";
        let padding = "a".repeat(100_000);
        let input = format!("{prefix}{suffix}{padding}");
        assert_eq!(EnglishChecker::default().decide(&input), Action::Block);
    }

    #[test]
    fn obfuscated_prefix_with_ascii_suffix_does_not_false_block() {
        let prefix = "ʏᴏ ʙʀᴏ ᴊᴜꜱᴛ ";
        let suffix = "hello ".repeat(200);
        let input = format!("{prefix}{suffix}");
        let action = EnglishChecker::default().evaluate(&input).action;
        assert_ne!(action, Action::Block);
        assert_eq!(EnglishChecker::default().decide(&input), action);
    }

    #[test]
    fn early_exit_safety_cases() {
        let blocked = WordStats {
            total_word_count: 3,
            obfuscated_word_count: 3,
            english_word_count: 3,
        };
        assert!(!would_safe_early_block(&blocked, "hello hello hello"));
        assert!(!would_safe_early_block(
            &blocked,
            "ᴡᴀɴᴛᴇᴅ ᴛᴏ ꜱʜᴏᴡ ꜱᴏᴍᴇ ʀᴇᴀʟ",
        ));

        let strong = WordStats {
            total_word_count: 10,
            obfuscated_word_count: 10,
            english_word_count: 10,
        };
        assert!(would_safe_early_block(&strong, "ʙʀᴏ ᴊᴜꜱᴛ"));
        assert!(would_safe_early_block(&strong, "hello hello ʙʀᴏ"));

        let tight = WordStats {
            total_word_count: 3,
            obfuscated_word_count: 3,
            english_word_count: 3,
        };
        assert!(!would_safe_early_block(&tight, "ʙʀᴏ ᴊᴜꜱᴛ ᴡᴀɴᴛᴇᴅ"));
    }

    #[test]
    fn suffix_forecast_splits_ascii_and_unknown() {
        let f = forecast_from_kinds(&collect_token_kinds("hello hello ʙʀᴏ ᴡᴀɴᴛᴇᴅ"));
        assert_eq!(f.ascii_plain_words, 2);
        assert_eq!(f.unknown_words, 2);
        assert_eq!(f.dilution_words(), 4);
    }

    #[test]
    fn precompute_suffix_forecasts_at_boundaries() {
        let text = "ʏᴏ ʙʀᴏ hello hello ʙʀᴏ ᴡᴀɴᴛᴇᴅ";
        let table = precompute_suffix_forecasts(text);
        assert_eq!(
            table[1],
            forecast_from_kinds(&collect_token_kinds(" ʙʀᴏ hello hello ʙʀᴏ ᴡᴀɴᴛᴇᴅ"))
        );
        assert_eq!(
            table[2],
            forecast_from_kinds(&collect_token_kinds(" hello hello ʙʀᴏ ᴡᴀɴᴛᴇᴅ"))
        );
    }

    fn mask_all(text: &str) -> Vec<bool> {
        vec![true; text.chars().count()]
    }

    #[test]
    fn analyze_words_basic() {
        let text = "bro just wanted to show";
        let w = analyze_words(text, &mask_all(text));
        assert_eq!(w.total_word_count, 5);
        assert_eq!(w.obfuscated_word_count, 5);
        assert_eq!(w.english_word_count, 5);
    }

    #[test]
    fn contractions_stay_together() {
        let text = "you've been and you're here";
        let w = analyze_words(text, &mask_all(text));
        assert_eq!(w.total_word_count, 5);
        assert_eq!(w.english_word_count, 5);
        let split_text = "you xy";
        let split = analyze_words(split_text, &mask_all(split_text));
        assert_eq!(split.english_word_count, 1);
    }

    #[test]
    fn curly_apostrophe_contraction() {
        let text = "you\u{2019}ve got it";
        let w = analyze_words(text, &mask_all(text));
        assert_eq!(w.total_word_count, 3);
        assert_eq!(w.english_word_count, 3);
    }

    #[test]
    fn plain_ascii_words_not_in_obfuscated_denominator() {
        let text = "tell him HASSAN on DEEMATRIX7 discord";
        let mask = vec![false; text.chars().count()];
        let w = analyze_words(text, &mask);
        assert_eq!(w.total_word_count, 4);
        assert_eq!(w.obfuscated_word_count, 0);
    }

    #[test]
    fn word_obfuscation_ratio_counts_plain_words_in_total() {
        let r = EnglishChecker::default().evaluate("ʏᴏ ʙʀᴏ hello");
        assert_eq!(r.total_word_count, 3);
        assert_eq!(r.obfuscated_word_count, 2);
        assert!((r.obfuscation_ratio - 2.0 / 3.0).abs() < f32::EPSILON);
        assert_eq!(r.english_ratio, 1.0);
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    use cm_core::TWITCH_SMALL_CAPS_SPAM;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repo root")
    }

    #[test]
    fn twitch_small_caps_spam_blocks_on_english_plus_obfuscation() {
        let root = repo_root();
        let input = std::fs::read_to_string(
            root.join("fixtures")
                .join(TWITCH_SMALL_CAPS_SPAM)
                .join("input.txt"),
        )
        .expect("example input");

        let result = EnglishChecker::default().evaluate(input.trim());
        assert_eq!(result.action, Action::Block);
        assert!(result.obfuscation_ratio >= 0.75);
        assert!(result.english_ratio >= 0.75);
    }
}
