use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use crate::{
    event::{EventBus, RoomEvent, RoomEventError, RoomEventHandler},
    game::{
        cards::Card,
        gamemanager::GameManager,
        logic::{Game, Player},
    },
};

pub struct GameEventRoomSubscriber {
    game_manager: Arc<GameManager>,
    event_bus: EventBus,
}


#[async_trait]
impl RoomEventHandler for GameEventRoomSubscriber {
    async fn handle_room_event(
        &self,
        room_id: &str,
        event: RoomEvent,
    ) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            event = ?event,
            "Handling game event for WebSocket connections"
        );

        match event {
            RoomEvent::CreateGame { players } => {
                self.handle_create_game(room_id, &players).await?;
            }
            RoomEvent::TryPlayMove { player, cards } => {
                self.handle_player_played_move(room_id, &player, &cards)
                    .await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handler_name(&self) -> &'static str {
        "GameEventRoomSubscriber"
    }
}

impl GameEventRoomSubscriber {
    pub fn new(game_manager: Arc<GameManager>, event_bus: EventBus) -> Self {
        Self {
            game_manager,
            event_bus,
        }
    }

    async fn handle_create_game(
        &self,
        room_id: &str,
        players: &[String],
    ) -> Result<(), RoomEventError> {
        info!(room_id = %room_id, "Starting Game");

        self.game_manager
            .create_game(room_id, players)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to create game: {}", e)))?;

        let game =
            self.game_manager
                .get_game(room_id)
                .await
                .ok_or(RoomEventError::HandlerError(format!(
                    "Game not found for room: {}",
                    room_id
                )))?;

        let game_message = RoomEvent::StartGame { game: game.clone() };

        self.event_bus.emit_to_room(room_id, game_message).await;

        Ok(())
    }

    async fn handle_player_played_move(
        &self,
        room_id: &str,
        player: &str,
        cards: &[Card],
    ) -> Result<(), RoomEventError> {
        info!(room_id = %room_id, player = %player, cards = ?cards, "Player played move");

        // Execute the move and check if player won (Game::play_cards handles all validation)
        let player_won = self.execute_move(room_id, player, cards).await?;

        // If player won, the GameWon event was already emitted in execute_move
        // Don't emit normal game flow events
        if player_won {
            return Ok(());
        }

        // Get updated game after move execution to get current turn
        let updated_game =
            self.game_manager
                .get_game(room_id)
                .await
                .ok_or(RoomEventError::HandlerError(format!(
                    "Game not found for room after move: {}",
                    room_id
                )))?;

        // Emit move played event
        self.event_bus
            .emit_to_room(
                room_id,
                RoomEvent::MovePlayed {
                    player: player.to_string(),
                    cards: cards.to_vec(),
                    game: updated_game.clone(),
                },
            )
            .await;

        // Emit turn changed event with the new current player
        self.event_bus
            .emit_to_room(
                room_id,
                RoomEvent::TurnChanged {
                    player: updated_game.current_player_turn(),
                },
            )
            .await;

        Ok(())
    }

    async fn execute_move(
        &self,
        room_id: &str,
        player: &str,
        cards: &[Card],
    ) -> Result<bool, RoomEventError> {
        // Get and update the game
        let mut game =
            self.game_manager
                .get_game(room_id)
                .await
                .ok_or(RoomEventError::HandlerError(format!(
                    "Game not found for room: {}",
                    room_id
                )))?;

        // Execute the move and check if player won
        let player_won = game.play_cards(player, cards)
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to play cards: {}", e)))?;

        // Update the game in the manager
        self.game_manager
            .update_game(room_id, game.clone())
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to update game: {}", e)))?;

        // If player won, emit GameWon event and don't continue with normal flow
        if player_won {
            self.event_bus
                .emit_to_room(
                    room_id,
                    RoomEvent::GameWon {
                        winner: player.to_string(),
                    },
                )
                .await;
            return Ok(true);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{
        cards::{Card, Hand, Rank, Suit},
        logic::{Game, Player},
    };

    fn create_test_players() -> Vec<Player> {
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
                cards: vec![
                    Card::new(Rank::Six, Suit::Clubs),
                    Card::new(Rank::Seven, Suit::Diamonds),
                    Card::new(Rank::Eight, Suit::Hearts),
                ],
            },
            Player {
                name: "Charlie".to_string(),
                cards: vec![
                    Card::new(Rank::Nine, Suit::Spades),
                    Card::new(Rank::Ten, Suit::Clubs),
                    Card::new(Rank::Jack, Suit::Diamonds),
                ],
            },
            Player {
                name: "David".to_string(),
                cards: vec![
                    Card::new(Rank::Queen, Suit::Hearts),
                    Card::new(Rank::King, Suit::Spades),
                    Card::new(Rank::Ace, Suit::Clubs),
                ],
            },
        ]
    }

