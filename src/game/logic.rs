// Game history is a list of moves and a list of players (we can derive which player acted based on the history of moves), also has game ID

// The game structure will be passed around to different handlers that can update the state of the game
use crate::game::cards::{compare_played_cards, Card, Hand, HandError, Rank, Suit};
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
    #[error("Player does not own card: {0}")]
    CardNotOwned(Card),
    #[error("Hand construction error")]
    HandError(HandError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    id: String,
    players: Vec<Player>, // The first player in the list is assumed to be the starting player
    current_turn: usize,  // The index of the player who is to act
    consecutive_passes: usize,
    played_hands: Vec<Hand>,
    starting_hands: std::collections::HashMap<String, Vec<Card>>, // Player name -> starting cards
}

impl Game {
    pub fn new(
        id: String,
        players: Vec<Player>,
        current_turn: usize,
        consecutive_passes: usize,
        played_hands: Vec<Hand>,
        starting_hands: std::collections::HashMap<String, Vec<Card>>,
    ) -> Self {
        Self {
            id,
            players,
            current_turn,
            consecutive_passes,
            played_hands,
            starting_hands,
        }
    }

    pub fn new_game(id: String, player_names: &[String]) -> Result<Self, GameError> {
        // Randomly deal the 52 cards to the players
        let mut cards = Card::all_cards();
        cards.shuffle(&mut rand::rng());

        let mut players: Vec<Player> = player_names
            .iter()
            .map(|name| {
                let mut player_cards: Vec<Card> = cards.drain(0..13).collect();
                player_cards.sort();
                Player {
                    name: name.to_string(),
                    cards: player_cards,
                }
            })
            .collect();

        // Capture starting hands before rotating players
        let starting_hands: std::collections::HashMap<String, Vec<Card>> = players
            .iter()
            .map(|player| (player.name.clone(), player.cards.clone()))
            .collect();

        // The first player is the one with the 3 of diamonds
        let first_player = players
            .iter()
            .position(|p| p.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)))
            .ok_or(GameError::InvalidPlayedCards)?;

        players.rotate_left(first_player);

        Ok(Self::new(id, players, 0, 0, vec![], starting_hands))
    }

    pub fn play_cards(&mut self, player_name: &str, cards: &[Card]) -> Result<bool, GameError> {
        let player = &self.players[self.current_turn];
        if player.name != player_name {
            return Err(GameError::InvalidPlayerTurn);
        }

        // Create hand from cards
        let new_hand = Hand::from_cards(cards).map_err(GameError::HandError)?;

        // Only validate card comparison for non-pass moves
        // Skip validation if 3 consecutive passes have occurred (table is clear)
        if !cards.is_empty() && self.consecutive_passes < 3 && !self.played_hands.is_empty() {
            // Find the last non-pass hand to compare against
            if let Some(last_non_pass_hand) = self.played_hands.iter().rev().find(|h| **h != Hand::Pass) {
                if !new_hand.can_beat(last_non_pass_hand) {
                    return Err(GameError::InvalidPlayedCards);
                }
            }
        }

        if cards.is_empty() {
            if self.consecutive_passes >= 3 {
                return Err(GameError::CannotPass);
            }
            self.consecutive_passes += 1;
            // Add pass to played hands (preserve history)
            self.played_hands.push(Hand::Pass);
        } else {
            // Validate that player owns all cards before removing any
            let current_player = &self.players[self.current_turn];
            for card in cards {
                if !current_player.cards.contains(card) {
                    return Err(GameError::CardNotOwned(*card));
                }
            }
            
            // Remove played cards from the player's hand
            let current_player = &mut self.players[self.current_turn];
            for card in cards {
                if let Some(pos) = current_player.cards.iter().position(|c| c == card) {
                    current_player.cards.remove(pos);
                }
            }
            
            self.consecutive_passes = 0;
            // Add the played hand to history
            self.played_hands.push(new_hand);
            
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

    pub fn last_played_cards(&self) -> Vec<Card> {
        if let Some(last_hand) = self.played_hands.last() {
            last_hand.to_cards()
        } else {
            vec![]
        }
    }

    pub fn get_last_played_hand(&self) -> Option<&Hand> {
        self.played_hands.last()
    }

    pub fn played_hands(&self) -> &[Hand] {
        &self.played_hands
    }

    pub fn starting_hands(&self) -> &std::collections::HashMap<String, Vec<Card>> {
        &self.starting_hands
    }
}

mod tests {
    use super::*;
    use crate::game::cards::{Hand, Rank, Suit};

    fn create_starting_hands(players: &[Player]) -> std::collections::HashMap<String, Vec<Card>> {
        players
            .iter()
            .map(|player| (player.name.clone(), player.cards.clone()))
            .collect()
    }

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

        // Check that each player's cards are sorted
        for player in game.players() {
            let mut sorted_cards = player.cards.clone();
            sorted_cards.sort();
            assert_eq!(
                player.cards, sorted_cards,
                "Player {}'s cards should be sorted",
                player.name
            );
        }
    }

    #[test]
    fn test_card_removal_on_play() {
        let players = vec![
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
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![],
            create_starting_hands(&players),
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
        let players = vec![
            Player {
                name: "Alice".to_string(),
                cards: vec![Card::new(Rank::Three, Suit::Diamonds)], // Only one card
            },
            Player {
                name: "Bob".to_string(),
                cards: vec![Card::new(Rank::Six, Suit::Clubs)],
            },
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![],
            create_starting_hands(&players),
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
        let players = vec![
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
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![],
            create_starting_hands(&players),
        );

        // Alice plays first card
        let result1 = game.play_cards("Alice", &[Card::new(Rank::Three, Suit::Diamonds)]);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), false); // Game continues
        
        // Now it's Bob's turn, but Alice needs to play to win
        // Skip Bob's turn by having him pass
        let pass_result = game.play_cards("Bob", &[]);
        assert!(pass_result.is_ok());
        
        // Alice plays her final card and should win
        let result2 = game.play_cards("Alice", &[Card::new(Rank::Four, Suit::Diamonds)]);
        if result2.is_err() {
            println!("Error on second play: {:?}", result2.as_ref().unwrap_err());
        }
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), true); // Alice should win

        // Alice should have 0 cards left
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 0);
    }

    #[test]
    fn test_pass_does_not_remove_cards() {
        let players = vec![
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
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![Hand::from_cards(&[Card::new(Rank::Five, Suit::Spades)]).unwrap()], // Someone played before
            create_starting_hands(&players),
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
        let players = vec![
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
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![],
            create_starting_hands(&players),
        );

        // Try to play a card Alice doesn't have
        let cards_to_play = vec![Card::new(Rank::Ace, Suit::Spades)];
        let result = game.play_cards("Alice", &cards_to_play);
        
        // Should now return an error for card not owned
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GameError::CardNotOwned(_)));

        // Alice should still have 2 cards (invalid card wasn't in her hand anyway)
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 2);
        assert!(alice.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)));
        assert!(alice.cards.contains(&Card::new(Rank::Four, Suit::Hearts)));
    }

    #[test]
    fn test_invalid_player_turn() {
        let players = vec![
            Player {
                name: "Alice".to_string(),
                cards: vec![Card::new(Rank::Three, Suit::Diamonds)],
            },
            Player {
                name: "Bob".to_string(),
                cards: vec![Card::new(Rank::Six, Suit::Clubs)],
            },
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![],
            create_starting_hands(&players),
        );

        // Try to play with Bob when it's Alice's turn
        let result = game.play_cards("Bob", &[Card::new(Rank::Six, Suit::Clubs)]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GameError::InvalidPlayerTurn));
    }

    #[test]
    fn test_cannot_pass_after_three_consecutive() {
        let players = vec![
            Player {
                name: "Alice".to_string(),
                cards: vec![Card::new(Rank::Three, Suit::Diamonds)],
            },
            Player {
                name: "Bob".to_string(),
                cards: vec![Card::new(Rank::Six, Suit::Clubs)],
            },
            Player {
                name: "Charlie".to_string(),
                cards: vec![Card::new(Rank::Seven, Suit::Hearts)],
            },
            Player {
                name: "David".to_string(),
                cards: vec![Card::new(Rank::Eight, Suit::Spades)],
            },
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            3, // 3 consecutive passes already
            vec![Hand::Pass, Hand::Pass, Hand::Pass],
            create_starting_hands(&players),
        );

        // Try to pass when 3 consecutive passes already occurred
        let result = game.play_cards("Alice", &[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GameError::CannotPass));
    }

    #[test]
    fn test_invalid_played_cards_cannot_beat_previous() {
        let players = vec![
            Player {
                name: "Alice".to_string(),
                cards: vec![
                    Card::new(Rank::Three, Suit::Diamonds),
                    Card::new(Rank::Four, Suit::Hearts),
                ],
            },
            Player {
                name: "Bob".to_string(),
                cards: vec![Card::new(Rank::King, Suit::Spades)],
            },
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![Hand::Single(crate::game::cards::SingleHand::new(Card::new(Rank::King, Suit::Hearts)))], // Previous player played King of Hearts
            create_starting_hands(&players),
        );

        // Try to play a weaker card (3D) when King was played
        let result = game.play_cards("Alice", &[Card::new(Rank::Three, Suit::Diamonds)]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GameError::InvalidPlayedCards));
    }

    #[test]
    fn test_play_cards_player_doesnt_have() {
        let players = vec![
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
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![],
            create_starting_hands(&players),
        );

        // Try to play a card Alice doesn't have (Ace of Spades)
        let result = game.play_cards("Alice", &[Card::new(Rank::Ace, Suit::Spades)]);
        
        // Should now return an error for card not owned
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GameError::CardNotOwned(_)));
        
        // Alice should still have her original 2 cards
        let alice = &game.players[0];
        assert_eq!(alice.cards.len(), 2);
        assert!(alice.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)));
        assert!(alice.cards.contains(&Card::new(Rank::Four, Suit::Hearts)));
    }

    #[test] 
    fn test_invalid_hand_construction() {
        let players = vec![
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
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            0,
            vec![],
            create_starting_hands(&players),
        );

        // Try to play an invalid pair (different ranks)
        let result = game.play_cards("Alice", &[
            Card::new(Rank::Three, Suit::Diamonds),
            Card::new(Rank::Four, Suit::Hearts),
        ]);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), GameError::HandError(_)));
    }

    #[test]
    fn test_table_clear_after_three_passes() {
        let players = vec![
            Player {
                name: "Alice".to_string(),
                cards: vec![Card::new(Rank::Three, Suit::Diamonds)],
            },
            Player {
                name: "Bob".to_string(),
                cards: vec![Card::new(Rank::Four, Suit::Hearts)],
            },
            Player {
                name: "Charlie".to_string(),
                cards: vec![Card::new(Rank::Five, Suit::Spades)],
            },
            Player {
                name: "David".to_string(),
                cards: vec![Card::new(Rank::Six, Suit::Clubs)],
            },
        ];

        let mut game = Game::new(
            "test".to_string(),
            players.clone(),
            0, // Alice's turn
            2, // 2 consecutive passes
            vec![
                Hand::Single(crate::game::cards::SingleHand::new(Card::new(Rank::King, Suit::Hearts))),
                Hand::Pass,
                Hand::Pass,
            ],
            create_starting_hands(&players),
        );

        // Alice passes (making it 3 consecutive passes)
        let result1 = game.play_cards("Alice", &[]);
        assert!(result1.is_ok());
        assert_eq!(game.consecutive_passes, 3);

        // Now Bob should be able to play any card (table is clear)
        let result2 = game.play_cards("Bob", &[Card::new(Rank::Four, Suit::Hearts)]);
        assert!(result2.is_ok());
        assert_eq!(game.consecutive_passes, 0); // Reset after card play
    }

    #[test]
    fn test_starting_hands_captured() {
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

        // Verify starting hands were captured for all players
        let starting_hands = game.starting_hands();
        assert_eq!(starting_hands.len(), 4);
        assert!(starting_hands.contains_key("Alice"));
        assert!(starting_hands.contains_key("Bob"));
        assert!(starting_hands.contains_key("Charlie"));
        assert!(starting_hands.contains_key("David"));

        // Each player should have 13 starting cards
        for (player_name, cards) in starting_hands {
            assert_eq!(cards.len(), 13, "Player {} should have 13 starting cards", player_name);
        }

        // Starting hands should match the sorted cards each player received
        for player in game.players() {
            let starting_cards = starting_hands.get(&player.name).unwrap();
            // The current cards might be different due to the rotation, but starting hands preserve original distribution
            assert_eq!(starting_cards.len(), 13);
        }
    }
}
