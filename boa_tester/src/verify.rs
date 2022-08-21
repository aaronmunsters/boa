use std::io::Write;
use std::{
    fs::{read, File},
    path::PathBuf,
};

use boa_engine::Context;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

struct NameProgram {
    name: String,
    program: Vec<u8>,
}

fn read_programs(path: PathBuf) -> Vec<NameProgram> {
    path.read_dir()
        .expect("Path must be directory")
        .map(|entry| entry.expect("Could not read entry"))
        .map(|entry| NameProgram {
            name: String::from(
                entry
                    .file_name()
                    .as_os_str()
                    .to_str()
                    .expect("Could not convert to string"),
            ),
            program: read(entry.path()).expect("Could not read program"),
        })
        .collect()
}

pub(crate) fn verify_instrumentation(
    analysis_base: PathBuf,
    input_base: PathBuf,
    output_result: PathBuf,
) {
    let input_bases = read_programs(input_base);
    let analysis_bases = read_programs(analysis_base);
    let test_results: Vec<Option<BaseResult>> = input_bases
        .par_iter()
        .map(|input| verify_once(input, &analysis_bases))
        .collect();
    let mut csv_result: String = analyses_to_csv_row(&analysis_bases);
    for result in test_results {
        if let Some(r) = result {
            csv_result.push_str(&format!("\n{}", r.to_csv_row()))
        }
    }
    let mut output_file = File::create(output_result.as_path()).expect("Creation of output failed");
    write!(output_file, "{}", csv_result).expect("Writing to output failed!");
}

fn analyses_to_csv_row(analyses: &Vec<NameProgram>) -> String {
    format!(
        ",{}",
        analyses
            .iter()
            .map(|NameProgram { name, program: _ }| name.clone())
            .collect::<Vec<String>>()
            .join(",")
    )
}

struct BaseResult {
    input: String,
    results: Vec<String>,
}

impl BaseResult {
    fn to_csv_row(&self) -> String {
        match self {
            BaseResult { input, results } => format!("{},{}", input, results.join(",")),
        }
    }
}

fn verify_once(input: &NameProgram, analyses: &Vec<NameProgram>) -> Option<BaseResult> {
    let mut context = Context::default();
    println!("Running: {} bare", input.name);
    match context.eval(input.program.clone()) {
        Ok(uninstr_res) => {
            let mut per_analysis_result = vec![];
            for analysis in analyses {
                println!("Running: {} with {}", input.name, analysis.name);
                let mut context = Context::default();
                context.install_advice(analysis.program.clone());
                match context.eval(input.program.clone()) {
                    Ok(instr_res) => match uninstr_res.strict_equals(&instr_res) {
                        true => per_analysis_result.push(String::from("Success")),
                        false => per_analysis_result.push(String::from("Unmatched")),
                    },
                    Err(_) => per_analysis_result.push(String::from("Crash")),
                };
            }
            Some(BaseResult {
                input: input.name.clone(),
                results: per_analysis_result,
            })
        }
        Err(_) => None,
    }
}
