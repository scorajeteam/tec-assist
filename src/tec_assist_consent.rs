// Tec Assist: consent belongs to one connection, never to a device-wide preference.
#[derive(Default)]
pub(crate) struct ConsentGate {
    password_verified: bool,
    accepted: bool,
}
impl ConsentGate {
    pub(crate) fn password_checked(&mut self, verified: bool) {
        self.password_verified = verified;
        self.accepted = false;
    }
    pub(crate) fn password_verified(&self) -> bool { self.password_verified }
    // The manager must never start while it has no verified request to display.
    pub(crate) fn may_start_manager(&self) -> bool { self.password_verified }
    pub(crate) fn approve(&mut self) -> bool {
        if !self.password_verified { return false; }
        self.accepted = true;
        true
    }
    pub(crate) fn is_authorized(&self) -> bool {
        self.password_verified && self.accepted
    }
}
#[cfg(test)]
mod tests {
    use super::ConsentGate;
    #[test]
    fn new_connection_is_not_authorized() { assert!(!ConsentGate::default().is_authorized()); }
    #[test]
    fn approval_without_password_is_rejected() {
        let mut gate = ConsentGate::default();
        assert!(!gate.approve()); assert!(!gate.is_authorized());
    }
    #[test]
    fn password_alone_is_not_enough() {
        let mut gate = ConsentGate::default(); gate.password_checked(true);
        assert!(!gate.is_authorized());
    }
    #[test]
    fn password_and_local_approval_are_both_required() {
        let mut gate = ConsentGate::default(); gate.password_checked(true);
        assert!(gate.approve()); assert!(gate.is_authorized());
    }
    #[test]
    fn failed_password_revokes_previous_approval() {
        let mut gate = ConsentGate::default(); gate.password_checked(true); gate.approve();
        gate.password_checked(false); assert!(!gate.password_verified());
        assert!(!gate.approve()); assert!(!gate.is_authorized());
    }
    #[test]
    fn a_second_connection_does_not_inherit_consent() {
        let mut first = ConsentGate::default(); first.password_checked(true); first.approve();
        let mut second = ConsentGate::default(); second.password_checked(true);
        assert!(first.is_authorized()); assert!(!second.is_authorized());
    }
    #[test]
    fn manager_is_not_started_while_the_operator_enters_password() {
        let mut gate = ConsentGate::default();
        assert!(!gate.may_start_manager());
        gate.password_checked(false);
        assert!(!gate.may_start_manager());
    }
    #[test]
    fn manager_starts_after_password_but_before_access_is_granted() {
        let mut gate = ConsentGate::default();
        gate.password_checked(true);
        assert!(gate.may_start_manager());
        assert!(!gate.is_authorized());
        assert!(gate.approve());
        assert!(gate.is_authorized());
    }
}