//! Launch status tracking state machine.
//! GAP AREA: C++ had no tests for LaunchingStatus class.
//!
//! Tracks per-app launch progress: Pending → Launched → Moved → Done.

use std::collections::HashMap;

/// Status of a single app in a workspace launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLaunchState {
    Pending,
    Launched,
    Moved,
    Failed,
}

/// Tracks launch progress for all apps in a workspace.
#[derive(Debug)]
pub struct LaunchStatus {
    apps: HashMap<String, AppLaunchState>,
}

impl LaunchStatus {
    pub fn new(app_ids: &[String]) -> Self {
        let apps = app_ids.iter().map(|id| (id.clone(), AppLaunchState::Pending)).collect();
        Self { apps }
    }

    pub fn mark_launched(&mut self, app_id: &str) -> bool {
        if let Some(state) = self.apps.get_mut(app_id) {
            if *state == AppLaunchState::Pending {
                *state = AppLaunchState::Launched;
                return true;
            }
        }
        false
    }

    pub fn mark_moved(&mut self, app_id: &str) -> bool {
        if let Some(state) = self.apps.get_mut(app_id) {
            if *state == AppLaunchState::Launched {
                *state = AppLaunchState::Moved;
                return true;
            }
        }
        false
    }

    pub fn mark_failed(&mut self, app_id: &str) -> bool {
        if let Some(state) = self.apps.get_mut(app_id) {
            *state = AppLaunchState::Failed;
            return true;
        }
        false
    }

    pub fn get_state(&self, app_id: &str) -> Option<AppLaunchState> {
        self.apps.get(app_id).copied()
    }

    /// Are all apps launched (or beyond)?
    pub fn all_launched(&self) -> bool {
        self.apps.values().all(|s| *s != AppLaunchState::Pending)
    }

    /// Are all apps moved to their final position (or failed)?
    pub fn all_done(&self) -> bool {
        self.apps.values().all(|s| matches!(s, AppLaunchState::Moved | AppLaunchState::Failed))
    }

    pub fn pending_count(&self) -> usize {
        self.apps.values().filter(|s| **s == AppLaunchState::Pending).count()
    }

    pub fn launched_count(&self) -> usize {
        self.apps.values().filter(|s| **s == AppLaunchState::Launched).count()
    }

    pub fn moved_count(&self) -> usize {
        self.apps.values().filter(|s| **s == AppLaunchState::Moved).count()
    }

    pub fn failed_count(&self) -> usize {
        self.apps.values().filter(|s| **s == AppLaunchState::Failed).count()
    }

    pub fn total(&self) -> usize {
        self.apps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ids(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("app-{}", i)).collect()
    }

    #[test]
    fn new_all_pending() {
        let status = LaunchStatus::new(&make_ids(3));
        assert_eq!(status.pending_count(), 3);
        assert_eq!(status.total(), 3);
        assert!(!status.all_launched());
        assert!(!status.all_done());
    }

    #[test]
    fn mark_launched() {
        let mut status = LaunchStatus::new(&make_ids(2));
        assert!(status.mark_launched("app-0"));
        assert_eq!(status.get_state("app-0"), Some(AppLaunchState::Launched));
        assert_eq!(status.pending_count(), 1);
        assert_eq!(status.launched_count(), 1);
    }

    #[test]
    fn mark_moved() {
        let mut status = LaunchStatus::new(&make_ids(1));
        status.mark_launched("app-0");
        assert!(status.mark_moved("app-0"));
        assert_eq!(status.get_state("app-0"), Some(AppLaunchState::Moved));
        assert!(status.all_done());
    }

    #[test]
    fn mark_failed() {
        let mut status = LaunchStatus::new(&make_ids(2));
        status.mark_launched("app-0");
        status.mark_launched("app-1");
        assert!(status.mark_failed("app-0"));
        assert_eq!(status.failed_count(), 1);
        status.mark_moved("app-1");
        assert!(status.all_done());
    }

    #[test]
    fn cannot_move_before_launch() {
        let mut status = LaunchStatus::new(&make_ids(1));
        assert!(!status.mark_moved("app-0")); // still pending
        assert_eq!(status.get_state("app-0"), Some(AppLaunchState::Pending));
    }

    #[test]
    fn cannot_launch_twice() {
        let mut status = LaunchStatus::new(&make_ids(1));
        assert!(status.mark_launched("app-0"));
        assert!(!status.mark_launched("app-0")); // already launched
    }

    #[test]
    fn unknown_app_id() {
        let mut status = LaunchStatus::new(&make_ids(1));
        assert!(!status.mark_launched("nonexistent"));
        assert!(status.get_state("nonexistent").is_none());
    }

    #[test]
    fn all_launched_check() {
        let mut status = LaunchStatus::new(&make_ids(3));
        status.mark_launched("app-0");
        status.mark_launched("app-1");
        assert!(!status.all_launched()); // app-2 still pending
        status.mark_launched("app-2");
        assert!(status.all_launched());
    }

    #[test]
    fn all_done_mixed_moved_failed() {
        let mut status = LaunchStatus::new(&make_ids(3));
        for id in &make_ids(3) {
            status.mark_launched(id);
        }
        status.mark_moved("app-0");
        status.mark_moved("app-1");
        status.mark_failed("app-2");
        assert!(status.all_done());
    }

    #[test]
    fn empty_workspace() {
        let status = LaunchStatus::new(&[]);
        assert!(status.all_launched());
        assert!(status.all_done());
        assert_eq!(status.total(), 0);
    }

    #[test]
    fn full_lifecycle() {
        let ids = make_ids(2);
        let mut status = LaunchStatus::new(&ids);

        // pending → launched
        for id in &ids { status.mark_launched(id); }
        assert!(status.all_launched());
        assert!(!status.all_done());

        // launched → moved
        for id in &ids { status.mark_moved(id); }
        assert!(status.all_done());
        assert_eq!(status.moved_count(), 2);
    }

    #[test]
    fn fail_from_any_state() {
        let mut status = LaunchStatus::new(&make_ids(1));
        // fail from pending
        assert!(status.mark_failed("app-0"));
        assert_eq!(status.get_state("app-0"), Some(AppLaunchState::Failed));
        assert!(status.all_done());
    }
}
