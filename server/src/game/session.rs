use super::{PlayerId, RoomId};
use crate::message::ClientMessage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Session {
    pub player_id: PlayerId,
    pub username: Option<String>,
    pub token: String,
    pub created_at: Instant,
    pub expires_at: Instant,
}

#[derive(Clone)]
pub struct PlayerSession {
    pub player_id: PlayerId,
    pub tx: tokio::sync::mpsc::Sender<crate::message::ServerMessage>,
    pub username: Option<String>,
    pub joined_at: Instant,
    pub submission_count: u32,
}

impl PlayerSession {
    pub fn new(
        player_id: PlayerId,
        tx: tokio::sync::mpsc::Sender<crate::message::ServerMessage>,
        username: Option<String>,
    ) -> Self {
        Self {
            player_id,
            tx,
            username,
            joined_at: Instant::now(),
            submission_count: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    Expired,
    NotFound,
    NotConfigured,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::Expired => write!(f, "session expired"),
            SessionError::NotFound => write!(f, "session not found"),
            SessionError::NotConfigured => write!(f, "session store backend not configured"),
        }
    }
}

impl std::error::Error for SessionError {}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(
        &self,
        player_id: PlayerId,
        username: Option<String>,
    ) -> Result<Session, SessionError>;
    async fn get(&self, token: &str) -> Result<Option<Session>, SessionError>; // honours TTL
    async fn touch(&self, token: &str) -> Result<(), SessionError>;
    async fn revoke(&self, token: &str) -> Result<(), SessionError>;
}

/// In-memory session store using Arc<RwLock<HashMap>> with TTL checks on read
/// and a background sweeper task running every 10 seconds.
pub struct InMemorySessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    ttl: Duration,
}

impl InMemorySessionStore {
    pub fn new(ttl: Duration) -> Self {
        let store = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        };

        // Background sweeper task every 10s
        let sessions_clone = store.sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let now = Instant::now();
                let mut map = sessions_clone.write().await;
                map.retain(|_, s| s.expires_at > now);
            }
        });

        store
    }

    pub async fn get_by_player_id(&self, player_id: PlayerId) -> Option<Session> {
        let mut map = self.sessions.write().await;
        let now = Instant::now();
        let mut found = None;
        map.retain(|_, s| {
            if s.expires_at <= now {
                false
            } else {
                if s.player_id == player_id {
                    found = Some(s.clone());
                }
                true
            }
        });
        found
    }

    pub async fn revoke_player(&self, player_id: PlayerId) {
        let mut map = self.sessions.write().await;
        map.retain(|_, s| s.player_id != player_id);
    }
}

impl Default for InMemorySessionStore {
    fn default() -> Self {
        Self::new(Duration::from_secs(60))
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn create(
        &self,
        player_id: PlayerId,
        username: Option<String>,
    ) -> Result<Session, SessionError> {
        let token = uuid::Uuid::new_v4().to_string();
        let now = Instant::now();
        let session = Session {
            player_id,
            username,
            token: token.clone(),
            created_at: now,
            expires_at: now + self.ttl,
        };

        let mut map = self.sessions.write().await;
        map.insert(token, session.clone());
        Ok(session)
    }

    async fn get(&self, token: &str) -> Result<Option<Session>, SessionError> {
        let mut map = self.sessions.write().await;
        if let Some(session) = map.get(token) {
            if session.expires_at <= Instant::now() {
                map.remove(token);
                Ok(None)
            } else {
                Ok(Some(session.clone()))
            }
        } else {
            Ok(None)
        }
    }

    async fn touch(&self, token: &str) -> Result<(), SessionError> {
        let mut map = self.sessions.write().await;
        if let Some(session) = map.get_mut(token) {
            if session.expires_at <= Instant::now() {
                map.remove(token);
                Err(SessionError::Expired)
            } else {
                session.expires_at = Instant::now() + self.ttl;
                Ok(())
            }
        } else {
            Err(SessionError::NotFound)
        }
    }

    async fn revoke(&self, token: &str) -> Result<(), SessionError> {
        let mut map = self.sessions.write().await;
        map.remove(token);
        Ok(())
    }
}

/// Stub for Redis-backed session store.
/// Per ADR-006: the store is a trait with an in-memory implementation first;
/// RedisSessionStore returns Err(NotConfigured) without adding external dependencies.
pub struct RedisSessionStore;

#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn create(
        &self,
        _player_id: PlayerId,
        _username: Option<String>,
    ) -> Result<Session, SessionError> {
        Err(SessionError::NotConfigured)
    }

    async fn get(&self, _token: &str) -> Result<Option<Session>, SessionError> {
        Err(SessionError::NotConfigured)
    }

    async fn touch(&self, _token: &str) -> Result<(), SessionError> {
        Err(SessionError::NotConfigured)
    }

    async fn revoke(&self, _token: &str) -> Result<(), SessionError> {
        Err(SessionError::NotConfigured)
    }
}

