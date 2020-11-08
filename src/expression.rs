use std::convert::TryFrom;
use std::ops::{BitOr, Mul, Neg, Shl};

#[derive(Debug)]
pub enum Expression {
    Known(i32),
    Unknown,
}

impl Expression {
    pub fn check_hram(self) -> Self {
        unimplemented!();
        self
    }
}

impl From<i32> for Expression {
    fn from(x: i32) -> Self {
        Self::Known(x)
    }
}

impl TryFrom<Expression> for i32 {
    type Error = crate::AssemblerError;

    fn try_from(expr: Expression) -> Result<Self, Self::Error> {
        match expr {
            Expression::Known(val) => Ok(val),
            Expression::Unknown => Err(Self::Error::ExprNotConstant),
        }
    }
}

impl BitOr for Expression {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        match self {
            Self::Known(val) => val | rhs,
            Self::Unknown => unimplemented!(),
        }
    }
}

impl BitOr<i32> for Expression {
    type Output = Self;
    fn bitor(self, rhs: i32) -> Self {
        match self {
            Self::Known(val) => Self::Known(val | rhs),
            Self::Unknown => unimplemented!(),
        }
    }
}

impl BitOr<Expression> for i32 {
    type Output = Expression;
    fn bitor(self, rhs: Expression) -> Expression {
        rhs | self
    }
}

impl Neg for Expression {
    type Output = Self;
    fn neg(self) -> Self {
        match self {
            Self::Known(val) => Self::Known(-val),
            Self::Unknown => unimplemented!(),
        }
    }
}

impl Shl for Expression {
    type Output = Self;
    fn shl(self, rhs: Self) -> Self {
        match rhs {
            Self::Known(val) => self << val,
            Self::Unknown => unimplemented!(),
        }
    }
}

impl Shl<i32> for Expression {
    type Output = Self;
    fn shl(self, rhs: i32) -> Self {
        match self {
            Self::Known(lhs) => Self::Known(lhs << rhs),
            Self::Unknown => unimplemented!(),
        }
    }
}
