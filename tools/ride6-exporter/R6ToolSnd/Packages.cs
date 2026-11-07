using CUE4Parse.FileProvider;
using CUE4Parse.FileProvider.Objects;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace R6ToolSnd;

/// <summary>Reads packages of the install as the JSON of their exports (the form <c>R6ToolParts jsonmany</c> writes),
/// each package once.</summary>
public sealed class Packages
{
    private readonly DefaultFileProvider provider;
    private readonly Dictionary<string, JArray?> cache = new(StringComparer.OrdinalIgnoreCase);

    public Packages(DefaultFileProvider provider) => this.provider = provider;

    /// <summary>The exports of a package named by an Unreal object path (<c>/Game/x/y.y</c>, <c>/Plugin/x/y</c>) or a
    /// file-system style path, with or without the object suffix; null when the install has no such package.</summary>
    public JArray? Load(string path)
    {
        if (cache.TryGetValue(path, out var known)) return known;
        var file = Find(path);
        JArray? exports = null;
        if (file != null)
        {
            var package = provider.LoadPackage(file);
            exports = JArray.Parse(JsonConvert.SerializeObject(package.GetExports()));
        }
        cache[path] = exports;
        return exports;
    }

    /// <summary>The first export of a package whose <c>Type</c> is <paramref name="type"/>.</summary>
    public static JToken? Export(JArray exports, string type) =>
        exports.FirstOrDefault(export => (string?)export["Type"] == type);

    private GameFile? Find(string path)
    {
        path = path.Trim().Replace('\\', '/');
        var dot = path.LastIndexOf('.');
        var slash = path.LastIndexOf('/');
        if (dot > slash && !path.EndsWith(".uasset") && !path.EndsWith(".umap")) path = path[..dot];
        foreach (var candidate in Candidates(path))
        {
            foreach (var ext in new[] { "", ".uasset", ".umap" })
                if (provider.Files.TryGetValue(candidate + ext, out var file) && file.IsUePackage) return file;
        }
        return null;
    }

    private static IEnumerable<string> Candidates(string path)
    {
        yield return path;
        var trimmed = path.TrimStart('/');
        yield return trimmed;
        var first = trimmed.IndexOf('/');
        if (first < 0) yield break;
        var mount = trimmed[..first];
        var rest = trimmed[(first + 1)..];
        if (mount.Equals("Game", StringComparison.OrdinalIgnoreCase))
        {
            yield return "ride6/Content/" + rest;
            yield break;
        }
        yield return $"ride6/Plugins/{mount}/Content/{rest}";
    }
}
