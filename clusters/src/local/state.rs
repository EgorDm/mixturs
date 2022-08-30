use std::marker::PhantomData;
use nalgebra::{Dim, DMatrix, DVector, Dynamic, Matrix, Storage};
use rand::distributions::{Distribution, WeightedIndex};
use rand::Rng;
use statrs::distribution::{Continuous};
use crate::global::GlobalState;
use crate::options::ModelOptions;
use crate::stats::{GaussianPrior, SufficientStats};
use crate::utils::row_normalize_log_weights;

pub type LocalStats<P: GaussianPrior> = Vec<(P::SuffStats, [P::SuffStats; 2])>;

#[derive(Debug, Clone, PartialEq)]
pub struct LocalState<P: GaussianPrior> {
    pub data: DMatrix<f64>,
    pub labels: DVector<usize>,
    pub labels_aux: DVector<usize>,
    pub _phantoms: PhantomData<P>,
}

impl<P: GaussianPrior> LocalState<P> {
    pub fn from_init<R: Rng>(
        data: DMatrix<f64>,
        n_clusters: usize,
        options: &ModelOptions<P>,
        rng: &mut R
    ) -> Self {
        let n_points = data.nrows();
        let n_clusters = n_clusters + options.outlier.is_some() as usize;
        let labels = DVector::from_fn(n_points, |i, _| rng.gen_range(0..n_clusters));
        let labels_aux = DVector::from_fn(n_points, |i, _| rng.gen_range(0..2));
        Self {
            data,
            labels,
            labels_aux,
            _phantoms: PhantomData,
        }
    }

    pub fn n_points(&self) -> usize {
        self.data.nrows()
    }

    pub fn update_sample_labels<R: Rng>(
        global: &GlobalState<P>,
        local: &mut LocalState<P>,
        is_final: bool,
        rng: &mut R,
    ) {
        // Calculate log likelihood for each point
        let ln_weights = global.weights.iter().map(|w| w.ln()).collect::<Vec<_>>();
        let mut ll = DMatrix::zeros(local.n_points(), global.clusters.len());
        for (k, cluster) in global.clusters.iter().enumerate() {
            for (i, point) in local.data.row_iter().enumerate() {
                ll[(i, k)] = cluster.prim.dist.ln_pdf(&point.transpose()) + ln_weights[k];
            }
        }

        // Sample labels
        if is_final {
            for (i, row) in ll.row_iter().enumerate() {
                local.labels[i] = argmax(&row);
            }
        } else {
            row_normalize_log_weights(&mut ll);
            sample_weighted(&ll, &mut local.labels, rng);
        }
    }

    pub fn update_sample_labels_aux<R: Rng>(
        global: &GlobalState<P>,
        local: &mut LocalState<P>,
        rng: &mut R,
    ) {
        let mut ll = DMatrix::zeros(local.n_points(), 2);
        for (k, cluster) in global.clusters.iter().enumerate() {
            let ln_weights = cluster.weights.iter().map(|w| w.ln()).collect::<Vec<_>>();
            for (i, _) in local.labels.iter().enumerate().filter(|(_, &label)| label == k) {
                let point = local.data.row(i).transpose();
                for a in 0..2 {
                    ll[(i, a)] = cluster.aux[a].dist.ln_pdf(&point) + ln_weights[a];
                }
            }
        }

        // Sample labels
        row_normalize_log_weights(&mut ll);
        sample_weighted(&ll, &mut local.labels_aux, rng);
    }

    pub fn collect_stats(
        local: &LocalState<P>,
        n_clusters: usize,
    ) -> LocalStats<P> {
        (0..n_clusters)
            .map(|k| Self::collect_stats_cluster(local, k))
            .collect()
    }

    pub fn collect_stats_cluster(
        local: &LocalState<P>,
        cluster_id: usize,
    ) -> (P::SuffStats, [P::SuffStats; 2]) {
        let idx_l: Vec<_> = local.labels.iter().cloned()
            .zip(local.labels_aux.iter().cloned())
            .enumerate()
            .filter(|(_, (x, y))| *x == cluster_id && *y == 0)
            .map(|(i, _)| i)
            .collect();
        let idx_r = local.labels.iter().cloned()
            .zip(local.labels_aux.iter().cloned())
            .enumerate()
            .filter(|(_, (x, y))| *x == cluster_id && *y == 1)
            .map(|(i, _)| i);

        let idx: Vec<_> = idx_l.iter().cloned().chain(idx_r).collect();
        let points = local.data.select_rows(&idx);

        let prim = P::SuffStats::from_data(&points);
        let aux = [
            P::SuffStats::from_data(&points.rows_range(0..idx_l.len()).into_owned()),
            P::SuffStats::from_data(&points.rows_range(idx_l.len()..).into_owned()),
        ];

        (prim, aux)
    }

    pub fn update_reset_clusters<R: Rng>(
        local: &mut LocalState<P>,
        cluster_idx: &[usize],
        rng: &mut R,
    ) {
        for k in cluster_idx.iter().cloned() {
            for i in 0..local.n_points() {
                if local.labels[i] == k {
                    local.labels_aux[i] = rng.gen_range(0..2);
                }
            }
        }
    }

    pub fn update_remove_clusters(
        local: &mut LocalState<P>,
        cluster_idx: &[usize],
    ) {
        let mut removed = 0;
        for k in cluster_idx.iter().cloned() {
            for l in local.labels.iter_mut() {
                if *l > k - removed {
                    *l -= 1;
                }
            }
            removed += 1;
        }
    }
}

pub fn sample_weighted<R: Rng>(weights: &DMatrix<f64>, labels: &mut DVector<usize>, rng: &mut R) {
    for (i, row) in weights.row_iter().enumerate() {
        // TODO: take the weighted reservoir sampler from tch-geometric
        let dist = WeightedIndex::new(&row).unwrap();
        labels[i] = dist.sample(rng);
    }
}

pub fn argmax<R: Dim, C: Dim, S: Storage<f64, R, C>>(
    a: &Matrix<f64, R, C, S>,
) -> usize {
    let mut mv = std::f64::MIN;
    let mut mi = 0;
    a.iter().enumerate().for_each(|(i, &v)| {
        if v > mv {
            mv = v;
            mi = i;
        }
    });
    mi
}