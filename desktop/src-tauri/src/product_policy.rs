use serde::Serialize;

use crate::product_contract::{
    HIVE_ARTIFACT_NAME, HIVE_BUNDLE_IDENTIFIER, HIVE_DEEP_LINK_SCHEME, HIVE_PRODUCT_NAME,
    HIVE_UPDATE_CHANNEL, HIVE_VERSION,
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
                product_name: HIVE_PRODUCT_NAME,
                version: HIVE_VERSION,
                bundle_identifier: HIVE_BUNDLE_IDENTIFIER,
                deep_link_scheme: HIVE_DEEP_LINK_SCHEME,
                artifact_name: HIVE_ARTIFACT_NAME,
                update_channel: HIVE_UPDATE_CHANNEL,
                updater_enabled: cfg!(buzz_updater_enabled),
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
            assert_eq!(policy.product_name, "Hive");
            assert_eq!(policy.deep_link_scheme, "buzz");
            assert!(!policy.upstream_hosted_services_enabled);
        } else {
            assert_eq!(policy.product_name, "Buzz");
            assert_eq!(policy.deep_link_scheme, "buzz");
            assert!(policy.upstream_hosted_services_enabled);
        }
    }
}
