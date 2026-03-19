use anyhow::Result;
use influx_query::{execute_query, format_response, parse_args};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let options = parse_args(std::env::args())?;
    let body = execute_query(&options)?;
    let rendered = format_response(&options, &body)?;
    if !rendered.is_empty() {
        println!("{rendered}");
    }
    Ok(())
}
