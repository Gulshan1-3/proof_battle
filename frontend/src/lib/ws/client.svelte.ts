import type { ClientMessage, ServerMessage, ProofIntent } from '$lib/ws/generated';
import { type ConnState, type ConnEvent, getNextState } from './state-machine';
import { game, type VerdictResult } from '$lib/stores/game.svelte';
import { SvelteMap } from 'svelte/reactivity';

interface PendingRequest {
	reqId: string;
	intent: ProofIntent;
	resolve: (verdict: VerdictResult) => void;
	reject: (err: Error) => void;
	timeoutId: ReturnType<typeof setTimeout>;
}

export class ProofBattleClient {
	state = $state<ConnState>('disconnected');
	private ws: WebSocket | null = null;
	private url: string = '';
	private pendingRequests = new SvelteMap<string, PendingRequest>();
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private pingTimer: ReturnType<typeof setInterval> | null = null;
	private isDev = import.meta.env.DEV;

	constructor() {
		// Session token retrieval from localStorage if present
		if (typeof localStorage !== 'undefined') {
			const savedToken = localStorage.getItem('proofbattle.session_token');
			if (savedToken) {
				game.sessionToken = savedToken;
			}
		}
	}

	transition(event: ConnEvent) {
		const next = getNextState(this.state, event, this.isDev);
		this.state = next;
	}

	connect(url?: string) {
		if (url) this.url = url;
		if (!this.url) {
			if (typeof window !== 'undefined') {
				const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
				this.url = `${protocol}//${window.location.host}/ws`;
			} else {
				this.url = 'ws://127.0.0.1:3000/ws';
			}
		}

		if (
			this.state === 'disconnected' ||
			this.state === 'reconnecting' ||
			this.state === 'error' ||
			this.state === 'game_ended'
		) {
			this.transition('CONNECT');
			this.openSocket();
		}
	}

	private openSocket() {
		try {
			this.ws = new WebSocket(this.url);
		} catch {
			this.transition('ERROR');
			return;
		}

		this.ws.onopen = () => {
			// Send Hello immediately upon socket open
			const hello: ClientMessage = {
				type: 'Hello',
				version: 1,
				token: game.sessionToken,
				username: game.username || 'Anonymous'
			};
			this.send(hello);
		};

		this.ws.onmessage = (event) => {
			try {
				const msg = JSON.parse(event.data) as ServerMessage;
				this.handleMessage(msg);
			} catch (err) {
				console.error('Failed to parse incoming WebSocket message:', err);
			}
		};

		this.ws.onclose = () => {
			this.cleanupHeartbeat();
			if (
				this.state === 'in_game' ||
				this.state === 'submitting' ||
				this.state === 'game_starting'
			) {
				this.transition('DISCONNECT');
				this.scheduleReconnect();
			} else if (this.state !== 'disconnected') {
				this.transition('DISCONNECT');
			}
		};

		this.ws.onerror = (e) => {
			console.error('WebSocket encountered an error:', e);
			if (this.state !== 'error') {
				this.transition('ERROR');
			}
		};
	}

