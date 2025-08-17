#[cfg(test)]
mod tests {
    use super::basic::{Card, Rank, Suit};
    use super::hands::{Hand, HandError, FiveCardHand};
    use rstest::rstest;
    use strum::IntoEnumIterator;

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
    fn test_single_hand() {
        let card1 = Card::new(Rank::King, Suit::Hearts);
        let card2 = Card::new(Rank::Queen, Suit::Spades);

        let hand1 = Hand::from_cards(&vec![card1]).unwrap();
        let hand2 = Hand::from_cards(&vec![card2]).unwrap();

        assert!(hand1.can_beat(&hand2));

        // Test that we can't compare across types
        let pair = Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Three, Suit::Spades),
        ])
        .unwrap();

        assert!(!hand1.can_beat(&pair)); // Single cannot beat pair
        assert!(!pair.can_beat(&hand1)); // Pair cannot beat single
    }

    #[test]
    fn test_pair_hand() {
        let pair1 = Hand::from_cards(&vec![
            Card::new(Rank::King, Suit::Hearts),
            Card::new(Rank::King, Suit::Spades),
        ])
        .unwrap();

        let pair2 = Hand::from_cards(&vec![
            Card::new(Rank::Queen, Suit::Diamonds),
            Card::new(Rank::Queen, Suit::Clubs),
        ])
        .unwrap();

        assert!(pair1.can_beat(&pair2));

        // Test same rank pairs with different suits
        let pair3 = Hand::from_cards(&vec![
            Card::new(Rank::King, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
        ])
        .unwrap();

        assert!(pair1.can_beat(&pair3)); // Spades beats Clubs for the high card
    }

    #[test]
    fn test_triple_hand() {
        let triple1 = Hand::from_cards(&vec![
            Card::new(Rank::King, Suit::Hearts),
            Card::new(Rank::King, Suit::Spades),
            Card::new(Rank::King, Suit::Diamonds),
        ])
        .unwrap();

        let triple2 = Hand::from_cards(&vec![
            Card::new(Rank::Queen, Suit::Hearts),
            Card::new(Rank::Queen, Suit::Spades),
            Card::new(Rank::Queen, Suit::Diamonds),
        ])
        .unwrap();

        assert!(triple1.can_beat(&triple2));
    }

    #[test]
    fn test_construct_invalid_straight() {
        let straight = Hand::from_cards(&vec![
            Card::new(Rank::Jack, Suit::Hearts),
            Card::new(Rank::Queen, Suit::Spades),
            Card::new(Rank::King, Suit::Diamonds),
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Two, Suit::Hearts),
        ]);

        assert!(straight.is_err());
        assert_eq!(straight.err(), Some(HandError::InvalidHandType));
    }

    #[test]
    fn test_construct_valid_straight_with_ace() {
        let straight = Hand::from_cards(&vec![
            Card::new(Rank::Ace, Suit::Hearts),
            Card::new(Rank::Two, Suit::Spades),
            Card::new(Rank::Three, Suit::Diamonds),
            Card::new(Rank::Four, Suit::Clubs),
            Card::new(Rank::Five, Suit::Hearts),
        ]);

        assert!(straight.is_ok());
    }

    #[test]
    fn test_construct_valid_ace_high_straight() {
        let straight = Hand::from_cards(&vec![
            Card::new(Rank::Ten, Suit::Hearts),
            Card::new(Rank::Jack, Suit::Spades),
            Card::new(Rank::Queen, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);

        assert!(straight.is_ok());
        match straight.unwrap() {
            Hand::Five(FiveCardHand::Straight(_)) => (),
            _ => panic!("Expected straight hand"),
        }
    }

    #[test]
    fn test_straight() {
        let straight = Hand::from_cards(&vec![
            Card::new(Rank::Six, Suit::Clubs),
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Five, Suit::Diamonds),
            Card::new(Rank::Four, Suit::Spades),
            Card::new(Rank::Seven, Suit::Hearts),
        ])
        .unwrap();

        match straight {
            Hand::Five(five) => match five {
                FiveCardHand::Straight(_) => (),
                _ => panic!("Expected straight hand"),
            },
            _ => panic!("Expected five card hand"),
        }
    }

    #[rstest]
    #[case(vec![
        Card::new(Rank::King, Suit::Hearts),
        Card::new(Rank::Ace, Suit::Spades),
        Card::new(Rank::Two, Suit::Diamonds),
        Card::new(Rank::Three, Suit::Clubs),
        Card::new(Rank::Four, Suit::Hearts),
    ])] // K-A-2-3-4 wraparound
    #[case(vec![
        Card::new(Rank::Queen, Suit::Hearts),
        Card::new(Rank::King, Suit::Spades),
        Card::new(Rank::Ace, Suit::Diamonds),
        Card::new(Rank::Two, Suit::Clubs),
        Card::new(Rank::Three, Suit::Hearts),
    ])] // Q-K-A-2-3 wraparound
    #[case(vec![
        Card::new(Rank::Ace, Suit::Hearts),
        Card::new(Rank::Two, Suit::Spades),
        Card::new(Rank::Three, Suit::Diamonds),
        Card::new(Rank::Four, Suit::Clubs),
        Card::new(Rank::Six, Suit::Hearts),
    ])] // A-2-3-4-6 (not consecutive)
    #[case(vec![
        Card::new(Rank::Two, Suit::Hearts),
        Card::new(Rank::Three, Suit::Spades),
        Card::new(Rank::Four, Suit::Diamonds),
        Card::new(Rank::Five, Suit::Clubs),
        Card::new(Rank::Seven, Suit::Hearts),
    ])] // 2-3-4-5-7 (not consecutive)
    fn test_invalid_straights(#[case] cards: Vec<Card>) {
        let result = Hand::from_cards(&cards);
        assert!(result.is_err());
        assert_eq!(result.err(), Some(HandError::InvalidHandType));
    }

    #[rstest]
    #[case(vec![
        Card::new(Rank::Three, Suit::Hearts),
        Card::new(Rank::Four, Suit::Spades),
        Card::new(Rank::Five, Suit::Diamonds),
        Card::new(Rank::Six, Suit::Clubs),
        Card::new(Rank::Seven, Suit::Hearts),
    ])] // 3-4-5-6-7
    #[case(vec![
        Card::new(Rank::Seven, Suit::Hearts),
        Card::new(Rank::Eight, Suit::Spades),
        Card::new(Rank::Nine, Suit::Diamonds),
        Card::new(Rank::Ten, Suit::Clubs),
        Card::new(Rank::Jack, Suit::Hearts),
    ])] // 7-8-9-10-J
    #[case(vec![
        Card::new(Rank::Nine, Suit::Hearts),
        Card::new(Rank::Ten, Suit::Spades),
        Card::new(Rank::Jack, Suit::Diamonds),
        Card::new(Rank::Queen, Suit::Clubs),
        Card::new(Rank::King, Suit::Hearts),
    ])] // 9-10-J-Q-K
    fn test_valid_normal_straights(#[case] cards: Vec<Card>) {
        let result = Hand::from_cards(&cards);
        assert!(result.is_ok());
        match result.unwrap() {
            Hand::Five(FiveCardHand::Straight(_)) => (),
            _ => panic!("Expected straight hand"),
        }
    }

    #[rstest]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Four, Suit::Spades),
            Card::new(Rank::Five, Suit::Diamonds),
            Card::new(Rank::Six, Suit::Clubs),
            Card::new(Rank::Seven, Suit::Hearts),
        ])
        .unwrap(), true)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Five, Suit::Hearts),
            Card::new(Rank::Six, Suit::Spades),
            Card::new(Rank::Seven, Suit::Diamonds),
            Card::new(Rank::Eight, Suit::Clubs),
            Card::new(Rank::Nine, Suit::Diamonds),
        ])
        .unwrap(), true)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Five, Suit::Hearts),
            Card::new(Rank::Six, Suit::Spades),
            Card::new(Rank::Seven, Suit::Diamonds),
            Card::new(Rank::Eight, Suit::Clubs),
            Card::new(Rank::Nine, Suit::Spades),
        ])
        .unwrap(), false)]
    fn test_straight_comparison_with_straight(#[case] other_hand: Hand, #[case] expected: bool) {
        let straight = Hand::from_cards(&vec![
            Card::new(Rank::Five, Suit::Hearts),
            Card::new(Rank::Six, Suit::Spades),
            Card::new(Rank::Seven, Suit::Diamonds),
            Card::new(Rank::Eight, Suit::Clubs),
            Card::new(Rank::Nine, Suit::Hearts),
        ])
        .unwrap();

        assert_eq!(straight.can_beat(&other_hand), expected);
    }

    #[rstest]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Five, Suit::Diamonds),
            Card::new(Rank::Six, Suit::Diamonds),
            Card::new(Rank::Seven, Suit::Diamonds),
            Card::new(Rank::Eight, Suit::Diamonds),
            Card::new(Rank::King, Suit::Diamonds),
        ])
        .unwrap(), true)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Five, Suit::Spades),
            Card::new(Rank::Six, Suit::Spades),
            Card::new(Rank::Seven, Suit::Spades),
            Card::new(Rank::Eight, Suit::Spades),
            Card::new(Rank::Jack, Suit::Spades),
        ])
        .unwrap(), false)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Five, Suit::Hearts),
            Card::new(Rank::Six, Suit::Hearts),
            Card::new(Rank::Seven, Suit::Hearts),
            Card::new(Rank::Eight, Suit::Hearts),
            Card::new(Rank::King, Suit::Hearts),
        ])
        .unwrap(), false)]
    fn test_flush_comparison_with_flush(#[case] other_hand: Hand, #[case] expected: bool) {
        let flush = Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Six, Suit::Hearts),
            Card::new(Rank::Seven, Suit::Hearts),
            Card::new(Rank::Eight, Suit::Hearts),
            Card::new(Rank::Jack, Suit::Hearts),
        ])
        .unwrap();

        assert_eq!(flush.can_beat(&other_hand), expected);
    }

    #[rstest]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Four, Suit::Spades),
            Card::new(Rank::Five, Suit::Diamonds),
            Card::new(Rank::Six, Suit::Clubs),
            Card::new(Rank::Seven, Suit::Hearts),
        ])
        .unwrap(), true)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Five, Suit::Hearts),
            Card::new(Rank::Seven, Suit::Hearts),
            Card::new(Rank::Nine, Suit::Hearts),
            Card::new(Rank::Jack, Suit::Hearts),
        ])
        .unwrap(), true)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Three, Suit::Spades),
            Card::new(Rank::Three, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
            Card::new(Rank::King, Suit::Hearts),
        ])
        .unwrap(), true)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Two, Suit::Hearts),
            Card::new(Rank::Two, Suit::Spades),
            Card::new(Rank::Two, Suit::Diamonds),
            Card::new(Rank::Two, Suit::Clubs),
            Card::new(Rank::King, Suit::Hearts),
        ])
        .unwrap(), false)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Four, Suit::Hearts),
            Card::new(Rank::Five, Suit::Hearts),
            Card::new(Rank::Six, Suit::Hearts),
            Card::new(Rank::Seven, Suit::Hearts),
        ])
        .unwrap(), false)]
    fn test_four_of_a_kind_comparison(#[case] other_hand: Hand, #[case] expected: bool) {
        let four_of_a_kind = Hand::from_cards(&vec![
            Card::new(Rank::Ace, Suit::Hearts),
            Card::new(Rank::Ace, Suit::Spades),
            Card::new(Rank::Ace, Suit::Diamonds),
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::King, Suit::Hearts),
        ])
        .unwrap();

        assert_eq!(four_of_a_kind.can_beat(&other_hand), expected);
    }

    #[test]
    fn test_full_house() {
        let full_house = Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Three, Suit::Spades),
            Card::new(Rank::Three, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
            Card::new(Rank::King, Suit::Hearts),
        ])
        .unwrap();

        match full_house {
            Hand::Five(five) => match five {
                FiveCardHand::FullHouse(_) => (),
                _ => panic!("Expected full house"),
            },
            _ => panic!("Expected five card hand"),
        }
    }

    #[rstest]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Three, Suit::Hearts),
            Card::new(Rank::Three, Suit::Spades),
            Card::new(Rank::Three, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
            Card::new(Rank::King, Suit::Hearts),
        ])
        .unwrap(), true)]
    #[case(Hand::from_cards(&vec![
            Card::new(Rank::Queen, Suit::Hearts),
            Card::new(Rank::Queen, Suit::Spades),
            Card::new(Rank::Queen, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
            Card::new(Rank::King, Suit::Hearts),
        ])
        .unwrap(), false)]
    fn test_full_house_comparison(#[case] other_hand: Hand, #[case] expected: bool) {
        let full_house = Hand::from_cards(&vec![
            Card::new(Rank::Five, Suit::Hearts),
            Card::new(Rank::Five, Suit::Spades),
            Card::new(Rank::Five, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
            Card::new(Rank::King, Suit::Diamonds),
        ])
        .unwrap();

        assert_eq!(full_house.can_beat(&other_hand), expected);
    }

    #[rstest]
    #[case(vec![], "Pass")]
    #[case(vec![Card::new(Rank::King, Suit::Hearts)], "Single")]
    #[case(vec![
        Card::new(Rank::King, Suit::Hearts),
        Card::new(Rank::King, Suit::Spades),
    ], "Pair")]
    #[case(vec![
        Card::new(Rank::King, Suit::Hearts),
        Card::new(Rank::King, Suit::Spades),
        Card::new(Rank::King, Suit::Diamonds),
    ], "Triple")]
    #[case(vec![
        Card::new(Rank::Three, Suit::Hearts),
        Card::new(Rank::Four, Suit::Spades),
        Card::new(Rank::Five, Suit::Diamonds),
        Card::new(Rank::Six, Suit::Clubs),
        Card::new(Rank::Seven, Suit::Hearts),
    ], "Straight")]
    #[case(vec![
        Card::new(Rank::Three, Suit::Hearts),
        Card::new(Rank::Five, Suit::Hearts),
        Card::new(Rank::Seven, Suit::Hearts),
        Card::new(Rank::Nine, Suit::Hearts),
        Card::new(Rank::Jack, Suit::Hearts),
    ], "Flush")]
    #[case(vec![
        Card::new(Rank::Three, Suit::Hearts),
        Card::new(Rank::Three, Suit::Spades),
        Card::new(Rank::Three, Suit::Diamonds),
        Card::new(Rank::King, Suit::Clubs),
        Card::new(Rank::King, Suit::Hearts),
    ], "Full House")]
    #[case(vec![
        Card::new(Rank::Three, Suit::Hearts),
        Card::new(Rank::Three, Suit::Spades),
        Card::new(Rank::Three, Suit::Diamonds),
        Card::new(Rank::Three, Suit::Clubs),
        Card::new(Rank::King, Suit::Hearts),
    ], "Four of a Kind")]
    #[case(vec![
        Card::new(Rank::Three, Suit::Hearts),
        Card::new(Rank::Four, Suit::Hearts),
        Card::new(Rank::Five, Suit::Hearts),
        Card::new(Rank::Six, Suit::Hearts),
        Card::new(Rank::Seven, Suit::Hearts),
    ], "Straight Flush")]
    fn test_hand_type_names(#[case] cards: Vec<Card>, #[case] expected_name: &str) {
        let hand = Hand::from_cards(&cards).unwrap();
        assert_eq!(hand.hand_type_name(), expected_name);
    }

    #[test]
    fn test_invalid_hands() {
        // Invalid pair (different ranks)
        let result = Hand::from_cards(&vec![
            Card::new(Rank::King, Suit::Hearts),
            Card::new(Rank::Queen, Suit::Spades),
        ]);
        assert!(result.is_err());

        // Invalid hand size
        let result = Hand::from_cards(&vec![
            Card::new(Rank::King, Suit::Hearts),
            Card::new(Rank::King, Suit::Spades),
            Card::new(Rank::King, Suit::Diamonds),
            Card::new(Rank::King, Suit::Clubs),
        ]);
        assert!(result.is_err());
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

    #[rstest]
    #[case(vec![], vec![], true)] // For technicality, pass 'beats' pass
    #[case(vec![], vec![Card::from_string("3D").unwrap()], true)] // Pass doesn't beat 3D
    #[case(vec![Card::from_string("KH").unwrap()], vec![Card::from_string("QS").unwrap()], true)] // QS doesn't beat KH
    #[case(vec![Card::from_string("QS").unwrap()], vec![Card::from_string("KH").unwrap()], false)] // KH beats QS
    #[case(vec![Card::from_string("KH").unwrap()], vec![Card::from_string("KH").unwrap()], false)] // Same card doesn't beat itself
    #[case(vec![Card::from_string("KH").unwrap(), Card::from_string("KS").unwrap()], vec![Card::from_string("AH").unwrap(), Card::from_string("AS").unwrap()], false)] // AH-AS doesn't beat KH-KS
    fn test_compare_played_cards(
        #[case] played_cards: Vec<Card>,
        #[case] current_cards: Vec<Card>,
        #[case] expected: bool,
    ) {
        let result = compare_played_cards(&played_cards, &current_cards);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);
    }
}