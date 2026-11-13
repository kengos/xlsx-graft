//! The state a server holds for every request it answers.

use xlsx_graft_host::Policy;

/// What the server holds from its start: the policy that bounds every tool,
/// read once from the `--policy` file (or the default policy). A tool takes
/// its limits from here, not from its call's arguments, so that a caller
/// cannot widen them.
#[derive(Debug, Default)]
pub struct Context {
    policy: Policy,
}

impl Context {
    /// A context that holds `policy`.
    #[must_use]
    pub fn new(policy: Policy) -> Self {
        Self { policy }
    }

    /// The policy every tool runs under.
    #[must_use]
    pub fn policy(&self) -> &Policy {
        &self.policy
    }
}

#[cfg(test)]
mod tests;
