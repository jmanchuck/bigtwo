use std::collections::HashMap;
use std::sync::Arc;

use crate::game::Game;

use super::{
    calculators::{CardCountScoreCalculator, TenPlusMultiplierCalculator},
    collectors::CardsRemainingCollector,
    repository::StatsRepository,
    CalculationContext, CollectedData, GameResult, PlayerGameResult, ScoreCalculator,
    StatCollector, StatsError,
};

pub struct StatsService {
    collectors: Vec<Arc<dyn StatCollector>>,
    calculators: Vec<Arc<dyn ScoreCalculator>>,
    repository: Arc<dyn StatsRepository>,
}

impl StatsService {
    pub fn builder(repository: Arc<dyn StatsRepository>) -> StatsServiceBuilder {
        StatsServiceBuilder::new(repository)
    }

    pub async fn record_game_completion(
        &self,
        room_id: &str,
        game_number: u32,
        game: &Game,
        winner_uuid: &str,
        had_bots: bool,
    ) -> Result<GameResult, StatsError> {
        let collected = self.collect_all(game, winner_uuid).await?;

        let player_metadata: Vec<(String, usize)> = game
            .players()
            .iter()
            .map(|p| (p.uuid.clone(), p.cards.len()))
            .collect();

        let (raw_scores, final_scores) = self.calculate_scores(
            &player_metadata,
            &collected,
            room_id,
            game_number,
            winner_uuid,
            had_bots,
        );

        let player_results: Vec<PlayerGameResult> = player_metadata
            .iter()
            .map(|(uuid, cards)| PlayerGameResult {
                uuid: uuid.clone(),
                cards_remaining: *cards as u8,
                raw_score: raw_scores.get(uuid).copied().unwrap_or_default(),
                final_score: final_scores.get(uuid).copied().unwrap_or_default(),
            })
            .collect();

        let game_result = GameResult {
            room_id: room_id.to_string(),
            game_number,
            winner_uuid: winner_uuid.to_string(),
            players: player_results,
            completed_at: chrono::Utc::now(),
            had_bots,
        };

        self.repository.record_game(game_result.clone()).await?;
        Ok(game_result)
    }

    fn calculate_scores(
        &self,
        player_metadata: &[(String, usize)],
        collected: &[CollectedData],
        room_id: &str,
        game_number: u32,
        winner_uuid: &str,
        had_bots: bool,
    ) -> (HashMap<String, i32>, HashMap<String, i32>) {
        let mut current_scores: HashMap<String, i32> = HashMap::new();
        let mut raw_scores: HashMap<String, i32> = HashMap::new();

        for (index, calculator) in self.calculators.iter().enumerate() {
            let snapshot_players: Vec<PlayerGameResult> = player_metadata
                .iter()
                .map(|(uuid, cards)| PlayerGameResult {
                    uuid: uuid.clone(),
                    cards_remaining: *cards as u8,
                    raw_score: current_scores.get(uuid).copied().unwrap_or_default(),
                    final_score: current_scores.get(uuid).copied().unwrap_or_default(),
                })
                .collect();

            let snapshot = GameResult {
                room_id: room_id.to_string(),
                game_number,
                winner_uuid: winner_uuid.to_string(),
                players: snapshot_players,
                completed_at: chrono::Utc::now(),
                had_bots,
            };

            let context = CalculationContext::new(&snapshot, &current_scores);

            let mut next_scores = current_scores.clone();
            for (uuid, _) in player_metadata {
                let updated = calculator.calculate(uuid, collected, &context);
                next_scores.insert(uuid.clone(), updated);
            }

            if index == 0 {
                raw_scores = next_scores.clone();
            }

            current_scores = next_scores;
        }

        (raw_scores, current_scores)
    }

    async fn collect_all(
        &self,
        game: &Game,
        winner_uuid: &str,
    ) -> Result<Vec<CollectedData>, StatsError> {
        let mut collected = Vec::new();
        for collector in &self.collectors {
            collected.extend(collector.collect(game, winner_uuid).await?);
        }
        Ok(collected)
    }
}

pub struct StatsServiceBuilder {
    collectors: Vec<Arc<dyn StatCollector>>,
    calculators: Vec<Arc<dyn ScoreCalculator>>,
    repository: Arc<dyn StatsRepository>,
}

impl StatsServiceBuilder {
    fn new(repository: Arc<dyn StatsRepository>) -> Self {
        Self {
            collectors: vec![Arc::new(CardsRemainingCollector::new())],
            calculators: vec![
                Arc::new(CardCountScoreCalculator::new()),
                Arc::new(TenPlusMultiplierCalculator::new()),
            ],
            repository,
        }
    }

    pub fn with_collector(mut self, collector: Arc<dyn StatCollector>) -> Self {
        self.collectors.push(collector);
        self
    }

    pub fn with_calculator(mut self, calculator: Arc<dyn ScoreCalculator>) -> Self {
        self.calculators.push(calculator);
        self
    }

    pub fn build(mut self) -> StatsService {
        self.calculators.sort_by_key(|c| c.priority());
        StatsService {
            collectors: self.collectors,
            calculators: self.calculators,
            repository: self.repository,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Card, Game};
    use crate::stats::InMemoryStatsRepository;
    use chrono::Utc;

    fn card(code: &str) -> Card {
        Card::from_string(code).unwrap()
    }

    fn game_with_players(players: Vec<(String, String, Vec<Card>)>) -> Game {
        Game::new_game_with_cards("room".to_string(), players).unwrap()
    }

    fn repo_and_service() -> (Arc<InMemoryStatsRepository>, StatsService) {
        let repo = Arc::new(InMemoryStatsRepository::new());
        let service = StatsService::builder(repo.clone()).build();
        (repo, service)
    }

    #[tokio::test]
    async fn records_raw_and_final_scores() {
        let (repo, service) = repo_and_service();
        let game = game_with_players(vec![
            (
                "Alice".to_string(),
                "alice".to_string(),
                (1..=13).map(|_| card("3D")).collect(),
            ),
            (
                "Bob".to_string(),
                "bob".to_string(),
                vec![card("4H"), card("5S")],
            ),
        ]);

        let result = service
            .record_game_completion("room", 1, &game, "bob", false)
            .await
            .unwrap();

        let alice = result.players.iter().find(|p| p.uuid == "alice").unwrap();
        assert!(alice.raw_score > 0);
        assert!(alice.final_score >= alice.raw_score);

        let room_stats = repo.get_room_stats("room").await.unwrap().unwrap();
        assert_eq!(room_stats.games_played, 1);
        assert!(room_stats.player_stats.get("bob").unwrap().wins > 0);
    }
}
