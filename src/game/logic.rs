// Game history is a list of moves and a list of players (we can derive which player acted based on the history of moves), also has game ID

// The game structure will be passed around to different handlers that can update the state of the game
use crate::game::cards::{compare_played_cards, Card, HandError, Rank, Suit};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub cards: Vec<Card>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    id: String,
    players: Vec<Player>, // The first player in the list is assumed to be the starting player
    current_turn: usize,  // The index of the player who is to act
    consecutive_passes: usize,
    last_played_cards: Vec<Card>,
}

impl Game {
    pub fn new(
        id: String,
        players: Vec<Player>,
        current_turn: usize,
        consecutive_passes: usize,
        last_played_cards: Vec<Card>,
    ) -> Self {
        Self {
            id,
            players,
            current_turn,
            consecutive_passes,
            last_played_cards,
        }
    }

    pub fn new_game(id: String, player_names: &[String]) -> Result<Self, GameError> {
        // Randomly deal the 52 cards to the players
        let mut cards = Card::all_cards();
        cards.shuffle(&mut rand::rng());

        let mut players: Vec<Player> = player_names
            .iter()
            .map(|name| Player {
                name: name.to_string(),
                cards: cards.drain(0..13).collect(),
            })
            .collect();

        // The first player is the one with the 3 of diamonds
        let first_player = players
            .iter()
            .position(|p| p.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)))
            .ok_or(GameError::InvalidPlayedCards)?;

        players.rotate_left(first_player);

        Ok(Self::new(id, players, 0, 0, vec![]))
    }

    pub fn play_cards(&mut self, player_name: &str, cards: &[Card]) -> Result<bool, GameError> {
        let player = &self.players[self.current_turn];
        if player.name != player_name {
            return Err(GameError::InvalidPlayerTurn);
        }

        // Only validate card comparison for non-pass moves and when there are previous cards
        if !cards.is_empty()
            && !self.last_played_cards.is_empty()
            && !compare_played_cards(cards, &self.last_played_cards)
                .map_err(GameError::HandError)?
        {
            return Err(GameError::InvalidPlayedCards);
        }

        if cards.is_empty() {
            if self.consecutive_passes >= 3 {
                return Err(GameError::CannotPass);
            }
            self.consecutive_passes += 1;
            // When 3 consecutive passes occur, clear the last played cards
            if self.consecutive_passes == 3 {
                self.last_played_cards = vec![];
            }
        } else {
            // Remove played cards from the player's hand
            let current_player = &mut self.players[self.current_turn];
            for card in cards {
                if let Some(pos) = current_player.cards.iter().position(|c| c == card) {
                    current_player.cards.remove(pos);
                }
            }
            
            self.consecutive_passes = 0;
            self.last_played_cards = cards.to_vec();
            
            // Check if player won (has no cards left)
            if current_player.cards.is_empty() {
                return Ok(true); // Player won
            }
        }

        self.current_turn = (self.current_turn + 1) % self.players.len();
        Ok(false) // Game continues
    }

    pub fn players(&self) -> &Vec<Player> {
        &self.players
    }

    pub fn current_player_turn(&self) -> String {
        self.players[self.current_turn].name.clone()
    }

    pub fn consecutive_passes(&self) -> usize {
        self.consecutive_passes
    }

    pub fn last_played_cards(&self) -> &[Card] {
        &self.last_played_cards
    }
}

mod tests {
    use super::*;
    use crate::game::cards::{Rank, Suit};

