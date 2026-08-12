use std::collections::HashMap;

use crate::{
    candidate::{
        CandidateCatalog, CandidateEligibility, CandidateSummary, CurrentMembership,
        CurrentTupleStatus, HardExclusion, TupleStatus,
    },
    ccd::{CcdQueryFailureClass, CcdSnapshot, Rational},
    display::{DeviceEnumerationStatus, DisplayInventory, ModeEnumerationStatus},
    mapping::{CrossMap, PathClassification, SourceMatch, TargetMatch},
    observation::{
        CurrentObservationReport, ObservationClassification, ObservationRelation,
        PathObservation, Rotation,
    },
};

const MAX_QUALIFICATION_ADAPTERS: usize = 32;
const MANY_CANDIDATE_THRESHOLD: usize = 9;

// Microsoft documents these DISPLAYCONFIG_VIDEO_OUTPUT_TECHNOLOGY raw values as
// Miracast, indirect wired, and indirect virtual respectively. Observing another
// value is not proof that an output is local or physical; the virtual/session gap
// therefore remains present for every Step 8 report.
const OUTPUT_TECHNOLOGY_MIRACAST: i32 = 15;
const OUTPUT_TECHNOLOGY_INDIRECT_WIRED: i32 = 16;
const OUTPUT_TECHNOLOGY_INDIRECT_VIRTUAL: i32 = 17;
const OUTPUT_TECHNOLOGY_DISPLAYPORT_USB_TUNNEL: i32 = 18;
const OUTPUT_TECHNOLOGY_SDTVDONGLE: i32 = 14;
const OUTPUT_TECHNOLOGY_OTHER: i32 = -1;
const OUTPUT_TECHNOLOGY_INTERNAL: i32 = i32::MIN;

// These masks mirror the documented/SDK values returned by the read-only APIs.
// A known bit is not automatically admitted: only the narrow diagnostic subset
// is neutral for this precheck; all other known or unknown bits fail closed.
const GDI_STATE_FLAGS_KNOWN_MASK: u32 = 0x0F28_007F;
const GDI_ADAPTER_STATE_FLAGS_DIAGNOSTIC_MASK: u32 = 0x0800_0015;
const GDI_MONITOR_STATE_FLAGS_DIAGNOSTIC_MASK: u32 = 0x0800_0017;
const GDI_STATE_FLAG_MIRRORING_DRIVER: u32 = 0x0000_0008;
const GDI_STATE_FLAG_RDPUDD: u32 = 0x0100_0000;
const GDI_STATE_FLAG_REMOTE: u32 = 0x0400_0000;
const CCD_PATH_ACTIVE: u32 = 0x0000_0001;
const CCD_PATH_BOOST_REFRESH_RATE: u32 = 0x0000_0010;
const CCD_SOURCE_IN_USE: u32 = 0x0000_0001;
const CCD_TARGET_KNOWN_STATUS_MASK: u32 = 0x0000_003F;
const CCD_TARGET_IN_USE: u32 = 0x0000_0001;
const CCD_TARGET_IS_HMD: u32 = 0x0000_0020;

#[derive(Debug)]
pub enum MappingCapture {
    SampledStable(CrossMap),
    Unavailable(MappingCaptureFailure),
}

