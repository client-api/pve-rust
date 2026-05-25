// Runtime soft-skip gates. Macros so they can `return` from the caller —
// cargo's #[ignore] is binary and can't read env at compile time. Skips
// print to stderr (visible under --nocapture).

#[macro_export]
macro_rules! skip_if_no_kvm {
    () => {
        if std::env::var("PROXMOX_KVM_AVAILABLE").as_deref() != Ok("true") {
            eprintln!("SKIP: PROXMOX_KVM_AVAILABLE != true (KVM not exposed on this runner)");
            return;
        }
    };
}

#[macro_export]
macro_rules! skip_if_no_cgroupv2 {
    () => {
        if std::env::var("PROXMOX_CGROUPV2_AVAILABLE").as_deref() != Ok("true") {
            eprintln!(
                "SKIP: PROXMOX_CGROUPV2_AVAILABLE != true (cgroup v2 not exposed on this runner)"
            );
            return;
        }
    };
}

#[macro_export]
macro_rules! skip_if_pmg {
    ($creds:expr) => {
        if !$creds.token_auth_supported() {
            eprintln!("SKIP: token auth unsupported on this product (PMG sentinel)");
            return;
        }
    };
}

#[macro_export]
macro_rules! skip_if_no_network {
    () => {
        if std::env::var("PROXMOX_NO_NETWORK").as_deref() == Ok("1") {
            eprintln!("SKIP: PROXMOX_NO_NETWORK=1 (air-gapped runner)");
            return;
        }
    };
}
