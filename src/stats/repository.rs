use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{models::RoomStats, GameResult, PlayerGameResult, StatsError};

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn record_game(&self, game_result: GameResult) -> Result<(), StatsError>;
    async fn get_room_stats(&self, room_id: &str) -> Result<Option<RoomStats>, StatsError>;
    async fn reset_room_stats(&self, room_id: &str) -> Result<(), StatsError>;
}

#[derive(Debug, Default)]
pub struct InMemoryStatsRepository {
    rooms: Arc<RwLock<HashMap<String, RoomStats>>>,
}

impl InMemoryStatsRepository {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StatsRepository for InMemoryStatsRepository {
    async fn record_game(&self, game_result: GameResult) -> Result<(), StatsError> {
        let mut rooms = self.rooms.write().await;
        let room_stats = rooms
            .entry(game_result.room_id.clone())
            .or_insert_with(|| RoomStats {
                room_id: game_result.room_id.clone(),
                ..RoomStats::default()
            });

        room_stats.games_played += 1;

        for player_result in &game_result.players {
            let player_stats = room_stats
                .player_stats
                .entry(player_result.uuid.clone())
                .or_insert_with(|| super::PlayerStats {
                    uuid: player_result.uuid.clone(),
                    ..super::PlayerStats::default()
                });

            player_stats.games_played += 1;
            player_stats.total_score += player_result.final_score;

            if player_result.uuid == game_result.winner_uuid {
                player_stats.wins += 1;
                player_stats.current_win_streak += 1;
                player_stats.best_win_streak = player_stats
                    .best_win_streak
                    .max(player_stats.current_win_streak);
            } else {
                player_stats.current_win_streak = 0;
            }
        }

        Ok(())
    }

    async fn get_room_stats(&self, room_id: &str) -> Result<Option<RoomStats>, StatsError> {
        let rooms = self.rooms.read().await;
        Ok(rooms.get(room_id).cloned())
    }

    async fn reset_room_stats(&self, room_id: &str) -> Result<(), StatsError> {
        let mut rooms = self.rooms.write().await;
        rooms.remove(room_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_game(
        room_id: &str,
        winner_uuid: &str,
        players: Vec<(String, u8, i32, i32)>,
    ) -> GameResult {
        GameResult {
            room_id: room_id.to_string(),
            game_number: 1,
            winner_uuid: winner_uuid.to_string(),
            players: players
                .into_iter()
                .map(
                    |(uuid, cards_remaining, raw_score, final_score)| PlayerGameResult {
                        uuid,
                        cards_remaining,
                        raw_score,
                        final_score,
                    },
                )
                .collect(),
            completed_at: Utc::now(),
            had_bots: false,
        }
    }

    #[tokio::test]
    async fn records_game_and_updates_stats() {
        let repo = InMemoryStatsRepository::new();
        let game = sample_game(
            "room-1",
            "player-1",
            vec![
                ("player-1".to_string(), 0, 0, 0),
                ("player-2".to_string(), 5, 5, 5),
            ],
        );

        repo.record_game(game).await.unwrap();

        let stats = repo.get_room_stats("room-1").await.unwrap().unwrap();
        assert_eq!(stats.games_played, 1);
        assert_eq!(stats.player_stats.len(), 2);

        let winner = stats.player_stats.get("player-1").unwrap();
        assert_eq!(winner.wins, 1);
        assert_eq!(winner.current_win_streak, 1);
        assert_eq!(winner.best_win_streak, 1);

        let loser = stats.player_stats.get("player-2").unwrap();
        assert_eq!(loser.wins, 0);
        assert_eq!(loser.current_win_streak, 0);
        assert_eq!(loser.total_score, 5);
    }

    #[tokio::test]
    async fn maintains_streaks_across_games() {
        let repo = InMemoryStatsRepository::new();

        let players = vec![
            ("player-1".to_string(), 0, 0, 0),
            ("player-2".to_string(), 3, 3, 3),
        ];

        repo.record_game(sample_game("room", "player-1", players.clone()))
            .await
            .unwrap();
        repo.record_game(sample_game("room", "player-1", players.clone()))
            .await
            .unwrap();
        repo.record_game(sample_game("room", "player-2", players.clone()))
            .await
            .unwrap();

        let stats = repo.get_room_stats("room").await.unwrap().unwrap();
        let player1 = stats.player_stats.get("player-1").unwrap();
        assert_eq!(player1.wins, 2);
        assert_eq!(player1.best_win_streak, 2);
        assert_eq!(player1.current_win_streak, 0);
    }

    #[tokio::test]
    async fn reset_clears_room_stats() {
        let repo = InMemoryStatsRepository::new();
        repo.record_game(sample_game(
            "room-reset",
            "player-1",
            vec![
                ("player-1".to_string(), 0, 0, 0),
                ("player-2".to_string(), 2, 2, 2),
            ],
        ))
        .await
        .unwrap();

        repo.reset_room_stats("room-reset").await.unwrap();

        let stats = repo.get_room_stats("room-reset").await.unwrap();
        assert!(stats.is_none());
    }
}
