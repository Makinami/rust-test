use std::ops::{Add, AddAssign, Sub, SubAssign};

use rust_decimal::Decimal;
use serde::Deserialize;

/// New Type wrapper around a monetary amount (up to 4 decimal places).
///
/// The underlying `Decimal` is kept private; use `From<Decimal>` impl to construct one.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Amount(Decimal);

impl Default for Amount {
    fn default() -> Self {
        Self(Decimal::ZERO)
    }
}

impl From<Decimal> for Amount {
    fn from(value: Decimal) -> Self {
        Self::new(value)
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
