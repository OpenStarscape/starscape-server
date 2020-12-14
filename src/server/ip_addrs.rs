use super::*;

fn format_interface(interface: &get_if_addrs::Interface) -> String {
    format!("{} ({:?})", interface.name, interface.ip())
}

/// Used as an argument to get_ip().
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum IpVersion {
    V4,
    V6,
}

/// Checks a single interface.
fn check_interface_against(
    interface: &get_if_addrs::Interface,
    interface_name: Option<&str>,
    version: Option<IpVersion>,
    loopback: Option<bool>,
) -> Result<(), String> {
    if let Some(interface_name) = interface_name {
        if interface_name != interface.name {
            return Err(format!(
                "{}: interface name is not {}",
                format_interface(interface),
                interface_name
            ));
        }
    }
    if match version {
        Some(IpVersion::V4) => !matches!(interface.addr, get_if_addrs::IfAddr::V4(_)),
        Some(IpVersion::V6) => !matches!(interface.addr, get_if_addrs::IfAddr::V6(_)),
        None => false,
    } {
        return Err(format!(
            "{}: IP version is not {:?}",
            format_interface(interface),
            version.unwrap()
        ));
    }
    if let Some(loopback) = loopback {
        if loopback != interface.is_loopback() {
            return Err(format!(
                "{}: {} a loopback address",
                format_interface(interface),
                if interface.is_loopback() {
                    "is"
                } else {
                    "is not"
                }
            ));
        }
    }
    Ok(())
}

/// Returns an IP address that matches the given criteria (None means to ignore)
pub fn get_ip(
    interface_name: Option<&str>,
    version: Option<IpVersion>,
    loopback: Option<bool>,
) -> Result<IpAddr, Box<dyn Error>> {
    let interfaces = get_if_addrs::get_if_addrs()?;
    let mut candidates: Vec<&get_if_addrs::Interface> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for interface in &interfaces {
        match check_interface_against(interface, interface_name, version, loopback) {
            Ok(()) => candidates.push(interface),
            Err(e) => errors.push(e),
        }
    }
    if candidates.is_empty() {
        Err(format!(
            "There are no matching network interfaces: {:?}",
            errors.join(", "),
        )
        .into())
    } else {
        if candidates.len() > 1 {
            warn!(
                "There are multiple matching network interfaces: {}, choosing {}",
                candidates
                    .iter()
                    .map(|interface| format_interface(interface))
                    .collect::<Vec<String>>()
                    .join(", "),
                format_interface(&candidates[0]),
            );
        }
        Ok(candidates[0].ip())
    }
}
