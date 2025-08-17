use thiserror::Error;

use super::basic::{Card, Rank, Suit};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HandError {
    #[error("Invalid hand size")]
    InvalidHandSize,
    #[error("Invalid hand type")]
    InvalidHandType,
    #[error("Cards not sorted")]
    CardsNotSorted,
    #[error("Cannot compare across types")]
    CannotCompareAcrossTypes,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SingleHand {
    pub card: Card,
}

impl SingleHand {
    pub fn new(card: Card) -> Self {
        Self { card }
    }
}

impl PartialOrd for SingleHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.card.cmp(&other.card))
    }
}

impl Ord for SingleHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.card.cmp(&other.card)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PairHand {
    pub rank: Rank,
    pub cards: [Card; 2], // Store both cards in the pair
    pub high_card: Card,  // The higher suit of the pair (kept for comparison)
}

impl PairHand {
    pub fn new(card1: Card, card2: Card) -> Result<Self, HandError> {
        if card1.rank != card2.rank {
            return Err(HandError::InvalidHandType);
        }

        let high_card = if card1.suit > card2.suit {
            card1
        } else {
            card2
        };

        // Store cards in a consistent order: lower suit first, higher suit second
        let cards = if card1.suit < card2.suit {
            [card1, card2]
        } else {
            [card2, card1]
        };

        Ok(Self {
            rank: card1.rank,
            cards,
            high_card,
        })
    }
}

