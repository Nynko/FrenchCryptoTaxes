use std::ops::{Add, Div, Mul, Sub};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub struct SimpleDecimal {
    pub value: u64,
    pub decimal_offset: u8
}

impl SimpleDecimal {
    pub fn new(value: u64, decimal_offset: u8) -> Self {
        SimpleDecimal { value, decimal_offset }
    }

    pub fn from_str(str : &str) -> Self{
        let vec_str : Vec<&str> = str.trim().split('.').collect();
        let number_str = vec_str[0];
        let decimal_str = vec_str[1];

        let decimal = decimal_str.len();

        return  SimpleDecimal { , vec_str[1].len() }
    }
}

impl Add for SimpleDecimal {
    type Output = SimpleDecimal;

    fn add(self, other: SimpleDecimal) -> SimpleDecimal {
        // Align decimal offsets
        if self.decimal_offset > other.decimal_offset {
            let diff = self.decimal_offset - other.decimal_offset;
            let adjusted_other_value = other.value * 10_u64.pow(diff as u32);
            SimpleDecimal {
                value: self.value + adjusted_other_value,
                decimal_offset: self.decimal_offset,
            }
        } else if self.decimal_offset < other.decimal_offset {
            let diff = other.decimal_offset - self.decimal_offset;
            let adjusted_self_value = self.value * 10_u64.pow(diff as u32);
            SimpleDecimal {
                value: adjusted_self_value + other.value,
                decimal_offset: other.decimal_offset,
            }
        } else {
            SimpleDecimal {
                value: self.value + other.value,
                decimal_offset: self.decimal_offset,
            }
        }
    }
}


impl Add<u64> for SimpleDecimal {
    type Output = SimpleDecimal;

    fn add(self, other: u64) -> SimpleDecimal {
        // Convert u64 to SimpleDecimal with the same decimal offset as self
        let other_value = other * 10_u64.pow(self.decimal_offset as u32);
        SimpleDecimal {
            value: self.value + other_value,
            decimal_offset: self.decimal_offset,
        }
    }
}

impl Add<SimpleDecimal> for u64 {
    type Output = SimpleDecimal;

    fn add(self, other: SimpleDecimal) -> SimpleDecimal {
        other + self // Reuse the Add implementation in SimpleDecimal
    }
}


impl Sub for SimpleDecimal {
    type Output = SimpleDecimal;

    fn sub(self, other: SimpleDecimal) -> SimpleDecimal {
        // Align decimal offsets
        if self.decimal_offset > other.decimal_offset {
            let diff = self.decimal_offset - other.decimal_offset;
            let adjusted_other_value = other.value * 10_u64.pow(diff as u32);
            SimpleDecimal {
                value: self.value - adjusted_other_value,
                decimal_offset: self.decimal_offset,
            }
        } else if self.decimal_offset < other.decimal_offset {
            let diff = other.decimal_offset - self.decimal_offset;
            let adjusted_self_value = self.value * 10_u64.pow(diff as u32);
            SimpleDecimal {
                value: adjusted_self_value - other.value,
                decimal_offset: other.decimal_offset,
            }
        } else {
            SimpleDecimal {
                value: self.value - other.value,
                decimal_offset: self.decimal_offset,
            }
        }
    }
}

impl Sub<u64> for SimpleDecimal {
    type Output = SimpleDecimal;

    fn sub(self, other: u64) -> SimpleDecimal {
        // Convert u64 to SimpleDecimal with the same decimal offset as self
        let other_value = other * 10_u64.pow(self.decimal_offset as u32);
        SimpleDecimal {
            value: self.value - other_value,
            decimal_offset: self.decimal_offset,
        }
    }
}


impl Sub<SimpleDecimal> for u64 {
    type Output = SimpleDecimal;

    fn sub(self, other: SimpleDecimal) -> SimpleDecimal {
        // Convert self (u64) to SimpleDecimal with the same decimal offset as other
        let self_value = self * 10_u64.pow(other.decimal_offset as u32);
        SimpleDecimal {
            value: self_value - other.value,
            decimal_offset: other.decimal_offset,
        }
    }
}



impl Mul for SimpleDecimal {
    type Output = SimpleDecimal;

    fn mul(self, other: SimpleDecimal) -> SimpleDecimal {
        // Multiply values directly and sum offsets
        SimpleDecimal {
            value: self.value * other.value,
            decimal_offset: self.decimal_offset + other.decimal_offset,
        }
    }
}




impl Mul<u64> for SimpleDecimal {
    type Output = SimpleDecimal;

    fn mul(self, other: u64) -> SimpleDecimal {
        // Multiply the value directly and keep the same decimal offset
        SimpleDecimal {
            value: self.value * other,
            decimal_offset: self.decimal_offset,
        }
    }
}




impl Mul<SimpleDecimal> for u64 {
    type Output = SimpleDecimal;

    fn mul(self, other: SimpleDecimal) -> SimpleDecimal {
        other * self // Reuse the Mul implementation in SimpleDecimal
    }
}


// Implementing Division for SimpleDecimal
impl Div for SimpleDecimal {
    type Output = SimpleDecimal;

    fn div(self, other: SimpleDecimal) -> SimpleDecimal {
        // Determine the difference in decimal offsets
        let offset_diff = self.decimal_offset as i8 - other.decimal_offset as i8;

        if offset_diff > 0 {
            // Self has a higher decimal offset
            let adjusted_other_value = other.value * 10_u64.pow(offset_diff as u32);
            SimpleDecimal {
                value: self.value / adjusted_other_value,
                decimal_offset: self.decimal_offset,
            }
        } else if offset_diff < 0 {
            // Other has a higher decimal offset
            let adjusted_self_value = self.value * 10_u64.pow((-offset_diff) as u32);
            SimpleDecimal {
                value: adjusted_self_value / other.value,
                decimal_offset: other.decimal_offset,
            }
        } else {
            // Decimal offsets are equal
            SimpleDecimal {
                value: self.value / other.value,
                decimal_offset: self.decimal_offset,
            }
        }
    }
}