impl MappingCapture {
    pub fn stable_report(&self) -> Option<&CrossMap> {
        match self {
            Self::SampledStable(report) => Some(report),
            Self::Unavailable(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingCaptureFailure {
    InitialCcdQueryFailed(CcdQueryFailureClass),
    VerificationCcdQueryFailed(CcdQueryFailureClass),
    InventoryBoundExceeded,
    StaleSnapshot,
    InternalInconsistency,
}

#[derive(Debug)]
pub enum ObservationCapture {
    SampledStable(CurrentObservationReport),
    Unavailable(ObservationCaptureFailure),
}

impl ObservationCapture {
    pub fn stable_report(&self) -> Option<&CurrentObservationReport> {
        match self {
            Self::SampledStable(report) => Some(report),
            Self::Unavailable(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationCaptureFailure {
    InitialCcdQueryFailed(CcdQueryFailureClass),
    VerificationCcdQueryFailed(CcdQueryFailureClass),
    CrossMapUnavailable,
    StaleSnapshot,
    InternalInconsistency,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    RejectedByObservedEvidence,
    BlockedByMissingEvidence,
    NotAssessable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutationReadiness {
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum G1AGate {
    NotReadyEvidenceGaps,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase1AClosure {
    NotClaimed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingCaptureStatus {
    SampledStable,
    Unavailable(MappingCaptureFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationCaptureStatus {
    SampledStable,
    Unavailable(ObservationCaptureFailure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivePathAssessment {
    NotObserved,
    NoActivePaths,
    SingleActivePath,
    MultipleActivePaths { count: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateVolume {
    None,
    OneToEight { count: usize },
    NineOrMore { count: usize },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GdiEnvironmentMarkers {
    pub attached_adapter_devices: usize,
    pub attached_monitor_devices: usize,
    pub mirroring_driver_devices: usize,
    pub remote_sdk_devices: usize,
    pub rdpudd_sdk_devices: usize,
    pub known_unqualified_state_flag_devices: usize,
    pub unknown_state_flag_devices: usize,
    attached_adapter_mask: u32,
    attached_monitor_masks: [u32; MAX_QUALIFICATION_ADAPTERS],
}

impl GdiEnvironmentMarkers {
    fn any_observed(self) -> bool {
        self.mirroring_driver_devices != 0
            || self.remote_sdk_devices != 0
            || self.rdpudd_sdk_devices != 0
            || self.known_unqualified_state_flag_devices != 0
            || self.unknown_state_flag_devices != 0
    }

    fn marker_occurrences(self) -> usize {
        self.mirroring_driver_devices
            + self.remote_sdk_devices
            + self.rdpudd_sdk_devices
            + self.known_unqualified_state_flag_devices
            + self.unknown_state_flag_devices
    }
}

pub fn collect_gdi_environment_markers(
    inventory: &DisplayInventory,
) -> GdiEnvironmentMarkers {
    let mut markers = GdiEnvironmentMarkers::default();
    for adapter in &inventory.adapters {
        if adapter.info.is_attached_to_desktop {
            markers.attached_adapter_devices += 1;
            if let Some(mask) = bit_for_bounded_index(adapter.index) {
                markers.attached_adapter_mask |= mask;
            }
        }
        record_gdi_environment_marker(
            &mut markers,
            &adapter.info,
            GDI_ADAPTER_STATE_FLAGS_DIAGNOSTIC_MASK,
        );
        for monitor in &adapter.monitors {
            if monitor.info.is_attached_to_desktop {
                markers.attached_monitor_devices += 1;
                if let (Ok(adapter_index), Some(monitor_mask)) = (
                    usize::try_from(adapter.index),
                    bit_for_bounded_index(monitor.index),
                ) {
                    if let Some(mask) = markers.attached_monitor_masks.get_mut(adapter_index) {
                        *mask |= monitor_mask;
                    }
                }
            }
            record_gdi_environment_marker(
                &mut markers,
                &monitor.info,
                GDI_MONITOR_STATE_FLAGS_DIAGNOSTIC_MASK,
            );
        }
    }
    markers
}

fn bit_for_bounded_index(index: u32) -> Option<u32> {
    (index < 32).then(|| 1_u32 << index)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GdiActiveCoverage {
    NotAssessed,
    Assessed {
        attached_adapters: usize,
        exact_source_adapters: usize,
        attached_monitors: usize,
        exact_target_monitors: usize,
        consistent: bool,
    },
}

fn record_gdi_environment_marker(
    markers: &mut GdiEnvironmentMarkers,
    info: &crate::display::DisplayDeviceInfo,
    diagnostic_mask: u32,
) {
    if info.state_flags_raw & GDI_STATE_FLAG_MIRRORING_DRIVER != 0 {
        markers.mirroring_driver_devices += 1;
    }
    if info.state_flags_raw & GDI_STATE_FLAG_REMOTE != 0 {
        markers.remote_sdk_devices += 1;
    }
    if info.state_flags_raw & GDI_STATE_FLAG_RDPUDD != 0 {
        markers.rdpudd_sdk_devices += 1;
    }
    if info.state_flags_raw & (GDI_STATE_FLAGS_KNOWN_MASK & !diagnostic_mask) != 0 {
        markers.known_unqualified_state_flag_devices += 1;
    }
    if info.state_flags_raw & !GDI_STATE_FLAGS_KNOWN_MASK != 0 {
        markers.unknown_state_flag_devices += 1;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CcdNativeEvidenceMarkers {
    pub non_allowlisted_path_flag_paths: usize,
    pub boost_refresh_rate_paths: usize,
    pub unknown_source_status_paths: usize,
    pub non_allowlisted_target_status_paths: usize,
    pub unknown_target_status_paths: usize,
    pub hmd_paths: usize,
    pub unknown_rotation_paths: usize,
    pub non_allowlisted_scaling_paths: usize,
    pub unknown_scaling_paths: usize,
    pub non_allowlisted_scan_line_paths: usize,
    pub unknown_scan_line_paths: usize,
    pub non_allowlisted_pixel_format_paths: usize,
    pub unknown_pixel_format_paths: usize,
    pub non_gdi_pixel_format_paths: usize,
}

impl CcdNativeEvidenceMarkers {
    fn any_observed(self) -> bool {
        self.marker_occurrences() != 0
    }

    fn marker_occurrences(self) -> usize {
        self.non_allowlisted_path_flag_paths
            + self.boost_refresh_rate_paths
            + self.unknown_source_status_paths
            + self.non_allowlisted_target_status_paths
            + self.unknown_target_status_paths
            + self.hmd_paths
            + self.unknown_rotation_paths
            + self.non_allowlisted_scaling_paths
            + self.unknown_scaling_paths
            + self.non_allowlisted_scan_line_paths
            + self.unknown_scan_line_paths
            + self.non_allowlisted_pixel_format_paths
            + self.unknown_pixel_format_paths
            + self.non_gdi_pixel_format_paths
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadOnlyEvidenceGap {
    SupportCellIdentityNotIssued,
    ApprovedCcdSurfaceNotImplemented,
    CandidateTargetBindingMissing,
    ExpectedObservationMissing,
    SupportFingerprintMissing,
    SessionAndRdpEvidenceNotObserved,
    VirtualDisplayExclusionNotProven,
    HotplugBehaviorEvidenceMissing,
    PreferredModeNotObserved,
    PersistedBaselineNotObserved,
    AdvancedColorNotObserved,
    DynamicRefreshRateNotObserved,
    CallTraceAndTimeboxNotProduced,
    FormalEvidenceBundleNotProduced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QualificationBlocker {
    InternalInconsistency,
    MappingCaptureUnavailable(MappingCaptureFailure),
    ObservationCaptureUnavailable(ObservationCaptureFailure),
    InventoryIncomplete,
    CurrentTupleCaptureIncomplete,
    NoActivePaths,
    MultipleActivePaths { count: usize },
    NonExactMappingPaths { count: usize },
    CloneSourcePaths { count: usize },
    NonExactObservationPaths { count: usize },
    FractionalRefreshRelationUnresolved {
        comparisons: usize,
        distinct: usize,
    },
    UnqualifiedOutputTechnologyPaths { count: usize },
    CcdNativeEvidenceMarkersObserved { occurrences: usize },
    GdiEnvironmentMarkersObserved { occurrences: usize },
    GdiActiveCoverageMismatch,
    CurrentNotListedAdapters { count: usize },
    NoCandidateRecords,
    NoActiveAdapterLabUnqualifiedCandidates {
        active_adapters: usize,
        hard_excluded: usize,
    },
}

pub const READ_ONLY_EVIDENCE_GAPS: [ReadOnlyEvidenceGap; 14] = [
    ReadOnlyEvidenceGap::SupportCellIdentityNotIssued,
    ReadOnlyEvidenceGap::ApprovedCcdSurfaceNotImplemented,
    ReadOnlyEvidenceGap::CandidateTargetBindingMissing,
    ReadOnlyEvidenceGap::ExpectedObservationMissing,
    ReadOnlyEvidenceGap::SupportFingerprintMissing,
    ReadOnlyEvidenceGap::SessionAndRdpEvidenceNotObserved,
    ReadOnlyEvidenceGap::VirtualDisplayExclusionNotProven,
    ReadOnlyEvidenceGap::HotplugBehaviorEvidenceMissing,
    ReadOnlyEvidenceGap::PreferredModeNotObserved,
    ReadOnlyEvidenceGap::PersistedBaselineNotObserved,
    ReadOnlyEvidenceGap::AdvancedColorNotObserved,
    ReadOnlyEvidenceGap::DynamicRefreshRateNotObserved,
    ReadOnlyEvidenceGap::CallTraceAndTimeboxNotProduced,
    ReadOnlyEvidenceGap::FormalEvidenceBundleNotProduced,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HardExclusionHistogram {
    pub tuple_incomplete: usize,
    pub adapter_not_attached_to_desktop: usize,
    pub adapter_enumeration_incomplete: usize,
    pub monitor_enumeration_incomplete: usize,
    pub enumeration_empty_or_unavailable: usize,
    pub enumeration_incomplete: usize,
    pub current_unavailable: usize,
    pub current_changed_during_capture: usize,
    pub current_tuple_incomplete: usize,
    pub current_not_listed: usize,
    pub current_exact_record_ambiguous: usize,
    pub exact_tuple_duplicate: usize,
    pub driver_default_frequency: usize,
    pub current_driver_default_frequency: usize,
    pub bits_per_pixel_below_32: usize,
    pub current_bits_per_pixel_below_32: usize,
    pub known_but_unsupported_display_flags: usize,
    pub current_known_but_unsupported_display_flags: usize,
    pub policy_different: usize,
    pub policy_evidence_unavailable: usize,
}

impl HardExclusionHistogram {
    fn record(&mut self, reason: &HardExclusion) {
        let slot = match reason {
            HardExclusion::TupleIncomplete => &mut self.tuple_incomplete,
            HardExclusion::AdapterNotAttachedToDesktop => {
                &mut self.adapter_not_attached_to_desktop
            }
            HardExclusion::AdapterEnumerationIncomplete => {
                &mut self.adapter_enumeration_incomplete
            }
            HardExclusion::MonitorEnumerationIncomplete => {
                &mut self.monitor_enumeration_incomplete
            }
            HardExclusion::EnumerationEmptyOrUnavailable => {
                &mut self.enumeration_empty_or_unavailable
            }
            HardExclusion::EnumerationIncomplete => &mut self.enumeration_incomplete,
            HardExclusion::CurrentUnavailable => &mut self.current_unavailable,
            HardExclusion::CurrentChangedDuringCapture => {
                &mut self.current_changed_during_capture
            }
            HardExclusion::CurrentTupleIncomplete => &mut self.current_tuple_incomplete,
            HardExclusion::CurrentNotListed => &mut self.current_not_listed,
            HardExclusion::CurrentExactRecordAmbiguous => {
                &mut self.current_exact_record_ambiguous
            }
            HardExclusion::ExactTupleDuplicate => &mut self.exact_tuple_duplicate,
            HardExclusion::DriverDefaultFrequency { .. } => {
                &mut self.driver_default_frequency
            }
            HardExclusion::CurrentDriverDefaultFrequency { .. } => {
                &mut self.current_driver_default_frequency
            }
            HardExclusion::BitsPerPixelBelow32 { .. } => {
                &mut self.bits_per_pixel_below_32
            }
            HardExclusion::CurrentBitsPerPixelBelow32 { .. } => {
                &mut self.current_bits_per_pixel_below_32
            }
            HardExclusion::KnownButUnsupportedDisplayFlags { .. } => {
                &mut self.known_but_unsupported_display_flags
            }
            HardExclusion::CurrentKnownButUnsupportedDisplayFlags { .. } => {
                &mut self.current_known_but_unsupported_display_flags
            }
            HardExclusion::PolicyDifferent { .. } => &mut self.policy_different,
            HardExclusion::PolicyEvidenceUnavailable { .. } => {
                &mut self.policy_evidence_unavailable
            }
        };
        *slot += 1;
    }

    pub fn total_occurrences(&self) -> usize {
        self.tuple_incomplete
            + self.adapter_not_attached_to_desktop
            + self.adapter_enumeration_incomplete
            + self.monitor_enumeration_incomplete
            + self.enumeration_empty_or_unavailable
            + self.enumeration_incomplete
            + self.current_unavailable
            + self.current_changed_during_capture
            + self.current_tuple_incomplete
            + self.current_not_listed
            + self.current_exact_record_ambiguous
            + self.exact_tuple_duplicate
            + self.driver_default_frequency
            + self.current_driver_default_frequency
            + self.bits_per_pixel_below_32
            + self.current_bits_per_pixel_below_32
            + self.known_but_unsupported_display_flags
            + self.current_known_but_unsupported_display_flags
            + self.policy_different
            + self.policy_evidence_unavailable
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QualificationInvariants {
    pub adapter_indices_bounded_and_unique: bool,
    pub candidate_provenance_consistent: bool,
    pub candidate_counts_consistent: bool,
    pub mapping_counts_consistent: bool,
    pub mapping_snapshot_paths_consistent: bool,
    pub observation_counts_consistent: bool,
    pub observation_mapping_paths_consistent: bool,
    pub observation_locations_consistent: bool,
    pub read_only_counters_zero: bool,
}

impl QualificationInvariants {
    pub fn all_satisfied(&self) -> bool {
        self.adapter_indices_bounded_and_unique
            && self.candidate_provenance_consistent
            && self.candidate_counts_consistent
            && self.mapping_counts_consistent
            && self.mapping_snapshot_paths_consistent
            && self.observation_counts_consistent
            && self.observation_mapping_paths_consistent
            && self.observation_locations_consistent
            && self.read_only_counters_zero
    }
}

#[derive(Debug)]
pub struct ReadOnlyQualification {
    pub disposition: Disposition,
    pub mutation_readiness: MutationReadiness,
    pub g1a_gate: G1AGate,
    pub phase_1a_closure: Phase1AClosure,
    pub mapping_status: MappingCaptureStatus,
    pub observation_status: ObservationCaptureStatus,
    pub active_paths: ActivePathAssessment,
    pub inventory_complete: bool,
    pub current_tuple_capture_complete: bool,
    pub non_exact_mapping_paths: usize,
    pub clone_source_paths: usize,
    pub non_exact_observation_paths: usize,
    pub portrait_rotation_paths: usize,
    pub portrait_rotation_exact_paths: usize,
    pub positive_non_integral_refresh_comparisons: usize,
    pub positive_non_integral_refresh_distinct_comparisons: usize,
    pub unqualified_output_technology_paths: usize,
    pub ccd_native_evidence_markers: CcdNativeEvidenceMarkers,
    pub gdi_environment_markers: GdiEnvironmentMarkers,
    pub gdi_active_coverage: GdiActiveCoverage,
    pub candidate_volume: CandidateVolume,
    pub many_candidate_adapters: Vec<u32>,
    pub current_not_listed_adapters: Vec<u32>,
    pub candidate_records: usize,
    pub lab_unqualified_candidates: usize,
    pub active_adapter_lab_unqualified_candidates: usize,
    pub hard_excluded_candidates: usize,
    pub hard_exclusion_histogram: HardExclusionHistogram,
    pub invariants: QualificationInvariants,
    pub blockers: Vec<QualificationBlocker>,
    pub evidence_gaps: &'static [ReadOnlyEvidenceGap],
}

#[cfg(test)]
fn build_read_only_qualification(
    snapshot: Option<&CcdSnapshot>,
    mapping: &MappingCapture,
    observation: &ObservationCapture,
    catalog: &CandidateCatalog,
) -> ReadOnlyQualification {
    let markers = gdi_coverage_markers_for_test(mapping);
    build_read_only_qualification_with_markers(
        snapshot,
        mapping,
        observation,
        catalog,
        markers,
    )
}

#[cfg(test)]
fn gdi_coverage_markers_for_test(mapping: &MappingCapture) -> GdiEnvironmentMarkers {
    let mut markers = GdiEnvironmentMarkers::default();
    let Some(mapping) = mapping.stable_report() else {
        return markers;
    };

    for path in &mapping.paths {
        if path.classification != PathClassification::Exact {
            continue;
        }
        if let SourceMatch::Exact { adapter_index } = &path.source_match {
            if let Some(mask) = bit_for_bounded_index(*adapter_index) {
                markers.attached_adapter_mask |= mask;
            }
        }
        if let TargetMatch::Exact { location } = &path.target_match {
            if let (Ok(adapter_index), Some(monitor_mask)) = (
                usize::try_from(location.adapter_index),
                bit_for_bounded_index(location.monitor_index),
            ) {
                if let Some(adapter_mask) = markers.attached_monitor_masks.get_mut(adapter_index) {
                    *adapter_mask |= monitor_mask;
                }
            }
        }
    }
    markers.attached_adapter_devices = markers.attached_adapter_mask.count_ones() as usize;
    markers.attached_monitor_devices = markers
        .attached_monitor_masks
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .sum();
    markers
}

pub fn build_read_only_qualification_with_markers(
    snapshot: Option<&CcdSnapshot>,
    mapping: &MappingCapture,
    observation: &ObservationCapture,
    catalog: &CandidateCatalog,
    gdi_environment_markers: GdiEnvironmentMarkers,
) -> ReadOnlyQualification {
    let mapping_status = match mapping {
        MappingCapture::SampledStable(_) => MappingCaptureStatus::SampledStable,
        MappingCapture::Unavailable(reason) => MappingCaptureStatus::Unavailable(*reason),
    };
    let observation_status = match observation {
        ObservationCapture::SampledStable(_) => ObservationCaptureStatus::SampledStable,
        ObservationCapture::Unavailable(reason) => {
            ObservationCaptureStatus::Unavailable(*reason)
        }
    };

    let stable_mapping = mapping.stable_report();
    let stable_observation = observation.stable_report();
    let stable_snapshot = match (snapshot, stable_mapping) {
        (Some(snapshot), Some(_)) => Some(snapshot),
        _ => None,
    };

    let active_paths = match stable_snapshot {
        None => ActivePathAssessment::NotObserved,
        Some(snapshot) if snapshot.paths.is_empty() => ActivePathAssessment::NoActivePaths,
        Some(snapshot) if snapshot.paths.len() == 1 => {
            ActivePathAssessment::SingleActivePath
        }
        Some(snapshot) => ActivePathAssessment::MultipleActivePaths {
            count: snapshot.paths.len(),
        },
    };

    let inventory_complete = catalog.adapter_enumeration_status
        == DeviceEnumerationStatus::Complete
        && !catalog.adapters.is_empty()
        && catalog.adapters.iter().all(|adapter| {
            adapter.monitor_enumeration_status == DeviceEnumerationStatus::Complete
                && !matches!(
                    adapter.enumeration_status,
                    ModeEnumerationStatus::LimitReached { .. }
                )
        });
    let mut active_adapter_indices = Vec::with_capacity(MAX_QUALIFICATION_ADAPTERS);
    if let Some(report) = stable_mapping {
        for path in &report.paths {
            if let SourceMatch::Exact { adapter_index } = &path.source_match {
                if !active_adapter_indices.contains(adapter_index)
                    && active_adapter_indices.len() < MAX_QUALIFICATION_ADAPTERS
                {
                    active_adapter_indices.push(*adapter_index);
                }
            }
        }
    }
    let current_tuple_capture_complete = !active_adapter_indices.is_empty()
        && active_adapter_indices.iter().all(|adapter_index| {
            catalog
                .adapters
                .iter()
                .find(|adapter| adapter.adapter_index == *adapter_index)
                .map(|adapter| {
                    adapter.enumeration_status == ModeEnumerationStatus::Complete
                        && matches!(
                            &adapter.current_tuple_status,
                            CurrentTupleStatus::Complete
                        )
                })
                .unwrap_or(false)
        });
    let gdi_active_coverage = assess_gdi_active_coverage(stable_mapping, gdi_environment_markers);

    let non_exact_mapping_paths = stable_mapping
        .map(|report| {
            report
                .paths
                .iter()
                .filter(|path| path.classification != PathClassification::Exact)
                .count()
        })
        .unwrap_or(0);
    let clone_source_paths = stable_mapping
        .map(|report| {
            report
                .paths
                .iter()
                .filter(|path| path.source_endpoint_multiplicity != 1)
                .count()
        })
        .unwrap_or(0);
    let non_exact_observation_paths = stable_observation
        .map(|report| {
            report
                .paths
                .iter()
                .filter(|path| path.classification() != ObservationClassification::Exact)
                .count()
        })
        .unwrap_or(0);

    let mut portrait_rotation_paths = 0;
    let mut portrait_rotation_exact_paths = 0;
    let mut positive_non_integral_refresh_comparisons = 0;
    let mut positive_non_integral_refresh_distinct_comparisons = 0;
    if let Some(report) = stable_observation {
        for path in &report.paths {
            let PathObservation::Observed(path) = path else {
                continue;
            };

            if matches!(path.rotation, Rotation::Rotate90 | Rotation::Rotate270) {
                portrait_rotation_paths += 1;
                if path.classification == ObservationClassification::Exact {
                    portrait_rotation_exact_paths += 1;
                }
            }

            record_fractional_comparison(
                path.ccd_path_refresh,
                path.gdi_vs_ccd_path_refresh,
                &mut positive_non_integral_refresh_comparisons,
                &mut positive_non_integral_refresh_distinct_comparisons,
            );
            if let Some(target_vsync) = path.ccd_target_vsync {
                record_fractional_comparison(
                    target_vsync,
                    path.gdi_vs_ccd_target_vsync,
                    &mut positive_non_integral_refresh_comparisons,
                    &mut positive_non_integral_refresh_distinct_comparisons,
                );
            }
        }
    }

    let unqualified_output_technology_paths = stable_snapshot
        .map(|snapshot| {
            snapshot
                .paths
                .iter()
                .filter(|path| output_technology_is_unqualified(path.target.output_technology))
                .count()
        })
        .unwrap_or(0);
    let ccd_native_evidence_markers = stable_snapshot
        .map(collect_ccd_native_evidence_markers)
        .unwrap_or_default();

    let candidate_records = catalog
        .adapters
        .iter()
        .map(|adapter| adapter.candidates.len())
        .sum();
    let lab_unqualified_candidates = catalog
        .adapters
        .iter()
        .flat_map(|adapter| &adapter.candidates)
        .filter(|candidate| {
            matches!(&candidate.eligibility, CandidateEligibility::LabUnqualified { .. })
        })
        .count();
    let hard_excluded_candidates = candidate_records - lab_unqualified_candidates;
    let active_adapter_lab_unqualified_candidates = catalog
        .adapters
        .iter()
        .filter(|adapter| active_adapter_indices.contains(&adapter.adapter_index))
        .flat_map(|adapter| &adapter.candidates)
        .filter(|candidate| {
            matches!(
                &candidate.eligibility,
                CandidateEligibility::LabUnqualified { .. }
            )
        })
        .count();
    let candidate_volume = match candidate_records {
        0 => CandidateVolume::None,
        1..=8 => CandidateVolume::OneToEight {
            count: candidate_records,
        },
        count => CandidateVolume::NineOrMore { count },
    };

    let mut many_candidate_adapters = Vec::with_capacity(MAX_QUALIFICATION_ADAPTERS);
    let mut current_not_listed_adapters = Vec::with_capacity(MAX_QUALIFICATION_ADAPTERS);
    let mut hard_exclusion_histogram = HardExclusionHistogram::default();
    for adapter in &catalog.adapters {
        if adapter.candidates.len() >= MANY_CANDIDATE_THRESHOLD
            && many_candidate_adapters.len() < MAX_QUALIFICATION_ADAPTERS
        {
            many_candidate_adapters.push(adapter.adapter_index);
        }
        if matches!(&adapter.current_membership, CurrentMembership::NotListedExact { .. })
            && current_not_listed_adapters.len() < MAX_QUALIFICATION_ADAPTERS
        {
            current_not_listed_adapters.push(adapter.adapter_index);
        }
        for candidate in &adapter.candidates {
            if let CandidateEligibility::HardExcluded { reasons } = &candidate.eligibility {
                for reason in reasons {
                    hard_exclusion_histogram.record(reason);
                }
            }
        }
    }

    let invariants = qualification_invariants(snapshot, mapping, observation, catalog);
    let has_non_distinct_fractional_comparison =
        positive_non_integral_refresh_comparisons
            != positive_non_integral_refresh_distinct_comparisons;
    let capture_has_fail_closed_evidence = mapping_failure_is_fail_closed(mapping_status)
        || observation_failure_is_fail_closed(observation_status);
    let capture_has_not_assessable_failure = mapping_failure_is_not_assessable(mapping_status)
        || observation_failure_is_not_assessable(observation_status);
    let unavailable_capture = stable_mapping.is_none() || stable_observation.is_none();
    let mut blockers = Vec::with_capacity(20);
    if !invariants.all_satisfied() {
        blockers.push(QualificationBlocker::InternalInconsistency);
    }
    if let MappingCaptureStatus::Unavailable(reason) = mapping_status {
        blockers.push(QualificationBlocker::MappingCaptureUnavailable(reason));
    }
    if let ObservationCaptureStatus::Unavailable(reason) = observation_status {
        blockers.push(QualificationBlocker::ObservationCaptureUnavailable(reason));
    }
    if !inventory_complete {
        blockers.push(QualificationBlocker::InventoryIncomplete);
    }
    if !current_tuple_capture_complete {
        blockers.push(QualificationBlocker::CurrentTupleCaptureIncomplete);
    }
    match active_paths {
        ActivePathAssessment::NoActivePaths => {
            blockers.push(QualificationBlocker::NoActivePaths);
        }
        ActivePathAssessment::MultipleActivePaths { count } => {
            blockers.push(QualificationBlocker::MultipleActivePaths { count });
        }
        ActivePathAssessment::NotObserved | ActivePathAssessment::SingleActivePath => {}
    }
    if non_exact_mapping_paths != 0 {
        blockers.push(QualificationBlocker::NonExactMappingPaths {
            count: non_exact_mapping_paths,
        });
    }
    if clone_source_paths != 0 {
        blockers.push(QualificationBlocker::CloneSourcePaths {
            count: clone_source_paths,
        });
    }
    if non_exact_observation_paths != 0 {
        blockers.push(QualificationBlocker::NonExactObservationPaths {
            count: non_exact_observation_paths,
        });
    }
    if has_non_distinct_fractional_comparison {
        blockers.push(QualificationBlocker::FractionalRefreshRelationUnresolved {
            comparisons: positive_non_integral_refresh_comparisons,
            distinct: positive_non_integral_refresh_distinct_comparisons,
        });
    }
    if unqualified_output_technology_paths != 0 {
        blockers.push(QualificationBlocker::UnqualifiedOutputTechnologyPaths {
            count: unqualified_output_technology_paths,
        });
    }
    if ccd_native_evidence_markers.any_observed() {
        blockers.push(QualificationBlocker::CcdNativeEvidenceMarkersObserved {
            occurrences: ccd_native_evidence_markers.marker_occurrences(),
        });
    }
    if gdi_environment_markers.any_observed() {
        blockers.push(QualificationBlocker::GdiEnvironmentMarkersObserved {
            occurrences: gdi_environment_markers.marker_occurrences(),
        });
    }
    if matches!(
        gdi_active_coverage,
        GdiActiveCoverage::Assessed {
            consistent: false,
            ..
        }
    ) {
        blockers.push(QualificationBlocker::GdiActiveCoverageMismatch);
    }
    if !current_not_listed_adapters.is_empty() {
        blockers.push(QualificationBlocker::CurrentNotListedAdapters {
            count: current_not_listed_adapters.len(),
        });
    }
    if candidate_records == 0 {
        blockers.push(QualificationBlocker::NoCandidateRecords);
    }
    if active_adapter_lab_unqualified_candidates == 0 {
        blockers.push(
            QualificationBlocker::NoActiveAdapterLabUnqualifiedCandidates {
                active_adapters: active_adapter_indices.len(),
                hard_excluded: hard_excluded_candidates,
            },
        );
    }

    let disposition = if !invariants.all_satisfied() {
        Disposition::NotAssessable
    } else if capture_has_not_assessable_failure {
        Disposition::NotAssessable
    } else if capture_has_fail_closed_evidence {
        Disposition::RejectedByObservedEvidence
    } else if unavailable_capture {
        Disposition::NotAssessable
    } else if !blockers.is_empty() {
        Disposition::RejectedByObservedEvidence
    } else {
        // Step 8 deliberately has no positive readiness outcome. Even the best
        // structurally viable candidate retains the fixed evidence gaps below.
        Disposition::BlockedByMissingEvidence
    };

    ReadOnlyQualification {
        disposition,
        mutation_readiness: MutationReadiness::Blocked,
        g1a_gate: G1AGate::NotReadyEvidenceGaps,
        phase_1a_closure: Phase1AClosure::NotClaimed,
        mapping_status,
        observation_status,
        active_paths,
        inventory_complete,
        current_tuple_capture_complete,
        non_exact_mapping_paths,
        clone_source_paths,
        non_exact_observation_paths,
        portrait_rotation_paths,
        portrait_rotation_exact_paths,
        positive_non_integral_refresh_comparisons,
        positive_non_integral_refresh_distinct_comparisons,
        unqualified_output_technology_paths,
        ccd_native_evidence_markers,
        gdi_environment_markers,
        gdi_active_coverage,
        candidate_volume,
        many_candidate_adapters,
        current_not_listed_adapters,
        candidate_records,
        lab_unqualified_candidates,
        active_adapter_lab_unqualified_candidates,
        hard_excluded_candidates,
        hard_exclusion_histogram,
        invariants,
        blockers,
        evidence_gaps: &READ_ONLY_EVIDENCE_GAPS,
    }
}

fn assess_gdi_active_coverage(
    mapping: Option<&CrossMap>,
    markers: GdiEnvironmentMarkers,
) -> GdiActiveCoverage {
    let Some(mapping) = mapping else {
        return GdiActiveCoverage::NotAssessed;
    };

    let mut exact_source_adapter_mask = 0_u32;
    let mut exact_target_monitor_masks = [0_u32; MAX_QUALIFICATION_ADAPTERS];
    let mut indices_in_range = true;
    for path in &mapping.paths {
        if path.classification != PathClassification::Exact {
            continue;
        }

        match &path.source_match {
            SourceMatch::Exact { adapter_index } => {
                if let Some(mask) = bit_for_bounded_index(*adapter_index) {
                    exact_source_adapter_mask |= mask;
                } else {
                    indices_in_range = false;
                }
            }
            _ => indices_in_range = false,
        }
        match &path.target_match {
            TargetMatch::Exact { location } => {
                if let (Ok(adapter_index), Some(monitor_mask)) = (
                    usize::try_from(location.adapter_index),
                    bit_for_bounded_index(location.monitor_index),
                ) {
                    if let Some(adapter_mask) =
                        exact_target_monitor_masks.get_mut(adapter_index)
                    {
                        *adapter_mask |= monitor_mask;
                    } else {
                        indices_in_range = false;
                    }
                } else {
                    indices_in_range = false;
                }
            }
            _ => indices_in_range = false,
        }
    }

    let exact_source_adapters = exact_source_adapter_mask.count_ones() as usize;
    let exact_target_monitors = exact_target_monitor_masks
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .sum();
    let attached_adapter_mask_count = markers.attached_adapter_mask.count_ones() as usize;
    let attached_monitor_mask_count = markers
        .attached_monitor_masks
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .sum();
    let consistent = indices_in_range
        && markers.attached_adapter_devices == attached_adapter_mask_count
        && markers.attached_monitor_devices == attached_monitor_mask_count
        && markers.attached_adapter_mask == exact_source_adapter_mask
        && markers.attached_monitor_masks == exact_target_monitor_masks;

    GdiActiveCoverage::Assessed {
        attached_adapters: markers.attached_adapter_devices,
        exact_source_adapters,
        attached_monitors: markers.attached_monitor_devices,
        exact_target_monitors,
        consistent,
    }
}

fn mapping_failure_is_fail_closed(status: MappingCaptureStatus) -> bool {
    match status {
        MappingCaptureStatus::SampledStable => false,
        MappingCaptureStatus::Unavailable(
            MappingCaptureFailure::InventoryBoundExceeded
            | MappingCaptureFailure::StaleSnapshot,
        ) => true,
        MappingCaptureStatus::Unavailable(
            MappingCaptureFailure::InitialCcdQueryFailed(reason)
            | MappingCaptureFailure::VerificationCcdQueryFailed(reason),
        ) => ccd_failure_is_fail_closed(reason),
        MappingCaptureStatus::Unavailable(MappingCaptureFailure::InternalInconsistency) => false,
    }
}

fn mapping_failure_is_not_assessable(status: MappingCaptureStatus) -> bool {
    match status {
        MappingCaptureStatus::Unavailable(
            MappingCaptureFailure::InitialCcdQueryFailed(reason)
            | MappingCaptureFailure::VerificationCcdQueryFailed(reason),
        ) => ccd_failure_is_not_assessable(reason),
        MappingCaptureStatus::Unavailable(MappingCaptureFailure::InternalInconsistency) => true,
        MappingCaptureStatus::SampledStable
        | MappingCaptureStatus::Unavailable(
            MappingCaptureFailure::InventoryBoundExceeded
            | MappingCaptureFailure::StaleSnapshot,
        ) => false,
    }
}

fn observation_failure_is_fail_closed(status: ObservationCaptureStatus) -> bool {
    match status {
        ObservationCaptureStatus::Unavailable(ObservationCaptureFailure::StaleSnapshot) => true,
        ObservationCaptureStatus::Unavailable(
            ObservationCaptureFailure::InitialCcdQueryFailed(reason)
            | ObservationCaptureFailure::VerificationCcdQueryFailed(reason),
        ) => ccd_failure_is_fail_closed(reason),
        ObservationCaptureStatus::SampledStable
        | ObservationCaptureStatus::Unavailable(
            ObservationCaptureFailure::CrossMapUnavailable
            | ObservationCaptureFailure::InternalInconsistency,
        ) => false,
    }
}

fn observation_failure_is_not_assessable(status: ObservationCaptureStatus) -> bool {
    match status {
        ObservationCaptureStatus::Unavailable(
            ObservationCaptureFailure::InitialCcdQueryFailed(reason)
            | ObservationCaptureFailure::VerificationCcdQueryFailed(reason),
        ) => ccd_failure_is_not_assessable(reason),
        ObservationCaptureStatus::Unavailable(ObservationCaptureFailure::InternalInconsistency) => {
            true
        }
        ObservationCaptureStatus::SampledStable
        | ObservationCaptureStatus::Unavailable(
            ObservationCaptureFailure::CrossMapUnavailable
            | ObservationCaptureFailure::StaleSnapshot,
        ) => false,
    }
}

fn ccd_failure_is_fail_closed(reason: CcdQueryFailureClass) -> bool {
    matches!(
        reason,
        CcdQueryFailureClass::ConsoleOrDesktopAccessDenied
            | CcdQueryFailureClass::BoundExceeded
            | CcdQueryFailureClass::TopologyRace
            | CcdQueryFailureClass::UnsupportedNativeEvidence
    )
}

fn ccd_failure_is_not_assessable(reason: CcdQueryFailureClass) -> bool {
    matches!(
        reason,
        CcdQueryFailureClass::InvalidNativeEvidence | CcdQueryFailureClass::ApiError
    )
}

fn collect_ccd_native_evidence_markers(
    snapshot: &CcdSnapshot,
) -> CcdNativeEvidenceMarkers {
    let mut markers = CcdNativeEvidenceMarkers::default();

    for path in &snapshot.paths {
        if path.flags != CCD_PATH_ACTIVE {
            markers.non_allowlisted_path_flag_paths += 1;
        }
        if path.flags & CCD_PATH_BOOST_REFRESH_RATE != 0 {
            markers.boost_refresh_rate_paths += 1;
        }
        if path.source.status_flags & !CCD_SOURCE_IN_USE != 0 {
            markers.unknown_source_status_paths += 1;
        }
        if path.target.status_flags != CCD_TARGET_IN_USE {
            markers.non_allowlisted_target_status_paths += 1;
        }
        if path.target.status_flags & !CCD_TARGET_KNOWN_STATUS_MASK != 0 {
            markers.unknown_target_status_paths += 1;
        }
        if path.target.status_flags & CCD_TARGET_IS_HMD != 0 {
            markers.hmd_paths += 1;
        }
        if !matches!(path.target.rotation, 1..=4) {
            markers.unknown_rotation_paths += 1;
        }

        if !matches!(path.target.scaling, 1..=4) {
            markers.non_allowlisted_scaling_paths += 1;
            if !matches!(path.target.scaling, 5 | 128) {
                markers.unknown_scaling_paths += 1;
            }
        }

        let path_scan_line = path.target.scan_line_ordering;
        let mode_scan_line = path
            .target_mode
            .as_ref()
            .map(|mode| mode.scan_line_ordering);
        if path_scan_line != 1 || mode_scan_line.is_some_and(|raw| raw != 1) {
            markers.non_allowlisted_scan_line_paths += 1;
        }
        if !matches!(path_scan_line, 0..=3)
            || mode_scan_line.is_some_and(|raw| !matches!(raw, 0..=3))
        {
            markers.unknown_scan_line_paths += 1;
        }

        if let Some(source_mode) = &path.source_mode {
            if source_mode.pixel_format != 4 {
                markers.non_allowlisted_pixel_format_paths += 1;
            }
            if !matches!(source_mode.pixel_format, 1..=5) {
                markers.unknown_pixel_format_paths += 1;
            }
            if source_mode.pixel_format == 5 {
                markers.non_gdi_pixel_format_paths += 1;
            }
        }
    }

    markers
}

fn record_fractional_comparison(
    rational: Rational,
    relation: ObservationRelation,
    comparisons: &mut usize,
    distinct_comparisons: &mut usize,
) {
    if rational.denominator == 0
        || rational.numerator == 0
        || rational.numerator % rational.denominator == 0
    {
        return;
    }

    *comparisons += 1;
    if relation == ObservationRelation::Distinct {
        *distinct_comparisons += 1;
    }
}

fn output_technology_is_unqualified(raw: i32) -> bool {
    if matches!(
        raw,
        OUTPUT_TECHNOLOGY_OTHER
            | OUTPUT_TECHNOLOGY_MIRACAST
            | OUTPUT_TECHNOLOGY_INDIRECT_WIRED
            | OUTPUT_TECHNOLOGY_INDIRECT_VIRTUAL
            | OUTPUT_TECHNOLOGY_DISPLAYPORT_USB_TUNNEL
            | OUTPUT_TECHNOLOGY_SDTVDONGLE
    ) {
        return true;
    }

    // The narrow diagnostic set is 0..=6 and 8..=13. INTERNAL is
    // also documented. Accepting one here only means that no explicit special or
    // unknown marker was observed; the fixed physical/virtual evidence gap stays.
    !matches!(raw, 0..=6 | 8..=13 | OUTPUT_TECHNOLOGY_INTERNAL)
}

fn qualification_invariants(
    snapshot: Option<&CcdSnapshot>,
    mapping: &MappingCapture,
    observation: &ObservationCapture,
    catalog: &CandidateCatalog,
) -> QualificationInvariants {
    let mut seen_adapter_indices = [false; MAX_QUALIFICATION_ADAPTERS];
    let mut adapter_indices_bounded_and_unique =
        catalog.adapters.len() <= MAX_QUALIFICATION_ADAPTERS;
    let mut candidate_provenance_consistent = true;
    let mut candidate_counts_consistent = true;
    let mut computed_total = CandidateSummary::default();

    for adapter in &catalog.adapters {
        let index = usize::try_from(adapter.adapter_index).ok();
        match index.filter(|index| *index < MAX_QUALIFICATION_ADAPTERS) {
            Some(index) if !seen_adapter_indices[index] => {
                seen_adapter_indices[index] = true;
            }
            _ => adapter_indices_bounded_and_unique = false,
        }

        if adapter
            .candidates
            .iter()
            .any(|candidate| candidate.provenance.adapter_index != adapter.adapter_index)
        {
            candidate_provenance_consistent = false;
        }

        let computed = summarize_adapter(adapter);
        if computed != adapter.summary {
            candidate_counts_consistent = false;
        }
        add_summary(&mut computed_total, computed);
    }
    if computed_total != catalog.summary {
        candidate_counts_consistent = false;
    }

    let mapping_counts_consistent = mapping.stable_report().map_or(true, |report| {
        let exact = report
            .paths
            .iter()
            .filter(|path| path.classification == PathClassification::Exact)
            .count();
        let unmapped = report
            .paths
            .iter()
            .filter(|path| path.classification == PathClassification::Unmapped)
            .count();
        let ambiguous = report
            .paths
            .iter()
            .filter(|path| path.classification == PathClassification::Ambiguous)
            .count();
        let inconsistent = report
            .paths
            .iter()
            .filter(|path| path.classification == PathClassification::Inconsistent)
            .count();
        exact == report.exact_paths
            && unmapped == report.unmapped_paths
            && ambiguous == report.ambiguous_paths
            && inconsistent == report.inconsistent_paths
            && exact + unmapped + ambiguous + inconsistent == report.paths.len()
    });
    let mapping_snapshot_paths_consistent = match (mapping.stable_report(), snapshot) {
        (Some(report), Some(snapshot)) => {
            let mut source_multiplicities = HashMap::with_capacity(snapshot.paths.len());
            let mut target_multiplicities = HashMap::with_capacity(snapshot.paths.len());
            for path in &snapshot.paths {
                *source_multiplicities
                    .entry((path.source.adapter_luid.as_u64(), path.source.id))
                    .or_insert(0_usize) += 1;
                *target_multiplicities
                    .entry((path.target.adapter_luid.as_u64(), path.target.id))
                    .or_insert(0_usize) += 1;
            }

            report.paths.len() == snapshot.paths.len()
                && indices_form_exact_range(
                    snapshot.paths.len(),
                    snapshot.paths.iter().map(|path| path.index),
                )
                && indices_form_exact_range(
                    report.paths.len(),
                    report.paths.iter().map(|path| path.path_index),
                )
                && report.paths.iter().all(|mapping_path| {
                    snapshot
                        .paths
                        .get(mapping_path.path_index)
                        .map(|snapshot_path| {
                            snapshot_path.index == mapping_path.path_index
                                && source_multiplicities
                                    .get(&(
                                        snapshot_path.source.adapter_luid.as_u64(),
                                        snapshot_path.source.id,
                                    ))
                                    .copied()
                                    == Some(mapping_path.source_endpoint_multiplicity)
                                && target_multiplicities
                                    .get(&(
                                        snapshot_path.target.adapter_luid.as_u64(),
                                        snapshot_path.target.id,
                                    ))
                                    .copied()
                                    == Some(mapping_path.target_endpoint_multiplicity)
                        })
                        .unwrap_or(false)
                })
        }
        (Some(_), None) => false,
        (None, _) => true,
    };

    let observation_counts_consistent = observation.stable_report().map_or(true, |report| {
        let exact = report
            .paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Exact)
            .count();
        let distinct = report
            .paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Distinct)
            .count();
        let mismatch = report
            .paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Mismatch)
            .count();
        let unavailable = report
            .paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Unavailable)
            .count();
        exact == report.exact_paths
            && distinct == report.distinct_paths
            && mismatch == report.mismatch_paths
            && unavailable == report.unavailable_paths
            && exact + distinct + mismatch + unavailable == report.paths.len()
    });
    let observation_mapping_paths_consistent =
        match (observation.stable_report(), mapping.stable_report()) {
            (Some(observation), Some(mapping)) => {
                observation.paths.len() == mapping.paths.len()
                    && indices_form_exact_range(
                        observation.paths.len(),
                        observation.paths.iter().map(path_observation_index),
                    )
                    && indices_form_exact_range(
                        mapping.paths.len(),
                        mapping.paths.iter().map(|path| path.path_index),
                    )
            }
            (Some(_), None) => false,
            (None, _) => true,
        };
    let observation_locations_consistent =
        match (observation.stable_report(), mapping.stable_report()) {
            (Some(observation), Some(mapping)) => observation.paths.iter().all(|path| {
                let path_index = path_observation_index(path);
                let Some(mapping_path) = mapping
                    .paths
                    .get(path_index)
                    .filter(|mapping_path| mapping_path.path_index == path_index)
                else {
                    return false;
                };

                match path {
                    PathObservation::Observed(observed) => matches!(
                        (&mapping_path.source_match, &mapping_path.target_match),
                        (
                            SourceMatch::Exact { adapter_index },
                            crate::mapping::TargetMatch::Exact { location }
                        ) if mapping_path.classification == PathClassification::Exact
                            && *adapter_index == observed.adapter_index
                            && location.adapter_index == observed.adapter_index
                            && location.monitor_index == observed.monitor_index
                    ),
                    PathObservation::Unavailable { .. } => true,
                }
            }),
            (Some(_), None) => false,
            (None, _) => true,
        };

    let read_only_counters_zero = catalog.summary.product_allowed_records == 0
        && catalog.summary.selection_tokens_issued == 0
        && catalog.adapters.iter().all(|adapter| {
            adapter.summary.product_allowed_records == 0
                && adapter.summary.selection_tokens_issued == 0
        });

    QualificationInvariants {
        adapter_indices_bounded_and_unique,
        candidate_provenance_consistent,
        candidate_counts_consistent,
        mapping_counts_consistent,
        mapping_snapshot_paths_consistent,
        observation_counts_consistent,
        observation_mapping_paths_consistent,
        observation_locations_consistent,
        read_only_counters_zero,
    }
}

fn path_observation_index(path: &PathObservation) -> usize {
    match path {
        PathObservation::Observed(path) => path.path_index,
        PathObservation::Unavailable { path_index, .. } => *path_index,
    }
}

fn indices_form_exact_range(
    expected_len: usize,
    indices: impl Iterator<Item = usize>,
) -> bool {
    let mut seen = vec![false; expected_len];
    for index in indices {
        let Some(slot) = seen.get_mut(index) else {
            return false;
        };
        if *slot {
            return false;
        }
        *slot = true;
    }
    seen.into_iter().all(|value| value)
}

fn summarize_adapter(adapter: &crate::candidate::AdapterCandidateCatalog) -> CandidateSummary {
    let complete_records = adapter
        .candidates
        .iter()
        .filter(|candidate| candidate.tuple_status == TupleStatus::Complete)
        .count();
    let exact_duplicate_records = adapter
        .candidates
        .iter()
        .filter(|candidate| {
            matches!(
                &candidate.exact_duplicate,
                crate::candidate::ExactDuplicateStatus::ExactTupleDuplicate { .. }
            )
        })
        .count();
    let projection_collision_records = adapter
        .candidates
        .iter()
        .filter(|candidate| candidate.projection_collision.is_some())
        .count();
    let lab_unqualified_records = adapter
        .candidates
        .iter()
        .filter(|candidate| {
            matches!(&candidate.eligibility, CandidateEligibility::LabUnqualified { .. })
        })
        .count();

    CandidateSummary {
        records: adapter.candidates.len(),
        complete_records,
        incomplete_records: adapter.candidates.len() - complete_records,
        exact_duplicate_groups: adapter.exact_duplicate_groups.len(),
        exact_duplicate_records,
        projection_collision_records,
        lab_unqualified_records,
        hard_excluded_records: adapter.candidates.len() - lab_unqualified_records,
        product_allowed_records: 0,
        selection_tokens_issued: 0,
    }
}

fn add_summary(total: &mut CandidateSummary, value: CandidateSummary) {
    total.records += value.records;
    total.complete_records += value.complete_records;
    total.incomplete_records += value.incomplete_records;
    total.exact_duplicate_groups += value.exact_duplicate_groups;
    total.exact_duplicate_records += value.exact_duplicate_records;
    total.projection_collision_records += value.projection_collision_records;
    total.lab_unqualified_records += value.lab_unqualified_records;
    total.hard_excluded_records += value.hard_excluded_records;
    total.product_allowed_records += value.product_allowed_records;
    total.selection_tokens_issued += value.selection_tokens_issued;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        candidate::{
            AdapterCandidateCatalog, AdvancedColorEvidence, ApplyTuple, CandidateIdentity,
            CurrentRelation, DisplayLabel, EnumerationProvenance, ExactDuplicateStatus,
            ExpectedObservationStatus, FrequencyLabel, ModeCandidate, PolicyRelations,
            QualificationGap,
        },
        ccd::{
            AdapterLuid, CcdPath, CcdSource, CcdSourceMode, CcdTarget, CcdTargetMode,
        },
        mapping::{MonitorLocation, PathMapping, SourceMatch, TargetMatch},
        observation::{CurrentObservation, GdiRefresh},
    };

    fn candidate(eligibility: CandidateEligibility, index: u32) -> ModeCandidate {
        ModeCandidate {
            candidate_identity: CandidateIdentity::NotIssuedReadOnlyStep7,
            provenance: EnumerationProvenance {
                adapter_index: 0,
                enumeration_index: index,
            },
            display_label: DisplayLabel {
                width_pixels: Some(1920),
                height_pixels: Some(1080),
                frequency: FrequencyLabel::Hertz(60),
            },
            public_size_bytes: 220,
            driver_extra_bytes: 0,
            apply_tuple: ApplyTuple {
                field_mask: 0,
                position: None,
                orientation: None,
                fixed_output: None,
                bits_per_pixel: Some(32),
                width_pixels: Some(1920),
                height_pixels: Some(1080),
                display_flags: Some(0),
                display_frequency_hz: Some(60),
            },
            tuple_status: TupleStatus::Complete,
            tuple_issues: Vec::new(),
            exact_duplicate: ExactDuplicateStatus::Unique,
            projection_collision: None,
            current_relation: CurrentRelation::Different,
            policy_relations: PolicyRelations {
                position: crate::candidate::FieldRelation::Exact,
                orientation: crate::candidate::FieldRelation::Exact,
                fixed_output: crate::candidate::FieldRelation::Exact,
                bits_per_pixel: crate::candidate::FieldRelation::Exact,
                display_flags: crate::candidate::FieldRelation::Exact,
            },
            advanced_color_evidence: AdvancedColorEvidence::NotObserved,
            expected_observation:
                ExpectedObservationStatus::MissingNonCurrentRequiresQualification,
            eligibility,
        }
    }

    fn catalog_with_candidates(
        candidates: Vec<ModeCandidate>,
        membership: CurrentMembership,
    ) -> CandidateCatalog {
        let lab = candidates
            .iter()
            .filter(|candidate| {
                matches!(&candidate.eligibility, CandidateEligibility::LabUnqualified { .. })
            })
            .count();
        let summary = CandidateSummary {
            records: candidates.len(),
            complete_records: candidates.len(),
            incomplete_records: 0,
            exact_duplicate_groups: 0,
            exact_duplicate_records: 0,
            projection_collision_records: 0,
            lab_unqualified_records: lab,
            hard_excluded_records: candidates.len() - lab,
            product_allowed_records: 0,
            selection_tokens_issued: 0,
        };
        CandidateCatalog {
            adapter_enumeration_status: DeviceEnumerationStatus::Complete,
            adapters: vec![AdapterCandidateCatalog {
                adapter_index: 0,
                device_name: r"\\.\DISPLAY1".to_owned(),
                monitor_enumeration_status: DeviceEnumerationStatus::Complete,
                enumeration_status: ModeEnumerationStatus::Complete,
                current_tuple_status: CurrentTupleStatus::Complete,
                current_membership: membership,
                exact_duplicate_groups: Vec::new(),
                projection_collision_groups: Vec::new(),
                candidates,
                summary,
            }],
            summary,
        }
    }

    fn lab_catalog(count: usize) -> CandidateCatalog {
        let candidates = (0..count)
            .map(|index| {
                candidate(
                    CandidateEligibility::LabUnqualified {
                        gaps: vec![QualificationGap::ExpectedObservationMissing],
                    },
                    u32::try_from(index).unwrap(),
                )
            })
            .collect();
        catalog_with_candidates(
            candidates,
            CurrentMembership::ListedUnique { mode_index: 0 },
        )
    }

    fn add_detached_empty_adapter(
        catalog: &mut CandidateCatalog,
        enumeration_status: ModeEnumerationStatus,
    ) {
        let current_membership = match enumeration_status {
            ModeEnumerationStatus::LimitReached { limit } => {
                CurrentMembership::EnumerationIncomplete { limit }
            }
            ModeEnumerationStatus::Complete | ModeEnumerationStatus::EmptyOrUnavailable => {
                CurrentMembership::EnumerationEmptyOrUnavailable
            }
        };
        catalog.adapters.push(AdapterCandidateCatalog {
            adapter_index: 1,
            device_name: r"\\.\DISPLAY2".to_owned(),
            monitor_enumeration_status: DeviceEnumerationStatus::Complete,
            enumeration_status,
            current_tuple_status: CurrentTupleStatus::Unavailable,
            current_membership,
            exact_duplicate_groups: Vec::new(),
            projection_collision_groups: Vec::new(),
            candidates: Vec::new(),
            summary: CandidateSummary::default(),
        });
    }

    fn add_non_active_lab_adapter(catalog: &mut CandidateCatalog) {
        let mut mode = candidate(
            CandidateEligibility::LabUnqualified {
                gaps: vec![QualificationGap::ExpectedObservationMissing],
            },
            0,
        );
        mode.provenance.adapter_index = 1;
        let summary = CandidateSummary {
            records: 1,
            complete_records: 1,
            incomplete_records: 0,
            exact_duplicate_groups: 0,
            exact_duplicate_records: 0,
            projection_collision_records: 0,
            lab_unqualified_records: 1,
            hard_excluded_records: 0,
            product_allowed_records: 0,
            selection_tokens_issued: 0,
        };
        catalog.adapters.push(AdapterCandidateCatalog {
            adapter_index: 1,
            device_name: r"\\.\DISPLAY2".to_owned(),
            monitor_enumeration_status: DeviceEnumerationStatus::Complete,
            enumeration_status: ModeEnumerationStatus::Complete,
            current_tuple_status: CurrentTupleStatus::Complete,
            current_membership: CurrentMembership::ListedUnique { mode_index: 0 },
            exact_duplicate_groups: Vec::new(),
            projection_collision_groups: Vec::new(),
            candidates: vec![mode],
            summary,
        });
        add_summary(&mut catalog.summary, summary);
    }

    fn snapshot(path_count: usize, output_technology: i32) -> CcdSnapshot {
        CcdSnapshot {
            paths: (0..path_count)
                .map(|index| CcdPath {
                    index,
                    source: CcdSource {
                        adapter_luid: AdapterLuid {
                            low_part: 1,
                            high_part: 0,
                        },
                        id: u32::try_from(index).unwrap(),
                        gdi_device_name: Some(r"\\.\DISPLAY1".to_owned()),
                        gdi_device_name_key: Some(vec![1]),
                        mode_info_index: Some(0),
                        status_flags: 1,
                    },
                    target: CcdTarget {
                        adapter_luid: AdapterLuid {
                            low_part: 1,
                            high_part: 0,
                        },
                        id: u32::try_from(index).unwrap(),
                        friendly_name: "monitor".to_owned(),
                        device_path: Some("path".to_owned()),
                        device_path_key: Some(vec![2]),
                        device_name_flags: 0,
                        metadata_output_technology: output_technology,
                        edid_manufacture_id: 0,
                        edid_product_code_id: 0,
                        connector_instance: 0,
                        mode_info_index: Some(0),
                        output_technology,
                        rotation: 1,
                        scaling: 1,
                        refresh_rate: Rational {
                            numerator: 60,
                            denominator: 1,
                        },
                        scan_line_ordering: 1,
                        available: true,
                        status_flags: 1,
                    },
                    source_mode: Some(CcdSourceMode {
                        width_pixels: 1920,
                        height_pixels: 1080,
                        pixel_format: 4,
                        position_x: i32::try_from(index).unwrap() * 1920,
                        position_y: 0,
                    }),
                    target_mode: Some(CcdTargetMode {
                        pixel_rate: 148_500_000,
                        horizontal_sync: Rational {
                            numerator: 67_500,
                            denominator: 1,
                        },
                        vertical_sync: Rational {
                            numerator: 60,
                            denominator: 1,
                        },
                        active_width_pixels: 1920,
                        active_height_pixels: 1080,
                        total_width_pixels: 2200,
                        total_height_pixels: 1125,
                        scan_line_ordering: 1,
                    }),
                    flags: 1,
                })
                .collect(),
        }
    }

    fn mapping(path_count: usize, source_multiplicity: usize) -> MappingCapture {
        let paths = (0..path_count)
            .map(|index| PathMapping {
                path_index: index,
                source_match: SourceMatch::Exact { adapter_index: 0 },
                target_match: TargetMatch::Exact {
                    location: MonitorLocation {
                        adapter_index: 0,
                        monitor_index: u32::try_from(index).unwrap(),
                    },
                },
                source_attached_to_desktop: Some(true),
                parent_adapter_consistent: Some(true),
                target_attached_to_desktop: Some(true),
                output_technology_consistent: true,
                source_endpoint_multiplicity: source_multiplicity,
                source_endpoint_identity_consistent: true,
                source_in_use: true,
                target_endpoint_multiplicity: 1,
                target_available: true,
                target_in_use: true,
                target_forced_availability: false,
                target_friendly_name_forced: false,
                target_name_has_unknown_flags: false,
                path_active: true,
                classification: PathClassification::Exact,
            })
            .collect();
        MappingCapture::SampledStable(CrossMap {
            paths,
            exact_paths: path_count,
            unmapped_paths: 0,
            ambiguous_paths: 0,
            inconsistent_paths: 0,
        })
    }

    fn observed_path(
        index: usize,
        rotation: Rotation,
        path_refresh: Rational,
        refresh_relation: ObservationRelation,
        classification: ObservationClassification,
    ) -> PathObservation {
        PathObservation::Observed(CurrentObservation {
            path_index: index,
            adapter_index: 0,
            monitor_index: u32::try_from(index).unwrap(),
            device_name: r"\\.\DISPLAY1".to_owned(),
            friendly_label: "monitor".to_owned(),
            rotation,
            scaling_raw: 1,
            gdi_resolution: None,
            ccd_source_resolution: None,
            rotation_applied_source_resolution: None,
            ccd_target_active_resolution: None,
            desktop_resolution_relation: ObservationRelation::Exact,
            source_target_resolution_relation: ObservationRelation::Exact,
            gdi_refresh: GdiRefresh::Hertz(60),
            ccd_path_refresh: path_refresh,
            ccd_target_vsync: None,
            gdi_vs_ccd_path_refresh: refresh_relation,
            gdi_vs_ccd_target_vsync: ObservationRelation::Exact,
            ccd_path_vs_target_vsync: ObservationRelation::Exact,
            classification,
        })
    }

    fn observation(paths: Vec<PathObservation>) -> ObservationCapture {
        let exact = paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Exact)
            .count();
        let distinct = paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Distinct)
            .count();
        let mismatch = paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Mismatch)
            .count();
        let unavailable = paths
            .iter()
            .filter(|path| path.classification() == ObservationClassification::Unavailable)
            .count();
        ObservationCapture::SampledStable(CurrentObservationReport {
            paths,
            exact_paths: exact,
            distinct_paths: distinct,
            mismatch_paths: mismatch,
            unavailable_paths: unavailable,
        })
    }

    fn exact_observation(path_count: usize) -> ObservationCapture {
        observation(
            (0..path_count)
                .map(|index| {
                    observed_path(
                        index,
                        Rotation::Identity,
                        Rational {
                            numerator: 60,
                            denominator: 1,
                        },
                        ObservationRelation::Exact,
                        ObservationClassification::Exact,
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn stable_report_helpers_do_not_expose_unavailable_payloads() {
        assert!(mapping(1, 1).stable_report().is_some());
        assert!(MappingCapture::Unavailable(MappingCaptureFailure::StaleSnapshot)
            .stable_report()
            .is_none());
        assert!(exact_observation(1).stable_report().is_some());
        assert!(ObservationCapture::Unavailable(ObservationCaptureFailure::StaleSnapshot)
            .stable_report()
            .is_none());
    }

    #[test]
    fn best_read_only_result_is_blocked_by_missing_evidence_and_never_ready() {
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &lab_catalog(1),
        );

        assert_eq!(
            report.disposition,
            Disposition::BlockedByMissingEvidence
        );
        assert_eq!(report.mutation_readiness, MutationReadiness::Blocked);
        assert_eq!(report.g1a_gate, G1AGate::NotReadyEvidenceGaps);
        assert_eq!(report.phase_1a_closure, Phase1AClosure::NotClaimed);
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn stale_mapping_is_explicit_fail_closed_evidence() {
        let report = build_read_only_qualification(
            None,
            &MappingCapture::Unavailable(MappingCaptureFailure::StaleSnapshot),
            &ObservationCapture::Unavailable(ObservationCaptureFailure::CrossMapUnavailable),
            &lab_catalog(1),
        );
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn ordinary_api_failure_is_not_assessable() {
        let report = build_read_only_qualification(
            None,
            &MappingCapture::Unavailable(MappingCaptureFailure::InitialCcdQueryFailed(
                CcdQueryFailureClass::ApiError,
            )),
            &ObservationCapture::Unavailable(
                ObservationCaptureFailure::InitialCcdQueryFailed(
                    CcdQueryFailureClass::ApiError,
                ),
            ),
            &lab_catalog(1),
        );
        assert_eq!(report.disposition, Disposition::NotAssessable);
    }

    #[test]
    fn access_denied_is_rejected_without_claiming_rdp() {
        let report = build_read_only_qualification(
            None,
            &MappingCapture::Unavailable(MappingCaptureFailure::InitialCcdQueryFailed(
                CcdQueryFailureClass::ConsoleOrDesktopAccessDenied,
            )),
            &ObservationCapture::Unavailable(
                ObservationCaptureFailure::InitialCcdQueryFailed(
                    CcdQueryFailureClass::ConsoleOrDesktopAccessDenied,
                ),
            ),
            &lab_catalog(1),
        );
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
        assert!(report
            .evidence_gaps
            .contains(&ReadOnlyEvidenceGap::SessionAndRdpEvidenceNotObserved));
    }

    #[test]
    fn invalid_native_capture_is_not_assessable() {
        let report = build_read_only_qualification(
            None,
            &MappingCapture::Unavailable(MappingCaptureFailure::InitialCcdQueryFailed(
                CcdQueryFailureClass::InvalidNativeEvidence,
            )),
            &ObservationCapture::Unavailable(
                ObservationCaptureFailure::InitialCcdQueryFailed(
                    CcdQueryFailureClass::InvalidNativeEvidence,
                ),
            ),
            &lab_catalog(1),
        );
        assert_eq!(report.disposition, Disposition::NotAssessable);
    }

    #[test]
    fn exact_multi_path_capture_still_fails_closed() {
        let snapshot = snapshot(3, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(3, 1),
            &exact_observation(3),
            &lab_catalog(9),
        );
        assert_eq!(
            report.active_paths,
            ActivePathAssessment::MultipleActivePaths { count: 3 }
        );
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn clone_source_multiplicity_blocks_an_exact_cross_map() {
        let mut snapshot = snapshot(2, 5);
        snapshot.paths[1].source.id = snapshot.paths[0].source.id;
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(2, 2),
            &exact_observation(2),
            &lab_catalog(1),
        );
        assert_eq!(report.clone_source_paths, 2);
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn exact_portrait_rotation_is_recorded_without_qualifying_mutation() {
        let snapshot = snapshot(1, 5);
        let observation = observation(vec![observed_path(
            0,
            Rotation::Rotate90,
            Rational {
                numerator: 60,
                denominator: 1,
            },
            ObservationRelation::Exact,
            ObservationClassification::Exact,
        )]);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &observation,
            &lab_catalog(1),
        );
        assert_eq!(report.portrait_rotation_paths, 1);
        assert_eq!(report.portrait_rotation_exact_paths, 1);
        assert_eq!(
            report.disposition,
            Disposition::BlockedByMissingEvidence
        );
    }

    #[test]
    fn positive_fractional_refresh_must_remain_distinct() {
        let snapshot = snapshot(1, 5);
        let observation = observation(vec![observed_path(
            0,
            Rotation::Identity,
            Rational {
                numerator: 60_000,
                denominator: 1_001,
            },
            ObservationRelation::Distinct,
            ObservationClassification::Distinct,
        )]);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &observation,
            &lab_catalog(1),
        );
        assert_eq!(report.positive_non_integral_refresh_comparisons, 1);
        assert_eq!(
            report.positive_non_integral_refresh_distinct_comparisons,
            1
        );
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn fractional_refresh_that_is_not_distinct_fails_closed() {
        let snapshot = snapshot(1, 5);
        let observation = observation(vec![observed_path(
            0,
            Rotation::Identity,
            Rational {
                numerator: 60_000,
                denominator: 1_001,
            },
            ObservationRelation::Exact,
            ObservationClassification::Exact,
        )]);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &observation,
            &lab_catalog(1),
        );
        assert_eq!(report.positive_non_integral_refresh_comparisons, 1);
        assert_eq!(
            report.positive_non_integral_refresh_distinct_comparisons,
            0
        );
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn nine_candidates_are_classified_as_many() {
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &lab_catalog(9),
        );
        assert_eq!(report.candidate_volume, CandidateVolume::NineOrMore { count: 9 });
        assert_eq!(report.many_candidate_adapters, vec![0]);
    }

    #[test]
    fn detached_empty_modes_are_allowed_but_any_mode_limit_is_not() {
        let snapshot = snapshot(1, 5);
        let mut empty_catalog = lab_catalog(1);
        add_detached_empty_adapter(
            &mut empty_catalog,
            ModeEnumerationStatus::EmptyOrUnavailable,
        );
        let empty_report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &empty_catalog,
        );
        assert!(empty_report.inventory_complete);
        assert!(empty_report.current_tuple_capture_complete);
        assert_eq!(
            empty_report.disposition,
            Disposition::BlockedByMissingEvidence
        );

        let mut limited_catalog = lab_catalog(1);
        add_detached_empty_adapter(
            &mut limited_catalog,
            ModeEnumerationStatus::LimitReached { limit: 4096 },
        );
        let limited_report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &limited_catalog,
        );
        assert!(!limited_report.inventory_complete);
        assert_eq!(
            limited_report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn current_not_listed_all_hard_excluded_is_negative_success() {
        let candidates = (0..619)
            .map(|index| {
                candidate(
                    CandidateEligibility::HardExcluded {
                        reasons: vec![HardExclusion::CurrentNotListed],
                    },
                    index,
                )
            })
            .collect();
        let catalog = catalog_with_candidates(
            candidates,
            CurrentMembership::NotListedExact {
                projection_only_indices: Vec::new(),
            },
        );
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &catalog,
        );
        assert_eq!(report.candidate_records, 619);
        assert_eq!(report.hard_excluded_candidates, 619);
        assert_eq!(report.lab_unqualified_candidates, 0);
        assert_eq!(report.hard_exclusion_histogram.current_not_listed, 619);
        assert_eq!(report.current_not_listed_adapters, vec![0]);
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn non_active_lab_candidate_cannot_satisfy_the_active_adapter_gate() {
        let active_candidate = candidate(
            CandidateEligibility::HardExcluded {
                reasons: vec![HardExclusion::CurrentNotListed],
            },
            0,
        );
        let mut catalog = catalog_with_candidates(
            vec![active_candidate],
            CurrentMembership::NotListedExact {
                projection_only_indices: Vec::new(),
            },
        );
        add_non_active_lab_adapter(&mut catalog);
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &catalog,
        );
        assert_eq!(report.lab_unqualified_candidates, 1);
        assert_eq!(report.active_adapter_lab_unqualified_candidates, 0);
        assert!(report.blockers.iter().any(|blocker| matches!(
            blocker,
            QualificationBlocker::NoActiveAdapterLabUnqualifiedCandidates { .. }
        )));
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn special_or_indirect_output_is_a_blocker_not_physical_evidence() {
        for technology in [
            OUTPUT_TECHNOLOGY_OTHER,
            OUTPUT_TECHNOLOGY_MIRACAST,
            OUTPUT_TECHNOLOGY_INDIRECT_WIRED,
            OUTPUT_TECHNOLOGY_INDIRECT_VIRTUAL,
            OUTPUT_TECHNOLOGY_DISPLAYPORT_USB_TUNNEL,
            OUTPUT_TECHNOLOGY_SDTVDONGLE,
            7,
            19,
        ] {
            let snapshot = snapshot(1, technology);
            let report = build_read_only_qualification(
                Some(&snapshot),
                &mapping(1, 1),
                &exact_observation(1),
                &lab_catalog(1),
            );
            assert_eq!(report.unqualified_output_technology_paths, 1);
            assert_eq!(
                report.disposition,
                Disposition::RejectedByObservedEvidence
            );
        }
    }

    #[test]
    fn special_and_unknown_ccd_values_cannot_reach_missing_evidence_only() {
        let mut snapshot = snapshot(1, 5);
        let path = &mut snapshot.paths[0];
        path.flags |= CCD_PATH_BOOST_REFRESH_RATE;
        path.source.status_flags |= 1 << 31;
        path.target.status_flags |= CCD_TARGET_IS_HMD | (1 << 30);
        path.target.scaling = 999;
        path.target.scan_line_ordering = 0;
        path.source_mode.as_mut().unwrap().pixel_format = 5;

        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &lab_catalog(1),
        );

        assert_eq!(report.ccd_native_evidence_markers.boost_refresh_rate_paths, 1);
        assert_eq!(report.ccd_native_evidence_markers.unknown_source_status_paths, 1);
        assert_eq!(report.ccd_native_evidence_markers.unknown_target_status_paths, 1);
        assert_eq!(report.ccd_native_evidence_markers.hmd_paths, 1);
        assert_eq!(report.ccd_native_evidence_markers.unknown_scaling_paths, 1);
        assert_eq!(report.ccd_native_evidence_markers.non_gdi_pixel_format_paths, 1);
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn positive_gdi_remote_marker_fails_closed_without_claiming_rdp() {
        let snapshot = snapshot(1, 5);
        let mapping = mapping(1, 1);
        let mut markers = gdi_coverage_markers_for_test(&mapping);
        markers.remote_sdk_devices = 1;
        let report = build_read_only_qualification_with_markers(
            Some(&snapshot),
            &mapping,
            &exact_observation(1),
            &lab_catalog(1),
            markers,
        );
        assert_eq!(report.gdi_environment_markers.remote_sdk_devices, 1);
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
        assert!(report
            .evidence_gaps
            .contains(&ReadOnlyEvidenceGap::SessionAndRdpEvidenceNotObserved));
    }

    #[test]
    fn gdi_state_flag_classifier_separates_context_and_unknown_bits() {
        let mut info = crate::display::DisplayDeviceInfo {
            device_name: String::new(),
            device_string: String::new(),
            device_id: String::new(),
            device_key: String::new(),
            is_primary: false,
            is_attached_to_desktop: false,
            state_flags_raw: 0x0000_0002,
            mirroring_driver_marker: false,
            remote_sdk_marker: false,
            rdpudd_sdk_marker: false,
        };

        let mut adapter_markers = GdiEnvironmentMarkers::default();
        record_gdi_environment_marker(
            &mut adapter_markers,
            &info,
            GDI_ADAPTER_STATE_FLAGS_DIAGNOSTIC_MASK,
        );
        assert_eq!(adapter_markers.known_unqualified_state_flag_devices, 1);

        let mut monitor_markers = GdiEnvironmentMarkers::default();
        record_gdi_environment_marker(
            &mut monitor_markers,
            &info,
            GDI_MONITOR_STATE_FLAGS_DIAGNOSTIC_MASK,
        );
        assert_eq!(monitor_markers.known_unqualified_state_flag_devices, 0);

        info.state_flags_raw = 1 << 31;
        let mut unknown_markers = GdiEnvironmentMarkers::default();
        record_gdi_environment_marker(
            &mut unknown_markers,
            &info,
            GDI_ADAPTER_STATE_FLAGS_DIAGNOSTIC_MASK,
        );
        assert_eq!(unknown_markers.unknown_state_flag_devices, 1);
    }

    #[test]
    fn extra_gdi_attached_devices_are_rejected_by_reverse_coverage() {
        let snapshot = snapshot(1, 5);
        let mapping = mapping(1, 1);
        let mut markers = gdi_coverage_markers_for_test(&mapping);
        markers.attached_adapter_mask |= 1 << 1;
        markers.attached_adapter_devices += 1;
        markers.attached_monitor_masks[1] |= 1;
        markers.attached_monitor_devices += 1;

        let report = build_read_only_qualification_with_markers(
            Some(&snapshot),
            &mapping,
            &exact_observation(1),
            &lab_catalog(1),
            markers,
        );

        assert!(matches!(
            report.gdi_active_coverage,
            GdiActiveCoverage::Assessed {
                attached_adapters: 2,
                exact_source_adapters: 1,
                attached_monitors: 2,
                exact_target_monitors: 1,
                consistent: false,
            }
        ));
        assert!(report
            .blockers
            .contains(&QualificationBlocker::GdiActiveCoverageMismatch));
        assert_eq!(
            report.disposition,
            Disposition::RejectedByObservedEvidence
        );
    }

    #[test]
    fn nonzero_read_only_allow_counters_are_internal_inconsistency() {
        let mut catalog = lab_catalog(1);
        catalog.adapters[0].summary.product_allowed_records = 1;
        catalog.summary.product_allowed_records = 1;
        let mut snapshot = snapshot(1, 5);
        snapshot.paths[0].flags |= CCD_PATH_BOOST_REFRESH_RATE;
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &catalog,
        );
        assert!(!report.invariants.read_only_counters_zero);
        assert_eq!(report.ccd_native_evidence_markers.boost_refresh_rate_paths, 1);
        assert_eq!(report.disposition, Disposition::NotAssessable);
    }

    #[test]
    fn mapping_count_mismatch_is_not_assessable() {
        let mut mapping = mapping(1, 1);
        let MappingCapture::SampledStable(report) = &mut mapping else {
            unreachable!();
        };
        report.exact_paths = 0;
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping,
            &exact_observation(1),
            &lab_catalog(1),
        );
        assert!(!report.invariants.mapping_counts_consistent);
        assert_eq!(report.disposition, Disposition::NotAssessable);
    }

    #[test]
    fn observation_count_mismatch_is_not_assessable() {
        let mut observation = exact_observation(1);
        let ObservationCapture::SampledStable(report) = &mut observation else {
            unreachable!();
        };
        report.exact_paths = 0;
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &observation,
            &lab_catalog(1),
        );
        assert!(!report.invariants.observation_counts_consistent);
        assert_eq!(report.disposition, Disposition::NotAssessable);
    }

    #[test]
    fn path_identity_and_location_mismatches_are_not_assessable() {
        let snapshot = snapshot(1, 5);
        let mut wrong_index_mapping = mapping(1, 1);
        let MappingCapture::SampledStable(mapping_report) = &mut wrong_index_mapping else {
            unreachable!();
        };
        mapping_report.paths[0].path_index = 9;
        let report = build_read_only_qualification(
            Some(&snapshot),
            &wrong_index_mapping,
            &exact_observation(1),
            &lab_catalog(1),
        );
        assert!(!report.invariants.mapping_snapshot_paths_consistent);
        assert_eq!(report.disposition, Disposition::NotAssessable);

        let mut wrong_location_observation = exact_observation(1);
        let ObservationCapture::SampledStable(observation_report) =
            &mut wrong_location_observation
        else {
            unreachable!();
        };
        let PathObservation::Observed(observed) = &mut observation_report.paths[0] else {
            unreachable!();
        };
        observed.monitor_index = 99;
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &wrong_location_observation,
            &lab_catalog(1),
        );
        assert!(!report.invariants.observation_locations_consistent);
        assert_eq!(report.disposition, Disposition::NotAssessable);
    }

    #[test]
    fn candidate_summary_mismatch_is_not_assessable() {
        let mut catalog = lab_catalog(1);
        catalog.summary.records = 2;
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &catalog,
        );
        assert!(!report.invariants.candidate_counts_consistent);
        assert_eq!(report.disposition, Disposition::NotAssessable);
    }

    #[test]
    fn approved_surface_and_phase_closure_gaps_are_fixed() {
        let snapshot = snapshot(1, 5);
        let report = build_read_only_qualification(
            Some(&snapshot),
            &mapping(1, 1),
            &exact_observation(1),
            &lab_catalog(1),
        );
        assert!(report
            .evidence_gaps
            .contains(&ReadOnlyEvidenceGap::ApprovedCcdSurfaceNotImplemented));
        assert!(report
            .evidence_gaps
            .contains(&ReadOnlyEvidenceGap::SessionAndRdpEvidenceNotObserved));
        assert!(report
            .evidence_gaps
            .contains(&ReadOnlyEvidenceGap::VirtualDisplayExclusionNotProven));
        assert_eq!(report.phase_1a_closure, Phase1AClosure::NotClaimed);
    }
}
