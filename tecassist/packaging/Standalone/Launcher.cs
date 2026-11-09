using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Security.Cryptography;
using System.Windows.Forms;

[assembly: AssemblyTitle("Tec Assist Standalone")]
[assembly: AssemblyProduct("Tec Assist")]
[assembly: AssemblyVersion("1.4.4.25")]

internal static class Launcher
{
    private const string Client = "TecAssist_Standalone.exe";

    [STAThread]
    private static void Main(string[] args)
    {
        try
        {
            string root = Path.Combine(Environment.GetFolderPath(
                Environment.SpecialFolder.LocalApplicationData), "TecAssist", "Standalone");
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
