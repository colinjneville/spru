pub const SERVER_TO_CLIENT_LANES: [aeronet::transport::lane::LaneKind; 2] = [
    // Connection coordination
    aeronet::transport::lane::LaneKind::ReliableOrdered,
    // spru signals
    aeronet::transport::lane::LaneKind::ReliableOrdered,
];

pub const CLIENT_TO_SERVER_LANES: [aeronet::transport::lane::LaneKind; 2] = [
    // Connection coordination
    aeronet::transport::lane::LaneKind::ReliableOrdered,
    // spru signals
    aeronet::transport::lane::LaneKind::ReliableOrdered,
];

pub const SERVER_TO_CLIENT_LANE_COORDINATION: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(0);
pub const SERVER_TO_CLIENT_LANE_SIGNAL: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(1);

pub const CLIENT_TO_SERVER_LANE_COORDINATION: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(0);
pub const CLIENT_TO_SERVER_LANE_SIGNAL: aeronet::transport::lane::LaneIndex = aeronet::transport::lane::LaneIndex::new(1);