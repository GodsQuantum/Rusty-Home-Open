use hars_supervisor_apps::catalog_filter::{Filter, FilterError};
use hars_supervisor_apps::official_store::parse_store_response;
use serde_json::json;

fn sample(size: usize) -> hars_supervisor_apps::official_store::Catalog {
    let apps: Vec<_> = (0..size)
        .rev()
        .map(|i| {
            json!({
                "slug": format!("future_{i:05}"),
                "name": format!("Arbitrary Application {i}"),
                "repository": format!("third_party_{}",i%11),
                "available": true,"installed": i%2==0,
                "update_available": i%3==0
            })
        })
        .collect();
    let repos: Vec<_> = (0..11)
        .map(|i| json!({"slug":format!("third_party_{i}"),"name":format!("Third Party {i}")}))
        .collect();
    let value = json!({"result":"ok","data":{"apps":apps,"repositories":repos}});
    parse_store_response(&serde_json::to_vec(&value).unwrap()).unwrap()
}

#[test]
fn arbitrary_unlisted_apps_and_11_repositories_paginate_without_duplicates() {
    let catalog = sample(1373);
    let mut offset = 0;
    let mut seen = Vec::new();
    loop {
        let page = catalog
            .filter_apps(Filter {
                offset,
                limit: 83,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.total, 1373);
        seen.extend(page.apps.iter().map(|a| a.slug.as_str()));
        if let Some(next) = page.next_offset {
            offset = next;
        } else {
            break;
        }
    }
    assert_eq!(seen.len(), 1373);
    assert_eq!(seen[0], "future_00000");
    assert_eq!(seen[1372], "future_01372");
    assert_eq!(
        seen.iter().collect::<std::collections::HashSet<_>>().len(),
        1373
    );
}
#[test]
fn filters_any_repository_updates_installed_and_unicode_name() {
    let catalog = sample(333);
    let page = catalog
        .filter_apps(Filter {
            repository: Some("THIRD_PARTY_7"),
            installed: Some(true),
            updates_only: true,
            limit: 200,
            offset: 0,
            ..Default::default()
        })
        .unwrap();
    assert!(page.total > 0);
    assert!(
        page.apps
            .iter()
            .all(|app| app.repository == "third_party_7" && app.installed && app.update_available)
    );
    let custom = json!({"apps":[{"slug":"alpha","name":"Éclairage intérieur",
       "repository":"unlisted","available":true,"installed":false,"update_available":false}],
       "repositories":[{"slug":"unlisted","name":"Custom"}]});
    let data = parse_store_response(&serde_json::to_vec(&custom).unwrap()).unwrap();
    assert_eq!(
        data.filter_apps(Filter {
            text: Some("ÉCLAIRAGE"),
            limit: 1,
            ..Default::default()
        })
        .unwrap()
        .total,
        1
    );
}
#[test]
fn rejects_untrusted_queries_and_handles_offset_overflow() {
    let catalog = sample(201);
    for size in [0, 201, usize::MAX] {
        assert!(matches!(
            catalog.filter_apps(Filter {
                limit: size,
                ..Default::default()
            }),
            Err(FilterError::InvalidLimit)
        ));
    }
    for text in ["new\nline", "\0bad"] {
        assert!(matches!(
            catalog.filter_apps(Filter {
                limit: 10,
                text: Some(text),
                ..Default::default()
            }),
            Err(FilterError::InvalidQuery)
        ));
    }
    let end = catalog
        .filter_apps(Filter {
            limit: 50,
            offset: usize::MAX,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(end.total, 201);
    assert!(end.apps.is_empty() && end.next_offset.is_none());
}
