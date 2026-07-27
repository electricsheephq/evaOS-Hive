import { expect, test } from "@playwright/test";

import { installMockBridge } from "../helpers/bridge";

const HIVE_POLICY = {
  managed: true,
  productName: "Hive",
  version: "0.4.26-es.2",
  bundleIdentifier: "com.electricsheephq.evaos.teams",
  deepLinkScheme: "evaos-teams",
  artifactName: "Hive-0.4.26-es.2-arm64.dmg",
  updateChannel: "hive-internal",
  updaterEnabled: true,
  upstreamHostedServicesEnabled: false,
  originAttribution: "Built from Buzz by Block, licensed under Apache-2.0.",
};

const BUZZ_AGENT_RUNTIME = {
  id: "buzz-agent",
  label: "Buzz Agent",
  avatar_url: "",
  availability: "available",
  command: "buzz-agent",
  binary_path: "/Applications/Hive.app/Contents/MacOS/buzz-agent",
  default_args: [],
  mcp_command: "buzz-dev-mcp",
  install_hint: "Ships with the Buzz desktop app.",
  install_instructions_url: "https://github.com/block/buzz",
  can_auto_install: false,
  underlying_cli_path: null,
  node_required: false,
  auth_status: { status: "not_applicable" },
};

async function openSettings(page: import("@playwright/test").Page) {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByTestId("open-settings").click();
  await page.getByTestId("profile-popover-settings").click();
  await expect(page.getByTestId("settings-view")).toBeVisible();
}

test("managed Hive productizes presentation while preserving Buzz attribution", async ({
  page,
}) => {
  await installMockBridge(page, {
    desktopProductPolicy: HIVE_POLICY,
    acpRuntimesCatalog: [BUZZ_AGENT_RUNTIME],
  });
  await openSettings(page);

  await page.getByTestId("settings-nav-local-archive").click();
  const archive = page.getByTestId("settings-local-archive");
  await expect(archive).toContainText("Hive nest");
  await expect(archive).not.toContainText("Buzz nest");

  await page.getByTestId("settings-nav-appearance").click();
  await expect(page.getByTestId("theme-pair-buzz")).toContainText("Hive");
  await expect(page.getByTestId("theme-pair-buzz")).not.toContainText("Buzz");

  await page.getByTestId("settings-nav-agents").click();
  const runtimes = page.getByTestId("settings-agent-runtimes");
  await expect(runtimes).toContainText("Hive Agent");
  await expect(runtimes).not.toContainText("Buzz Agent");

  await page.getByTestId("settings-nav-custom-emoji").click();
  await expect(page.getByTestId("settings-custom-emoji")).toContainText(
    "Hive will suggest a name",
  );

  await page.getByTestId("settings-nav-about").click();
  await expect(page.getByTestId("settings-about")).toContainText("About Hive");
  await expect(page.getByTestId("settings-about")).toContainText(
    "Built from Buzz by Block",
  );
});
