use cm_core::Action as CmAction;
use cm_english::EnglishChecker;
use optimod_plugin_sdk::Action;
use serde_json::{json, Value};

use super::CheckerModule;

/// Built-in checker for the `small-caps-obfuscation` plugin (English-only today).
pub struct SmallCapsObfuscationModule(pub EnglishChecker);

impl Default for SmallCapsObfuscationModule {
    fn default() -> Self {
        Self(EnglishChecker::default())
    }
}

fn map_action(action: CmAction) -> Action {
    match action {
        CmAction::Pass => Action::Pass,
        CmAction::Flag => Action::Flag,
        CmAction::Block => Action::Block,
    }
}

fn action_label(action: CmAction) -> &'static str {
    match action {
        CmAction::Pass => "pass",
        CmAction::Flag => "flag",
        CmAction::Block => "block",
    }
}

impl CheckerModule for SmallCapsObfuscationModule {
    fn name(&self) -> &str {
        "small-caps-obfuscation"
    }

    fn decide(&self, text: &str) -> Action {
        map_action(self.0.decide(text))
    }

    fn debug_detail(&self, text: &str) -> Option<Value> {
        let result = self.0.evaluate(text);
        Some(json!({
            "action": action_label(result.action),
            "obfuscation_ratio": result.obfuscation_ratio,
            "conversion_ratio": result.conversion_ratio,
            "english_ratio": result.english_ratio,
            "converted": result.converted,
            "suspicious": result.suspicious,
            "relevant": result.relevant,
            "total_word_count": result.total_word_count,
            "obfuscated_word_count": result.obfuscated_word_count,
            "english_word_count": result.english_word_count,
            "canonical": result.canonical,
        }))
    }
}
