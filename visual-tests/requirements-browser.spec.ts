import { test, expect } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const reports = [
  ["basic", "sample_projects/basic/out/index.html"],
  ["rust", "sample_projects/rust/out/index.html"],
] as const;

let generated = false;
test.beforeAll(() => {
  if (generated) return;
  for (const [, output] of reports) {
    const project = output.split("/out/")[0];
    execFileSync("cargo", ["run", "-q", "-p", "shallgraph", "--", "html", "--project-dir", project, "--output", `${project}/out`, "--combined"], { cwd: root, stdio: "pipe" });
  }
  generated = true;
});

test("basic requirements browser matches the desktop reference", async ({ page }) => {
  const file = resolve(root, reports[0][1]);
  expect(existsSync(file)).toBe(true);
  await page.goto(`file://${file}`);
  await expect(page.locator(".rq-root")).toHaveScreenshot("basic-desktop.png", { fullPage: true });
});

test("basic requirements browser is usable on mobile", async ({ page }) => {
  const file = resolve(root, reports[0][1]);
  await page.goto(`file://${file}`);
  await page.getByPlaceholder("Filter by ID or title").fill("login");
  await expect(page.locator(".rq-nav-item:visible")).toHaveCount(2);
  await expect(page.locator(".rq-root")).toHaveScreenshot("basic-mobile.png", { fullPage: true });
});

test("rust report exposes traceability links", async ({ page }) => {
  const file = resolve(root, reports[1][1]);
  await page.goto(`file://${file}`);
  await page.locator(".rq-nav-item").first().click();
  await expect(page.locator(".requirement-detail:visible")).toContainText("Satisfied by");
  await expect(page.locator(".rq-root")).toHaveScreenshot("rust-traceability.png", { fullPage: true });
});
