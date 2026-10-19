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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use boa_engine::{Context, JsValue, Source};

    /// Builds a JsValue by evaluating a JS expression.
    fn eval_js(ctx: &mut Context, src: &str) -> JsValue {
        ctx.eval(Source::from_bytes(src.as_bytes())).unwrap()
    }

    // --- required_string ---

    #[test]
    fn required_string_ok() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({name: 'hi'})");
        let got = required_string(&obj, "name", &mut ctx).unwrap();
        assert_eq!(got, "hi");
    }

    #[test]
    fn required_string_missing() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({})");
        let err = required_string(&obj, "name", &mut ctx).unwrap_err();
        assert!(format!("{err}").contains("missing `name`"));
    }

    #[test]
    fn required_string_undefined() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({name: undefined})");
        let err = required_string(&obj, "name", &mut ctx).unwrap_err();
        assert!(format!("{err}").contains("missing `name`"));
    }

    #[test]
    fn required_string_null() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({name: null})");
        let err = required_string(&obj, "name", &mut ctx).unwrap_err();
        assert!(format!("{err}").contains("missing `name`"));
    }

    #[test]
    fn required_string_not_object() {
        let mut ctx = Context::default();
        let err = required_string(&eval_js(&mut ctx, "42"), "name", &mut ctx).unwrap_err();
        assert!(format!("{err}").contains("expected an object"));
    }

    #[test]
    fn required_string_coerces_number() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({prop: 42})");
        // boa's to_string coerces numbers rather than throwing a TypeError.
        let got = required_string(&obj, "prop", &mut ctx).unwrap();
        assert_eq!(got, "42");
    }

    // --- optional_f64 ---

    #[test]
    fn optional_f64_some() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({x: 1.5})");
        let got = optional_f64(&obj, "x", &mut ctx).unwrap();
        assert_eq!(got, Some(1.5));
    }

    #[test]
    fn optional_f64_missing() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({})");
        let got = optional_f64(&obj, "x", &mut ctx).unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn optional_f64_undefined() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({x: undefined})");
        let got = optional_f64(&obj, "x", &mut ctx).unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn optional_f64_null() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({x: null})");
        let got = optional_f64(&obj, "x", &mut ctx).unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn optional_f64_not_object() {
        let mut ctx = Context::default();
        let err = optional_f64(&eval_js(&mut ctx, "'hi'"), "x", &mut ctx).unwrap_err();
        assert!(format!("{err}").contains("expected an object"));
    }

    // --- optional_i64 ---

    #[test]
    fn optional_i64_some() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({n: 42.0})");
        let got = optional_i64(&obj, "n", &mut ctx).unwrap();
        assert_eq!(got, Some(42));
    }

    #[test]
    fn optional_i64_missing() {
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({})");
        let got = optional_i64(&obj, "n", &mut ctx).unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn optional_i64_nan_to_zero() {
        // NaN → i64 yields 0 (the unusual branch in the saturating cast).
        let mut ctx = Context::default();
        let obj = eval_js(&mut ctx, "({n: NaN})");
        let got = optional_i64(&obj, "n", &mut ctx).unwrap();
        assert_eq!(got, Some(0));
    }
}
