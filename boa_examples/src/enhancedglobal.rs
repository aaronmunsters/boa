//! There are APIs for adding in global properties, (which will be on window), plus APIs for adding closures

use boa::{property::Attribute, Context};
use std::fs::read_to_string;

pub fn main() {
    let js_file_path = "./scripts/enhancedglobal.js";
    let buffer = read_to_string(js_file_path);

    match buffer {
        Err(error) => {
            println!("Error: {}", error);
        }
        Ok(buffer) => {
            // We need to create a Javascript context.
            let mut ctx = Context::new();

            // We can register a global property
            ctx.register_global_property(
                "customstring",
                "Hello! I am a custom global variable",
                Attribute::all(),
            );

            if let Err(error) = ctx.register_global_closure("rusty_hello", 0, |_, args, _| {
                let arg = args.get(0).unwrap();
                let val = format!(
                    "Hello from Rust! You passed {}",
                    arg.as_string().unwrap().as_str()
                );

                Ok(val.into())
            }) {
                println!("Error: {}", error.display().to_string());
            };

            if let Err(error) = ctx.register_global_closure("add", 0, |_, args, _| {
                let arg = args.get(0).unwrap();
                let val = format!(
                    "Hello from Rust! You passed {}",
                    arg.as_string().unwrap().as_str()
                );

                Ok(val.into())
            }) {
                println!("Error: {}", error.display().to_string());
            };

            //Loading, parsing and executing the JS code from the source file
            if let Err(error) = ctx.eval(&buffer) {
                println!("Error: {}", error.display().to_string());
            }
        }
    }
}
