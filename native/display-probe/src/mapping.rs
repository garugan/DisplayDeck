use std::fmt;

use windows::Win32::Graphics::Gdi::{
    DISPLAYCONFIG_PATH_ACTIVE, DISPLAYCONFIG_SOURCE_IN_USE,
    DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_BOOT,
    DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_PATH,
    DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_SYSTEM, DISPLAYCONFIG_TARGET_IN_USE,
};

use crate::{
    ccd::{CcdPath, CcdSnapshot},
    display::{DisplayAdapter, MonitorInterfacePath},
};

const DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_MASK: u32 =
    DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_BOOT
        | DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_PATH
        | DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_SYSTEM;
const TARGET_NAME_FLAG_FRIENDLY_NAME_FORCED: u32 = 1 << 1;
const TARGET_NAME_KNOWN_FLAGS_MASK: u32 = (1 << 0) | (1 << 1) | (1 << 2);

#[derive(Debug)]
pub struct CrossMap {
    pub paths: Vec<PathMapping>,
    pub exact_paths: usize,
    pub unmapped_paths: usize,
    pub ambiguous_paths: usize,
    pub inconsistent_paths: usize,
}

#[derive(Debug)]
pub struct PathMapping {
    pub path_index: usize,
    pub source_match: SourceMatch,
    pub target_match: TargetMatch,
    pub source_attached_to_desktop: Option<bool>,
    pub parent_adapter_consistent: Option<bool>,
    pub target_attached_to_desktop: Option<bool>,
    pub output_technology_consistent: bool,
    pub source_endpoint_multiplicity: usize,
    pub source_endpoint_identity_consistent: bool,
    pub source_in_use: bool,
    pub target_endpoint_multiplicity: usize,
    pub target_available: bool,
    pub target_in_use: bool,
    pub target_forced_availability: bool,
    pub target_friendly_name_forced: bool,
    pub target_name_has_unknown_flags: bool,
    pub path_active: bool,
    pub classification: PathClassification,
}

#[derive(Debug)]
pub enum SourceMatch {
    Exact { adapter_index: u32 },
    IdentityUnavailable,
    Unmapped,
    Ambiguous { adapter_indices: Vec<u32> },
}

#[derive(Clone, Copy, Debug)]
pub struct MonitorLocation {
    pub adapter_index: u32,
    pub monitor_index: u32,
}

