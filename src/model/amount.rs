use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// New Type wrapper around a monetary amount (up to 4 decimal places).
// NOTE: The currently underlying `Decimal` allows for values approximately between -7.9e28 and 7.9e28.
// Because we need 4 decimal places of precision, usable range is actually between -7.9e24 and 7.9e24.
// Should this not be sufficient, we can consider using a different numeric type (e.g. u128/u256).
// or a custom implementation to handle larger ranges.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct Amount(Decimal);

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize via a string rather than `Decimal`'s own `Deserialize` impl: some formats
        // (e.g. `csv`) use `deserialize_any` and infer a numeric type from the raw field text,
        // which fails for plain-integer fields wider than `u64` (e.g. `Decimal::MAX`).
        let s = String::deserialize(deserializer)?;
        let value = Decimal::from_str(&s).map_err(serde::de::Error::custom)?;

        // let max_safe_value = Decimal::MAX / Decimal::new(10000, 0);
        // if value > max_safe_value {
        //     return Err(serde::de::Error::custom(AmountError::Overflow));
        // }
        Amount::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Amount {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl Amount {
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Returns the underlying decimal value, e.g. for persistence.
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }

    /// Safely subtracts the given amount from this amount, returning an error if the result would be negative.
    pub fn checked_sub(self, rhs: Self) -> Result<Self, AmountError> {
        if self.0 >= rhs.0 {
            Ok(Self(self.0 - rhs.0))
        } else {
            Err(AmountError::Negative)
        }
    }

    /// Safely adds the given amount to this amount, returning an error if the result would wrap around.
    pub fn checked_add(self, rhs: Self) -> Result<Self, AmountError> {
        Ok(Self(
            self.0.checked_add(rhs.0).ok_or(AmountError::Overflow)?,
        ))
    }
}

// Mainly for convenience when dealing with integer literals.
#[cfg(test)]
impl From<u32> for Amount {
    fn from(value: u32) -> Self {
        Self(Decimal::from(value))
    }
}

impl TryFrom<Decimal> for Amount {
    type Error = AmountError;

    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        if value < Decimal::ZERO {
            return Err(AmountError::Negative);
        }
        Ok(Self(value.round_dp(4)))
    }
}

#[derive(Debug, Error)]
pub enum AmountError {
    #[error("Amount cannot be negative")]
    Negative,
    #[error("Amount overflowed")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Deserializes a single CSV field into an `Amount`, mirroring how the real CSV records are parsed.
    fn parse_amount(field: &str) -> Result<Amount, csv::Error> {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(field.as_bytes());
        reader.deserialize().next().unwrap()
    }

    #[test]
    fn deserializes_valid_amount() {
        let amount = parse_amount("12.3456").unwrap();
        assert_eq!(amount, Amount::try_from(Decimal::new(123456, 4)).unwrap());
    }

    #[test]
    fn rejects_negative_amount_on_deserialize() {
        assert!(parse_amount("-1.0").is_err());
    }

    // #[test]
    // fn rejects_amount_exceeding_decimal_range_on_deserialize() {
    //     // This value exceeds the maximum representable decimal value by an order of magnitude.
    //     assert!(parse_amount("79228162514264337593543950335").is_err());
    // }

    #[test]
    fn checked_add_succeeds() {
        let a = Amount::from(1u32);
        let b = Amount::from(2u32);
        assert_eq!(a.checked_add(b).unwrap(), Amount::from(3u32));
    }

    #[test]
    fn checked_add_overflows() {
        let max = Amount::try_from(Decimal::MAX).unwrap();
        let one = Amount::from(1u32);
        assert!(matches!(max.checked_add(one), Err(AmountError::Overflow)));
    }

    #[test]
    fn checked_sub_succeeds() {
        let a = Amount::from(5u32);
        let b = Amount::from(2u32);
        assert_eq!(a.checked_sub(b).unwrap(), Amount::from(3u32));
    }

    #[test]
    fn checked_sub_rejects_negative_result() {
        let a = Amount::from(1u32);
        let b = Amount::from(2u32);
        assert!(matches!(a.checked_sub(b), Err(AmountError::Negative)));
    }
}
