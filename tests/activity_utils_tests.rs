use activity_api::activities::utils::get_activites_from_rows;
use uuid::Uuid;

#[tokio::test]
async fn test_get_activites_from_rows_parses_valid_lines() {
    let id = Uuid::new_v4();
    let csv = format!(
        "{},2025-05-05 17:06:59,Run,Running,6.14,32:09,5.14,11.46,700.0,94,nil,nil,nil,test.gpx",
        id
    );
    let rows = vec![csv];
    let result = get_activites_from_rows(rows).await;
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, id);
}

#[tokio::test]
async fn test_get_activites_from_rows_skips_invalid_lines() {
    let rows = vec![
        "invalid,csv,line".to_string(),
        "missing,fields,here".to_string(),
    ];
    let result = get_activites_from_rows(rows).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_empty_rows_returns_empty_vec() {
    let rows: Vec<String> = vec![];
    let result = get_activites_from_rows(rows).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_row_with_invalid_uuid_is_skipped() {
    let row = "not-a-uuid,2025-05-05 17:06:59,Run,Running,6.14,32:09,5.14,11.46,700.0,94,test.gpx"
        .to_string();
    let result = get_activites_from_rows(vec![row]).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_row_with_missing_fields_is_skipped() {
    let row = "123e4567-e89b-12d3-a456-426614174000,2025-05-05 17:06:59,Run".to_string();
    let result = get_activites_from_rows(vec![row]).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_row_with_invalid_number_format_is_skipped() {
    let row = "123e4567-e89b-12d3-a456-426614174000,2025-05-05 17:06:59,Run,Running,not-a-number,32:09,5.14,11.46,700.0,94,test.gpx".to_string();
    let result = get_activites_from_rows(vec![row]).await;
    assert!(result.is_empty());
}