	private handleMessage(msg: ServerMessage) {
		switch (msg.type) {
			case 'Welcome': {
				this.transition('MSG_WELCOME');
				game.playerId = msg.player_id;
				game.sessionToken = msg.session_token;
				if (typeof localStorage !== 'undefined') {
					localStorage.setItem('proofbattle.session_token', msg.session_token);
				}

				// Clock offset synchronization
				this.syncClock(Number(msg.server_time_ms));
				this.startHeartbeat(Number(msg.heartbeat_interval_ms));
				game.reconnectCount = 0;
				break;
			}

			case 'ServerTime': {
				this.syncClock(Number(msg.server_time_ms));
				break;
			}

			case 'QueueStatus': {
				game.queueSize = msg.queue_size;
				game.elo = msg.elo;
				game.searchRange = msg.search_range;
				break;
			}

			case 'MatchFound': {
				this.transition('MSG_MATCH_FOUND');
				game.roomId = msg.room_id;
				game.you = msg.you;
				game.opponent = msg.opponent;
				break;
			}

			case 'RoundStart': {
				this.transition('MSG_ROUND_START');
				game.roomId = msg.room_id;
				game.problem = msg.problem;
				game.startsAtMs = Number(msg.starts_at_ms);
				game.endsAtMs = Number(msg.ends_at_ms);
				game.durationMs = Number(msg.duration_ms);
				this.syncClock(Number(msg.server_time_ms));
				break;
			}

			case 'Verdict': {
				const pending = this.pendingRequests.get(msg.req_id);
				const verdictResult: VerdictResult = {
					reqId: msg.req_id,
					verdict: msg.verdict,
					reason: msg.reason,
					message: msg.message,
					diagnostics: msg.diagnostics,
					elapsedMs: msg.elapsed_ms,
					checkSkipped: msg.check_skipped
				};

				if (pending) {
					clearTimeout(pending.timeoutId);
					this.pendingRequests.delete(msg.req_id);
					pending.resolve(verdictResult);
				}

				if (pending?.intent === 'Submit') {
					game.isSubmitting = false;
					game.lastSubmitVerdict = verdictResult;
					game.submitDiagnostics = msg.diagnostics;
					game.lastVerdict = verdictResult;
					game.diagnostics = msg.diagnostics;
					if (msg.verdict === 'Rejected' || msg.verdict === 'Error') {
						this.transition('MSG_VERDICT_REJECTED');
					}
				} else if (pending?.intent === 'Check') {
					game.isChecking = false;
					game.checkSkipped = msg.check_skipped;
					// Per S6-P2 / S8-P2: silent on check_skipped
					// A late-arriving check verdict must NEVER overwrite submit diagnostics
					if (!msg.check_skipped) {
						game.lastCheckVerdict = verdictResult;
						game.checkDiagnostics = msg.diagnostics;
						if (game.submitDiagnostics.length === 0) {
							game.diagnostics = msg.diagnostics;
							game.lastVerdict = verdictResult;
						}
					}
				} else {
					// Fallback for un-tracked verdicts
					game.lastVerdict = verdictResult;
					game.diagnostics = msg.diagnostics;
					game.checkSkipped = msg.check_skipped;
				}
				break;
			}

			case 'OpponentActivity': {
				game.opponentStatus = msg.status;
				break;
			}

			case 'RoundEnd': {
				this.transition('MSG_ROUND_END');
				game.result = {
					roomId: msg.room_id,
					outcome: msg.outcome,
					winnerId: msg.winner_id,
					winningProof: msg.winning_proof,
					canonicalProof: msg.canonical_proof,
					eloDelta: msg.elo_delta,
					durationMs: msg.duration_ms,
					seq: msg.seq
				};
				if (!game.isSpectating) {
					game.elo += msg.elo_delta;
				}
				break;
			}

			case 'ServerError': {
				console.error(`[Server Error ${msg.code}]: ${msg.message}`);
				game.serverError = { code: msg.code, message: msg.message };
				if (this.state === 'matchmaking') {
					this.transition('LEAVE_QUEUE');
				}
				break;
			}

			case 'PrivateRoomCreated': {
				game.privateRoomCode = msg.room_code;
				game.isPrivateRoomHost = true;
				break;
			}

			case 'PrivateRoomWaiting': {
				game.privateRoomCode = msg.room_code;
				break;
			}

			case 'SpectatorJoined': {
				this.transition('SPECTATE_JOINED');
				game.isSpectating = true;
				game.roomId = msg.room_id;
				game.privateRoomCode = msg.room_code ?? null;
				game.problem = msg.problem;
				game.you = msg.player1;
				game.opponent = msg.player2;
				game.startsAtMs = Number(msg.starts_at_ms);
				game.endsAtMs = Number(msg.ends_at_ms);
				game.durationMs = Number(msg.duration_ms);
				game.spectatorState = {
					roomId: msg.room_id,
					roomCode: msg.room_code ?? null,
					player1: msg.player1,
					player2: msg.player2,
					p1Status: 'Idle',
					p2Status: 'Idle',
					problem: msg.problem,
					durationMs: Number(msg.duration_ms),
					elapsedMs: Number(msg.elapsed_ms),
					startsAtMs: Number(msg.starts_at_ms),
					endsAtMs: Number(msg.ends_at_ms)
				};
				break;
			}

			case 'SpectatorUpdate': {
				if (game.spectatorState) {
					if (msg.player_id === game.spectatorState.player1.player_id) {
						game.spectatorState.p1Status = msg.status;
					} else if (msg.player_id === game.spectatorState.player2.player_id) {
						game.spectatorState.p2Status = msg.status;
					}
				}
				break;
			}

			case 'Pong': {
				// Echo reply recorded
				break;
			}
		}
	}

	private syncClock(serverTimeMs: number) {
		const offset = serverTimeMs - Date.now();
		game.serverTimeOffsetMs = offset;
	}

	private startHeartbeat(intervalMs: number) {
		this.cleanupHeartbeat();
		const interval = Math.max(5000, intervalMs || 15000);
		this.pingTimer = setInterval(() => {
			if (this.ws && this.ws.readyState === WebSocket.OPEN) {
				this.send({ type: 'Pong', ts: BigInt(Date.now()) });
			}
		}, interval);
	}

	private cleanupHeartbeat() {
		if (this.pingTimer) {
			clearInterval(this.pingTimer);
			this.pingTimer = null;
		}
	}

	joinQueue() {
		if (this.state === 'connected_idle' || this.state === 'game_ended') {
			this.transition('JOIN_QUEUE');
			game.reset();
			this.send({ type: 'QueueJoin' });
		}
	}

