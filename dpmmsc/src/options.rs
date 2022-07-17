use crate::priors::{GaussianPrior, PriorHyperParams};

#[derive(Debug, Clone, PartialEq)]
pub struct OutlierRemoval<P: GaussianPrior> {
    pub weight: f64,
    pub dist: P::HyperParams,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelOptions<P: GaussianPrior> {
    pub data_dist: P::HyperParams,
    pub alpha: f64,
    pub dim: usize,
    pub burnout_period: usize,
    pub outlier: Option<OutlierRemoval<P>>,
    pub hard_assignment: bool,
}

impl<P: GaussianPrior> ModelOptions<P> {
    pub fn default(dim: usize) -> Self {
        Self {
            data_dist: P::HyperParams::default(dim),
            alpha: 10.0,
            dim,
            burnout_period: 20,
            outlier: Some(OutlierRemoval {
                weight: 0.05,
                dist: P::HyperParams::default(dim),
            }),
            hard_assignment: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FitOptions {
    pub seed: u64,
    pub init_clusters: usize,
    pub max_clusters: usize,
    pub iters: usize,
    pub argmax_sample_stop: usize,
    pub iter_split_stop: usize,
}

impl Default for FitOptions {
    fn default() -> Self {
        Self {
            seed: 42,
            init_clusters: 1,
            max_clusters: usize::MAX,
            iters: 100,
            argmax_sample_stop: 5,
            iter_split_stop: 5,
        }
    }
}
