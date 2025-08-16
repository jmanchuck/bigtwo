use std::sync::Arc;

use bigtwo::{
    event::RoomSubscription,
    game::{Card, GameEventRoomSubscriber, Rank, Suit},
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

    /// Create a simple four-player game scenario
    pub fn with_simple_four_player_game(self) -> Self {
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
            (
                "charlie",
                vec![
                    Card::new(Rank::Nine, Suit::Spades),
                    Card::new(Rank::Ten, Suit::Clubs),
                    Card::new(Rank::Jack, Suit::Diamonds),
                ],
            ),
            (
                "david",
                vec![
                    Card::new(Rank::Queen, Suit::Hearts),
                    Card::new(Rank::King, Suit::Spades),
                    Card::new(Rank::Ace, Suit::Clubs),
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
            (
                "charlie",
                vec![
                    Card::new(Rank::Seven, Suit::Spades),
                    Card::new(Rank::Eight, Suit::Clubs),
                    Card::new(Rank::Nine, Suit::Diamonds),
                ],
            ),
            (
                "david",
                vec![
                    Card::new(Rank::Ten, Suit::Hearts),
                    Card::new(Rank::Jack, Suit::Spades),
                    Card::new(Rank::Queen, Suit::Clubs),
                ],
            ),
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
        let first_player_name = if self.player_cards.is_empty() {
            // No custom cards specified - use standard random dealing
            let player_names = setup.players.clone();
            setup
                .game_service
                .create_game(
                    "room-123",
                    &player_names
                        .iter()
                        .map(|p| p.0.clone())
                        .collect::<Vec<String>>(),
                )
                .await
                .unwrap();

            // Get the game to find who has 3D (they go first)
            let game = setup.game_service.get_game("room-123").await.unwrap();
            game.current_player_turn()
        } else {
            // Custom cards specified - create game with predetermined cards
            self.create_custom_game(setup).await
        };

        // Setup game event subscriber
        let game_event_subscriber = Arc::new(GameEventRoomSubscriber::new(
            setup.game_service.clone(),
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

    async fn create_custom_game(&self, setup: &TestSetup) -> String {
        // Use the new public API to create game with predetermined cards
        let game = setup
            .game_service
            .create_game_with_cards("room-123", self.player_cards.clone())
            .await
            .unwrap();

        // Return the name of the first player (who has 3D)
        game.current_player_turn()
    }
}
