use std::time::{Duration, Instant};

use godot::prelude::*;

use crate::godot_classes::{dcl_android_plugin::DclAndroidPlugin, dcl_ios_plugin::DclIosPlugin};

use super::{
    data_definition::{non_zero_id, DeviceIdentifiers},
    install_attribution::get_string,
};

/// How long the first batch may wait for the Android plugin's first Play services read, so this
/// launch's System Info Report and App Opened carry the ids. Fails open: past it, they ship
/// without.
const ANDROID_FIRST_READ_WAIT: Duration = Duration::from_secs(5);

/// The marketing device identifiers (#3272) as last read from the platform plugin. Reading is
/// local; nothing here sends anything, so it is safe before the EULA gate opens.
#[derive(Default)]
pub struct DeviceIdentifierReader {
    current: Option<DeviceIdentifiers>,
    // Android only: the plugin has not finished its first read yet.
    pending: bool,
    wait_until: Option<Instant>,
    identify_sent: Option<DeviceIdentifiers>,
}

impl DeviceIdentifierReader {
    pub fn start(&mut self) {
        if DclAndroidPlugin::is_available() {
            self.wait_until = Some(Instant::now() + ANDROID_FIRST_READ_WAIT);
        }
        self.refresh();
    }

    /// Re-reads the ids. Cheap on both platforms: the plugins answer from memory.
    pub fn refresh(&mut self) {
        if DclIosPlugin::is_available() {
            self.current = read_ios();
        } else if DclAndroidPlugin::is_available() {
            match DclAndroidPlugin::get_advertising_ids_internal() {
                Some(dict) if get_string(&dict, "status") == "ok" => {
                    self.pending = false;
                    self.current = Some(read_android(&dict));
                }
                Some(_) => self.pending = true,
                None => {
                    self.pending = false;
                    self.current = None;
                }
            }
        }
    }

    /// `None` on desktop and on a plugin binary that predates these ids.
    pub fn current(&self) -> Option<&DeviceIdentifiers> {
        self.current.as_ref()
    }

    /// Whether a batch should wait for the first Android read.
    pub fn is_holding(&self) -> bool {
        self.pending && self.wait_until.is_some_and(|until| Instant::now() < until)
    }

    /// The ids, when they changed since the last time this returned them.
    pub fn take_changed(&mut self) -> Option<DeviceIdentifiers> {
        let current = self.current.clone()?;
        if self.identify_sent.as_ref() == Some(&current) {
            return None;
        }
        self.identify_sent = Some(current.clone());
        Some(current)
    }
}

fn read_ios() -> Option<DeviceIdentifiers> {
    let att_status = DclIosPlugin::get_tracking_authorization_status()?;
    let idfa = if att_status == "authorized" {
        non_zero_id(DclIosPlugin::get_advertising_id())
    } else {
        None
    };
    Some(DeviceIdentifiers::Ios {
        idfv: non_zero_id(DclIosPlugin::get_vendor_id()),
        idfa,
        att_status,
        tenjin_analytics_installation_id: DclIosPlugin::tenjin_get_analytics_installation_id(),
    })
}

fn read_android(dict: &VarDictionary) -> DeviceIdentifiers {
    let gaid = non_zero_id(Some(get_string(dict, "gaid")));
    let opted_out = dict
        .get("limit_ad_tracking")
        .and_then(|v| v.try_to::<bool>().ok())
        .unwrap_or(true);
    DeviceIdentifiers::Android {
        limit_ad_tracking: opted_out || gaid.is_none(),
        gaid,
        app_set_id: non_zero_id(Some(get_string(dict, "app_set_id"))),
        tenjin_analytics_installation_id: DclAndroidPlugin::tenjin_get_analytics_installation_id(),
    }
}
