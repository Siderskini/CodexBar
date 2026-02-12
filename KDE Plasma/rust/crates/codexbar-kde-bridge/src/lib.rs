use codexbar_core::WidgetSnapshot;
use serde::{Deserialize, Serialize};

pub const DBUS_SERVICE_NAME: &str = "dev.codexbar.WidgetService";
pub const DBUS_OBJECT_PATH: &str = "/dev/codexbar/WidgetService";
pub const DBUS_INTERFACE_NAME: &str = "dev.codexbar.WidgetService";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEnvelope {
    pub schema_version: u32,
    pub snapshot: WidgetSnapshot,
}

impl SnapshotEnvelope {
    pub fn new(snapshot: WidgetSnapshot) -> Self {
        Self {
            schema_version: 1,
            snapshot,
        }
    }
}

pub trait SnapshotProvider {
    fn current_snapshot(&self) -> SnapshotEnvelope;
}
