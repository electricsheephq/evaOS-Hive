use serde::Serialize;

use crate::product_contract::{
    EVAOS_TEAMS_ARTIFACT_NAME, EVAOS_TEAMS_BUNDLE_IDENTIFIER, EVAOS_TEAMS_DEEP_LINK_SCHEME,
    EVAOS_TEAMS_PRODUCT_NAME, EVAOS_TEAMS_UPDATE_CHANNEL, EVAOS_TEAMS_VERSION,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopProductPolicy {
    managed: bool,
    product_name: &'static str,
    version: &'static str,
    bundle_identifier: &'static str,
    deep_link_scheme: &'static str,
    artifact_name: &'static str,
    update_channel: &'static str,
    updater_enabled: bool,
    upstream_hosted_services_enabled: bool,
    origin_attribution: &'static str,
}

impl DesktopProductPolicy {
    fn current() -> Self {
        if cfg!(feature = "evaos-teams-managed") {
            Self {
                managed: true,
                product_name: EVAOS_TEAMS_PRODUCT_NAME,
                version: EVAOS_TEAMS_VERSION,
                bundle_identifier: EVAOS_TEAMS_BUNDLE_IDENTIFIER,
                deep_link_scheme: EVAOS_TEAMS_DEEP_LINK_SCHEME,
                artifact_name: EVAOS_TEAMS_ARTIFACT_NAME,
                update_channel: EVAOS_TEAMS_UPDATE_CHANNEL,
                updater_enabled: false,
                upstream_hosted_services_enabled: false,
                origin_attribution: "Built from Buzz by Block, used under the Apache License 2.0.",
            }
        } else {
            Self {
                managed: false,
                product_name: "Buzz",
                version: env!("CARGO_PKG_VERSION"),
                bundle_identifier: "xyz.block.buzz.app",
                deep_link_scheme: "buzz",
                artifact_name: "",
                update_channel: "upstream",
                updater_enabled: cfg!(buzz_updater_enabled),
                upstream_hosted_services_enabled: true,
                origin_attribution: "Buzz by Block, licensed under the Apache License 2.0.",
            }
        }
    }
}

#[tauri::command]
pub(crate) fn get_desktop_product_policy() -> DesktopProductPolicy {
    DesktopProductPolicy::current()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_policy_matches_compile_time_variant() {
        let policy = DesktopProductPolicy::current();
        assert_eq!(policy.managed, cfg!(feature = "evaos-teams-managed"));
        if policy.managed {
            assert_eq!(policy.product_name, "evaOS Teams");
            assert_eq!(policy.deep_link_scheme, "evaos-teams");
            assert!(!policy.updater_enabled);
            assert!(!policy.upstream_hosted_services_enabled);
        } else {
            assert_eq!(policy.product_name, "Buzz");
            assert_eq!(policy.deep_link_scheme, "buzz");
            assert!(policy.upstream_hosted_services_enabled);
        }
    }
}
