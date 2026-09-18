use std::ops::{Add, AddAssign, Sub, SubAssign};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// New Type wrapper around a monetary amount (up to 4 decimal places).
// NOTE: The currently underlying `Decimal` allows for values approximately between -7e24 and 7e24.
// Should this not be sufficient, we can consider using a different numeric type (e.g. u128/u256)
// or a custom implementation to handle larger ranges.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Amount(Decimal);

impl Default for Amount {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl From<Decimal> for Amount {
    fn from(value: Decimal) -> Self {
        Self(value.round_dp(4))
    }
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
