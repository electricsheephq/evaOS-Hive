use serde_json::{json, Value};

pub(crate) const HIVE_PRODUCT_NAME: &str = "Hive";
pub(crate) const HIVE_VERSION: &str = "0.5.1-es.1";
pub(crate) const HIVE_BUNDLE_IDENTIFIER: &str = "com.electricsheephq.evaos.teams";
pub(crate) const HIVE_DEEP_LINK_SCHEME: &str = "buzz";
pub(crate) const HIVE_ARTIFACT_NAME: &str = "Hive-0.5.1-es.1-arm64.dmg";
pub(crate) const HIVE_UPDATE_CHANNEL: &str = "hive-internal";
#[allow(dead_code)] // Used by build.rs through this shared module.
pub(crate) const HIVE_UPDATE_ENDPOINT: &str = "https://github.com/electricsheephq/evaOS-Hive/releases/download/hive-desktop-latest/latest.json";

#[allow(dead_code)] // Used by build.rs through this shared module.
pub(crate) fn managed_tauri_overlay(updater_public_key: Option<&str>) -> Value {
    let updater_enabled = updater_public_key.is_some();
    json!({
        "productName": HIVE_PRODUCT_NAME,
        "mainBinaryName": HIVE_PRODUCT_NAME,
        "version": HIVE_VERSION,
        "identifier": HIVE_BUNDLE_IDENTIFIER,
        "plugins": {
            "updater": {
                "endpoints": if updater_enabled { json!([HIVE_UPDATE_ENDPOINT]) } else { json!([]) },
                "pubkey": updater_public_key.unwrap_or_default()
            },
            "deep-link": {
                "desktop": {
                    "schemes": [HIVE_DEEP_LINK_SCHEME]
                }
            }
        },
        "bundle": {
            "createUpdaterArtifacts": updater_enabled,
            "targets": ["app", "dmg"],
            "icon": ["hive/icon.png", "hive/icon.icns"],
            "resources": {
                "../../LICENSE": "licenses/Buzz-Apache-2.0.txt",
                "hive/NOTICE.txt": "licenses/Hive-NOTICE.txt"
            },
            "macOS": {
                "infoPlist": "hive/Info.plist",
                "dmg": {
                    "background": "hive/dmg-background.png"
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_overlay_uses_only_the_hive_release_channel() {
        let overlay = managed_tauri_overlay(Some("signed-public-key"));
        assert_eq!(overlay["productName"], HIVE_PRODUCT_NAME);
        assert_eq!(overlay["version"], HIVE_VERSION);
        assert_eq!(overlay["identifier"], HIVE_BUNDLE_IDENTIFIER);
        assert_eq!(
            overlay["plugins"]["updater"]["endpoints"],
            json!([HIVE_UPDATE_ENDPOINT])
        );
        assert_eq!(overlay["plugins"]["updater"]["pubkey"], "signed-public-key");
        assert_eq!(overlay["bundle"]["createUpdaterArtifacts"], true);
    }

    #[test]
    fn unsigned_internal_build_has_no_update_authority() {
        let overlay = managed_tauri_overlay(None);
        assert_eq!(overlay["plugins"]["updater"]["endpoints"], json!([]));
        assert_eq!(overlay["plugins"]["updater"]["pubkey"], "");
        assert_eq!(overlay["bundle"]["createUpdaterArtifacts"], false);
    }

    #[test]
    fn checked_in_package_contract_matches_constants() {
        let contract: Value =
            serde_json::from_str(include_str!("../hive/package-contract.json")).unwrap();
        assert_eq!(contract["productName"], HIVE_PRODUCT_NAME);
        assert_eq!(contract["version"], HIVE_VERSION);
        assert_eq!(contract["bundleIdentifier"], HIVE_BUNDLE_IDENTIFIER);
        assert_eq!(contract["deepLinkScheme"], HIVE_DEEP_LINK_SCHEME);
        assert_eq!(contract["artifactName"], HIVE_ARTIFACT_NAME);
        assert_eq!(contract["updateChannel"], HIVE_UPDATE_CHANNEL);
        assert_eq!(contract["updateEndpoint"], HIVE_UPDATE_ENDPOINT);
    }
}
