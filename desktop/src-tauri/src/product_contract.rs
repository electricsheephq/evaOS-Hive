use serde_json::{json, Value};

pub(crate) const EVAOS_TEAMS_PRODUCT_NAME: &str = "Hive";
pub(crate) const EVAOS_TEAMS_VERSION: &str = "0.4.26-es.1";
pub(crate) const EVAOS_TEAMS_BUNDLE_IDENTIFIER: &str = "com.electricsheephq.evaos.teams";
pub(crate) const EVAOS_TEAMS_DEEP_LINK_SCHEME: &str = "evaos-teams";
pub(crate) const EVAOS_TEAMS_ARTIFACT_NAME: &str = "Hive-0.4.26-es.1-arm64.dmg";
pub(crate) const EVAOS_TEAMS_UPDATE_CHANNEL: &str = "managed-beta";
pub(crate) const EVAOS_TEAMS_CSP: &str = "default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost buzz-media: blob: data:; media-src 'self' asset: http://asset.localhost buzz-media: blob:; font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; worker-src 'self' blob:; object-src 'none'; base-uri 'none'; frame-src 'none'";

#[allow(dead_code)]
pub(crate) fn managed_tauri_overlay() -> Value {
    json!({
        "productName": EVAOS_TEAMS_PRODUCT_NAME,
        "mainBinaryName": EVAOS_TEAMS_PRODUCT_NAME,
        "version": EVAOS_TEAMS_VERSION,
        "identifier": EVAOS_TEAMS_BUNDLE_IDENTIFIER,
        "app": {
            "security": {
                "csp": EVAOS_TEAMS_CSP
            }
        },
        "plugins": {
            "updater": {
                "endpoints": []
            },
            "deep-link": {
                "desktop": {
                    "schemes": [EVAOS_TEAMS_DEEP_LINK_SCHEME]
                }
            }
        },
        "bundle": {
            "createUpdaterArtifacts": false,
            "targets": ["app", "dmg"],
            "icon": [
                "evaos-teams/icon.png",
                "evaos-teams/icon.icns"
            ],
            "resources": {
                "../../LICENSE": "licenses/Buzz-Apache-2.0.txt",
                "evaos-teams/NOTICE.txt": "licenses/Hive-NOTICE.txt"
            },
            "macOS": {
                "infoPlist": "evaos-teams/Info.plist",
                "dmg": {
                    "background": "evaos-teams/dmg-background.png"
                }
            }
        }
    })
}

#[allow(dead_code)]
pub(crate) fn updater_enabled(
    managed: bool,
    public_key: Option<&str>,
    endpoint: Option<&str>,
) -> Result<bool, &'static str> {
    let configured = public_key.is_some() || endpoint.is_some();
    if managed && configured {
        return Err("Hive managed builds reject BUZZ_UPDATER_PUBLIC_KEY and BUZZ_UPDATER_ENDPOINT");
    }
    Ok(!managed && public_key.is_some() && endpoint.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_overlay_pins_package_identity_and_disables_updater() {
        let overlay = managed_tauri_overlay();
        assert_eq!(overlay["productName"], EVAOS_TEAMS_PRODUCT_NAME);
        assert_eq!(overlay["mainBinaryName"], EVAOS_TEAMS_PRODUCT_NAME);
        assert_eq!(overlay["version"], EVAOS_TEAMS_VERSION);
        assert_eq!(overlay["identifier"], EVAOS_TEAMS_BUNDLE_IDENTIFIER);
        assert_eq!(overlay["app"]["security"]["csp"], EVAOS_TEAMS_CSP);
        assert!(!EVAOS_TEAMS_CSP.contains("http://127.0.0.1"));
        assert!(!EVAOS_TEAMS_CSP.contains("http://localhost"));
        assert!(EVAOS_TEAMS_CSP.contains("buzz-media:"));
        assert!(!EVAOS_TEAMS_CSP.contains("https:"));
        assert!(!EVAOS_TEAMS_CSP.contains("wss:"));
        assert_eq!(
            overlay["plugins"]["deep-link"]["desktop"]["schemes"],
            json!([EVAOS_TEAMS_DEEP_LINK_SCHEME])
        );
        assert_eq!(overlay["plugins"]["updater"]["endpoints"], json!([]));
        assert_eq!(overlay["bundle"]["createUpdaterArtifacts"], false);
        assert_eq!(
            overlay["bundle"]["resources"]["../../LICENSE"],
            "licenses/Buzz-Apache-2.0.txt"
        );
        assert_eq!(
            overlay["bundle"]["macOS"]["dmg"]["background"],
            "evaos-teams/dmg-background.png"
        );
    }

    #[test]
    fn checked_in_package_contract_matches_rust_constants() {
        let contract: Value =
            serde_json::from_str(include_str!("../evaos-teams/package-contract.json")).unwrap();
        assert_eq!(contract["productName"], EVAOS_TEAMS_PRODUCT_NAME);
        assert_eq!(contract["version"], EVAOS_TEAMS_VERSION);
        assert_eq!(contract["bundleIdentifier"], EVAOS_TEAMS_BUNDLE_IDENTIFIER);
        assert_eq!(contract["deepLinkScheme"], EVAOS_TEAMS_DEEP_LINK_SCHEME);
        assert_eq!(contract["artifactName"], EVAOS_TEAMS_ARTIFACT_NAME);
        assert_eq!(contract["updateChannel"], EVAOS_TEAMS_UPDATE_CHANNEL);
    }

    #[test]
    fn managed_build_rejects_every_updater_configuration() {
        for (key, endpoint) in [
            (Some("key"), None),
            (None, Some("https://updates.example")),
            (Some("key"), Some("https://updates.example")),
        ] {
            assert!(updater_enabled(true, key, endpoint).is_err());
        }
        assert_eq!(updater_enabled(true, None, None), Ok(false));
    }

    #[test]
    fn native_updater_contract_is_unchanged() {
        assert_eq!(updater_enabled(false, None, None), Ok(false));
        assert_eq!(updater_enabled(false, Some("key"), None), Ok(false));
        assert_eq!(
            updater_enabled(false, Some("key"), Some("https://updates.example")),
            Ok(true)
        );
    }
}
