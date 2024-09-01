use colorgrad::Color;
use palette::color_difference::ImprovedCiede2000;
use palette::color_theory::{Analogous, Complementary, SplitComplementary, Tetradic, Triadic};
use palette::{FromColor, Hsl, Lab};
use rand::Rng;
use std::cmp::Ordering;

pub fn random_palette<R>(
    size: usize,
    primary_color: Hsl,
    random: &mut R,
) -> Vec<Color> where
    R: Rng,
{
    let mut palette = Vec::with_capacity(size + 3);
    palette.push(primary_color);

    while palette.len() < size {
        let base_color = palette[random.gen_range(0..palette.len())];
        let mut split = match random.gen_range(0..=4) {
            0 => vec![base_color.complementary()],
            1 => {
                let c = base_color.split_complementary();
                vec![c.0, c.1]
            }
            2 => {
                let c = base_color.analogous();
                vec![c.0, c.1]
            }
            3 => {
                let c = base_color.triadic();
                vec![c.0, c.1]
            }
            4 => {
                let c = base_color.tetradic();
                vec![c.0, c.1, c.2]
            }
            _ => panic!("Invalid random range in random_palette() method")
        };

        for i in 0..split.len().min(size - palette.len()) {
            let color = split[i];

            /*
             * I assume that the `size` parameter of the palette will not be large enough, so Vec O(n) will suffice.
             * Otherwise we will probably have to implement a custom HslHash structure with Hash impl.
             */
            if !palette.contains(&color) {
                palette.push(color)
            }
        }
    }

    let primary_lab = Lab::from_color(primary_color);
    let mut hsl_to_difference: Vec<(Hsl, f32)> = palette
        .iter()
        .map(|hsl| (*hsl, Lab::from_color(*hsl).improved_difference(primary_lab)))
        .collect();

    hsl_to_difference.sort_by(|t1, t2| {
        t1.1.partial_cmp(&t2.1).unwrap_or(Ordering::Equal)
    });

    hsl_to_difference
        .iter()
        .map(|t| t.0)
        .map(|hsl| Color::from_hsla(f32::from(hsl.hue), hsl.saturation, hsl.lightness, 1.0))
        .collect()
}
