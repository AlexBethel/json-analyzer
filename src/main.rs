use std::path::Path;

use clap::Arg;

fn main() {
    let app = clap::App::new("json-analyzer")
        .arg(
            Arg::with_name("file")
                .index(1)
                .help("The JSON file to analyze")
                .required(true),
        )
        .get_matches();

    let filename = Path::new(app.value_of_os("file").expect("Required option"));
    let text = match std::fs::read_to_string(filename) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Unable to open {:?} for reading: {}", filename, e);
            return;
        }
    };

    let data = match json::parse(&text) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Invalid json data: {}", e);
            return;
        }
    };

    println!("{}", data);
}
