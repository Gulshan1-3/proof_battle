import { test } from '@playwright/test';

test('capture live app screenshots', async ({ page }) => {
	const outDir = process.env.ARTIFACT_DIR || './screenshots';
	await page.goto('http://localhost:5173/');
	await page.waitForTimeout(1000);
	await page.screenshot({
		path: `${outDir}/09_live_landing.png`
	});

	await page.fill('#username-input', 'Grandmaster_Lean');
	await page.click('button:has-text("Play Ranked")');
	await page.waitForTimeout(1000);
	await page.screenshot({
		path: `${outDir}/10_live_lobby.png`
	});
});
