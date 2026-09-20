use std::fmt;

use rust_decimal::Decimal;
use serde::Serialize;
use thiserror::Error;

/// New Type wrapper around representing a balance
// NOTE: The currently underlying `Decimal` allows for values approximately between -7.9e28 and 7.9e28.
// Should this not be sufficient, we can consider using a different numeric type (e.g. u128/u256).
// or a custom implementation to handle larger ranges.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct Balance(Decimal);

impl fmt::Display for Balance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Balance {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl Balance {
    pub const ZERO: Self = Self(Decimal::ZERO);
}

// Mainly for convenience when dealing with integer literals.
#[cfg(test)]
impl From<i32> for Balance {
    fn from(value: i32) -> Self {
        Self(Decimal::from(value))
    }
}

impl Balance {
    /// Returns the underlying decimal value, e.g. for persistence.
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }

    /// Safely subtracts the given balance from this balance, returning an error if the result would be negative.
    pub fn checked_sub(self, rhs: impl Into<Decimal>) -> Result<Self, BalanceError> {
        Ok(Self(
            self.0
                .checked_sub(rhs.into())
                .ok_or(BalanceError::Underflow)?,
        ))
    }

    /// Safely adds the given balance to this balance, returning an error if the result would wrap around.
    pub fn checked_add(self, rhs: impl Into<Decimal>) -> Result<Self, BalanceError> {
        Ok(Self(
            self.0
                .checked_add(rhs.into())
                .ok_or(BalanceError::Overflow)?,
        ))
    }
}

#[derive(Debug, Error)]
pub enum BalanceError {
    #[error("Balance underflowed")]
    Underflow,
    #[error("Balance overflowed")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_add_succeeds() {
        let a = Balance::from(1i32);
        let b = Balance::from(2i32);
        assert_eq!(a.checked_add(b.as_decimal()).unwrap(), Balance::from(3i32));
    }

    #[test]
    fn checked_add_overflows() {
        let max = Balance(Decimal::MAX);
        let one = Balance::from(1i32);
        assert!(matches!(
            max.checked_add(one.as_decimal()),
            Err(BalanceError::Overflow)
        ));
    }

    #[test]
    fn checked_sub_succeeds() {
        let a = Balance::from(5i32);
        let b = Balance::from(2i32);
        assert_eq!(a.checked_sub(b.as_decimal()).unwrap(), Balance::from(3i32));
    }

    #[test]
    fn checked_sub_underflows() {
        let a = Balance(Decimal::MIN);
        let b = Balance::from(1i32);
        assert!(matches!(
            a.checked_sub(b.as_decimal()),
            Err(BalanceError::Underflow)
        ));
    }
}
