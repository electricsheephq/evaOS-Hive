pub(crate) fn require_native_agent_authority() -> Result<(), String> {
    if cfg!(feature = "evaos-teams-managed") {
        Err("Managed agents are assigned by ElectricSheep".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn require_native_huddle_authority() -> Result<(), String> {
    if cfg!(feature = "evaos-teams-managed") {
        Err("Managed huddle authority is controlled by ElectricSheep".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_agent_authority_matches_the_build_variant() {
        assert_eq!(
            super::require_native_agent_authority().is_err(),
            cfg!(feature = "evaos-teams-managed")
        );
        assert_eq!(
            super::require_native_huddle_authority().is_err(),
            cfg!(feature = "evaos-teams-managed")
        );
    }
}
