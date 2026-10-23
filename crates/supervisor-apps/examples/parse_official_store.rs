//! Read-only Supervisor catalog tool; stdin is a bounded official store response.
//! Supports generic name/repository filtering and paginated results.
use hars_supervisor_apps::catalog_filter::Filter;
use hars_supervisor_apps::official_store::parse_store_response;
use std::io::{self, Read};

#[derive(Default)]
struct OwnedFilters {
    text: Option<String>,
    repository: Option<String>,
    installed: Option<bool>,
    updates_only: bool,
    offset: usize,
    limit: usize,
}
impl OwnedFilters {
    fn query(&self) -> Filter<'_> {
        Filter {
            text: self.text.as_deref(),
            repository: self.repository.as_deref(),
            installed: self.installed,
            updates_only: self.updates_only,
            offset: self.offset,
            limit: self.limit,
        }
    }
}

fn read_flags(args: impl IntoIterator<Item = String>) -> Result<Option<OwnedFilters>, String> {
    let args: Vec<String> = args.into_iter().collect();
    if args.is_empty() {
        return Ok(None);
    }
    let mut q = OwnedFilters {
        limit: 50,
        ..Default::default()
    };
    let mut seen = std::collections::HashSet::new();
    let mut it = args.into_iter();
    while let Some(flag) = it.next() {
        if !seen.insert(flag.clone()) {
            return Err("duplicate option".into());
        }
        match flag.as_str() {
            "--updates-only" => q.updates_only = true,
            "--text" | "--repository" | "--installed" | "--offset" | "--limit" => {
                let raw = it.next().ok_or("missing option value")?;
                match flag.as_str() {
                    "--text" => q.text = Some(raw),
                    "--repository" => q.repository = Some(raw),
                    "--installed" => {
                        q.installed =
                            Some(raw.parse::<bool>().map_err(|_| "invalid installed flag")?)
                    }
                    "--offset" => q.offset = raw.parse::<usize>().map_err(|_| "invalid offset")?,
                    "--limit" => q.limit = raw.parse::<usize>().map_err(|_| "invalid limit")?,
                    _ => unreachable!(),
                }
            }
            _ => return Err("unknown option".into()),
        }
    }
    Ok(Some(q))
}
fn main() {
    if nix::unistd::Uid::effective().is_root() {
        eprintln!("REFUSED: read-only catalog tool may not run as root");
        std::process::exit(2);
    }
    let query = match read_flags(std::env::args().skip(1)) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("REFUSED: {e}");
            std::process::exit(2);
        }
    };
    let mut bytes = Vec::new();
    if io::stdin()
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        std::process::exit(2);
    }
    let store = match parse_store_response(&bytes) {
        Ok(store) => store,
        Err(_) => {
            eprintln!("REFUSED: invalid official Supervisor response");
            std::process::exit(2);
        }
    };
    let result = match query {
        Some(q) => match store.filter_apps(q.query()) {
            Ok(page) => serde_json::json!({
                "total":page.total,
                "apps":page.apps,
                "next_offset":page.next_offset,
                "activation_authorized":false,
            }),
            Err(_) => {
                eprintln!("REFUSED: invalid catalog filter");
                std::process::exit(2);
            }
        },
        None => serde_json::to_value(&store).expect("typed catalog"),
    };
    println!(
        "{}",
        serde_json::to_string(&result).expect("typed response")
    );
}
