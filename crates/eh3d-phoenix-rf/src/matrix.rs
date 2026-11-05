// SPDX-License-Identifier: GPL-3.0-or-later
//! A square matrix of complex numbers, for the parameters of networks of any number of ports.

use crate::complex::Complex;
use crate::error::RfError;

/// A square complex matrix stored by rows.
#[derive(Clone, Debug, PartialEq)]
pub struct CMatrix {
    n: usize,
    data: Vec<Complex>,
}

impl CMatrix {
    /// A matrix of `n` rows and columns from its entries by rows. `n` must be above zero, the
    /// number of entries `n * n` and every entry finite.
    pub fn new(n: usize, data: Vec<Complex>) -> Result<CMatrix, RfError> {
        if n == 0 || n.checked_mul(n) != Some(data.len()) {
            return Err(RfError::InvalidNetwork {
                reason: format!("a matrix of {n} ports needs {n} x {n} entries"),
            });
        }
        if data.iter().any(|z| !z.is_finite()) {
            return Err(RfError::InvalidNetwork {
                reason: "an entry is not finite".to_string(),
            });
        }
        Ok(CMatrix { n, data })
    }

    /// The identity matrix of `n` rows (at least one).
    pub fn identity(n: usize) -> CMatrix {
        let n = n.max(1);
        let mut data = vec![Complex::ZERO; n * n];
        for k in 0..n {
            data[k * n + k] = Complex::ONE;
        }
        CMatrix { n, data }
    }

    /// The number of rows (and of columns).
    pub fn size(&self) -> usize {
        self.n
    }

    /// The entries by rows.
    pub fn data(&self) -> &[Complex] {
        &self.data
    }

    /// The entry at a row and a column, or zero outside the matrix.
    pub fn get(&self, row: usize, col: usize) -> Complex {
        if row >= self.n || col >= self.n {
            return Complex::ZERO;
        }
        self.data
            .get(row * self.n + col)
            .copied()
            .unwrap_or(Complex::ZERO)
    }

    fn same_size(&self, other: &CMatrix) -> Result<(), RfError> {
        if self.n == other.n {
            Ok(())
        } else {
            Err(RfError::InvalidNetwork {
                reason: format!("matrices of {} and {} ports", self.n, other.n),
            })
        }
    }

    /// The sum of two matrices of the same size.
    pub fn plus(&self, other: &CMatrix) -> Result<CMatrix, RfError> {
        self.same_size(other)?;
        let data = self
            .data
            .iter()
            .zip(&other.data)
            .map(|(a, b)| *a + *b)
            .collect();
        CMatrix::new(self.n, data)
    }

    /// The difference of two matrices of the same size.
    pub fn minus(&self, other: &CMatrix) -> Result<CMatrix, RfError> {
        self.same_size(other)?;
        let data = self
            .data
            .iter()
            .zip(&other.data)
            .map(|(a, b)| *a - *b)
            .collect();
        CMatrix::new(self.n, data)
    }

    /// The product `self * other` of two matrices of the same size.
    pub fn times(&self, other: &CMatrix) -> Result<CMatrix, RfError> {
        self.same_size(other)?;
        let n = self.n;
        let mut data = Vec::with_capacity(n * n);
        for row in 0..n {
            for col in 0..n {
                let mut sum = Complex::ZERO;
                for k in 0..n {
                    sum = sum + self.get(row, k) * other.get(k, col);
                }
                data.push(sum);
            }
        }
        CMatrix::new(n, data)
    }

    /// The matrix with every entry multiplied by a complex factor.
    pub fn scaled(&self, factor: Complex) -> Result<CMatrix, RfError> {
        let data = self.data.iter().map(|z| *z * factor).collect();
        CMatrix::new(self.n, data)
    }

