//! Per-key scripts of timed phases.

use std::collections::HashMap;
use std::hash::Hash;
use std::time::Duration;

/// A key moved from one phase to the next (`to: Some`) or finished its script (`to: None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change<K, P> {
    pub key: K,
    pub from: P,
    pub to: Option<P>,
}

struct Track<P> {
    steps: Vec<(P, Duration)>,
    index: usize,
    ends_at: Duration,
    seq: u64,
}

/// Keys playing scripts of `(phase, duration)` steps.
///
/// Time is a [`Duration`] since an origin the app chooses; pass the same clock to every call.
///
/// ```
/// use std::time::Duration;
/// use gus_anim_state::{Change, Timeline};
///
/// let ms = Duration::from_millis;
/// let mut t = Timeline::new();
/// t.play("row", &[("lingering", ms(600)), ("leaving", ms(220))], ms(0));
/// assert_eq!(t.phase(&"row"), Some(&"lingering"));
/// assert_eq!(t.next_deadline(), Some(ms(600)));
/// assert_eq!(t.advance(ms(600)), vec![Change { key: "row", from: "lingering", to: Some("leaving") }]);
/// assert_eq!(t.advance(ms(820)), vec![Change { key: "row", from: "leaving", to: None }]);
/// ```
pub struct Timeline<K, P> {
    tracks: HashMap<K, Track<P>>,
    next_seq: u64,
}

impl<K, P> Default for Timeline<K, P> {
    fn default() -> Self {
        Self { tracks: HashMap::new(), next_seq: 0 }
    }
}

