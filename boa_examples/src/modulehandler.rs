use std::fs::read_to_string;

use boa::{property::Attribute, Context, JsResult, JsValue};

pub fn main() {
    let js_file_path = "./scripts/calctest.js";
    let buffer = read_to_string(js_file_path);

    if buffer.is_err() {
        println!("Error: {}", buffer.unwrap_err());
        return;
    }

    //Creating the execution context
    let mut ctx = Context::new();

    //Adding custom implementation that mimics 'require'
    ctx.register_global_function("require", 0, require).unwrap();

    //Addming custom object that mimics 'module.exports'
    ctx.register_global_property(String::from("ioj"), "uiasd", Attribute::all());

    //Loading, parsing and executing the JS code from the source file
    ctx.eval(buffer.unwrap()).unwrap();
}

//Custom implementation that mimics 'require' module loader
fn require(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let arg = args.get(0).unwrap();

    //BUG: Dev branch seems to be passing string arguments along with quotes
    let libfile = arg.to_string(ctx).unwrap().replace("\"", "");

    //Read the module source file
    println!("Loading: {}", libfile);
    let buffer = read_to_string(libfile);
    match buffer {
        Err(error) => {
            println!("Error: {}", error);
            Ok(JsValue::from(-1))
        }
        Ok(buffer) => {
            //Load and parse the module source
            match ctx.eval(buffer) {
                Ok(t) => t,
                Err(_) => panic!("called `Result::unwrap()` on an `Err` value"),
            };

            //Access module.exports and return as ResultValue
            let module_exports = ctx
                .global_object()
                .get("module", ctx)
                .unwrap()
                .as_object()
                .unwrap()
                .get("exports", ctx)
                .unwrap();

            Ok(module_exports)
        }
    }
}
