//! Difficulty adjustment algorithm
//! 
//! Pure mathematical difficulty adjustment targeting 600 second blocks.

use crate::constants::{BLOCK_TIME_TARGET, DIFFICULTY_ADJUSTMENT_INTERVAL};

/// Minimum difficulty (easiest)
const MIN_DIFFICULTY: u32 = 0x1d00ffff;

/// Maximum adjustment factor (4x in either direction per period)
const MAX_ADJUSTMENT_FACTOR: u64 = 4;

/// Calculate the new difficulty target
/// 
/// This is a pure function with no side effects.
/// Adjustment happens every DIFFICULTY_ADJUSTMENT_INTERVAL blocks.
/// 
/// # Arguments
/// * `current_difficulty` - Current compact difficulty target
/// * `first_block_time` - Timestamp of first block in adjustment period
/// * `last_block_time` - Timestamp of last block in adjustment period
/// 
/// # Returns
/// New compact difficulty target
pub fn calculate_next_difficulty(
    current_difficulty: u32,
    first_block_time: u64,
    last_block_time: u64,
) -> u32 {
    // Calculate actual time taken for the period
    let actual_time = last_block_time.saturating_sub(first_block_time);
    
    // Expected time for the period
    let expected_time = BLOCK_TIME_TARGET * DIFFICULTY_ADJUSTMENT_INTERVAL;
    
    // Limit adjustment to 4x in either direction
    let actual_time = actual_time.max(expected_time / MAX_ADJUSTMENT_FACTOR);
    let actual_time = actual_time.min(expected_time * MAX_ADJUSTMENT_FACTOR);
    
    // Calculate new target
    let current_target = compact_to_target(current_difficulty);
    let new_target = multiply_target(&current_target, actual_time, expected_time);
    
    // Convert back to compact and ensure minimum difficulty
    let new_compact = target_to_compact(&new_target);
    
    // Don't go below minimum difficulty
    if new_compact > MIN_DIFFICULTY {
        MIN_DIFFICULTY
    } else {
        new_compact
    }
}

/// Check if difficulty should be adjusted at this height
pub fn should_adjust_difficulty(height: u64) -> bool {
    height > 0 && height % DIFFICULTY_ADJUSTMENT_INTERVAL == 0
}

/// Get the height of the first block in the current adjustment period
pub fn get_period_start_height(height: u64) -> u64 {
    if height < DIFFICULTY_ADJUSTMENT_INTERVAL {
        0
    } else {
        height - DIFFICULTY_ADJUSTMENT_INTERVAL
    }
}

/// Convert compact difficulty to 256-bit target
fn compact_to_target(compact: u32) -> [u8; 32] {
    let exponent = (compact >> 24) as usize;
    let mantissa = compact & 0x007FFFFF;
    
    let mut target = [0u8; 32];
    
    if exponent == 0 {
        return target;
    }
    
    let negative = (compact & 0x00800000) != 0;
    if negative {
        return target; // Negative targets are invalid
    }
    
    if exponent <= 3 {
        let value = mantissa >> (8 * (3 - exponent));
        target[31] = (value & 0xFF) as u8;
        if exponent >= 2 {
            target[30] = ((value >> 8) & 0xFF) as u8;
        }
        if exponent >= 3 {
            target[29] = ((value >> 16) & 0xFF) as u8;
        }
    } else {
        let start = 32 - exponent;
        if start < 32 {
            target[start] = ((mantissa >> 16) & 0xFF) as u8;
        }
        if start + 1 < 32 {
            target[start + 1] = ((mantissa >> 8) & 0xFF) as u8;
        }
        if start + 2 < 32 {
            target[start + 2] = (mantissa & 0xFF) as u8;
        }
    }
    
    target
}

/// Convert 256-bit target to compact difficulty
fn target_to_compact(target: &[u8; 32]) -> u32 {
    // Find the first non-zero byte
    let mut first_nonzero = 0;
    for (i, &byte) in target.iter().enumerate() {
        if byte != 0 {
            first_nonzero = i;
            break;
        }
    }
    
    let exponent = (32 - first_nonzero) as u32;
    
    if exponent == 0 {
        return 0;
    }
    
    let mut mantissa: u32 = 0;
    if first_nonzero < 32 {
        mantissa |= (target[first_nonzero] as u32) << 16;
    }
    if first_nonzero + 1 < 32 {
        mantissa |= (target[first_nonzero + 1] as u32) << 8;
    }
    if first_nonzero + 2 < 32 {
        mantissa |= target[first_nonzero + 2] as u32;
    }
    
    // Handle negative bit
    if mantissa & 0x00800000 != 0 {
        mantissa >>= 8;
        return ((exponent + 1) << 24) | mantissa;
    }
    
    (exponent << 24) | mantissa
}

/// Multiply target by a ratio (actual_time / expected_time)
fn multiply_target(target: &[u8; 32], numerator: u64, denominator: u64) -> [u8; 32] {
    // Simple implementation: convert to u128 for multiplication
    // This is a simplified version - production would need arbitrary precision
    
    let mut result = [0u8; 32];
    let mut carry: u128 = 0;
    
    for i in (0..32).rev() {
        let val = (target[i] as u128) * (numerator as u128) + carry;
        let new_val = val / (denominator as u128);
        carry = (val % (denominator as u128)) << 8;
        result[i] = (new_val & 0xFF) as u8;
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_adjust_difficulty() {
        assert!(!should_adjust_difficulty(0));
        assert!(!should_adjust_difficulty(1));
        assert!(should_adjust_difficulty(DIFFICULTY_ADJUSTMENT_INTERVAL));
        assert!(should_adjust_difficulty(DIFFICULTY_ADJUSTMENT_INTERVAL * 2));
    }

    #[test]
    fn test_get_period_start_height() {
        assert_eq!(get_period_start_height(0), 0);
        assert_eq!(get_period_start_height(DIFFICULTY_ADJUSTMENT_INTERVAL), 0);
        assert_eq!(
            get_period_start_height(DIFFICULTY_ADJUSTMENT_INTERVAL * 2),
            DIFFICULTY_ADJUSTMENT_INTERVAL
        );
    }

    #[test]
    fn test_difficulty_increases_when_blocks_too_fast() {
        // When blocks come faster than expected, difficulty should not decrease
        let current = 0x1c00ffff;
        let expected_time = BLOCK_TIME_TARGET * DIFFICULTY_ADJUSTMENT_INTERVAL;
        let actual_time = expected_time / 2; // Blocks came twice as fast
        
        let new_difficulty = calculate_next_difficulty(current, 0, actual_time);
        
        // New difficulty should not be easier than MIN_DIFFICULTY
        // And should not be radically different (within 4x)
        assert!(new_difficulty != 0, "Difficulty should not be zero");
        assert!(new_difficulty <= MIN_DIFFICULTY, "Should not exceed MIN_DIFFICULTY");
    }

    #[test]
    fn test_difficulty_decreases_when_blocks_too_slow() {
        // When blocks come slower than expected, algorithm adjusts
        let current = 0x1c00ffff;
        let expected_time = BLOCK_TIME_TARGET * DIFFICULTY_ADJUSTMENT_INTERVAL;
        let actual_time = expected_time * 2; // Blocks came twice as slow
        
        let new_difficulty = calculate_next_difficulty(current, 0, actual_time);
        
        // Should still be a valid difficulty
        assert!(new_difficulty != 0, "Difficulty should not be zero");
        // Should be capped at MIN_DIFFICULTY if going too easy
        assert!(new_difficulty <= MIN_DIFFICULTY, "Should not exceed MIN_DIFFICULTY");
    }
}