impl<K: Hash + Eq + Clone, P: Clone> Timeline<K, P> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts `script` for `key` at `now`, replacing whatever the key was playing. The first
    /// phase is current immediately (no [`Change`] for it). An empty script is a [`cancel`](Self::cancel).
    pub fn play(&mut self, key: K, script: &[(P, Duration)], now: Duration) {
        let Some((_, first)) = script.first() else {
            self.tracks.remove(&key);
            return;
        };
        let seq = self.next_seq;
        self.next_seq += 1;
        self.tracks.insert(key, Track { steps: script.to_vec(), index: 0, ends_at: now.saturating_add(*first), seq });
    }

    /// Stops `key` at once, without a [`Change`]. Returns the phase it was in.
    pub fn cancel(&mut self, key: &K) -> Option<P> {
        self.tracks.remove(key).map(|mut track| track.steps.swap_remove(track.index).0)
    }

    /// Applies every step whose deadline is `<= now`, earliest first (ties: the key played
    /// first). A long gap yields every intermediate change, in order.
    pub fn advance(&mut self, now: Duration) -> Vec<Change<K, P>> {
        let mut changes = Vec::new();
        loop {
            let due = self.tracks.iter().filter(|(_, t)| t.ends_at <= now).min_by_key(|(_, t)| (t.ends_at, t.seq)).map(|(k, _)| k.clone());
            let Some(key) = due else { break };
            let Some(track) = self.tracks.get_mut(&key) else { break };
            let from = track.steps[track.index].0.clone();
            track.index += 1;
            match track.steps.get(track.index) {
                Some((phase, duration)) => {
                    let to = phase.clone();
                    track.ends_at = track.ends_at.saturating_add(*duration);
                    changes.push(Change { key, from, to: Some(to) });
                }
                None => {
                    self.tracks.remove(&key);
                    changes.push(Change { key, from, to: None });
                }
            }
        }
        changes
    }

    /// The phase `key` is in, if it is playing.
    pub fn phase(&self, key: &K) -> Option<&P> {
        self.tracks.get(key).map(|track| &track.steps[track.index].0)
    }

    /// Every key currently playing, in no particular order.
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.tracks.keys()
    }

    /// The earliest pending deadline: arm one timer for it after each `play` / `cancel` / `advance`.
    pub fn next_deadline(&self) -> Option<Duration> {
        self.tracks.values().map(|track| track.ends_at).min()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Change, Timeline};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Exit {
        Lingering,
        Leaving,
    }
    use Exit::{Leaving, Lingering};

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    const EXIT: [(Exit, Duration); 2] = [(Lingering, Duration::from_millis(600)), (Leaving, Duration::from_millis(220))];

    fn change(key: u32, from: Exit, to: Option<Exit>) -> Change<u32, Exit> {
        Change { key, from, to }
    }

    #[test]
    fn empty_timeline_has_nothing() {
        let mut t: Timeline<u32, Exit> = Timeline::new();
        assert_eq!(t.phase(&1), None);
        assert_eq!(t.keys().count(), 0);
        assert_eq!(t.next_deadline(), None);
        assert!(t.advance(ms(1_000)).is_empty());
    }

    #[test]
    fn play_starts_the_first_phase_immediately_without_a_change() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(100));
        assert_eq!(t.phase(&1), Some(&Lingering));
        assert_eq!(t.keys().copied().collect::<Vec<_>>(), vec![1]);
        assert_eq!(t.next_deadline(), Some(ms(700)));
    }

    #[test]
    fn script_runs_step_by_step_then_the_key_is_gone() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        assert!(t.advance(ms(599)).is_empty());
        assert_eq!(t.advance(ms(600)), vec![change(1, Lingering, Some(Leaving))]);
        assert_eq!(t.phase(&1), Some(&Leaving));
        assert_eq!(t.next_deadline(), Some(ms(820)));
        assert_eq!(t.advance(ms(820)), vec![change(1, Leaving, None)]);
        assert_eq!(t.phase(&1), None);
        assert_eq!(t.next_deadline(), None);
    }

    #[test]
    fn long_suspension_emits_every_step_in_order() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        assert_eq!(t.advance(ms(10_000)), vec![change(1, Lingering, Some(Leaving)), change(1, Leaving, None)]);
    }

    #[test]
    fn changes_across_keys_follow_deadlines_then_play_order() {
        let mut t = Timeline::new();
        t.play(2, &[(Leaving, ms(300))], ms(0));
        t.play(1, &EXIT, ms(0));
        t.play(3, &[(Leaving, ms(300))], ms(0));
        assert_eq!(
            t.advance(ms(1_000)),
            vec![change(2, Leaving, None), change(3, Leaving, None), change(1, Lingering, Some(Leaving)), change(1, Leaving, None)]
        );
    }

    #[test]
    fn play_on_a_playing_key_replaces_its_script() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(1, &[(Leaving, ms(220))], ms(100));
        assert_eq!(t.phase(&1), Some(&Leaving));
        assert_eq!(t.next_deadline(), Some(ms(320)));
        assert_eq!(t.advance(ms(1_000)), vec![change(1, Leaving, None)]);
    }

    #[test]
    fn replaying_restarts_the_clock() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(1, &EXIT, ms(400));
        assert!(t.advance(ms(999)).is_empty());
        assert_eq!(t.advance(ms(1_000)), vec![change(1, Lingering, Some(Leaving))]);
    }

    #[test]
    fn empty_script_cancels() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(1, &[], ms(100));
        assert_eq!(t.phase(&1), None);
        assert!(t.advance(ms(1_000)).is_empty());
    }

    #[test]
    fn cancel_removes_at_once_and_returns_the_phase() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.advance(ms(600));
        assert_eq!(t.cancel(&1), Some(Leaving));
        assert_eq!(t.phase(&1), None);
        assert_eq!(t.next_deadline(), None);
        assert!(t.advance(ms(1_000)).is_empty());
        assert_eq!(t.cancel(&1), None);
        assert_eq!(t.cancel(&99), None);
    }

    #[test]
    fn zero_duration_phase_changes_on_the_next_advance() {
        let mut t = Timeline::new();
        t.play(1, &[(Leaving, ms(0))], ms(50));
        assert_eq!(t.phase(&1), Some(&Leaving));
        assert_eq!(t.next_deadline(), Some(ms(50)));
        assert_eq!(t.advance(ms(50)), vec![change(1, Leaving, None)]);
    }

    #[test]
    fn time_going_backwards_does_nothing() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(1_000));
        assert!(t.advance(ms(0)).is_empty());
        assert_eq!(t.phase(&1), Some(&Lingering));
    }

    #[test]
    fn huge_durations_saturate() {
        let mut t = Timeline::new();
        t.play(1, &[(Lingering, Duration::MAX), (Leaving, Duration::MAX)], ms(5));
        assert_eq!(t.next_deadline(), Some(Duration::MAX));
        assert_eq!(t.advance(Duration::MAX), vec![change(1, Lingering, Some(Leaving)), change(1, Leaving, None)]);
    }

    #[test]
    fn independent_keys_keep_their_own_scripts() {
        let mut t = Timeline::new();
        t.play(1, &EXIT, ms(0));
        t.play(2, &EXIT, ms(500));
        assert_eq!(t.advance(ms(600)), vec![change(1, Lingering, Some(Leaving))]);
        assert_eq!(t.phase(&2), Some(&Lingering));
        assert_eq!(t.next_deadline(), Some(ms(820)));
    }
}
