import { test } from '@playwright/test';

test('capture live app screenshots', async ({ page }) => {
	await page.goto('http://localhost:5173/');
	await page.waitForTimeout(1000);
	await page.screenshot({
		path: '/home/gulshansharma/.gemini/antigravity/brain/55900e62-6dd0-4a66-9d63-cbb51624059e/09_live_landing.png'
	});

	await page.fill('#username-input', 'Grandmaster_Lean');
	await page.click('button:has-text("Play Ranked")');
	await page.waitForTimeout(1000);
	await page.screenshot({
		path: '/home/gulshansharma/.gemini/antigravity/brain/55900e62-6dd0-4a66-9d63-cbb51624059e/10_live_lobby.png'
	});
});
