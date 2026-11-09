//! Local approval is separate from both connection consent and Windows UAC.
use std::sync::atomic::{AtomicBool, Ordering};

static PENDING: AtomicBool = AtomicBool::new(false);

pub(crate) fn pending() -> bool {
    PENDING.load(Ordering::SeqCst)
}

pub(crate) fn local_prompt<F:FnOnce()->bool>(prompt:F)->bool {
    if PENDING.compare_exchange(false,true,Ordering::SeqCst,Ordering::SeqCst).is_err(){return false;}
    let _guard=PendingGuard;prompt()
}
struct PendingGuard;
impl Drop for PendingGuard {
    fn drop(&mut self) { PENDING.store(false, Ordering::SeqCst); }
}

pub(crate) fn confirm(peer_id: &str) -> bool {
    // Concurrent/repeated requests cannot stack approval windows.
    if PENDING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return false;
    }
    let _guard = PendingGuard;
    native_confirm(peer_id)
}

#[cfg(windows)]
fn native_confirm(peer_id: &str) -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut std::ffi::c_void, text: *const u16, title: *const u16, flags: u32) -> i32;
    }
    let peer: String = peer_id.chars().filter(|c| c.is_ascii_digit()).take(20).collect();
    let text = format!("Il tecnico collegato (ID {}) richiede i privilegi amministrativi su questo computer.\n\nVuoi autorizzare questa richiesta?\n\nSi: avvia la richiesta di elevazione di Windows.\nNo: continua l'assistenza senza elevare i privilegi.\n\nSe Windows mostra un'ulteriore conferma, deve approvarla l'utente di questo computer.", peer);
    let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Tec Assist - Richiesta privilegi".encode_utf16().chain(Some(0)).collect();
    // YESNO | ICONWARNING | DEFBUTTON2 (No) | SETFOREGROUND | TOPMOST.
    let result = unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x00050134) };
    result == 6 // IDYES; all errors and other results deny elevation.
}

#[cfg(not(windows))]
fn native_confirm(_peer_id: &str) -> bool { false }
