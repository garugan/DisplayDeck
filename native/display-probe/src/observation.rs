use std::fmt;

use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_ROTATION_IDENTITY, DISPLAYCONFIG_ROTATION_ROTATE180,
    DISPLAYCONFIG_ROTATION_ROTATE270, DISPLAYCONFIG_ROTATION_ROTATE90,
};

use crate::{
    ccd::{CcdSnapshot, Rational},
    display::{DisplayAdapter, DisplayMode, RefreshRate},
    mapping::{CrossMap, PathClassification, SourceMatch, TargetMatch},
};

#[derive(Debug)]
pub struct CurrentObservationReport {
    pub paths: Vec<PathObservation>,
    pub exact_paths: usize,
    pub distinct_paths: usize,
    pub mismatch_paths: usize,
    pub unavailable_paths: usize,
}

#[derive(Debug)]
pub enum PathObservation {
    Observed(CurrentObservation),
    Unavailable {
        path_index: usize,
        reason: ObservationUnavailable,
    },
}

#[derive(Debug)]
pub struct CurrentObservation {
    pub path_index: usize,
    pub adapter_index: u32,
    pub monitor_index: u32,
    pub device_name: String,
    pub friendly_label: String,
    pub rotation: Rotation,
    pub scaling_raw: i32,
    pub gdi_resolution: Option<Dimensions>,
    pub ccd_source_resolution: Option<Dimensions>,
    pub rotation_applied_source_resolution: Option<Dimensions>,
    pub ccd_target_active_resolution: Option<Dimensions>,
    pub desktop_resolution_relation: ObservationRelation,
    pub source_target_resolution_relation: ObservationRelation,
    pub gdi_refresh: GdiRefresh,
    pub ccd_path_refresh: Rational,
    pub ccd_target_vsync: Option<Rational>,
    pub gdi_vs_ccd_path_refresh: ObservationRelation,
    pub gdi_vs_ccd_target_vsync: ObservationRelation,
    pub ccd_path_vs_target_vsync: ObservationRelation,
    pub classification: ObservationClassification,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rotation {
    Identity,
    Rotate90,
    Rotate180,
    Rotate270,
    Unknown(i32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GdiRefresh {
    Hertz(u32),
    DriverDefault,
    NotReported,
    ModeUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationRelation {
    Exact,
    Distinct,
    Mismatch,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationClassification {
    Exact,
    Distinct,
    Mismatch,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationUnavailable {
    MappingNotExact(PathClassification),
    CloneSourceNotQualified { multiplicity: usize },
    InternalMappingInconsistency,
}

pub fn build_current_observations(
    snapshot: &CcdSnapshot,
    cross_map: &CrossMap,
    adapters: &[DisplayAdapter],
) -> CurrentObservationReport {
    let mut paths = Vec::with_capacity(cross_map.paths.len());

    for mapping in &cross_map.paths {
        if mapping.classification != PathClassification::Exact {
            paths.push(PathObservation::Unavailable {
                path_index: mapping.path_index,
                reason: ObservationUnavailable::MappingNotExact(mapping.classification),
            });
            continue;
        }

        // Clone paths can share one GDI source while applying target-specific
        // rotation. Step 6 does not infer a per-target desktop observation from
        // that shared source; clone qualification remains a later support-cell task.
        if mapping.source_endpoint_multiplicity != 1 {
            paths.push(PathObservation::Unavailable {
                path_index: mapping.path_index,
                reason: ObservationUnavailable::CloneSourceNotQualified {
                    multiplicity: mapping.source_endpoint_multiplicity,
                },
            });
            continue;
        }

        let (adapter_index, target_location) = match (&mapping.source_match, &mapping.target_match)
        {
            (SourceMatch::Exact { adapter_index }, TargetMatch::Exact { location })
                if *adapter_index == location.adapter_index =>
            {
                (*adapter_index, *location)
            }
            _ => {
                paths.push(PathObservation::Unavailable {
                    path_index: mapping.path_index,
                    reason: ObservationUnavailable::InternalMappingInconsistency,
                });
                continue;
            }
        };

        let Some(path) = snapshot
            .paths
            .iter()
            .find(|path| path.index == mapping.path_index)
        else {
            paths.push(internal_inconsistency(mapping.path_index));
            continue;
        };
        let Some(adapter) = adapters
            .iter()
            .find(|adapter| adapter.index == adapter_index)
        else {
            paths.push(internal_inconsistency(mapping.path_index));
            continue;
        };
        let Some(_monitor) = adapter
            .monitors
            .iter()
            .find(|monitor| monitor.index == target_location.monitor_index)
        else {
            paths.push(internal_inconsistency(mapping.path_index));
            continue;
        };

        paths.push(PathObservation::Observed(observe_exact_path(
            path,
            adapter,
            target_location.monitor_index,
        )));
    }

    let exact_paths = count_classification(&paths, ObservationClassification::Exact);
    let distinct_paths = count_classification(&paths, ObservationClassification::Distinct);
    let mismatch_paths = count_classification(&paths, ObservationClassification::Mismatch);
    let unavailable_paths = count_classification(&paths, ObservationClassification::Unavailable);

    CurrentObservationReport {
        paths,
        exact_paths,
        distinct_paths,
        mismatch_paths,
        unavailable_paths,
    }
}

fn internal_inconsistency(path_index: usize) -> PathObservation {
    PathObservation::Unavailable {
        path_index,
        reason: ObservationUnavailable::InternalMappingInconsistency,
    }
}

fn observe_exact_path(
    path: &crate::ccd::CcdPath,
    adapter: &DisplayAdapter,
    monitor_index: u32,
) -> CurrentObservation {
    let rotation = Rotation::from_raw(path.target.rotation);
    let current_mode = adapter.current_mode.stable_mode();
    let gdi_resolution = gdi_dimensions(current_mode);
    let ccd_source_resolution = path
        .source_mode
        .as_ref()
        .and_then(|mode| Dimensions::new(mode.width_pixels, mode.height_pixels));
    let rotation_applied_source_resolution =
        ccd_source_resolution.and_then(|dimensions| rotation.apply(dimensions));
    let ccd_target_active_resolution = path
        .target_mode
        .as_ref()
        .and_then(|mode| Dimensions::new(mode.active_width_pixels, mode.active_height_pixels));

    let desktop_resolution_relation =
        compare_desktop_resolution(gdi_resolution, rotation_applied_source_resolution);
    let source_target_resolution_relation =
        compare_distinct_dimensions(ccd_source_resolution, ccd_target_active_resolution);

    let gdi_refresh = gdi_refresh(current_mode);
    let ccd_path_refresh = path.target.refresh_rate;
    let ccd_target_vsync = path.target_mode.as_ref().map(|mode| mode.vertical_sync);
    let gdi_vs_ccd_path_refresh = compare_gdi_to_rational(gdi_refresh, ccd_path_refresh);
    let gdi_vs_ccd_target_vsync = ccd_target_vsync
        .map(|target_vsync| compare_gdi_to_rational(gdi_refresh, target_vsync))
        .unwrap_or(ObservationRelation::Unavailable);
    let ccd_path_vs_target_vsync = ccd_target_vsync
        .map(|target_vsync| compare_rationals(ccd_path_refresh, target_vsync))
        .unwrap_or(ObservationRelation::Unavailable);

    let classification = classify_observation(&[
        desktop_resolution_relation,
        source_target_resolution_relation,
        gdi_vs_ccd_path_refresh,
        gdi_vs_ccd_target_vsync,
        ccd_path_vs_target_vsync,
    ]);

    CurrentObservation {
        path_index: path.index,
        adapter_index: adapter.index,
        monitor_index,
        device_name: adapter.info.device_name.clone(),
        friendly_label: path.target.friendly_name.clone(),
        rotation,
        scaling_raw: path.target.scaling,
        gdi_resolution,
        ccd_source_resolution,
        rotation_applied_source_resolution,
        ccd_target_active_resolution,
        desktop_resolution_relation,
        source_target_resolution_relation,
        gdi_refresh,
        ccd_path_refresh,
        ccd_target_vsync,
        gdi_vs_ccd_path_refresh,
        gdi_vs_ccd_target_vsync,
        ccd_path_vs_target_vsync,
        classification,
    }
}

fn gdi_dimensions(mode: Option<&DisplayMode>) -> Option<Dimensions> {
    let mode = mode?;
    Dimensions::new(mode.width_pixels?, mode.height_pixels?)
}

fn gdi_refresh(mode: Option<&DisplayMode>) -> GdiRefresh {
    match mode.map(DisplayMode::refresh_rate) {
        Some(RefreshRate::Hertz(hertz)) if hertz > 1 => GdiRefresh::Hertz(hertz),
        Some(RefreshRate::Hertz(_)) | Some(RefreshRate::DriverDefault) => GdiRefresh::DriverDefault,
        Some(RefreshRate::NotReported) => GdiRefresh::NotReported,
        None => GdiRefresh::ModeUnavailable,
    }
}

fn compare_desktop_resolution(
    gdi: Option<Dimensions>,
    rotation_applied_ccd: Option<Dimensions>,
) -> ObservationRelation {
    match (gdi, rotation_applied_ccd) {
        (Some(gdi), Some(ccd)) if gdi == ccd => ObservationRelation::Exact,
        (Some(_), Some(_)) => ObservationRelation::Mismatch,
        _ => ObservationRelation::Unavailable,
    }
}

fn compare_distinct_dimensions(
    left: Option<Dimensions>,
    right: Option<Dimensions>,
) -> ObservationRelation {
    match (left, right) {
        (Some(left), Some(right)) if left == right => ObservationRelation::Exact,
        (Some(_), Some(_)) => ObservationRelation::Distinct,
        _ => ObservationRelation::Unavailable,
    }
}

fn compare_gdi_to_rational(gdi: GdiRefresh, ccd: Rational) -> ObservationRelation {
    let GdiRefresh::Hertz(gdi_hertz) = gdi else {
        return ObservationRelation::Unavailable;
    };
    if gdi_hertz <= 1 || !rational_is_positive(ccd) {
        return ObservationRelation::Unavailable;
    }

    if u128::from(gdi_hertz) * u128::from(ccd.denominator) == u128::from(ccd.numerator) {
        ObservationRelation::Exact
    } else {
        ObservationRelation::Distinct
    }
}

fn compare_rationals(left: Rational, right: Rational) -> ObservationRelation {
    if !rational_is_positive(left) || !rational_is_positive(right) {
        return ObservationRelation::Unavailable;
    }

    if u128::from(left.numerator) * u128::from(right.denominator)
        == u128::from(right.numerator) * u128::from(left.denominator)
    {
        ObservationRelation::Exact
    } else {
        ObservationRelation::Distinct
    }
}

fn rational_is_positive(value: Rational) -> bool {
    value.numerator > 0 && value.denominator > 0
}

fn classify_observation(relations: &[ObservationRelation]) -> ObservationClassification {
    if relations.is_empty() || relations.contains(&ObservationRelation::Unavailable) {
        ObservationClassification::Unavailable
    } else if relations.contains(&ObservationRelation::Mismatch) {
        ObservationClassification::Mismatch
    } else if relations.contains(&ObservationRelation::Distinct) {
        ObservationClassification::Distinct
    } else {
        ObservationClassification::Exact
    }
}

fn count_classification(paths: &[PathObservation], expected: ObservationClassification) -> usize {
    paths
        .iter()
        .filter(|path| path.classification() == expected)
        .count()
}

impl PathObservation {
    pub fn classification(&self) -> ObservationClassification {
        match self {
            Self::Observed(observation) => observation.classification,
            Self::Unavailable { .. } => ObservationClassification::Unavailable,
        }
    }
}

impl Dimensions {
    fn new(width: u32, height: u32) -> Option<Self> {
        (width > 0 && height > 0).then_some(Self { width, height })
    }
}

impl Rotation {
    fn from_raw(raw: i32) -> Self {
        if raw == DISPLAYCONFIG_ROTATION_IDENTITY.0 {
            Self::Identity
        } else if raw == DISPLAYCONFIG_ROTATION_ROTATE90.0 {
            Self::Rotate90
        } else if raw == DISPLAYCONFIG_ROTATION_ROTATE180.0 {
            Self::Rotate180
        } else if raw == DISPLAYCONFIG_ROTATION_ROTATE270.0 {
            Self::Rotate270
        } else {
            Self::Unknown(raw)
        }
    }

    fn apply(self, dimensions: Dimensions) -> Option<Dimensions> {
        match self {
            Self::Identity | Self::Rotate180 => Some(dimensions),
            Self::Rotate90 | Self::Rotate270 => Some(Dimensions {
                width: dimensions.height,
                height: dimensions.width,
            }),
            Self::Unknown(_) => None,
        }
    }
}

impl fmt::Display for Dimensions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}x{}", self.width, self.height)
    }
}

impl fmt::Display for Rotation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity => {
                write!(
                    formatter,
                    "Identity ({})",
                    DISPLAYCONFIG_ROTATION_IDENTITY.0
                )
            }
            Self::Rotate90 => {
                write!(
                    formatter,
                    "Rotate90 ({})",
                    DISPLAYCONFIG_ROTATION_ROTATE90.0
                )
            }
            Self::Rotate180 => write!(
                formatter,
                "Rotate180 ({})",
                DISPLAYCONFIG_ROTATION_ROTATE180.0
            ),
            Self::Rotate270 => write!(
                formatter,
                "Rotate270 ({})",
                DISPLAYCONFIG_ROTATION_ROTATE270.0
            ),
            Self::Unknown(raw) => write!(formatter, "Unknown ({raw})"),
        }
    }
}

