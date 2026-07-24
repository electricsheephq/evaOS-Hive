import { expect, test } from "@playwright/test";

import { installMockBridge } from "../helpers/bridge";

const FORBIDDEN_HOSTS = [
  "app.builderlab.xyz",
  "communities.buzz.xyz",
  "github.com",
];

test("managed active UI uses evaOS identity and update path stays local", async ({
  page,
}) => {
  const forbiddenRequests: string[] = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (FORBIDDEN_HOSTS.includes(url.hostname)) {
      forbiddenRequests.push(request.url());
    }
  });
  await page.addInitScript(() => {
    window.localStorage.setItem("buzz-theme", "buzz-dark");
  });
  await installMockBridge(page, { evaosTeamsManaged: true });
  await page.goto("/");
  await expect(page.getByTestId("app-sidebar")).toBeVisible();
  await expect(page.locator("html")).toHaveClass(/dark/);
  await expect(page.locator("body")).not.toContainText("Builderlab");
  await expect(page.locator("body")).not.toContainText("Block");
  await expect(page.locator("body")).not.toContainText("Buzz");

  const launchCommands = await page.evaluate(
    () =>
      (
        window as Window & {
          __BUZZ_E2E_COMMANDS__?: string[];
        }
      ).__BUZZ_E2E_COMMANDS__ ?? [],
  );
  expect(
    launchCommands.some(
      (command) =>
        command.includes("builderlab") || command.startsWith("plugin:updater"),
    ),
  ).toBe(false);
  expect(forbiddenRequests).toEqual([]);

  await page.getByTestId("sidebar-profile-card").click();
  await page.getByTestId("profile-popover-settings").click();
  await expect(page.getByTestId("settings-nav-about")).toBeVisible();
  for (const hidden of [
    "agents",
    "compute",
    "experimental",
    "hosted-communities",
    "local-archive",
    "mobile",
  ]) {
    await expect(page.getByTestId(`settings-nav-${hidden}`)).toHaveCount(0);
  }
  await expect(page.getByTestId("profile-private-key-row")).toHaveCount(0);
  await expect(page.getByTestId("settings-signout")).toHaveCount(0);
  await expect(page.getByTestId("settings-managed-signout")).toBeVisible();

  await page.evaluate(() => {
    const testWindow = window as Window & {
      __BUZZ_E2E_COMMANDS__?: string[];
    };
    testWindow.__BUZZ_E2E_COMMANDS__ = [];
  });
  await page.getByTestId("settings-nav-updates").click();
  await expect(page.getByTestId("settings-panel-updates")).toContainText(
    "Managed updates",
  );
  await expect(page.getByTestId("settings-panel-updates")).toContainText(
    "managed-beta",
  );
  await expect(
    page.getByRole("button", { name: /check|update now|download/i }),
  ).toHaveCount(0);
  expect(
    await page.evaluate(
      () =>
        (
          window as Window & {
            __BUZZ_E2E_COMMANDS__?: string[];
          }
        ).__BUZZ_E2E_COMMANDS__ ?? [],
    ),
  ).toEqual([]);

  await page.getByTestId("settings-nav-about").click();
  const about = page.getByTestId("settings-panel-about");
  await expect(about).toContainText("About evaOS Teams");
  await expect(about).toContainText("0.4.23-es.1");
  await expect(about).toContainText("com.electricsheephq.evaos.teams");
  await expect(about).toContainText("evaos-teams://");
  await expect(about).toContainText(
    "Built from Buzz by Block, used under the Apache License 2.0.",
  );
});

for (const fixture of [
  {
    phase: "signed_out" as const,
    expected: "Sign in to evaOS Teams",
  },
  {
    phase: "keychain_locked" as const,
    expected: "Unlock macOS Keychain",
  },
]) {
  test(`managed ${fixture.phase} gate has no upstream brand or request`, async ({
    page,
  }) => {
    const forbiddenRequests: string[] = [];
    page.on("request", (request) => {
      const url = new URL(request.url());
      if (FORBIDDEN_HOSTS.includes(url.hostname)) {
        forbiddenRequests.push(request.url());
      }
    });
    await installMockBridge(page, {
      evaosTeamsManaged: true,
      evaosTeamsPhase: fixture.phase,
    });
    await page.goto("/");
    await expect(
      page.getByText(fixture.expected, { exact: true }),
    ).toBeVisible();
    await expect(page.getByAltText("evaOS Teams")).toBeVisible();
    await expect(page.locator("body")).not.toContainText("Builderlab");
    await expect(page.locator("body")).not.toContainText("Block");
    expect(forbiddenRequests).toEqual([]);
  });
}
