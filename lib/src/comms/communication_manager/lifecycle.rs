use super::*;

impl CommunicationManager {
    pub(super) fn stop_comms(&mut self, reason: i32) {
        if self.block_auto_reconnect {
            return;
        }
        self.block_auto_reconnect = true;
        self.recovery.stopped = Some(reason);
        let connection = self.current_connection_str.clone();
        self.clean();
        self.current_connection_str = connection;
        self.saved_adapter_for_resume = GString::default();
        self.base_mut()
            .emit_signal("disconnected", &[reason.to_variant()]);
    }

    pub(super) fn sync_island_with_pulse(&mut self) {
        #[cfg(feature = "use_livekit")]
        {
            #[cfg(feature = "use_pulse")]
            let ready = !self.block_auto_reconnect
                && self
                    .pulse_room
                    .as_ref()
                    .is_some_and(PulseRoom::is_established);
            #[cfg(not(feature = "use_pulse"))]
            let ready = false;
            if let CommsConnection::Archipelago(archipelago) = &mut self.current_connection {
                archipelago.set_pulse_ready(ready);
            }
        }
    }

    #[cfg(feature = "use_pulse")]
    pub(super) fn poll_pulse_room(&mut self) {
        if self.block_auto_reconnect || self.comms_on_hold {
            return;
        }
        let now = Instant::now();
        let Some(room) = self.pulse_room.as_mut() else {
            return;
        };
        let events = room.poll(now, || {
            let identity = DclGlobal::singleton().bind().get_player_identity();
            let identity = identity.bind();
            Some((
                identity.try_get_ephemeral_auth_chain()?,
                identity
                    .clone_profile()
                    .map(|p| p.version as i32)
                    .unwrap_or(0),
            ))
        });

        // A terminal reason outranks the failed-attempt event from the same poll.
        if let Some(reason) = events.iter().find_map(|event| match event {
            PulseRoomEvent::Dead { reason } => Some(*reason),
            _ => None,
        }) {
            match reason {
                Some(PulseDisconnect::DuplicateSession)
                    if self.recovery.reclaim_after_resume(now) =>
                {
                    if let Some(mut room) = self.pulse_room.take() {
                        room.clean();
                    }
                    self.sync_island_with_pulse();
                    self.ensure_pulse_room();
                }
                Some(PulseDisconnect::DuplicateSession) => self.stop_comms(0),
                Some(PulseDisconnect::Banned) => self.stop_comms(2),
                _ => self.stop_comms(4),
            }
            return;
        }
        for event in events {
            match event {
                PulseRoomEvent::Established => {
                    self.pulse_teleport_pending = true;
                    self.recovery.established();
                    self.base_mut().emit_signal("pulse_recovered", &[]);
                }
                PulseRoomEvent::AttemptFailed => {
                    self.recovery.failures = self.recovery.failures.saturating_add(1);
                }
                PulseRoomEvent::Dead { .. } => unreachable!(),
            }
        }
        if self
            .pulse_room
            .as_ref()
            .is_some_and(|room| !room.is_established())
            && self.recovery.unavailable(now)
        {
            self.base_mut()
                .emit_signal("disconnected", &[4i32.to_variant()]);
        }
        if self.pulse_teleport_pending {
            self.try_send_pulse_teleport();
        }
    }

    pub(super) fn retry_pulse_connection(&mut self) {
        // Eviction requires sign-in; a ban is handled by the ban navigation flow.
        if matches!(self.recovery.stopped, Some(0 | 2)) {
            return;
        }
        self.block_auto_reconnect = false;
        self.recovery.stopped = None;
        self.recovery.reset_outage();
        #[cfg(feature = "use_pulse")]
        {
            if let Some(mut room) = self.pulse_room.take() {
                room.clean();
            }
            self.sync_island_with_pulse();
            self.ensure_pulse_room();
        }
        let connection = self.current_connection_str.clone();
        if !connection.is_empty() && matches!(self.current_connection, CommsConnection::None) {
            self.change_adapter(connection);
        }
        #[cfg(feature = "use_livekit")]
        {
            let scene_id = DclGlobal::singleton()
                .bind()
                .get_scene_runner()
                .bind()
                .get_current_parcel_scene_id();
            self._on_change_scene_id(scene_id);
        }
    }
}
