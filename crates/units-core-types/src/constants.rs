use crate::id::UnitsObjectId;

/// Hardcoded system controller IDs for bootstrap and security
/// Simple hardcoded values for initial implementation simplicity
pub const SYSTEM_LOADER_ID: UnitsObjectId = UnitsObjectId::new([0; 32]);
pub const TOKEN_CONTROLLER_ID: UnitsObjectId = UnitsObjectId::new([1; 32]);
pub const ACCOUNT_CONTROLLER_ID: UnitsObjectId = UnitsObjectId::new([2; 32]);
pub const MODULE_MANAGER_ID: UnitsObjectId = UnitsObjectId::new([3; 32]);

/// Validate that an object ID is a system controller
pub fn is_system_controller(id: &UnitsObjectId) -> bool {
    *id == SYSTEM_LOADER_ID
        || *id == TOKEN_CONTROLLER_ID
        || *id == ACCOUNT_CONTROLLER_ID
        || *id == MODULE_MANAGER_ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_controller_validation() {
        // Test system controllers are recognized
        assert!(is_system_controller(&SYSTEM_LOADER_ID));
        assert!(is_system_controller(&TOKEN_CONTROLLER_ID));
        assert!(is_system_controller(&ACCOUNT_CONTROLLER_ID));
        assert!(is_system_controller(&MODULE_MANAGER_ID));

        // Test random ID is not a system controller
        let random_id = UnitsObjectId::new([99; 32]);
        assert!(!is_system_controller(&random_id));
    }

    #[test]
    fn test_system_controller_uniqueness() {
        // Ensure all system controller IDs are unique
        let ids = [
            SYSTEM_LOADER_ID,
            TOKEN_CONTROLLER_ID,
            ACCOUNT_CONTROLLER_ID,
            MODULE_MANAGER_ID,
        ];

        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                assert_ne!(ids[i], ids[j], "System controller IDs must be unique");
            }
        }
    }
}