//! Pure policy for a temporary reboot continuation. No OS or network side effects.
//! The caller must verify the cryptographic proof before passing proof_valid=true.
pub const MAX_LIFETIME: u64 = 30 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub peer_id: String,
    pub session_id: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub boot_marker: u128,
    pub revoked: bool,
}

impl Lease {
    pub fn new(peer_id: String, session_id: u64, now: u64, boot: u128,
               connected: bool, locally_approved: bool) -> Option<Self> {
        if !connected || !locally_approved || peer_id.is_empty() || session_id == 0 || now == 0 {
            return None;
        }
        Some(Self { peer_id, session_id, issued_at: now,
            expires_at: now.checked_add(MAX_LIFETIME)?, boot_marker: boot, revoked: false })
    }

    pub fn alive(&self, now: u64) -> bool {
        !self.revoked && now >= self.issued_at && now < self.expires_at
            && self.expires_at.checked_sub(self.issued_at) == Some(MAX_LIFETIME)
    }

    pub fn may_resume(&self, peer: &str, session: u64, now: u64,
                      boot: u128, proof_valid: bool, desktop_only: bool) -> bool {
        self.alive(now) && boot != self.boot_marker && proof_valid && desktop_only
            && self.peer_id == peer && self.session_id == session
    }

    pub fn revoke(&mut self) { self.revoked = true; }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lease() -> Lease { Lease::new("1439452503".into(), 27, 1_000, 10, true, true).unwrap() }
    fn valid(l: &Lease) -> bool { l.may_resume("1439452503", 27, 1_100, 20, true, true) }
    #[test] fn requires_live_connection() { assert!(Lease::new("a".into(), 1, 1, 1, false, true).is_none()); }
    #[test] fn requires_local_consent() { assert!(Lease::new("a".into(), 1, 1, 1, true, false).is_none()); }
    #[test] fn rejects_empty_peer() { assert!(Lease::new("".into(), 1, 1, 1, true, true).is_none()); }
    #[test] fn rejects_empty_session() { assert!(Lease::new("a".into(), 0, 1, 1, true, true).is_none()); }
    #[test] fn rejects_overflow() { assert!(Lease::new("a".into(), 1, u64::MAX, 1, true, true).is_none()); }
    #[test] fn same_session_can_resume() { assert!(valid(&lease())); }
    #[test] fn peer_id_alone_is_not_authentication() { assert!(!lease().may_resume("1439452503", 27, 1_100, 20, false, true)); }
    #[test] fn different_peer_is_rejected() { assert!(!lease().may_resume("other", 27, 1_100, 20, true, true)); }
    #[test] fn new_session_is_rejected() { assert!(!lease().may_resume("1439452503", 28, 1_100, 20, true, true)); }
    #[test] fn no_bypass_before_reboot() { assert!(!lease().may_resume("1439452503", 27, 1_100, 10, true, true)); }
    #[test] fn expired_is_rejected() { assert!(!lease().may_resume("1439452503", 27, 2_800, 20, true, true)); }
    #[test] fn last_second_is_valid() { assert!(lease().may_resume("1439452503", 27, 2_799, 20, true, true)); }
    #[test] fn clock_rollback_fails_closed() { assert!(!lease().may_resume("1439452503", 27, 999, 20, true, true)); }
    #[test] fn arbitrary_extended_expiry_fails_closed() { let mut l = lease(); l.expires_at += 1; assert!(!valid(&l)); }
    #[test] fn file_transfer_cannot_inherit_desktop_consent() { assert!(!lease().may_resume("1439452503", 27, 1_100, 20, true, false)); }
    #[test] fn termination_revokes_resume() { let mut l = lease(); l.revoke(); assert!(!valid(&l)); }
    #[test] fn reconnect_does_not_extend_deadline() { let l = lease(); assert!(valid(&l)); assert_eq!(l.expires_at, 2_800); }
    #[test] fn revocation_cannot_be_reset_by_time() { let mut l = lease(); l.revoke(); assert!(!l.alive(1_001)); assert!(!l.alive(2_799)); }
}
