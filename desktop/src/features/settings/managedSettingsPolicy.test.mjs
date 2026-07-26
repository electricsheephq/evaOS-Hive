import assert from "node:assert/strict";
import test from "node:test";

import { managedSettingsDisposition } from "./managedSettingsPolicy.ts";

test("managed Hive restores native product settings and hides only explicit exclusions", () => {
  for (const section of ["agents", "channel-templates", "custom-emoji"]) {
    assert.equal(managedSettingsDisposition(section), "restored", section);
  }
  for (const section of ["experimental", "local-archive", "updates"]) {
    assert.equal(managedSettingsDisposition(section), "native", section);
  }
  for (const section of ["compute", "hosted-communities", "mobile"]) {
    assert.equal(managedSettingsDisposition(section), "hidden", section);
  }
});