// =============================================================================
// Connection State Machine (ConnState)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    AwaitingHello,
    Connected,
    InQueue,
    InGame { room_id: RoomId },
    AwaitingPrivateOpponent,
    Spectating { room_id: RoomId },
    Closed,
}

impl ConnState {
    /// Validates whether a client message is permitted in the current connection state.
    /// Returns Ok(()) if permitted, or Err(code) if forbidden.
    pub fn validate_transition(&self, msg: &ClientMessage) -> Result<(), &'static str> {
        match (self, msg) {
            // AwaitingHello accepts ONLY Hello
            (ConnState::AwaitingHello, ClientMessage::Hello { .. }) => Ok(()),
            (ConnState::AwaitingHello, _) => Err("invalid_state: expected Hello frame"),

            // Connected (Lobby) accepts QueueJoin, PracticeJoin, CreatePrivateRoom, JoinPrivateRoom, SpectateRoom, Pong, Hello (re-auth)
            (ConnState::Connected, ClientMessage::QueueJoin {}) => Ok(()),
            (ConnState::Connected, ClientMessage::PracticeJoin {}) => Ok(()),
            (ConnState::Connected, ClientMessage::CreatePrivateRoom { .. }) => Ok(()),
            (ConnState::Connected, ClientMessage::JoinPrivateRoom { .. }) => Ok(()),
            (ConnState::Connected, ClientMessage::SpectateRoom { .. }) => Ok(()),
            (ConnState::Connected, ClientMessage::Pong { .. }) => Ok(()),
            (ConnState::Connected, ClientMessage::Hello { .. }) => Ok(()),
            (ConnState::Connected, ClientMessage::ProofRequest { .. }) => {
                Err("invalid_state: cannot submit proof while not in game")
            }
            (ConnState::Connected, ClientMessage::Resign {}) => {
                Err("invalid_state: cannot resign while not in game")
            }
            (ConnState::Connected, ClientMessage::QueueLeave {}) => {
                Err("invalid_state: not currently in queue")
            }

            // InQueue accepts QueueLeave, Pong
            (ConnState::InQueue, ClientMessage::QueueLeave {}) => Ok(()),
            (ConnState::InQueue, ClientMessage::Pong { .. }) => Ok(()),
            (ConnState::InQueue, ClientMessage::QueueJoin {}) => {
                Err("invalid_state: already in queue")
            }
            (ConnState::InQueue, ClientMessage::PracticeJoin {}) => {
                Err("invalid_state: already in queue")
            }
            (ConnState::InQueue, ClientMessage::CreatePrivateRoom { .. }) => {
                Err("invalid_state: already in queue")
            }
            (ConnState::InQueue, ClientMessage::JoinPrivateRoom { .. }) => {
                Err("invalid_state: already in queue")
            }
            (ConnState::InQueue, ClientMessage::SpectateRoom { .. }) => {
                Err("invalid_state: already in queue")
            }
            (ConnState::InQueue, ClientMessage::ProofRequest { .. }) => {
                Err("invalid_state: cannot submit proof while in queue")
            }
            (ConnState::InQueue, ClientMessage::Resign {}) => {
                Err("invalid_state: cannot resign while in queue")
            }
            (ConnState::InQueue, ClientMessage::Hello { .. }) => {
                Err("invalid_state: cannot re-authenticate while in queue")
            }

            // AwaitingPrivateOpponent accepts QueueLeave, Pong
            (ConnState::AwaitingPrivateOpponent, ClientMessage::QueueLeave {}) => Ok(()),
            (ConnState::AwaitingPrivateOpponent, ClientMessage::Pong { .. }) => Ok(()),
            (ConnState::AwaitingPrivateOpponent, _) => {
                Err("invalid_state: waiting for private room opponent")
            }

            // Spectating accepts QueueLeave, Pong
            (ConnState::Spectating { .. }, ClientMessage::QueueLeave {}) => Ok(()),
            (ConnState::Spectating { .. }, ClientMessage::Pong { .. }) => Ok(()),
            (ConnState::Spectating { .. }, _) => Err("invalid_state: currently spectating match"),

            // InGame accepts ProofRequest, Resign, Pong
            (ConnState::InGame { .. }, ClientMessage::ProofRequest { .. }) => Ok(()),
            (ConnState::InGame { .. }, ClientMessage::Resign {}) => Ok(()),
            (ConnState::InGame { .. }, ClientMessage::Pong { .. }) => Ok(()),
            (ConnState::InGame { .. }, ClientMessage::QueueJoin {}) => {
                Err("invalid_state: cannot join queue while in match")
            }
            (ConnState::InGame { .. }, ClientMessage::PracticeJoin {}) => {
                Err("invalid_state: cannot join practice while in match")
            }
            (ConnState::InGame { .. }, ClientMessage::CreatePrivateRoom { .. }) => {
                Err("invalid_state: cannot create room while in match")
            }
            (ConnState::InGame { .. }, ClientMessage::JoinPrivateRoom { .. }) => {
                Err("invalid_state: cannot join room while in match")
            }
            (ConnState::InGame { .. }, ClientMessage::SpectateRoom { .. }) => {
                Err("invalid_state: cannot spectate while in match")
            }
            (ConnState::InGame { .. }, ClientMessage::QueueLeave {}) => {
                Err("invalid_state: cannot leave queue while in match")
            }
            (ConnState::InGame { .. }, ClientMessage::Hello { .. }) => Ok(()), // Re-attaching

            // Closed state rejects all frames
            (ConnState::Closed, _) => Err("invalid_state: connection is closed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_lifecycle_create_get_touch_revoke() {
        let store = InMemorySessionStore::new(Duration::from_millis(50));
        let player_id = PlayerId::new();

        // 1. Create
        let session = store
            .create(player_id, Some("alice".to_string()))
            .await
            .unwrap();
        assert_eq!(session.player_id, player_id);
        assert_eq!(session.username.as_deref(), Some("alice"));

        // 2. Get active
        let retrieved = store.get(&session.token).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().player_id, player_id);

        // 3. Touch
        store.touch(&session.token).await.unwrap();

        // 4. Revoke
        store.revoke(&session.token).await.unwrap();
        let after_revoke = store.get(&session.token).await.unwrap();
        assert!(after_revoke.is_none());
    }

