import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('ProofBattle Competitive 1v1 Full Lifecycle & A11y Suite', () => {
	test('landing page accessibility check (axe zero critical/serious)', async ({ page }) => {
		await page.goto('/?mock=1');

		// Accessibility audit
		const accessibilityScanResults = await new AxeBuilder({ page })
			.withTags(['wcag2a', 'wcag2aa'])
			.analyze();

		const seriousOrCritical = accessibilityScanResults.violations.filter(
			(v) => v.impact === 'serious' || v.impact === 'critical'
		);

		expect(
			seriousOrCritical,
			`Found ${seriousOrCritical.length} serious/critical a11y violations`
		).toEqual([]);
	});

	test('post-match result page accessibility check (axe zero critical/serious)', async ({
		page
	}) => {
		await page.goto('/result?mock=1');

		const accessibilityScanResults = await new AxeBuilder({ page })
			.withTags(['wcag2a', 'wcag2aa'])
			.analyze();

		const seriousOrCritical = accessibilityScanResults.violations.filter(
			(v) => v.impact === 'serious' || v.impact === 'critical'
		);

		expect(
			seriousOrCritical,
			`Found ${seriousOrCritical.length} serious/critical a11y violations`
		).toEqual([]);
	});

	test('game page direct navigation check: /game?mock=1', async ({ page }) => {
		await page.goto('/game?mock=1');
		await expect(page.getByText('Goal to Prove:')).toBeVisible();
	});

	test('full game lifecycle smoke: / -> /lobby -> /game -> /result', async ({ page }) => {
		// 1. Visit landing page with mock enabled
		await page.goto('/?mock=1');
		await expect(page.locator('h1')).toHaveText('ProofBattle');
		await expect(page.getByText('Prove Fast.')).toBeVisible();

		// Fill handle and click Play Ranked
		await page.fill('#username-input', 'TestProver');
		await page.getByRole('button', { name: 'Play Ranked' }).click();

		// 2. Arrive at /lobby
		await expect(page).toHaveURL(/\/lobby/);
		await expect(page.getByText('Finding Your Opponent...')).toBeVisible();

		// 3. Mock pairs and navigates to /game within ~1.5s
		await expect(page).toHaveURL(/\/game/, { timeout: 10000 });
		await expect(page.getByText('Goal to Prove:')).toBeVisible();
		await expect(page.getByRole('button', { name: 'Submit Proof' })).toBeVisible();

		// 4. Test Check Only diagnostic execution
		await page.getByRole('button', { name: 'Check Only' }).click();
		await expect(page.getByText('Compiler Feedback')).toBeVisible();

		// 5. Submit Proof (first is rejected, second is accepted per mock script)
		await page.getByRole('button', { name: 'Submit Proof' }).click();
		await expect(page.getByText('Proof rejected: goal not closed')).toBeVisible();

		// Submit second time -> Victory and ResultModal
		await page.getByRole('button', { name: 'Submit Proof' }).click();
		await expect(page.getByText('VICTORY')).toBeVisible();
		await expect(page.getByText('Theorem verified! Proof accepted.')).toBeVisible();
	});
});
