use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::{
    event::{EventBus, RoomEvent, RoomEventError, RoomEventHandler},
    game::GameService,
};

use super::{manager::BotManager, strategy_factory::BotStrategyFactory};

/// Event subscriber that handles bot actions in response to game events
pub struct BotRoomSubscriber {
    bot_manager: Arc<BotManager>,
    game_service: Arc<GameService>,
    event_bus: EventBus,
}

impl BotRoomSubscriber {
    pub fn new(
        bot_manager: Arc<BotManager>,
        game_service: Arc<GameService>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            bot_manager,
            game_service,
            event_bus,
        }
    }

    /// Handle a turn change event - check if it's a bot's turn and make a move
    async fn handle_turn_changed(
        &self,
        room_id: &str,
        player_uuid: &str,
    ) -> Result<(), RoomEventError> {
        // Check if the current player is a bot
        if !self.bot_manager.is_bot(player_uuid).await {
            debug!(
                room_id = %room_id,
                player_uuid = %player_uuid,
                "Turn changed to human player, no bot action needed"
            );
            return Ok(());
        }

        // Get the bot to retrieve its difficulty
        let bot = self.bot_manager.get_bot(player_uuid).await.ok_or_else(|| {
            RoomEventError::HandlerError(format!("Bot not found: {}", player_uuid))
        })?;

        info!(
            room_id = %room_id,
            bot_uuid = %player_uuid,
            difficulty = ?bot.difficulty,
            "Bot's turn detected, deciding move"
        );

        // Get the current game state
        let game = match self.game_service.get_game(room_id).await {
            Some(game) => game,
            None => {
                debug!(
                    room_id = %room_id,
                    bot_uuid = %player_uuid,
                    "Game not found (possibly deleted or reset), skipping bot move"
                );
                return Ok(());
            }
        };

        // Verify it's still the bot's turn (guard against race conditions)
        if game.current_player_turn() != player_uuid {
            debug!(
                room_id = %room_id,
                bot_uuid = %player_uuid,
                current_turn = %game.current_player_turn(),
                "Turn changed before bot could act, skipping move"
            );
            return Ok(());
        }

        // Add a small delay to simulate human thinking (100-500ms random)
        let delay_ms = 100 + (rand::random::<u64>() % 400);
        sleep(Duration::from_millis(delay_ms)).await;

        // Get strategy based on bot difficulty
        let strategy = BotStrategyFactory::create_strategy(bot.difficulty);

        // Use strategy to decide on a move with error handling
        let move_decision = match tokio::time::timeout(
            Duration::from_secs(5),
            strategy.decide_move(&game, player_uuid),
        )
        .await
        {
            Ok(decision) => decision,
            Err(_) => {
                error!(
                    room_id = %room_id,
                    bot_uuid = %player_uuid,
                    "Bot strategy timed out after 5 seconds, forcing pass"
                );
                None
            }
        };

        // Determine cards to play (empty array for pass)
        let cards = move_decision.unwrap_or_else(Vec::new);

        if cards.is_empty() {
            info!(
                room_id = %room_id,
                bot_uuid = %player_uuid,
                "Bot passing (no valid moves or strategic pass)"
            );
        } else {
            info!(
                room_id = %room_id,
                bot_uuid = %player_uuid,
                cards = ?cards,
                "Bot playing cards"
            );
        }

        // Emit a TryPlayMove event (with empty cards array if passing)
        self.event_bus
            .emit_to_room(
                room_id,
                RoomEvent::TryPlayMove {
                    player: player_uuid.to_string(),
                    cards,
                },
            )
            .await;

        Ok(())
    }
}

#[async_trait]
impl RoomEventHandler for BotRoomSubscriber {
    async fn handle_room_event(
        &self,
        room_id: &str,
        event: RoomEvent,
    ) -> Result<(), RoomEventError> {
        match event {
            RoomEvent::TurnChanged { player } => {
                self.handle_turn_changed(room_id, &player).await?;
            }
            RoomEvent::GameWon { winner } => {
                if self.bot_manager.is_bot(&winner).await {
                    info!(
                        room_id = %room_id,
                        bot_uuid = %winner,
                        "Bot won the game!"
                    );
                }
            }
            _ => {
                // Ignore other events
            }
        }

        Ok(())
    }

    fn handler_name(&self) -> &'static str {
        "BotRoomSubscriber"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::{Card, Game, Rank, Suit},
        user::{mapping_service::InMemoryPlayerMappingService, PlayerMappingService},
    };

    #[tokio::test]
    async fn test_bot_room_subscriber_handles_turn_changed() {
        let bot_manager = Arc::new(BotManager::new());
        let player_mapping = Arc::new(InMemoryPlayerMappingService::new());
        let game_service = Arc::new(GameService::new(player_mapping.clone()));
        let event_bus = EventBus::new();

        let subscriber =
            BotRoomSubscriber::new(bot_manager.clone(), game_service.clone(), event_bus.clone());

        // Create a bot
        let bot = bot_manager
            .create_bot(
                "room1".to_string(),
                super::super::types::BotDifficulty::Easy,
            )
            .await
            .unwrap();

        // Register bot in player mapping
        let _ = player_mapping
            .register_player(bot.uuid.clone(), bot.name.clone())
            .await;

        // Create a simple game with the bot
        let player_data = vec![
            (
                bot.name.clone(),
                bot.uuid.clone(),
                vec![Card::new(Rank::Three, Suit::Diamonds)],
            ),
            (
                "Human".to_string(),
                "human-123".to_string(),
                vec![Card::new(Rank::Four, Suit::Hearts)],
            ),
        ];

        game_service
            .create_game_with_cards("room1", player_data)
            .await
            .unwrap();

        // Subscribe to room events
        let mut rx = event_bus.subscribe_to_room("room1").await;

        // Handle turn changed event for the bot
        let result = subscriber
            .handle_room_event(
                "room1",
                RoomEvent::TurnChanged {
                    player: bot.uuid.clone(),
                },
            )
            .await;

        assert!(result.is_ok());

        // Check that a TryPlayMove event was emitted
        let emitted_event = rx.recv().await.unwrap();
        assert!(matches!(emitted_event, RoomEvent::TryPlayMove { .. }));
    }

    #[tokio::test]
    async fn test_bot_room_subscriber_ignores_human_turns() {
        let bot_manager = Arc::new(BotManager::new());
        let player_mapping = Arc::new(InMemoryPlayerMappingService::new());
        let game_service = Arc::new(GameService::new(player_mapping));
        let event_bus = EventBus::new();

        let subscriber = BotRoomSubscriber::new(bot_manager, game_service, event_bus.clone());

        // Subscribe to room events
        let mut rx = event_bus.subscribe_to_room("room1").await;

        // Handle turn changed event for a human player
        let result = subscriber
            .handle_room_event(
                "room1",
                RoomEvent::TurnChanged {
                    player: "human-123".to_string(),
                },
            )
            .await;

        assert!(result.is_ok());

        // No event should have been emitted
        let recv_result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(recv_result.is_err()); // Timeout means no event was received
    }
}
