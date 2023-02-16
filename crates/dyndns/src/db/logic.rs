use crate::db::crud::domain_records::*;
use crate::db::types::*;
use crate::types::{IpAddrKind, IpAddrV4AndV6};
use chrono::NaiveDateTime;
use color_eyre::eyre::{eyre, Error, Result};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use tracing::trace;

pub fn handle_ip_fetch(
    conn: &mut SqliteConnection,
    maybe_fetched_ips: Option<IpAddrV4AndV6>,
) -> Result<(Option<DomainIpFetch>, NaiveDateTime)> {
    use super::crud::domain_ip_fetches::create_domain_ip_fetch;

    trace!("Recording IP fetch result in DB: {:?}", maybe_fetched_ips);

    let current_date = chrono::offset::Utc::now().naive_utc();
    let domain_ip_fetch = conn.transaction::<_, Error, _>(|conn| {
        let is_ip_fetch_successful = maybe_fetched_ips.is_some();
        let domain_ip_fetch = create_domain_ip_fetch(
            conn,
            &NewDomainIpFetch {
                attempt_date: current_date,
                success: is_ip_fetch_successful,
                fetched_ipv4: maybe_fetched_ips
                    .clone()
                    .and_then(|ips| ips.to_ipv4_string()),
                fetched_ipv6: maybe_fetched_ips
                    .clone()
                    .and_then(|ips| ips.to_ipv6_string()),
            },
        )?;

        create_domain_ip_last_fetch(conn, current_date, maybe_fetched_ips.clone())?;
        Ok(domain_ip_fetch)
    })?;

    Ok((domain_ip_fetch, current_date))
}

pub fn handle_updater_attempt(
    conn: &mut SqliteConnection,
    domain_record_name: &str,
    record_type: &str,
    domain_ip_fetch: &DomainIpFetch,
    attempt_date: NaiveDateTime,
    is_domain_record_update_successful: bool,
    ip_kind: Option<IpAddrKind>,
) -> Result<()> {
    let new_domain_record = NewDomainRecord {
        name: domain_record_name,
        record_type,
    };

    trace!(
        "Creating / updating domain record in the DB {:?}",
        new_domain_record
    );
    let domain_record = create_domain_record(conn, &new_domain_record)?.ok_or_else(|| {
        eyre!(
            "No domain record could be found for {}:{}",
            domain_record_name,
            record_type
        )
    })?;
    create_updater_attempt(
        conn,
        &domain_record,
        domain_ip_fetch,
        attempt_date,
        is_domain_record_update_successful,
        ip_kind,
    )?;
    Ok(())
}

