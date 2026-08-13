use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use icu_collator::{Collator, options::CollatorOptions};
use icu_locale::Locale;
use unicode_segmentation::UnicodeSegmentation;

use crate::domain::{
    EntrySortMode, EntrySortSettingsV1, EntrySummary, ManualSortItem, ManualSortLayoutV1,
};

#[derive(Debug, Clone)]
pub(crate) struct SortableSummary {
    pub summary: EntrySummary,
    pub sort_text: String,
    pub section_override: Option<String>,
}

pub(crate) fn order_summaries(
    mut rows: Vec<SortableSummary>,
    settings: &EntrySortSettingsV1,
    layout: &ManualSortLayoutV1,
    language_tag: Option<&str>,
) -> Vec<EntrySummary> {
    sort_automatic(&mut rows, settings, language_tag);
    if settings.mode == EntrySortMode::Auto {
        return rows
            .into_iter()
            .map(|mut row| {
                row.summary.section_label = Some(section_for(&row, settings));
                row.summary.manual_order_pending = false;
                row.summary
            })
            .collect();
    }

    let mut by_id = rows
        .into_iter()
        .map(|row| (row.summary.id.clone(), row))
        .collect::<HashMap<_, _>>();
    let mut result = Vec::new();
    let mut current_heading: Option<String> = None;
    for item in &layout.items {
        match item {
            ManualSortItem::Heading { label, .. } => current_heading = Some(label.clone()),
            ManualSortItem::Entry { entry_id } => {
                if let Some(mut row) = by_id.remove(entry_id) {
                    row.summary.section_label.clone_from(&current_heading);
                    row.summary.manual_order_pending = false;
                    result.push(row.summary);
                }
            }
        }
    }

    let mut pending = by_id.into_values().collect::<Vec<_>>();
    sort_automatic(&mut pending, settings, language_tag);
    for mut row in pending {
        let section = section_for(&row, settings);
        row.summary.section_label = Some(section.clone());
        row.summary.manual_order_pending = true;
        let insertion = result
            .iter()
            .rposition(|item| item.section_label.as_deref() == Some(section.as_str()))
            .map_or(result.len(), |index| index + 1);
        result.insert(insertion, row.summary);
    }
    result
}

pub(crate) fn validate_settings(
    settings: &EntrySortSettingsV1,
    writing_system_ids: &HashSet<&str>,
) -> Result<(), &'static str> {
    if settings.version != 1 || !writing_system_ids.contains(settings.writing_system_id.as_str()) {
        return Err("Sort settings reference a missing writing system.");
    }
    let mut seen = HashSet::new();
    for item in &settings.alphabet {
        let key = item.trim().to_lowercase();
        if key.is_empty() || !seen.insert(key) {
            return Err("Alphabet elements must be non-empty and unique.");
        }
    }
    Ok(())
}

pub(crate) fn validate_layout(layout: &ManualSortLayoutV1) -> Result<(), &'static str> {
    if layout.version != 1 {
        return Err("Unsupported manual layout version.");
    }
    let mut ids = HashSet::new();
    for item in &layout.items {
        match item {
            ManualSortItem::Heading { id, label }
                if id.trim().is_empty() || label.trim().is_empty() =>
            {
                return Err("Manual headings require an id and label.");
            }
            ManualSortItem::Entry { entry_id } if entry_id.trim().is_empty() => {
                return Err("Manual entry items require an entry id.");
            }
            ManualSortItem::Heading { id, .. } if !ids.insert(format!("h:{id}")) => {
                return Err("Manual layout items must be unique.");
            }
            ManualSortItem::Entry { entry_id } if !ids.insert(format!("e:{entry_id}")) => {
                return Err("Manual layout items must be unique.");
            }
            _ => {}
        }
    }
    Ok(())
}

fn sort_automatic(
    rows: &mut [SortableSummary],
    settings: &EntrySortSettingsV1,
    language_tag: Option<&str>,
) {
    let alphabet = normalized_alphabet(settings);
    let collator = collator(language_tag.unwrap_or("und"));
    rows.sort_by(|left, right| {
        let left_section = section_for(left, settings);
        let right_section = section_for(right, settings);
        section_rank(&left_section, &alphabet)
            .cmp(&section_rank(&right_section, &alphabet))
            .then_with(|| {
                compare_text(
                    &left.sort_text,
                    &right.sort_text,
                    &alphabet,
                    collator.as_ref(),
                )
            })
            .then_with(|| left.summary.id.cmp(&right.summary.id))
    });
}

fn normalized_alphabet(settings: &EntrySortSettingsV1) -> Vec<String> {
    settings
        .alphabet
        .iter()
        .map(|item| item.trim().to_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

fn section_for(row: &SortableSummary, settings: &EntrySortSettingsV1) -> String {
    if let Some(value) = row
        .section_override
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return value.trim().to_owned();
    }
    let lower = row.sort_text.to_lowercase();
    let mut matches = settings
        .alphabet
        .iter()
        .filter(|item| lower.starts_with(&item.trim().to_lowercase()))
        .collect::<Vec<_>>();
    matches.sort_by_key(|item| std::cmp::Reverse(item.chars().count()));
    matches.first().map_or_else(
        || {
            row.sort_text
                .graphemes(true)
                .next()
                .unwrap_or("#")
                .to_uppercase()
        },
        |item| item.trim().to_uppercase(),
    )
}

fn section_rank(section: &str, alphabet: &[String]) -> (usize, String) {
    let key = section.to_lowercase();
    (
        alphabet
            .iter()
            .position(|item| item == &key)
            .unwrap_or(usize::MAX),
        key,
    )
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Unit {
    Known(usize),
    Unknown(String),
}

fn custom_key(text: &str, alphabet: &[String]) -> Vec<Unit> {
    let lower = text.to_lowercase();
    let mut offset = 0;
    let mut result = Vec::new();
    while offset < lower.len() {
        let rest = &lower[offset..];
        let matched = alphabet
            .iter()
            .enumerate()
            .filter(|(_, item)| rest.starts_with(item.as_str()))
            .max_by_key(|(_, item)| item.len());
        if let Some((rank, item)) = matched {
            result.push(Unit::Known(rank));
            offset += item.len();
        } else if let Some(grapheme) = rest.graphemes(true).next() {
            result.push(Unit::Unknown(grapheme.to_owned()));
            offset += grapheme.len();
        } else {
            break;
        }
    }
    result
}

fn compare_text(
    left: &str,
    right: &str,
    alphabet: &[String],
    collator: Option<&icu_collator::CollatorBorrowed<'_>>,
) -> Ordering {
    if alphabet.is_empty() {
        collator
            .map(|value| value.compare(left, right))
            .unwrap_or_else(|| left.cmp(right))
    } else {
        custom_key(left, alphabet).cmp(&custom_key(right, alphabet))
    }
}

fn collator(language_tag: &str) -> Option<icu_collator::CollatorBorrowed<'static>> {
    let locale = language_tag.parse::<Locale>().ok()?;
    Collator::try_new(locale.into(), CollatorOptions::default()).ok()
}

#[cfg(test)]
mod tests {
    use super::custom_key;

    #[test]
    fn longest_alphabet_element_wins() {
        let alphabet = vec!["n".into(), "ng".into()];
        assert!(custom_key("naga", &alphabet) < custom_key("ngayo", &alphabet));
    }
}
