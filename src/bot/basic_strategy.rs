use async_trait::async_trait;
use tracing::debug;

use crate::game::{Card, Game, Hand, Rank, SingleHand, Suit};

use super::types::BotStrategy;

/// Basic bot strategy that plays the lowest valid cards
pub struct BasicBotStrategy;

impl BasicBotStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Find all valid single card moves
    fn find_valid_singles(&self, game: &Game, available_cards: &[Card]) -> Vec<Vec<Card>> {
        let mut valid_moves = Vec::new();

        // If it's the first turn, must include 3D
        if game.played_hands().is_empty() {
            let three_diamonds = Card::new(Rank::Three, Suit::Diamonds);
            if available_cards.contains(&three_diamonds) {
                valid_moves.push(vec![three_diamonds]);
            }
            return valid_moves;
        }

        // If table is clear (3 consecutive passes), can play any single
        if game.consecutive_passes() >= 3 {
            for card in available_cards {
                valid_moves.push(vec![*card]);
            }
            return valid_moves;
        }

        // Need to beat the last non-pass hand
        if let Some(last_hand) = game.played_hands().iter().rev().find(|h| **h != Hand::Pass) {
            for card in available_cards {
                let potential_hand = Hand::from_cards(&[*card]);
                if let Ok(hand) = potential_hand {
                    if hand.can_beat(last_hand) {
                        valid_moves.push(vec![*card]);
                    }
                }
            }
        } else {
            // No previous hand, can play any single
            for card in available_cards {
                valid_moves.push(vec![*card]);
            }
        }

        valid_moves
    }

    /// Find all valid pair moves
    fn find_valid_pairs(&self, game: &Game, available_cards: &[Card]) -> Vec<Vec<Card>> {
        let mut valid_moves = Vec::new();

        // First turn must include 3D
        if game.played_hands().is_empty() {
            let three_diamonds = Card::new(Rank::Three, Suit::Diamonds);
            if available_cards.contains(&three_diamonds) {
                // Find another 3 to make a pair
                for card in available_cards {
                    if card.rank == Rank::Three && *card != three_diamonds {
                        let mut pair = vec![three_diamonds, *card];
                        pair.sort();
                        if Hand::from_cards(&pair).is_ok() {
                            valid_moves.push(pair);
                        }
                    }
                }
            }
            return valid_moves;
        }

        // Group cards by rank
        let mut rank_groups: std::collections::HashMap<Rank, Vec<Card>> =
            std::collections::HashMap::new();
        for card in available_cards {
            rank_groups.entry(card.rank).or_insert_with(Vec::new).push(*card);
        }

        // Find pairs
        for (_, cards) in rank_groups.iter() {
            if cards.len() >= 2 {
                // Try all combinations of 2 cards from this rank
                for i in 0..cards.len() {
                    for j in i + 1..cards.len() {
                        let mut pair = vec![cards[i], cards[j]];
                        pair.sort();

                        if let Ok(hand) = Hand::from_cards(&pair) {
                            // Check if this beats the last hand
                            if game.consecutive_passes() >= 3 {
                                valid_moves.push(pair);
                            } else if let Some(last_hand) =
                                game.played_hands().iter().rev().find(|h| **h != Hand::Pass)
                            {
                                if hand.can_beat(last_hand) {
                                    valid_moves.push(pair);
                                }
                            } else {
                                valid_moves.push(pair);
                            }
                        }
                    }
                }
            }
        }

        valid_moves
    }

    /// Choose the best move from available options
    fn choose_best_move(&self, valid_moves: Vec<Vec<Card>>) -> Option<Vec<Card>> {
        if valid_moves.is_empty() {
            return None;
        }

        // Strategy: Play the lowest value cards first
        // Sort by the sum of card ranks (lower rank = lower value)
        let mut scored_moves: Vec<(Vec<Card>, u32)> = valid_moves
            .into_iter()
            .map(|cards| {
                let score: u32 = cards.iter().map(|c| c.rank as u32).sum();
                (cards, score)
            })
            .collect();

        scored_moves.sort_by_key(|(_, score)| *score);

        // Return the move with the lowest score
        scored_moves.into_iter().next().map(|(cards, _)| cards)
    }
}