fn create_domain_ip_last_fetch(
    conn: &mut SqliteConnection,
    current_date: chrono::NaiveDateTime,
    maybe_fetched_ips: Option<IpAddrV4AndV6>,
) -> Result<()> {
    use super::crud::domain_ip_last_fetches::create_domain_ip_last_fetch;
    use super::crud::domain_ip_last_fetches::get_domain_ip_last_fetch;

    let is_ip_fetch_successful = maybe_fetched_ips.is_some();
    let fetch_success_count_delta = i64::from(is_ip_fetch_successful);
    let fetch_fail_count_delta = i64::from(!is_ip_fetch_successful);

    let mut new_domain_ip_last_fetch = NewDomainIpLastFetch {
        attempt_count: 1,
        success_count: fetch_success_count_delta,
        fail_count: fetch_fail_count_delta,
        last_attempt_date: current_date,
        last_success_date: {
            if is_ip_fetch_successful {
                Some(current_date)
            } else {
                None
            }
        },
        last_successful_fetched_ipv4: {
            if is_ip_fetch_successful {
                maybe_fetched_ips
                    .clone()
                    .and_then(|ips| ips.to_ipv4_string())
            } else {
                None
            }
        },
        last_successful_fetched_ipv4_change_date: current_date,
        last_successful_fetched_ipv6: {
            if is_ip_fetch_successful {
                maybe_fetched_ips
                    .clone()
                    .and_then(|ips| ips.to_ipv6_string())
            } else {
                None
            }
        },
        last_successful_fetched_ipv6_change_date: current_date,
    };
    let domain_ip_last_fetch = get_domain_ip_last_fetch(conn)?;
    if let Some(domain_ip_last_fetch) = domain_ip_last_fetch {
        new_domain_ip_last_fetch = NewDomainIpLastFetch {
            attempt_count: domain_ip_last_fetch.attempt_count + 1,
            success_count: domain_ip_last_fetch.success_count + fetch_success_count_delta,
            fail_count: domain_ip_last_fetch.fail_count + fetch_fail_count_delta,
            last_success_date: {
                if is_ip_fetch_successful {
                    Some(current_date)
                } else {
                    domain_ip_last_fetch.last_success_date
                }
            },
            last_successful_fetched_ipv4: {
                if is_ip_fetch_successful {
                    maybe_fetched_ips
                        .clone()
                        .and_then(|ips| ips.to_ipv4_string())
                } else {
                    domain_ip_last_fetch.last_successful_fetched_ipv4.clone()
                }
            },
            last_successful_fetched_ipv4_change_date: {
                match (
                    maybe_fetched_ips
                        .clone()
                        .and_then(|ips| ips.to_ipv4_string()),
                    domain_ip_last_fetch.last_successful_fetched_ipv4,
                ) {
                    (Some(fetched_ip), Some(last_fetched_ip)) => {
                        if fetched_ip != last_fetched_ip {
                            current_date
                        } else {
                            domain_ip_last_fetch.last_successful_fetched_ipv4_change_date
                        }
                    }
                    _ => domain_ip_last_fetch.last_successful_fetched_ipv4_change_date,
                }
            },
            last_successful_fetched_ipv6: {
                if is_ip_fetch_successful {
                    maybe_fetched_ips
                        .clone()
                        .and_then(|ips| ips.to_ipv6_string())
                } else {
                    domain_ip_last_fetch.last_successful_fetched_ipv6.clone()
                }
            },
            last_successful_fetched_ipv6_change_date: {
                match (
                    maybe_fetched_ips.and_then(|ips| ips.to_ipv6_string()),
                    domain_ip_last_fetch.last_successful_fetched_ipv6,
                ) {
                    (Some(fetched_ip), Some(last_fetched_ip)) => {
                        if fetched_ip != last_fetched_ip {
                            current_date
                        } else {
                            domain_ip_last_fetch.last_successful_fetched_ipv6_change_date
                        }
                    }
                    _ => domain_ip_last_fetch.last_successful_fetched_ipv6_change_date,
                }
            },
            ..new_domain_ip_last_fetch
        };
    }
    create_domain_ip_last_fetch(conn, &new_domain_ip_last_fetch)?;
    Ok(())
}

fn create_domain_record_last_update(
    conn: &mut SqliteConnection,
    current_date: chrono::NaiveDateTime,
    domain_record: &DomainRecord,
    set_ip: String,
    is_domain_record_update_successful: bool,
) -> Result<()> {
    use super::crud::domain_record_last_updates::create_domain_record_last_update;
    use super::crud::domain_record_last_updates::get_domain_record_last_update_by_domain_record_id;

    let fetch_success_count_delta = i64::from(is_domain_record_update_successful);
    let fetch_fail_count_delta = i64::from(!is_domain_record_update_successful);

    let mut new_domain_record_last_update = NewDomainRecordLastUpdate {
        domain_record_id: domain_record.id,
        attempt_count: 1,
        success_count: fetch_success_count_delta,
        fail_count: fetch_fail_count_delta,
        last_attempt_date: current_date,
        last_success_date: {
            if is_domain_record_update_successful {
                Some(current_date)
            } else {
                None
            }
        },
        last_set_ip: set_ip,
    };
    let domain_record_last_update =
        get_domain_record_last_update_by_domain_record_id(conn, domain_record.id)?;
    if let Some(domain_ip_last_fetch) = domain_record_last_update {
        new_domain_record_last_update = NewDomainRecordLastUpdate {
            attempt_count: domain_ip_last_fetch.attempt_count + 1,
            success_count: domain_ip_last_fetch.success_count + fetch_success_count_delta,
            fail_count: domain_ip_last_fetch.fail_count + fetch_fail_count_delta,
            last_success_date: {
                if is_domain_record_update_successful {
                    Some(current_date)
                } else {
                    domain_ip_last_fetch.last_success_date
                }
            },
            ..new_domain_record_last_update
        };
    }
    create_domain_record_last_update(conn, &new_domain_record_last_update)?;
    Ok(())
}

