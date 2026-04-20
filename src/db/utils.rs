use std::cmp::Ordering;

use crate::models::Tag;

/// Sorting function for a list of tags associated with a task
///
/// Order:
/// * category ascending, null-first
/// * label ascending
/// * updated_at descending (if label is equal)
/// * id ascending (if label and updated_at are equal)
pub fn order_task_tag(a: &Tag, b: &Tag) -> Ordering {
    a.category
        .cmp(&b.category) // order ascending (None before Some)
        .then_with(|| a.label.cmp(&b.label)) // label ascending
        .then_with(|| b.updated_at.cmp(&a.updated_at)) // updated_at descending
        .then_with(|| a.id.cmp(&b.id)) // id ascending
}

#[cfg(test)]
mod order_task_tag {
    use std::cmp::Ordering;

    use chrono::{Duration, TimeZone, Utc};
    use uuid::Uuid;

    use super::*;

    fn make_tag(
        id: u128,
        label: &str,
        category: Option<&str>,
        updated_at: chrono::DateTime<Utc>,
    ) -> Tag {
        Tag {
            id: Uuid::from_u128(id),
            label: label.to_string(),
            category: category.map(str::to_string),
            created_at: Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
            updated_at,
            created_by: Uuid::nil(),
        }
    }

    #[test]
    fn category_is_sorted_ascending() {
        let t = Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap();

        let a = make_tag(1, "same", Some("alpha"), t);
        let b = make_tag(2, "same", Some("beta"), t);

        assert_eq!(order_task_tag(&a, &b), Ordering::Less);
        assert_eq!(order_task_tag(&b, &a), Ordering::Greater);
    }

    #[test]
    fn category_is_null_first() {
        let t = Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap();

        let a = make_tag(1, "same", None, t);
        let b = make_tag(2, "same", Some("work"), t);

        assert_eq!(order_task_tag(&a, &b), Ordering::Less);
        assert_eq!(order_task_tag(&b, &a), Ordering::Greater);
    }

    #[test]
    fn label_is_sorted_ascending_when_category_matches() {
        let t = Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap();

        let a = make_tag(1, "apple", Some("work"), t);
        let b = make_tag(2, "banana", Some("work"), t);

        assert_eq!(order_task_tag(&a, &b), Ordering::Less);
        assert_eq!(order_task_tag(&b, &a), Ordering::Greater);
    }

    #[test]
    fn updated_at_is_sorted_descending_when_category_and_label_match() {
        let base = Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap();

        let older = make_tag(1, "apple", Some("work"), base);
        let newer = make_tag(2, "apple", Some("work"), base + Duration::hours(1));

        assert_eq!(order_task_tag(&newer, &older), Ordering::Less);
        assert_eq!(order_task_tag(&older, &newer), Ordering::Greater);
    }

    #[test]
    fn id_is_sorted_ascending_when_other_fields_match() {
        let t = Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap();

        let a = make_tag(1, "apple", Some("work"), t);
        let b = make_tag(2, "apple", Some("work"), t);

        assert_eq!(order_task_tag(&a, &b), Ordering::Less);
        assert_eq!(order_task_tag(&b, &a), Ordering::Greater);
    }

    #[test]
    fn equal_when_all_sort_fields_match() {
        let t = Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap();

        let a = make_tag(1, "apple", Some("work"), t);
        let b = make_tag(1, "apple", Some("work"), t);

        assert_eq!(order_task_tag(&a, &b), Ordering::Equal);
        assert_eq!(order_task_tag(&b, &a), Ordering::Equal);
    }

    #[test]
    fn sorting_full_list_matches_expected_order() {
        let base = Utc.with_ymd_and_hms(2026, 4, 20, 12, 0, 0).unwrap();

        let mut tags = vec![
            make_tag(5, "banana", Some("work"), base),
            make_tag(4, "apple", Some("work"), base),
            make_tag(3, "apple", Some("work"), base + Duration::hours(1)),
            make_tag(2, "apple", Some("home"), base),
            make_tag(1, "apple", None, base),
        ];

        tags.sort_by(order_task_tag);

        assert_eq!(tags[0].id, Uuid::from_u128(1)); // None category first
        assert_eq!(tags[1].id, Uuid::from_u128(2)); // home before work
        assert_eq!(tags[2].id, Uuid::from_u128(3)); // newer apple/work first
        assert_eq!(tags[3].id, Uuid::from_u128(4)); // older apple/work next
        assert_eq!(tags[4].id, Uuid::from_u128(5)); // banana/work last
    }
}