    /// The inverse, by Gauss-Jordan elimination with row pivoting, or [`RfError::Singular`].
    pub fn inverse(&self) -> Result<CMatrix, RfError> {
        let n = self.n;
        let mut a = self.data.clone();
        let mut inv = CMatrix::identity(n).data;
        for col in 0..n {
            let mut pivot = col;
            let mut best = a.get(col * n + col).map_or(0.0, |z| z.abs());
            for row in (col + 1)..n {
                let magnitude = a.get(row * n + col).map_or(0.0, |z| z.abs());
                if magnitude > best {
                    best = magnitude;
                    pivot = row;
                }
            }
            if best == 0.0 || !best.is_finite() {
                return Err(RfError::Singular {
                    what: "the matrix has no inverse".to_string(),
                });
            }
            if pivot != col {
                for k in 0..n {
                    a.swap(pivot * n + k, col * n + k);
                    inv.swap(pivot * n + k, col * n + k);
                }
            }
            let p = a[col * n + col];
            for k in 0..n {
                a[col * n + k] = a[col * n + k].checked_div(p)?;
                inv[col * n + k] = inv[col * n + k].checked_div(p)?;
            }
            for row in 0..n {
                if row == col {
                    continue;
                }
                let factor = a[row * n + col];
                if factor == Complex::ZERO {
                    continue;
                }
                for k in 0..n {
                    let da = factor * a[col * n + k];
                    let di = factor * inv[col * n + k];
                    a[row * n + k] = a[row * n + k] - da;
                    inv[row * n + k] = inv[row * n + k] - di;
                }
            }
        }
        CMatrix::new(n, inv).map_err(|_| RfError::Singular {
            what: "the inverse is not finite".to_string(),
        })
    }

    /// The largest absolute difference between the entries of two matrices of the same size
    /// (infinity when the sizes differ).
    pub fn max_difference(&self, other: &CMatrix) -> f64 {
        if self.n != other.n {
            return f64::INFINITY;
        }
        self.data
            .iter()
            .zip(&other.data)
            .map(|(a, b)| (*a - *b).abs())
            .fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(re: f64, im: f64) -> Complex {
        Complex::new(re, im)
    }

    fn sample() -> CMatrix {
        CMatrix::new(
            3,
            vec![
                c(2.0, 1.0),
                c(0.0, -1.0),
                c(1.0, 0.0),
                c(1.0, 1.0),
                c(3.0, 0.0),
                c(0.0, 2.0),
                c(0.5, 0.0),
                c(-1.0, 1.0),
                c(4.0, -2.0),
            ],
        )
        .unwrap()
    }

    #[test]
    fn the_inverse_times_the_matrix_is_the_identity() {
        let m = sample();
        let product = m.times(&m.inverse().unwrap()).unwrap();
        assert!(product.max_difference(&CMatrix::identity(3)) < 1e-12);
        let other_way = m.inverse().unwrap().times(&m).unwrap();
        assert!(other_way.max_difference(&CMatrix::identity(3)) < 1e-12);
    }

    #[test]
    fn a_singular_matrix_has_no_inverse() {
        let m = CMatrix::new(2, vec![c(1.0, 0.0), c(2.0, 0.0), c(2.0, 0.0), c(4.0, 0.0)]).unwrap();
        assert!(matches!(m.inverse(), Err(RfError::Singular { .. })));
        let zero = CMatrix::new(1, vec![Complex::ZERO]).unwrap();
        assert!(matches!(zero.inverse(), Err(RfError::Singular { .. })));
    }

    #[test]
    fn a_pivot_in_another_row_is_found() {
        let m = CMatrix::new(
            2,
            vec![Complex::ZERO, c(1.0, 0.0), c(1.0, 0.0), Complex::ZERO],
        )
        .unwrap();
        let inv = m.inverse().unwrap();
        assert!(inv.max_difference(&m) < 1e-15);
    }

    #[test]
    fn sums_differences_and_scaling() {
        let m = sample();
        let i = CMatrix::identity(3);
        let sum = m.plus(&i).unwrap();
        assert_eq!(sum.get(0, 0), c(3.0, 1.0));
        assert_eq!(sum.get(0, 1), m.get(0, 1));
        assert!(sum.minus(&i).unwrap().max_difference(&m) < 1e-15);
        let twice = m.scaled(c(2.0, 0.0)).unwrap();
        assert_eq!(twice.get(2, 2), c(8.0, -4.0));
        assert_eq!(m.get(5, 0), Complex::ZERO);
        assert_eq!((m.size(), m.data().len()), (3, 9));
    }

    #[test]
    fn wrong_shapes_and_non_finite_entries_are_refused() {
        assert!(CMatrix::new(0, vec![]).is_err());
        assert!(CMatrix::new(2, vec![Complex::ONE; 3]).is_err());
        assert!(CMatrix::new(1, vec![c(f64::NAN, 0.0)]).is_err());
        let a = CMatrix::identity(2);
        let b = CMatrix::identity(3);
        assert!(a.plus(&b).is_err() && a.minus(&b).is_err() && a.times(&b).is_err());
        assert_eq!(a.max_difference(&b), f64::INFINITY);
    }
}
