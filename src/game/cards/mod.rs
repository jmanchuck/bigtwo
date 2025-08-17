pub mod basic;
pub mod hands;

#[cfg(test)]
mod tests;

pub use basic::{Card, Rank, Suit};
pub use hands::{
    Hand, HandError, SingleHand, PairHand, TripleHand, FiveCardHand,
    StraightHand, FlushHand, FullHouseHand, FourOfAKindHand, StraightFlushHand,
    compare_played_cards,
};