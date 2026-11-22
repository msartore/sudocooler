// Headless smoke test of the Rust root: `nsh test`.
import { expect, open, test } from "nsh/test";

test("sudocooler renders its dashboard", async () => {
  const app = await open(".");
  expect(app.texts()).toContain("SUDOKOOLER");
  expect(app.texts()).toContain("Overview");
  expect(app.texts()).toContain("CPU PACKAGE TEMPERATURE");
});

test("the device list is reachable", async () => {
  const app = await open(".");
  await app.press("Devices");
  expect(app.texts()).toContain("USB HID interfaces");
  expect(app.texts()).toContain("381C:0003 · sk700v · verified");
});
