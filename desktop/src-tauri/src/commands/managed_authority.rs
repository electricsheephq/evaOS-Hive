pub(super) fn require_native_agent_authority() -> Result<(), String> {
    if cfg!(feature = "evaos-teams-managed") {
        Err("Managed agents are assigned by ElectricSheep".to_string())
    } else {
        Ok(())
    }
}
