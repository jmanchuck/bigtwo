use async_trait::async_trait;

use crate::game::Game;

use super::super::{CollectedData, StatCollector, StatsError};

pub struct CardsRemainingCollector;

impl Default for CardsRemainingCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl CardsRemainingCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl StatCollector for CardsRemainingCollector {
    async fn collect(
        &self,
        game: &Game,
        _winner_uuid: &str,
    ) -> Result<Vec<CollectedData>, StatsError> {
        let data = game
            .players()
            .iter()
            .map(|player| CollectedData::CardsRemaining {
                player_uuid: player.uuid.clone(),
                count: player.cards.len() as u8,
            })
            .collect();

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Card, Game};

    fn card(code: &str) -> Card {
        Card::from_string(code).unwrap()
    }

    fn test_game(players: Vec<(String, String, Vec<Card>)>) -> Game {
        Game::new_game_with_cards("test-room".to_string(), players).unwrap()
    }

    #[tokio::test]
    async fn collects_card_counts_for_all_players() {
        let collector = CardsRemainingCollector::new();
        let game = test_game(vec![
            (
                "Alice".to_string(),
                "alice".to_string(),
                vec![card("3D"), card("4H")],
            ),
            ("Bot".to_string(), "bot-123".to_string(), vec![card("5S")]),
        ]);

        let data = collector.collect(&game, "alice").await.unwrap();
        assert_eq!(data.len(), 2);

        match &data[0] {
            CollectedData::CardsRemaining { player_uuid, count } => {
                assert_eq!(player_uuid, "alice");
                assert_eq!(*count, 2);
            }
            _ => panic!("Expected CardsRemaining variant"),
        }
    }
}