pub fn create_updater_attempt(
    conn: &mut SqliteConnection,
    domain_record: &DomainRecord,
    domain_ip_fetch: &DomainIpFetch,
    attempt_date: NaiveDateTime,
    is_domain_record_update_successful: bool,
    ip_kind: Option<IpAddrKind>,
) -> Result<()> {
    use super::crud::domain_record_updates::create_domain_record_update;
    use super::crud::updater_attempts::create_updater_attempt;

    conn.transaction(|conn| {
        let maybe_fetched_ip = ip_kind.and_then(|ip_kind| match ip_kind {
            IpAddrKind::V4 => domain_ip_fetch.fetched_ipv4.as_ref(),
            IpAddrKind::V6 => domain_ip_fetch.fetched_ipv6.as_ref(),
        });

        let is_ip_fetch_successful = maybe_fetched_ip.is_some();

        let domain_record_update = {
            if is_ip_fetch_successful {
                let domain_update_record = create_domain_record_update(
                    conn,
                    &NewDomainRecordUpdate {
                        domain_record_id: domain_record.id,
                        set_ip: maybe_fetched_ip
                            .expect("ip should exist at this point")
                            .to_owned(),
                        attempt_date,
                        success: is_domain_record_update_successful,
                    },
                )?
                .map(|fetch| fetch.id);

                create_domain_record_last_update(
                    conn,
                    attempt_date,
                    domain_record,
                    maybe_fetched_ip
                        .expect("ip should exist at this point")
                        .to_owned(),
                    is_domain_record_update_successful,
                )?;
                domain_update_record
            } else {
                None
            }
        };

        let updater_attempt = create_updater_attempt(
            conn,
            &NewUpdaterAttempt {
                domain_record_id: domain_record.id,
                domain_ip_fetches_id: domain_ip_fetch.id,
                domain_record_updates_id: domain_record_update,
                attempt_date,
            },
        )?;

        trace!("Inserted updater attempt into DB {:?}", updater_attempt);

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::crud::updater_attempts::get_updater_attempts;
    use crate::db::setup::setup_db;

    #[test]
    fn test_do_ops_with_db() -> Result<()> {
        let maybe_db_path = None;
        let conn = &mut setup_db(maybe_db_path)?;
        let domain_record = create_domain_record(
            conn,
            &NewDomainRecord {
                name: "foos",
                record_type: "A",
            },
        )?
        .unwrap();

        let maybe_fetched_ips = Some(IpAddrV4AndV6 {
            ipv4: Some("127.0.1.2".parse().expect("valid ip")),
            ipv6: None,
        });
        let (maybe_domain_ip_fetch, attempt_date) = handle_ip_fetch(conn, maybe_fetched_ips)?;
        let domain_ip_fetch =
            maybe_domain_ip_fetch.ok_or_else(|| eyre!("No domain ip fetch entry found"))?;

        create_updater_attempt(
            conn,
            &domain_record,
            &domain_ip_fetch,
            attempt_date,
            true,
            Some(IpAddrKind::V4),
        )?;
        get_domain_records(conn)?;
        get_updater_attempts(conn)?;
        handle_updater_attempt(
            conn,
            "foos",
            "CNAME",
            &domain_ip_fetch,
            attempt_date,
            true,
            Some(IpAddrKind::V4),
        )?;
        Ok(())
    }
}
