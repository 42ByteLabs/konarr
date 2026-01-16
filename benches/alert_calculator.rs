use criterion::async_executor::FuturesExecutor;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use geekorm::{ConnectionManager, prelude::*};
use konarr::tasks::alerts::alert_calculator;

use konarr::bom::BillOfMaterials;
use konarr::bom::sbom::{BomComponent, BomType};
use konarr::models::{ProjectType, Projects, Snapshot};
use konarr::{Config, KonarrError};

pub fn criterion_benchmark(c: &mut Criterion) {
    for projects in &[1, 10, 100, 1000] {
        c.bench_with_input(
            BenchmarkId::new("alert-calculator", projects),
            &projects,
            |b, &n| {
                b.to_async(FuturesExecutor)
                    .iter(|| test_alert_calculator(*n));
            },
        );
    }
}

async fn database(count: i32) -> Result<ConnectionManager, KonarrError> {
    let mut config = Config::default();
    config.database.path = Some(":memory:".to_string());

    let connection = config.database.database().await?;

    konarr::db::init(&connection.acquire().await).await?;

    let mut bill = BillOfMaterials::new(BomType::CycloneDX_1_6, "0.1.0".to_string());

    for i in 1..=count {
        bill.components.push(BomComponent::from_purl(
            format!("pkg:deb/debian/curl@0.{}.0", i).to_string(),
        ));
    }

    assert_eq!(bill.components.len(), count as usize);

    // Serialize the BOM to JSON bytes
    let bom_bytes = serde_json::to_vec(&bill)?;

    for project_id in 1..=count {
        let mut project = Projects::new(format!("test-{}", project_id), ProjectType::Container);
        project.save(&connection.acquire().await).await?;
        assert_eq!(project.id, project_id.into());

        let mut snapshot = Snapshot::new();
        snapshot.save(&connection.acquire().await).await?;
        snapshot.add_bom(&connection.acquire().await, bom_bytes.clone()).await?;

        project.add_snapshot(&connection.acquire().await, snapshot.clone()).await?;
        project.update(&connection.acquire().await).await?;
    }

    let total = Projects::total(&connection.acquire().await).await?;
    assert_eq!(total as i32, count);

    Ok(connection)
}

async fn test_alert_calculator(projects: i32) -> Result<(), KonarrError> {
    let connection = database(projects).await?;

    // Fetch all projects with their latest snapshots
    let mut projects = Projects::fetch_containers(&connection.acquire().await).await?;
    
    for project in projects.iter_mut() {
        project.fetch_latest_snapshot(&connection.acquire().await).await?;
    }

    alert_calculator(&connection.acquire().await, &mut projects).await?;

    Ok(())
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
