use activity_api::activities::parser::parse_csv_row;
use uuid::Uuid;

const USER_ID: &str = "123e4567-e89b-12d3-a456-426614174000";

fn user_id() -> Uuid {
    Uuid::parse_str(USER_ID).unwrap()
}

#[test]
fn test_parse_csv_row_valid() {
    let id = Uuid::new_v4();
    let csv = format!(
        "{},2025-05-05 17:06:59,Run,Running,6.14,32:09,5.14,11.46,700.0,94,nil,nil,nil,test.gpx",
        id
    );
    let result = parse_csv_row(&csv, user_id());
    assert!(result.is_ok());
    let activity = result.unwrap();
    assert_eq!(activity.id, id);
    assert_eq!(activity.gps_file, "test.gpx");
}

#[test]
fn test_parse_csv_row_wrong_column_count() {
    let result = parse_csv_row("invalid,csv,line", user_id());
    assert!(result.is_err());
}

#[test]
fn test_parse_csv_row_invalid_uuid() {
    let row = "not-a-uuid,2025-05-05 17:06:59,Run,Running,6.14,32:09,5.14,11.46,700.0,94,nil,nil,nil,test.gpx";
    let result = parse_csv_row(row, user_id());
    assert!(result.is_err());
}

#[test]
fn test_parse_csv_row_invalid_number() {
    let id = Uuid::new_v4();
    let row = format!(
        "{},2025-05-05 17:06:59,Run,Running,not-a-number,32:09,5.14,11.46,700.0,94,nil,nil,nil,test.gpx",
        id
    );
    let result = parse_csv_row(&row, user_id());
    assert!(result.is_err());
}
