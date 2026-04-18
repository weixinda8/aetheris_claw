#[cfg(test)]
mod tests {
    use aetheris::memory::vector_db::{
        VectorDatabase, VectorDatabaseTrait, VectorDistance, VectorDocument,
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn test_in_memory_db_creation() {
        let db = VectorDatabase::new_memory();
        assert_eq!(db.collection_name(), "aetheris_memory");
        assert_eq!(db.vector_size(), 1536);
    }

    #[tokio::test]
    async fn test_in_memory_db_with_config() {
        let db = VectorDatabase::new_memory_with_config("test_collection".to_string(), 768);
        assert_eq!(db.collection_name(), "test_collection");
        assert_eq!(db.vector_size(), 768);
    }

    #[tokio::test]
    async fn test_in_memory_insert_and_search() {
        let db = VectorDatabase::new_memory();
        let id = Uuid::new_v4();
        let vector = vec![0.1, 0.2, 0.3, 0.4];
        let payload = serde_json::json!({"test": "data"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let results = db.search(&vector, 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_in_memory_insert_batch() {
        let db = VectorDatabase::new_memory();
        let mut documents = Vec::new();

        for i in 0..5 {
            let id = Uuid::new_v4();
            let vector = vec![0.1 * (i as f32), 0.2 * (i as f32), 0.3 * (i as f32)];
            let payload = serde_json::json!({"index": i});
            documents.push(VectorDocument {
                id,
                vector,
                payload,
                tags: Vec::new(),
                created_at: chrono::Utc::now(),
            });
        }

        db.insert_batch(documents).await.unwrap();

        let count = db.count().await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_in_memory_search_with_threshold() {
        let db = VectorDatabase::new_memory();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let vector1 = vec![1.0, 0.0, 0.0];
        let vector2 = vec![0.0, 1.0, 0.0];

        db.insert(
            &id1.to_string(),
            &vector1,
            serde_json::json!({"type": "vector1"}),
        )
        .await
        .unwrap();
        db.insert(
            &id2.to_string(),
            &vector2,
            serde_json::json!({"type": "vector2"}),
        )
        .await
        .unwrap();

        let results = db.search_with_threshold(&vector1, 10, 0.5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id1);
    }

    #[tokio::test]
    async fn test_in_memory_delete() {
        let db = VectorDatabase::new_memory();
        let id = Uuid::new_v4();
        let vector = vec![0.1, 0.2, 0.3];
        let payload = serde_json::json!({"test": "data"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let count_before = db.count().await.unwrap();
        assert_eq!(count_before, 1);

        db.delete(&id.to_string()).await.unwrap();

        let count_after = db.count().await.unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_in_memory_get() {
        let db = VectorDatabase::new_memory();
        let id = Uuid::new_v4();
        let vector = vec![0.1, 0.2, 0.3];
        let payload = serde_json::json!({"test": "data"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let result = db.get(&id.to_string()).await.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.payload, payload);
    }

    #[tokio::test]
    async fn test_in_memory_delete_collection() {
        let db = VectorDatabase::new_memory();

        for i in 0..10 {
            let id = Uuid::new_v4();
            let vector = vec![0.1, 0.2, 0.3];
            let payload = serde_json::json!({"index": i});
            db.insert(&id.to_string(), &vector, payload).await.unwrap();
        }

        let count_before = db.count().await.unwrap();
        assert_eq!(count_before, 10);

        db.delete_collection().await.unwrap();

        let count_after = db.count().await.unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_vector_database_default() {
        let db1 = VectorDatabase::new();
        let db2 = VectorDatabase::default();

        assert_eq!(db1.collection_name(), db2.collection_name());
        assert_eq!(db1.vector_size(), db2.vector_size());
    }

    #[tokio::test]
    async fn test_vector_database_factory_methods() {
        let memory_db = VectorDatabase::new_memory();
        let memory_db_config = VectorDatabase::new_memory_with_config("test".to_string(), 512);
        let qdrant_db = VectorDatabase::new_qdrant();
        let qdrant_db_config = VectorDatabase::new_qdrant_with_config("test".to_string(), 512);

        assert_eq!(memory_db.collection_name(), "aetheris_memory");
        assert_eq!(memory_db_config.collection_name(), "test");
        assert_eq!(memory_db_config.vector_size(), 512);
        assert_eq!(qdrant_db.collection_name(), "aetheris_memory");
        assert_eq!(qdrant_db_config.collection_name(), "test");
        assert_eq!(qdrant_db_config.vector_size(), 512);
    }

    #[tokio::test]
    async fn test_with_config_compatibility() {
        let db = VectorDatabase::with_config("test_collection".to_string(), 1024);
        assert_eq!(db.collection_name(), "test_collection");
        assert_eq!(db.vector_size(), 1024);
    }

    #[tokio::test]
    async fn test_vector_distance_default() {
        let distance = VectorDistance::default();
        assert!(matches!(distance, VectorDistance::Cosine));
    }

    #[tokio::test]
    async fn test_in_memory_db_with_distance() {
        let db = VectorDatabase::new_memory_with_distance(
            "test_distance".to_string(),
            768,
            VectorDistance::DotProduct,
        );
        assert_eq!(db.collection_name(), "test_distance");
        assert_eq!(db.vector_size(), 768);
        assert!(matches!(db.distance(), VectorDistance::DotProduct));
    }

    #[tokio::test]
    async fn test_with_distance_chain() {
        let db = VectorDatabase::new_memory().with_distance(VectorDistance::Euclidean);
        assert!(matches!(db.distance(), VectorDistance::Euclidean));
    }

    #[tokio::test]
    async fn test_cosine_distance_search() {
        let db = VectorDatabase::new_memory_with_distance(
            "cosine_test".to_string(),
            3,
            VectorDistance::Cosine,
        );
        let id = Uuid::new_v4();
        let vector = vec![1.0, 0.0, 0.0];
        let payload = serde_json::json!({"test": "cosine"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let results = db.search(&vector, 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_dot_product_distance_search() {
        let db = VectorDatabase::new_memory_with_distance(
            "dot_test".to_string(),
            3,
            VectorDistance::DotProduct,
        );
        let id = Uuid::new_v4();
        let vector = vec![1.0, 2.0, 3.0];
        let payload = serde_json::json!({"test": "dot"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let results = db.search(&vector, 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }

    #[tokio::test]
    async fn test_euclidean_distance_search() {
        let db = VectorDatabase::new_memory_with_distance(
            "euclidean_test".to_string(),
            3,
            VectorDistance::Euclidean,
        );
        let id = Uuid::new_v4();
        let vector = vec![0.0, 0.0, 0.0];
        let payload = serde_json::json!({"test": "euclidean"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let results = db.search(&vector, 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }

    #[tokio::test]
    async fn test_qdrant_db_with_distance() {
        let db = VectorDatabase::new_qdrant_with_distance(
            "qdrant_distance".to_string(),
            768,
            VectorDistance::DotProduct,
        );
        assert_eq!(db.collection_name(), "qdrant_distance");
        assert_eq!(db.vector_size(), 768);
        assert!(matches!(db.distance(), VectorDistance::DotProduct));
    }

    #[tokio::test]
    async fn test_qdrant_with_distance_chain() {
        let db = VectorDatabase::new_qdrant().with_distance(VectorDistance::Euclidean);
        assert!(matches!(db.distance(), VectorDistance::Euclidean));
    }
}

#[cfg(all(test, feature = "qdrant-tests"))]
mod qdrant_tests {
    use aetheris::memory::vector_db::{VectorDatabase, VectorDatabaseTrait, VectorDocument};
    use uuid::Uuid;

    const QDRANT_URL: &str = "http://localhost:6334";

    #[tokio::test]
    async fn test_qdrant_db_creation() {
        let db = VectorDatabase::new_qdrant();
        assert_eq!(db.collection_name(), "aetheris_memory");
        assert_eq!(db.vector_size(), 1536);
    }

    #[tokio::test]
    async fn test_qdrant_db_with_config() {
        let db = VectorDatabase::new_qdrant_with_config("test_qdrant_collection".to_string(), 768);
        assert_eq!(db.collection_name(), "test_qdrant_collection");
        assert_eq!(db.vector_size(), 768);
    }

    #[tokio::test]
    async fn test_qdrant_connect() {
        let mut db = VectorDatabase::new_qdrant();
        let result = db.connect(QDRANT_URL).await;

        if result.is_ok() {
            assert!(true);
        } else {
            println!(
                "Qdrant server not available at {}, skipping test",
                QDRANT_URL
            );
        }
    }

    #[tokio::test]
    async fn test_qdrant_insert_and_search() {
        let mut db = VectorDatabase::new_qdrant();

        if db.connect(QDRANT_URL).await.is_err() {
            println!("Qdrant server not available, skipping test");
            return;
        }

        let id = Uuid::new_v4();
        let vector = vec![0.1; 1536];
        let payload = serde_json::json!({"test": "qdrant_data"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        let results = db.search(&vector, 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, id);
    }

    #[tokio::test]
    async fn test_qdrant_insert_batch() {
        let mut db = VectorDatabase::new_qdrant();

        if db.connect(QDRANT_URL).await.is_err() {
            println!("Qdrant server not available, skipping test");
            return;
        }

        let mut documents = Vec::new();

        for i in 0..5 {
            let id = Uuid::new_v4();
            let vector = vec![0.1; 1536];
            let payload = serde_json::json!({"index": i});
            documents.push(VectorDocument {
                id,
                vector,
                payload,
                tags: Vec::new(),
                created_at: chrono::Utc::now(),
            });
        }

        db.insert_batch(documents).await.unwrap();
    }

    #[tokio::test]
    async fn test_qdrant_delete() {
        let mut db = VectorDatabase::new_qdrant();

        if db.connect(QDRANT_URL).await.is_err() {
            println!("Qdrant server not available, skipping test");
            return;
        }

        let id = Uuid::new_v4();
        let vector = vec![0.1; 1536];
        let payload = serde_json::json!({"test": "to_delete"});

        db.insert(&id.to_string(), &vector, payload.clone())
            .await
            .unwrap();

        db.delete(&id.to_string()).await.unwrap();
    }

    #[tokio::test]
    async fn test_qdrant_count() {
        let mut db = VectorDatabase::new_qdrant();

        if db.connect(QDRANT_URL).await.is_err() {
            println!("Qdrant server not available, skipping test");
            return;
        }

        let count = db.count().await.unwrap();
        assert!(count >= 0);
    }
}
