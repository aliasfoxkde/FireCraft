//! Value marshalling between JS and the model's scalar types: string /
//! number / boolean / point reads off host objects, with typed
//! refusals instead of coercions gone wrong. Grows with the DOM
//! projection (M9 task 2).

use boa_engine::{Context, JsNativeError, JsResult, JsValue};

/// Reads a required string property off an object-ish value.
///
/// # Errors
/// A `TypeError` when the value is not an object or the property is
/// missing / not a string.
pub fn required_string(value: &JsValue, property: &str, context: &mut Context) -> JsResult<String> {
    let Some(object) = value.as_object() else {
        return Err(JsNativeError::typ()
            .with_message(format!("expected an object with `{property}`"))
            .into());
    };
    let raw = object.get(boa_engine::js_string!(property), context)?;
    if raw.is_undefined() || raw.is_null() {
        return Err(JsNativeError::typ()
            .with_message(format!("missing `{property}`"))
            .into());
    }
    Ok(raw.to_string(context)?.to_std_string_escaped())
}

/// Reads an optional `f64` property (`undefined`/`null` -> `None`).
///
/// # Errors
/// A `TypeError` when the value is an object without numeric
/// coercion.
pub fn optional_f64(
    value: &JsValue,
    property: &str,
    context: &mut Context,
) -> JsResult<Option<f64>> {
    let Some(object) = value.as_object() else {
        return Err(JsNativeError::typ()
            .with_message(format!("expected an object with `{property}`"))
            .into());
    };
    let raw = object.get(boa_engine::js_string!(property), context)?;
    if raw.is_undefined() || raw.is_null() {
        return Ok(None);
    }
    Ok(Some(raw.to_number(context)?))
}

/// Reads an optional `i64` property (truncation toward zero, the JS
/// `ToInt32`-family posture; out-of-range saturates).
///
/// # Errors
/// Same as [`optional_f64`].
pub fn optional_i64(
    value: &JsValue,
    property: &str,
    context: &mut Context,
) -> JsResult<Option<i64>> {
    let number = optional_f64(value, property, context)?;
    Ok(number.map(|n| if n.is_nan() { 0 } else { n as i64 }))
}
