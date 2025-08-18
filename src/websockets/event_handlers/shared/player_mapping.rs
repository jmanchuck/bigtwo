use crate::user::PlayerMappingService;
use std::{collections::HashMap, sync::Arc};

pub struct PlayerMappingUtils;

impl PlayerMappingUtils {
    pub async fn build_uuid_to_name_mapping(
        player_mapping: &Arc<dyn PlayerMappingService>,
        uuids: &[String],
    ) -> HashMap<String, String> {
        let mut mapping = HashMap::new();
        for uuid in uuids {
            if let Some(name) = player_mapping.get_playername(uuid).await {
                mapping.insert(uuid.clone(), name);
            }
        }
        mapping
    }

    pub async fn get_player_name(
        player_mapping: &Arc<dyn PlayerMappingService>,
        uuid: &str,
    ) -> Option<String> {
        player_mapping.get_playername(uuid).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::mapping_service::InMemoryPlayerMappingService;
    use crate::user::PlayerMappingService;

    #[tokio::test]
    async fn test_build_uuid_to_name_mapping() {
        let svc_concrete = Arc::new(InMemoryPlayerMappingService::new());
        let svc: Arc<dyn PlayerMappingService> = svc_concrete.clone();
        let u1 = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let u2 = "550e8400-e29b-41d4-a716-446655440001".to_string();
        svc_concrete
            .register_player(u1.clone(), "Alice".to_string())
            .await
            .unwrap();
        svc_concrete
            .register_player(u2.clone(), "Bob".to_string())
            .await
            .unwrap();

        let mapping =
            PlayerMappingUtils::build_uuid_to_name_mapping(&svc, &[u1.clone(), u2.clone()]).await;
        assert_eq!(mapping.get(&u1).unwrap(), "Alice");
        assert_eq!(mapping.get(&u2).unwrap(), "Bob");
    }

    #[tokio::test]
    async fn test_get_player_name() {
        let svc_concrete = Arc::new(InMemoryPlayerMappingService::new());
        let svc: Arc<dyn PlayerMappingService> = svc_concrete.clone();
        let u = "550e8400-e29b-41d4-a716-446655440000".to_string();
        svc_concrete
            .register_player(u.clone(), "Carol".to_string())
            .await
            .unwrap();

        let name = PlayerMappingUtils::get_player_name(&svc, &u).await;
        assert_eq!(name, Some("Carol".to_string()));

        let none = PlayerMappingUtils::get_player_name(&svc, "missing").await;
        assert!(none.is_none());
    }
}
