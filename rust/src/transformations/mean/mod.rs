#[cfg(feature = "ffi")]
mod ffi;

use num::Float;
use opendp_derive::bootstrap;

use crate::core::{Metric, MetricSpace, Transformation};
use crate::domains::{AtomDomain, VectorDomain};
use crate::error::Fallible;
use crate::metrics::AbsoluteDistance;
use crate::traits::{ExactIntCast, InfMul};

use super::{
    make_lipschitz_float_mul, make_sum, LipschitzMulFloatDomain, LipschitzMulFloatMetric, MakeSum,
};

#[bootstrap(
    features("contrib"),
    generics(MI(suppress), T(suppress))
)]
/// Make a Transformation that computes the mean of bounded data.
///
/// This uses a restricted-sensitivity proof that takes advantage of known dataset size.
/// Use `make_clamp` to bound data and `make_resize` to establish dataset size.
///
/// # Arguments
/// * `size` - Number of records in input data.
/// * `bounds` - Tuple of inclusive lower and upper bounds.
///
/// # Generics
/// * `MI` - Input Metric. One of `SymmetricDistance` or `InsertDeleteDistance`
/// * `T` - Atomic Input Type and Output Type.
pub fn make_mean<MI, T>(
    input_domain: VectorDomain<AtomDomain<T>>,
    input_metric: MI,
) -> Fallible<Transformation<VectorDomain<AtomDomain<T>>, AtomDomain<T>, MI, AbsoluteDistance<T>>>
where
    MI: 'static + Metric,
    T: 'static + MakeSum<MI> + ExactIntCast<usize> + Float + InfMul,
    AtomDomain<T>: LipschitzMulFloatDomain<Atom = T>,
    AbsoluteDistance<T>: LipschitzMulFloatMetric<Distance = T>,
    (VectorDomain<AtomDomain<T>>, MI): MetricSpace,
    (AtomDomain<T>, AbsoluteDistance<T>): MetricSpace,
{
    let size = input_domain
        .size
        .ok_or_else(|| err!(MakeTransformation, "dataset size must be known. Either specify size in the input domain or use make_resize"))?;
    let bounds = input_domain
        .element_domain
        .get_closed_bounds()
        .ok_or_else(|| {
            err!(
                MakeTransformation,
                "input domain must consist of bounded data. Either specify bounds in the input domain or use make_clamp."
            )
        })?;
    if size == 0 {
        return fallible!(MakeTransformation, "dataset size must be positive");
    }
    let size_ = T::exact_int_cast(size)?;
    // don't loosen the bounds by the relaxation term because any value greater than nU is pure error
    let sum_bounds = (size_.neg_inf_mul(&bounds.0)?, size_.inf_mul(&bounds.1)?);
    make_sum::<MI, T>(input_domain, input_metric)?
        >> make_lipschitz_float_mul::<AtomDomain<T>, _>(size_.recip(), sum_bounds)?
}

#[cfg(test)]
mod tests {
    use crate::metrics::SymmetricDistance;

    use super::*;

    #[test]
    fn test_make_bounded_mean_symmetric() -> Fallible<()> {
        let transformation = make_mean(
            VectorDomain::new(AtomDomain::new_closed((0., 10.))?).with_size(5),
            SymmetricDistance::default(),
        )?;
        let arg = vec![1., 2., 3., 4., 5.];
        let ret = transformation.invoke(&arg)?;
        let expected = 3.;
        assert_eq!(ret, expected);
        assert!(transformation.check(&1, &2.)?);

        Ok(())
    }
}
