// @generated automatically by Diesel CLI.

diesel::table! {
    use crate::db::sqlite_mapping::*;

    domain_ip_fetches (id) {
        id -> Integer,
        attempt_date -> Timestamp,
        success -> Bool,
        fetched_ipv4 -> Nullable<Text>,
        fetched_ipv6 -> Nullable<Text>,
    }
}

diesel::table! {
    use crate::db::sqlite_mapping::*;

    domain_ip_last_fetches (id) {
        id -> Integer,
        attempt_count -> Integer,
        success_count -> Integer,
        fail_count -> Integer,
        last_attempt_date -> Timestamp,
        last_success_date -> Nullable<Timestamp>,
        last_successful_fetched_ipv4 -> Nullable<Text>,
        last_successful_fetched_ipv4_change_date -> Timestamp,
        last_successful_fetched_ipv6 -> Nullable<Text>,
        last_successful_fetched_ipv6_change_date -> Timestamp,
    }
}

diesel::table! {
    use crate::db::sqlite_mapping::*;

    domain_record_last_updates (id) {
        id -> Integer,
        domain_record_id -> Integer,
        attempt_count -> Integer,
        success_count -> Integer,
        fail_count -> Integer,
        last_attempt_date -> Timestamp,
        last_success_date -> Nullable<Timestamp>,
        last_set_ip -> Text,
    }
}

diesel::table! {
    use crate::db::sqlite_mapping::*;

    domain_record_updates (id) {
        id -> Integer,
        domain_record_id -> Integer,
        set_ip -> Text,
        attempt_date -> Timestamp,
        success -> Bool,
    }
}

diesel::table! {
    use crate::db::sqlite_mapping::*;

    domain_records (id) {
        id -> Integer,
        name -> Text,
        record_type -> Text,
    }
}

diesel::table! {
    use crate::db::sqlite_mapping::*;

    info (id) {
        id -> Integer,
        name -> Text,
        value -> Text,
    }
}

diesel::table! {
    use crate::db::sqlite_mapping::*;

    updater_attempts (id) {
        id -> Integer,
        domain_record_id -> Integer,
        domain_ip_fetches_id -> Integer,
        domain_record_updates_id -> Nullable<Integer>,
        attempt_date -> Timestamp,
    }
}

diesel::joinable!(domain_record_last_updates -> domain_records (domain_record_id));
diesel::joinable!(domain_record_updates -> domain_records (domain_record_id));
diesel::joinable!(updater_attempts -> domain_ip_fetches (domain_ip_fetches_id));
diesel::joinable!(updater_attempts -> domain_record_updates (domain_record_updates_id));
diesel::joinable!(updater_attempts -> domain_records (domain_record_id));

diesel::allow_tables_to_appear_in_same_query!(
    domain_ip_fetches,
    domain_ip_last_fetches,
    domain_record_last_updates,
    domain_record_updates,
    domain_records,
    info,
    updater_attempts,
);
