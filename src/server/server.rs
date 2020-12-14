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

fn format_interface_list(interfaces: &[get_if_addrs::Interface]) -> String {
    interfaces
        .iter()
        .map(|interface| format_interface(interface))
        .collect::<Vec<String>>()
        .join(", ")
}

/// Returns the IP address for the given interface name.
/// Run `$ ip addr` to see available interfaces from the command line
#[allow(dead_code)]
fn get_ip_with_interface_name(interface_name: &str) -> Result<IpAddr, Box<dyn Error>> {
    let interfaces = get_if_addrs::get_if_addrs()?;
    for interface in &interfaces {
        if interface.name == interface_name {
            return Ok(interface.ip());
        }
    }
    Err(format!(
        "No network interface named {:?}, available interfaces: {}",
        interface_name,
        format_interface_list(&interfaces),
    )
    .into())
}

/// Returns an IP that is either a loopback (local) address or not.
fn get_ip_with_loopback(loopback: bool) -> Result<IpAddr, Box<dyn Error>> {
    let interfaces = get_if_addrs::get_if_addrs()?;
    let candidates: Vec<get_if_addrs::Interface> = interfaces
        .iter()
        .filter_map(|interface| {
            if interface.is_loopback() == loopback {
                Some(interface.clone())
            } else {
                None
            }
        })
        .collect();
    if candidates.is_empty() {
        Err(format!(
            "There are no network interfaces with loopback={}, available interfaces: {}",
            loopback,
            format_interface_list(&interfaces),
        )
        .into())
    } else {
        if candidates.len() > 1 {
            warn!(
                "There are multiple network interfaces with loopback={}: {}, choosing {}",
                loopback,
                format_interface_list(&candidates),
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
            let ip = get_ip_with_loopback(false)?;
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