    #[test]
    fn test_new_game() {
        let game = Game::new_game(
            "1".to_string(),
            &[
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "David".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(game.players().len(), 4);
        assert_eq!(game.consecutive_passes(), 0);

        // Check that the current player is the one who has the 3 of diamonds
        let current_player_name = game.current_player_turn();
        let current_player = game
            .players()
            .iter()
            .find(|p| p.name == current_player_name)
            .unwrap();
        let three_of_diamonds = Card::new(Rank::Three, Suit::Diamonds);
        assert!(
            current_player.cards.contains(&three_of_diamonds),
            "Current player '{}' should have the 3 of diamonds",
            current_player_name
        );

        // Check the cards are dealt and are all 52 unique cards
        let mut all_cards = Card::all_cards();
        all_cards.sort();

        // Collect all the cards in the players' hands
        let mut dealt_cards = game
            .players()
            .iter()
            .map(|p| p.cards.clone())
            .flatten()
            .collect::<Vec<Card>>();
        dealt_cards.sort();
        // Check that all the cards are unique
        assert_eq!(dealt_cards.len(), 52);
        assert_eq!(dealt_cards, all_cards);
    }

    #[test]
    fn test_card_removal_on_play() {
        let mut game = Game::new(
            "test".to_string(),
            vec![
                Player {
                    name: "Alice".to_string(),
                    cards: vec![
                        Card::new(Rank::Three, Suit::Diamonds),
                        Card::new(Rank::Four, Suit::Hearts),
                        Card::new(Rank::Five, Suit::Spades),
                    ],
                },
                Player {
                    name: "Bob".to_string(),
                    cards: vec![Card::new(Rank::Six, Suit::Clubs)],
                },
            ],
            0, // Alice's turn
            0,
            vec![],
        );

        let cards_to_play = vec![Card::new(Rank::Three, Suit::Diamonds)];
        let result = game.play_cards("Alice", &cards_to_play);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // Game should continue, Alice didn't win

        // Alice should have 2 cards left
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 2);
        assert!(!alice.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)));
        assert!(alice.cards.contains(&Card::new(Rank::Four, Suit::Hearts)));
        assert!(alice.cards.contains(&Card::new(Rank::Five, Suit::Spades)));
    }

    #[test]
    fn test_win_detection_single_card() {
        let mut game = Game::new(
            "test".to_string(),
            vec![
                Player {
                    name: "Alice".to_string(),
                    cards: vec![Card::new(Rank::Three, Suit::Diamonds)], // Only one card
                },
                Player {
                    name: "Bob".to_string(),
                    cards: vec![Card::new(Rank::Six, Suit::Clubs)],
                },
            ],
            0, // Alice's turn
            0,
            vec![],
        );

        let cards_to_play = vec![Card::new(Rank::Three, Suit::Diamonds)];
        let result = game.play_cards("Alice", &cards_to_play);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // Alice should win

        // Alice should have 0 cards left
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 0);
    }

    #[test]
    fn test_win_detection_multiple_cards() {
        let mut game = Game::new(
            "test".to_string(),
            vec![
                Player {
                    name: "Alice".to_string(),
                    cards: vec![
                        Card::new(Rank::Three, Suit::Diamonds),
                        Card::new(Rank::Four, Suit::Diamonds),
                    ], // Two cards left
                },
                Player {
                    name: "Bob".to_string(),
                    cards: vec![Card::new(Rank::Six, Suit::Clubs)],
                },
            ],
            0, // Alice's turn
            0,
            vec![],
        );

        // Alice plays both remaining cards
        let cards_to_play = vec![
            Card::new(Rank::Three, Suit::Diamonds),
            Card::new(Rank::Four, Suit::Diamonds),
        ];
        let result = game.play_cards("Alice", &cards_to_play);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // Alice should win

        // Alice should have 0 cards left
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 0);
    }

    #[test]
    fn test_pass_does_not_remove_cards() {
        let mut game = Game::new(
            "test".to_string(),
            vec![
                Player {
                    name: "Alice".to_string(),
                    cards: vec![
                        Card::new(Rank::Three, Suit::Diamonds),
                        Card::new(Rank::Four, Suit::Hearts),
                    ],
                },
                Player {
                    name: "Bob".to_string(),
                    cards: vec![Card::new(Rank::Six, Suit::Clubs)],
                },
            ],
            0, // Alice's turn
            0,
            vec![Card::new(Rank::Five, Suit::Spades)], // Someone played before
        );

        // Alice passes (empty cards vector)
        let result = game.play_cards("Alice", &[]);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // Game continues

        // Alice should still have 2 cards
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 2);
        assert!(alice.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)));
        assert!(alice.cards.contains(&Card::new(Rank::Four, Suit::Hearts)));
    }

    #[test]
    fn test_invalid_card_not_removed() {
        let mut game = Game::new(
            "test".to_string(),
            vec![
                Player {
                    name: "Alice".to_string(),
                    cards: vec![
                        Card::new(Rank::Three, Suit::Diamonds),
                        Card::new(Rank::Four, Suit::Hearts),
                    ],
                },
                Player {
                    name: "Bob".to_string(),
                    cards: vec![Card::new(Rank::Six, Suit::Clubs)],
                },
            ],
            0, // Alice's turn
            0,
            vec![],
        );

        // Try to play a card Alice doesn't have
        let cards_to_play = vec![Card::new(Rank::Ace, Suit::Spades)];
        let result = game.play_cards("Alice", &cards_to_play);
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // Game continues

        // Alice should still have 2 cards (invalid card wasn't in her hand anyway)
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 2);
        assert!(alice.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)));
        assert!(alice.cards.contains(&Card::new(Rank::Four, Suit::Hearts)));
    }
}
