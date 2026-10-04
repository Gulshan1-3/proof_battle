export type ConnState =
	| 'disconnected'
	| 'connecting'
	| 'connected_idle'
	| 'matchmaking'
	| 'game_starting'
	| 'in_game'
	| 'submitting'
	| 'spectating'
	| 'reconnecting'
	| 'reattaching'
	| 'game_ended'
	| 'error';

export type ConnEvent =
	| 'CONNECT'
	| 'OPEN'
	| 'JOIN_QUEUE'
	| 'LEAVE_QUEUE'
	| 'MSG_WELCOME'
	| 'MSG_MATCH_FOUND'
	| 'MSG_ROUND_START'
	| 'SUBMIT_PROOF'
	| 'MSG_VERDICT_REJECTED'
	| 'MSG_ROUND_END'
	| 'SPECTATE_JOINED'
	| 'SPECTATE_LEAVE'
	| 'RESIGN'
	| 'DISCONNECT'
	| 'RECONNECT_ATTEMPT'
	| 'REATTACH_SUCCESS'
	| 'ERROR'
	| 'RESET';

export const TRANSITIONS: Record<ConnState, Partial<Record<ConnEvent, ConnState>>> = {
	disconnected: {
		CONNECT: 'connecting',
		RECONNECT_ATTEMPT: 'reconnecting'
	},
	connecting: {
		OPEN: 'connected_idle',
		MSG_WELCOME: 'connected_idle',
		DISCONNECT: 'disconnected',
		ERROR: 'error'
	},
	connected_idle: {
		MSG_WELCOME: 'connected_idle',
		JOIN_QUEUE: 'matchmaking',
		SPECTATE_JOINED: 'spectating',
		DISCONNECT: 'disconnected',
		ERROR: 'error'
	},
	matchmaking: {
		LEAVE_QUEUE: 'connected_idle',
		MSG_MATCH_FOUND: 'game_starting',
		MSG_ROUND_START: 'in_game',
		DISCONNECT: 'disconnected',
		ERROR: 'error'
	},
	game_starting: {
		MSG_ROUND_START: 'in_game',
		DISCONNECT: 'reconnecting',
		ERROR: 'error'
	},
	in_game: {
		SUBMIT_PROOF: 'submitting',
		MSG_ROUND_END: 'game_ended',
		RESIGN: 'game_ended',
		DISCONNECT: 'reconnecting',
		ERROR: 'error'
	},
	submitting: {
		MSG_VERDICT_REJECTED: 'in_game',
		MSG_ROUND_END: 'game_ended',
		DISCONNECT: 'reconnecting',
		ERROR: 'error'
	},
	spectating: {
		SPECTATE_LEAVE: 'connected_idle',
		LEAVE_QUEUE: 'connected_idle',
		MSG_ROUND_END: 'game_ended',
		DISCONNECT: 'disconnected',
		ERROR: 'error'
	},
	reconnecting: {
		CONNECT: 'connecting',
		REATTACH_SUCCESS: 'in_game',
		DISCONNECT: 'disconnected',
		ERROR: 'error'
	},
	reattaching: {
		REATTACH_SUCCESS: 'in_game',
		MSG_ROUND_START: 'in_game',
		DISCONNECT: 'disconnected',
		ERROR: 'error'
	},
	game_ended: {
		JOIN_QUEUE: 'matchmaking',
		SPECTATE_LEAVE: 'connected_idle',
		LEAVE_QUEUE: 'connected_idle',
		RESET: 'connected_idle',
		CONNECT: 'connecting',
		DISCONNECT: 'disconnected'
	},
	error: {
		CONNECT: 'connecting',
		RESET: 'disconnected',
		DISCONNECT: 'disconnected'
	}
};

export class IllegalStateTransitionError extends Error {
	constructor(
		public from: ConnState,
		public event: ConnEvent
	) {
		super(`Illegal transition from state "${from}" via event "${event}"`);
		this.name = 'IllegalStateTransitionError';
	}
}

export function getNextState(
	current: ConnState,
	event: ConnEvent,
	isDev: boolean = true
): ConnState {
	const allowed = TRANSITIONS[current]?.[event];
	if (!allowed) {
		if (isDev) {
			throw new IllegalStateTransitionError(current, event);
		} else {
			console.error(`Illegal transition: ${current} -> ${event}`);
			return current;
		}
	}
	return allowed;
}
