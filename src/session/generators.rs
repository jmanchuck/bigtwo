use async_trait::async_trait;

/// Trait for generating usernames
#[async_trait]
pub trait UsernameGenerator: Send + Sync {
    async fn generate(&self) -> String;
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
