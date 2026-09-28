import type { ClientMessage, ServerMessage, Problem } from '$lib/ws/generated';
import { MOCK_PROBLEMS } from './fixtures';

export class MockWebSocket {
	static readonly CONNECTING = 0;
	static readonly OPEN = 1;
	static readonly CLOSING = 2;
	static readonly CLOSED = 3;

	readonly CONNECTING = 0;
	readonly OPEN = 1;
	readonly CLOSING = 2;
	readonly CLOSED = 3;

	static originalWebSocket = typeof window !== 'undefined' ? window.WebSocket : null;
	static isInstalled = false;

	url: string;
	readyState: number = MockWebSocket.CONNECTING;
	onopen: ((event: Event) => void) | null = null;
	onmessage: ((event: MessageEvent) => void) | null = null;
	onclose: ((event: CloseEvent) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;

	private submitCount = 0;
	private mode: string | null = null;
	private timers: ReturnType<typeof setTimeout>[] = [];

	constructor(url: string) {
		this.url = url;

		if (typeof window !== 'undefined') {
			const params = new URLSearchParams(window.location.search);
			this.mode = params.get('mock');
		}

		setTimeout(() => {
			if (this.mode === 'fail') {
				const errorMsg: ServerMessage = {
					type: 'ServerError',
					code: 'simulated_failure',
					message: 'Mock server failure induced by ?mock=fail',
					retry_after_ms: null,
					retryable: false
				};
				this.emitMessage(errorMsg);
				this.close();
				return;
			}

			this.readyState = MockWebSocket.OPEN;
			this.onopen?.(new Event('open'));
		}, 50);
	}

	send(data: string) {
		const msg = JSON.parse(data) as ClientMessage;

		switch (msg.type) {
			case 'Hello': {
				const welcome: ServerMessage = {
					type: 'Welcome',
					player_id: 'mock-player-1',
					session_token: 'mock-session-tok-123',
					server_time_ms: BigInt(Date.now()),
					heartbeat_interval_ms: 15000n,
					protocol_version: 1
				};
				const serverTime: ServerMessage = {
					type: 'ServerTime',
					server_time_ms: BigInt(Date.now())
				};
				this.emitMessage(welcome);
				this.emitMessage(serverTime);
				break;
			}

			case 'QueueJoin': {
				const queueStatus: ServerMessage = {
					type: 'QueueStatus',
					queue_size: 2,
					elo: 1200,
					search_range: { min_elo: 1000, max_elo: 1400 }
				};
				this.emitMessage(queueStatus);

				// Simulate match pairing after 800ms
				const matchTimer = setTimeout(() => {
					const matchFound: ServerMessage = {
						type: 'MatchFound',
						room_id: 'mock-room-alpha',
						you: {
							player_id: 'mock-player-1',
							username: 'You',
							elo: 1200
						},
						opponent: {
							player_id: 'mock-player-2',
							username: 'Grandmaster_Lean',
							elo: 1248
						}
					};
					this.emitMessage(matchFound);

					const now = Date.now();
					const problem: Problem = MOCK_PROBLEMS[0]!;
					const roundStart: ServerMessage = {
						type: 'RoundStart',
						room_id: 'mock-room-alpha',
						problem,
						starts_at_ms: BigInt(now),
						ends_at_ms: BigInt(now + 300_000),
						duration_ms: 300_000n,
						server_time_ms: BigInt(now),
						seq: 1n
					};
					this.emitMessage(roundStart);

					// If mode is disconnect, simulate dropping the socket after RoundStart
					if (this.mode === 'disconnect') {
						setTimeout(() => {
							this.close();
						}, 1000);
					}
				}, 800);

				this.timers.push(matchTimer);
				break;
			}

			case 'PracticeJoin': {
				const queueStatus: ServerMessage = {
					type: 'QueueStatus',
					queue_size: 1,
					elo: 1200,
					search_range: { min_elo: 1200, max_elo: 1200 }
				};
				this.emitMessage(queueStatus);

				// Simulate match pairing with practice bot after 150ms
				const matchTimer = setTimeout(() => {
					const matchFound: ServerMessage = {
						type: 'MatchFound',
						room_id: 'practice-room-' + Math.random().toString(36).slice(2, 8),
						you: {
							player_id: 'mock-player-1',
							username: 'You',
							elo: 1200
						},
						opponent: {
							player_id: 'bot-1',
							username: 'Lean Practice Bot',
							elo: 1200
						}
					};
					this.emitMessage(matchFound);

					const now = Date.now();
					const problem: Problem = MOCK_PROBLEMS[0]!;
					const roundStart: ServerMessage = {
						type: 'RoundStart',
						room_id: matchFound.room_id,
						problem,
						starts_at_ms: BigInt(now),
						ends_at_ms: BigInt(now + 600_000),
						duration_ms: 600_000n,
						server_time_ms: BigInt(now),
						seq: 1n
					};
					this.emitMessage(roundStart);
				}, 150);

				this.timers.push(matchTimer);
				break;
			}

			case 'QueueLeave': {
				break;
			}

			case 'ProofRequest': {
				const { req_id, intent, code } = msg;

				const lines = code.split('\n');
				const targetLine = lines.length >= 3 ? 3 : 1;
				const lineContent = lines[targetLine - 1] || code;

				if (intent === 'Check') {
					const checkTimer = setTimeout(() => {
						const verdict: ServerMessage = {
							type: 'Verdict',
							req_id,
							verdict: 'Rejected',
							reason: null,
							message: `Syntax error: unexpected token at line ${targetLine}`,
							diagnostics: [
								{
									line: targetLine,
									col: 1,
									end_line: targetLine,
									end_col: Math.max(2, lineContent.length + 1),
									severity: 'error',
									message: `unknown identifier '${lineContent.trim() || 'ident'}' in tactic script`
								}
							],
							elapsed_ms: 380n,
							check_skipped: false
						};
						this.emitMessage(verdict);
					}, 350);
					this.timers.push(checkTimer);
				} else {
					// Submit intent: reject once, then accept
					this.submitCount++;
					const isFirst = this.submitCount === 1;

					const submitTimer = setTimeout(() => {
						if (isFirst) {
							const rejectVerdict: ServerMessage = {
								type: 'Verdict',
								req_id,
								verdict: 'Rejected',
								reason: { type: 'LeaningFailed' },
								message: `Proof rejected: goal not closed (error on line ${targetLine})`,
								diagnostics: [
									{
										line: targetLine,
										col: 1,
										end_line: targetLine,
										end_col: Math.max(2, lineContent.length + 1),
										severity: 'error',
										message: `syntax error: unexpected token '${lineContent.trim() || 'error'}' at line ${targetLine}`
									}
								],
								elapsed_ms: 420n,
								check_skipped: false
							};
							this.emitMessage(rejectVerdict);
						} else {
							const acceptVerdict: ServerMessage = {
								type: 'Verdict',
								req_id,
								verdict: 'Accepted',
								reason: null,
								message: 'Theorem verified! Proof accepted.',
								diagnostics: [],
								elapsed_ms: 450n,
								check_skipped: false
							};
							this.emitMessage(acceptVerdict);

							// Dispatch RoundEnd immediately
							const roundEnd: ServerMessage = {
								type: 'RoundEnd',
								room_id: 'mock-room-alpha',
								outcome: 'Won',
								winner_id: 'mock-player-1',
								winning_proof: code,
								canonical_proof: 'intro n\nsimp',
								elo_delta: 24,
								duration_ms: 45_000n,
								seq: 2n
							};
							this.emitMessage(roundEnd);
						}
					}, 450);
					this.timers.push(submitTimer);
				}
				break;
			}

			case 'Resign': {
				const roundEnd: ServerMessage = {
					type: 'RoundEnd',
					room_id: 'mock-room-alpha',
					outcome: 'Lost',
					winner_id: 'mock-player-2',
					winning_proof: null,
					canonical_proof: 'intro n\nsimp',
					elo_delta: -20,
					duration_ms: 12_000n,
					seq: 2n
				};
				this.emitMessage(roundEnd);
				break;
			}

			case 'Pong': {
				break;
			}
		}
	}

	private emitMessage(msg: ServerMessage) {
		if (this.readyState !== MockWebSocket.OPEN) return;
		const serialized = JSON.stringify(msg, (_, v) => (typeof v === 'bigint' ? Number(v) : v));
		this.onmessage?.(new MessageEvent('message', { data: serialized }));
	}

	close() {
		this.readyState = MockWebSocket.CLOSED;
		for (const t of this.timers) clearTimeout(t);
		this.onclose?.(new CloseEvent('close', { wasClean: true, code: 1000 }));
	}

	static install() {
		if (typeof window !== 'undefined') {
			window.WebSocket = MockWebSocket as unknown as typeof WebSocket;
			globalThis.WebSocket = MockWebSocket as unknown as typeof WebSocket;
			MockWebSocket.isInstalled = true;
		}
	}

	static restore() {
		if (typeof window !== 'undefined' && MockWebSocket.originalWebSocket) {
			window.WebSocket = MockWebSocket.originalWebSocket;
			globalThis.WebSocket = MockWebSocket.originalWebSocket;
			MockWebSocket.isInstalled = false;
		}
	}
}
