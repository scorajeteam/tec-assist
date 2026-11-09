//! Windows-only temporary reboot continuation. No permanent password or Windows auto-login.
use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use hbb_common::{config::Config, sha2::{Digest, Sha256}, message_proto::{LoginRequest, Hash}};
use serde::{Serialize, Deserialize};
use hbb_common::rand::{self, RngCore};
#[path="tec_assist_resume_policy.rs"] mod policy;

pub(crate) const TASK_NAME: &str = "TecAssist-RipresaTemporanea";
const TTL: u64 = 1800;
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Ticket {
    pub peer_id: String, pub session_id: u64, pub issued_at: u64, pub expires_at: u64,
    pub boot: String, pub token: String,
    pub permissions: [bool; 7],
    #[serde(default)] pub transfer_expires_at: u64,
    #[serde(default)] pub source: Option<String>, #[serde(default)] pub config: Option<String>,
}
lazy_static::lazy_static! {
    static ref CLIENT: Mutex<HashMap<(String, u64), Ticket>> = Mutex::new(HashMap::new());
}
pub(crate) fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|x|x.as_secs()).unwrap_or(0) }
pub(crate) fn root() -> PathBuf { PathBuf::from(std::env::var_os("ProgramFiles").unwrap_or_default()).join("TecAssistResume") }
pub(crate) fn host() -> bool {
    std::env::current_exe().ok().and_then(|x|x.parent().map(|x|x.to_path_buf()))
        .map(|x| x == root().join("app")).unwrap_or(false)
}
fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b|format!("{b:02x}")).collect() }

#[repr(C)] struct Blob { len: u32, ptr: *mut u8 }
#[link(name="crypt32")]
extern "system" {
    fn CryptProtectData(input: *const Blob, desc: *const u16, entropy: *const Blob, reserved: *mut u8, prompt: *mut u8, flags: u32, output: *mut Blob) -> i32;
    fn CryptUnprotectData(input: *const Blob, desc: *mut *mut u16, entropy: *const Blob, reserved: *mut u8, prompt: *mut u8, flags: u32, output: *mut Blob) -> i32;
}
#[link(name="kernel32")]
extern "system" { fn LocalFree(p: *mut u8) -> *mut u8; }
fn dpapi(bytes: &[u8], protect: bool) -> Result<Vec<u8>, String> {
    dpapi_scope(bytes,protect,false)
}
fn dpapi_scope(bytes: &[u8], protect: bool, machine:bool) -> Result<Vec<u8>, String> {
    let input = Blob { len: bytes.len() as u32, ptr: bytes.as_ptr() as *mut u8 };
    let mut output = Blob { len:0, ptr:std::ptr::null_mut() };
    let ok = unsafe {
        if protect { CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), if machine {5}else{1}, &mut output) }
        else { CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), 1, &mut output) }
    };
    if ok == 0 { return Err("Protezione Windows non disponibile".into()); }
    let result = unsafe { std::slice::from_raw_parts(output.ptr, output.len as usize).to_vec() };
    unsafe { LocalFree(output.ptr); }
    Ok(result)
}
fn protect_request_directory(path:&std::path::Path)->Result<(),String> {
    #[link(name="advapi32")]
    extern "system" {
        fn ConvertStringSecurityDescriptorToSecurityDescriptorW(text:*const u16,revision:u32,descriptor:*mut *mut u8,size:*mut u32)->i32;
        fn SetFileSecurityW(path:*const u16,information:u32,descriptor:*mut u8)->i32;
    }
    use std::os::windows::ffi::OsStrExt;
    // Owner, SYSTEM and elevated administrators only, inherited by request files.
    let sddl:Vec<u16>="D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FA;;;OW)".encode_utf16().chain(Some(0)).collect();
    let name:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut descriptor=std::ptr::null_mut();
    if unsafe{ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(),1,&mut descriptor,std::ptr::null_mut())}==0 {return Err("Impossibile proteggere la richiesta temporanea.".into());}
    let ok=unsafe{SetFileSecurityW(name.as_ptr(),0x80000004,descriptor)};
    unsafe{LocalFree(descriptor);}
    if ok!=0{Ok(())}else{Err("Permessi della richiesta temporanea non applicati.".into())}
}

