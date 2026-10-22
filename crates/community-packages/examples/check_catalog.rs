//! Offline parser for an explicitly supplied local HACS catalog JSON list.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: check_catalog /absolute/path/to/catalog-json".into());
    }
    let input = std::fs::read(&args[1])?;
    let entries = hars_community_packages::parse_catalog(&input)
        .map_err(|e| format!("catalog structural validation rejected: {e:?}"))?;
    println!(
        "PASS: {} structurally valid unique owner/repo entries; no packages installed or executed",
        entries.len()
    );
    Ok(())
}