	practiceJoin() {
		if (this.state === 'connected_idle' || this.state === 'game_ended') {
			this.transition('JOIN_QUEUE');
			game.reset();
			this.send({ type: 'PracticeJoin' });
		}
	}

	leaveQueue() {
		if (this.state === 'matchmaking') {
			this.transition('LEAVE_QUEUE');
			game.privateRoomCode = null;
			game.isPrivateRoomHost = false;
			this.send({ type: 'QueueLeave' });
		}
	}

	createPrivateRoom(category?: string, difficulty?: number, durationSecs?: number) {
		if (this.state === 'connected_idle' || this.state === 'game_ended') {
			this.transition('JOIN_QUEUE');
			game.reset();
			this.send({
				type: 'CreatePrivateRoom',
				category: category ?? null,
				difficulty: difficulty ?? null,
				duration_secs: durationSecs ? BigInt(durationSecs) : null
			});
		}
	}

	joinPrivateRoom(roomCode: string) {
		if (this.state === 'connected_idle' || this.state === 'game_ended') {
			this.transition('JOIN_QUEUE');
			game.reset();
			this.send({
				type: 'JoinPrivateRoom',
				room_code: roomCode.trim().toUpperCase()
			});
		}
	}

	spectateRoom(roomCode: string) {
		if (this.state === 'connected_idle' || this.state === 'game_ended') {
			game.reset();
			this.send({
				type: 'SpectateRoom',
				room_code: roomCode.trim().toUpperCase()
			});
		}
	}

	leaveSpectator() {
		if (this.state === 'spectating' || this.state === 'game_ended') {
			this.transition('SPECTATE_LEAVE');
			game.isSpectating = false;
			game.spectatorState = null;
			this.send({ type: 'QueueLeave' });
		}
	}

	submitProof(code: string): Promise<VerdictResult> {
		const reqId = crypto.randomUUID();
		game.isSubmitting = true;
		this.transition('SUBMIT_PROOF');

		return this.createPendingRequest(reqId, 'Submit', () => {
			this.send({
				type: 'ProofRequest',
				req_id: reqId,
				code,
				intent: 'Submit'
			});
		});
	}

	checkProof(code: string): Promise<VerdictResult> {
		const reqId = crypto.randomUUID();
		game.isChecking = true;

		return this.createPendingRequest(reqId, 'Check', () => {
			this.send({
				type: 'ProofRequest',
				req_id: reqId,
				code,
				intent: 'Check'
			});
		});
	}

	private createPendingRequest(
		reqId: string,
		intent: ProofIntent,
		action: () => void
	): Promise<VerdictResult> {
		return new Promise<VerdictResult>((resolve, reject) => {
			const timeoutId = setTimeout(() => {
				this.pendingRequests.delete(reqId);
				if (intent === 'Submit') game.isSubmitting = false;
				if (intent === 'Check') game.isChecking = false;
				reject(new Error(`Proof verification timed out for request ${reqId}`));
			}, 35000);

			this.pendingRequests.set(reqId, {
				reqId,
				intent,
				resolve,
				reject,
				timeoutId
			});

			action();
		});
	}

	resign() {
		if (this.state === 'in_game' || this.state === 'submitting') {
			this.transition('RESIGN');
			this.send({ type: 'Resign' });
		}
	}

	disconnect() {
		this.cleanupHeartbeat();
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}
		if (typeof localStorage !== 'undefined') {
			localStorage.removeItem('proofbattle.session_token');
		}
		game.sessionToken = null;
		if (this.ws) {
			this.ws.onclose = null;
			this.ws.onerror = null;
			this.ws.onmessage = null;
			this.ws.close();
			this.ws = null;
		}
		this.state = 'disconnected';
	}

	private scheduleReconnect() {
		if (game.reconnectCount >= 5) {
			console.warn('Max reconnect attempts (5) reached. Giving up.');
			this.transition('ERROR');
			return;
		}

		if (this.state !== 'reconnecting') {
			this.transition('RECONNECT_ATTEMPT');
		}
		game.reconnectCount += 1;

		// Exponential backoff: 1s, 2s, 4s, 8s, 16s with 20% random jitter, capped at 30s
		const base = Math.min(30000, 1000 * Math.pow(2, game.reconnectCount - 1));
		const jitter = base * 0.2 * Math.random();
		const delay = base + jitter;

		this.reconnectTimer = setTimeout(() => {
			this.connect();
		}, delay);
	}

	send(msg: ClientMessage) {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			const serialized = JSON.stringify(msg, (_, value) =>
				typeof value === 'bigint' ? Number(value) : value
			);
			this.ws.send(serialized);
		}
	}
}

export const wsClient = new ProofBattleClient();
