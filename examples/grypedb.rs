use anyhow::Result;
use geekorm::GeekConnector;
use std::path::PathBuf;

use konarr::{
    models::Component,
    utils::grypedb::{GrypeDatabase, GrypeVulnerability},
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Grype DB...");

    let grypedb_dir = PathBuf::from("./data/grypedb");
    let grype_conn = GrypeDatabase::connect(&grypedb_dir).await?;

    let grype = GrypeDatabase::fetch_grype(&grype_conn).await?;
    let vulnerabilities_count = GrypeVulnerability::total(&grype_conn).await?;
    println!(
        "GrypeDB({}) = {}\n",
        grype.build_timestamp, vulnerabilities_count
    );

    // Create a OpenSSL component
    let openssl_versions = vec![
        Component::from_purl("pkg:deb/debian/openssl@1.1.1").unwrap(),
        // Debian and Alpine versions
        Component::from_purl("pkg:deb/debian/openssl@3.2.1").unwrap(),
        Component::from_purl("pkg:apk/alpine/openssl@3.2.1").unwrap(),
        Component::from_purl("pkg:rpm/centos/openssl@3.2.1").unwrap(),
        Component::from_purl("pkg:deb/debian/openssl@3.3.0").unwrap(),
        // Latest Version
        Component::from_purl("pkg:deb/debian/openssl@3.4.0").unwrap(),
    ];

    for (comp_openssl, comp_openssl_ver) in openssl_versions {
        // Find vulnerabilities for the OpenSSL component
        let results =
            GrypeVulnerability::find_vulnerabilities(&grype_conn, &comp_openssl, &comp_openssl_ver)
                .await?;

        println!(
            "> {}@{} :: {:>4} alerts",
            comp_openssl.purl(),
            comp_openssl_ver.version,
            results.len()
        );

        // for alert in results {
        //     println!("  - {}", alert.id);
        // }
    }

    Ok(())
}