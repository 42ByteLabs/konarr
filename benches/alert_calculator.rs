use criterion::async_executor::FuturesExecutor;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use geekorm::prelude::*;
use konarr::tasks::alert_calculator;
use std::sync::Arc;
use tokio::sync::Mutex;

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

async fn database(count: i32) -> Result<Arc<Mutex<libsql::Connection>>, KonarrError> {
    let mut config = Config::default();
    config.database.path = Some(":memory:".to_string());

    let conn = config.database.connection().await?;
    let connection = Arc::new(Mutex::new(conn));

    konarr::db::init(&connection).await?;

    let mut bill = BillOfMaterials::new(BomType::CycloneDX_1_6, "0.1.0".to_string());

    for i in 1..=count {
        bill.components.push(BomComponent::from_purl(
            format!("pkg:deb/debian/curl@0.{}.0", i).to_string(),
        ));
    }

    assert_eq!(bill.components.len(), count as usize);

    for project_id in 1..=count {
        let mut project = Projects::new(format!("test-{}", project_id), ProjectType::Container);
        project.save(&connection).await?;
        assert_eq!(project.id, project_id.into());

        let mut snapshot = Snapshot::new();
        snapshot.add_bom(&connection, &bill).await?;
        snapshot.fetch_or_create(&connection).await?;

        project.add_snapshot(&connection, snapshot).await?;
        project.update(&connection).await?;
    }

    let total = Projects::total(&connection).await?;
    assert_eq!(total as i32, count);

    Ok(connection)
}

async fn test_alert_calculator(projects: i32) -> Result<(), crate::KonarrError> {
    let connection = database(projects).await?;

    alert_calculator(&connection).await?;

    Ok(())
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
