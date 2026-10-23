use crate::model::{NAME_MAX_CHARS, Profile, Settings};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("the name is empty")]
    NameEmpty,
    #[error("the name has more than {NAME_MAX_CHARS} characters")]
    NameTooLong,
    #[error("another profile is already called {0:?}")]
    NameTaken(String),
    #[error("the last profile cannot be deleted")]
    LastProfile,
    #[error("no profile {0}")]
    NotFound(String),
    #[error("the new order must list every profile exactly once")]
    OrderInvalid,
}

impl ProfileError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NameEmpty => "NAME_EMPTY",
            Self::NameTooLong => "NAME_TOO_LONG",
            Self::NameTaken(_) => "NAME_TAKEN",
            Self::LastProfile => "LAST_PROFILE",
            Self::NotFound(_) => "PROFILE_NOT_FOUND",
            Self::OrderInvalid => "ORDER_INVALID",
        }
    }
}

pub fn normalize_name(raw: &str) -> Result<String, ProfileError> {
    let printable: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let name = printable.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(ProfileError::NameEmpty);
    }
    if name.chars().count() > NAME_MAX_CHARS {
        return Err(ProfileError::NameTooLong);
    }
    Ok(name)
}

fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn taken(profiles: &[Profile], name: &str, except: Option<&str>) -> bool {
    profiles
        .iter()
        .any(|p| Some(p.id.as_str()) != except && same_name(&p.name, name))
}

fn base_of(name: &str) -> &str {
    if let Some(open) = name.rfind(" (")
        && let Some(n) = name[open + 2..].strip_suffix(')')
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
    {
        return &name[..open];
    }
    name
}

pub fn unique_name(base: &str, profiles: &[Profile], except: Option<&str>) -> String {
    if !taken(profiles, base, except) {
        return base.to_owned();
    }
    let stem = base_of(base);
    (2..)
        .map(|n| {
            let suffix = format!(" ({n})");
            let room = NAME_MAX_CHARS - suffix.chars().count();
            let trimmed: String = stem.chars().take(room).collect();
            format!("{}{suffix}", trimmed.trim_end())
        })
        .find(|candidate| !taken(profiles, candidate, except))
        .unwrap_or_else(|| base.to_owned())
}

fn index_of(profiles: &[Profile], id: &str) -> Result<usize, ProfileError> {
    profiles
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| ProfileError::NotFound(id.to_owned()))
}

fn insert_order(settings: &mut Settings, id: &str, after: Option<&str>) {
    let at = after
        .and_then(|a| settings.profile_order.iter().position(|o| o == a))
        .map_or(settings.profile_order.len(), |i| i + 1);
    settings.profile_order.insert(at, id.to_owned());
}

pub fn create(
    settings: &mut Settings,
    profiles: &mut Vec<Profile>,
    id: &str,
    base_name: &str,
    now: &str,
) -> Result<Profile, ProfileError> {
    let name = unique_name(&normalize_name(base_name)?, profiles, None);
    let profile = Profile::new_default(id, name, now);
    profiles.push(profile.clone());
    insert_order(settings, id, None);
    Ok(profile)
}

pub fn duplicate(
    settings: &mut Settings,
    profiles: &mut Vec<Profile>,
    source_id: &str,
    new_id: &str,
    now: &str,
) -> Result<Profile, ProfileError> {
    let source = profiles[index_of(profiles, source_id)?].clone();
    let mut copy = source.clone();
    copy.id = new_id.to_owned();
    copy.name = unique_name(&source.name, profiles, None);
    copy.hotkey = None;
    copy.created_at = now.to_owned();
    copy.updated_at = now.to_owned();
    profiles.push(copy.clone());
    insert_order(settings, new_id, Some(source_id));
    Ok(copy)
}

pub fn rename(
    profiles: &mut [Profile],
    id: &str,
    raw: &str,
    now: &str,
) -> Result<Profile, ProfileError> {
    let name = normalize_name(raw)?;
    if taken(profiles, &name, Some(id)) {
        return Err(ProfileError::NameTaken(name));
    }
    let i = index_of(profiles, id)?;
    let profile = &mut profiles[i];
    profile.name = name;
    profile.updated_at = now.to_owned();
    Ok(profile.clone())
}

