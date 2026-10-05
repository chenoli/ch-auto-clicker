use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    Idle,
    Countdown,
    Running,
    Paused,
    Stopping,
    Error,
}

impl EngineState {
    pub const ALL: [EngineState; 6] = [
        Self::Idle,
        Self::Countdown,
        Self::Running,
        Self::Paused,
        Self::Stopping,
        Self::Error,
    ];

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(v: u8) -> Self {
        Self::ALL.get(usize::from(v)).copied().unwrap_or(Self::Idle)
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Countdown | Self::Running | Self::Paused)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    User,
    Timer,
    Emergency,
    Completed,
    Error,
    ProfileSwitch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineEvent {
    Start { countdown: bool },
    CountdownElapsed,
    Cancel,
    TogglePause,
    Stop,
    Stopped,
    Fail,
    Ack,
}

pub fn transition(state: EngineState, event: EngineEvent) -> Option<EngineState> {
    use EngineEvent as E;
    use EngineState as S;
    Some(match (state, event) {
        (S::Idle | S::Error, E::Start { countdown: true }) => S::Countdown,
        (S::Idle | S::Error, E::Start { countdown: false }) => S::Running,
        (S::Error, E::Ack) => S::Idle,
        (S::Countdown, E::CountdownElapsed) => S::Running,
        (S::Countdown, E::Cancel | E::Stop) => S::Idle,
        (S::Running, E::TogglePause) => S::Paused,
        (S::Paused, E::TogglePause) => S::Running,
        (S::Running | S::Paused, E::Stop | E::Fail) => S::Stopping,
        (S::Stopping, E::Stopped) => S::Idle,
        (S::Stopping, E::Fail) => S::Error,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::EngineEvent as E;
    use super::EngineState as S;
    use super::*;

    const EVENTS: [EngineEvent; 9] = [
        E::Start { countdown: true },
        E::Start { countdown: false },
        E::CountdownElapsed,
        E::Cancel,
        E::TogglePause,
        E::Stop,
        E::Stopped,
        E::Fail,
        E::Ack,
    ];

    #[test]
    fn documented_transitions() {
        let table = [
            (S::Idle, E::Start { countdown: true }, S::Countdown),
            (S::Idle, E::Start { countdown: false }, S::Running),
            (S::Error, E::Start { countdown: true }, S::Countdown),
            (S::Error, E::Ack, S::Idle),
            (S::Countdown, E::CountdownElapsed, S::Running),
            (S::Countdown, E::Cancel, S::Idle),
            (S::Countdown, E::Stop, S::Idle),
            (S::Running, E::TogglePause, S::Paused),
            (S::Paused, E::TogglePause, S::Running),
            (S::Running, E::Stop, S::Stopping),
            (S::Paused, E::Stop, S::Stopping),
            (S::Running, E::Fail, S::Stopping),
            (S::Stopping, E::Stopped, S::Idle),
            (S::Stopping, E::Fail, S::Error),
        ];
        for (from, event, to) in table {
            assert_eq!(transition(from, event), Some(to), "{from:?} + {event:?}");
        }
    }

    #[test]
    fn everything_else_is_ignored() {
        let valid = |s: S, e: E| transition(s, e).is_some();
        let count: usize = S::ALL
            .iter()
            .map(|&s| EVENTS.iter().filter(|&&e| valid(s, e)).count())
            .sum();
        assert_eq!(count, 16, "only the documented transitions exist");
        assert_eq!(transition(S::Idle, E::TogglePause), None);
        assert_eq!(transition(S::Running, E::Start { countdown: true }), None);
        assert_eq!(transition(S::Stopping, E::Stop), None);
    }

    #[test]
    fn round_trips_through_u8() {
        for s in S::ALL {
            assert_eq!(S::from_u8(s.to_u8()), s);
        }
        assert_eq!(S::from_u8(200), S::Idle);
    }

    #[test]
    fn active_states() {
        let active: Vec<_> = S::ALL.into_iter().filter(|s| s.is_active()).collect();
        assert_eq!(active, vec![S::Countdown, S::Running, S::Paused]);
    }

    #[test]
    fn serializes_for_the_ui() {
        assert_eq!(
            serde_json::to_value(S::Countdown).expect("json"),
            "countdown"
        );
        assert_eq!(
            serde_json::to_value(StopReason::ProfileSwitch).expect("json"),
            "profileSwitch"
        );
    }
}
