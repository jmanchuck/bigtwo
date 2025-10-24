use bigtwo::event::RoomEvent;

mod utils;
use utils::{GameBuilder, TestSetupBuilder};

#[tokio::test]
async fn stats_service_records_game_when_game_won_event_emitted() {
    let setup = TestSetupBuilder::new().with_four_players().build().await;

    let first_player = GameBuilder::new()
        .with_simple_four_player_game()
        .build_with_setup(&setup)
        .await;

    setup
        .event_bus
        .emit_to_room(
            "room-123",
            RoomEvent::GameWon {
                winner: first_player.clone(),
                winning_hand: vec![],
            },
        )
        .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let stats = setup
        .stats_repository
        .get_room_stats("room-123")
        .await
        .expect("stats retrieval should succeed")
        .expect("room stats should exist after game win");

    assert_eq!(stats.games_played, 1);
    assert!(stats.player_stats.contains_key(&first_player));
}

