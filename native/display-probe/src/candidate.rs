use std::{collections::BTreeMap, fmt};

use windows::Win32::Graphics::Gdi::{
    DM_BITSPERPEL, DM_DISPLAYFIXEDOUTPUT, DM_DISPLAYFLAGS, DM_DISPLAYFREQUENCY,
    DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
};

use crate::display::{
    devmode_public_size_bytes, CurrentModeSample, DeviceEnumerationStatus,
    DisplayAdapter, DisplayInventory, DisplayMode, DisplayPosition,
    ModeEnumerationStatus,
};

const REQUIRED_FIELD_MASK: u32 = DM_BITSPERPEL.0
    | DM_PELSWIDTH.0
    | DM_PELSHEIGHT.0
    | DM_DISPLAYFLAGS.0
    | DM_DISPLAYFREQUENCY.0;
const OPTIONAL_FIELD_MASK: u32 =
    DM_POSITION.0 | DM_DISPLAYORIENTATION.0 | DM_DISPLAYFIXEDOUTPUT.0;
const ALLOWLISTED_FIELD_MASK: u32 = REQUIRED_FIELD_MASK | OPTIONAL_FIELD_MASK;
// DEVMODEW documents legacy grayscale (0x1), interlaced (0x2), and text-mode
// (0x4) bits for dmDisplayFlags. Preserve these raw bits, but do not classify an
// unrecognized bit as part of a complete DisplayDeck tuple.
const KNOWN_DISPLAY_FLAGS_MASK: u32 = 0x0000_0007;
const KNOWN_BUT_UNSUPPORTED_DISPLAY_FLAGS_MASK: u32 = 0x0000_0005;

#[derive(Debug)]
pub struct CandidateCatalog {
    pub adapter_enumeration_status: DeviceEnumerationStatus,
    pub adapters: Vec<AdapterCandidateCatalog>,
    pub summary: CandidateSummary,
}

