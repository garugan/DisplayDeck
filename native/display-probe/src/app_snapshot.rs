use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDisplaySnapshot {
    pub schema_version: u16,
    pub platform: &'static str,
    pub capture_status: String,
    pub mutation_allowed: bool,
    pub blockers: Vec<String>,
    pub displays: Vec<AppDisplay>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDisplay {
    pub adapter_index: u32,
    pub device_name: String,
    pub friendly_name: String,
    pub attached_to_desktop: bool,
    pub primary: bool,
    pub current_mode: Option<AppMode>,
    pub current_membership: String,
    pub candidates: Vec<AppCandidate>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMode {
    pub width_pixels: Option<u32>,
    pub height_pixels: Option<u32>,
    pub refresh_hz: Option<u32>,
    pub orientation: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppCandidate {
    pub enumeration_index: u32,
    pub width_pixels: Option<u32>,
    pub height_pixels: Option<u32>,
    pub refresh_label: String,
    pub eligibility: String,
}

#[cfg(not(target_os = "windows"))]
pub fn capture() -> AppDisplaySnapshot {
    AppDisplaySnapshot {
        schema_version: 1,
        platform: "unsupported",
        capture_status: "UnsupportedPlatform".into(),
        mutation_allowed: false,
        blockers: vec!["WindowsOnly".into(), "ReadOnlyMvp".into()],
        displays: Vec::new(),
    }
}

#[cfg(target_os = "windows")]
pub fn capture() -> AppDisplaySnapshot {
    use crate::{candidate, ccd, display, qualification};

    let initial = ccd::query_active_display_config();
    let inventory = display::enumerate_display_adapters();
    let verification = ccd::query_active_display_config();
    let mapping_capture = mapping_capture(&initial, &verification, &inventory);
    let observation_capture =
        observation_capture(&initial, &verification, &mapping_capture, &inventory);
    let catalog = candidate::build_candidate_catalog(&inventory);
    let markers = qualification::collect_gdi_environment_markers(&inventory);
    let report = qualification::build_read_only_qualification_with_markers(
        initial.as_ref().ok(),
        &mapping_capture,
        &observation_capture,
        &catalog,
        markers,
    );

    let displays = inventory
        .adapters
        .iter()
        .map(|adapter| {
            let adapter_catalog = catalog
                .adapters
                .iter()
                .find(|candidate| candidate.adapter_index == adapter.index);
            AppDisplay {
                adapter_index: adapter.index,
                device_name: adapter.info.device_name.clone(),
                friendly_name: adapter
                    .monitors
                    .iter()
                    .find(|monitor| monitor.info.is_attached_to_desktop)
                    .or_else(|| adapter.monitors.first())
                    .map(|monitor| monitor.info.device_string.clone())
                    .unwrap_or_else(|| adapter.info.device_string.clone()),
                attached_to_desktop: adapter.info.is_attached_to_desktop,
                primary: adapter.info.is_primary,
                current_mode: stable_mode(&adapter.current_mode),
                current_membership: adapter_catalog
                    .map(|value| format!("{:?}", value.current_membership))
                    .unwrap_or_else(|| "Unavailable".into()),
                candidates: adapter_catalog
                    .map(|value| {
                        value
                            .candidates
                            .iter()
                            .map(|mode| AppCandidate {
                                enumeration_index: mode.provenance.enumeration_index,
                                width_pixels: mode.display_label.width_pixels,
                                height_pixels: mode.display_label.height_pixels,
                                refresh_label: format!("{:?}", mode.display_label.frequency),
                                eligibility: format!("{:?}", mode.eligibility),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect();

    let mut blockers = report
        .blockers
        .iter()
        .map(|blocker| format!("{blocker:?}"))
        .collect::<Vec<_>>();
    blockers.push("D07NoGo".into());
    blockers.push("ReadOnlyMvp".into());

    AppDisplaySnapshot {
        schema_version: 1,
        platform: "windows",
        capture_status: if report.invariants.all_satisfied() {
            "Captured"
        } else {
            "FailedClosed"
        }
        .into(),
        mutation_allowed: false,
        blockers,
        displays,
    }
}

#[cfg(target_os = "windows")]
fn mapping_capture(
    initial: &Result<crate::ccd::CcdSnapshot, crate::ccd::CcdQueryError>,
    verification: &Result<crate::ccd::CcdSnapshot, crate::ccd::CcdQueryError>,
    inventory: &crate::display::DisplayInventory,
) -> crate::qualification::MappingCapture {
    use crate::{ccd, display::DeviceEnumerationStatus, qualification};

    let snapshot = match initial {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return qualification::MappingCapture::Unavailable(
                qualification::MappingCaptureFailure::InitialCcdQueryFailed(error.failure_class()),
            )
        }
    };
    if inventory.adapter_enumeration_status != DeviceEnumerationStatus::Complete
        || inventory
            .adapters
            .iter()
            .any(|adapter| adapter.monitor_enumeration_status != DeviceEnumerationStatus::Complete)
    {
        return qualification::MappingCapture::Unavailable(
            qualification::MappingCaptureFailure::InventoryBoundExceeded,
        );
    }
    let verification = match verification {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return qualification::MappingCapture::Unavailable(
                qualification::MappingCaptureFailure::VerificationCcdQueryFailed(
                    error.failure_class(),
                ),
            )
        }
    };
    if !ccd::has_same_mapping_evidence(snapshot, verification) {
        return qualification::MappingCapture::Unavailable(
            qualification::MappingCaptureFailure::StaleSnapshot,
        );
    }
    qualification::MappingCapture::SampledStable(crate::mapping::cross_map(
        snapshot,
        &inventory.adapters,
    ))
}

#[cfg(target_os = "windows")]
fn observation_capture(
    initial: &Result<crate::ccd::CcdSnapshot, crate::ccd::CcdQueryError>,
    verification: &Result<crate::ccd::CcdSnapshot, crate::ccd::CcdQueryError>,
    mapping: &crate::qualification::MappingCapture,
    inventory: &crate::display::DisplayInventory,
) -> crate::qualification::ObservationCapture {
    use crate::{ccd, qualification};

    let snapshot = match initial {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return qualification::ObservationCapture::Unavailable(
                qualification::ObservationCaptureFailure::InitialCcdQueryFailed(
                    error.failure_class(),
                ),
            )
        }
    };
    let verification = match verification {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return qualification::ObservationCapture::Unavailable(
                qualification::ObservationCaptureFailure::VerificationCcdQueryFailed(
                    error.failure_class(),
                ),
            )
        }
    };
    let Some(mapping) = mapping.stable_report() else {
        return qualification::ObservationCapture::Unavailable(
            qualification::ObservationCaptureFailure::CrossMapUnavailable,
        );
    };
    if !ccd::has_same_current_observation_evidence(snapshot, verification) {
        return qualification::ObservationCapture::Unavailable(
            qualification::ObservationCaptureFailure::StaleSnapshot,
        );
    }
    qualification::ObservationCapture::SampledStable(
        crate::observation::build_current_observations(snapshot, mapping, &inventory.adapters),
    )
}

#[cfg(target_os = "windows")]
fn stable_mode(sample: &crate::display::CurrentModeSample) -> Option<AppMode> {
    let crate::display::CurrentModeSample::SampledStable(mode) = sample else {
        return None;
    };
    Some(AppMode {
        width_pixels: mode.width_pixels,
        height_pixels: mode.height_pixels,
        refresh_hz: mode.display_frequency_hz,
        orientation: mode.orientation,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn unsupported_platform_is_read_only() {
        let snapshot = super::capture();
        assert!(!snapshot.mutation_allowed);
        assert_eq!(snapshot.capture_status, "UnsupportedPlatform");
        assert!(snapshot.blockers.iter().any(|value| value == "WindowsOnly"));
    }
}
