using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Security.Cryptography;
using System.Windows.Forms;

[assembly: AssemblyTitle("Tec Assist Console")]
[assembly: AssemblyProduct("Tec Assist")]
[assembly: AssemblyVersion("1.4.4.25")]

internal static class Launcher
{
    private const string Client = "TecAssist_Console.exe";

    [STAThread]
    private static void Main(string[] args)
    {
        try
        {
            string root = Path.Combine(Environment.GetFolderPath(
                Environment.SpecialFolder.LocalApplicationData), "TecAssist", "Console");
            Directory.CreateDirectory(root);
            using (Stream payload = Assembly.GetExecutingAssembly().GetManifestResourceStream("payload.zip"))
            using (SHA256 hash = SHA256.Create())
            {
                string version = Digest(hash, payload);
                payload.Position = 0;
                string destination = Path.Combine(root, version);
                using (ZipArchive archive = new ZipArchive(payload))
                {
                    if (!Directory.Exists(destination))
                    {
                        string staging = Path.Combine(root, Guid.NewGuid().ToString("N"));
                        Directory.CreateDirectory(staging);
                        try
                        {
                            archive.ExtractToDirectory(staging);
                            try { Directory.Move(staging, destination); }
                            catch (IOException) { if (!Directory.Exists(destination)) throw; }
                        }
                        finally
                        {
                            if (Directory.Exists(staging)) {
                                if (!String.Equals(Path.GetDirectoryName(Path.GetFullPath(staging)), Path.GetFullPath(root), StringComparison.OrdinalIgnoreCase)) throw new IOException("Percorso temporaneo non valido.");
                                Directory.Delete(staging, true);
                            }
                        }
                    }
                    Verify(archive, destination, hash);
                }
                if (args.Length == 1 && args[0] == "--verify-only") return;
                if (args.Length == 1 && args[0].StartsWith("--check-engine=")) {
                    string expectedEngine;
                    using (Stream file = File.OpenRead(Path.Combine(destination, "librustdesk.dll"))) expectedEngine = Digest(hash, file);
                    Environment.ExitCode = IsOtherEngine(args[0].Substring(15), expectedEngine, hash) ? 2 : 0; return;
                }
                bool conflict = HasConflictingConsole(destination, hash);
                if (args.Length == 1 && args[0] == "--check-running-version") {
                    Environment.ExitCode = conflict ? 2 : 0; return;
                }
                if (conflict) {
                    Environment.ExitCode = 2;
                    MessageBox.Show("È ancora aperta un'altra versione della Console.\n\nChiudila con il pulsante Termina programma, poi riapri questa Console 1.4.4.24.\n\nLa nuova Console non verrà avviata sopra quella precedente.", "Tec Assist — Chiudi la Console precedente", MessageBoxButtons.OK, MessageBoxIcon.Information);
                    return;
                }
                Process.Start(new ProcessStartInfo(Path.Combine(destination, Client))
                {
                    WorkingDirectory = destination,
                    UseShellExecute = false
                });
            }
        }
        catch (Exception error)
        {
            Environment.ExitCode = 1;
            if (args.Length == 1 && args[0] == "--verify-only") return;
            MessageBox.Show(error.Message, "Tec Assist - Impossibile avviare",
                MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
    }

    private static bool IsOtherEngine(string dll, string expected, SHA256 hash)
    {
        using (Stream file = File.OpenRead(dll))
            return !String.Equals(Digest(hash, file), expected, StringComparison.OrdinalIgnoreCase);
    }

    private static bool HasConflictingConsole(string destination, SHA256 hash)
    {
        if (!String.Equals(Client, "TecAssist_Console.exe", StringComparison.OrdinalIgnoreCase)) return false;
        string expected;
        using (Stream file = File.OpenRead(Path.Combine(destination, "librustdesk.dll"))) expected = Digest(hash, file);
        foreach (Process running in Process.GetProcessesByName("TecAssist_Console"))
        using (running)
        {
            try
            {
                if (running.HasExited) continue;
                string dll = Path.Combine(Path.GetDirectoryName(running.MainModule.FileName), "librustdesk.dll");
                if (IsOtherEngine(dll, expected, hash)) return true;
            }
            catch (InvalidOperationException) { continue; }
            catch { return true; }
        }
        return false;
    }

    private static void Verify(ZipArchive archive, string destination, SHA256 hash)
    {
        var pending = new System.Collections.Generic.Stack<string>();
        pending.Push(destination);
        int files = 0; int expectedFiles = 0;
        while (pending.Count != 0)
        {
            string directory = pending.Pop();
            RejectLink(directory);
            foreach (string child in Directory.GetDirectories(directory)) pending.Push(child);
            foreach (string file in Directory.GetFiles(directory))
            {
                RejectLink(file);
                files++;
            }
        }

        foreach (ZipArchiveEntry entry in archive.Entries)
        {
            if (String.IsNullOrEmpty(entry.Name)) continue; expectedFiles++; string path = Path.Combine(destination, entry.FullName);
            using (Stream original = entry.Open())
            using (Stream extracted = File.OpenRead(path))
            {
                if (entry.Length != extracted.Length || Digest(hash, original) != Digest(hash, extracted))
                    throw new IOException("File di Tec Assist alterato: " + path);
            }
        }

        if (files != expectedFiles)
            throw new IOException("File aggiuntivi nella cartella di Tec Assist: " + destination);
    }

    private static void RejectLink(string path)
    {
        if ((File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0)
            throw new IOException("Percorso di Tec Assist non valido: " + path);
    }

    private static string Digest(SHA256 hash, Stream stream)
    {
        return BitConverter.ToString(hash.ComputeHash(stream)).Replace("-", "");
    }
}