#[repr(C)] struct BootInfo { boot_time:i64, current_time:i64, time_zone_bias:i64, time_zone_id:u32, reserved:u32, boot_time_bias:u64, sleep_time_bias:u64 }
#[link(name="ntdll")]
extern "system" { fn NtQuerySystemInformation(class:u32, info:*mut BootInfo, len:u32, returned:*mut u32)->i32; }
fn boot() -> Result<String,String> {
    let mut info:BootInfo=unsafe{std::mem::zeroed()}; let mut returned = 0;
    // SystemTimeOfDayInformation: actual kernel boot time, NOT the BCD boot-entry GUID.
    let status = unsafe { NtQuerySystemInformation(3, &mut info, std::mem::size_of::<BootInfo>() as u32, &mut returned) };
    if status != 0 || info.boot_time<=0 { Err("Impossibile identificare l'avvio Windows".into()) } else { Ok(format!("{:032x}",info.boot_time as u128)) }
}
fn live(t:&Ticket) -> bool {
    !t.peer_id.is_empty() && t.session_id != 0 && t.token.len()==64 && t.boot.len()==32
        && lease(t).map(|l|l.alive(now())).unwrap_or(false)
}
fn lease(t:&Ticket)->Option<policy::Lease> { Some(policy::Lease {peer_id:t.peer_id.clone(),session_id:t.session_id,
    issued_at:t.issued_at,expires_at:t.expires_at,boot_marker:u128::from_str_radix(&t.boot,16).ok()?,revoked:false}) }
