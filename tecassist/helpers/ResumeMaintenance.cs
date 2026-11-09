using System;
using System.IO;
using System.Collections.Generic;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Web.Script.Serialization;

internal static class ResumeMaintenance {
    internal const string Marker="TecAssist-Resume-v1.marker";
    const string MarkerText="TecAssist temporary reboot continuation v1";
    // BEGIN GENERATED IDENTITIES
    static readonly string[] KnownPairs=new string[]{"682d62fdfa502ce155cf988d047368f06fdc68e754fd30cf88c14ab1777de5cd:9894a8bc5bc0539956bce663ae5c57b36ff11a3cfba75ee9aaa220c2449f7b48","8438f0773351ad8b12db137f0a4ff7068f6779a2735900e18ea1ab9ca7bd9b98:a479401ccc82f4625785cf40ab18bc97e50dfbe008091680f31fd459202772ff","8e7b4b97a879e994cc9a4f5db616b3fe57eff92eae8e55e23cdc41aca79c36a3:88665630e46b010bcd9b6fa7bb241e22d1c135dd4d47865f4e8d652d4f94c54b","90e8d7d1f6d5f2bba02abdcee550061b7146a90001820e6af34d658382ab728d:94fb0f1ac5ffea377fffa3a3b0873fa2e72881933dfe1a3b6a964e47e98e28c3","abe3dbf54c7f47bd4a7489c5dae451f3c2343499f518edbb7cb703074bb863e1:ca3ccc48f795f8c9e45d5739323df84798a2d866d9c66407ab672a9dba1f9e53"};
    static readonly string[] KnownAppFiles=new string[]{"LICENZA_RUSTDESK.txt","TecAssist-PreparaRipresa.exe","TecAssist_Standalone.exe","data/app.so","data/flutter_assets/AssetManifest.bin","data/flutter_assets/AssetManifest.json","data/flutter_assets/FontManifest.json","data/flutter_assets/NOTICES.Z","data/flutter_assets/assets/actions.svg","data/flutter_assets/assets/actions_mobile.svg","data/flutter_assets/assets/address_book.ttf","data/flutter_assets/assets/android.svg","data/flutter_assets/assets/arrow.svg","data/flutter_assets/assets/auth-apple.svg","data/flutter_assets/assets/auth-auth0.svg","data/flutter_assets/assets/auth-azure.svg","data/flutter_assets/assets/auth-default.svg","data/flutter_assets/assets/auth-facebook.svg","data/flutter_assets/assets/auth-github.svg","data/flutter_assets/assets/auth-gitlab.svg","data/flutter_assets/assets/auth-google.svg","data/flutter_assets/assets/auth-okta.svg","data/flutter_assets/assets/call_end.svg","data/flutter_assets/assets/call_wait.svg","data/flutter_assets/assets/chat.svg","data/flutter_assets/assets/chat2.svg","data/flutter_assets/assets/checkbox-outline.svg","data/flutter_assets/assets/chevron_up_chevron_down.svg","data/flutter_assets/assets/close.svg","data/flutter_assets/assets/device_group.ttf","data/flutter_assets/assets/display.svg","data/flutter_assets/assets/dots.svg","data/flutter_assets/assets/file.svg","data/flutter_assets/assets/file_transfer.svg","data/flutter_assets/assets/folder.svg","data/flutter_assets/assets/folder_new.svg","data/flutter_assets/assets/fullscreen.svg","data/flutter_assets/assets/fullscreen_exit.svg","data/flutter_assets/assets/gestures.ttf","data/flutter_assets/assets/home.svg","data/flutter_assets/assets/icon.svg","data/flutter_assets/assets/insecure.svg","data/flutter_assets/assets/insecure_relay.svg","data/flutter_assets/assets/kb_layout_iso.svg","data/flutter_assets/assets/kb_layout_not_iso.svg","data/flutter_assets/assets/keyboard.svg","data/flutter_assets/assets/linux.svg","data/flutter_assets/assets/mac.svg","data/flutter_assets/assets/message_24dp_5F6368.svg","data/flutter_assets/assets/more.ttf","data/flutter_assets/assets/peer_searchbar.ttf","data/flutter_assets/assets/pinned.svg","data/flutter_assets/assets/rec.svg","data/flutter_assets/assets/record_screen.svg","data/flutter_assets/assets/refresh.svg","data/flutter_assets/assets/scam.png","data/flutter_assets/assets/screen.svg","data/flutter_assets/assets/search.svg","data/flutter_assets/assets/secure.svg","data/flutter_assets/assets/secure_relay.svg","data/flutter_assets/assets/tabbar.ttf","data/flutter_assets/assets/tec_assist_logo.png","data/flutter_assets/assets/transfer.svg","data/flutter_assets/assets/trash.svg","data/flutter_assets/assets/unpinned.svg","data/flutter_assets/assets/voice_call.svg","data/flutter_assets/assets/voice_call_waiting.svg","data/flutter_assets/assets/win.svg","data/flutter_assets/fonts/MaterialIcons-Regular.otf","data/flutter_assets/packages/dash_chat_2/assets/placeholder.png","data/flutter_assets/packages/dash_chat_2/assets/profile_placeholder.png","data/flutter_assets/packages/flex_color_picker/assets/opacity.png","data/flutter_assets/packages/wakelock_plus/assets/no_sleep.js","data/flutter_assets/packages/window_manager/images/ic_chrome_close.png","data/flutter_assets/packages/window_manager/images/ic_chrome_maximize.png","data/flutter_assets/packages/window_manager/images/ic_chrome_minimize.png","data/flutter_assets/packages/window_manager/images/ic_chrome_unmaximize.png","data/flutter_assets/shaders/ink_sparkle.frag","data/icudtl.dat","desktop_drop_plugin.dll","desktop_multi_window_plugin.dll","dylib_virtual_display.dll","file_selector_windows_plugin.dll","flutter_custom_cursor_plugin.dll","flutter_gpu_texture_renderer_plugin.dll","flutter_windows.dll","librustdesk.dll","screen_retriever_plugin.dll","texture_rgba_renderer_plugin.dll","uni_links_desktop_plugin.dll","url_launcher_windows_plugin.dll","window_manager_plugin.dll","window_size_plugin.dll"};
    // END GENERATED IDENTITIES
    static string Digest(string file) { using(var sha=SHA256.Create()) using(var input=File.OpenRead(file)) return BitConverter.ToString(sha.ComputeHash(input)).Replace("-","").ToLowerInvariant(); }
    static ulong Now(){return (ulong)(DateTime.UtcNow-new DateTime(1970,1,1)).TotalSeconds;}
    static void CheckTree(string path) {
        if((File.GetAttributes(path)&FileAttributes.ReparsePoint)!=0)throw new IOException("Collegamento nella cartella di ripresa: nessuna pulizia eseguita.");
        if(Directory.Exists(path))foreach(string child in Directory.GetFileSystemEntries(path))CheckTree(child);
    }
    static void CheckKnownContents(string root) {
        var top=new HashSet<string>(new string[]{"app","config","log",Marker,"runtime-status.log","lease.dpapi","committed","revoked"},StringComparer.OrdinalIgnoreCase);
        foreach(string item in Directory.GetFileSystemEntries(root))if(!top.Contains(Path.GetFileName(item)))throw new IOException("La cartella contiene file non previsti: non sono stati rimossi.");
        string app=Path.Combine(root,"app");
        if(Directory.Exists(app)){
            var known=new HashSet<string>(KnownAppFiles,StringComparer.OrdinalIgnoreCase);
            foreach(string file in Directory.GetFiles(app,"*",SearchOption.AllDirectories)){
                string relative=file.Substring(app.Length+1).Replace('\\','/');
                if(!known.Contains(relative))throw new IOException("Sono presenti file aggiuntivi nella cartella programmi: pulizia interrotta.");
            }
        }
    }
    static bool Recognized(string root) {
        string marker=Path.Combine(root,Marker);
        if(File.Exists(marker)&&File.ReadAllText(marker)==MarkerText)return true;
        string exe=Path.Combine(root,"app","TecAssist_Standalone.exe"),dll=Path.Combine(root,"app","librustdesk.dll");
        if(!File.Exists(exe)||!File.Exists(dll))return Directory.GetFileSystemEntries(root).Length==0;
        string pair=Digest(exe)+":"+Digest(dll);
        return Array.IndexOf(KnownPairs,pair)>=0;
    }
    static void RejectActiveLease(string root) {
        string file=Path.Combine(root,"lease.dpapi");
        if(!File.Exists(file))return;
        Dictionary<string,object> lease;
        try {
            byte[] raw=System.Security.Cryptography.ProtectedData.Unprotect(File.ReadAllBytes(file),null,System.Security.Cryptography.DataProtectionScope.LocalMachine);
            lease=new JavaScriptSerializer().Deserialize<Dictionary<string,object>>(System.Text.Encoding.UTF8.GetString(raw));
        }catch(System.Security.Cryptography.CryptographicException){return;}
        catch(ArgumentException){return;}
        catch(FileNotFoundException){return;}
        catch(InvalidOperationException){return;}
        if(lease==null)return;
        object expiry;
        if(!lease.TryGetValue("expires_at",out expiry))return;
        ulong expires;try{expires=Convert.ToUInt64(expiry);}catch{return;}
        string revoked=Path.Combine(root,"revoked");
        byte[] state=File.Exists(revoked)?File.ReadAllBytes(revoked):new byte[0];
        if(expires>Now()&&state.Length==1&&state[0]==0)throw new IOException("La ripresa precedente ÃƒÂ¨ ancora autorizzata. Premi prima Termina assistenza sul PC2.");
    }
    static void StopInactiveProcesses(string root) {
        string expected=Path.GetFullPath(Path.Combine(root,"app","TecAssist_Standalone.exe"));
        foreach(Process process in Process.GetProcessesByName("TecAssist_Standalone"))using(process){
            string actual;
            try{actual=process.MainModule.FileName;}catch{continue;}
            if(!String.Equals(Path.GetFullPath(actual),expected,StringComparison.OrdinalIgnoreCase))continue;
            if(process.Id==Process.GetCurrentProcess().Id)throw new IOException("Chiudi prima questa copia dello Standalone.");
            try{
                process.CloseMainWindow();
                if(!process.WaitForExit(1500)){process.Kill();if(!process.WaitForExit(3000))throw new IOException("Una vecchia copia ÃƒÂ¨ ancora in uso.");}
            }catch(InvalidOperationException){}
        }
    }
    static void RemovePreservingMarker(string root) {
        // Resolve and verify the exact explicitly named directory before recursive deletion.
        string target=Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar);
        if(!String.Equals(target,root.TrimEnd(Path.DirectorySeparatorChar),StringComparison.OrdinalIgnoreCase))throw new IOException("Percorso di pulizia non valido.");
        CheckTree(target);
        foreach(string item in Directory.GetFileSystemEntries(target)){
            if(String.Equals(Path.GetFileName(item),Marker,StringComparison.OrdinalIgnoreCase))continue;
            string resolved=Path.GetFullPath(item);
            if(!String.Equals(Path.GetDirectoryName(resolved),target,StringComparison.OrdinalIgnoreCase))throw new IOException("Elemento esterno alla cartella di ripresa.");
            if(Directory.Exists(item))Directory.Delete(item,true);else File.Delete(item);
        }
        // A failure above leaves this ownership marker intact for the next attempt.
        File.Delete(Path.Combine(target,Marker));Directory.Delete(target,false);
    }
    internal static void Cleanup(string root,dynamic folder,bool taskExists) {
        root=Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar);
        if(Directory.Exists(root)){
            CheckTree(root);CheckKnownContents(root);
            if(!Recognized(root))throw new IOException("Cartella non riconosciuta: nessun file ÃƒÂ¨ stato cancellato.");
            RejectActiveLease(root);
        }
        if(taskExists){
            dynamic task=folder.GetTask("TecAssist-RipresaTemporanea");
            string executable=task.Definition.Actions.Item(1).Path;
            string arguments=task.Definition.Actions.Item(1).Arguments;
            if(!String.Equals(executable,Path.Combine(root,"app","TecAssist_Standalone.exe"),StringComparison.OrdinalIgnoreCase)||arguments!="--tec-resume-broker")throw new IOException("AttivitÃƒÂ  esistente non riconosciuta.");
            if((int)task.State==4)task.Stop(0);
            try{folder.DeleteTask("TecAssist-RipresaTemporanea",0);}catch(Exception ex){if(ex.HResult!=unchecked((int)0x80070002))throw;}
        }
        if(!Directory.Exists(root))return;
        // Legacy partial directories are accepted only with known executable/DLL fingerprints.
        // Restore ownership proof BEFORE attempting any deletion or process cleanup.
        File.WriteAllText(Path.Combine(root,Marker),MarkerText);
        StopInactiveProcesses(root);
        RemovePreservingMarker(root);
    }
    internal static Dictionary<string,bool> SelfTest(string parent,string fixtureSource) {
        string basePath=Path.Combine(Path.GetFullPath(parent),"maintenance-test-"+Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(basePath);
        var result=new Dictionary<string,bool>();
        string locked=Path.Combine(basePath,"locked");Directory.CreateDirectory(Path.Combine(locked,"app"));
        File.WriteAllText(Path.Combine(locked,Marker),MarkerText);
        string dll=Path.Combine(locked,"app","librustdesk.dll");File.WriteAllText(dll,"fixture");
        using(var handle=new FileStream(dll,FileMode.Open,FileAccess.Read,FileShare.Read)){
            bool failed=false;try{Cleanup(locked,null,false);}catch(IOException){failed=true;}
            result["lockedFileKeepsOwnershipMarker"]=failed&&File.Exists(Path.Combine(locked,Marker));
        }
        Cleanup(locked,null,false);result["retryCleansOwnedDirectory"]=!Directory.Exists(locked);
        string unknown=Path.Combine(basePath,"unknown");Directory.CreateDirectory(unknown);File.WriteAllText(Path.Combine(unknown,"customer.txt"),"preserve");
        bool rejected=false;try{Cleanup(unknown,null,false);}catch(IOException){rejected=true;}
        result["unrecognizedDirectoryPreserved"]=rejected&&File.ReadAllText(Path.Combine(unknown,"customer.txt"))=="preserve";
        File.Delete(Path.Combine(unknown,"customer.txt"));Directory.Delete(unknown);
        bool recovery=false;
        if(!String.IsNullOrEmpty(fixtureSource)){
            string legacy=Path.Combine(basePath,"legacy");Directory.CreateDirectory(Path.Combine(legacy,"app"));
            File.Copy(Path.Combine(fixtureSource,"TecAssist_Standalone.exe"),Path.Combine(legacy,"app","TecAssist_Standalone.exe"));
            File.Copy(Path.Combine(fixtureSource,"librustdesk.dll"),Path.Combine(legacy,"app","librustdesk.dll"));
            using(var handle=new FileStream(Path.Combine(legacy,"app","librustdesk.dll"),FileMode.Open,FileAccess.Read,FileShare.Read)){
                try{Cleanup(legacy,null,false);}catch(IOException){}
                recovery=File.Exists(Path.Combine(legacy,Marker));
            }
            Cleanup(legacy,null,false);recovery=recovery&&!Directory.Exists(legacy);
        }
        result["knownLegacyPartialDirectoryRecovered"]=recovery;
        string active=Path.Combine(basePath,"active");Directory.CreateDirectory(active);
        File.WriteAllText(Path.Combine(active,Marker),MarkerText);
        byte[] lease=System.Text.Encoding.UTF8.GetBytes("{\"expires_at\":"+(Now()+1800).ToString()+"}");
        File.WriteAllBytes(Path.Combine(active,"lease.dpapi"),ProtectedData.Protect(lease,null,DataProtectionScope.LocalMachine));
        File.WriteAllBytes(Path.Combine(active,"revoked"),new byte[]{0});
        bool activeRejected=false;try{Cleanup(active,null,false);}catch(IOException){activeRejected=true;}
        result["activeAuthorizationPreserved"]=activeRejected&&File.Exists(Path.Combine(active,"lease.dpapi"))&&File.ReadAllBytes(Path.Combine(active,"revoked")).Length==1;
        File.WriteAllBytes(Path.Combine(active,"revoked"),new byte[]{0,1});Cleanup(active,null,false);
        Directory.Delete(basePath,false);
        result["passed"]=result["lockedFileKeepsOwnershipMarker"]&&result["retryCleansOwnedDirectory"]&&result["unrecognizedDirectoryPreserved"]&&recovery&&result["activeAuthorizationPreserved"];
        return result;
    }
}
