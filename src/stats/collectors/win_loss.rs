use async_trait::async_trait;

use crate::game::Game;

use super::super::{CollectedData, StatCollector, StatsError};

pub struct WinLossCollector;

impl WinLossCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StatCollector for WinLossCollector {
    async fn collect(
        &self,
        game: &Game,
        winner_uuid: &str,
    ) -> Result<Vec<CollectedData>, StatsError> {
        let players = game.players();

        if players.is_empty() {
            return Err(StatsError::Validation(
                "WinLossCollector requires at least one player".to_string(),
            ));
        }

        let data = players
            .iter()
            .map(|player| CollectedData::WinLoss {
                player_uuid: player.uuid.clone(),
                won: player.uuid == winner_uuid,
            })
            .collect();

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Card, Game, Player};
    use std::collections::HashMap;

    fn card(code: &str) -> Card {
        Card::from_string(code).unwrap()
    }

    fn players(vecs: Vec<(String, String, Vec<Card>)>) -> Vec<Player> {
        vecs.into_iter()
            .map(|(name, uuid, cards)| Player { name, uuid, cards })
            .collect()
    }

    fn game_with_players(vecs: Vec<(String, String, Vec<Card>)>) -> Game {
        let players_vec = players(vecs);
        Game::new(
            "room".to_string(),
            players_vec,
            0,
            0,
            vec![],
            HashMap::new(),
        )
    }

    fn empty_game() -> Game {
        Game::new("empty".to_string(), vec![], 0, 0, vec![], HashMap::new())
    }

    #[tokio::test]
    async fn marks_winner_correctly() {
        let collector = WinLossCollector::new();
        let game = game_with_players(vec![
            ("Alice".to_string(), "alice".to_string(), vec![card("3D")]),
            ("Bob".to_string(), "bob".to_string(), vec![card("4H")]),
        ]);

        let data = collector.collect(&game, "alice").await.unwrap();
        assert_eq!(data.len(), 2);
        assert!(matches!(data[0], CollectedData::WinLoss { won: true, .. }));
        assert!(matches!(data[1], CollectedData::WinLoss { won: false, .. }));
    }

    #[tokio::test]
    async fn errors_when_no_players() {
        let collector = WinLossCollector::new();
        let game = empty_game();

        let result = collector.collect(&game, "any").await;
        assert!(matches!(result, Err(StatsError::Validation(_))));
    }
}