fn marker(name:&str) -> bool { std::fs::read(root().join(name)).map(|x|x==[1]).unwrap_or(false) }
fn trace(event:&str) {
    use std::io::Write;
    if host(){if let Ok(mut f)=std::fs::OpenOptions::new().append(true).open(root().join("runtime-status.log")){let _=writeln!(f,"{} {}",now(),event);}}
}
pub(crate) fn stage(label:&'static str) {
    if host(){trace(label);return;}
    if crate::get_app_name()!="TecAssistConsole"{return;}
    use std::io::Write;
    let folder=Config::log_path();let _=std::fs::create_dir_all(&folder);
    if let Ok(mut file)=std::fs::OpenOptions::new().create(true).append(true).open(folder.join("resume-stages.log")){let _=writeln!(file,"{} {}",now(),label);}
}
fn checked_ticket()->Result<Ticket,&'static str> {
    if !host(){return Err("host-path-invalid");}
    if std::fs::read(root().join("revoked")).map_err(|_|"revocation-unreadable")? != [0]{return Err("revoked");}
    if !marker("committed"){return Err("not-committed");}
    let raw=dpapi(&std::fs::read(root().join("lease.dpapi")).map_err(|_|"ticket-unreadable")?,false).map_err(|_|"ticket-decryption-failed")?;
    let t:Ticket=serde_json::from_slice(&raw).map_err(|_|"ticket-format-invalid")?;
    if !live(&t){return Err("ticket-expired-or-invalid");}
    if boot().map_err(|_|"boot-time-unavailable")?==t.boot{return Err("same-boot");}
    Ok(t)
}
pub(crate) fn read_ticket() -> Option<Ticket> {checked_ticket().ok()}
pub(crate) fn restore_password()->bool {
    if checked_ticket().is_err(){return false;}
    let raw=match std::fs::read(root().join("lease.dpapi")).ok().and_then(|x|dpapi(&x,false).ok()){Some(x)=>x,None=>return false};
    let value:serde_json::Value=match serde_json::from_slice(&raw){Ok(x)=>x,Err(_)=>return false};
    if let Some(password)=value.get("temporary_password").and_then(|v|v.as_str()).filter(|p|p.len()>=6&&p.len()<=32){
        *hbb_common::password_security::TEMPORARY_PASSWORD.write().unwrap()=password.to_owned();trace("temporary-password-restored");true
    }else{trace("temporary-password-unavailable");false}
}
fn proof(t:&Ticket, hash:&Hash) -> Vec<u8> {
    use hbb_common::sodiumoxide::crypto::auth;
    let key_bytes=Sha256::digest(t.token.as_bytes());
    let key=auth::Key::from_slice(&key_bytes).expect("SHA256 key length");
    let mut message=b"TecAssist-Reboot-v1\0".to_vec();
    for part in [hash.salt.as_bytes(),hash.challenge.as_bytes(),t.peer_id.as_bytes()] {
        message.extend_from_slice(&(part.len() as u64).to_le_bytes()); message.extend_from_slice(part);
    }
    message.extend_from_slice(&t.session_id.to_le_bytes());
    auth::authenticate(&message,&key).as_ref().to_vec()
}
pub(crate) fn login_proof(remote:&str, session:u64, hash:&Hash) -> Vec<u8> {
    credentials(remote,session,hash).1
}
fn cache_file(remote:&str)->PathBuf {
    let canonical=remote.split_whitespace().collect::<String>().to_lowercase();
    Config::path("TecAssistResumeTickets-v1").join(format!("{}.dpapi",hex(&Sha256::digest(canonical.as_bytes()))))
}
fn save_client_ticket(remote:&str,t:&Ticket)->Result<(),String> {
    let path=cache_file(remote);let dir=path.parent().ok_or("Percorso ripresa non disponibile")?;
    std::fs::create_dir_all(dir).map_err(|e|e.to_string())?;protect_request_directory(dir)?;
    let bytes=dpapi(&serde_json::to_vec(t).map_err(|e|e.to_string())?,true)?;
    let temporary=dir.join(format!("{}.tmp",uuid::Uuid::new_v4()));
    std::fs::write(&temporary,bytes).map_err(|e|e.to_string())?;
    let result=std::fs::rename(&temporary,path).map_err(|e|e.to_string());
    let _=std::fs::remove_file(temporary);result
}
pub(crate) fn forget(remote:&str) {
    CLIENT.lock().unwrap().retain(|(id,_),_|id!=remote);
    let _=std::fs::remove_file(cache_file(remote));
}
fn client_ticket(remote:&str)->Option<Ticket> {
    if crate::get_app_name()!="TecAssistConsole"{return None;}
    {let mut tickets=CLIENT.lock().unwrap();tickets.retain(|_,t|live(t));
        if let Some((_,t))=tickets.iter().find(|((id,_),_)|id==remote){return Some(t.clone());}}
    let raw=std::fs::read(cache_file(remote)).ok().and_then(|x|dpapi(&x,false).ok())?;
    let t:Ticket=serde_json::from_slice(&raw).ok()?;
    if !live(&t)||t.peer_id!=Config::get_id(){forget(remote);return None;}
    CLIENT.lock().unwrap().insert((remote.to_owned(),t.session_id),t.clone());Some(t)
}
pub(crate) fn has_ticket(remote:&str)->bool{client_ticket(remote).is_some()}
pub(crate) fn credentials(remote:&str,fallback:u64,hash:&Hash)->(u64,Vec<u8>){
    client_ticket(remote).map(|t|(t.session_id,proof(&t,hash))).unwrap_or((fallback,Vec::new()))
}
pub(crate) fn accept_proof(lr:&LoginRequest, hash:&Hash, desktop:bool) -> bool {
    let t=match read_ticket(){Some(t)=>t,None=>return false};
    let current=match boot().ok().and_then(|s|u128::from_str_radix(&s,16).ok()){Some(x)=>x,None=>return false};
    lease(&t).map(|l|l.may_resume(&lr.my_id,lr.session_id,now(),current,
        hbb_common::sodiumoxide::utils::memcmp(&lr.tec_assist_resume_proof, &proof(&t,hash)),desktop)).unwrap_or(false)
}
pub(crate) fn remember(remote:&str, session:u64, json:&str) -> bool {
    if crate::get_app_name()!="TecAssistConsole" {return false;}
    if let Ok(t)=serde_json::from_str::<Ticket>(json) {
        if live(&t) && t.session_id==session && t.peer_id==Config::get_id() {
            if save_client_ticket(remote,&t).is_err(){return false;}
            CLIENT.lock().unwrap().insert((remote.to_owned(),session),t); return true;
        }
    }
    false
}
static END_REQUESTED: std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
pub(crate) fn cancel_pending() { END_REQUESTED.store(true,std::sync::atomic::Ordering::SeqCst); FILE_SERVER.lock().unwrap().clear(); FILE_CLIENT.lock().unwrap().clear(); }
fn check_not_cancelled()->Result<(),String> {
    if END_REQUESTED.load(std::sync::atomic::Ordering::SeqCst) {Err("Assistenza terminata: riavvio annullato.".into())}else{Ok(())}
}
pub(crate) fn revoke()->Result<(),String> { revoke_directory(&root()) }
fn revoke_directory(directory:&std::path::Path)->Result<(),String> {
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    let path=directory.join("revoked");
    // FILE_APPEND_DATA | SYNCHRONIZE: do not request FILE_WRITE_DATA, which
    // would allow changing a revoked ticket back into an active ticket.
    let mut file=match std::fs::OpenOptions::new().append(true).access_mode(0x00100004).open(&path) {
        Ok(file)=>file,
        // A missing marker cannot authorize resume: checked_ticket fails closed.
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(()),
        Err(e)=>return Err(format!("Impossibile revocare la ripresa: {e}")),
    };
    file.write_all(&[1]).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
    if std::fs::read(path).map(|x|x.len()>1 && x.last()==Some(&1)).unwrap_or(false){Ok(())}else{Err("Revoca della ripresa non confermata.".into())}
}

