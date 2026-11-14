//! Math functions with BASIC error checks.
//!
//! QB64's `func_exp_single`. The `_FLOAT` variants live in `qb-float80`.

/// `EXP` for SINGLE/DOUBLE arguments: `Err(6)` above 88.02969.
pub fn exp_single(x: f64) -> Result<f64, i32> {
    if x <= 88.02969 {
        Ok(x.exp())
    } else {
        Err(6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_functions() {
        assert_eq!(exp_single(88.1), Err(6));
        assert_eq!(exp_single(0.0), Ok(1.0));
    }
}