impl PartialOrd for PairHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PairHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.rank.cmp(&other.rank) {
            std::cmp::Ordering::Equal => self.high_card.suit.cmp(&other.high_card.suit),
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TripleHand {
    pub rank: Rank,
    pub cards: [Card; 3], // Store all three cards in the triple
    pub high_card: Card,  // The highest suit of the triple (kept for comparison)
}

impl TripleHand {
    pub fn new(card1: Card, card2: Card, card3: Card) -> Result<Self, HandError> {
        if !(card1.rank == card2.rank && card2.rank == card3.rank) {
            return Err(HandError::InvalidHandType);
        }

        let high_card = *[card1, card2, card3].iter().max().unwrap();

        // Store cards in sorted order by suit
        let mut cards = [card1, card2, card3];
        cards.sort_by_key(|card| card.suit);

        Ok(Self {
            rank: card1.rank,
            cards,
            high_card,
        })
    }
}

impl PartialOrd for TripleHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TripleHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.rank.cmp(&other.rank) {
            std::cmp::Ordering::Equal => self.high_card.suit.cmp(&other.high_card.suit),
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StraightHand {
    pub cards: Vec<Card>,
    pub high_card: Card,
}

impl StraightHand {
    pub fn new(cards: Vec<Card>) -> Self {
        let high_card = *cards.iter().max().unwrap();
        Self { cards, high_card }
    }
}

impl PartialOrd for StraightHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StraightHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Straights compare by high card only
        self.high_card.cmp(&other.high_card)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FlushHand {
    pub cards: Vec<Card>,
    pub suit: Suit,
    pub high_card: Card,
}

impl FlushHand {
    pub fn new(cards: Vec<Card>) -> Self {
        let suit = cards[0].suit; // All cards have same suit in a flush
        let high_card = *cards.iter().max().unwrap();
        Self {
            cards,
            suit,
            high_card,
        }
    }
}

impl PartialOrd for FlushHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FlushHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Flushes: compare by suit first, then by highest card
        match self.suit.cmp(&other.suit) {
            std::cmp::Ordering::Equal => self.high_card.cmp(&other.high_card),
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FullHouseHand {
    pub cards: Vec<Card>,
    pub triple_rank: Rank,
    pub high_card: Card,
}

impl FullHouseHand {
    pub fn new(cards: Vec<Card>) -> Self {
        let rank_counts = Self::count_ranks(&cards);
        let triple_rank = *rank_counts.iter().find(|(_, &count)| count == 3).unwrap().0;
        let high_card = *cards.iter().max().unwrap();
        Self {
            cards,
            triple_rank,
            high_card,
        }
    }

    fn count_ranks(cards: &[Card]) -> std::collections::HashMap<Rank, usize> {
        let mut counts = std::collections::HashMap::new();
        for card in cards {
            *counts.entry(card.rank).or_insert(0) += 1;
        }
        counts
    }
}

impl PartialOrd for FullHouseHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FullHouseHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Full houses compare by the rank of the triple
        self.triple_rank.cmp(&other.triple_rank)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FourOfAKindHand {
    pub cards: Vec<Card>,
    pub quad_rank: Rank,
    pub high_card: Card,
}

impl FourOfAKindHand {
    pub fn new(cards: Vec<Card>) -> Self {
        let rank_counts = Self::count_ranks(&cards);
        let quad_rank = *rank_counts.iter().find(|(_, &count)| count == 4).unwrap().0;
        let high_card = *cards.iter().max().unwrap();
        Self {
            cards,
            quad_rank,
            high_card,
        }
    }

    fn count_ranks(cards: &[Card]) -> std::collections::HashMap<Rank, usize> {
        let mut counts = std::collections::HashMap::new();
        for card in cards {
            *counts.entry(card.rank).or_insert(0) += 1;
        }
        counts
    }
}

impl PartialOrd for FourOfAKindHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FourOfAKindHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Four of a kind compares by the rank of the quad
        self.quad_rank.cmp(&other.quad_rank)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StraightFlushHand {
    pub cards: Vec<Card>,
    pub suit: Suit,
    pub high_card: Card,
}

impl StraightFlushHand {
    pub fn new(cards: Vec<Card>) -> Self {
        let suit = cards[0].suit; // All cards have same suit
        let high_card = *cards.iter().max().unwrap();
        Self {
            cards,
            suit,
            high_card,
        }
    }
}

impl PartialOrd for StraightFlushHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StraightFlushHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Straight flushes: compare by suit first, then by highest card
        match self.suit.cmp(&other.suit) {
            std::cmp::Ordering::Equal => self.high_card.cmp(&other.high_card),
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FiveCardHand {
    Straight(StraightHand),
    Flush(FlushHand),
    FullHouse(FullHouseHand),
    FourOfAKind(FourOfAKindHand),
    StraightFlush(StraightFlushHand),
}

impl FiveCardHand {
    pub fn new(cards: &[Card]) -> Result<Self, HandError> {
        if cards.len() != 5 {
            return Err(HandError::InvalidHandSize);
        }

        let mut cards = cards.to_owned();
        cards.sort();
        Self::classify_hand(cards)
    }

    fn classify_hand(cards: Vec<Card>) -> Result<Self, HandError> {
        let is_flush = cards.iter().all(|card| card.suit == cards[0].suit);
        let is_straight = Self::is_straight(&cards);

        if is_straight && is_flush {
            return Ok(FiveCardHand::StraightFlush(StraightFlushHand::new(cards)));
        }

        // Check for four of a kind
        let rank_counts = Self::count_ranks(&cards);
        if rank_counts.values().any(|&count| count == 4) {
            return Ok(FiveCardHand::FourOfAKind(FourOfAKindHand::new(cards)));
        }

        // Check for full house
        let mut counts: Vec<_> = rank_counts.values().collect();
        counts.sort();
        if counts == vec![&2, &3] {
            return Ok(FiveCardHand::FullHouse(FullHouseHand::new(cards)));
        }

        if is_flush {
            return Ok(FiveCardHand::Flush(FlushHand::new(cards)));
        }

        if is_straight {
            return Ok(FiveCardHand::Straight(StraightHand::new(cards)));
        }

        Err(HandError::InvalidHandType)
    }

    fn is_straight(cards: &[Card]) -> bool {
        // Extract and sort ranks
        let mut ranks: Vec<Rank> = cards.iter().map(|c| c.rank).collect();
        ranks.sort();

        // Special case 1: Ace-low straight (A-2-3-4-5)
        // Ace acts as value 1, Two acts as value 2
        if ranks == vec![Rank::Three, Rank::Four, Rank::Five, Rank::Ace, Rank::Two] {
            return true;
        }

        // Special case 2: Ace-high straight (10-J-Q-K-A)
        // Ace acts as value 14 (after King)
        if ranks == vec![Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace] {
            return true;
        }

        // Normal consecutive straights: 3-4-5-6-7 through 9-10-J-Q-K
        // These should be consecutive in rank value and not contain Ace or Two
        // (because Ace and Two have special positioning rules)
        if ranks.contains(&Rank::Ace) || ranks.contains(&Rank::Two) {
            return false; // Any other combination with Ace or Two is invalid
        }

        // Check if remaining ranks are consecutive
        for i in 1..ranks.len() {
            let prev_rank_value = ranks[i - 1] as u8;
            let curr_rank_value = ranks[i] as u8;

            if curr_rank_value != prev_rank_value + 1 {
                return false;
            }
        }

        true
    }

    fn count_ranks(cards: &[Card]) -> std::collections::HashMap<Rank, usize> {
        let mut counts = std::collections::HashMap::new();
        for card in cards {
            *counts.entry(card.rank).or_insert(0) += 1;
        }
        counts
    }

    pub fn hand_type_value(&self) -> u8 {
        match self {
            FiveCardHand::Straight(_) => 0,
            FiveCardHand::Flush(_) => 1,
            FiveCardHand::FullHouse(_) => 2,
            FiveCardHand::FourOfAKind(_) => 3,
            FiveCardHand::StraightFlush(_) => 4,
        }
    }
}

impl PartialOrd for FiveCardHand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FiveCardHand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.hand_type_value().cmp(&other.hand_type_value()) {
            std::cmp::Ordering::Equal => {
                // Same hand type - delegate to specific type comparison
                match (self, other) {
                    (FiveCardHand::Straight(a), FiveCardHand::Straight(b)) => a.cmp(b),
                    (FiveCardHand::Flush(a), FiveCardHand::Flush(b)) => a.cmp(b),
                    (FiveCardHand::FullHouse(a), FiveCardHand::FullHouse(b)) => a.cmp(b),
                    (FiveCardHand::FourOfAKind(a), FiveCardHand::FourOfAKind(b)) => a.cmp(b),
                    (FiveCardHand::StraightFlush(a), FiveCardHand::StraightFlush(b)) => a.cmp(b),
                    _ => unreachable!("Same hand type value but different variants"),
                }
            }
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Hand {
    Pass,
    Single(SingleHand),
    Pair(PairHand),
    Triple(TripleHand),
    Five(FiveCardHand),
}

impl Hand {
    /// Create a hand from a vector of cards
    pub fn from_cards(cards: &[Card]) -> Result<Self, HandError> {
        match cards.len() {
            0 => Ok(Hand::Pass),
            1 => Ok(Hand::Single(SingleHand::new(cards[0]))),
            2 => Ok(Hand::Pair(PairHand::new(cards[0], cards[1])?)),
            3 => Ok(Hand::Triple(TripleHand::new(cards[0], cards[1], cards[2])?)),
            5 => Ok(Hand::Five(FiveCardHand::new(cards)?)),
            _ => Err(HandError::InvalidHandSize),
        }
    }

    /// Check if this hand can beat another hand according to Big Two rules
    /// Only hands of the same type can be compared (except all 5-card combos can compete)
    pub fn can_beat(&self, other: &Hand) -> bool {
        match (self, other) {
            (Hand::Pass, _) => true,
            (_, Hand::Pass) => false,
            (Hand::Single(a), Hand::Single(b)) => a > b,
            (Hand::Pair(a), Hand::Pair(b)) => a > b,
            (Hand::Triple(a), Hand::Triple(b)) => a > b,
            (Hand::Five(a), Hand::Five(b)) => a > b,
            _ => false, // Cannot compare across different hand types
        }
    }

    /// Get the hand type name for display
    pub fn hand_type_name(&self) -> &'static str {
        match self {
            Hand::Pass => "Pass",
            Hand::Single(_) => "Single",
            Hand::Pair(_) => "Pair",
            Hand::Triple(_) => "Triple",
            Hand::Five(five) => match five {
                FiveCardHand::Straight(_) => "Straight",
                FiveCardHand::Flush(_) => "Flush",
                FiveCardHand::FullHouse(_) => "Full House",
                FiveCardHand::FourOfAKind(_) => "Four of a Kind",
                FiveCardHand::StraightFlush(_) => "Straight Flush",
            },
        }
    }

    /// Get the cards that make up this hand
    pub fn to_cards(&self) -> Vec<Card> {
        match self {
            Hand::Pass => vec![],
            Hand::Single(single) => vec![single.card],
            Hand::Pair(pair) => {
                // Return all cards in the pair
                pair.cards.to_vec()
            }
            Hand::Triple(triple) => {
                // Return all cards in the triple
                triple.cards.to_vec()
            }
            Hand::Five(five) => match five {
                FiveCardHand::Straight(straight) => straight.cards.clone(),
                FiveCardHand::Flush(flush) => flush.cards.clone(),
                FiveCardHand::FullHouse(full_house) => full_house.cards.clone(),
                FiveCardHand::FourOfAKind(four_kind) => four_kind.cards.clone(),
                FiveCardHand::StraightFlush(straight_flush) => straight_flush.cards.clone(),
            },
        }
    }
}

/// Compare two sets of played cards to determine if the first can beat the second
/// Returns Ok(true) if played_cards can beat current_cards, Ok(false) otherwise
/// Empty played_cards represents a pass
pub fn compare_played_cards(
    played_cards: &[Card],
    current_cards: &[Card],
) -> Result<bool, HandError> {
    let played_hand = if played_cards.is_empty() {
        Hand::Pass
    } else {
        Hand::from_cards(played_cards)?
    };

    let current_hand = if current_cards.is_empty() {
        Hand::Pass
    } else {
        Hand::from_cards(current_cards)?
    };

    Ok(played_hand.can_beat(&current_hand))
}