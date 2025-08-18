use std::{collections::HashMap, sync::Arc};
use crate::user::PlayerMappingService;

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