#[derive(Debug)]
pub struct AdapterCandidateCatalog {
    pub adapter_index: u32,
    pub device_name: String,
    pub monitor_enumeration_status: DeviceEnumerationStatus,
    pub enumeration_status: ModeEnumerationStatus,
    pub current_tuple_status: CurrentTupleStatus,
    pub current_membership: CurrentMembership,
    pub exact_duplicate_groups: Vec<CandidateRecordGroup>,
    pub projection_collision_groups: Vec<CandidateRecordGroup>,
    pub candidates: Vec<ModeCandidate>,
    pub summary: CandidateSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModeCandidate {
    pub candidate_identity: CandidateIdentity,
    pub provenance: EnumerationProvenance,
    pub display_label: DisplayLabel,
    pub public_size_bytes: u16,
    pub driver_extra_bytes: u16,
    pub apply_tuple: ApplyTuple,
    pub tuple_status: TupleStatus,
    pub tuple_issues: Vec<TupleIssue>,
    pub exact_duplicate: ExactDuplicateStatus,
    pub projection_collision: Option<RecordGroupReference>,
    pub current_relation: CurrentRelation,
    pub policy_relations: PolicyRelations,
    pub advanced_color_evidence: AdvancedColorEvidence,
    pub expected_observation: ExpectedObservationStatus,
    pub eligibility: CandidateEligibility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateIdentity {
    NotIssuedReadOnlyStep7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnumerationProvenance {
    pub adapter_index: u32,
    pub enumeration_index: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateRecordGroup {
    pub group_id: usize,
    pub mode_indices: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecordGroupReference {
    pub group_id: usize,
    pub record_count: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ApplyTuple {
    pub field_mask: u32,
    pub position: Option<DisplayPosition>,
    pub orientation: Option<u32>,
    pub fixed_output: Option<u32>,
    pub bits_per_pixel: Option<u32>,
    pub width_pixels: Option<u32>,
    pub height_pixels: Option<u32>,
    pub display_flags: Option<u32>,
    pub display_frequency_hz: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DisplayLabel {
    pub width_pixels: Option<u32>,
    pub height_pixels: Option<u32>,
    pub frequency: FrequencyLabel,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FrequencyLabel {
    Hertz(u32),
    DriverDefault0,
    DriverDefault1,
    NotReported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurrentTupleStatus {
    Complete,
    Incomplete { issues: Vec<TupleIssue> },
    Unavailable,
    ChangedDuringCapture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurrentMembership {
    ListedUnique { mode_index: u32 },
    AmbiguousExactRecords { mode_indices: Vec<u32> },
    NotListedExact { projection_only_indices: Vec<u32> },
    CurrentUnavailable,
    CurrentChangedDuringCapture,
    CurrentTupleIncomplete,
    EnumerationEmptyOrUnavailable,
    EnumerationIncomplete { limit: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TupleStatus {
    Complete,
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TupleIssue {
    UnexpectedPublicSize { expected: u16, observed: u16 },
    DriverPrivateData { bytes: u16 },
    MissingRequiredFields { mask: u32 },
    UnsupportedFields { mask: u32 },
    MissingCapturedValue { field_mask: u32 },
    ZeroBitsPerPixel,
    ZeroWidth,
    ZeroHeight,
    UnknownOrientation { raw: u32 },
    UnknownFixedOutput { raw: u32 },
    UnknownDisplayFlagBits { mask: u32 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactDuplicateStatus {
    Unique,
    ExactTupleDuplicate { group: RecordGroupReference },
    NotComparableIncomplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurrentRelation {
    Exact,
    Different,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldRelation {
    Exact,
    Different,
    NotReported,
    PresenceMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PolicyRelations {
    pub position: FieldRelation,
    pub orientation: FieldRelation,
    pub fixed_output: FieldRelation,
    pub bits_per_pixel: FieldRelation,
    pub display_flags: FieldRelation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdvancedColorEvidence {
    NotObserved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectedObservationStatus {
    MissingCurrentCandidateNotLinked,
    MissingNonCurrentRequiresQualification,
    MissingCurrentRelationUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateEligibility {
    LabUnqualified { gaps: Vec<QualificationGap> },
    HardExcluded { reasons: Vec<HardExclusion> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QualificationGap {
    ExpectedObservationMissing,
    AdvancedColorNotObserved,
    ExactTargetMappingNotLinked,
    SupportFingerprintMissing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HardExclusion {
    TupleIncomplete,
    AdapterNotAttachedToDesktop,
    AdapterEnumerationIncomplete,
    MonitorEnumerationIncomplete,
    EnumerationEmptyOrUnavailable,
    EnumerationIncomplete,
    CurrentUnavailable,
    CurrentChangedDuringCapture,
    CurrentTupleIncomplete,
    CurrentNotListed,
    CurrentExactRecordAmbiguous,
    ExactTupleDuplicate,
    DriverDefaultFrequency { raw: u32 },
    CurrentDriverDefaultFrequency { raw: u32 },
    BitsPerPixelBelow32 { raw: u32 },
    CurrentBitsPerPixelBelow32 { raw: u32 },
    KnownButUnsupportedDisplayFlags { mask: u32 },
    CurrentKnownButUnsupportedDisplayFlags { mask: u32 },
    PolicyDifferent { field: PolicyField },
    PolicyEvidenceUnavailable { field: PolicyField },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyField {
    Position,
    Orientation,
    FixedOutput,
    BitsPerPixel,
    DisplayFlags,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CandidateSummary {
    pub records: usize,
    pub complete_records: usize,
    pub incomplete_records: usize,
    pub exact_duplicate_groups: usize,
    pub exact_duplicate_records: usize,
    pub projection_collision_records: usize,
    pub lab_unqualified_records: usize,
    pub hard_excluded_records: usize,
    pub product_allowed_records: usize,
    pub selection_tokens_issued: usize,
}

#[derive(Clone)]
struct CandidateSeed {
    index: u32,
    public_size_bytes: u16,
    driver_extra_bytes: u16,
    apply_tuple: ApplyTuple,
    display_label: DisplayLabel,
    tuple_status: TupleStatus,
    tuple_issues: Vec<TupleIssue>,
}

struct CandidateGroups {
    exact_duplicate_groups: Vec<CandidateRecordGroup>,
    projection_collision_groups: Vec<CandidateRecordGroup>,
    exact_duplicate_refs: BTreeMap<u32, RecordGroupReference>,
    projection_collision_refs: BTreeMap<u32, RecordGroupReference>,
}

pub fn build_candidate_catalog(inventory: &DisplayInventory) -> CandidateCatalog {
    let monitor_inventory_complete = inventory.adapters.iter().all(|adapter| {
        adapter.monitor_enumeration_status
            == DeviceEnumerationStatus::Complete
    });
    let adapters = inventory
        .adapters
        .iter()
        .map(|adapter| {
            classify_adapter(
                adapter,
                inventory.adapter_enumeration_status,
                monitor_inventory_complete,
            )
        })
        .collect::<Vec<_>>();
    let summary = adapters.iter().fold(
        CandidateSummary::default(),
        |mut total, adapter| {
            total.add(adapter.summary);
            total
        },
    );

    CandidateCatalog {
        adapter_enumeration_status: inventory.adapter_enumeration_status,
        adapters,
        summary,
    }
}

fn classify_adapter(
    adapter: &DisplayAdapter,
    adapter_enumeration_status: DeviceEnumerationStatus,
    monitor_inventory_complete: bool,
) -> AdapterCandidateCatalog {
    let seeds = adapter
        .available_modes
        .iter()
        .map(|enumerated| CandidateSeed::new(enumerated.index, &enumerated.mode))
        .collect::<Vec<_>>();

    let (current_tuple_status, current_tuple, current_label) =
        assess_current(&adapter.current_mode);
    let current_membership = classify_current_membership(
        &adapter.current_mode,
        adapter.mode_enumeration_status,
        current_tuple.as_ref(),
        current_label,
        &seeds,
    );
    let groups = build_candidate_groups(&seeds);

    let candidates = seeds
        .iter()
        .map(|seed| {
            classify_candidate(
                adapter.index,
                adapter.info.is_attached_to_desktop,
                adapter_enumeration_status,
                monitor_inventory_complete,
                seed,
                &current_membership,
                current_tuple.as_ref(),
                groups.exact_duplicate_refs.get(&seed.index).copied(),
                groups
                    .projection_collision_refs
                    .get(&seed.index)
                    .copied(),
            )
        })
        .collect::<Vec<_>>();
    let summary = summarize(
        &candidates,
        &groups.exact_duplicate_groups,
        &groups.projection_collision_groups,
    );

    AdapterCandidateCatalog {
        adapter_index: adapter.index,
        device_name: adapter.info.device_name.clone(),
        monitor_enumeration_status: adapter.monitor_enumeration_status,
        enumeration_status: adapter.mode_enumeration_status,
        current_tuple_status,
        current_membership,
        exact_duplicate_groups: groups.exact_duplicate_groups,
        projection_collision_groups: groups.projection_collision_groups,
        candidates,
        summary,
    }
}

fn assess_current(
    current: &CurrentModeSample,
) -> (CurrentTupleStatus, Option<ApplyTuple>, Option<DisplayLabel>) {
    match current {
        CurrentModeSample::SampledStable(mode) => {
            let seed = CandidateSeed::new(0, mode);
            if seed.tuple_status == TupleStatus::Complete {
                (
                    CurrentTupleStatus::Complete,
                    Some(seed.apply_tuple),
                    Some(seed.display_label),
                )
            } else {
                (
                    CurrentTupleStatus::Incomplete {
                        issues: seed.tuple_issues,
                    },
                    None,
                    Some(seed.display_label),
                )
            }
        }
        CurrentModeSample::Unavailable => {
            (CurrentTupleStatus::Unavailable, None, None)
        }
        CurrentModeSample::Changed { .. } => (
            CurrentTupleStatus::ChangedDuringCapture,
            None,
            None,
        ),
    }
}

fn classify_current_membership(
    current: &CurrentModeSample,
    enumeration_status: ModeEnumerationStatus,
    current_tuple: Option<&ApplyTuple>,
    current_label: Option<DisplayLabel>,
    seeds: &[CandidateSeed],
) -> CurrentMembership {
    match current {
        CurrentModeSample::Unavailable => return CurrentMembership::CurrentUnavailable,
        CurrentModeSample::Changed { .. } => {
            return CurrentMembership::CurrentChangedDuringCapture;
        }
        CurrentModeSample::SampledStable(_) if current_tuple.is_none() => {
            return CurrentMembership::CurrentTupleIncomplete;
        }
        CurrentModeSample::SampledStable(_) => {}
    }

    match enumeration_status {
        ModeEnumerationStatus::Complete => {}
        ModeEnumerationStatus::EmptyOrUnavailable => {
            return CurrentMembership::EnumerationEmptyOrUnavailable;
        }
        ModeEnumerationStatus::LimitReached { limit } => {
            return CurrentMembership::EnumerationIncomplete { limit };
        }
    }

    let current_tuple = current_tuple.expect("stable complete current tuple must exist");
    let exact_indices = seeds
        .iter()
        .filter(|seed| {
            seed.tuple_status == TupleStatus::Complete
                && &seed.apply_tuple == current_tuple
        })
        .map(|seed| seed.index)
        .collect::<Vec<_>>();

    match exact_indices.as_slice() {
        [mode_index] => CurrentMembership::ListedUnique {
            mode_index: *mode_index,
        },
        [] => {
            let projection_only_indices = current_label
                .map(|label| {
                    seeds
                        .iter()
                        .filter(|seed| seed.display_label == label)
                        .map(|seed| seed.index)
                        .collect()
                })
                .unwrap_or_default();
            CurrentMembership::NotListedExact {
                projection_only_indices,
            }
        }
        _ => CurrentMembership::AmbiguousExactRecords {
            mode_indices: exact_indices,
        },
    }
}

fn build_candidate_groups(seeds: &[CandidateSeed]) -> CandidateGroups {
    let mut exact_by_tuple = BTreeMap::<ApplyTuple, Vec<u32>>::new();
    let mut projection_by_label = BTreeMap::<DisplayLabel, Vec<u32>>::new();
    let seed_by_index = seeds
        .iter()
        .map(|seed| (seed.index, seed))
        .collect::<BTreeMap<_, _>>();

    for seed in seeds {
        if seed.tuple_status == TupleStatus::Complete {
            exact_by_tuple
                .entry(seed.apply_tuple.clone())
                .or_default()
                .push(seed.index);
        }
        if seed.display_label.is_fully_reported() {
            projection_by_label
                .entry(seed.display_label)
                .or_default()
                .push(seed.index);
        }
    }

    let mut exact_duplicate_groups = Vec::new();
    let mut exact_duplicate_refs = BTreeMap::new();
    for mode_indices in exact_by_tuple
        .into_values()
        .filter(|indices| indices.len() > 1)
    {
        let group_id = exact_duplicate_groups.len();
        let reference = RecordGroupReference {
            group_id,
            record_count: mode_indices.len(),
        };
        for index in &mode_indices {
            exact_duplicate_refs.insert(*index, reference);
        }
        exact_duplicate_groups.push(CandidateRecordGroup {
            group_id,
            mode_indices,
        });
    }

    let mut projection_collision_groups = Vec::new();
    let mut projection_collision_refs = BTreeMap::new();
    for mode_indices in projection_by_label
        .into_values()
        .filter(|indices| indices.len() > 1)
    {
        let first = seed_by_index
            .get(&mode_indices[0])
            .expect("projection group index must refer to a seed");
        let only_one_complete_tuple = first.tuple_status == TupleStatus::Complete
            && mode_indices.iter().all(|index| {
                let seed = seed_by_index
                    .get(index)
                    .expect("projection group index must refer to a seed");
                seed.tuple_status == TupleStatus::Complete
                    && &seed.apply_tuple == &first.apply_tuple
            });
        if only_one_complete_tuple {
            continue;
        }

        let group_id = projection_collision_groups.len();
        let reference = RecordGroupReference {
            group_id,
            record_count: mode_indices.len(),
        };
        for index in &mode_indices {
            projection_collision_refs.insert(*index, reference);
        }
        projection_collision_groups.push(CandidateRecordGroup {
            group_id,
            mode_indices,
        });
    }

    CandidateGroups {
        exact_duplicate_groups,
        projection_collision_groups,
        exact_duplicate_refs,
        projection_collision_refs,
    }
}

fn classify_candidate(
    adapter_index: u32,
    adapter_attached_to_desktop: bool,
    adapter_enumeration_status: DeviceEnumerationStatus,
    monitor_inventory_complete: bool,
    seed: &CandidateSeed,
    current_membership: &CurrentMembership,
    current_tuple: Option<&ApplyTuple>,
    exact_duplicate_group: Option<RecordGroupReference>,
    projection_collision: Option<RecordGroupReference>,
) -> ModeCandidate {
    let exact_duplicate = if seed.tuple_status == TupleStatus::Incomplete {
        ExactDuplicateStatus::NotComparableIncomplete
    } else if let Some(group) = exact_duplicate_group {
        ExactDuplicateStatus::ExactTupleDuplicate { group }
    } else {
        ExactDuplicateStatus::Unique
    };

    let current_relation = match (seed.tuple_status, current_tuple) {
        (TupleStatus::Complete, Some(current)) if &seed.apply_tuple == current => {
            CurrentRelation::Exact
        }
        (TupleStatus::Complete, Some(_)) => CurrentRelation::Different,
        _ => CurrentRelation::Unavailable,
    };
    let policy_relations = compare_policy(current_tuple, &seed.apply_tuple);
    let expected_observation = match current_relation {
        CurrentRelation::Exact => {
            ExpectedObservationStatus::MissingCurrentCandidateNotLinked
        }
        CurrentRelation::Different => {
            ExpectedObservationStatus::MissingNonCurrentRequiresQualification
        }
        CurrentRelation::Unavailable => {
            ExpectedObservationStatus::MissingCurrentRelationUnavailable
        }
    };
    let eligibility = classify_eligibility(
        seed,
        adapter_attached_to_desktop,
        adapter_enumeration_status,
        monitor_inventory_complete,
        current_membership,
        current_tuple,
        &exact_duplicate,
        policy_relations,
    );

    ModeCandidate {
        candidate_identity: CandidateIdentity::NotIssuedReadOnlyStep7,
        provenance: EnumerationProvenance {
            adapter_index,
            enumeration_index: seed.index,
        },
        display_label: seed.display_label,
        public_size_bytes: seed.public_size_bytes,
        driver_extra_bytes: seed.driver_extra_bytes,
        apply_tuple: seed.apply_tuple.clone(),
        tuple_status: seed.tuple_status,
        tuple_issues: seed.tuple_issues.clone(),
        exact_duplicate,
        projection_collision,
        current_relation,
        policy_relations,
        advanced_color_evidence: AdvancedColorEvidence::NotObserved,
        expected_observation,
        eligibility,
    }
}

fn compare_policy(
    current: Option<&ApplyTuple>,
    candidate: &ApplyTuple,
) -> PolicyRelations {
    let Some(current) = current else {
        return PolicyRelations {
            position: FieldRelation::NotReported,
            orientation: FieldRelation::NotReported,
            fixed_output: FieldRelation::NotReported,
            bits_per_pixel: FieldRelation::NotReported,
            display_flags: FieldRelation::NotReported,
        };
    };

    PolicyRelations {
        position: compare_optional(current.position, candidate.position),
        orientation: compare_optional(current.orientation, candidate.orientation),
        fixed_output: compare_optional(current.fixed_output, candidate.fixed_output),
        bits_per_pixel: compare_optional(
            current.bits_per_pixel,
            candidate.bits_per_pixel,
        ),
        display_flags: compare_optional(current.display_flags, candidate.display_flags),
    }
}

fn compare_optional<T: Eq>(current: Option<T>, candidate: Option<T>) -> FieldRelation {
    match (current, candidate) {
        (Some(current), Some(candidate)) if current == candidate => FieldRelation::Exact,
        (Some(_), Some(_)) => FieldRelation::Different,
        (None, None) => FieldRelation::NotReported,
        _ => FieldRelation::PresenceMismatch,
    }
}

fn classify_eligibility(
    seed: &CandidateSeed,
    adapter_attached_to_desktop: bool,
    adapter_enumeration_status: DeviceEnumerationStatus,
    monitor_inventory_complete: bool,
    current_membership: &CurrentMembership,
    current_tuple: Option<&ApplyTuple>,
    exact_duplicate: &ExactDuplicateStatus,
    policy: PolicyRelations,
) -> CandidateEligibility {
    let mut reasons = Vec::new();

    if seed.tuple_status == TupleStatus::Incomplete {
        reasons.push(HardExclusion::TupleIncomplete);
    }
    if !adapter_attached_to_desktop {
        reasons.push(HardExclusion::AdapterNotAttachedToDesktop);
    }
    if matches!(
        adapter_enumeration_status,
        DeviceEnumerationStatus::LimitReached { .. }
    ) {
        reasons.push(HardExclusion::AdapterEnumerationIncomplete);
    }
    if !monitor_inventory_complete {
        reasons.push(HardExclusion::MonitorEnumerationIncomplete);
    }
    if matches!(
        exact_duplicate,
        ExactDuplicateStatus::ExactTupleDuplicate { .. }
    ) {
        reasons.push(HardExclusion::ExactTupleDuplicate);
    }

    match current_membership {
        CurrentMembership::ListedUnique { .. } => {}
        CurrentMembership::AmbiguousExactRecords { .. } => {
            reasons.push(HardExclusion::CurrentExactRecordAmbiguous);
        }
        CurrentMembership::NotListedExact { .. } => {
            reasons.push(HardExclusion::CurrentNotListed);
        }
        CurrentMembership::CurrentUnavailable => {
            reasons.push(HardExclusion::CurrentUnavailable);
        }
        CurrentMembership::CurrentChangedDuringCapture => {
            reasons.push(HardExclusion::CurrentChangedDuringCapture);
        }
        CurrentMembership::CurrentTupleIncomplete => {
            reasons.push(HardExclusion::CurrentTupleIncomplete);
        }
        CurrentMembership::EnumerationEmptyOrUnavailable => {
            reasons.push(HardExclusion::EnumerationEmptyOrUnavailable);
        }
        CurrentMembership::EnumerationIncomplete { .. } => {
            reasons.push(HardExclusion::EnumerationIncomplete);
        }
    }

    if let Some(raw) = driver_default_frequency(seed.apply_tuple.display_frequency_hz) {
        reasons.push(HardExclusion::DriverDefaultFrequency { raw });
    }
    if let Some(raw) = current_tuple
        .and_then(|current| driver_default_frequency(current.display_frequency_hz))
    {
        reasons.push(HardExclusion::CurrentDriverDefaultFrequency { raw });
    }
    if let Some(raw) = seed
        .apply_tuple
        .bits_per_pixel
        .filter(|bits| *bits < 32)
    {
        reasons.push(HardExclusion::BitsPerPixelBelow32 { raw });
    }
    if let Some(raw) = current_tuple
        .and_then(|current| current.bits_per_pixel)
        .filter(|bits| *bits < 32)
    {
        reasons.push(HardExclusion::CurrentBitsPerPixelBelow32 { raw });
    }
    if let Some(mask) = seed
        .apply_tuple
        .display_flags
        .map(|flags| flags & KNOWN_BUT_UNSUPPORTED_DISPLAY_FLAGS_MASK)
        .filter(|mask| *mask != 0)
    {
        reasons.push(HardExclusion::KnownButUnsupportedDisplayFlags { mask });
    }
    if let Some(mask) = current_tuple
        .and_then(|current| current.display_flags)
        .map(|flags| flags & KNOWN_BUT_UNSUPPORTED_DISPLAY_FLAGS_MASK)
        .filter(|mask| *mask != 0)
    {
        reasons.push(HardExclusion::CurrentKnownButUnsupportedDisplayFlags { mask });
    }

    add_policy_exclusion(&mut reasons, PolicyField::Position, policy.position);
    add_policy_exclusion(
        &mut reasons,
        PolicyField::Orientation,
        policy.orientation,
    );
    add_policy_exclusion(
        &mut reasons,
        PolicyField::FixedOutput,
        policy.fixed_output,
    );
    add_policy_exclusion(
        &mut reasons,
        PolicyField::BitsPerPixel,
        policy.bits_per_pixel,
    );
    add_policy_exclusion(
        &mut reasons,
        PolicyField::DisplayFlags,
        policy.display_flags,
    );

    if reasons.is_empty() {
        CandidateEligibility::LabUnqualified {
            gaps: vec![
                QualificationGap::ExpectedObservationMissing,
                QualificationGap::AdvancedColorNotObserved,
                QualificationGap::ExactTargetMappingNotLinked,
                QualificationGap::SupportFingerprintMissing,
            ],
        }
    } else {
        CandidateEligibility::HardExcluded { reasons }
    }
}

fn add_policy_exclusion(
    reasons: &mut Vec<HardExclusion>,
    field: PolicyField,
    relation: FieldRelation,
) {
    match relation {
        FieldRelation::Exact => {}
        FieldRelation::Different => {
            reasons.push(HardExclusion::PolicyDifferent { field });
        }
        FieldRelation::NotReported | FieldRelation::PresenceMismatch => {
            reasons.push(HardExclusion::PolicyEvidenceUnavailable { field });
        }
    }
}

fn driver_default_frequency(frequency: Option<u32>) -> Option<u32> {
    frequency.filter(|raw| *raw <= 1)
}

fn summarize(
    candidates: &[ModeCandidate],
    exact_duplicate_groups: &[CandidateRecordGroup],
    projection_collision_groups: &[CandidateRecordGroup],
) -> CandidateSummary {
    let complete_records = candidates
        .iter()
        .filter(|candidate| candidate.tuple_status == TupleStatus::Complete)
        .count();
    let exact_duplicate_records = candidates
        .iter()
        .filter(|candidate| {
            matches!(
                &candidate.exact_duplicate,
                ExactDuplicateStatus::ExactTupleDuplicate { .. }
            )
        })
        .count();
    let projection_collision_records = candidates
        .iter()
        .filter(|candidate| candidate.projection_collision.is_some())
        .count();
    let grouped_projection_records = projection_collision_groups
        .iter()
        .map(|group| group.mode_indices.len())
        .sum::<usize>();
    debug_assert_eq!(projection_collision_records, grouped_projection_records);
    let lab_unqualified_records = candidates
        .iter()
        .filter(|candidate| {
            matches!(
                &candidate.eligibility,
                CandidateEligibility::LabUnqualified { .. }
            )
        })
        .count();
    let hard_excluded_records = candidates.len() - lab_unqualified_records;

    CandidateSummary {
        records: candidates.len(),
        complete_records,
        incomplete_records: candidates.len() - complete_records,
        exact_duplicate_groups: exact_duplicate_groups.len(),
        exact_duplicate_records,
        projection_collision_records,
        lab_unqualified_records,
        hard_excluded_records,
        product_allowed_records: 0,
        selection_tokens_issued: 0,
    }
}

impl CandidateSeed {
    fn new(index: u32, mode: &DisplayMode) -> Self {
        let apply_tuple = ApplyTuple::from(mode);
        let tuple_issues = assess_tuple(mode, &apply_tuple);
        let tuple_status = if tuple_issues.is_empty() {
            TupleStatus::Complete
        } else {
            TupleStatus::Incomplete
        };

        Self {
            index,
            public_size_bytes: mode.public_size_bytes,
            driver_extra_bytes: mode.driver_extra_bytes,
            display_label: DisplayLabel::from(&apply_tuple),
            apply_tuple,
            tuple_status,
            tuple_issues,
        }
    }
}

impl From<&DisplayMode> for ApplyTuple {
    fn from(mode: &DisplayMode) -> Self {
        let field_is_present = |mask: u32| mode.field_mask & mask != 0;
        Self {
            field_mask: mode.field_mask,
            position: field_is_present(DM_POSITION.0)
                .then_some(mode.position)
                .flatten(),
            orientation: field_is_present(DM_DISPLAYORIENTATION.0)
                .then_some(mode.orientation)
                .flatten(),
            fixed_output: field_is_present(DM_DISPLAYFIXEDOUTPUT.0)
                .then_some(mode.fixed_output)
                .flatten(),
            bits_per_pixel: field_is_present(DM_BITSPERPEL.0)
                .then_some(mode.bits_per_pixel)
                .flatten(),
            width_pixels: field_is_present(DM_PELSWIDTH.0)
                .then_some(mode.width_pixels)
                .flatten(),
            height_pixels: field_is_present(DM_PELSHEIGHT.0)
                .then_some(mode.height_pixels)
                .flatten(),
            display_flags: field_is_present(DM_DISPLAYFLAGS.0)
                .then_some(mode.display_flags)
                .flatten(),
            display_frequency_hz: field_is_present(DM_DISPLAYFREQUENCY.0)
                .then_some(mode.display_frequency_hz)
                .flatten(),
        }
    }
}

impl From<&ApplyTuple> for DisplayLabel {
    fn from(tuple: &ApplyTuple) -> Self {
        let frequency = match tuple.display_frequency_hz {
            Some(0) => FrequencyLabel::DriverDefault0,
            Some(1) => FrequencyLabel::DriverDefault1,
            Some(hertz) => FrequencyLabel::Hertz(hertz),
            None => FrequencyLabel::NotReported,
        };

        Self {
            width_pixels: tuple.width_pixels,
            height_pixels: tuple.height_pixels,
            frequency,
        }
    }
}

impl DisplayLabel {
    fn is_fully_reported(self) -> bool {
        self.width_pixels.is_some()
            && self.height_pixels.is_some()
            && self.frequency != FrequencyLabel::NotReported
    }
}

fn assess_tuple(mode: &DisplayMode, tuple: &ApplyTuple) -> Vec<TupleIssue> {
    let mut issues = Vec::new();
    let expected_size = devmode_public_size_bytes();

    if mode.public_size_bytes != expected_size {
        issues.push(TupleIssue::UnexpectedPublicSize {
            expected: expected_size,
            observed: mode.public_size_bytes,
        });
    }
    if mode.driver_extra_bytes != 0 {
        issues.push(TupleIssue::DriverPrivateData {
            bytes: mode.driver_extra_bytes,
        });
    }

    let missing_required = REQUIRED_FIELD_MASK & !tuple.field_mask;
    if missing_required != 0 {
        issues.push(TupleIssue::MissingRequiredFields {
            mask: missing_required,
        });
    }
    let unsupported = tuple.field_mask & !ALLOWLISTED_FIELD_MASK;
    if unsupported != 0 {
        issues.push(TupleIssue::UnsupportedFields { mask: unsupported });
    }

    for (mask, value_is_some) in [
        (DM_POSITION.0, tuple.position.is_some()),
        (DM_DISPLAYORIENTATION.0, tuple.orientation.is_some()),
        (DM_DISPLAYFIXEDOUTPUT.0, tuple.fixed_output.is_some()),
        (DM_BITSPERPEL.0, tuple.bits_per_pixel.is_some()),
        (DM_PELSWIDTH.0, tuple.width_pixels.is_some()),
        (DM_PELSHEIGHT.0, tuple.height_pixels.is_some()),
        (DM_DISPLAYFLAGS.0, tuple.display_flags.is_some()),
        (
            DM_DISPLAYFREQUENCY.0,
            tuple.display_frequency_hz.is_some(),
        ),
    ] {
        if tuple.field_mask & mask != 0 && !value_is_some {
            issues.push(TupleIssue::MissingCapturedValue { field_mask: mask });
        }
    }

    if tuple.bits_per_pixel == Some(0) {
        issues.push(TupleIssue::ZeroBitsPerPixel);
    }
    if tuple.width_pixels == Some(0) {
        issues.push(TupleIssue::ZeroWidth);
    }
    if tuple.height_pixels == Some(0) {
        issues.push(TupleIssue::ZeroHeight);
    }
    if let Some(raw) = tuple.orientation.filter(|raw| *raw > 3) {
        issues.push(TupleIssue::UnknownOrientation { raw });
    }
    if let Some(raw) = tuple.fixed_output.filter(|raw| *raw > 2) {
        issues.push(TupleIssue::UnknownFixedOutput { raw });
    }
    if let Some(mask) = tuple
        .display_flags
        .map(|flags| flags & !KNOWN_DISPLAY_FLAGS_MASK)
        .filter(|mask| *mask != 0)
    {
        issues.push(TupleIssue::UnknownDisplayFlagBits { mask });
    }

    issues
}

impl CandidateSummary {
    fn add(&mut self, other: Self) {
        self.records += other.records;
        self.complete_records += other.complete_records;
        self.incomplete_records += other.incomplete_records;
        self.exact_duplicate_groups += other.exact_duplicate_groups;
        self.exact_duplicate_records += other.exact_duplicate_records;
        self.projection_collision_records += other.projection_collision_records;
        self.lab_unqualified_records += other.lab_unqualified_records;
        self.hard_excluded_records += other.hard_excluded_records;
        self.product_allowed_records += other.product_allowed_records;
        self.selection_tokens_issued += other.selection_tokens_issued;
    }
}

impl fmt::Display for DisplayLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.width_pixels, self.height_pixels) {
            (Some(width), Some(height)) => write!(formatter, "{width}x{height}")?,
            _ => write!(formatter, "resolution unavailable")?,
        }
        write!(formatter, " @ {}", self.frequency)
    }
}

impl fmt::Display for CandidateIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotIssuedReadOnlyStep7 => write!(formatter, "NotIssued (read-only Step 7)"),
        }
    }
}

impl fmt::Display for FrequencyLabel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hertz(hertz) => write!(formatter, "{hertz} Hz (raw integer)"),
            Self::DriverDefault0 => write!(formatter, "driver default (raw 0)"),
            Self::DriverDefault1 => write!(formatter, "driver default (raw 1)"),
            Self::NotReported => write!(formatter, "not reported"),
        }
    }
}

impl fmt::Display for CurrentTupleStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Complete => write!(formatter, "Complete"),
            Self::Incomplete { issues } => {
                write!(formatter, "Incomplete ({})", format_debug_list(issues))
            }
            Self::Unavailable => write!(formatter, "Unavailable"),
            Self::ChangedDuringCapture => write!(formatter, "ChangedDuringCapture"),
        }
    }
}

impl fmt::Display for CurrentMembership {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ListedUnique { mode_index } => {
                write!(formatter, "ListedUnique (Mode {mode_index})")
            }
            Self::AmbiguousExactRecords { mode_indices } => write!(
                formatter,
                "AmbiguousExactRecords (Modes {})",
                format_indices(mode_indices)
            ),
            Self::NotListedExact {
                projection_only_indices,
            } if projection_only_indices.is_empty() => {
                write!(formatter, "NotListedExact (no projection-only match)")
            }
            Self::NotListedExact {
                projection_only_indices,
            } => write!(
                formatter,
                "NotListedExact (projection-only Modes {})",
                format_indices(projection_only_indices)
            ),
            Self::CurrentUnavailable => write!(formatter, "CurrentUnavailable"),
            Self::CurrentChangedDuringCapture => {
                write!(formatter, "CurrentChangedDuringCapture")
            }
            Self::CurrentTupleIncomplete => write!(formatter, "CurrentTupleIncomplete"),
            Self::EnumerationEmptyOrUnavailable => {
                write!(formatter, "EnumerationEmptyOrUnavailable")
            }
            Self::EnumerationIncomplete { limit } => {
                write!(formatter, "EnumerationIncomplete (limit {limit})")
            }
        }
    }
}

impl fmt::Display for TupleStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl fmt::Display for ExactDuplicateStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unique => write!(formatter, "Unique"),
            Self::ExactTupleDuplicate { group } => write!(
                formatter,
                "ExactTupleDuplicate (Group {} / {} records)",
                group.group_id,
                group.record_count
            ),
            Self::NotComparableIncomplete => write!(formatter, "NotComparableIncomplete"),
        }
    }
}

impl fmt::Display for CurrentRelation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl fmt::Display for FieldRelation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl fmt::Display for AdvancedColorEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "NotObserved")
    }
}

impl fmt::Display for ExpectedObservationStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCurrentCandidateNotLinked => {
                write!(formatter, "Missing (current candidate not linked in Step 7)")
            }
            Self::MissingNonCurrentRequiresQualification => {
                write!(formatter, "Missing (non-current candidate requires qualification)")
            }
            Self::MissingCurrentRelationUnavailable => {
                write!(formatter, "Missing (current relation unavailable)")
            }
        }
    }
}

impl fmt::Display for CandidateEligibility {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LabUnqualified { gaps } => {
                write!(formatter, "LabUnqualified ({})", format_debug_list(gaps))
            }
            Self::HardExcluded { reasons } => {
                write!(formatter, "HardExcluded ({})", format_debug_list(reasons))
            }
        }
    }
}

fn format_indices(indices: &[u32]) -> String {
    indices
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_debug_list<T: fmt::Debug>(values: &[T]) -> String {
    values
        .iter()
        .map(|value| format!("{value:?}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::{
        DisplayDeviceInfo, DisplayInventory, EnumeratedDisplayMode,
    };

    fn complete_mode(width: u32, height: u32, frequency: u32) -> DisplayMode {
        DisplayMode {
            public_size_bytes: devmode_public_size_bytes(),
            driver_extra_bytes: 0,
            field_mask: ALLOWLISTED_FIELD_MASK,
            position: Some(DisplayPosition { x: 0, y: 0 }),
            orientation: Some(0),
            fixed_output: Some(0),
            bits_per_pixel: Some(32),
            width_pixels: Some(width),
            height_pixels: Some(height),
            display_flags: Some(0),
            display_frequency_hz: Some(frequency),
        }
    }

    fn adapter(
        current: CurrentModeSample,
        modes: Vec<DisplayMode>,
    ) -> DisplayAdapter {
        DisplayAdapter {
            index: 7,
            info: DisplayDeviceInfo {
                device_name: r"\\.\DISPLAY7".to_owned(),
                device_string: String::new(),
                device_id: String::new(),
                device_key: String::new(),
                is_primary: false,
                is_attached_to_desktop: true,
            },
            device_name_key: None,
            current_mode: current,
            available_modes: modes
                .into_iter()
                .enumerate()
                .map(|(index, mode)| EnumeratedDisplayMode {
                    index: u32::try_from(index).unwrap(),
                    mode,
                })
                .collect(),
            mode_enumeration_status: ModeEnumerationStatus::Complete,
            monitors: Vec::new(),
            monitor_enumeration_status: DeviceEnumerationStatus::Complete,
        }
    }

    fn catalog_for(adapter: DisplayAdapter) -> AdapterCandidateCatalog {
        let inventory = DisplayInventory {
            adapters: vec![adapter],
            adapter_enumeration_status: DeviceEnumerationStatus::Complete,
        };
        let mut catalog = build_candidate_catalog(&inventory);
        catalog.adapters.remove(0)
    }

    #[test]
    fn complete_allowlisted_tuple_is_structurally_complete() {
        let mode = complete_mode(1920, 1080, 60);
        let seed = CandidateSeed::new(0, &mode);

        assert_eq!(seed.tuple_status, TupleStatus::Complete);
        assert!(seed.tuple_issues.is_empty());
        assert_eq!(seed.apply_tuple.position, Some(DisplayPosition { x: 0, y: 0 }));
    }

    #[test]
    fn missing_required_and_unknown_fields_are_incomplete() {
        let mut mode = complete_mode(1920, 1080, 60);
        mode.field_mask &= !DM_BITSPERPEL.0;
        mode.bits_per_pixel = None;
        mode.field_mask |= 0x0000_0001;
        let seed = CandidateSeed::new(0, &mode);

        assert_eq!(seed.tuple_status, TupleStatus::Incomplete);
        assert!(seed.tuple_issues.contains(&TupleIssue::MissingRequiredFields {
            mask: DM_BITSPERPEL.0,
        }));
        assert!(seed
            .tuple_issues
            .contains(&TupleIssue::UnsupportedFields { mask: 1 }));
    }

    #[test]
    fn optional_absence_is_not_default_value() {
        let current = complete_mode(1920, 1080, 60);
        let mut candidate = current.clone();
        candidate.field_mask &= !DM_DISPLAYORIENTATION.0;
        // A value in an inactive union field is not evidence and must be dropped.
        candidate.orientation = Some(3);

        let current_tuple = ApplyTuple::from(&current);
        let candidate_tuple = ApplyTuple::from(&candidate);
        assert_ne!(current_tuple, candidate_tuple);
        assert_eq!(
            compare_policy(Some(&current_tuple), &candidate_tuple).orientation,
            FieldRelation::PresenceMismatch
        );
    }

    #[test]
    fn negative_position_is_preserved() {
        let mut mode = complete_mode(1080, 1920, 60);
        mode.position = Some(DisplayPosition { x: -1080, y: -280 });

        assert_eq!(
            ApplyTuple::from(&mode).position,
            Some(DisplayPosition { x: -1080, y: -280 })
        );
    }

    #[test]
    fn invalid_orientation_and_fixed_output_are_incomplete() {
        let mut mode = complete_mode(1920, 1080, 60);
        mode.orientation = Some(4);
        mode.fixed_output = Some(3);
        let seed = CandidateSeed::new(0, &mode);

        assert_eq!(seed.tuple_status, TupleStatus::Incomplete);
        assert!(seed
            .tuple_issues
            .contains(&TupleIssue::UnknownOrientation { raw: 4 }));
        assert!(seed
            .tuple_issues
            .contains(&TupleIssue::UnknownFixedOutput { raw: 3 }));
    }

    #[test]
    fn raw_default_frequencies_remain_distinct_and_hard_excluded() {
        let current = complete_mode(1920, 1080, 60);
        let zero = complete_mode(1280, 720, 0);
        let one = complete_mode(1280, 720, 1);
        assert_ne!(ApplyTuple::from(&zero), ApplyTuple::from(&one));
        assert_eq!(CandidateSeed::new(0, &zero).tuple_status, TupleStatus::Complete);
        assert_eq!(CandidateSeed::new(1, &one).tuple_status, TupleStatus::Complete);

        let report = catalog_for(adapter(
            CurrentModeSample::SampledStable(current.clone()),
            vec![current, zero, one],
        ));
        for candidate in &report.candidates[1..] {
            assert!(matches!(
                &candidate.eligibility,
                CandidateEligibility::HardExcluded { .. }
            ));
        }
    }

    #[test]
    fn exact_duplicates_are_retained_and_make_membership_ambiguous() {
        let current = complete_mode(1920, 1080, 60);
        let report = catalog_for(adapter(
            CurrentModeSample::SampledStable(current.clone()),
            vec![current.clone(), current],
        ));

        assert_eq!(
            report.current_membership,
            CurrentMembership::AmbiguousExactRecords {
                mode_indices: vec![0, 1],
            }
        );
        assert!(report.candidates.iter().all(|candidate| matches!(
            &candidate.exact_duplicate,
            ExactDuplicateStatus::ExactTupleDuplicate { .. }
        )));
    }

    #[test]
    fn same_label_with_different_bpp_is_projection_collision() {
        let current = complete_mode(1920, 1080, 60);
        let mut other = current.clone();
        other.bits_per_pixel = Some(24);
        let report = catalog_for(adapter(
            CurrentModeSample::SampledStable(current.clone()),
            vec![current, other],
        ));

        assert_eq!(report.projection_collision_groups.len(), 1);
        assert_eq!(
            report.projection_collision_groups[0].mode_indices,
            vec![0, 1]
        );
        assert_eq!(
            report.candidates[0].projection_collision,
            Some(RecordGroupReference {
                group_id: 0,
                record_count: 2,
            })
        );
        assert_eq!(
            report.candidates[1].projection_collision,
            Some(RecordGroupReference {
                group_id: 0,
                record_count: 2,
            })
        );
        assert_eq!(
            report.candidates[1].policy_relations.bits_per_pixel,
            FieldRelation::Different
        );
        let CandidateEligibility::HardExcluded { reasons } =
            &report.candidates[1].eligibility
        else {
            panic!("24 bpp candidate must remain hard excluded");
        };
        assert!(reasons.contains(&HardExclusion::BitsPerPixelBelow32 { raw: 24 }));
    }

    #[test]
    fn current_membership_uses_full_tuple_not_display_label() {
        let current = complete_mode(3440, 1440, 144);
        let mut listed = current.clone();
        listed.display_flags = Some(2);
        let report = catalog_for(adapter(
            CurrentModeSample::SampledStable(current),
            vec![listed],
        ));

        assert_eq!(
            report.current_membership,
            CurrentMembership::NotListedExact {
                projection_only_indices: vec![0],
            }
        );
    }

    #[test]
    fn incomplete_enumeration_does_not_claim_current_not_listed() {
        let current = complete_mode(1920, 1080, 60);
        let mut value = adapter(
            CurrentModeSample::SampledStable(current),
            Vec::new(),
        );
        value.mode_enumeration_status =
            ModeEnumerationStatus::LimitReached { limit: 4096 };
        let report = catalog_for(value);

        assert_eq!(
            report.current_membership,
            CurrentMembership::EnumerationIncomplete { limit: 4096 }
        );
    }

    #[test]
    fn empty_enumeration_does_not_claim_current_not_listed() {
        let current = complete_mode(1920, 1080, 60);
        let mut value = adapter(
            CurrentModeSample::SampledStable(current),
            Vec::new(),
        );
        value.mode_enumeration_status = ModeEnumerationStatus::EmptyOrUnavailable;
        let report = catalog_for(value);

        assert_eq!(
            report.current_membership,
            CurrentMembership::EnumerationEmptyOrUnavailable
        );
    }

    #[test]
    fn unknown_display_flag_bits_make_tuple_incomplete() {
        let mut mode = complete_mode(1920, 1080, 60);
        mode.display_flags = Some(0x8000_0000);
        let seed = CandidateSeed::new(0, &mode);

        assert_eq!(seed.tuple_status, TupleStatus::Incomplete);
        assert!(seed
            .tuple_issues
            .contains(&TupleIssue::UnknownDisplayFlagBits {
                mask: 0x8000_0000,
            }));

        let mut legacy = complete_mode(1920, 1080, 60);
        legacy.display_flags = Some(0x1);
        let report = catalog_for(adapter(
            CurrentModeSample::SampledStable(legacy.clone()),
            vec![legacy],
        ));
        let CandidateEligibility::HardExcluded { reasons } =
            &report.candidates[0].eligibility
        else {
            panic!("known legacy display flag must remain hard excluded");
        };
        assert!(reasons.contains(
            &HardExclusion::KnownButUnsupportedDisplayFlags { mask: 0x1 }
        ));
    }

    #[test]
    fn current_change_during_capture_fails_closed() {
        let before = complete_mode(1920, 1080, 60);
        let after = complete_mode(1920, 1080, 144);
        let report = catalog_for(adapter(
            CurrentModeSample::Changed {
                before: Some(before.clone()),
                after: Some(after),
            },
            vec![before],
        ));

        assert_eq!(
            report.current_membership,
            CurrentMembership::CurrentChangedDuringCapture
        );
        assert!(matches!(
            &report.candidates[0].eligibility,
            CandidateEligibility::HardExcluded { .. }
        ));
    }

    #[test]
    fn complete_unique_policy_exact_candidate_is_only_lab_unqualified() {
        let current = complete_mode(1920, 1080, 60);
        let candidate = complete_mode(2560, 1440, 60);
        let report = catalog_for(adapter(
            CurrentModeSample::SampledStable(current.clone()),
            vec![current, candidate],
        ));

        assert!(matches!(
            &report.candidates[1].eligibility,
            CandidateEligibility::LabUnqualified { .. }
        ));
        assert_eq!(
            report.candidates[1].candidate_identity,
            CandidateIdentity::NotIssuedReadOnlyStep7
        );
        let CandidateEligibility::LabUnqualified { gaps } =
            &report.candidates[1].eligibility
        else {
            unreachable!();
        };
        for expected in [
            QualificationGap::ExpectedObservationMissing,
            QualificationGap::AdvancedColorNotObserved,
            QualificationGap::ExactTargetMappingNotLinked,
            QualificationGap::SupportFingerprintMissing,
        ] {
            assert!(gaps.contains(&expected));
        }
        assert_eq!(report.summary.product_allowed_records, 0);
        assert_eq!(report.summary.selection_tokens_issued, 0);

        let current = complete_mode(1920, 1080, 60);
        let mut disconnected = adapter(
            CurrentModeSample::SampledStable(current.clone()),
            vec![current],
        );
        disconnected.info.is_attached_to_desktop = false;
        disconnected.monitor_enumeration_status =
            DeviceEnumerationStatus::LimitReached { limit: 32 };
        let inventory = DisplayInventory {
            adapters: vec![disconnected],
            adapter_enumeration_status:
                DeviceEnumerationStatus::LimitReached { limit: 32 },
        };
        let catalog = build_candidate_catalog(&inventory);
        let CandidateEligibility::HardExcluded { reasons } =
            &catalog.adapters[0].candidates[0].eligibility
        else {
            panic!("incomplete disconnected inventory must remain hard excluded");
        };
        for expected in [
            HardExclusion::AdapterNotAttachedToDesktop,
            HardExclusion::AdapterEnumerationIncomplete,
            HardExclusion::MonitorEnumerationIncomplete,
        ] {
            assert!(reasons.contains(&expected));
        }
    }

    #[test]
    fn driver_private_bytes_make_tuple_incomplete() {
        let mut mode = complete_mode(1920, 1080, 60);
        mode.driver_extra_bytes = 16;
        let seed = CandidateSeed::new(0, &mode);

        assert_eq!(seed.tuple_status, TupleStatus::Incomplete);
        assert!(seed
            .tuple_issues
            .contains(&TupleIssue::DriverPrivateData { bytes: 16 }));
    }
}
