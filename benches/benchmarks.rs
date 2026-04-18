use aetheris::config::AppConfig;
use aetheris::constants::*;
use aetheris::digital_twin::{TwinEntity, TwinEntityType, TwinModel, TwinState, TwinStateUpdate};
use aetheris::edge_coordination::GlobalCoordinator;
use aetheris::skill::Version;
use chrono::Utc;
use criterion::{Criterion, criterion_group, criterion_main};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;

fn bench_config_loading(c: &mut Criterion) {
    c.bench_function("config_loading", |b| {
        b.iter(|| {
            let _config = AppConfig::load();
        });
    });
}

fn bench_version_parsing(c: &mut Criterion) {
    c.bench_function("version_parsing", |b| {
        b.iter(|| {
            let _version = Version::from_string("1.2.3").unwrap();
        });
    });
}

fn bench_version_comparison(c: &mut Criterion) {
    let v1 = Version::new(1, 0, 0);
    let v2 = Version::new(1, 1, 0);

    c.bench_function("version_comparison", |b| {
        b.iter(|| {
            let _ = v1 < v2;
        });
    });
}

fn bench_dashmap_insert(c: &mut Criterion) {
    c.bench_function("dashmap_insert", |b| {
        b.iter(|| {
            let map = DashMap::new();
            for i in 0..100 {
                map.insert(i, format!("value{}", i));
            }
        });
    });
}

fn bench_hashmap_insert(c: &mut Criterion) {
    c.bench_function("hashmap_insert", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for i in 0..100 {
                map.insert(i, format!("value{}", i));
            }
        });
    });
}

fn bench_dashmap_get(c: &mut Criterion) {
    let map = DashMap::new();
    for i in 0..100 {
        map.insert(i, format!("value{}", i));
    }

    c.bench_function("dashmap_get", |b| {
        b.iter(|| {
            for i in 0..100 {
                let _ = map.get(&i);
            }
        });
    });
}

fn bench_hashmap_get(c: &mut Criterion) {
    let mut map = HashMap::new();
    for i in 0..100 {
        map.insert(i, format!("value{}", i));
    }

    c.bench_function("hashmap_get", |b| {
        b.iter(|| {
            for i in 0..100 {
                let _ = map.get(&i);
            }
        });
    });
}

fn bench_skill_priority_operations(c: &mut Criterion) {
    use aetheris::skill::SkillPriority;

    c.bench_function("skill_priority_operations", |b| {
        b.iter(|| {
            let priorities = vec![
                SkillPriority::Mandatory,
                SkillPriority::High,
                SkillPriority::Medium,
                SkillPriority::Low,
                SkillPriority::OnDemand,
                SkillPriority::Disabled,
            ];

            for priority in priorities {
                let _ = priority.should_load();
                let _ = priority.should_preload();
                let _ = priority.is_lazy_load();
                let _ = priority.as_str();
                let _ = priority.as_u8();
            }
        });
    });
}

fn bench_constant_access(c: &mut Criterion) {
    c.bench_function("constant_access", |b| {
        b.iter(|| {
            let _ = DEFAULT_SERVER_HOST;
            let _ = DEFAULT_SERVER_PORT;
            let _ = DEFAULT_JWT_EXPIRATION_HOURS;
            let _ = DEFAULT_LLM_PROVIDER;
            let _ = CACHE_DEFAULT_TTL_SECONDS;
            let _ = ONE_SECOND_MILLIS;
            let _ = ONE_KB_BYTES;
        });
    });
}

fn bench_twin_model_entity_operations(c: &mut Criterion) {
    let model = TwinModel::new();

    c.bench_function("twin_model_add_entity", |b| {
        b.iter(|| {
            let entity = TwinEntity {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Test Entity".to_string(),
                entity_type: TwinEntityType::Device,
                properties: HashMap::new(),
                state: TwinState::Online,
                parent_id: None,
                children_ids: Vec::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            model.add_entity(entity);
        });
    });
}

fn bench_twin_model_state_update(c: &mut Criterion) {
    let model = TwinModel::new();
    let entity = TwinEntity {
        id: "test-entity".to_string(),
        name: "Test Entity".to_string(),
        entity_type: TwinEntityType::Device,
        properties: HashMap::new(),
        state: TwinState::Online,
        parent_id: None,
        children_ids: Vec::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    model.add_entity(entity);

    c.bench_function("twin_model_update_state", |b| {
        b.iter(|| {
            let update = TwinStateUpdate {
                entity_id: "test-entity".to_string(),
                state: TwinState::Degraded,
                properties: None,
                timestamp: Utc::now(),
                source: "benchmark".to_string(),
            };
            let _ = model.update_state(update);
        });
    });
}

fn bench_global_coordinator_node_operations(c: &mut Criterion) {
    use aetheris::edge_coordination::{EdgeNode, NodeStatus, NodeType};

    let coordinator = GlobalCoordinator::new();

    c.bench_function("global_coordinator_register_node", |b| {
        b.iter(|| {
            let node = EdgeNode {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Test Node".to_string(),
                node_type: NodeType::Edge,
                status: NodeStatus::Online,
                resource_usage: None,
                last_seen: Utc::now(),
                capabilities: Vec::new(),
                location: None,
            };
            coordinator.register_node(node);
        });
    });
}

fn bench_list_operations(c: &mut Criterion) {
    use aetheris::edge_coordination::{EdgeNode, NodeStatus, NodeType};

    let model = TwinModel::new();
    for i in 0..100 {
        let entity = TwinEntity {
            id: format!("entity-{}", i),
            name: format!("Entity {}", i),
            entity_type: TwinEntityType::Device,
            properties: HashMap::new(),
            state: TwinState::Online,
            parent_id: None,
            children_ids: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        model.add_entity(entity);
    }

    let coordinator = GlobalCoordinator::new();
    for i in 0..100 {
        let node = EdgeNode {
            id: format!("node-{}", i),
            name: format!("Node {}", i),
            node_type: NodeType::Edge,
            status: NodeStatus::Online,
            resource_usage: None,
            last_seen: Utc::now(),
            capabilities: Vec::new(),
            location: None,
        };
        coordinator.register_node(node);
    }

    c.bench_function("twin_model_list_entities", |b| {
        b.iter(|| {
            let _ = model.list_entities();
        });
    });

    c.bench_function("global_coordinator_list_nodes", |b| {
        b.iter(|| {
            let _ = coordinator.list_nodes();
        });
    });
}

criterion_group!(
    benches,
    bench_config_loading,
    bench_version_parsing,
    bench_version_comparison,
    bench_dashmap_insert,
    bench_hashmap_insert,
    bench_dashmap_get,
    bench_hashmap_get,
    bench_skill_priority_operations,
    bench_constant_access,
    bench_twin_model_entity_operations,
    bench_twin_model_state_update,
    bench_global_coordinator_node_operations,
    bench_list_operations
);
criterion_main!(benches);
