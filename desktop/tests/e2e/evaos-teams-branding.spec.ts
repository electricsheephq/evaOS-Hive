import { expect, test } from "@playwright/test";

import {
  TEST_IDENTITIES,
  installMockBridge,
  openCreateChannelDialog,
  openNewMessagePage,
} from "../helpers/bridge";

const FORBIDDEN_HOSTS = [
  "app.builderlab.xyz",
  "communities.buzz.xyz",
  "pairing.buzz.xyz",
  "push.buzz.xyz",
  "support.buzz.xyz",
  "telemetry.buzz.xyz",
  "github.com",
  "raw.githubusercontent.com",
  "block.github.io",
];

function isForbiddenHost(hostname: string) {
  return FORBIDDEN_HOSTS.some(
    (host) => hostname === host || hostname.endsWith(`.${host}`),
  );
}

test("managed active UI uses evaOS identity and update path stays local", async ({
  page,
}) => {
  expect(isForbiddenHost("tenant.communities.buzz.xyz")).toBe(true);
  expect(isForbiddenHost("api.github.com")).toBe(true);
  expect(isForbiddenHost("github.example")).toBe(false);

  const forbiddenRequests: string[] = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (isForbiddenHost(url.hostname)) {
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
  for (const nativeManagedCommand of [
    "search_users",
    "list_relay_agents",
    "list_personas",
    "list_teams",
    "list_managed_agents",
    "create_managed_agent",
    "create_channel",
    "open_dm",
    "add_channel_members",
  ]) {
    expect(launchCommands).not.toContain(nativeManagedCommand);
  }
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
  await expect(page.getByTestId("community-access-loading")).toHaveCount(0);
  await page.getByTestId("settings-nav-community-members").click();
  await expect(
    page.getByText("Workspace access", { exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Benjamin", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Sign in required", { exact: true }),
  ).toBeVisible();
  await page.getByLabel("Teammate email").fill("new.hive@example.com");
  await page.getByRole("button", { name: "Send invite" }).click();
  await expect(
    page.getByText("Invitation sent.", { exact: true }),
  ).toBeVisible();
  await page.getByLabel("Channel").selectOption({
    label: "general",
  });
  await page.getByLabel("Agent").selectOption({
    label: "Atris",
  });
  await page.getByRole("button", { name: "Add agent" }).click();
  await expect(
    page.getByText("Agent added to the channel.", { exact: true }),
  ).toBeVisible();
  const workspaceCommands = await page.evaluate(
    () =>
      (
        window as Window & {
          __BUZZ_E2E_COMMANDS__?: string[];
        }
      ).__BUZZ_E2E_COMMANDS__ ?? [],
  );
  expect(workspaceCommands).toContain("invite_hive_member");
  expect(workspaceCommands).toContain("add_hive_room_participant");

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
  await expect(about).toContainText("About Hive");
  await expect(about).toContainText("0.4.26-es.1");
  await expect(about).toContainText(
    "Hive by Electric Sheep. Open-source licenses and origin notices are included with the app.",
  );
  await expect(about).not.toContainText("evaos-teams");
  await expect(about).not.toContainText("com.electricsheephq.evaos.teams");
  await expect(about).not.toContainText("Buzz");
});

test("managed owner can open New Message and create a channel", async ({
  page,
}) => {
  await installMockBridge(page, { evaosTeamsManaged: true });
  await page.goto("/");
  await expect(page.getByTestId("app-sidebar")).toBeVisible();

  await openNewMessagePage(page);
  await expect(page.getByTestId("new-message-page")).toBeVisible();

  await page.getByTestId("new-dm-search").fill("Benjamin");
  await page.getByTestId(`new-dm-result-${TEST_IDENTITIES.bob.pubkey}`).click();
  await page.getByTestId("new-dm-search").fill("Alex");
  await page
    .getByTestId(`new-dm-result-${TEST_IDENTITIES.alice.pubkey}`)
    .click();
  const commandCountBeforeDm = await page.evaluate(
    () =>
      (
        window as Window & {
          __BUZZ_E2E_COMMANDS__?: string[];
        }
      ).__BUZZ_E2E_COMMANDS__?.length ?? 0,
  );
  await page.getByTestId("message-input").fill("Hive managed DM canary");
  await page.getByTestId("send-message").click();
  await expect(page.getByTestId("chat-title")).toBeVisible();
  const dmCommands = await page.evaluate(
    (offset) =>
      (
        window as Window & {
          __BUZZ_E2E_COMMANDS__?: string[];
        }
      ).__BUZZ_E2E_COMMANDS__?.slice(offset) ?? [],
    commandCountBeforeDm,
  );
  expect(dmCommands).toContain("open_hive_dm");
  expect(dmCommands).not.toContain("open_dm");

  await page.getByRole("button", { name: "Inbox" }).click();
  await openCreateChannelDialog(page);
  const commandCountBeforeCreate = await page.evaluate(
    () =>
      (
        window as Window & {
          __BUZZ_E2E_COMMANDS__?: string[];
        }
      ).__BUZZ_E2E_COMMANDS__?.length ?? 0,
  );
  const channelName = `hive-canary-${Date.now()}`;
  await page.getByTestId("create-channel-name").fill(channelName);
  await page.getByTestId("create-channel-submit").click();

  await expect(page).toHaveURL(/#\/channels\/[0-9a-f-]+$/);
  await page.getByTestId("channel-members-trigger").click();
  await expect(page.getByTestId("members-sidebar")).toBeVisible();
  await page.getByTestId("channel-management-search-users").fill("Alex");
  await page
    .getByTestId(`channel-user-search-result-${TEST_IDENTITIES.alice.pubkey}`)
    .click();
  await expect(
    page.locator("[data-testid^='sidebar-change-role-']"),
  ).toHaveCount(0);
  await expect(
    page.locator("[data-testid^='sidebar-remove-member-']"),
  ).toHaveCount(0);
  await page.keyboard.press("Escape");
  const commands = await page.evaluate(
    (offset) =>
      (
        window as Window & {
          __BUZZ_E2E_COMMANDS__?: string[];
        }
      ).__BUZZ_E2E_COMMANDS__?.slice(offset) ?? [],
    commandCountBeforeCreate,
  );
  expect(commands).toContain("create_hive_channel");
  expect(commands).toContain("add_hive_room_participant");
  expect(commands).not.toContain("create_channel");
  expect(commands).not.toContain("add_channel_members");
});

test("managed bootstrap purges content-bearing renderer persistence", async ({
  page,
}) => {
  await page.addInitScript(() => {
    window.localStorage.setItem(
      "buzz-drafts.v2:wss://relay.example:person",
      "private draft",
    );
    window.localStorage.setItem(
      "buzz-channel-messages.v1:wss://relay.example:room",
      "private message",
    );
    window.localStorage.setItem("buzz-theme", "buzz-dark");
  });
  await installMockBridge(page, { evaosTeamsManaged: true });
  await page.goto("/");
  await expect(page.getByTestId("app-sidebar")).toBeVisible();

  const persisted = await page.evaluate(() => ({
    draft: window.localStorage.getItem(
      "buzz-drafts.v2:wss://relay.example:person",
    ),
    message: window.localStorage.getItem(
      "buzz-channel-messages.v1:wss://relay.example:room",
    ),
    theme: window.localStorage.getItem("buzz-theme"),
  }));
  expect(persisted).toEqual({
    draft: null,
    message: null,
    theme: "buzz-dark",
  });
});

for (const fixture of [
  {
    phase: "signed_out" as const,
    expected: "Sign in to Hive",
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
      if (isForbiddenHost(url.hostname)) {
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
    await expect(page.getByAltText("Hive")).toBeVisible();
    await expect(page.locator("body")).not.toContainText("Builderlab");
    await expect(page.locator("body")).not.toContainText("Block");
    expect(forbiddenRequests).toEqual([]);
  });
}
