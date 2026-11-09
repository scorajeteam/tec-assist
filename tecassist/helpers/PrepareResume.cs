// Runs only after a local consent dialog and Windows UAC. No reboot is performed here.
using System;
using System.IO;
using System.Text;
using System.Security.AccessControl;
using System.Security.Principal;
using System.Security.Cryptography;
using System.Web.Script.Serialization;
using System.Runtime.InteropServices;
using System.Windows.Forms;

class PrepareResume {
    const string TaskName = "TecAssist-RipresaTemporanea";
    static readonly string Root = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles), "TecAssistResume");
    static readonly SecurityIdentifier SystemSid = new SecurityIdentifier(WellKnownSidType.LocalSystemSid, null);
    static readonly SecurityIdentifier AdminSid = new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid, null);
    static readonly SecurityIdentifier UsersSid = new SecurityIdentifier(WellKnownSidType.BuiltinUsersSid, null);
    static readonly JavaScriptSerializer Json = new JavaScriptSerializer();
    public class Request {
        public string peer_id; public ulong session_id; public ulong issued_at; public ulong expires_at;
        public string boot; public string token; public string source; public string config;
        public bool[] permissions;
        public ulong transfer_expires_at;
        public string temporary_password;
    }
    static ulong Now() { return (ulong)(DateTime.UtcNow - new DateTime(1970,1,1)).TotalSeconds; }
    static void ProtectedFile(string path, byte[] data, bool writableByUsers) {
        File.WriteAllBytes(path, data);
        FileSecurity acl = new FileSecurity(); acl.SetAccessRuleProtection(true, false);
        acl.SetOwner(AdminSid);
        acl.AddAccessRule(new FileSystemAccessRule(SystemSid, FileSystemRights.FullControl, AccessControlType.Allow));
        acl.AddAccessRule(new FileSystemAccessRule(AdminSid, FileSystemRights.FullControl, AccessControlType.Allow));
        if (writableByUsers) acl.AddAccessRule(new FileSystemAccessRule(UsersSid, (Path.GetFileName(path) == "revoked" ? FileSystemRights.AppendData : FileSystemRights.Write) | FileSystemRights.Read, AccessControlType.Allow));
        File.SetAccessControl(path, acl);
    }
    static void CheckTree(string path) {
        if ((File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0) throw new IOException("Collegamenti non consentiti.");
        if (Directory.Exists(path)) foreach (string child in Directory.GetFileSystemEntries(path)) CheckTree(child);
    }
    static void CopyTree(string source, string dest) {
        Directory.CreateDirectory(dest);
        foreach (string file in Directory.GetFiles(source)) File.Copy(file, Path.Combine(dest, Path.GetFileName(file)), false);
        foreach (string dir in Directory.GetDirectories(source)) CopyTree(dir, Path.Combine(dest, Path.GetFileName(dir)));
    }
    [STAThread] static int Main(string[] args) {
        try {
            if (args.Length == 2 && args[0] == "--self-test") {
                byte[] sample = Encoding.UTF8.GetBytes("TecAssist-DPAPI-test-only");
                bool current = Convert.ToBase64String(ProtectedData.Unprotect(ProtectedData.Protect(sample, null, DataProtectionScope.CurrentUser), null, DataProtectionScope.CurrentUser)) == Convert.ToBase64String(sample);
                bool machine = Convert.ToBase64String(ProtectedData.Unprotect(ProtectedData.Protect(sample, null, DataProtectionScope.LocalMachine), null, DataProtectionScope.LocalMachine)) == Convert.ToBase64String(sample);
                Request test = Json.Deserialize<Request>("{\"peer_id\":\"test\",\"session_id\":18446744073709551615,\"permissions\":[true,false,true,false,true,false,true]}");
                test.temporary_password="test-only"; test.transfer_expires_at=1800;
                test=Json.Deserialize<Request>(Json.Serialize(test));
                bool serialization = test.session_id == UInt64.MaxValue && test.permissions.Length == 7 && !test.permissions[1] && test.temporary_password=="test-only" && test.transfer_expires_at==1800;
                dynamic service = Activator.CreateInstance(Type.GetTypeFromProgID("Schedule.Service")); service.Connect();
                dynamic definition = service.NewTask(0);
                definition.Principal.UserId = "SYSTEM"; definition.Principal.LogonType = 5; definition.Principal.RunLevel = 1;
                definition.Settings.DeleteExpiredTaskAfter = "PT1M";
                dynamic triggerTest = definition.Triggers.Create(8);
                triggerTest.StartBoundary = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ");
                triggerTest.EndBoundary = DateTime.UtcNow.AddMinutes(30).ToString("yyyy-MM-ddTHH:mm:ssZ");
                dynamic actionTest = definition.Actions.Create(0); actionTest.Path = "C:\\Windows\\System32\\whoami.exe";
                string xml = definition.XmlText;
                bool schedulerDefinition = xml.Contains("BootTrigger") && xml.Contains("EndBoundary") && xml.Contains("DeleteExpiredTaskAfter");
                dynamic validationFolder = service.GetFolder("\\");
                string validationName = "TecAssist-ValidationOnly-" + Guid.NewGuid().ToString("N");
                validationFolder.RegisterTaskDefinition(validationName, definition, 1, "SYSTEM", null, 5, null); // TASK_VALIDATE_ONLY
                bool absent = false; try { validationFolder.GetTask(validationName); } catch (Exception ex) { if (ex.HResult != unchecked((int)0x80070002)) throw; absent = true; }
                File.WriteAllText(args[1], Json.Serialize(new { dpapiCurrentUser=current, dpapiLocalMachine=machine, sessionIdUInt64RoundTrip=serialization, schedulerDefinitionOnly=schedulerDefinition, schedulerXmlValidated=true, taskRegistered=!absent, rebootRequested=false, passed=current&&machine&&serialization&&schedulerDefinition&&absent }));
                return current&&machine&&serialization&&schedulerDefinition&&absent ? 0 : 1;
            }
            if (args.Length != 1 || !new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator)) throw new Exception("Sono necessari i privilegi amministrativi.");
            using(var gate=new System.Threading.Mutex(false,@"Global\TecAssist-ResumeMaintenance-v1")) {
            bool held=false;
            try {
            try { held=gate.WaitOne(1000); } catch(System.Threading.AbandonedMutexException) { held=true; }
            if(!held)throw new Exception("Preparazione già in corso: attendi e riprova.");
            byte[] bytes = ProtectedData.Unprotect(File.ReadAllBytes(args[0]), null, DataProtectionScope.LocalMachine);
            Request r = Json.Deserialize<Request>(Encoding.UTF8.GetString(bytes));
            if (r == null || String.IsNullOrEmpty(r.peer_id) || r.session_id == 0 || r.token == null || r.token.Length != 64 || r.boot == null || r.boot.Length != 32 || r.permissions == null || r.permissions.Length != 7 || r.issued_at > Now() || Now() >= r.expires_at || r.expires_at - r.issued_at != 1800) throw new Exception("Richiesta scaduta o non valida.");
            if (r.transfer_expires_at != 0 && (!r.permissions[3] || r.transfer_expires_at <= r.issued_at || r.transfer_expires_at > r.expires_at)) throw new Exception("Consenso file non valido.");
            string sourceExe = Path.GetFullPath(r.source);
            if (!String.Equals(Path.GetFileName(sourceExe), "TecAssist_Standalone.exe", StringComparison.OrdinalIgnoreCase)) throw new Exception("Client non riconosciuto.");
            string sourceDir = Path.GetDirectoryName(sourceExe);
            CheckTree(sourceDir); CheckTree(r.config);
            if (!File.Exists(Path.Combine(sourceDir, "librustdesk.dll"))) throw new Exception("Componenti client mancanti.");
            dynamic scheduler = Activator.CreateInstance(Type.GetTypeFromProgID("Schedule.Service")); scheduler.Connect();
            dynamic folder = scheduler.GetFolder("\\");
            bool exists = false; try { folder.GetTask(TaskName); exists = true; } catch (Exception ex) { if (ex.HResult != unchecked((int)0x80070002)) throw; }
            ResumeMaintenance.Cleanup(Root, folder, exists);
            Directory.CreateDirectory(Root);
            DirectorySecurity acl = new DirectorySecurity(); acl.SetAccessRuleProtection(true, false); acl.SetOwner(AdminSid);
            InheritanceFlags inherit = InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit;
            acl.AddAccessRule(new FileSystemAccessRule(SystemSid, FileSystemRights.FullControl, inherit, PropagationFlags.None, AccessControlType.Allow));
            acl.AddAccessRule(new FileSystemAccessRule(AdminSid, FileSystemRights.FullControl, inherit, PropagationFlags.None, AccessControlType.Allow));
            acl.AddAccessRule(new FileSystemAccessRule(UsersSid, FileSystemRights.ReadAndExecute, inherit, PropagationFlags.None, AccessControlType.Allow));
            Directory.SetAccessControl(Root, acl);
            File.WriteAllText(Path.Combine(Root, "TecAssist-Resume-v1.marker"), "TecAssist temporary reboot continuation v1");
            File.WriteAllText(Path.Combine(Root, "runtime-status.log"), "preparation-completed\r\n");
            CopyTree(sourceDir, Path.Combine(Root, "app"));
            string config = Path.Combine(Root, "config"); Directory.CreateDirectory(config);
            DirectorySecurity configAcl = new DirectorySecurity(); configAcl.SetAccessRuleProtection(true, false); configAcl.SetOwner(AdminSid);
            configAcl.AddAccessRule(new FileSystemAccessRule(SystemSid, FileSystemRights.FullControl, inherit, PropagationFlags.None, AccessControlType.Allow));
            configAcl.AddAccessRule(new FileSystemAccessRule(AdminSid, FileSystemRights.FullControl, inherit, PropagationFlags.None, AccessControlType.Allow));
            Directory.SetAccessControl(config, configAcl);
            foreach (string file in Directory.GetFiles(r.config, "TecAssistStandalone*.toml")) File.Copy(file, Path.Combine(config, Path.GetFileName(file)));
            if (!File.Exists(Path.Combine(config, "TecAssistStandalone.toml"))) throw new Exception("IdentitÃ  client non salvata: riavvio annullato.");
            // Remove source paths from the protected ticket.
            r.source = null; r.config = null;
            ProtectedFile(Path.Combine(Root, "lease.dpapi"), ProtectedData.Protect(Encoding.UTF8.GetBytes(Json.Serialize(r)), null, DataProtectionScope.LocalMachine), false);
            ProtectedFile(Path.Combine(Root, "revoked"), new byte[] {0}, true);
            ProtectedFile(Path.Combine(Root, "committed"), new byte[] {0}, true);
            dynamic task = scheduler.NewTask(0);
            task.RegistrationInfo.Description = "Tec Assist: ripresa temporanea dell'assistenza autorizzata, massimo 30 minuti.";
            task.Principal.UserId = "SYSTEM"; task.Principal.LogonType = 5; task.Principal.RunLevel = 1;
            task.Settings.Enabled = true; task.Settings.ExecutionTimeLimit = "PT30M";
            task.Settings.DisallowStartIfOnBatteries = false; task.Settings.StopIfGoingOnBatteries = false;
            task.Settings.DeleteExpiredTaskAfter = "PT1M"; task.Settings.MultipleInstances = 2;
            dynamic trigger = task.Triggers.Create(8); // TASK_TRIGGER_BOOT
            trigger.StartBoundary = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ");
            trigger.EndBoundary = new DateTime(1970,1,1,0,0,0,DateTimeKind.Utc).AddSeconds(r.expires_at).ToString("yyyy-MM-ddTHH:mm:ssZ");
            dynamic action = task.Actions.Create(0);
            action.Path = Path.Combine(Root, "app", "TecAssist_Standalone.exe");
            action.Arguments = "--tec-resume-broker"; action.WorkingDirectory = Path.Combine(Root, "app");
            if (Now() >= r.expires_at) throw new Exception("La preparazione Ã¨ scaduta: ripeti la richiesta dalla Console.");
            folder.RegisterTaskDefinition(TaskName, task, 2, "SYSTEM", null, 5, null); // CREATE, never overwrite
            return 0;
            } finally { if(held)gate.ReleaseMutex(); }
            }
        } catch (Exception e) {
            if (args.Length == 2 && args[0] == "--self-test") {
                File.WriteAllText(args[1], Json.Serialize(new {passed=false,errorType=e.GetType().Name,hresult=e.HResult,taskRegistered=false,rebootRequested=false})); return 1;
            }
            // No reboot and no token on errors. Expiry also gates any leftover task.
            MessageBox.Show("Ripresa dopo riavvio non preparata. Il computer non verrÃ  riavviato.\n\n" + e.Message, "Tec Assist", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            return 1;
        }
    }
}