    #[tokio::test]
    async fn test_session_expiry_honours_ttl() {
        let store = InMemorySessionStore::new(Duration::from_millis(20));
        let player_id = PlayerId::new();

        let session = store.create(player_id, None).await.unwrap();
        assert!(store.get(&session.token).await.unwrap().is_some());

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(30)).await;
        let expired = store.get(&session.token).await.unwrap();
        assert!(expired.is_none(), "Expired session must return None");
    }

    #[test]
    fn test_conn_state_transitions() {
        let awaiting = ConnState::AwaitingHello;
        // Only Hello permitted in AwaitingHello
        assert!(
            awaiting
                .validate_transition(&ClientMessage::Hello {
                    version: 1,
                    token: None,
                    username: None
                })
                .is_ok()
        );
        assert!(
            awaiting
                .validate_transition(&ClientMessage::QueueJoin {})
                .is_err()
        );
        assert!(
            awaiting
                .validate_transition(&ClientMessage::Resign {})
                .is_err()
        );
        assert!(
            awaiting
                .validate_transition(&ClientMessage::ProofRequest {
                    req_id: "1".into(),
                    code: "x".into(),
                    intent: crate::ws::message::ProofIntent::Submit
                })
                .is_err()
        );

        let connected = ConnState::Connected;
        assert!(
            connected
                .validate_transition(&ClientMessage::QueueJoin {})
                .is_ok()
        );
        assert!(
            connected
                .validate_transition(&ClientMessage::ProofRequest {
                    req_id: "1".into(),
                    code: "x".into(),
                    intent: crate::ws::message::ProofIntent::Submit
                })
                .is_err()
        );

        let in_game = ConnState::InGame {
            room_id: RoomId::new(),
        };
        assert!(
            in_game
                .validate_transition(&ClientMessage::ProofRequest {
                    req_id: "1".into(),
                    code: "x".into(),
                    intent: crate::ws::message::ProofIntent::Submit
                })
                .is_ok()
        );
        assert!(
            in_game
                .validate_transition(&ClientMessage::QueueJoin {})
                .is_err()
        );
    }
}
