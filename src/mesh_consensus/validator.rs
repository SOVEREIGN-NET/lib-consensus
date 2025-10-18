//! Mesh Validator Management
//!
//! Tracks validators participating in mesh consensus

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Status of a validator in the mesh
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorStatus {
    /// Active and participating in consensus
    Active,
    /// Temporarily offline but still registered
    Offline,
    /// Removed from validator set
    Removed,
    /// Slashed for Byzantine behavior
    Slashed,
}

/// Mesh validator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshValidator {
    /// Unique validator ID (node ID)
    pub validator_id: [u8; 32],
    
    /// Wallet address for rewards
    pub wallet_address: [u8; 32],
    
    /// Reputation score (0-100)
    pub reputation: u32,
    
    /// Number of blocks validated
    pub blocks_validated: u64,
    
    /// Number of blocks proposed
    pub blocks_proposed: u64,
    
    /// Current status
    pub status: ValidatorStatus,
    
    /// When this validator joined
    pub joined_at_height: u64,
    
    /// Last time validator was seen
    pub last_seen: u64,
    
    /// Voting power (based on reputation)
    pub voting_power: u32,
}

impl MeshValidator {
    /// Create new validator
    pub fn new(
        validator_id: [u8; 32],
        wallet_address: [u8; 32],
        joined_at_height: u64,
    ) -> Self {
        Self {
            validator_id,
            wallet_address,
            reputation: 50, // Start at 50/100
            blocks_validated: 0,
            blocks_proposed: 0,
            status: ValidatorStatus::Active,
            joined_at_height,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            voting_power: 1, // Default voting power
        }
    }
    
    /// Update last seen timestamp
    pub fn mark_online(&mut self) {
        self.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if self.status == ValidatorStatus::Offline {
            self.status = ValidatorStatus::Active;
        }
    }
    
    /// Increment blocks validated
    pub fn record_validation(&mut self) {
        self.blocks_validated += 1;
        self.mark_online();
        
        // Increase reputation (cap at 100)
        if self.reputation < 100 {
            self.reputation = (self.reputation + 1).min(100);
        }
    }
    
    /// Increment blocks proposed
    pub fn record_proposal(&mut self) {
        self.blocks_proposed += 1;
        self.mark_online();
    }
    
    /// Decrease reputation for bad behavior
    pub fn slash_reputation(&mut self, amount: u32) {
        self.reputation = self.reputation.saturating_sub(amount);
        
        // If reputation drops too low, mark as slashed
        if self.reputation < 10 {
            self.status = ValidatorStatus::Slashed;
        }
    }
    
    /// Check if validator is active and online
    pub fn is_active(&self) -> bool {
        self.status == ValidatorStatus::Active
    }
    
    /// Check if validator has been offline too long (5 minutes)
    pub fn is_offline(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - self.last_seen > 300 // 5 minutes
    }
}

/// Set of validators for a mesh
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSet {
    /// All validators (active and inactive)
    validators: HashMap<[u8; 32], MeshValidator>,
    
    /// Ordered list of active validator IDs (for round-robin)
    active_validators: Vec<[u8; 32]>,
    
    /// Current proposer index
    proposer_index: usize,
}

impl ValidatorSet {
    /// Create new validator set
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            active_validators: Vec::new(),
            proposer_index: 0,
        }
    }
    
    /// Add a validator to the set
    pub fn add_validator(&mut self, validator: MeshValidator) {
        let validator_id = validator.validator_id;
        self.validators.insert(validator_id, validator);
        
        // Add to active list if active
        if self.validators[&validator_id].is_active() {
            self.active_validators.push(validator_id);
        }
    }
    
    /// Remove a validator
    pub fn remove_validator(&mut self, validator_id: &[u8; 32]) {
        if let Some(validator) = self.validators.get_mut(validator_id) {
            validator.status = ValidatorStatus::Removed;
        }
        
        self.active_validators.retain(|id| id != validator_id);
    }
    
    /// Get current proposer for this round
    pub fn get_proposer(&self) -> Option<[u8; 32]> {
        if self.active_validators.is_empty() {
            return None;
        }
        
        Some(self.active_validators[self.proposer_index % self.active_validators.len()])
    }
    
    /// Rotate to next proposer
    pub fn rotate_proposer(&mut self) {
        self.proposer_index = (self.proposer_index + 1) % self.active_validators.len().max(1);
    }
    
    /// Get total number of active validators
    pub fn active_count(&self) -> usize {
        self.active_validators.len()
    }
    
    /// Get validator by ID
    pub fn get_validator(&self, validator_id: &[u8; 32]) -> Option<&MeshValidator> {
        self.validators.get(validator_id)
    }
    
    /// Get mutable validator by ID
    pub fn get_validator_mut(&mut self, validator_id: &[u8; 32]) -> Option<&mut MeshValidator> {
        self.validators.get_mut(validator_id)
    }
    
    /// Update validator statuses based on online/offline
    pub fn update_statuses(&mut self) {
        let mut offline_validators = Vec::new();
        
        for (id, validator) in self.validators.iter_mut() {
            if validator.is_offline() && validator.status == ValidatorStatus::Active {
                validator.status = ValidatorStatus::Offline;
                offline_validators.push(*id);
            }
        }
        
        // Remove offline validators from active list
        for id in offline_validators {
            self.active_validators.retain(|v| v != &id);
        }
    }
    
    /// Get all active validators
    pub fn get_active_validators(&self) -> Vec<&MeshValidator> {
        self.active_validators
            .iter()
            .filter_map(|id| self.validators.get(id))
            .collect()
    }
}

impl Default for ValidatorSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validator_creation() {
        let validator = MeshValidator::new([1u8; 32], [2u8; 32], 0);
        
        assert_eq!(validator.reputation, 50);
        assert_eq!(validator.blocks_validated, 0);
        assert_eq!(validator.status, ValidatorStatus::Active);
        assert!(validator.is_active());
    }
    
    #[test]
    fn test_validator_reputation() {
        let mut validator = MeshValidator::new([1u8; 32], [2u8; 32], 0);
        
        // Record validations increase reputation
        for _ in 0..10 {
            validator.record_validation();
        }
        assert_eq!(validator.reputation, 60);
        assert_eq!(validator.blocks_validated, 10);
        
        // Slash reputation
        validator.slash_reputation(20);
        assert_eq!(validator.reputation, 40);
        
        // Severe slash triggers slashed status
        validator.slash_reputation(35);
        assert_eq!(validator.status, ValidatorStatus::Slashed);
    }
    
    #[test]
    fn test_validator_set() {
        let mut set = ValidatorSet::new();
        
        let v1 = MeshValidator::new([1u8; 32], [2u8; 32], 0);
        let v2 = MeshValidator::new([3u8; 32], [4u8; 32], 0);
        let v3 = MeshValidator::new([5u8; 32], [6u8; 32], 0);
        
        set.add_validator(v1);
        set.add_validator(v2);
        set.add_validator(v3);
        
        assert_eq!(set.active_count(), 3);
        
        // Test proposer rotation
        let p1 = set.get_proposer().unwrap();
        set.rotate_proposer();
        let p2 = set.get_proposer().unwrap();
        set.rotate_proposer();
        let p3 = set.get_proposer().unwrap();
        set.rotate_proposer();
        let p4 = set.get_proposer().unwrap();
        
        // Should cycle back to first proposer
        assert_eq!(p1, p4);
        assert_ne!(p1, p2);
        assert_ne!(p2, p3);
    }
}
