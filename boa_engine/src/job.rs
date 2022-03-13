use crate::{Context, JsResult, JsValue};

use gc::{Finalize, Trace};

#[derive(Debug, Clone, Trace, Finalize)]
pub struct JobCallback {
    callback: Box<JsValue>,
}

impl JobCallback {
    fn new(callback: JsValue) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }

    pub fn make_job_callback(callback: JsValue) -> Self {
        Self::new(callback)
    }

    /// TODO: determine how to get rid of context
    pub fn call_job_callback(
        &self,
        v: JsValue,
        argument_list: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        let callback = match *self.callback {
            JsValue::Object(ref object) if object.is_callable() => object.clone(),
            _ => panic!("Callback is not a callable object"),
        };

        callback.__call__(&v, &argument_list, context)
    }

    pub fn run(&self, context: &mut Context) {
        let callback = match *self.callback {
            JsValue::Object(ref object) if object.is_callable() => object.clone(),
            _ => panic!("Callback is not a callable object"),
        };

        let _ = callback.__call__(&JsValue::Undefined, &[], context);
    }
}
