use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use super::hands::HandError;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, EnumIter,
)]
pub enum Suit {
    Diamonds = 0,
    Clubs = 1,
    Hearts = 2,
    Spades = 3,
}

impl PartialOrd for Suit {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Suit {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Suit::Diamonds => "D",
                Suit::Clubs => "C",
                Suit::Hearts => "H",
                Suit::Spades => "S",
            }
        )
    }
}

impl TryFrom<&str> for Suit {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "D" => Ok(Suit::Diamonds),
            "C" => Ok(Suit::Clubs),
            "H" => Ok(Suit::Hearts),
            "S" => Ok(Suit::Spades),
            _ => Err(s.to_string()),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, EnumIter,
)]
pub enum Rank {
    Three = 0,
    Four = 1,
    Five = 2,
    Six = 3,
    Seven = 4,
    Eight = 5,
    Nine = 6,
    Ten = 7,
    Jack = 8,
    Queen = 9,
    King = 10,
    Ace = 11,
    Two = 12,
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Rank::Three => "3",
                Rank::Four => "4",
                Rank::Five => "5",
                Rank::Six => "6",
                Rank::Seven => "7",
                Rank::Eight => "8",
                Rank::Nine => "9",
                Rank::Ten => "T",
                Rank::Jack => "J",
                Rank::Queen => "Q",
                Rank::King => "K",
                Rank::Ace => "A",
                Rank::Two => "2",
            }
        )
    }
}

impl TryFrom<&str> for Rank {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "3" => Ok(Rank::Three),
            "4" => Ok(Rank::Four),
            "5" => Ok(Rank::Five),
            "6" => Ok(Rank::Six),
            "7" => Ok(Rank::Seven),
            "8" => Ok(Rank::Eight),
            "9" => Ok(Rank::Nine),
            "T" => Ok(Rank::Ten),
            "J" => Ok(Rank::Jack),
            "Q" => Ok(Rank::Queen),
            "K" => Ok(Rank::King),
            "A" => Ok(Rank::Ace),
            "2" => Ok(Rank::Two),
            _ => Err(s.to_string()),
        }
    }
}

impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rank {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.rank.cmp(&other.rank) {
            std::cmp::Ordering::Equal => self.suit.cmp(&other.suit),
            other => other,
        }
    }
}

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { suit, rank }
    }

    pub fn from_string(s: &str) -> Result<Self, HandError> {
        if s.len() != 2 {
            return Err(HandError::InvalidHandType);
        }

        let rank = Rank::try_from(&s[0..1]).map_err(|_| HandError::InvalidHandType)?;
        let suit = Suit::try_from(&s[1..2]).map_err(|_| HandError::InvalidHandType)?;

        Ok(Self::new(rank, suit))
    }

    pub fn all_cards() -> Vec<Card> {
        let mut cards = Vec::new();
        for suit in Suit::iter() {
            for rank in Rank::iter() {
                cards.push(Card::new(rank, suit));
            }
        }
        cards
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank, self.suit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_ordering() {
        let card1 = Card::new(Rank::Three, Suit::Diamonds);
        let card2 = Card::new(Rank::Three, Suit::Spades);
        let card3 = Card::new(Rank::Two, Suit::Diamonds);

        assert!(card2 > card1); // Same rank, higher suit
        assert!(card3 > card1); // Higher rank
        assert!(card3 > card2); // Higher rank beats higher suit
    }

    #[test]
    fn test_card_from_string() {
        // Test various card string representations
        let king_hearts = Card::from_string("KH").unwrap();
        assert_eq!(king_hearts.rank, Rank::King);
        assert_eq!(king_hearts.suit, Suit::Hearts);

        let two_spades = Card::from_string("2S").unwrap();
        assert_eq!(two_spades.rank, Rank::Two);
        assert_eq!(two_spades.suit, Suit::Spades);

        let ten_diamonds = Card::from_string("TD").unwrap();
        assert_eq!(ten_diamonds.rank, Rank::Ten);
        assert_eq!(ten_diamonds.suit, Suit::Diamonds);

        // Test invalid strings
        assert!(Card::from_string("ZH").is_err()); // Invalid rank
        assert!(Card::from_string("KX").is_err()); // Invalid suit
        assert!(Card::from_string("K").is_err()); // Too short
    }

    #[test]
    fn test_suit_try_from() {
        // Test valid suits
        assert_eq!(Suit::try_from("D"), Ok(Suit::Diamonds));
        assert_eq!(Suit::try_from("C"), Ok(Suit::Clubs));
        assert_eq!(Suit::try_from("H"), Ok(Suit::Hearts));
        assert_eq!(Suit::try_from("S"), Ok(Suit::Spades));

        // Test invalid suits
        assert!(Suit::try_from("X").is_err());
        assert!(Suit::try_from("").is_err());
        assert!(Suit::try_from("DD").is_err());
    }

    #[test]
    fn test_suit_display() {
        assert_eq!(Suit::Diamonds.to_string(), "D");
        assert_eq!(Suit::Clubs.to_string(), "C");
        assert_eq!(Suit::Hearts.to_string(), "H");
        assert_eq!(Suit::Spades.to_string(), "S");
    }

    #[test]
    fn test_rank_try_from() {
        // Test valid ranks
        assert_eq!(Rank::try_from("3"), Ok(Rank::Three));
        assert_eq!(Rank::try_from("4"), Ok(Rank::Four));
        assert_eq!(Rank::try_from("5"), Ok(Rank::Five));
        assert_eq!(Rank::try_from("6"), Ok(Rank::Six));
        assert_eq!(Rank::try_from("7"), Ok(Rank::Seven));
        assert_eq!(Rank::try_from("8"), Ok(Rank::Eight));
        assert_eq!(Rank::try_from("9"), Ok(Rank::Nine));
        assert_eq!(Rank::try_from("T"), Ok(Rank::Ten));
        assert_eq!(Rank::try_from("J"), Ok(Rank::Jack));
        assert_eq!(Rank::try_from("Q"), Ok(Rank::Queen));
        assert_eq!(Rank::try_from("K"), Ok(Rank::King));
        assert_eq!(Rank::try_from("A"), Ok(Rank::Ace));
        assert_eq!(Rank::try_from("2"), Ok(Rank::Two));

        // Test invalid ranks
        assert!(Rank::try_from("1").is_err());
        assert!(Rank::try_from("0").is_err());
        assert!(Rank::try_from("X").is_err());
        assert!(Rank::try_from("").is_err());
        assert!(Rank::try_from("TT").is_err());
    }

    #[test]
    fn test_rank_display() {
        assert_eq!(Rank::Three.to_string(), "3");
        assert_eq!(Rank::Four.to_string(), "4");
        assert_eq!(Rank::Five.to_string(), "5");
        assert_eq!(Rank::Six.to_string(), "6");
        assert_eq!(Rank::Seven.to_string(), "7");
        assert_eq!(Rank::Eight.to_string(), "8");
        assert_eq!(Rank::Nine.to_string(), "9");
        assert_eq!(Rank::Ten.to_string(), "T");
        assert_eq!(Rank::Jack.to_string(), "J");
        assert_eq!(Rank::Queen.to_string(), "Q");
        assert_eq!(Rank::King.to_string(), "K");
        assert_eq!(Rank::Ace.to_string(), "A");
        assert_eq!(Rank::Two.to_string(), "2");
    }

    #[test]
    fn test_card_display() {
        let king_hearts = Card::new(Rank::King, Suit::Hearts);
        assert_eq!(king_hearts.to_string(), "KH");

        let two_spades = Card::new(Rank::Two, Suit::Spades);
        assert_eq!(two_spades.to_string(), "2S");

        let ten_diamonds = Card::new(Rank::Ten, Suit::Diamonds);
        assert_eq!(ten_diamonds.to_string(), "TD");

        let ace_clubs = Card::new(Rank::Ace, Suit::Clubs);
        assert_eq!(ace_clubs.to_string(), "AC");
    }

    #[test]
    fn test_card_from_string_edge_cases() {
        // Test empty string
        assert!(Card::from_string("").is_err());

        // Test single character
        assert!(Card::from_string("K").is_err());

        // Test three characters
        assert!(Card::from_string("KHS").is_err());

        // Test valid cards with all combinations
        for rank in Rank::iter() {
            for suit in Suit::iter() {
                let card = Card::new(rank, suit);
                let card_str = card.to_string();
                let parsed_card = Card::from_string(&card_str).unwrap();
                assert_eq!(card, parsed_card);
            }
        }
    }
}