    fn create_test_game() -> Game {
        Game::new(
            "test_room".to_string(),
            create_test_players(),
            0,      // Alice's turn
            0,      // No consecutive passes
            vec![], // No last played cards
        )
    }


    #[tokio::test]
    async fn test_game_room_subscriber_new() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();

        let subscriber = GameEventRoomSubscriber::new(game_manager, event_bus);
        assert_eq!(subscriber.handler_name(), "GameEventRoomSubscriber");
    }

    #[tokio::test]
    async fn test_handle_create_game() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager.clone(), event_bus);

        let players = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "David".to_string(),
        ];

        let result = subscriber.handle_create_game("test_room", &players).await;
        assert!(result.is_ok());

        // Verify game was created
        let game = game_manager.get_game("test_room").await;
        assert!(game.is_some());

        let game = game.unwrap();
        assert_eq!(game.players().len(), 4);
        // Note: We can't assert current_player_turn() == "Alice" because new_game()
        // rotates players based on who has 3D, which is random
        assert!(game.players().iter().any(|p| p.name == "Alice"));
    }

    #[tokio::test]
    async fn test_handle_player_played_move_success() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager.clone(), event_bus);

        // Manually insert a deterministic game
        let test_game = create_test_game();
        game_manager
            .update_game("test_room", test_game)
            .await
            .unwrap();

        // Alice has 3D, so she can play it
        let result = subscriber
            .handle_player_played_move(
                "test_room",
                "Alice",
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_player_played_move_game_not_found() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager, event_bus);

        let result = subscriber
            .handle_player_played_move(
                "nonexistent_room",
                "Alice",
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Game not found"));
    }

    #[tokio::test]
    async fn test_handle_player_played_move_wrong_turn() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager.clone(), event_bus);

        // Manually insert a deterministic game where Alice is current player
        let test_game = create_test_game();
        game_manager
            .update_game("test_room", test_game)
            .await
            .unwrap();

        // Try to play with Bob when it's Alice's turn
        let result = subscriber
            .handle_player_played_move("test_room", "Bob", &[Card::new(Rank::Six, Suit::Clubs)])
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid player"));
    }

    #[tokio::test]
    async fn test_execute_move_success() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager.clone(), event_bus);

        // Manually insert a deterministic game
        let test_game = create_test_game();
        game_manager
            .update_game("test_room", test_game)
            .await
            .unwrap();

        let result = subscriber
            .execute_move(
                "test_room",
                "Alice",
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;
        assert!(result.is_ok());
    }

    // Note: Invalid card format test removed since validation now happens
    // early in the websocket handler, not in execute_move

    #[tokio::test]
    async fn test_execute_move_game_not_found() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager, event_bus);

        let result = subscriber
            .execute_move(
                "nonexistent_room",
                "Alice",
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Game not found"));
    }

    #[tokio::test]
    async fn test_execute_move_player_wins() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager.clone(), event_bus.clone());

        // Create a test game where Alice has only one card left
        let test_game = Game::new(
            "test_room".to_string(),
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

        game_manager
            .update_game("test_room", test_game)
            .await
            .unwrap();

        // Alice plays her last card - should win
        let result = subscriber
            .execute_move(
                "test_room",
                "Alice",
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // Should return true indicating player won

        // Verify Alice has no cards left
        let updated_game = game_manager.get_game("test_room").await.unwrap();
        let alice = updated_game.players().iter().find(|p| p.name == "Alice").unwrap();
        assert_eq!(alice.cards.len(), 0);

        // Note: In a real integration test, we would also verify that the GameWon event was emitted,
        // but that would require more complex event bus testing infrastructure
    }

    #[tokio::test]
    async fn test_execute_move_game_continues() {
        let game_manager = Arc::new(GameManager::new());
        let event_bus = EventBus::new();
        let subscriber = GameEventRoomSubscriber::new(game_manager.clone(), event_bus.clone());

        // Create a test game where Alice has multiple cards
        let test_game = Game::new(
            "test_room".to_string(),
            vec![
                Player {
                    name: "Alice".to_string(),
                    cards: vec![
                        Card::new(Rank::Three, Suit::Diamonds),
                        Card::new(Rank::Four, Suit::Hearts), // Multiple cards
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

        game_manager
            .update_game("test_room", test_game)
            .await
            .unwrap();

        // Alice plays one card - game should continue
        let result = subscriber
            .execute_move(
                "test_room",
                "Alice",
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // Should return false indicating game continues

        // Verify Alice has one card left
        let updated_game = game_manager.get_game("test_room").await.unwrap();
        let alice = updated_game.players().iter().find(|p| p.name == "Alice").unwrap();
        assert_eq!(alice.cards.len(), 1);
        assert!(alice.cards.contains(&Card::new(Rank::Four, Suit::Hearts)));
    }
}
