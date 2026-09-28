use cm_core::Action as CmAction;
use cm_english::EnglishChecker;
use optimod_plugin_sdk::Action;

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

impl CheckerModule for SmallCapsObfuscationModule {
    fn name(&self) -> &str {
        "small-caps-obfuscation"
    }

    fn decide(&self, text: &str) -> Action {
        map_action(self.0.decide(text))
    }
}