#[async_trait]
impl BotStrategy for BasicBotStrategy {
    async fn decide_move(&self, game: &Game, bot_uuid: &str) -> Option<Vec<Card>> {
        // Verify it's the bot's turn
        if game.current_player_turn() != bot_uuid {
            debug!(bot_uuid = %bot_uuid, "Not bot's turn");
            return None;
        }

        // Get the bot's available cards
        let bot_player = game.players().iter().find(|p| p.uuid == bot_uuid)?;
        let available_cards = &bot_player.cards;

        debug!(
            bot_uuid = %bot_uuid,
            card_count = available_cards.len(),
            "Bot deciding move"
        );

        // Find all valid moves
        let mut all_valid_moves = Vec::new();

        // Try singles
        all_valid_moves.extend(self.find_valid_singles(game, available_cards));

        // Try pairs (only if we have at least 2 cards)
        if available_cards.len() >= 2 {
            all_valid_moves.extend(self.find_valid_pairs(game, available_cards));
        }

        // Choose the best move
        let chosen_move = self.choose_best_move(all_valid_moves);

        debug!(
            bot_uuid = %bot_uuid,
            chosen_move = ?chosen_move,
            "Bot decided on move"
        );

        chosen_move
    }

    fn strategy_name(&self) -> &'static str {
        "BasicBotStrategy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Game, Player};

    #[tokio::test]
    async fn test_bot_plays_three_diamonds_first_turn() {
        let strategy = BasicBotStrategy::new();

        let players = vec![
            Player {
                name: "Bot".to_string(),
                uuid: "bot-123".to_string(),
                cards: vec![
                    Card::new(Rank::Three, Suit::Diamonds),
                    Card::new(Rank::Four, Suit::Hearts),
                ],
            },
            Player {
                name: "Human".to_string(),
                uuid: "human-456".to_string(),
                cards: vec![Card::new(Rank::Five, Suit::Spades)],
            },
        ];

        let game = Game::new(
            "test".to_string(),
            players.clone(),
            0,
            0,
            vec![],
            players.iter().map(|p| (p.uuid.clone(), p.cards.clone())).collect(),
        );

        let move_decision = strategy.decide_move(&game, "bot-123").await;
        assert!(move_decision.is_some());
        assert_eq!(move_decision.unwrap(), vec![Card::new(Rank::Three, Suit::Diamonds)]);
    }

    #[tokio::test]
    async fn test_bot_passes_when_no_valid_moves() {
        let strategy = BasicBotStrategy::new();

        let players = vec![
            Player {
                name: "Bot".to_string(),
                uuid: "bot-123".to_string(),
                cards: vec![Card::new(Rank::Three, Suit::Diamonds)],
            },
            Player {
                name: "Human".to_string(),
                uuid: "human-456".to_string(),
                cards: vec![Card::new(Rank::Five, Suit::Spades)],
            },
        ];

        let game = Game::new(
            "test".to_string(),
            players.clone(),
            0,
            0,
            vec![Hand::Single(SingleHand::new(Card::new(Rank::King, Suit::Spades)))],
            players.iter().map(|p| (p.uuid.clone(), p.cards.clone())).collect(),
        );

        let move_decision = strategy.decide_move(&game, "bot-123").await;
        assert!(move_decision.is_none()); // Bot should pass
    }

    #[tokio::test]
    async fn test_bot_plays_lowest_card() {
        let strategy = BasicBotStrategy::new();

        let players = vec![
            Player {
                name: "Bot".to_string(),
                uuid: "bot-123".to_string(),
                cards: vec![
                    Card::new(Rank::Three, Suit::Diamonds),
                    Card::new(Rank::King, Suit::Spades),
                    Card::new(Rank::Ace, Suit::Hearts),
                ],
            },
            Player {
                name: "Human".to_string(),
                uuid: "human-456".to_string(),
                cards: vec![Card::new(Rank::Five, Suit::Spades)],
            },
        ];

        let game = Game::new(
            "test".to_string(),
            players.clone(),
            0,
            0,
            vec![],
            players.iter().map(|p| (p.uuid.clone(), p.cards.clone())).collect(),
        );

        let move_decision = strategy.decide_move(&game, "bot-123").await;
        assert!(move_decision.is_some());
        // Should play 3D (first turn requirement)
        assert_eq!(move_decision.unwrap(), vec![Card::new(Rank::Three, Suit::Diamonds)]);
    }
}
