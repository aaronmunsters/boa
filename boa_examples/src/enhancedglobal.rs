use boa::{property::Attribute, Context};
use std::fs::read_to_string;

pub fn run() {
    let js_file_path = "./scripts/enhancedglobal.js";
    let buffer = read_to_string(js_file_path);

    if buffer.is_err() {
        println!("Error: {}", buffer.unwrap_err());
        return;
    }

    // We need to create a Javascript context.
    let mut ctx = Context::new();

    // We can register a global property
    ctx.register_global_property(
        "customstring",
        "Hello! I am a custom global variable",
        Attribute::all(),
    );

    ctx.register_global_closure("rusty_hello", 0, |_, args, _| {
        let arg = args.get(0).unwrap();
        let val = format!(
            "Hello from Rust! You passed {}",
            arg.as_string().unwrap().as_str()
        );

        Ok(val.into())
    });

    ctx.register_global_closure("add", 0, |_, args, _| {
        let arg = args.get(0).unwrap();
        let val = format!(
            "Hello from Rust! You passed {}",
            arg.as_string().unwrap().as_str()
        );

        Ok(val.into())
    });

    //Loading, parsing and executing the JS code from the source file
    ctx.eval(&buffer.unwrap());
}
