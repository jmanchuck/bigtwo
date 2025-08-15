use std::sync::Arc;

use bigtwo::{
    event::RoomSubscription,
    game::{Card, Game, GameEventRoomSubscriber, Player, Rank, Suit},
};

use super::setup::TestSetup;

// ============================================================================
// Card Creation Macro
// ============================================================================

#[macro_export]
macro_rules! cards {
    ($($rank:ident $suit:ident),* $(,)?) => {
        vec![$($crate::Card::new($crate::Rank::$rank, $crate::Suit::$suit)),*]
    };
}

// ============================================================================
// Game Setup Utilities
// ============================================================================

pub struct GameBuilder {
    player_cards: Vec<(String, Vec<Card>)>,
}

impl GameBuilder {
    pub fn new() -> Self {
        Self {
            player_cards: vec![],
        }
    }

    /// Create a simple two-player game scenario
    pub fn with_simple_two_player_game(self) -> Self {
        self.with_cards(vec![
            (
                "alice",
                vec![
                    Card::new(Rank::Three, Suit::Diamonds),
                    Card::new(Rank::Four, Suit::Hearts),
                    Card::new(Rank::Five, Suit::Spades),
                ],
            ), // Alice has 3D, goes first
            (
                "bob",
                vec![
                    Card::new(Rank::Six, Suit::Clubs),
                    Card::new(Rank::Seven, Suit::Diamonds),
                    Card::new(Rank::Eight, Suit::Hearts),
                ],
            ),
        ])
    }

    /// Create a scenario with pairs for testing pair gameplay
    pub fn with_pair_scenario(self) -> Self {
        self.with_cards(vec![
            (
                "alice",
                vec![
                    Card::new(Rank::Four, Suit::Hearts),
                    Card::new(Rank::Four, Suit::Spades),
                    Card::new(Rank::King, Suit::Clubs),
                ],
            ), // Alice has pair of 4s
            (
                "bob",
                vec![
                    Card::new(Rank::Three, Suit::Diamonds),
                    Card::new(Rank::Five, Suit::Hearts),
                    Card::new(Rank::Five, Suit::Clubs),
                ],
            ), // Bob has 3D (goes first) and pair of 5s
        ])
    }

    /// Create a custom game with specific card distributions
    pub fn with_cards(mut self, cards: Vec<(&str, Vec<Card>)>) -> Self {
        self.player_cards = cards
            .into_iter()
            .map(|(name, cards)| (name.to_string(), cards))
            .collect();
        self
    }

    /// Build the game and return the first player's name
    pub async fn build_with_setup(self, setup: &TestSetup) -> String {
        let players: Vec<Player> = self
            .player_cards
            .into_iter()
            .map(|(name, cards)| Player { name, cards })
            .collect();

        let first_player_index = players
            .iter()
            .position(|p| p.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)))
            .expect("One player must have 3D");

        let first_player_name = players[first_player_index].name.clone();

        let starting_hands = players
            .iter()
            .map(|player| (player.name.clone(), player.cards.clone()))
            .collect();
        
        let game = Game::new(
            "room-123".to_string(),
            players,
            first_player_index,
            0,
            vec![],
            starting_hands,
        );
        setup
            .game_manager
            .update_game("room-123", game)
            .await
            .unwrap();

        // Setup game event subscriber
        let game_event_subscriber = Arc::new(GameEventRoomSubscriber::new(
            setup.game_manager.clone(),
            setup.event_bus.clone(),
        ));
        let game_subscription = RoomSubscription::new(
            "room-123".to_string(),
            game_event_subscriber,
            setup.event_bus.clone(),
        );
        let _handle = game_subscription.start().await;

        first_player_name
    }
}
