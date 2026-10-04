import { test, expect } from '@playwright/test';

const ARTIFACT_DIR =
	'/home/gulshansharma/.gemini/antigravity/brain/55900e62-6dd0-4a66-9d63-cbb51624059e';

test('capture complete application showcase', async ({ page }) => {
	// Set viewport to 1440x900 for high quality desktop capture
	await page.setViewportSize({ width: 1440, height: 900 });

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

	// =========================================================================
	// 1. Landing Page
	// =========================================================================
	await page.goto('/?mock=1');
	await expect(page.locator('h1')).toHaveText('ProofBattle');
	await page.fill('#username-input', 'Grandmaster_Lean');
	await page.waitForTimeout(500);
	await page.screenshot({ path: `${ARTIFACT_DIR}/01_landing_showcase.png` });

	// =========================================================================
	// 2. Lobby Page (Queue & Matchmaking)
	// =========================================================================
	await page.getByRole('button', { name: 'Play Ranked' }).click();
	await expect(page).toHaveURL(/\/lobby/);
	await page.waitForTimeout(300);
	await page.screenshot({ path: `${ARTIFACT_DIR}/02_lobby_showcase.png` });

	// =========================================================================
	// 3. Active Game Arena
	// =========================================================================
	await expect(page).toHaveURL(/\/game/, { timeout: 10000 });
	await expect(page.getByText('Goal to Prove:')).toBeVisible({ timeout: 8000 });
	await expect(page.getByRole('button', { name: 'Submit Proof' })).toBeVisible();
	await page.waitForTimeout(1000);
	await page.screenshot({ path: `${ARTIFACT_DIR}/03_game_arena_showcase.png` });

	// =========================================================================
	// 4. Unicode Palette & Compiler Diagnostics
	// =========================================================================
	// Click symbol on Unicode Palette
	const palette = page.getByRole('toolbar', { name: 'Lean 4 Unicode Symbol Palette' });
	const forallBtn = palette.locator('button').filter({ hasText: '∀' }).first();
	if (await forallBtn.isVisible()) {
		await forallBtn.click();
	}

	// Trigger Check Only
	await page.getByRole('button', { name: 'Check Only' }).click();
	await page.waitForTimeout(800);
	await page.screenshot({ path: `${ARTIFACT_DIR}/04_editor_diagnostics_showcase.png` });

	// =========================================================================
	// 5. Submit Proof & Victory Modal
	// =========================================================================
	// First submit rejects
	await page.getByRole('button', { name: 'Submit Proof' }).click();
	await page.waitForTimeout(600);

	// Second submit accepts and finishes match
	await page.getByRole('button', { name: 'Submit Proof' }).click();
	await page.waitForTimeout(1200);
	await page.screenshot({ path: `${ARTIFACT_DIR}/05_victory_modal_showcase.png` });

	// =========================================================================
	// 6. Post-Match Result Summary Page
	// =========================================================================
	await page.goto('/result?mock=1');
	await page.waitForTimeout(600);
	await page.screenshot({ path: `${ARTIFACT_DIR}/06_result_page_showcase.png` });

	// =========================================================================
	// 7. Match History Dashboard
	// =========================================================================
	await page.goto('/history?mock=1');
	await page.waitForTimeout(800);
	await page.screenshot({ path: `${ARTIFACT_DIR}/07_history_dashboard_showcase.png` });

	// =========================================================================
	// 8. Interactive Proof Replay Viewer
	// =========================================================================
	await page.goto('/replay/match-uuid-101?mock=1');
	await page.waitForTimeout(800);
	await page.screenshot({ path: `${ARTIFACT_DIR}/08_proof_replay_showcase.png` });

	// =========================================================================
	// 9. Private Room Controls in Lobby
	// =========================================================================
	await page.goto('/lobby?mock=1');
	await page.waitForTimeout(500);
	await page.screenshot({ path: `${ARTIFACT_DIR}/09_private_room_showcase.png` });
});
