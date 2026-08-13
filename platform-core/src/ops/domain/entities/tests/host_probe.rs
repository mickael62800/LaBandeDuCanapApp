//! Tests des noms de sondes host (unicite + non vide).

use super::*;

#[test]
fn feature_names_unique_and_nonempty() {
    let all = [
        HostProbe::SshFailures,
        HostProbe::DiskTrend,
        HostProbe::Connections,
        HostProbe::OpenPorts,
        HostProbe::Trivy,
        HostProbe::TlsErrors,
        HostProbe::FileIntegrity,
        HostProbe::Outbound,
        HostProbe::NginxSuspicious,
    ];
    let mut names: Vec<&str> = all.iter().map(|p| p.feature()).collect();
    for n in &names {
        assert!(!n.is_empty());
    }
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), all.len());
}

#[test]
fn feature_sample_values() {
    assert_eq!(HostProbe::SshFailures.feature(), "ssh-failures");
    assert_eq!(HostProbe::NginxSuspicious.feature(), "nginx-suspicious");
}
