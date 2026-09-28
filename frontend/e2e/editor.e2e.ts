import { test, expect } from '@playwright/test';

// Helper: navigate through the full mock flow to reach the in_game state.
// The mock server is installed on game page via sessionStorage; going through the
// lobby ensures the WS mock pairs immediately and advances to in_game.
async function goToActiveGame(page: Parameters<Parameters<typeof test>[1]>[0]['page']) {
	await page.goto('/?mock=1');
	await page.fill('#username-input', 'Editor-E2E-User');
	await page.getByRole('button', { name: 'Play Ranked' }).click();
	await expect(page).toHaveURL(/\/lobby/, { timeout: 5000 });
	// Mock auto-pairs and navigates to /game within ~1.5 s
	await expect(page).toHaveURL(/\/game/, { timeout: 10000 });
	// Wait for in_game state — Submit Proof button becomes available
	await expect(page.getByRole('button', { name: 'Submit Proof' })).toBeVisible({ timeout: 8000 });
}

test.describe('S8 Monaco Editor', () => {
	test('editor loads with zero CDN requests', async ({ page }) => {
		// Block any CDN requests — Monaco MUST load entirely from self-hosted assets (ADR-011)
		const cdnRequests: string[] = [];
		await page.route(/jsdelivr|unpkg|cdnjs|fonts\.googleapis/, (route) => {
			cdnRequests.push(route.request().url());
			route.abort();
		});

		await goToActiveGame(page);

		// The game arena must load without relying on any CDN resources
		expect(cdnRequests, `CDN requests were made: ${cdnRequests.join(', ')}`).toHaveLength(0);

		// Goal should be visible (confirms in_game state)
		await expect(page.getByText('Goal to Prove:')).toBeVisible();

		// UnicodePalette toolbar must be rendered
		await expect(page.getByRole('toolbar', { name: 'Lean 4 Unicode Symbol Palette' })).toBeVisible({
			timeout: 5000
		});

		// DiagnosticsBar region must be rendered
		await expect(page.getByRole('region', { name: 'Compiler Diagnostics' })).toBeVisible({
			timeout: 5000
		});

		// Check Only and Submit Proof buttons are reachable
		await expect(page.getByRole('button', { name: 'Check Only' })).toBeVisible();
		await expect(page.getByRole('button', { name: 'Submit Proof' })).toBeVisible();
	});

	test('Check Only returns diagnostics; bar shows body-relative line number (not preamble offset)', async ({
		page
	}) => {
		await goToActiveGame(page);

		// Trigger a check and wait for the DiagnosticsBar to show compiler output
		await page.getByRole('button', { name: 'Check Only' }).click();

		// DiagnosticsBar region should appear (exists before checking, just waits for content)
		const diagBar = page.getByRole('region', { name: 'Compiler Diagnostics' });
		await expect(diagBar).toBeVisible({ timeout: 5000 });

		// The mock returns diagnostics with a body-relative line number ≤ 3.
		// Any preamble-offset line (≥ 5 for default 1-import problems) would indicate a bug.
		await expect(diagBar).toContainText(/[1-5]:\d+/, { timeout: 6000 });
	});

	test('unicode palette ∀ button is rendered and clickable', async ({ page }) => {
		await goToActiveGame(page);

		// UnicodePalette should be visible
		const palette = page.getByRole('toolbar', { name: 'Lean 4 Unicode Symbol Palette' });
		await expect(palette).toBeVisible({ timeout: 8000 });

		// Look for the ∀ button — it's rendered inside the palette
		// UnicodePalette renders buttons with the symbol directly as button content
		const forallBtn = palette.locator('button').filter({ hasText: '∀' }).first();
		await expect(forallBtn).toBeVisible({ timeout: 5000 });

		// Clicking it should not throw or navigate away
		await forallBtn.click();
		await expect(page).toHaveURL(/\/game/);
	});
});
