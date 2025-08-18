use async_trait::async_trait;
use uuid::Uuid;

/// Trait for generating unique identifiers
#[async_trait]
pub trait UuidGenerator: Send + Sync {
    async fn generate(&self) -> String;
}

/// Trait for generating usernames
#[async_trait]
pub trait UsernameGenerator: Send + Sync {
    async fn generate(&self) -> String;
}

/// Default UUID generator using UUID v4
pub struct DefaultUuidGenerator;

impl DefaultUuidGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultUuidGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UuidGenerator for DefaultUuidGenerator {
    async fn generate(&self) -> String {
        Uuid::new_v4().to_string()
    }
}

/// Pet name-based username generator
pub struct PetNameUsernameGenerator;

impl PetNameUsernameGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PetNameUsernameGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UsernameGenerator for PetNameUsernameGenerator {
    async fn generate(&self) -> String {
        petname::Petnames::default().generate_one(2, "-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_uuid_generator() {
        let generator = DefaultUuidGenerator::new();
        let uuid1 = generator.generate().await;
        let uuid2 = generator.generate().await;

        // Should generate valid UUIDs
        assert!(Uuid::parse_str(&uuid1).is_ok());
        assert!(Uuid::parse_str(&uuid2).is_ok());

        // Should be unique
        assert_ne!(uuid1, uuid2);
    }

    #[tokio::test]
    async fn test_petname_username_generator() {
        let generator = PetNameUsernameGenerator::new();
        let username1 = generator.generate().await;
        let username2 = generator.generate().await;

        // Should generate non-empty usernames with dashes
        assert!(!username1.is_empty());
        assert!(username1.contains('-'));
        assert!(!username2.is_empty());
        assert!(username2.contains('-'));

        // Should typically be unique (though not guaranteed)
        // Just verify they're properly formatted
        let parts1: Vec<&str> = username1.split('-').collect();
        let parts2: Vec<&str> = username2.split('-').collect();
        assert_eq!(parts1.len(), 2);
        assert_eq!(parts2.len(), 2);
    }
}
