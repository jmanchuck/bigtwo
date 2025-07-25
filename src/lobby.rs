use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum LobbyError {
    #[error("Room is full")]
    RoomFull,
    #[error("Player already in room")]
    PlayerAlreadyInRoom,
    #[error("Player not in room")]
    PlayerNotInRoom,
}

pub struct Lobby {
    players: Vec<String>,
    host: String,
}

impl Lobby {
    pub fn new(host: String) -> Self {
        Self {
            players: vec![host.clone()],
            host,
        }
    }

    pub fn add_player(&mut self, player: String) -> Result<(), LobbyError> {
        if self.players.len() == 4 {
            error!("Room is full: {}", self.players.len());
            return Err(LobbyError::RoomFull);
        }

        if self.players.contains(&player) {
            error!("Player already in room: {}", player);
            return Err(LobbyError::PlayerAlreadyInRoom);
        }

        self.players.push(player);
        Ok(())
    }

    pub fn remove_player(&mut self, player: &str) -> Result<String, LobbyError> {
        if !self.players.iter().any(|p| p == player) {
            error!("Player not in room: {}", player);
            return Err(LobbyError::PlayerNotInRoom);
        }

        self.players.retain(|p| p != player);

        // Set the new host to the first player in the list
        self.host = self.players[0].clone();

        Ok(self.host.clone())
    }

    pub fn host(&self) -> String {
        self.host.clone()
    }

    pub fn players(&self) -> Vec<String> {
        self.players.clone()
    }

    pub fn can_start(&self) -> bool {
        self.players.len() == 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lobby() {
        let mut lobby = Lobby::new("host".to_string());

        assert_eq!(lobby.host(), "host");
        assert_eq!(lobby.players().len(), 1);
        assert_eq!(lobby.players()[0], "host");
    }

    #[test]
    fn test_add_player() {
        let mut lobby = Lobby::new("host".to_string());

        assert!(lobby.add_player("player1".to_string()).is_ok());
        assert_eq!(lobby.players().len(), 2);
        assert_eq!(lobby.players()[1], "player1");

        assert!(lobby.add_player("player2".to_string()).is_ok());
        assert_eq!(lobby.players().len(), 3);
        assert_eq!(lobby.players()[2], "player2");

        assert!(lobby.add_player("player3".to_string()).is_ok());
    }

    #[test]
    fn test_remove_player() {
        let mut lobby = Lobby::new("host".to_string());

        assert!(lobby.add_player("player1".to_string()).is_ok());
        assert!(lobby.add_player("player2".to_string()).is_ok());
        assert!(lobby.add_player("player3".to_string()).is_ok());

        assert_eq!(lobby.remove_player("host").is_ok_and(|_| true), true);
        assert_eq!(lobby.host(), "player1");
        assert_eq!(lobby.players().len(), 3);
    }
}
