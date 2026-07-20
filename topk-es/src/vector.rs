use topk_rs::proto::v1::control::VectorDistanceMetric;
use topk_rs::proto::v1::data::Value;

use crate::Error;

// A JSON number that overflows `f32` lands as infinity; ES rejects those rather
// than scoring against them.
pub fn ensure_finite(value: &Value) -> Result<(), Error> {
    let finite = match components(value) {
        Some(values) => values.iter().all(|v| v.is_finite()),
        None => true,
    };

    match finite {
        true => Ok(()),
        false => Err(Error::InvalidQuery(
            "element_type [float] vectors do not support infinite values".into(),
        )),
    }
}

// ES validates vector magnitude on write; TopK's metrics accept anything, so
// reject here to keep the ES contract.
pub fn ensure_magnitude(
    name: &str,
    metric: VectorDistanceMetric,
    value: &Value,
) -> Result<(), Error> {
    let norm = match value.as_f32_list() {
        Some(values) => values.iter().map(|v| v * v).sum::<f32>().sqrt(),
        None => return Ok(()),
    };

    match metric {
        VectorDistanceMetric::DotProduct if (norm - 1.0).abs() > 1e-4 => {
            Err(Error::BadRequest(format!(
                "The [dot_product] similarity can only be used with unit-length vectors. \
                 Field [{name}] has magnitude [{norm}]."
            )))
        }
        // Cosine is undefined for a zero vector — there is no direction to compare.
        VectorDistanceMetric::Cosine if norm == 0.0 => Err(Error::BadRequest(format!(
            "The [cosine] similarity does not support vectors with zero magnitude. \
             Preview of invalid vector: field [{name}]."
        ))),
        _ => Ok(()),
    }
}

fn components(value: &Value) -> Option<&[f32]> {
    match (value.as_f32_list(), value.as_f32_matrix()) {
        (Some(values), _) => Some(values),
        (_, Some((_, _, values))) => Some(values),
        _ => None,
    }
}
