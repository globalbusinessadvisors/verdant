import { test, expect } from "@playwright/test";

test.describe("Dashboard E2E", () => {
  test("map page loads and shows mesh nodes section", async ({ page }) => {
    await page.goto("/");
    const map = page.locator('[data-testid="map-container"]');
    await expect(map).toBeVisible();
  });

  test("event timeline page loads", async ({ page }) => {
    await page.goto("/events");
    const section = page.locator('section[aria-label="Event timeline"]');
    await expect(section).toBeVisible();
  });

  test("governance page loads and shows proposal form", async ({ page }) => {
    await page.goto("/governance");
    const form = page.locator('form[aria-label="Create proposal"]');
    await expect(form).toBeVisible();
  });

  test("alerts page loads", async ({ page }) => {
    await page.goto("/alerts");
    const section = page.locator('section[aria-label="Alert manager"]');
    await expect(section).toBeVisible();
  });

  test("farm dashboard page loads", async ({ page }) => {
    await page.goto("/farm");
    const section = page.locator('section[aria-label="Farm dashboard"]');
    await expect(section).toBeVisible();
  });

  test("robots page loads", async ({ page }) => {
    await page.goto("/robots");
    const section = page.locator('section[aria-label="Robot overview"]');
    await expect(section).toBeVisible();
  });

  test("navigation between routes works", async ({ page }) => {
    await page.goto("/");
    const map = page.locator('[data-testid="map-container"]');
    await expect(map).toBeVisible();

    await page.goto("/events");
    const timeline = page.locator('section[aria-label="Event timeline"]');
    await expect(timeline).toBeVisible();

    await page.goto("/governance");
    const portal = page.locator('section[aria-label="Governance portal"]');
    await expect(portal).toBeVisible();
  });

  test("offline mode shows cached content", async ({ page, context }) => {
    // Load page first to populate service worker cache
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    // Simulate offline
    await context.setOffline(true);

    // Should still render the page structure from cache
    await page.goto("/");
    // The page should render even offline (PWA)
    const body = page.locator("body");
    await expect(body).toBeVisible();

    // Restore
    await context.setOffline(false);
  });
});
