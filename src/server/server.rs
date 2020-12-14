use super::*;

/// Represents an object that lives for the lifetime of the server, such as a listener for a
/// particular network protocol
pub trait ServerComponent: Debug {}

/// Creates and owns the various components that allow clients to connect
pub struct Server {
    _components: Vec<Box<dyn ServerComponent>>,
}

fn format_interface(interface: &get_if_addrs::Interface) -> String {
    format!("{} ({:?})", interface.name, interface.ip())
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum IpVersion {
    V4,
    V6,
}

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

/// Returns an IP that is either a loopback (local) address or not.
fn get_ip(
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

impl Server {
    pub fn new(
        enable_tcp: bool,
        enable_webrtc: bool,
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut components: Vec<Box<dyn ServerComponent>> = Vec::new();

        // Is there a simpler way to make an empty warp filter?
        let mut warp_filter = warp::any()
            .and_then(|| async { Err::<Box<dyn warp::Reply>, _>(warp::reject::not_found()) })
            .boxed();

        if enable_tcp {
            let tcp = TcpListener::new(new_session_tx.clone(), None, None)
                .map_err(|e| format!("failed to create TcpListener: {}", e))?;
            components.push(Box::new(tcp));
        }

        if enable_webrtc {
            let ip = get_ip(None, Some(IpVersion::V4), Some(false))?;
            let listen_addr = SocketAddr::new(ip, 42424);
            let (rtc_warp_filter, webrtc) = WebrtcServer::new(listen_addr, new_session_tx)
                .map_err(|e| format!("failed to create WebrtcServer: {}", e))?;
            components.push(Box::new(webrtc));
            warp_filter = warp_filter.or(rtc_warp_filter).unify().boxed();
        }

        let http_server = HttpServer::new(warp_filter, None, None)?;
        components.push(Box::new(http_server));

        for component in &components {
            info!("{:?}", component);
        }

        Ok(Self {
            _components: components,
        })
    }
}