pub fn delete(
    settings: &mut Settings,
    profiles: &mut Vec<Profile>,
    id: &str,
) -> Result<Profile, ProfileError> {
    let i = index_of(profiles, id)?;
    if profiles.len() == 1 {
        return Err(ProfileError::LastProfile);
    }
    let position = settings.profile_order.iter().position(|o| o == id);
    let removed = profiles.remove(i);
    settings.profile_order.retain(|o| o != id);
    if settings.active_profile_id.as_deref() == Some(id) {
        let next = position
            .and_then(|p| {
                settings
                    .profile_order
                    .get(p)
                    .or_else(|| settings.profile_order.last())
            })
            .cloned()
            .or_else(|| profiles.first().map(|p| p.id.clone()));
        settings.active_profile_id = next;
    }
    Ok(removed)
}

pub fn reorder(
    settings: &mut Settings,
    profiles: &[Profile],
    order: Vec<String>,
) -> Result<(), ProfileError> {
    let mut sorted = order.clone();
    sorted.sort();
    sorted.dedup();
    let mut ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    if sorted.len() != order.len() || sorted.iter().map(String::as_str).ne(ids.iter().copied()) {
        return Err(ProfileError::OrderInvalid);
    }
    settings.profile_order = order;
    Ok(())
}

pub fn activate(
    settings: &mut Settings,
    profiles: &[Profile],
    id: &str,
) -> Result<(), ProfileError> {
    index_of(profiles, id)?;
    settings.active_profile_id = Some(id.to_owned());
    Ok(())
}

