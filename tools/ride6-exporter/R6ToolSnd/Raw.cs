using System.Text.RegularExpressions;
using CUE4Parse.FileProvider;

namespace R6ToolSnd;

/// <summary>Mode <c>raw &lt;regex&gt; &lt;folder&gt;</c>: saves every file whose path matches the regex (case insensitive)
/// under the folder, at its path in the install (<c>ride6/Content/WwiseAudio/Media/1234.wem</c>). A file that is already
/// there with the same size is kept, so an interrupted run resumes.</summary>
public static class Raw
{
    public static int Run(DefaultFileProvider provider, string pattern, string outDir)
    {
        var regex = new Regex(pattern, RegexOptions.IgnoreCase | RegexOptions.Compiled);
        var files = provider.Files.Values.Where(file => regex.IsMatch(file.Path)).OrderBy(file => file.Path, StringComparer.Ordinal).ToList();
        Console.WriteLine($"{files.Count} files match");
        var written = 0;
        var kept = 0;
        long bytes = 0;
        foreach (var file in files)
        {
            var target = Path.Combine(outDir, file.Path);
            if (File.Exists(target) && new FileInfo(target).Length == file.Size)
            {
                kept++;
                continue;
            }
            var data = file.Read();
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            var temp = target + ".tmp";
            File.WriteAllBytes(temp, data);
            File.Move(temp, target, true);
            written++;
            bytes += data.Length;
        }
        Console.WriteLine($"{written} files written ({bytes / 1048576} MiB), {kept} already there");
        return files.Count == 0 ? 1 : 0;
    }
}
