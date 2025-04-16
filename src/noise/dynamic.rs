use noise::{
    Abs, BasicMulti, Billow, Fbm, HybridMulti, Max, Min, Negate, NoiseFn, Perlin, PerlinSurflet,
    RidgedMulti, RotatePoint, Seedable, Simplex, SuperSimplex, Turbulence, Worley,
};

type Scalar = f64;
const DIM: usize = 2;


#[derive(Clone)]
pub enum DynNoise {
    Perlin(Perlin),
    PerlinSurflet(PerlinSurflet),
    Simplex(Simplex),
    SuperSimplex(SuperSimplex),
    Worley(Worley),

    Fbm(Fbm<Box<DynNoise>>),
    Billow(Billow<Box<DynNoise>>),
    BasicMulti(BasicMulti<Box<DynNoise>>),
    HybridMulti(HybridMulti<Box<DynNoise>>),
    RidgedMulti(RidgedMulti<Box<DynNoise>>),
    Turbulence(Turbulence<Box<DynNoise>, Box<DynNoise>>),

    Abs(Abs<Scalar, Box<DynNoise>, DIM>),
    Negate(Negate<Scalar, Box<DynNoise>, DIM>),
    RotatePoint(RotatePoint<Box<DynNoise>>),
    Min(Min<Scalar, Box<DynNoise>, Box<DynNoise>, DIM>),
    Max(Max<Scalar, Box<DynNoise>, Box<DynNoise>, DIM>),
}


macro_rules! impl_from {
    ($Variant:ident, $Type:ty) => {
        impl From<$Type> for DynNoise {
            fn from(value: $Type) -> Self {
                DynNoise::$Variant(value)
            }
        }
    };
}

impl_from!(Perlin, Perlin);
impl_from!(PerlinSurflet, PerlinSurflet);
impl_from!(Simplex, Simplex);
impl_from!(SuperSimplex, SuperSimplex);
impl_from!(Worley, Worley);

impl_from!(Fbm, Fbm<Box<DynNoise>>);
impl_from!(Billow, Billow<Box<DynNoise>>);
impl_from!(BasicMulti, BasicMulti<Box<DynNoise>>);
impl_from!(HybridMulti, HybridMulti<Box<DynNoise>>);
impl_from!(RidgedMulti, RidgedMulti<Box<DynNoise>>);
impl_from!(Turbulence, Turbulence<Box<DynNoise>, Box<DynNoise>>);

impl_from!(Abs, Abs<Scalar, Box<DynNoise>, DIM>);
impl_from!(Negate, Negate<Scalar, Box<DynNoise>, DIM>);
impl_from!(RotatePoint, RotatePoint<Box<DynNoise>>);

impl_from!(Min, Min<Scalar, Box<DynNoise>, Box<DynNoise>, DIM>);
impl_from!(Max, Max<Scalar, Box<DynNoise>, Box<DynNoise>, DIM>);


impl Default for DynNoise {
    fn default() -> Self {
        DynNoise::from(Perlin::default())
    }
}


impl NoiseFn<Scalar, DIM> for DynNoise {
    fn get(&self, point: [Scalar; DIM]) -> f64 {
        match self {
            DynNoise::Perlin(n) => n.get(point),
            DynNoise::PerlinSurflet(n) => n.get(point),
            DynNoise::Simplex(n) => n.get(point),
            DynNoise::SuperSimplex(n) => n.get(point),
            DynNoise::Worley(n) => n.get(point),
            DynNoise::Fbm(n) => n.get(point),
            DynNoise::Billow(n) => n.get(point),
            DynNoise::BasicMulti(n) => n.get(point),
            DynNoise::HybridMulti(n) => n.get(point),
            DynNoise::RidgedMulti(n) => n.get(point),
            DynNoise::Turbulence(n) => n.get(point),
            DynNoise::Abs(n) => n.get(point),
            DynNoise::Negate(n) => n.get(point),
            DynNoise::RotatePoint(n) => n.get(point),
            DynNoise::Min(n) => n.get(point),
            DynNoise::Max(n) => n.get(point),
        }
    }
}

impl Seedable for DynNoise {
    fn set_seed(self, seed: u32) -> Self {
        match self {
            DynNoise::Perlin(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::PerlinSurflet(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::Simplex(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::SuperSimplex(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::Worley(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::Fbm(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::Billow(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::BasicMulti(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::HybridMulti(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::RidgedMulti(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::Turbulence(n) => DynNoise::from(n.set_seed(seed)),
            DynNoise::Abs(n) => {
                let source = n.source.set_seed(seed);
                DynNoise::from(Abs::new(source))
            }
            DynNoise::Negate(n) => {
                let source = n.source.set_seed(seed);
                DynNoise::from(Negate::new(source))
            }
            DynNoise::RotatePoint(n) => {
                let source = n.source.set_seed(seed);
                let noise =
                    RotatePoint::new(source).set_angles(n.x_angle, n.y_angle, n.z_angle, n.u_angle);
                DynNoise::from(noise)
            }
            DynNoise::Min(n) => {
                let first = n.source1.set_seed(seed);
                let second = n.source2.set_seed(seed);
                DynNoise::from(Min::new(first, second))
            }
            DynNoise::Max(n) => {
                let first = n.source1.set_seed(seed);
                let second = n.source2.set_seed(seed);
                DynNoise::from(Max::new(first, second))
            }
        }
    }

    fn seed(&self) -> u32 {
        match self {
            DynNoise::Perlin(n) => n.seed(),
            DynNoise::PerlinSurflet(n) => n.seed(),
            DynNoise::Simplex(n) => n.seed(),
            DynNoise::SuperSimplex(n) => n.seed(),
            DynNoise::Worley(n) => n.seed(),
            DynNoise::Fbm(n) => n.seed(),
            DynNoise::Billow(n) => n.seed(),
            DynNoise::BasicMulti(n) => n.seed(),
            DynNoise::HybridMulti(n) => n.seed(),
            DynNoise::RidgedMulti(n) => n.seed(),
            DynNoise::Turbulence(n) => n.seed(),
            _ => 0,
        }
    }
}

impl Seedable for Box<DynNoise> {
    fn set_seed(self, seed: u32) -> Self {
        Box::new(self.as_ref().clone().set_seed(seed))
    }

    fn seed(&self) -> u32 {
        self.as_ref().seed()
    }
}
