use std::collections::hash_map::Entry;
use std::mem::MaybeUninit;
use nalgebra::{DMatrix, Dynamic, Matrix, Storage};
use std::collections::HashMap;
use num_traits::{FromPrimitive, One, PrimInt};
use std::hash::Hash;

pub fn each_ref<T, const N: usize>(data: &[T; N]) -> [&T; N] {
    // Unlike in `map`, we don't need a guard here, as dropping a reference
    // is a noop.
    let mut out = [MaybeUninit::uninit(); N];
    for (src, dst) in data.iter().zip(&mut out) {
        dst.write(src);
    }

    // SAFETY: All elements of `dst` are properly initialized and
    // `MaybeUninit<T>` has the same layout as `T`, so this cast is valid.
    unsafe { (&mut out as *mut _ as *mut [&T; N]).read() }
}


pub fn unique_with_indices<T: Copy + Hash + Eq + Ord>(data: &[T], sorted: bool) -> (Vec<T>, Vec<usize>) {
    let mut index = HashMap::new();
    let mut unique = Vec::new();

    for u in data {
        if !index.contains_key(u) {
            unique.push(*u);
            index.insert(*u, 0);
        }
    }

    if sorted {
        unique.sort();
    }
    for (i, u) in unique.iter().enumerate() {
        index.insert(*u, i);
    }

    let mut unique_index = Vec::with_capacity(data.len());
    for d in data {
        unique_index.push(index[d]);
    }

    (unique, unique_index)
}

pub fn bincount<T: Copy + Hash + Eq>(data: &[T]) -> HashMap<T, usize> {
    let mut counts = HashMap::new();
    for &u in data {
        match counts.entry(u) {
            Entry::Occupied(mut e) => {
                *e.get_mut() += 1;
            }
            Entry::Vacant(mut e) => {
                e.insert(1);
            }
        }
    }
    counts
}

pub fn row_normalize_log_weights(
    weights: &mut DMatrix<f64>
) {
    for mut row in weights.row_iter_mut() {
        let max = row.max();
        for x in row.iter_mut() {
            *x = (*x - max).exp();
        }
    }
}

pub fn col_normalize_log_weights(
    weights: &mut DMatrix<f64>
) {
    for mut col in weights.column_iter_mut() {
        let max = col.max();
        for x in col.iter_mut() {
            *x = (*x - max).exp();
        }
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::DMatrix;
    use num_traits::real::Real;
    use crate::stats::tests::test_almost_mat;

    #[test]
    fn test_unique_with_indices() {
        let data = [1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 1];
        let (unique, unique_index) = super::unique_with_indices(&data, false);
        assert_eq!(unique, vec![1, 2, 3, 4, 5]);
        assert_eq!(unique_index, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0]);
    }

    #[test]
    fn test_bincount() {
        let data = [1, 1, 2, 2, 3, 3, 4, 4, 5, 5];
        let counts = super::bincount(&data);
        let mut bincounts: Vec<_> = counts.into_iter().collect();
        bincounts.sort();
        assert_eq!(bincounts, vec![(1, 2), (2, 2), (3, 2), (4, 2), (5, 2)]);
    }

    #[test]
    fn test_normalize_log_weights() {
        let mut weights = DMatrix::from_row_slice(3, 3, &[
            1.0f64, 2.0, 4.0,
            1.0, 2.0, 4.0,
            1.0, 2.0, 4.0,
        ]);
        weights.iter_mut().for_each(|x| *x = x.ln());
        super::row_normalize_log_weights(&mut weights);
        test_almost_mat(&weights, &DMatrix::from_row_slice(3, 3, &[
            0.25, 0.5, 1.0,
            0.25, 0.5, 1.0,
            0.25, 0.5, 1.0,
        ]), 1e-4);

        let mut weights = DMatrix::from_row_slice(3, 3, &[
            1.0f64, 2.0, 4.0,
            1.0, 2.0, 4.0,
            1.0, 2.0, 4.0,
        ]);
        weights.iter_mut().for_each(|x| *x = x.ln());
        super::col_normalize_log_weights(&mut weights);
        test_almost_mat(&weights, &DMatrix::from_row_slice(3, 3, &[
            1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
        ]), 1e-4);
    }
}