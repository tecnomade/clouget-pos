use crate::db::{Database, SesionState};

/// Estado compartido del servidor HTTP
pub struct ServerState {
    pub db: Database,
    pub sesion: SesionState,
    pub token: String,
}
