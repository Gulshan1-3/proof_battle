import type {
	PlayerInfo,
	Problem,
	MatchOutcome,
	Diagnostic,
	VerdictStatus,
	RejectReason,
	OpponentStatus
} from '$lib/ws/generated';

export interface VerdictResult {
	reqId: string;
	verdict: VerdictStatus;
	reason: RejectReason | null;
	message: string;
	diagnostics: Diagnostic[];
	elapsedMs: bigint;
	checkSkipped: boolean;
}

export interface MatchResult {
	roomId: string;
	outcome: MatchOutcome;
	winnerId: string | null;
	winningProof: string | null;
	canonicalProof: string | null;
	eloDelta: number;
	durationMs: bigint;
	seq: bigint;
}

export class GameState {
	playerId = $state<string | null>(null);
	sessionToken = $state<string | null>(null);
	username = $state<string | null>(null);
	elo = $state<number>(1200);

	// Matchmaking
	queueSize = $state<number>(0);
	searchRange = $state<{ min_elo: number; max_elo: number } | null>(null);

	// Active match
	roomId = $state<string | null>(null);
	you = $state<PlayerInfo | null>(null);
	opponent = $state<PlayerInfo | null>(null);
	opponentStatus = $state<OpponentStatus>('Idle');
	problem = $state<Problem | null>(null);

	startsAtMs = $state<number>(0);
	endsAtMs = $state<number>(0);
	durationMs = $state<number>(0);
	serverTimeOffsetMs = $state<number>(0);

	// Client status
	isSubmitting = $state<boolean>(false);
	isChecking = $state<boolean>(false);
	checkSkipped = $state<boolean>(false);
	reconnectCount = $state<number>(0);

	// Results & Feedback (Separated Check vs Submit channels per S8-P2)
	diagnostics = $state<Diagnostic[]>([]);
	checkDiagnostics = $state<Diagnostic[]>([]);
	submitDiagnostics = $state<Diagnostic[]>([]);
	lastVerdict = $state<VerdictResult | null>(null);
	lastCheckVerdict = $state<VerdictResult | null>(null);
	lastSubmitVerdict = $state<VerdictResult | null>(null);
	result = $state<MatchResult | null>(null);

	// Active diagnostics: submit diagnostics have precedence over live check diagnostics.
	// Implemented as a getter (not $derived) to avoid Svelte 5 class-field rune
	// compilation issues in production SSR/CSR builds.
	get activeDiagnostics(): Diagnostic[] {
		if (this.submitDiagnostics.length > 0) {
			return this.submitDiagnostics;
		}
		return this.checkDiagnostics;
	}

	// Plain getters (not $derived) for production build compatibility.
	// Svelte 5 $state fields are reactive so reading them in getters still
	// triggers reactive updates when accessed from reactive contexts (templates, $effect).
	get timeRemainingMs(): number {
		if (this.endsAtMs <= 0) return 0;
		const currentServerTime = Date.now() + this.serverTimeOffsetMs;
		return Math.max(0, this.endsAtMs - currentServerTime);
	}

	get canSubmit(): boolean {
		return (
			this.roomId !== null &&
			this.problem !== null &&
			this.result === null &&
			!this.isSubmitting &&
			this.timeRemainingMs > 0
		);
	}

	get canCheck(): boolean {
		return (
			this.roomId !== null && this.problem !== null && this.result === null && !this.isChecking
		);
	}

	clearSubmitDiagnostics() {
		this.submitDiagnostics = [];
	}

	reset() {
		this.roomId = null;
		this.you = null;
		this.opponent = null;
		this.opponentStatus = 'Idle';
		this.problem = null;
		this.startsAtMs = 0;
		this.endsAtMs = 0;
		this.durationMs = 0;
		this.isSubmitting = false;
		this.isChecking = false;
		this.checkSkipped = false;
		this.diagnostics = [];
		this.checkDiagnostics = [];
		this.submitDiagnostics = [];
		this.lastVerdict = null;
		this.lastCheckVerdict = null;
		this.lastSubmitVerdict = null;
		this.result = null;
	}
}

export const game = new GameState();
