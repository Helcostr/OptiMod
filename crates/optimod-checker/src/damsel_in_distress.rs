use cm_core::Action as CmAction;
use damsel_core::DamselChecker;
use optimod_plugin_sdk::Action;
use serde_json::{json, Value};

use super::CheckerModule;

/// Built-in checker for the `damsel-in-distress` plugin.
pub struct DamselInDistressModule(pub DamselChecker);

impl Default for DamselInDistressModule {
    fn default() -> Self {
        Self(DamselChecker)
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

impl CheckerModule for DamselInDistressModule {
    fn name(&self) -> &str {
        damsel_core::PLUGIN_MODULE_ID
    }

    fn decide(&self, text: &str) -> Action {
        map_action(self.0.decide(text))
    }

    fn debug_detail(&self, text: &str) -> Option<Value> {
        let result = self.0.evaluate(text);
        Some(json!({
            "action": action_label(result.action),
            "has_flattery": result.has_flattery,
            "has_discord_invite": result.has_discord_invite,
        }))
    }
}
