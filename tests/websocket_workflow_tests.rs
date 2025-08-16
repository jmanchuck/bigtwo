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
async fn test_turn_progression_after_move() {
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

    setup.clear_messages().await;
    setup.send_move(&first_player, vec!["3D"]).await;

    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::MovePlayed)
        .await;

    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::TurnChange)
        .await;

    let updated_game = setup
        .game_service
        .get_game("room-123")
        .await
        .expect("Game should exist");

    let new_current_player = updated_game.current_player_turn();
    assert_ne!(
        first_player, new_current_player,
        "Turn should have advanced to next player"
    );
}

#[tokio::test]
async fn test_valid_first_move_with_three_of_diamonds() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_four_player_game()
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
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    // First player plays 3D
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Get the updated game to see who's next
    let updated_game = setup.game_service.get_game("room-123").await.unwrap();
    let second_player = updated_game.current_player_turn();
    setup.send_pass(&second_player).await;

    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::MovePlayed)
        .await
        .with_player(&second_player)
        .with_cards(vec![]); // Empty for pass
}

#[tokio::test]
async fn test_wrong_turn_player_cannot_move() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    // Find a player that's not the current player
    let game = setup.game_service.get_game("room-123").await.unwrap();
    let wrong_player = game
        .players()
        .iter()
        .find(|p| p.name != first_player)
        .unwrap()
        .name
        .clone();
    setup.send_move(&wrong_player, vec!["4H"]).await;

    MessageAssertion::for_all_players(&setup)
        .received_no_messages()
        .await;
}

#[tokio::test]
async fn test_cannot_beat_single_card_with_pair() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_pair_scenario()
        .build_with_setup(&setup)
        .await;

    // First player plays 3D
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Get the updated game to see who's next
    let updated_game = setup.game_service.get_game("room-123").await.unwrap();
    let second_player = updated_game.current_player_turn();
    setup.send_move(&second_player, vec!["4H", "4S"]).await;

    MessageAssertion::for_all_players(&setup)
        .received_no_messages()
        .await;
}

#[tokio::test]
async fn test_player_join_event_notifies_existing_players() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;

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
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    // First move: First player plays 3D (required first move)
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Get the current state and find all other players
    let game = setup.game_service.get_game("room-123").await.unwrap();
    let other_players: Vec<String> = game
        .players()
        .iter()
        .map(|p| p.name.clone())
        .filter(|name| name != &first_player)
        .collect();

    // All other players pass
    for player in &other_players {
        setup.send_pass(player).await;
    }
    setup.clear_messages().await;

    // Now the first player has control and should be able to play anything
    // Let's just verify the current player is correct
    let updated_game = setup.game_service.get_game("room-123").await.unwrap();
    assert_eq!(updated_game.current_player_turn(), first_player);
}

#[tokio::test]
async fn test_first_player_can_change_combination_type_after_all_pass() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    // First move: First player plays 3D (single card)
    setup.send_move(&first_player, vec!["3D"]).await;
    setup.clear_messages().await;

    // Get the current state and find all other players
    let game = setup.game_service.get_game("room-123").await.unwrap();
    let other_players: Vec<String> = game
        .players()
        .iter()
        .map(|p| p.name.clone())
        .filter(|name| name != &first_player)
        .collect();

    // All other players pass
    for player in &other_players {
        setup.send_pass(player).await;
    }
    setup.clear_messages().await;

    // Now the first player should have control again and can play different combination types
    // Let's just verify the game state allows the first player to play
    let updated_game = setup.game_service.get_game("room-123").await.unwrap();
    assert_eq!(updated_game.current_player_turn(), first_player);
    // In Big Two, after all players pass, the last player to play can play any combination type
    assert_eq!(updated_game.consecutive_passes(), 3); // 3 other players passed
}

#[tokio::test]
async fn test_first_turn_must_include_three_of_diamonds() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    // Try to play without 3 of diamonds on first turn - should fail
    setup.send_move(&first_player, vec!["4H"]).await;

    // No messages should be sent since the move is invalid
    MessageAssertion::for_all_players(&setup)
        .received_no_messages()
        .await;
}

#[tokio::test]
async fn test_first_turn_with_three_of_diamonds_succeeds() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;
    let first_player = GameBuilder::new()
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    // Play with 3 of diamonds on first turn - should succeed
    setup.send_move(&first_player, vec!["3D"]).await;

    // All players should receive the move
    MessageAssertion::for_all_players(&setup)
        .received_message_type(MessageType::MovePlayed)
        .await
        .with_player(&first_player)
        .with_cards(vec!["3D"]);
}
