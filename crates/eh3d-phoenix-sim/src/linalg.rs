// SPDX-License-Identifier: GPL-3.0-or-later
//! Dense linear systems by Gaussian elimination with partial pivoting, real and complex. A pivot
//! that is zero, not finite or smaller than a fraction of the largest value of its column means
//! the matrix is singular: the error is the index of the column where the elimination stopped,
//! which is the unknown the equations leave undetermined.

use eh3d_phoenix_rf::Complex;

/// Solves `a x = b` in place for a real matrix of `n` rows stored by rows. On success `b` holds
/// the solution.
pub fn solve_real(a: &mut [f64], b: &mut [f64], n: usize, tolerance: f64) -> Result<(), usize> {
    debug_assert!(a.len() == n * n && b.len() == n);
    let mut scale = vec![0.0f64; n];
    for row in a.chunks(n.max(1)) {
        for (column, value) in row.iter().enumerate() {
            scale[column] = scale[column].max(value.abs());
        }
    }
    for k in 0..n {
        let mut pivot_row = k;
        let mut best = a[k * n + k].abs();
        for i in (k + 1)..n {
            let candidate = a[i * n + k].abs();
            if candidate > best {
                best = candidate;
                pivot_row = i;
            }
        }
        if !best.is_finite() || best <= tolerance * scale[k] || best == 0.0 {
            return Err(k);
        }
        if pivot_row != k {
            for j in k..n {
                a.swap(k * n + j, pivot_row * n + j);
            }
            b.swap(k, pivot_row);
        }
        let pivot = a[k * n + k];
        for i in (k + 1)..n {
            let factor = a[i * n + k] / pivot;
            if factor != 0.0 {
                for j in k..n {
                    a[i * n + j] -= factor * a[k * n + j];
                }
                b[i] -= factor * b[k];
            }
        }
    }
    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum -= a[i * n + j] * b[j];
        }
        b[i] = sum / a[i * n + i];
    }
    Ok(())
}

/// Solves `a x = b` in place for a complex matrix; see [`solve_real`].
pub fn solve_complex(
    a: &mut [Complex],
    b: &mut [Complex],
    n: usize,
    tolerance: f64,
) -> Result<(), usize> {
    debug_assert!(a.len() == n * n && b.len() == n);
    let mut scale = vec![0.0f64; n];
    for row in a.chunks(n.max(1)) {
        for (column, value) in row.iter().enumerate() {
            scale[column] = scale[column].max(value.abs());
        }
    }
    for k in 0..n {
        let mut pivot_row = k;
        let mut best = a[k * n + k].abs();
        for i in (k + 1)..n {
            let candidate = a[i * n + k].abs();
            if candidate > best {
                best = candidate;
                pivot_row = i;
            }
        }
        if !best.is_finite() || best <= tolerance * scale[k] || best == 0.0 {
            return Err(k);
        }
        if pivot_row != k {
            for j in k..n {
                a.swap(k * n + j, pivot_row * n + j);
            }
            b.swap(k, pivot_row);
        }
        let pivot = a[k * n + k];
        for i in (k + 1)..n {
            let entry = a[i * n + k];
            if entry.re == 0.0 && entry.im == 0.0 {
                continue;
            }
            let factor = entry / pivot;
            for j in k..n {
                let delta = factor * a[k * n + j];
                a[i * n + j] = a[i * n + j] - delta;
            }
            let delta = factor * b[k];
            b[i] = b[i] - delta;
        }
    }
    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum = sum - a[i * n + j] * b[j];
        }
        b[i] = sum / a[i * n + i];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_system_is_solved() {
        // 2x + y = 5 ; x + 3y = 10 -> x = 1, y = 3
        let mut a = [2.0, 1.0, 1.0, 3.0];
        let mut b = [5.0, 10.0];
        solve_real(&mut a, &mut b, 2, 1e-12).unwrap();
        assert!((b[0] - 1.0).abs() < 1e-12 && (b[1] - 3.0).abs() < 1e-12);
    }

    #[test]
    fn a_zero_on_the_diagonal_is_handled_by_the_pivot_search() {
        // y = 2 ; x + y = 5
        let mut a = [0.0, 1.0, 1.0, 1.0];
        let mut b = [2.0, 5.0];
        solve_real(&mut a, &mut b, 2, 1e-12).unwrap();
        assert!((b[0] - 3.0).abs() < 1e-12 && (b[1] - 2.0).abs() < 1e-12);
    }

    #[test]
    fn a_singular_matrix_names_the_column() {
        let mut a = [1.0, 2.0, 2.0, 4.0];
        let mut b = [1.0, 2.0];
        assert_eq!(solve_real(&mut a, &mut b, 2, 1e-12), Err(1));
        let mut zero = [0.0, 0.0, 0.0, 1.0];
        let mut c = [0.0, 1.0];
        assert_eq!(solve_real(&mut zero, &mut c, 2, 1e-12), Err(0));
        let mut nan = [f64::NAN, 0.0, 0.0, 1.0];
        let mut d = [0.0, 1.0];
        assert_eq!(solve_real(&mut nan, &mut d, 2, 1e-12), Err(0));
    }

    #[test]
    fn the_solution_satisfies_the_equations_for_a_larger_system() {
        let n = 12;
        let mut a = vec![0.0; n * n];
        let mut b = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                // a diagonally dominant matrix with a fixed pattern
                let v = ((i * 7 + j * 3) % 11) as f64 - 5.0;
                a[i * n + j] = if i == j { 40.0 + v } else { v };
            }
            b[i] = i as f64 + 1.0;
        }
        let (a0, b0) = (a.clone(), b.clone());
        solve_real(&mut a, &mut b, n, 1e-12).unwrap();
        for i in 0..n {
            let mut sum = 0.0;
            for j in 0..n {
                sum += a0[i * n + j] * b[j];
            }
            assert!((sum - b0[i]).abs() < 1e-9, "row {i}");
        }
    }

    #[test]
    fn a_complex_system_is_solved_and_a_singular_one_is_refused() {
        // (1+j) x = 2j -> x = 1+j
        let mut a = [Complex::new(1.0, 1.0)];
        let mut b = [Complex::new(0.0, 2.0)];
        solve_complex(&mut a, &mut b, 1, 1e-12).unwrap();
        assert!((b[0].re - 1.0).abs() < 1e-12 && (b[0].im - 1.0).abs() < 1e-12);

        let mut a2 = [
            Complex::new(0.0, 1.0),
            Complex::new(1.0, 0.0),
            Complex::new(1.0, 0.0),
            Complex::new(0.0, -1.0),
        ];
        let mut b2 = [Complex::new(1.0, 0.0), Complex::new(2.0, 0.0)];
        assert_eq!(solve_complex(&mut a2, &mut b2, 2, 1e-12), Err(1));
    }

    #[test]
    fn a_complex_system_with_a_known_answer() {
        // [[1, j], [-j, 2]] x = [1+j*1... ] with x = [1, j]: b0 = 1 + j*j = 0 ; b1 = -j + 2j = j
        let mut a = [
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 1.0),
            Complex::new(0.0, -1.0),
            Complex::new(2.0, 0.0),
        ];
        let mut b = [Complex::new(0.0, 0.0), Complex::new(0.0, 1.0)];
        solve_complex(&mut a, &mut b, 2, 1e-12).unwrap();
        assert!((b[0].re - 1.0).abs() < 1e-12 && b[0].im.abs() < 1e-12);
        assert!(b[1].re.abs() < 1e-12 && (b[1].im - 1.0).abs() < 1e-12);
    }
}