fn confirm(peer:&str)->bool {
    #[link(name="user32")]
    extern "system" { fn MessageBoxW(hwnd:*mut std::ffi::c_void,text:*const u16,title:*const u16,flags:u32)->i32; }
    let peer:String=peer.chars().filter(|c|c.is_ascii_digit()).take(20).collect();
    let text=format!("Il tecnico collegato (ID {peer}) richiede di riavviare questo computer.\n\nConsenti allo stesso tecnico di riprendere questa assistenza dopo il riavvio, senza una nuova conferma?\n\nL'autorizzazione dura al massimo 30 minuti e viene revocata con Termina assistenza. Non viene impostato l'accesso automatico a Windows.\n\nWindows potrebbe richiedere la tua conferma per preparare la ripresa. Salva prima i documenti aperti.");
    let text:Vec<u16>=text.encode_utf16().chain(Some(0)).collect();
    let title:Vec<u16>="Tec Assist â€” Riavvio e ripresa temporanea".encode_utf16().chain(Some(0)).collect();
    unsafe {MessageBoxW(std::ptr::null_mut(),text.as_ptr(),title.as_ptr(),0x00050134)==6}
}
pub(crate) fn prepare(peer:String, session:u64, permissions:[bool;7], transfer_expires_at:u64)->Result<String,String> {
    if host() {return Err("Termina la ripresa corrente prima di preparare un altro riavvio.".into());}
    if !confirm(&peer) {return Err("Il cliente non ha autorizzato il riavvio.".into());}
    check_not_cancelled()?;
    let mut secret=[0u8;32]; rand::rngs::OsRng.fill_bytes(&mut secret);
    let issued=now(); let exe=std::env::current_exe().map_err(|e|e.to_string())?;
    let helper=exe.parent().ok_or("Percorso client non disponibile")?.join("TecAssist-PreparaRipresa.exe");
    if !helper.is_file() {return Err("Questo Standalone non contiene il componente per la ripresa.".into());}
    let t=Ticket {peer_id:peer, session_id:session, issued_at:issued, expires_at:issued+TTL,
        boot:boot()?,token:hex(&secret),permissions,transfer_expires_at:if transfer_expires_at>issued {transfer_expires_at.min(issued+TTL)}else{0},source:Some(exe.to_string_lossy().into()),
        config:Some(Config::file().parent().ok_or("Configurazione non disponibile")?.to_string_lossy().into())};
    let dir=std::env::temp_dir().join(format!("TecAssist-Resume-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).map_err(|e|e.to_string())?;
    let req=dir.join("request.dpapi");
    let result=(|| {
        protect_request_directory(&dir)?;
        // Machine scope plus a protected DACL allows UAC credentials belonging
        // to a different administrator, without making the ticket readable to other users.
        let mut envelope=serde_json::to_value(&t).map_err(|e|e.to_string())?;
        envelope["temporary_password"]=serde_json::Value::String(hbb_common::password_security::temporary_password());
        std::fs::write(&req,dpapi_scope(&serde_json::to_vec(&envelope).map_err(|e|e.to_string())?,true,true)?).map_err(|e|e.to_string())?;
        if !run_elevated(&helper,&req)? {return Err("Preparazione non riuscita o conferma Windows rifiutata.".into());}
        if let Err(e)=check_not_cancelled(){let _=revoke();return Err(e);}
        let mut result=t.clone(); result.source=None;result.config=None;
        serde_json::to_string(&result).map_err(|e|e.to_string())
    })();
    let _=std::fs::remove_file(req); let _=std::fs::remove_dir(dir); result
}
fn run_elevated(helper:&std::path::Path,request:&std::path::Path)->Result<bool,String> {
    use winapi::{um::{shellapi::{SHELLEXECUTEINFOW,ShellExecuteExW},synchapi::WaitForSingleObject,processthreadsapi::GetExitCodeProcess,handleapi::CloseHandle},shared::minwindef::DWORD};
    fn w(s:&std::ffi::OsStr)->Vec<u16>{use std::os::windows::ffi::OsStrExt;s.encode_wide().chain(Some(0)).collect()}
    let verb=w(std::ffi::OsStr::new("runas")); let path=w(helper.as_os_str());
    let args=w(std::ffi::OsStr::new(&format!("\"{}\"",request.display())));
    let mut info:SHELLEXECUTEINFOW=unsafe{std::mem::zeroed()};info.cbSize=std::mem::size_of_val(&info) as DWORD;
    info.fMask=0x40;info.lpVerb=verb.as_ptr();info.lpFile=path.as_ptr();info.lpParameters=args.as_ptr(); info.nShow=0;
    if unsafe{ShellExecuteExW(&mut info)}==0 {return Err("Conferma Windows annullata.".into());}
    unsafe {WaitForSingleObject(info.hProcess,180000);let mut code=1;GetExitCodeProcess(info.hProcess,&mut code);CloseHandle(info.hProcess);Ok(code==0)}
}
pub(crate) fn commit()->Result<(),String> {
    check_not_cancelled()?;
    std::fs::write(root().join("committed"),[1]).map_err(|e|e.to_string())?;
    use std::os::windows::process::CommandExt;
    let result=std::process::Command::new("C:\\Windows\\System32\\shutdown.exe").args(["/r","/t","0"]).creation_flags(0x08000000).status().map_err(|e|e.to_string())?;
    if result.success(){Ok(())}else{let _=revoke();Err("Windows non ha avviato il riavvio.".into())}
}
pub(crate) fn guard() {
    std::thread::spawn(||loop {
        if let Err(reason)=checked_ticket(){trace(reason);let _=crate::platform::send_close("");std::process::exit(0);}
        std::thread::sleep(std::time::Duration::from_secs(1));
    });
}
pub(crate) fn broker() {
    use std::os::windows::process::CommandExt;
    use winapi::um::{processthreadsapi::{GetExitCodeProcess,TerminateProcess},handleapi::CloseHandle};
    let mut previous=None;let mut child:winapi::shared::ntdef::HANDLE=std::ptr::null_mut();let mut ui:winapi::shared::ntdef::HANDLE=std::ptr::null_mut();let mut ui_session=None;
    trace("broker-started");
    loop {
        if let Err(reason)=checked_ticket(){trace(reason);break;}
        let sid=crate::platform::get_current_session_id(false);
        let mut alive=false;
        if !child.is_null(){let mut code=0;alive=unsafe{GetExitCodeProcess(child,&mut code)}!=0&&code==259;}
        if sid != u32::MAX && (previous != Some(sid)||!alive) {
            if !child.is_null(){unsafe{CloseHandle(child);};child=std::ptr::null_mut();}
            if previous.is_some() {let _=crate::platform::send_close("");}
            let exe=match std::env::current_exe(){Ok(x)=>x,Err(_)=>break};
            let cmd=format!("\"{}\" --server --tec-resume-server",exe.display());
            if let Ok(handle)=crate::platform::launch_privileged_process(sid,&cmd) {
                if !handle.is_null() {child=handle;previous=Some(sid);trace("server-created");}
            }
        }
        // Wait for the backend IPC, otherwise a GUI started too early could
        // create a second backend with its own freshly generated password.
        let backend_ready=alive&&ui_session!=Some(sid)&&crate::ipc::get_config("temporary-password")
            .ok().flatten().map(|p|p.len()>=6).unwrap_or(false);
        if backend_ready {
            if let Ok(exe)=std::env::current_exe(){
                if let Ok(handle)=crate::platform::tec_assist_launch_resume_ui(sid,&exe.to_string_lossy()){
                    if !handle.is_null(){if !ui.is_null(){unsafe{CloseHandle(ui);}}ui=handle;ui_session=Some(sid);trace("standalone-window-created");}
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    let _=revoke();
    let _=crate::platform::send_close("");
    if !child.is_null(){unsafe{TerminateProcess(child,0);CloseHandle(child);}}
    if !ui.is_null(){unsafe{TerminateProcess(ui,0);CloseHandle(ui);}}
    let _=std::fs::remove_file(root().join("lease.dpapi"));
    for name in ["TecAssistStandalone.toml","TecAssistStandalone2.toml"] {
        let _=std::fs::remove_file(root().join("config").join(name));
    }
    let _=std::process::Command::new("C:\\Windows\\System32\\schtasks.exe").args(["/Delete","/TN",TASK_NAME,"/F"]).creation_flags(0x08000000).status();
}

/// Side-effect-free with respect to startup, networking and reboot. Writes only its report.
pub(crate) fn self_test(output:&str) {
    use hbb_common::protobuf::Message as _;
    let clear=b"TecAssist-DPAPI-test-only-not-a-credential";
    let protected=dpapi(clear,true);
    let roundtrip=protected.as_ref().ok().and_then(|x|dpapi(x,false).ok()).map(|x|x==clear).unwrap_or(false);
    let machine_roundtrip=dpapi_scope(clear,true,true).ok().and_then(|x|dpapi(&x,false).ok()).map(|x|x==clear).unwrap_or(false);
    let dir=std::env::temp_dir().join(format!("TecAssist-Resume-Selftest-{}",uuid::Uuid::new_v4()));
    let acl_ok=std::fs::create_dir(&dir).is_ok() && protect_request_directory(&dir).is_ok();
    let missing_revocation=revoke_directory(&dir).is_ok();
    let marker=dir.join("revoked");
    let append_revocation=std::fs::write(&marker,[0]).is_ok() && revoke_directory(&dir).is_ok() && std::fs::read(&marker).ok()==Some(vec![0,1]);
    let repeated_revocation=revoke_directory(&dir).is_ok() && std::fs::read(&marker).ok()==Some(vec![0,1,1]);
    let _=std::fs::remove_file(marker);
    let _=std::fs::remove_dir(&dir);
    let boot_value=boot().ok();
    let boot_ok=boot_value.as_ref().map(|b|b.len()==32).unwrap_or(false)&&boot_value==boot().ok();
    let t=Ticket {peer_id:"test-peer".into(),session_id:7,issued_at:now(),expires_at:now()+TTL,
        boot:"00000000000000000000000000000001".into(),token:"ab".repeat(32),permissions:[true;7],transfer_expires_at:0,source:None,config:None};
    let mut h=Hash::new();h.salt="salt".into();h.challenge="challenge-a".into();
    let a=proof(&t,&h);h.challenge="challenge-b".into();let b=proof(&t,&h);
    let mut other=t.clone();other.session_id+=1;
    let binding=a!=b && b!=proof(&other,&h);
    let remote=format!("selftest-{}",uuid::Uuid::new_v4());
    let mut cached=t.clone();cached.peer_id=Config::get_id();
    let stored=save_client_ticket(&remote,&cached).is_ok();
    CLIENT.lock().unwrap().retain(|(id,_),_|id!=&remote);
    let (original,loaded_proof)=credentials(&remote,999,&h);
    let reload=stored&&original==cached.session_id&&loaded_proof==proof(&cached,&h);
    let frames=crate::client::tec_assist_resume_frame_test(&remote,&h,cached.session_id);
    forget(&remote);
    let mut probe=LoginRequest::new();probe.session_id=original;probe.tec_assist_resume_probe=true;probe.tec_assist_resume_proof=loaded_proof.into();
    let protocol=probe.write_to_bytes().ok().and_then(|x|LoginRequest::parse_from_bytes(&x).ok())
        .map(|x|x.tec_assist_resume_probe&&x.session_id==cached.session_id&&!x.tec_assist_resume_proof.is_empty()).unwrap_or(false);
    let transfer_checks=transfer_selftest();
    let result=serde_json::json!({"transfer25":transfer_checks,"dpapiCurrentUserRoundTrip":roundtrip,"dpapiLocalMachineRoundTrip":machine_roundtrip,"requestDirectoryProtected":acl_ok,"windowsBootTimeAvailable":boot_ok,"bootTimeFileTimeHex":boot_value,
        "proofBoundToChallengeAndSession":binding,"startupModified":false,"rebootRequested":false,
        "encryptedTicketReloadForNewWindow":reload,
        "resumeProbeFieldsRoundTrip":protocol,
        "probeThenConnectFramesVerified":frames,
        "missingRevocationIsIdempotent":missing_revocation,"appendRevocationVerified":append_revocation,"repeatedRevocationVerified":repeated_revocation,
        "passed":transfer_checks.values().all(|v|v.as_bool()==Some(true))&&missing_revocation&&append_revocation&&repeated_revocation&&roundtrip&&machine_roundtrip&&acl_ok&&boot_ok&&binding&&reload&&protocol&&frames});
    let _=std::fs::write(output,result.to_string());
}

// File scope is separate from desktop authentication, never inferred from a file capability.
lazy_static::lazy_static! {
    static ref FILE_SERVER: Mutex<HashMap<(String,u64),Ticket>>=Mutex::new(HashMap::new());
    static ref FILE_CLIENT: Mutex<HashMap<String,Ticket>>=Mutex::new(HashMap::new());
}
pub(crate) fn confirm_files(peer:&str)->bool {
    #[link(name="user32")] extern "system" {fn MessageBoxW(hwnd:*mut std::ffi::c_void,text:*const u16,title:*const u16,flags:u32)->i32;}
    let peer:String=peer.chars().filter(|c|c.is_ascii_digit()).take(20).collect();
    let text=format!("Autorizzare ANCHE il trasferimento file per il tecnico {} in questa sessione?\n\nVale al massimo 30 minuti da ora, anche dopo Riavvia e riprendi, senza rinnovare la scadenza. Termina assistenza o disabilita File per revocare.\n\nSì: desktop e file. No: solo desktop, con richiesta separata per i file.",peer);
    let text:Vec<u16>=text.encode_utf16().chain(Some(0)).collect();
    let title:Vec<u16>="Tec Assist — Consenso iniziale ai file".encode_utf16().chain(Some(0)).collect();
    // YES/NO, default NO, foreground, warning. This is a local CM action only.
    crate::tec_assist_elevation::local_prompt(|| unsafe{MessageBoxW(std::ptr::null_mut(),text.as_ptr(),title.as_ptr(),0x00050134)==6})
}
fn file_scope(t:&Ticket,peer:&str,session:u64,time:u64)->bool {
    live(t) && t.peer_id==peer && t.session_id==session && t.permissions[3]
        && t.transfer_expires_at>t.issued_at && time>=t.issued_at
        && time<t.transfer_expires_at && t.transfer_expires_at<=t.expires_at
}
fn file_proof(t:&Ticket,hash:&Hash)->Vec<u8> {
    let mut scoped=hash.clone();
    scoped.challenge=format!("TecAssist-FileTransfer-v25\0{}",hash.challenge);
    proof(t,&scoped)
}
pub(crate) fn issue_files(lr:&LoginRequest,permissions:[bool;7])->Option<(u64,String)> {
    check_not_cancelled().ok()?;
    if lr.my_id.is_empty() || lr.session_id==0 || !permissions[3]{return None;}
    let issued=now();let expires=issued.checked_add(TTL)?;
    let mut secret=[0u8;32];rand::rngs::OsRng.fill_bytes(&mut secret);
    let t=Ticket{peer_id:lr.my_id.clone(),session_id:lr.session_id,issued_at:issued,expires_at:expires,
        boot:boot().ok()?,token:hex(&secret),permissions,transfer_expires_at:expires,source:None,config:None};
    let json=serde_json::to_string(&t).ok()?;
    FILE_SERVER.lock().unwrap().insert((lr.my_id.clone(),lr.session_id),t);
    Some((expires,json))
}
pub(crate) fn remember_files(remote:&str,session:u64,json:&str) {
    if crate::get_app_name()!="TecAssistConsole"{return;}
    if let Ok(t)=serde_json::from_str::<Ticket>(json) {
        if file_scope(&t,&Config::get_id(),session,now()) {
            FILE_CLIENT.lock().unwrap().insert(remote.to_owned(),t);
        }
    }
}
pub(crate) fn file_credentials(remote:&str,fallback:u64,hash:&Hash)->(u64,Vec<u8>) {
    if crate::get_app_name()!="TecAssistConsole"{return (fallback,Vec::new());}
    // A reboot ticket supersedes the pre-reboot ticket, even when it lacks file scope.
    let t=client_ticket(remote).or_else(||FILE_CLIENT.lock().unwrap().get(remote).cloned());
    t.filter(|t|file_scope(t,&Config::get_id(),t.session_id,now()))
        .map(|t|(t.session_id,file_proof(&t,hash))).unwrap_or((fallback,Vec::new()))
}
fn server_file_ticket(lr:&LoginRequest)->Option<Ticket> {
    check_not_cancelled().ok()?;
    let t=if host(){read_ticket()}else{FILE_SERVER.lock().unwrap().get(&(lr.my_id.clone(),lr.session_id)).cloned()}?;
    if !file_scope(&t,&lr.my_id,lr.session_id,now()){return None;}
    Some(t)
}
pub(crate) fn accept_files(lr:&LoginRequest,hash:&Hash)->Option<Ticket> {
    if !lr.has_file_transfer() || lr.tec_assist_resume_probe{return None;}
    let t=server_file_ticket(lr)?;
    if !hbb_common::sodiumoxide::utils::memcmp(&lr.tec_assist_resume_proof,&file_proof(&t,hash)){return None;}
    Some(t)
}
pub(crate) fn files_alive(lr:&LoginRequest)->bool {server_file_ticket(lr).is_some()}
pub(crate) fn revoke_files(peer:&str,session:u64){FILE_SERVER.lock().unwrap().remove(&(peer.to_owned(),session));}

fn transfer_selftest()->serde_json::Map<String,serde_json::Value> {
    use hbb_common::protobuf::Message;
    let mut checks=serde_json::Map::new();
    macro_rules! check {($name:expr,$value:expr)=>{checks.insert($name.into(),serde_json::json!($value));};}
    let mut lr=LoginRequest::new();lr.my_id="transfer-selftest-peer".into();lr.session_id=912345;
    lr.set_file_transfer(Default::default());
    let mut h=Hash::new();h.salt="test-salt".into();h.challenge="challenge-1".into();
    let (_,json)=issue_files(&lr,[true;7]).unwrap();
    let t:Ticket=serde_json::from_str(&json).unwrap();
    lr.tec_assist_resume_proof=file_proof(&t,&h).into();
    check!("specificConsentAccepted",accept_files(&lr,&h).is_some());
    check!("deadlineMaximum30Minutes",t.expires_at-t.issued_at==1800 && t.transfer_expires_at==t.expires_at);
    let original_deadline=t.transfer_expires_at;
    let _=accept_files(&lr,&h);let _=accept_files(&lr,&h);
    check!("reconnectDoesNotRenew",server_file_ticket(&lr).unwrap().transfer_expires_at==original_deadline);
    let mut changed=lr.clone();changed.my_id="other-peer".into();
    check!("differentTechnicianRejected",accept_files(&changed,&h).is_none());
    changed=lr.clone();changed.session_id+=1;
    check!("differentSessionRejected",accept_files(&changed,&h).is_none());
    let mut fresh=h.clone();fresh.challenge="challenge-2".into();
    check!("replayOnNewChallengeRejected",accept_files(&lr,&fresh).is_none());
    changed=lr.clone();changed.tec_assist_resume_proof=proof(&t,&h).into();
    check!("desktopProofCannotAuthorizeFiles",accept_files(&changed,&h).is_none());
    changed=lr.clone();changed.union=None;
    check!("fileProofCannotAuthorizeDesktop",accept_files(&changed,&h).is_none());
    changed=lr.clone();changed.set_port_forward(Default::default());
    check!("portForwardRejected",accept_files(&changed,&h).is_none());
    changed=lr.clone();changed.tec_assist_resume_probe=true;
    check!("probeCannotAuthorizeFiles",accept_files(&changed,&h).is_none());
    let mut denied=t.clone();denied.transfer_expires_at=0;
    check!("noSpecificConsentRejected",!file_scope(&denied,&lr.my_id,lr.session_id,now()));
    denied=t.clone();denied.permissions[3]=false;
    check!("filePermissionRequired",!file_scope(&denied,&lr.my_id,lr.session_id,now()));
    check!("expiryBoundaryRejected",!file_scope(&t,&lr.my_id,lr.session_id,t.transfer_expires_at));
    check!("clockRollbackRejected",!file_scope(&t,&lr.my_id,lr.session_id,t.issued_at-1));
    denied=t.clone();denied.expires_at+=1;
    check!("extendedLeaseRejected",!file_scope(&denied,&lr.my_id,lr.session_id,now()));
    denied=t.clone();denied.transfer_expires_at+=1;
    check!("fileDeadlineCannotExceedLease",!file_scope(&denied,&lr.my_id,lr.session_id,now()));
    let mut resumed=t.clone();resumed.issued_at+=1;resumed.expires_at+=1;
    let serialized=serde_json::to_string(&resumed).unwrap();
    let loaded:Ticket=serde_json::from_str(&serialized).unwrap();
    check!("rebootSerializationPreservesOriginalDeadline",loaded.transfer_expires_at==original_deadline);
    revoke_files(&lr.my_id,lr.session_id);
    check!("revocationRejectsNewAndActiveTransfer",accept_files(&lr,&h).is_none() && !files_alive(&lr));
    check!("fileRequestProtocolRoundTrip",LoginRequest::parse_from_bytes(&lr.write_to_bytes().unwrap()).unwrap().has_file_transfer());
    let remote=format!("transfer-frame-{}",uuid::Uuid::new_v4());
    lr.my_id=Config::get_id();
    let (_,json)=issue_files(&lr,[true;7]).unwrap();
    remember_files(&remote,lr.session_id,&json);
    let frame=crate::client::tec_assist_file_frame_test(&remote,&h);
    check!("realFileFrameUsesOriginalSession",frame.has_file_transfer() && !frame.tec_assist_resume_probe && frame.session_id==lr.session_id);
    check!("realFileFrameProofAccepted",accept_files(&frame,&h).is_some());
    FILE_CLIENT.lock().unwrap().remove(&remote);
    revoke_files(&lr.my_id,lr.session_id);
    checks
}
