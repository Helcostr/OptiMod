use optimod_plugin_sdk::Action;

/// One checker’s outcome for a single message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleResult {
    pub module: String,
    pub action: Action,
}

/// Aggregated outcome after running every registered checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineVerdict {
    pub action: Action,
    pub modules: Vec<ModuleResult>,
}

impl PipelineVerdict {
    #[inline]
    pub fn is_good(&self) -> bool {
        self.action.is_good()
    }

    pub fn security_flags(&self) -> Vec<String> {
        self.modules
            .iter()
            .filter_map(|result| match result.action {
                Action::Pass => None,
                Action::Flag => Some(format!("{}:flag", result.module)),
                Action::Block => Some(format!("{}:block", result.module)),
            })
            .collect()
    }
}

pub trait CheckerModule: Send + Sync {
    fn name(&self) -> &str;
    fn decide(&self, text: &str) -> Action;
}

pub fn run_checkers(checkers: &[&dyn CheckerModule], text: &str) -> PipelineVerdict {
    let modules = checkers
        .iter()
        .map(|checker| ModuleResult {
            module: checker.name().to_string(),
            action: checker.decide(text),
        })
        .collect::<Vec<_>>();

    PipelineVerdict {
        action: worst_action(modules.iter().map(|result| result.action)),
        modules,
    }
}

fn worst_action(actions: impl IntoIterator<Item = Action>) -> Action {
    actions.into_iter().fold(Action::Pass, |worst, action| {
        if action_rank(action) > action_rank(worst) {
            action
        } else {
            worst
        }
    })
}

const fn action_rank(action: Action) -> u8 {
    match action {
        Action::Pass => 0,
        Action::Flag => 1,
        Action::Block => 2,
    }
}
