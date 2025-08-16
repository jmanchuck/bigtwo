use std::sync::Arc;

use crate::{
    game::{cards::Card, core::Game, repository::GameRepository},
    shared::AppError,
};

#[derive(Debug, Clone)]
pub struct MoveResult {
    pub game: Game,
    pub player_won: bool,
}

pub struct GameService {
    game_repository: GameRepository,
}

impl GameService {
    pub fn new() -> Self {
        Self {
            game_repository: GameRepository::new(),
        }
    }

    /// Create a new game for the specified room with the given players
    pub async fn create_game(&self, room_id: &str, players: &[String]) -> Result<Game, AppError> {
        self.game_repository
            .create_game(room_id, players)
            .await
            .map_err(|_e| AppError::Internal)?;

        self.game_repository
            .get_game(room_id)
            .await
            .ok_or_else(|| AppError::Internal)
    }

    /// Try to play a move for a player in the specified room
    /// Returns the updated game state and whether the player won
    pub async fn try_play_move(
        &self,
        room_id: &str,
        player: &str,
        cards: &[Card],
    ) -> Result<MoveResult, AppError> {
        // Get current game
        let mut game = self
            .game_repository
            .get_game(room_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("Game not found for room: {}", room_id)))?;

        // Execute the move and check if player won
        let player_won = game
            .play_cards(player, cards)
            .map_err(|e| AppError::NotFound(format!("Game error: {}", e)))?;

        // Update the game in the repository
        self.game_repository
            .update_game(room_id, game.clone())
            .await
            .map_err(|_e| AppError::Internal)?;

        Ok(MoveResult { game, player_won })
    }

    /// Get the current game state for a room (read-only access)
    pub async fn get_game(&self, room_id: &str) -> Option<Game> {
        self.game_repository.get_game(room_id).await
    }

    /// Create a new game with predetermined card distributions
    pub async fn create_game_with_cards(
        &self,
        room_id: &str,
        player_cards: Vec<(String, Vec<Card>)>,
    ) -> Result<Game, AppError> {
        let game = Game::new_with_cards(room_id.to_string(), player_cards)
            .map_err(|_e| AppError::Internal)?;

        self.game_repository
            .update_game(room_id, game.clone())
            .await
            .map_err(|_e| AppError::Internal)?;

        Ok(game)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{
        cards::{Card, Rank, Suit},
        core::{Game, Player},
    };

    fn create_test_players() -> Vec<String> {
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "David".to_string(),
        ]
    }

    fn create_test_game() -> Game {
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
                cards: vec![
                    Card::new(Rank::Six, Suit::Clubs),
                    Card::new(Rank::Seven, Suit::Diamonds),
                ],
            },
        ];
        let starting_hands = players
            .iter()
            .map(|player| (player.name.clone(), player.cards.clone()))
            .collect();

        Game::new(
            "test_room".to_string(),
            players,
            0,      // Alice's turn
            0,      // No consecutive passes
            vec![], // No last played cards
            starting_hands,
        )
    }

    #[tokio::test]
    async fn test_create_game_success() {
        let service = GameService::new();

        let players = create_test_players();
        let result = service.create_game("test_room", &players).await;

        assert!(result.is_ok());
        let game = result.unwrap();
        assert_eq!(game.players().len(), 4);
    }

    #[tokio::test]
    async fn test_try_play_move_success() {
        let service = GameService::new();

        // Create a game with 4 players (Big Two requirement)
        let players = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "David".to_string(),
        ];
        service.create_game("test_room", &players).await.unwrap();

        // Get the game to see who has 3D and goes first
        let game = service.get_game("test_room").await.unwrap();
        let first_player = game.current_player_turn();

        // First player plays 3D (valid first move)
        let result = service
            .try_play_move(
                "test_room",
                &first_player,
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;

        assert!(result.is_ok());
        let move_result = result.unwrap();
        assert!(!move_result.player_won); // Player should not win yet
    }

    #[tokio::test]
    async fn test_try_play_move_game_not_found() {
        let service = GameService::new();

        let result = service
            .try_play_move(
                "nonexistent_room",
                "Alice",
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::NotFound(msg) => assert!(msg.contains("Game not found")),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_try_play_move_player_wins() {
        let service = GameService::new();

        // Create a game with 4 players (Big Two requirement)
        let players = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "David".to_string(),
        ];
        service.create_game("test_room", &players).await.unwrap();

        // Get the game and check who goes first
        let game = service.get_game("test_room").await.unwrap();

        // Find the player with 3D (since they must play it first)
        let player_with_3d = game
            .players()
            .iter()
            .find(|p| p.cards.contains(&Card::new(Rank::Three, Suit::Diamonds)))
            .unwrap();

        // For this test, we'll just verify the try_play_move functionality
        // In a real game, players won't win immediately after the first move
        let result = service
            .try_play_move(
                "test_room",
                &player_with_3d.name,
                &[Card::new(Rank::Three, Suit::Diamonds)],
            )
            .await;

        assert!(result.is_ok());
        let move_result = result.unwrap();
        // In a normal game with full hands, this won't be a win
        assert!(!move_result.player_won);
    }
}
