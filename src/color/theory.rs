use crate::util::macros::debug_assert_finite;
use crate::util::range::is_range_within_another_ref;
use anyhow::{Context, Result};
use linfa::prelude::Fit;
use linfa::DatasetBase;
use linfa_clustering::KMeansParams;
use linfa_nn::distance::Distance;
use ndarray::{Array2, ArrayView, Axis, Dimension};
use palette::color_difference::ImprovedCiede2000;
use palette::white_point::D65;
use palette::{FromColor, Lab, Okhsl};
use rand::Rng;
use std::fmt::Debug;
use std::ops::RangeInclusive;

/// TODO Docs
///
/// Calculates the perceptual [Distance] between two Lab colors.
///
/// This implementation uses the [ImprovedCiede2000] for color difference measurements.
#[derive(Debug, Copy, Clone)]
pub struct LabImprovedCiede2000Distance;

impl Distance<f32> for LabImprovedCiede2000Distance {
    fn distance<D: Dimension>(&self, a: ArrayView<f32, D>, b: ArrayView<f32, D>) -> f32 {
        debug_assert!(a.len() == b.len());
        debug_assert!(a.dim() == b.dim());
        let shape = (a.len() / 3, 3);

        let a = a
            .to_shape(shape)
            .expect("failed to transform 'a' ArrayView");
        let b = b
            .to_shape(shape)
            .expect("failed to transform 'b' ArrayView");

        let mut sum = 0.0;
        let mut row = 0;

        while row < a.len_of(Axis(0)) {
            let a_lab = Lab::<D65>::new(a[(row, 0)], a[(row, 1)], a[(row, 2)]);
            let b_lab = Lab::<D65>::new(b[(row, 0)], b[(row, 1)], b[(row, 2)]);

            debug_assert_finite!(a_lab, b_lab);
            sum += a_lab.improved_difference(b_lab);
            row += 1;
        }

        sum
    }
}


/// TODO Docs
///
/// The original idea for this color palette generation is inspired by
/// [i want hue](https://medialab.github.io/iwanthue/).
///
/// This implementation uses k-means clustering, utilizing [D] to calculate color distances.
///
/// To specify the number of colors, specify the number of clusters in [KMeansParams].
///
/// * `hue_range`: degrees.
/// * `saturation_range`: 0..=1.
/// * `lightness_range`: 0..=1.
pub fn i_want_hue_color_palette<R, D>(
    hue_range: RangeInclusive<f32>,
    saturation_range: RangeInclusive<f32>,
    lightness_range: RangeInclusive<f32>,
    dataset_size: usize,
    kmeans_params: KMeansParams<f32, R, D>,
) -> Result<Vec<Okhsl>>
where
    R: Rng + Clone,
    D: Distance<f32>,
{
    assert!(!hue_range.is_empty(), "hue_range is empty");
    assert!(!saturation_range.is_empty(), "saturation_range is empty");
    assert!(!lightness_range.is_empty(), "lightness_range is empty");
    assert!(dataset_size > 0, "dataset_size must be > 0");

    assert!(
        is_range_within_another_ref(&saturation_range, &(0.0..=1.0)),
        "saturation_range must be within [0.0..=1.0] range"
    );
    assert!(
        is_range_within_another_ref(&lightness_range, &(0.0..=1.0)),
        "lightness_range must be within [0.0..=1.0] range"
    );

    let observations = {
        fn length_of(range: &RangeInclusive<f32>) -> f32 {
            (range.end() - range.start()) + 1.0
        }

        let (hue_step, saturation_step, light_step) = {
            let size = (dataset_size - 1) as f32;
            (
                length_of(&hue_range) / size,
                length_of(&saturation_range) / size,
                length_of(&lightness_range) / size,
            )
        };
        debug_assert_finite!(hue_step, saturation_step, light_step);

        let dataset: Vec<f32> = (0..dataset_size)
            .map(|i| i as f32)
            .flat_map(|i| {
                let hsl = Okhsl::new(
                    hue_range.start() + (i * hue_step),
                    saturation_range.start() + (i * saturation_step),
                    lightness_range.start() + (i * light_step),
                );
                let lab = Lab::from_color(hsl);

                debug_assert_finite!(hsl, lab);
                [lab.l, lab.a, lab.b]
            })
            .collect();

        let dataset = Array2::from_shape_vec((dataset.len() / 3, 3), dataset)
            .context("failed to create Lab colors dataset")?;
        DatasetBase::from(dataset)
    };

    let model = kmeans_params
        .fit(&observations)
        .context("failed to create Lab colors KMeans model")?;
    let mut colors: Vec<Lab> = model
        .centroids()
        .outer_iter()
        .map(|row| Lab::new(row[0], row[1], row[2]))
        .collect();

    {
        // Sort colors from darkest to lightest.
        let black = Lab::from_color(Okhsl::new(0.0, 0.0, 0.0));
        colors.sort_unstable_by(|&a, &b| {
            a.improved_difference(black)
                .total_cmp(&b.improved_difference(black))
        });
    }

    Ok(colors.into_iter().map(Okhsl::from_color).collect())
}
