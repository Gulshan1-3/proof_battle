import { chromium } from 'playwright';
import fs from 'fs';
import path from 'path';

const RECORDINGS_DIR = '/home/gulshansharma/proof_battle/recordings_raw';
// Clean out older raw recordings
if (fs.existsSync(RECORDINGS_DIR)) {
	for (const f of fs.readdirSync(RECORDINGS_DIR)) {
		try {
			fs.unlinkSync(path.join(RECORDINGS_DIR, f));
		} catch (e) {}
	}
} else {
	fs.mkdirSync(RECORDINGS_DIR, { recursive: true });
}

async function run() {
	console.log('Launching browser for showcase video recording...');
	const browser = await chromium.launch({
		headless: true
	});

	const context = await browser.newContext({
		recordVideo: {
			dir: RECORDINGS_DIR,
			size: { width: 1920, height: 1080 }
		},
		viewport: { width: 1920, height: 1080 }
	});

	const page = await context.newPage();

	// Mock REST API endpoints for History and Replay
	await page.route('**/api/stats*', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				player_id: '00000000-0000-0000-0000-000000000001',
				games_played: 42,
				wins: 34,
				losses: 6,
				draws: 2,
				win_rate: 81.0,
				current_rating: 1420,
				categories: [
					{ category: 'logic', games_played: 18, wins: 16, win_rate: 88.9 },
					{ category: 'arithmetic', games_played: 15, wins: 12, win_rate: 80.0 },
					{ category: 'algebra', games_played: 9, wins: 6, win_rate: 66.7 }
				]
			})
		});
	});

	await page.route('**/api/history*', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify([
				{
					id: 'match-uuid-101',
					room_id: 'room-omega-7',
					opponent_id: 'opp-uuid-202',
					opponent_username: 'Grandmaster_Lean',
					opponent_elo: 1395,
					outcome: 'Won',
					elo_delta: 24,
					problem_goal: '∀ n : ℕ, n + 0 = n',
					problem_category: 'arithmetic',
					problem_difficulty: 1,
					rated: true,
					duration_ms: 14200,
					created_at: '2026-10-04T18:40:00Z'
				},
				{
					id: 'match-uuid-102',
					room_id: 'room-alpha-9',
					opponent_id: 'opp-uuid-303',
					opponent_username: 'TacticProver',
					opponent_elo: 1440,
					outcome: 'Won',
					elo_delta: 18,
					problem_goal: '∀ a b : ℕ, a + b = b + a',
					problem_category: 'algebra',
					problem_difficulty: 2,
					rated: true,
					duration_ms: 28500,
					created_at: '2026-10-04T17:15:00Z'
				},
				{
					id: 'match-uuid-103',
					room_id: 'room-beta-4',
					opponent_id: 'opp-uuid-404',
					opponent_username: 'KernelHacker',
					opponent_elo: 1480,
					outcome: 'Lost',
					elo_delta: -16,
					problem_goal: '∀ P Q : Prop, P ∧ Q → Q ∧ P',
					problem_category: 'logic',
					problem_difficulty: 3,
					rated: true,
					duration_ms: 45000,
					created_at: '2026-10-04T16:00:00Z'
				}
			])
		});
	});

	await page.route('**/api/matches/*', async (route) => {
		await route.fulfill({
			status: 200,
			contentType: 'application/json',
			body: JSON.stringify({
				id: 'match-uuid-101',
				room_id: 'room-omega-7',
				player1_id: '00000000-0000-0000-0000-000000000001',
				player1_username: 'Grandmaster_Lean',
				player1_elo_before: 1396,
				player1_elo_after: 1420,
				elo_delta_p1: 24,
				player2_id: 'opp-uuid-202',
				player2_username: 'Challenger_AI',
				player2_elo_before: 1395,
				player2_elo_after: 1371,
				elo_delta_p2: -24,
				problem_id: 'prob-nat-add-zero',
				problem_goal: '∀ n : ℕ, n + 0 = n',
				problem_category: 'arithmetic',
				problem_difficulty: 1,
				winner_id: '00000000-0000-0000-0000-000000000001',
				outcome: 'Won',
				winning_proof: 'intro n\nsimp',
				canonical_proof: 'intro n\nsimp',
				rated: true,
				duration_ms: 14200,
				created_at: '2026-10-04T18:40:00Z'
			})
		});
	});

	const chapters = [];
	const startWallClock = Date.now();
	function markChapter(title, subtitle) {
		const elapsedSec = (Date.now() - startWallClock) / 1000;
		console.log(`[CHAPTER] ${elapsedSec.toFixed(2)}s: ${title} - ${subtitle}`);
		chapters.push({
			start: elapsedSec,
			title,
			subtitle
		});
	}

	// =========================================================================
	// 1. Landing Page
	// =========================================================================
	markChapter('Combatant Setup', 'Handle Registration & Telemetry');
	await page.goto('http://localhost:4173/?mock=1');
	await page.waitForTimeout(1500);

	// Type username with realistic human cadence
	const usernameInput = page.locator('#username-input');
	await usernameInput.click();
	await usernameInput.fill('');
	const handle = 'Grandmaster_Lean';
	for (const char of handle) {
		await usernameInput.pressSequentially(char, { delay: 85 });
	}
	await page.waitForTimeout(1500);

	// Hover over game mode buttons
	const privateBtn = page.getByRole('button', { name: 'Private Duel' });
	if (await privateBtn.isVisible()) {
		await privateBtn.hover();
		await page.waitForTimeout(800);
	}

	// =========================================================================
	// 2. Private Duel Room Controls
	// =========================================================================
	markChapter('Private Rooms', 'Custom 6-Character Duel Lobbies');
	await page.goto('http://localhost:4173/lobby?mock=1&private=1');
	await page.waitForTimeout(2000);

	// Switch between Create and Join tabs
	const joinTab = page.getByRole('button', { name: /Join/i }).first();
	if (await joinTab.isVisible()) {
		await joinTab.hover();
		await page.waitForTimeout(600);
		await joinTab.click();
		await page.waitForTimeout(1500);
	}

	// =========================================================================
	// 3. Ranked Matchmaking Queue
	// =========================================================================
	markChapter('Matchmaking Radar', 'Dynamic Elo Bracket Search (+/- 150)');
	await page.goto('http://localhost:4173/lobby?mock=1');
	await page.waitForTimeout(2500); // Showcase radar sweep and queue search

	// =========================================================================
	// 4. Live 1v1 Battle Arena
	// =========================================================================
	markChapter('1v1 Battle Arena', 'Formal Goal, Imports & Timer');
	await page.waitForURL(/\/game/, { timeout: 15000 });
	await page.waitForTimeout(2200); // Showcase goal statement, opponent card, countdown timer

	// =========================================================================
	// 5. Lean 4 Palette & Real-Time Diagnostics
	// =========================================================================
	markChapter('Unicode Palette & Diagnostics', 'Lean Math Symbols & Elaboration Feedback');
	const palette = page.getByRole('toolbar', { name: 'Lean 4 Unicode Symbol Palette' });
	if (await palette.isVisible()) {
		const symbolsToClick = ['∀', '→', 'ℕ'];
		for (const sym of symbolsToClick) {
			const btn = palette.locator('button').filter({ hasText: sym }).first();
			if (await btn.isVisible()) {
				await btn.hover();
				await page.waitForTimeout(300);
				await btn.click();
				await page.waitForTimeout(300);
			}
		}
	}

	// Trigger Check Only to showcase real-time compiler diagnostics feedback
	const checkBtn = page.getByRole('button', { name: 'Check Only' });
	await checkBtn.hover();
	await page.waitForTimeout(400);
	await checkBtn.click();
	await page.waitForTimeout(2500); // View diagnostics bar with red squiggly error marker

	// =========================================================================
	// 6. Authoring verified proof in Monaco editor
	// =========================================================================
	markChapter('Lean 4 Proof Authoring', 'Tactic Solving in Monaco Editor');
	const monacoArea = page.locator('.monaco-editor').first();
	await monacoArea.click();
	await page.keyboard.press('Control+A');
	await page.keyboard.press('Backspace');
	await page.waitForTimeout(400);

	const proofLines = ['intro n', 'simp'];
	for (const line of proofLines) {
		for (const char of line) {
			await page.keyboard.press(char);
			await page.waitForTimeout(65);
		}
		await page.keyboard.press('Enter');
		await page.waitForTimeout(250);
	}
	await page.waitForTimeout(1500);

	// =========================================================================
	// 7. Formal Verification & Victory Resolution
	// =========================================================================
	markChapter('Formal Verification', 'Match Victory & Rating Update (+24 ELO)');
	const submitBtn = page.getByRole('button', { name: 'Submit Proof' });
	await submitBtn.hover();
	await page.waitForTimeout(400);
	await submitBtn.click();
	await page.waitForTimeout(1200);

	// Second submit accepts and closes match
	await submitBtn.click();
	await page.waitForTimeout(4500); // Savor victory modal celebration, ELO delta +24, stats

	// =========================================================================
	// 8. Post-Match Result Breakdown
	// =========================================================================
	markChapter('Post-Match Breakdown', 'Submitted Proof vs Canonical Mathlib Solution');
	await page.goto('http://localhost:4173/result?mock=1');
	await page.waitForTimeout(4000); // Showcase winning proof vs canonical proof side by side

	// =========================================================================
	// 9. Match History & Performance Analytics Dashboard
	// =========================================================================
	markChapter('Performance Analytics', 'Career Rating, Win Rate & Match Logs');
	await page.goto('http://localhost:4173/history?mock=1');
	await page.waitForTimeout(2000);

	// Smooth scroll down to showcase match logs table
	await page.evaluate(() => window.scrollBy({ top: 350, behavior: 'smooth' }));
	await page.waitForTimeout(2000);
	await page.evaluate(() => window.scrollBy({ top: -350, behavior: 'smooth' }));
	await page.waitForTimeout(1800);

	// =========================================================================
	// 10. Interactive Proof Replay Viewer
	// =========================================================================
	markChapter('Proof Replay Viewer', 'Step-by-Step Match & Tactic Inspector');
	await page.goto('http://localhost:4173/replay/match-uuid-101?mock=1');
	await page.waitForTimeout(4000); // Showcase step-by-step replay inspection

	const totalDurationSec = (Date.now() - startWallClock) / 1000;
	console.log(`Total walkthrough recording duration: ${totalDurationSec.toFixed(2)}s`);

	// Compute chapter end boundaries
	for (let i = 0; i < chapters.length; i++) {
		const nextStart = i + 1 < chapters.length ? chapters[i + 1].start : totalDurationSec;
		chapters[i].end = nextStart;
	}

	fs.writeFileSync(
		path.join(RECORDINGS_DIR, 'chapters.json'),
		JSON.stringify({ total_duration: totalDurationSec, chapters }, null, 2)
	);

	await page.close();
	await context.close();
	await browser.close();

	const recordedFiles = fs.readdirSync(RECORDINGS_DIR).filter((f) => f.endsWith('.webm'));
	console.log('Recorded raw video file(s):', recordedFiles);
}

run().catch((err) => {
	console.error('Recording error:', err);
	process.exit(1);
});
