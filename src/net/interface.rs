pub enum NetInterfaceStatus {
    Up,
    Down,
}

pub struct NetInterface {
    state: NetInterfaceStatus,
    ip: IpAddress,
}
