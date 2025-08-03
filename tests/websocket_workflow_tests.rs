use bigtwo::{
    event::RoomEvent,
    game::{Card, Rank, Suit},
    websockets::MessageType,
};

mod utils;

use utils::*;

#[tokio::test]
async fn test_game_start_requires_host_and_four_players() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;

    setup.send_start_game("alice").await; // alice is host

    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::GameStarted)
        .await
        .verify_game_started_with_four_players();
}

#[tokio::test]
async fn test_non_host_cannot_start_game() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;

    setup.send_start_game("bob").await; // bob is not host

    MessageAssertion::for_all_players(&setup)
        .received_no_messages()
        .await;
}

#[tokio::test]
async fn test_insufficient_players_cannot_start_game() {
    let setup = TestSetupBuilder::new().with_two_players().build().await;

    setup.send_start_game("alice").await;

    MessageAssertion::for_all_players(&setup)
        .received_no_messages()
        .await;
}

#[tokio::test]
async fn test_valid_first_move_with_three_of_diamonds() {
    let setup = TestSetupBuilder::new().with_two_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_two_player_game()
        .build_with_setup(&setup)
        .await;

    setup.send_move(&first_player, vec!["3D"]).await;

    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::MovePlayed)
        .await
        .with_player(&first_player)
        .with_cards(vec!["3D"]);
}

#[tokio::test]
async fn test_pass_move_after_initial_play() {
    let setup = TestSetupBuilder::new().with_two_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_two_player_game()
        .build_with_setup(&setup)
        .await;

    // First player plays 3D
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Second player passes
    let second_player = if first_player == "alice" {
        "bob"
    } else {
        "alice"
    };
    setup.send_pass(second_player).await;

    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::MovePlayed)
        .await
        .with_player(second_player)
        .with_cards(vec![]); // Empty for pass
}

#[tokio::test]
async fn test_wrong_turn_player_cannot_move() {
    let setup = TestSetupBuilder::new().with_two_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_two_player_game()
        .build_with_setup(&setup)
        .await;

    let wrong_player = if first_player == "alice" {
        "bob"
    } else {
        "alice"
    };
    setup.send_move(wrong_player, vec!["4H"]).await;

    MessageAssertion::for_all_players(&setup)
        .received_no_messages()
        .await;
}

#[tokio::test]
async fn test_cannot_beat_single_card_with_pair() {
    let setup = TestSetupBuilder::new().with_two_players().build().await;
    let first_player = GameBuilder::new()
        .with_pair_scenario()
        .build_with_setup(&setup)
        .await;

    // First player plays 3D
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Second player tries to beat single with pair (invalid)
    let second_player = if first_player == "alice" {
        "bob"
    } else {
        "alice"
    };
    setup.send_move(second_player, vec!["4H", "4S"]).await;

    MessageAssertion::for_all_players(&setup)
        .received_no_messages()
        .await;
}

#[tokio::test]
async fn test_player_join_event_notifies_existing_players() {
    let setup = TestSetupBuilder::new().with_two_players().build().await;

    setup
        .emit_event(RoomEvent::PlayerJoined {
            player: "charlie".to_string(),
        })
        .await;

    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::PlayersList)
        .await;
}

#[tokio::test]
async fn test_first_player_can_play_anything_after_all_others_pass() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_cards(vec![
            (
                "alice",
                vec![
                    Card::new(Rank::Three, Suit::Diamonds), // Alice has 3D, goes first
                    Card::new(Rank::Four, Suit::Hearts),
                    Card::new(Rank::Four, Suit::Spades), // Alice has a pair of 4s
                    Card::new(Rank::Seven, Suit::Clubs),
                    Card::new(Rank::Seven, Suit::Hearts), // Alice has a pair of 7s
                ],
            ),
            (
                "bob",
                vec![
                    Card::new(Rank::Five, Suit::Clubs),
                    Card::new(Rank::Six, Suit::Diamonds),
                    Card::new(Rank::Eight, Suit::Hearts),
                ],
            ),
            (
                "charlie",
                vec![
                    Card::new(Rank::Nine, Suit::Spades),
                    Card::new(Rank::Ten, Suit::Clubs),
                    Card::new(Rank::Jack, Suit::Diamonds),
                ],
            ),
            (
                "david",
                vec![
                    Card::new(Rank::Queen, Suit::Hearts),
                    Card::new(Rank::King, Suit::Spades),
                    Card::new(Rank::Ace, Suit::Clubs),
                ],
            ),
        ])
        .build_with_setup(&setup)
        .await;

    // First move: Alice plays 3D (required first move)
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Bob passes
    setup.send_pass("bob").await;
    setup.clear_messages().await;

    // Charlie passes
    setup.send_pass("charlie").await;
    setup.clear_messages().await;

    // David passes
    setup.send_pass("david").await;
    setup.clear_messages().await;

    // Now Alice has control and can play anything - let's play a pair of 4s
    setup.send_move("alice", vec!["4H", "4S"]).await;

    // All players should receive the move
    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::MovePlayed)
        .await
        .with_player("alice")
        .with_cards(vec!["4H", "4S"]);
}

#[tokio::test]
async fn test_first_player_can_change_combination_type_after_all_pass() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_cards(vec![
            (
                "alice",
                vec![
                    Card::new(Rank::Three, Suit::Diamonds), // Alice has 3D, goes first
                    Card::new(Rank::Five, Suit::Hearts),
                    Card::new(Rank::Eight, Suit::Spades),
                    Card::new(Rank::Jack, Suit::Clubs),
                    Card::new(Rank::Jack, Suit::Hearts), // Alice has a pair of Jacks
                ],
            ),
            (
                "bob",
                vec![
                    Card::new(Rank::Four, Suit::Clubs),
                    Card::new(Rank::Six, Suit::Diamonds),
                    Card::new(Rank::Nine, Suit::Hearts),
                ],
            ),
            (
                "charlie",
                vec![
                    Card::new(Rank::Seven, Suit::Spades),
                    Card::new(Rank::Ten, Suit::Clubs),
                    Card::new(Rank::Queen, Suit::Diamonds),
                ],
            ),
            (
                "david",
                vec![
                    Card::new(Rank::King, Suit::Hearts),
                    Card::new(Rank::Ace, Suit::Spades),
                    Card::new(Rank::Two, Suit::Clubs),
                ],
            ),
        ])
        .build_with_setup(&setup)
        .await;

    // First move: Alice plays 3D (single card)
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Everyone else passes
    setup.send_pass("bob").await;
    setup.send_pass("charlie").await;
    setup.send_pass("david").await;
    setup.clear_messages().await;

    // Now Alice can play a different combination type - pair instead of single
    setup.send_move("alice", vec!["JC", "JH"]).await;

    // All players should receive the pair move
    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::MovePlayed)
        .await
        .with_player("alice")
        .with_cards(vec!["JC", "JH"]);
}
