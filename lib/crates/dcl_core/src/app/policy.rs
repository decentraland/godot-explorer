use super::State;

/// When the engine starts (D-42). `Advance` requests it once the first RN screen is drawn, so
/// Godot boots behind the menus; `OnDemand` waits for the first call that needs it. Never at
/// process launch. Starting at launch would be a third variant and a one-line change here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EngineStartPolicy {
    #[default]
    Advance,
    OnDemand,
}

impl EngineStartPolicy {
    /// A recovered session (Jump in is likely) or a new user headed to sign-in (avatar creation
    /// follows) both want 3D soon; before boot has finished there is nothing to judge.
    pub fn starts_in_advance(self, state: &State) -> bool {
        self == Self::Advance
            && matches!(
                state,
                State::SignedIn | State::SignedOut | State::SigningIn { .. }
            )
    }
}
