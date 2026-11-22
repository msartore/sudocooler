// Headless smoke test of the Rust root: `nsh test`.
import { expect, open, test } from "nsh/test";

test("sudocooler renders its dashboard", async () => {
  const app = await open(".");
  expect(app.texts()).toContain("sudocooler");
  expect(app.texts()).toContain("Overview");
  expect(app.texts()).toContain("CPU PACKAGE TEMPERATURE");
});

test("settings opens from the home top bar", async () => {
  const app = await open(".");
  await app.press("Settings");
  expect(app.texts()).toContain("Start at login");
  expect(app.texts()).toContain("Software update");
  expect(app.texts()).toContain("msartore.dev");
});
