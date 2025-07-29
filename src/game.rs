// Game is a struct that represents an individual game of Big Two. It has a unique ID (in the future we want to record every single game)

// Game has a list of players, a mapping from a player ID (1, 2, 3 or 4) to player name
// This is done so that players can leave and rejoin the game, and new players can take on empty spots

// Game has a mapping from player ID to a list of cards they have

// Game maintains the current turn, which player is to act

// Game history is a list of moves and a list of players (we can derive which player acted based on the history of moves), also has game ID

// The game structure will be passed around to different handlers that can update the state of the game

use crate::cards::{compare_played_cards, Card, HandError};

#[derive(Debug, Clone)]
pub struct Player {
    name: String,
    cards: Vec<Card>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum GameError {
    #[error("Invalid player")]
    InvalidPlayerTurn,
    #[error("Invalid played cards")]
    InvalidPlayedCards,
    #[error("Cannot pass - must play cards (3 consecutive passes)")]
    CannotPass,
    #[error("Hand construction error")]
    HandError(HandError),
}

#[derive(Debug, Clone)]
pub struct Game {
    id: String,
    players: Vec<Player>, // The first player in the list is assumed to be the starting player
    current_turn: usize,  // The index of the player who is to act
    consecutive_passes: usize,
    last_played_cards: Vec<Card>,
}

impl Game {
    pub fn new(id: String, players: Vec<Player>) -> Self {
        Self {
            id,
            players,
            current_turn: 0,
            consecutive_passes: 0,
            last_played_cards: vec![],
        }
    }

    pub fn play_cards(&mut self, player_name: &str, cards: &Vec<Card>) -> Result<(), GameError> {
        let player = &self.players[self.current_turn];
        if player.name != player_name {
            return Err(GameError::InvalidPlayerTurn);
        }

        if compare_played_cards(&self.last_played_cards, cards).is_err() {
            return Err(GameError::InvalidPlayedCards);
        }

        if cards.is_empty() {
            if self.consecutive_passes >= 3 {
                return Err(GameError::CannotPass);
            }
            self.consecutive_passes += 1;
        } else {
            self.consecutive_passes = 0;
        }

        self.last_played_cards = cards.clone();

        self.current_turn = (self.current_turn + 1) % self.players.len();
        Ok(())
    }
}