#[derive(Debug)]
pub enum TargetMatch {
    Exact { location: MonitorLocation },
    IdentityUnavailable,
    Unmapped,
    Ambiguous { locations: Vec<MonitorLocation> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathClassification {
    Exact,
    Unmapped,
    Ambiguous,
    Inconsistent,
}

pub fn cross_map(snapshot: &CcdSnapshot, adapters: &[DisplayAdapter]) -> CrossMap {
    let mut paths = Vec::with_capacity(snapshot.paths.len());

    for path in &snapshot.paths {
        let source_match = match_source(path, adapters);
        let target_match = match_target(path, adapters);

        let source_attached_to_desktop = match &source_match {
            SourceMatch::Exact { adapter_index } => adapters
                .iter()
                .find(|adapter| adapter.index == *adapter_index)
                .map(|adapter| adapter.info.is_attached_to_desktop),
            _ => None,
        };

        let parent_adapter_consistent = match (&source_match, &target_match) {
            (
                SourceMatch::Exact { adapter_index },
                TargetMatch::Exact { location },
            ) => Some(*adapter_index == location.adapter_index),
            _ => None,
        };
        let target_attached_to_desktop = match &target_match {
            TargetMatch::Exact { location } => adapters
                .iter()
                .find(|adapter| adapter.index == location.adapter_index)
                .and_then(|adapter| {
                    adapter
                        .monitors
                        .iter()
                        .find(|monitor| monitor.index == location.monitor_index)
                })
                .map(|monitor| monitor.info.is_attached_to_desktop),
            _ => None,
        };
        let output_technology_consistent =
            path.target.output_technology == path.target.metadata_output_technology;
        let source_endpoint_multiplicity = source_endpoint_multiplicity(path, snapshot);
        let source_endpoint_identity_consistent =
            source_endpoint_identity_consistent(path, snapshot);
        let target_endpoint_multiplicity = target_endpoint_multiplicity(path, snapshot);
        let target_available = path.target.available;
        let source_in_use = path.source.status_flags & DISPLAYCONFIG_SOURCE_IN_USE != 0;
        let target_in_use = path.target.status_flags & DISPLAYCONFIG_TARGET_IN_USE != 0;
        let target_forced_availability = path.target.status_flags
            & DISPLAYCONFIG_TARGET_FORCED_AVAILABILITY_MASK
            != 0;
        let target_friendly_name_forced =
            path.target.device_name_flags & TARGET_NAME_FLAG_FRIENDLY_NAME_FORCED != 0;
        let target_name_has_unknown_flags =
            path.target.device_name_flags & !TARGET_NAME_KNOWN_FLAGS_MASK != 0;
        let path_active = path.flags & DISPLAYCONFIG_PATH_ACTIVE != 0;

        let classification = classify(
            &source_match,
            &target_match,
            source_attached_to_desktop,
            parent_adapter_consistent,
            target_attached_to_desktop,
            output_technology_consistent,
            source_endpoint_identity_consistent,
            target_endpoint_multiplicity,
            target_available,
            source_in_use,
            target_in_use,
            target_forced_availability,
            target_friendly_name_forced,
            target_name_has_unknown_flags,
            path_active,
        );

        paths.push(PathMapping {
            path_index: path.index,
            source_match,
            target_match,
            source_attached_to_desktop,
            parent_adapter_consistent,
            target_attached_to_desktop,
            output_technology_consistent,
            source_endpoint_multiplicity,
            source_endpoint_identity_consistent,
            source_in_use,
            target_endpoint_multiplicity,
            target_available,
            target_in_use,
            target_forced_availability,
            target_friendly_name_forced,
            target_name_has_unknown_flags,
            path_active,
            classification,
        });
    }

    let exact_paths = count_classification(&paths, PathClassification::Exact);
    let unmapped_paths = count_classification(&paths, PathClassification::Unmapped);
    let ambiguous_paths = count_classification(&paths, PathClassification::Ambiguous);
    let inconsistent_paths = count_classification(&paths, PathClassification::Inconsistent);

    CrossMap {
        paths,
        exact_paths,
        unmapped_paths,
        ambiguous_paths,
        inconsistent_paths,
    }
}

fn match_source(path: &CcdPath, adapters: &[DisplayAdapter]) -> SourceMatch {
    let Some(source_key) = path.source.gdi_device_name_key.as_deref() else {
        return SourceMatch::IdentityUnavailable;
    };

    // Identity comparison deliberately uses the exact UTF-16 code units returned
    // by the two APIs. No case-folding, trimming, parsing, or lossy String value is
    // allowed to turn a near match into an identity match.
    let adapter_indices = adapters
        .iter()
        .filter(|adapter| adapter.device_name_key.as_deref() == Some(source_key))
        .map(|adapter| adapter.index)
        .collect::<Vec<_>>();

    match adapter_indices.as_slice() {
        [] => SourceMatch::Unmapped,
        [adapter_index] => SourceMatch::Exact {
            adapter_index: *adapter_index,
        },
        _ => SourceMatch::Ambiguous { adapter_indices },
    }
}

fn match_target(path: &CcdPath, adapters: &[DisplayAdapter]) -> TargetMatch {
    let Some(target_key) = path.target.device_path_key.as_deref() else {
        return TargetMatch::IdentityUnavailable;
    };

    // Search globally rather than assuming that the target belongs to the source
    // adapter. Parent consistency is checked separately after an exact match.
    let locations = adapters
        .iter()
        .flat_map(|adapter| {
            adapter.monitors.iter().filter_map(move |monitor| {
                let MonitorInterfacePath::Available { key, .. } = &monitor.interface_path else {
                    return None;
                };
                (key.as_slice() == target_key).then_some(MonitorLocation {
                    adapter_index: adapter.index,
                    monitor_index: monitor.index,
                })
            })
        })
        .collect::<Vec<_>>();

    match locations.as_slice() {
        [] => TargetMatch::Unmapped,
        [location] => TargetMatch::Exact {
            location: *location,
        },
        _ => TargetMatch::Ambiguous { locations },
    }
}

fn classify(
    source_match: &SourceMatch,
    target_match: &TargetMatch,
    source_attached_to_desktop: Option<bool>,
    parent_adapter_consistent: Option<bool>,
    target_attached_to_desktop: Option<bool>,
    output_technology_consistent: bool,
    source_endpoint_identity_consistent: bool,
    target_endpoint_multiplicity: usize,
    target_available: bool,
    source_in_use: bool,
    target_in_use: bool,
    target_forced_availability: bool,
    target_friendly_name_forced: bool,
    target_name_has_unknown_flags: bool,
    path_active: bool,
) -> PathClassification {
    if matches!(source_match, SourceMatch::Ambiguous { .. })
        || matches!(target_match, TargetMatch::Ambiguous { .. })
    {
        return PathClassification::Ambiguous;
    }

    if matches!(
        source_match,
        SourceMatch::IdentityUnavailable | SourceMatch::Unmapped
    ) || matches!(
        target_match,
        TargetMatch::IdentityUnavailable | TargetMatch::Unmapped
    ) {
        return PathClassification::Unmapped;
    }

    if source_attached_to_desktop == Some(true)
        && parent_adapter_consistent == Some(true)
        && target_attached_to_desktop == Some(true)
        && output_technology_consistent
        && source_endpoint_identity_consistent
        && target_endpoint_multiplicity == 1
        && target_available
        && source_in_use
        && target_in_use
        && !target_forced_availability
        && !target_friendly_name_forced
        && !target_name_has_unknown_flags
        && path_active
    {
        PathClassification::Exact
    } else {
        PathClassification::Inconsistent
    }
}

fn source_endpoint_identity_consistent(path: &CcdPath, snapshot: &CcdSnapshot) -> bool {
    snapshot.paths.iter().all(|candidate| {
        candidate.source.adapter_luid != path.source.adapter_luid
            || candidate.source.id != path.source.id
            || candidate.source.gdi_device_name_key == path.source.gdi_device_name_key
    })
}

fn source_endpoint_multiplicity(path: &CcdPath, snapshot: &CcdSnapshot) -> usize {
    snapshot
        .paths
        .iter()
        .filter(|candidate| {
            candidate.source.adapter_luid == path.source.adapter_luid
                && candidate.source.id == path.source.id
        })
        .count()
}

fn target_endpoint_multiplicity(path: &CcdPath, snapshot: &CcdSnapshot) -> usize {
    snapshot
        .paths
        .iter()
        .filter(|candidate| {
            candidate.target.adapter_luid == path.target.adapter_luid
                && candidate.target.id == path.target.id
        })
        .count()
}

fn count_classification(paths: &[PathMapping], expected: PathClassification) -> usize {
    paths
        .iter()
        .filter(|path| path.classification == expected)
        .count()
}

impl fmt::Display for SourceMatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact { adapter_index } => write!(formatter, "Exact (Adapter {adapter_index})"),
            Self::IdentityUnavailable => write!(formatter, "Unmapped (identity unavailable)"),
            Self::Unmapped => write!(formatter, "Unmapped (0 candidates)"),
            Self::Ambiguous { adapter_indices } => {
                write!(formatter, "Ambiguous (Adapters")?;
                for adapter_index in adapter_indices {
                    write!(formatter, " {adapter_index}")?;
                }
                write!(formatter, ")")
            }
        }
    }
}

impl fmt::Display for TargetMatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact { location } => write!(
                formatter,
                "Exact (Adapter {} / Monitor {})",
                location.adapter_index, location.monitor_index
            ),
            Self::IdentityUnavailable => write!(formatter, "Unmapped (identity unavailable)"),
            Self::Unmapped => write!(formatter, "Unmapped (0 candidates)"),
            Self::Ambiguous { locations } => {
                write!(formatter, "Ambiguous (")?;
                for (position, location) in locations.iter().enumerate() {
                    if position > 0 {
                        write!(formatter, ", ")?;
                    }
                    write!(
                        formatter,
                        "Adapter {} / Monitor {}",
                        location.adapter_index, location.monitor_index
                    )?;
                }
                write!(formatter, ")")
            }
        }
    }
}

impl fmt::Display for PathClassification {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact => write!(formatter, "Exact"),
            Self::Unmapped => write!(formatter, "Unmapped"),
            Self::Ambiguous => write!(formatter, "Ambiguous"),
            Self::Inconsistent => write!(formatter, "Inconsistent"),
        }
    }
}
