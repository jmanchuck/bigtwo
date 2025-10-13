use std::sync::Arc;

use super::{
    basic_strategy::BasicBotStrategy,
    types::{BotDifficulty, BotStrategy},
};

/// Factory for creating bot strategies based on difficulty level
pub struct BotStrategyFactory;

impl BotStrategyFactory {
    /// Create a strategy instance for the given difficulty level
    pub fn create_strategy(difficulty: BotDifficulty) -> Arc<dyn BotStrategy> {
        match difficulty {
            // All difficulty levels currently use BasicBotStrategy
            // Future implementation: add Medium and Hard strategies
            BotDifficulty::Easy | BotDifficulty::Medium | BotDifficulty::Hard => {
                Arc::new(BasicBotStrategy::new())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_easy_strategy() {
        let strategy = BotStrategyFactory::create_strategy(BotDifficulty::Easy);
        assert_eq!(strategy.strategy_name(), "BasicBotStrategy");
    }

    #[test]
    fn test_create_medium_strategy() {
        let strategy = BotStrategyFactory::create_strategy(BotDifficulty::Medium);
        // Currently uses BasicBotStrategy, will be updated when MediumBotStrategy is implemented
        assert_eq!(strategy.strategy_name(), "BasicBotStrategy");
    }

    #[test]
    fn test_create_hard_strategy() {
        let strategy = BotStrategyFactory::create_strategy(BotDifficulty::Hard);
        // Currently uses BasicBotStrategy, will be updated when HardBotStrategy is implemented
        assert_eq!(strategy.strategy_name(), "BasicBotStrategy");
    }
}