pub fn ordered(settings: &Settings, profiles: &[Profile]) -> Vec<Profile> {
    let mut out: Vec<Profile> = settings
        .profile_order
        .iter()
        .filter_map(|id| profiles.iter().find(|p| p.id == *id))
        .cloned()
        .collect();
    out.extend(
        profiles
            .iter()
            .filter(|p| !settings.profile_order.contains(&p.id))
            .cloned(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Hotkey, Locale};

    const NOW: &str = "2026-10-05T00:00:00Z";

    fn library(names: &[&str]) -> (Settings, Vec<Profile>) {
        let profiles: Vec<Profile> = names
            .iter()
            .enumerate()
            .map(|(i, n)| Profile::new_default(format!("p{i}"), *n, NOW))
            .collect();
        let mut settings = Settings::new_default(Locale::PtBr);
        settings.profile_order = profiles.iter().map(|p| p.id.clone()).collect();
        settings.active_profile_id = Some("p0".into());
        (settings, profiles)
    }

    fn names(settings: &Settings, profiles: &[Profile]) -> Vec<String> {
        ordered(settings, profiles)
            .into_iter()
            .map(|p| p.name)
            .collect()
    }

    #[test]
    fn names_are_normalized_and_bounded() {
        assert_eq!(normalize_name("  Café   run "), Ok("Café run".into()));
        assert_eq!(normalize_name("   "), Err(ProfileError::NameEmpty));
        assert_eq!(normalize_name("A\tB\u{7}C\n"), Ok("A B C".into()));
        assert_eq!(
            normalize_name(&"á".repeat(48)).map(|n| n.chars().count()),
            Ok(48)
        );
        assert_eq!(
            normalize_name(&"a".repeat(49)),
            Err(ProfileError::NameTooLong)
        );
    }

    #[test]
    fn unique_names_count_up_and_fit() {
        let (_, profiles) = library(&["Coleta", "Coleta (2)", "Farm (x)"]);
        assert_eq!(unique_name("Novo", &profiles, None), "Novo");
        assert_eq!(unique_name("coleta", &profiles, None), "coleta (3)");
        assert_eq!(unique_name("Coleta (2)", &profiles, None), "Coleta (3)");
        assert_eq!(unique_name("Farm (x)", &profiles, None), "Farm (x) (2)");
        assert_eq!(unique_name("Coleta", &profiles, Some("p0")), "Coleta");
        let long = "a".repeat(48);
        let (_, crowded) = library(&[&long]);
        let fitted = unique_name(&long, &crowded, None);
        assert_eq!(fitted.chars().count(), 48);
        assert!(fitted.ends_with(" (2)"));
    }

    #[test]
    fn create_appends_without_activating() {
        let (mut s, mut p) = library(&["My profile"]);
        let created = create(&mut s, &mut p, "new", "My profile", NOW).expect("create");
        assert_eq!(created.name, "My profile (2)");
        assert_eq!(s.profile_order, ["p0", "new"]);
        assert_eq!(s.active_profile_id.as_deref(), Some("p0"));
        assert_eq!(
            create(&mut s, &mut p, "x", " ", NOW),
            Err(ProfileError::NameEmpty)
        );
    }

    #[test]
    fn duplicate_goes_below_the_original_without_its_shortcut() {
        let (mut s, mut p) = library(&["A", "B"]);
        p[0].hotkey = Some(Hotkey::new(&[], "F9"));
        let copy = duplicate(&mut s, &mut p, "p0", "copy", "2026-10-06T00:00:00Z").expect("dup");
        assert_eq!(copy.name, "A (2)");
        assert_eq!(copy.hotkey, None);
        assert_eq!(copy.created_at, "2026-10-06T00:00:00Z");
        assert_eq!(copy.click, p[0].click);
        assert_eq!(names(&s, &p), ["A", "A (2)", "B"]);
        let again = duplicate(&mut s, &mut p, "copy", "copy2", NOW).expect("dup");
        assert_eq!(again.name, "A (3)");
        assert!(matches!(
            duplicate(&mut s, &mut p, "nope", "z", NOW),
            Err(ProfileError::NotFound(_))
        ));
    }

    #[test]
    fn rename_refuses_taken_names() {
        let (_, mut p) = library(&["A", "B"]);
        assert_eq!(
            rename(&mut p, "p1", " a ", NOW),
            Err(ProfileError::NameTaken("a".into()))
        );
        let renamed = rename(&mut p, "p1", "Farm", "2026-10-07T00:00:00Z").expect("rename");
        assert_eq!(
            (renamed.name.as_str(), renamed.updated_at.as_str()),
            ("Farm", "2026-10-07T00:00:00Z")
        );
        assert_eq!(
            rename(&mut p, "p1", "FARM", NOW).map(|r| r.name),
            Ok("FARM".into())
        );
        assert!(matches!(
            rename(&mut p, "zz", "Q", NOW),
            Err(ProfileError::NotFound(_))
        ));
    }

    #[test]
    fn delete_moves_the_active_profile_and_keeps_one() {
        let (mut s, mut p) = library(&["A", "B", "C"]);
        delete(&mut s, &mut p, "p0").expect("delete active");
        assert_eq!(
            s.active_profile_id.as_deref(),
            Some("p1"),
            "next in the list"
        );
        s.active_profile_id = Some("p2".into());
        delete(&mut s, &mut p, "p2").expect("delete last in order");
        assert_eq!(
            s.active_profile_id.as_deref(),
            Some("p1"),
            "previous at the end"
        );
        assert_eq!(delete(&mut s, &mut p, "p1"), Err(ProfileError::LastProfile));
        assert_eq!(s.profile_order, ["p1"]);
    }

    #[test]
    fn deleting_an_inactive_profile_keeps_the_active_one() {
        let (mut s, mut p) = library(&["A", "B"]);
        delete(&mut s, &mut p, "p1").expect("delete");
        assert_eq!(s.active_profile_id.as_deref(), Some("p0"));
        assert!(matches!(
            delete(&mut s, &mut p, "p9"),
            Err(ProfileError::NotFound(_))
        ));
    }

    #[test]
    fn reorder_needs_a_permutation() {
        let (mut s, p) = library(&["A", "B", "C"]);
        reorder(&mut s, &p, vec!["p2".into(), "p0".into(), "p1".into()]).expect("reorder");
        assert_eq!(names(&s, &p), ["C", "A", "B"]);
        for bad in [
            vec!["p0", "p1"],
            vec!["p0", "p0", "p1"],
            vec!["p0", "p1", "x"],
        ] {
            let bad = bad.into_iter().map(String::from).collect();
            assert_eq!(reorder(&mut s, &p, bad), Err(ProfileError::OrderInvalid));
        }
    }

    #[test]
    fn activate_and_order_fallbacks() {
        let (mut s, p) = library(&["A", "B"]);
        activate(&mut s, &p, "p1").expect("activate");
        assert_eq!(s.active_profile_id.as_deref(), Some("p1"));
        assert!(activate(&mut s, &p, "zz").is_err());
        s.profile_order = vec!["p1".into()];
        assert_eq!(
            names(&s, &p),
            ["B", "A"],
            "profiles missing from the order come last"
        );
    }

    #[test]
    fn every_error_has_a_code() {
        let codes: Vec<&str> = [
            ProfileError::NameEmpty,
            ProfileError::NameTooLong,
            ProfileError::NameTaken("a".into()),
            ProfileError::LastProfile,
            ProfileError::NotFound("a".into()),
            ProfileError::OrderInvalid,
        ]
        .iter()
        .map(|e| {
            assert!(!e.to_string().is_empty());
            e.code()
        })
        .collect();
        assert_eq!(codes.len(), 6);
        assert!(
            codes
                .iter()
                .all(|c| c.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_'))
        );
    }
}
