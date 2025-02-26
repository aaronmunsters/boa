//! A custom `TimeZone` object.
use crate::{property::PropertyKey, string::utf16, Context, JsObject, JsValue};

use boa_gc::{Finalize, Trace};
use boa_temporal::{
    components::{tz::TzProtocol, Instant},
    TemporalError, TemporalResult,
};
use num_bigint::BigInt;

#[derive(Debug, Clone, Trace, Finalize)]
pub(crate) struct JsCustomTimeZone {
    tz: JsObject,
}

impl TzProtocol for JsCustomTimeZone {
    fn get_offset_nanos_for(&self, ctx: &mut dyn std::any::Any) -> TemporalResult<BigInt> {
        let context = ctx
            .downcast_mut::<Context>()
            .expect("Context was not provided for a CustomTz");

        let method = self
            .tz
            .get(utf16!("getOffsetNanosFor"), context)
            .expect("Method must exist for the custom calendar to be valid.");

        let result = method
            .as_callable()
            .expect("is method")
            .call(&method, &[], context)
            .map_err(|e| TemporalError::general(e.to_string()))?;

        // TODO (nekevss): Validate that the below conversion is fine vs. matching to JsValue::BigInt()
        let Some(bigint) = result.as_bigint() else {
            return Err(TemporalError::r#type()
                .with_message("Expected BigInt return from getOffsetNanosFor"));
        };

        Ok(bigint.as_inner().clone())
    }

    fn get_possible_instant_for(
        &self,
        ctx: &mut dyn std::any::Any,
    ) -> TemporalResult<Vec<Instant>> {
        let _context = ctx
            .downcast_mut::<Context>()
            .expect("Context was not provided for a CustomTz");

        // TODO: Implement once Instant has been migrated to `boa_temporal`'s Instant.
        Err(TemporalError::range().with_message("Not yet implemented."))
    }

    fn id(&self, ctx: &mut dyn std::any::Any) -> TemporalResult<String> {
        let context = ctx
            .downcast_mut::<Context>()
            .expect("Context was not provided for a CustomTz");

        let ident = self
            .tz
            .__get__(
                &PropertyKey::from(utf16!("id")),
                JsValue::undefined(),
                &mut context.into(),
            )
            .expect("Method must exist for the custom calendar to be valid.");

        let JsValue::String(id) = ident else {
            return Err(
                TemporalError::r#type().with_message("Invalid custom Time Zone identifier type.")
            );
        };

        Ok(id.to_std_string_escaped())
    }
}
