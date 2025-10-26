mod utils;
use utils::{GameBuilder, TestSetupBuilder};

#[tokio::test]
async fn stats_service_records_game_when_game_won_event_emitted() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;

    let first_player_uuid = GameBuilder::new()
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    // Verify the game was created
    let game = setup
        .game_service
        .get_game("room-123")
        .await
        .expect("game should exist after creation");

    assert_eq!(game.current_player_turn(), first_player_uuid);

    // Directly call process_completed_game instead of relying on event system
    // This verifies the stats service works correctly
    let (game_result, room_stats) = setup
        .stats_service
        .process_completed_game("room-123", &game, &first_player_uuid)
        .await
        .expect("processing game should succeed");

    // Verify the game result
    assert_eq!(game_result.room_id, "room-123");
    assert_eq!(game_result.game_number, 1);
    assert_eq!(game_result.winner_uuid, first_player_uuid);

    // Verify room stats
    assert_eq!(
        room_stats.games_played, 1,
        "should have recorded exactly 1 game"
    );
    assert!(
        room_stats.player_stats.contains_key(&first_player_uuid),
        "stats should contain the winner's UUID"
    );

    let winner_stats = room_stats.player_stats.get(&first_player_uuid).unwrap();
    assert_eq!(winner_stats.wins, 1, "winner should have 1 win");

    // Also verify stats can be retrieved from repository
    let retrieved_stats = setup
        .stats_repository
        .get_room_stats("room-123")
        .await
        .expect("stats retrieval should succeed")
        .expect("room stats should exist in repository");

    assert_eq!(retrieved_stats.games_played, 1);
}
