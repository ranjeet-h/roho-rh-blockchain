//! Block reward calculation
//! 
//! Deterministic reward function that ensures total supply never exceeds 90M RH
//! (public issuance) and naturally approaches zero by year 2200.

use crate::constants::PUBLIC_ISSUANCE;

/// Decay factor for reward calculation
/// This value is chosen so that:
/// - Rewards decrease smoothly over time
/// - Total issuance approaches but never exceeds 90M RH
/// - Issuance effectively halts by year 2200
const DECAY_RATE: u64 = 1_000_000; // Per-block decay denominator

/// Initial reward per block (in base units)
/// At genesis, reward = remaining * decay_rate
const INITIAL_REWARD_NUMERATOR: u64 = 50; // 50 RH per block initially

/// Calculate block reward for a given height
/// 
/// This is a pure, deterministic function.
/// 
/// Formula: reward = (remaining_supply * INITIAL_REWARD_NUMERATOR) / DECAY_RATE
/// 
/// # Arguments
/// * `height` - Current block height
/// * `total_issued_so_far` - Total RH issued through mining up to previous block
/// 
/// # Returns
/// Reward in base units (satoshi-equivalent)
pub fn calculate_block_reward(height: u64, total_issued_so_far: u64) -> u64 {
    // Genesis block has no mining reward (founder allocation is separate)
    if height == 0 {
        return 0;
    }
    
    // Calculate remaining supply
    let remaining = PUBLIC_ISSUANCE.saturating_sub(total_issued_so_far);
    
    // If nothing remains, reward is 0
    if remaining == 0 {
        return 0;
    }
    
    // Calculate reward as fraction of remaining
    // This creates asymptotic decay
    let reward = (remaining as u128 * INITIAL_REWARD_NUMERATOR as u128) / DECAY_RATE as u128;
    
    // Ensure we don't exceed remaining supply
    let reward = reward.min(remaining as u128) as u64;
    
    // Minimum reward is 1 satoshi (until supply exhausted)
    if reward == 0 && remaining > 0 {
        1
    } else {
        reward
    }
}

/// Calculate total issued supply after a given number of blocks
/// 
/// This function simulates the reward schedule to verify supply constraints.
/// Used for testing and verification only.
pub fn calculate_total_issued(num_blocks: u64) -> u64 {
    let mut total_issued: u64 = 0;
    
    for height in 1..=num_blocks {
        let reward = calculate_block_reward(height, total_issued);
        total_issued = total_issued.saturating_add(reward);
        
        // Early exit if we've reached the limit
        if total_issued >= PUBLIC_ISSUANCE {
            return PUBLIC_ISSUANCE;
        }
    }
    
    total_issued
}

/// Verify that the reward schedule never exceeds total supply
/// 
/// INVARIANT: At no point can total_issued > PUBLIC_ISSUANCE
pub fn verify_supply_invariant(up_to_height: u64) -> bool {
    let mut total_issued: u64 = 0;
    
    for height in 1..=up_to_height {
        let reward = calculate_block_reward(height, total_issued);
        total_issued = total_issued.saturating_add(reward);
        
        if total_issued > PUBLIC_ISSUANCE {
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_has_no_reward() {
        assert_eq!(calculate_block_reward(0, 0), 0);
    }

    #[test]
    fn test_first_block_has_reward() {
        let reward = calculate_block_reward(1, 0);
        assert!(reward > 0);
    }

    #[test]
    fn test_rewards_decrease_over_time() {
        let reward1 = calculate_block_reward(1, 0);
        let reward2 = calculate_block_reward(2, reward1);
        
        // Rewards should decrease (or stay same for very small decreases)
        assert!(reward2 <= reward1);
    }

    #[test]
    fn test_supply_never_exceeded() {
        // Test for first million blocks
        assert!(verify_supply_invariant(1_000_000));
    }

    #[test]
    fn test_reward_approaches_zero() {
        // After significant issuance, rewards should be tiny
        let nearly_all_issued = PUBLIC_ISSUANCE - 1000;
        let reward = calculate_block_reward(1_000_000, nearly_all_issued);
        
        // Should be very small
        assert!(reward < 1000);
    }

    #[test]
    fn test_minimum_reward_is_one_satoshi() {
        // When remaining is tiny, reward should be at least 1
        let nearly_all_issued = PUBLIC_ISSUANCE - 1;
        let reward = calculate_block_reward(1_000_000, nearly_all_issued);
        
        assert!(reward >= 1);
    }

    #[test]
    fn test_no_reward_when_fully_issued() {
        let reward = calculate_block_reward(1_000_000, PUBLIC_ISSUANCE);
        assert_eq!(reward, 0);
    }
}