impl fmt::Display for GdiRefresh {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hertz(hertz) => write!(formatter, "{hertz} Hz (integer)"),
            Self::DriverDefault => write!(formatter, "driver default (unavailable)"),
            Self::NotReported => write!(formatter, "not reported"),
            Self::ModeUnavailable => write!(formatter, "current mode unavailable"),
        }
    }
}

impl fmt::Display for ObservationRelation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

impl fmt::Display for ObservationClassification {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

impl fmt::Display for ObservationUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MappingNotExact(classification) => {
                write!(formatter, "cross-map result is {classification}")
            }
            Self::CloneSourceNotQualified { multiplicity } => write!(
                formatter,
                "shared clone source has {multiplicity} paths; Step 6 does not qualify clones"
            ),
            Self::InternalMappingInconsistency => {
                write!(
                    formatter,
                    "exact cross-map could not be resolved internally"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE_1920_1080: Dimensions = Dimensions {
        width: 1920,
        height: 1080,
    };

    #[test]
    fn identity_and_rotate_180_keep_dimensions() {
        assert_eq!(
            Rotation::Identity.apply(SIZE_1920_1080),
            Some(SIZE_1920_1080)
        );
        assert_eq!(
            Rotation::Rotate180.apply(SIZE_1920_1080),
            Some(SIZE_1920_1080)
        );
    }

    #[test]
    fn quarter_turns_swap_dimensions() {
        let portrait = Some(Dimensions {
            width: 1080,
            height: 1920,
        });

        assert_eq!(Rotation::Rotate90.apply(SIZE_1920_1080), portrait);
        assert_eq!(Rotation::Rotate270.apply(SIZE_1920_1080), portrait);
    }

    #[test]
    fn unknown_rotation_is_unavailable() {
        assert_eq!(Rotation::Unknown(0).apply(SIZE_1920_1080), None);
    }

    #[test]
    fn integer_and_unreduced_rational_compare_exactly() {
        assert_eq!(
            compare_gdi_to_rational(
                GdiRefresh::Hertz(60),
                Rational {
                    numerator: 120_000,
                    denominator: 2_000,
                },
            ),
            ObservationRelation::Exact
        );
    }

    #[test]
    fn fractional_rate_is_not_rounded_to_integer() {
        assert_eq!(
            compare_gdi_to_rational(
                GdiRefresh::Hertz(60),
                Rational {
                    numerator: 60_000,
                    denominator: 1_001,
                },
            ),
            ObservationRelation::Distinct
        );
    }

    #[test]
    fn equivalent_ccd_rationals_compare_exactly() {
        assert_eq!(
            compare_rationals(
                Rational {
                    numerator: 60,
                    denominator: 1,
                },
                Rational {
                    numerator: 120,
                    denominator: 2,
                },
            ),
            ObservationRelation::Exact
        );
    }

    #[test]
    fn gdi_default_and_unreported_rates_are_unavailable() {
        let ccd = Rational {
            numerator: 60,
            denominator: 1,
        };

        assert_eq!(
            compare_gdi_to_rational(GdiRefresh::DriverDefault, ccd),
            ObservationRelation::Unavailable
        );
        assert_eq!(
            compare_gdi_to_rational(GdiRefresh::NotReported, ccd),
            ObservationRelation::Unavailable
        );
        assert_eq!(
            compare_gdi_to_rational(GdiRefresh::Hertz(1), ccd),
            ObservationRelation::Unavailable
        );
    }

    #[test]
    fn invalid_or_unspecified_rational_is_unavailable() {
        for rational in [
            Rational {
                numerator: 0,
                denominator: 0,
            },
            Rational {
                numerator: 60,
                denominator: 0,
            },
            Rational {
                numerator: 0,
                denominator: 1,
            },
        ] {
            assert_eq!(
                compare_gdi_to_rational(GdiRefresh::Hertz(60), rational),
                ObservationRelation::Unavailable
            );
        }
    }

    #[test]
    fn unavailable_precedes_mismatch_and_distinct() {
        assert_eq!(
            classify_observation(&[]),
            ObservationClassification::Unavailable
        );
        assert_eq!(
            classify_observation(&[
                ObservationRelation::Distinct,
                ObservationRelation::Unavailable,
                ObservationRelation::Mismatch,
            ]),
            ObservationClassification::Unavailable
        );
    }

    #[test]
    fn mismatch_precedes_distinct_when_all_values_are_available() {
        assert_eq!(
            classify_observation(&[ObservationRelation::Distinct, ObservationRelation::Mismatch,]),
            ObservationClassification::Mismatch
        );
    }
}
