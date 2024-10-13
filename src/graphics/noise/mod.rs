pub(crate) mod theory;

use crate::common::MyRc;
use crate::math::real::debug_assert_finite;
use noise::{
    Abs, Add, BasicMulti, Billow, Curve, Fbm, HybridMulti, Max, Min, Multiply, Negate, NoiseFn,
    Perlin, PerlinSurflet, Power, RidgedMulti, RotatePoint, Seedable, Simplex, SuperSimplex,
    Terrace, Turbulence, Worley,
};

type Float = f64;
const DIM: usize = 2;

#[derive(Clone)]
pub(crate) enum DynNoise {
    Perlin(Perlin),
    PerlinSurflet(PerlinSurflet),
    Simplex(Simplex),
    SuperSimplex(SuperSimplex),
    Worley(Worley),

    Fbm(Fbm<MyRc<DynNoise>>),
    Billow(Billow<MyRc<DynNoise>>),
    BasicMulti(BasicMulti<MyRc<DynNoise>>),
    HybridMulti(HybridMulti<MyRc<DynNoise>>),
    RidgedMulti(RidgedMulti<MyRc<DynNoise>>),
    Turbulence(Turbulence<MyRc<DynNoise>, MyRc<DynNoise>>),

    Abs(Abs<Float, MyRc<DynNoise>, DIM>),
    Negate(Negate<Float, MyRc<DynNoise>, DIM>),
    RotatePoint(RotatePoint<MyRc<DynNoise>>),
    Curve(Curve<Float, MyRc<DynNoise>, DIM>),
    Terrace(Terrace<Float, MyRc<DynNoise>, DIM>),

    Add(Add<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>),
    Multiply(Multiply<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>),
    Power(Power<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>),
    Min(Min<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>),
    Max(Max<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>),
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

impl_from!(Fbm, Fbm<MyRc<DynNoise>>);
impl_from!(Billow, Billow<MyRc<DynNoise>>);
impl_from!(BasicMulti, BasicMulti<MyRc<DynNoise>>);
impl_from!(HybridMulti, HybridMulti<MyRc<DynNoise>>);
impl_from!(RidgedMulti, RidgedMulti<MyRc<DynNoise>>);
impl_from!(Turbulence, Turbulence<MyRc<DynNoise>, MyRc<DynNoise>>);

impl_from!(Abs, Abs<Float, MyRc<DynNoise>, DIM>);
impl_from!(Negate, Negate<Float, MyRc<DynNoise>, DIM>);
impl_from!(RotatePoint, RotatePoint<MyRc<DynNoise>>);
impl_from!(Curve, Curve<Float, MyRc<DynNoise>, DIM>);
impl_from!(Terrace, Terrace<Float, MyRc<DynNoise>, DIM>);

impl_from!(Add, Add<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>);
impl_from!(Multiply, Multiply<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>);
impl_from!(Power, Power<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>);
impl_from!(Min, Min<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>);
impl_from!(Max, Max<Float, MyRc<DynNoise>, MyRc<DynNoise>, DIM>);

impl NoiseFn<Float, DIM> for DynNoise {
    fn get(&self, point: [Float; DIM]) -> f64 {
        fn clamp(value: Float) -> Float {
            // Prevent noise::math::vectors::Vector3<T>::floor_to_isize -> unwrap() panic
            debug_assert_finite!(value);
            value.clamp(isize::MIN as Float, isize::MAX as Float)
        }

        let point = {
            let mut safe_point: [Float; DIM] = [0.0; DIM];

            for i in 0..DIM {
                safe_point[i] = clamp(point[i]);
            }

            safe_point
        };

        let noise_value = match self {
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
            DynNoise::Curve(n) => n.get(point),
            DynNoise::Terrace(n) => n.get(point),
            DynNoise::Add(n) => n.get(point),
            DynNoise::Multiply(n) => n.get(point),
            DynNoise::Power(n) => n.get(point),
            DynNoise::Min(n) => n.get(point),
            DynNoise::Max(n) => n.get(point),
        };

        clamp(noise_value)
    }
}

impl NoiseFn<Float, DIM> for MyRc<DynNoise> {
    fn get(&self, point: [Float; DIM]) -> f64 {
        self.as_ref().get(point)
    }
}

impl Default for DynNoise {
    fn default() -> Self {
        DynNoise::from(Perlin::default())
    }
}

impl Default for MyRc<DynNoise> {
    fn default() -> Self {
        MyRc::new(DynNoise::default())
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
            DynNoise::Abs(_) => self,
            DynNoise::Negate(_) => self,
            DynNoise::RotatePoint(_) => self,
            DynNoise::Curve(_) => self,
            DynNoise::Terrace(_) => self,
            DynNoise::Add(_) => self,
            DynNoise::Multiply(_) => self,
            DynNoise::Power(_) => self,
            DynNoise::Min(_) => self,
            DynNoise::Max(_) => self,
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
            DynNoise::Abs(_) => 0,
            DynNoise::Negate(_) => 0,
            DynNoise::RotatePoint(_) => 0,
            DynNoise::Curve(_) => 0,
            DynNoise::Terrace(_) => 0,
            DynNoise::Add(_) => 0,
            DynNoise::Multiply(_) => 0,
            DynNoise::Power(_) => 0,
            DynNoise::Min(_) => 0,
            DynNoise::Max(_) => 0,
        }
    }
}

impl Seedable for MyRc<DynNoise> {
    fn set_seed(self, seed: u32) -> Self {
        MyRc::new(DynNoise::set_seed(self.as_ref().clone(), seed))
    }

    fn seed(&self) -> u32 {
        DynNoise::seed(self.as_ref())
    }
}
