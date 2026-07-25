# Hive 0.4.26-es.1 upstream dispositions

Hive 0.4.26-es.1 is a deliberate forward-port of the Electric Sheep managed
overlay onto exact upstream tag `v0.4.26` (`0096d710`). It is not a blind rebase.
The reviewed v0.4.24 dispositions remain recorded in Buzz issue #14 and PR #26;
the resulting fixes are now present through upstream ancestry rather than
duplicate cherry-picks.

## v0.4.25 commits

| Commit | Candidate disposition |
|---|---|
| `e341b09c` | Included; native community icon control is retained, while managed community selection remains server-owned. |
| `3d17195a` | Included; mobile release-check only, no macOS canary runtime effect. |
| `76aeae70` | Included; agent harness defaults are retained for Hermes, while managed agent creation uses the company projection. |
| `5ca36e7b` | Included; relay Git smart-HTTP gzip fix is safe and independent of the managed adapter. |
| `c86c4f59` | Included; focused-thread dismissal is user-visible upstream parity. |
| `9cc9652c` | Included; shared-compute UI remains in source, but laptop-local agent controls are not exposed in managed Hive. |
| `9081ab0e` | Included; actionable PR review behavior remains available to supported agent paths. |
| `72bbaece` | Included; channel-template styling is directly applicable to the restored browser. |
| `269ef357` | Included; runtime team-instruction parsing remains applicable to ACP runtimes. |
| `596386ee` | Included; Windows-only managed Node fallback has no Apple Silicon canary effect. |
| `9731cd81` | Included; native runtime installer error handling is retained, although provider setup remains inside Hermes. |
| `b78a684c` | Included; CI-only Windows/Linux workflows have no packaged macOS runtime effect. |
| `cfdea818` | Included; mobile-only channel/home UI has no macOS canary effect. |
| `fb4a801a` | Included; huddle failure cleanup is retained. Managed huddle creation still requires a control-plane adapter and is not claimed by this canary. |
| `3bd3a014` | Included and applicable; augmented PATH improves runtime/model discovery. |
| `f3981dbf` | Included; Windows-only PowerShell spawning has no Apple Silicon canary effect. |
| `0a9c26ee` | Included and applicable; ACP auth errors fail promptly with a re-auth hint. |
| `e8105d14` | Included as upstream lineage; product version is truthfully overridden to `0.4.26-es.1`. |

## v0.4.26 commits

| Commit | Candidate disposition |
|---|---|
| `bb445d3c` | Included; mobile QR scanning has no macOS canary effect. |
| `264a56a2` | Included; audit timestamp hashing is a correctness fix retained by the relay. |
| `8398468e` | Included; mobile composer only, no macOS canary effect. |
| `b8510ede` | Included; runtime setup copy is retained, with Hermes authentication truth corrected to setup checked on launch. |
| `e527d74f` | Included; macOS local-network explanation remains applicable. |
| `121033f9` | Included; deployment documentation only, no packaged runtime effect. |
| `bcca885b` | Included; generic VPN wording only and no Tailscale user requirement is added. |
| `60a171b1` | Included; webhook proxy bypass is a server reliability fix. |
| `384c72de` | Included; native community management UI remains in source, while managed Hive keeps one server-selected company workspace. |
| `50655ac0` | Included; mobile pairing style only, no macOS canary effect. |
| `c26bf594` | **Required security fix**; retained to block IPv6-transition SSRF targets. |
| `ab3af828` | **Required security fix**; retained to prevent unshared author-only event reads. |
| `50fadaa7` | Included; mobile navigation/creation only, no macOS canary effect. |
| `0096d710` | Included as exact upstream base; product version is `0.4.26-es.1`. |

The managed delta restores upstream channel browser, DM, forum, Agents, and
channel-management surfaces through server-scoped adapters. It does not weaken
the sole control-identity mutation boundary or make company-public channels
internet-public.
