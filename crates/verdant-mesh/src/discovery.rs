use serde::{Deserialize, Serialize};
use verdant_core::error::RadioError;
use verdant_core::traits::RadioHardware;
use verdant_core::types::{NodeId, SemVer, ZoneId};

/// BLE discovery beacon broadcast by each node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Beacon {
    pub node_id: NodeId,
    pub zone_id: ZoneId,
    pub firmware_version: SemVer,
    pub uptime_secs: u32,
}

/// A beacon received from a neighbor during scanning.
#[derive(Clone, Debug, PartialEq)]
pub struct NeighborBeacon {
    pub node_id: NodeId,
    pub zone_id: ZoneId,
    pub rssi: i8,
}

/// Handles BLE-based neighbor discovery.
pub struct DiscoveryService<R: RadioHardware> {
    radio: R,
    self_id: NodeId,
}

impl<R: RadioHardware> DiscoveryService<R> {
    pub fn new(radio: R, self_id: NodeId) -> Self {
        Self { radio, self_id }
    }

    /// Broadcast our beacon so neighbors can discover us.
    pub fn broadcast_beacon(&mut self, beacon: &Beacon) -> Result<(), RadioError> {
        let mut buf = [0u8; 64];
        let bytes = postcard::to_slice(beacon, &mut buf).map_err(|_| RadioError::TransmitFailed)?;
        self.radio.transmit_frame(bytes)
    }

    /// Scan for neighbor BLE beacons.
    pub fn scan_neighbors(&mut self) -> Result<heapless::Vec<NeighborBeacon, 32>, RadioError> {
        let beacons = self.radio.scan_ble_beacons()?;
        let mut result = heapless::Vec::new();
        for b in &beacons {
            if b.node_id != self.self_id {
                let _ = result.push(NeighborBeacon {
                    node_id: b.node_id,
                    zone_id: b.zone_id,
                    rssi: b.rssi,
                });
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use verdant_core::types::BleBeacon;

    mock! {
        pub Radio {}
        impl RadioHardware for Radio {
            fn transmit_frame(&mut self, raw: &[u8]) -> Result<(), RadioError>;
            fn receive_frame(&mut self, buf: &mut [u8]) -> Result<usize, RadioError>;
            fn scan_ble_beacons(&mut self) -> Result<heapless::Vec<BleBeacon, 32>, RadioError>;
        }
    }

    fn test_node(id: u8) -> NodeId {
        NodeId([id; 8])
    }

    #[test]
    fn broadcast_beacon_calls_radio_transmit() {
        let mut mock = MockRadio::new();
        mock.expect_transmit_frame()
            .times(1)
            .returning(|_| Ok(()));

        let mut svc = DiscoveryService::new(mock, test_node(1));
        let beacon = Beacon {
            node_id: test_node(1),
            zone_id: ZoneId([0; 4]),
            firmware_version: SemVer { major: 0, minor: 1, patch: 0 },
            uptime_secs: 3600,
        };
        svc.broadcast_beacon(&beacon).unwrap();
    }

    #[test]
    fn scan_neighbors_calls_radio_scan() {
        let mut mock = MockRadio::new();
        let mut beacons = heapless::Vec::new();
        let _ = beacons.push(BleBeacon { node_id: test_node(2), zone_id: ZoneId([1; 4]), rssi: -40 });
        let _ = beacons.push(BleBeacon { node_id: test_node(3), zone_id: ZoneId([1; 4]), rssi: -60 });
        let _ = beacons.push(BleBeacon { node_id: test_node(4), zone_id: ZoneId([2; 4]), rssi: -80 });

        mock.expect_scan_ble_beacons()
            .times(1)
            .return_once(move || Ok(beacons));

        let mut svc = DiscoveryService::new(mock, test_node(1));
        let neighbors = svc.scan_neighbors().unwrap();
        assert_eq!(neighbors.len(), 3);
        assert_eq!(neighbors[0].node_id, test_node(2));
    }

    #[test]
    fn scan_neighbors_filters_self() {
        let mut mock = MockRadio::new();
        let mut beacons = heapless::Vec::new();
        let _ = beacons.push(BleBeacon { node_id: test_node(1), zone_id: ZoneId([0; 4]), rssi: -20 });
        let _ = beacons.push(BleBeacon { node_id: test_node(2), zone_id: ZoneId([0; 4]), rssi: -50 });

        mock.expect_scan_ble_beacons()
            .return_once(move || Ok(beacons));

        let mut svc = DiscoveryService::new(mock, test_node(1));
        let neighbors = svc.scan_neighbors().unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].node_id, test_node(2));
    }
}
